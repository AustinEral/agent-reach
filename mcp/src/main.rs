//! agent-reach-mcp: MCP server for agent-reach discovery registry

use std::sync::Arc;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rmcp::{
    Error as McpError, ServiceExt,
    model::{
        ServerCapabilities, Implementation, ServerInfo, Tool, CallToolResult,
        Content, ListToolsResult, CallToolRequestParam, PaginatedRequestParam,
        ToolsCapability,
    },
    handler::server::ServerHandler,
    service::{RequestContext, RoleServer},
    transport::stdio,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::info;

use agent_id::RootKey;
use agent_id_handshake::{
    messages::{Hello, Challenge, ProofAccepted},
    protocol::sign_proof,
};

/// Default registry URL
const DEFAULT_REGISTRY_URL: &str = "https://reach.agent-id.ai";

/// Stored identity format (matches agent-id-mcp)
#[derive(Serialize, Deserialize)]
struct StoredIdentity {
    version: u32,
    did: String,
    private_key: String,
    created: String,
}

/// Identity file location (same as agent-id-mcp)
fn identity_path() -> PathBuf {
    directories::ProjectDirs::from("ai", "agent-id", "agent-id")
        .map(|dirs| dirs.data_dir().join("identity.json"))
        .unwrap_or_else(|| PathBuf::from("~/.agent-id/identity.json"))
}

/// Load identity from disk
fn load_identity() -> Result<RootKey> {
    let path = identity_path();
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read identity from {:?}", path))?;
    let stored: StoredIdentity = serde_json::from_str(&content)
        .context("Failed to parse identity file")?;
    
    let key_bytes = BASE64.decode(&stored.private_key)
        .context("Failed to decode private key")?;
    let key_array: [u8; 32] = key_bytes.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid private key length"))?;
    let key = RootKey::from_bytes(&key_array)
        .context("Failed to create key from bytes")?;
    Ok(key)
}

/// MCP Server state
#[derive(Clone)]
struct ReachMcpServer {
    key: Arc<RootKey>,
    client: reqwest::Client,
    registry_url: String,
    session: Arc<RwLock<Option<String>>>,
}

#[derive(Deserialize)]
struct ProofAcceptedResponse {
    session_id: String,
}

#[derive(Deserialize)]
struct LookupResponse {
    did: String,
    endpoint: String,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: String,
}

impl ReachMcpServer {
    fn new(key: RootKey) -> Self {
        Self {
            key: Arc::new(key),
            client: reqwest::Client::new(),
            registry_url: std::env::var("REACH_REGISTRY_URL")
                .unwrap_or_else(|_| DEFAULT_REGISTRY_URL.to_string()),
            session: Arc::new(RwLock::new(None)),
        }
    }

    async fn authenticate(&self) -> Result<String, String> {
        // Check existing session
        if let Some(ref session_id) = *self.session.read().await {
            return Ok(session_id.clone());
        }

        info!("Authenticating with registry...");

        // Step 1: Send Hello
        let hello = Hello::new(self.key.did().to_string());

        let resp = self.client
            .post(format!("{}/hello", self.registry_url))
            .json(&hello)
            .send()
            .await
            .map_err(|e| format!("Failed to send Hello: {}", e))?;

        if !resp.status().is_success() {
            let error = resp.text().await.unwrap_or_default();
            return Err(format!("Hello failed: {}", error));
        }

        let challenge: Challenge = resp.json().await
            .map_err(|e| format!("Failed to parse Challenge: {}", e))?;

        info!("Received challenge, signing proof...");

        // Step 2: Create and send Proof
        let my_did = self.key.did();
        let proof = sign_proof(&challenge, &my_did, &self.key, None)
            .map_err(|e| format!("Failed to create proof: {}", e))?;

        let resp = self.client
            .post(format!("{}/proof", self.registry_url))
            .json(&proof)
            .send()
            .await
            .map_err(|e| format!("Failed to send Proof: {}", e))?;

        if !resp.status().is_success() {
            let error = resp.text().await.unwrap_or_default();
            return Err(format!("Proof failed: {}", error));
        }

        let accepted: ProofAcceptedResponse = resp.json().await
            .map_err(|e| format!("Failed to parse ProofAccepted: {}", e))?;

        info!("Authentication successful");

        *self.session.write().await = Some(accepted.session_id.clone());

        Ok(accepted.session_id)
    }

    async fn handle_register(&self, args: serde_json::Map<String, serde_json::Value>) -> Result<String, String> {
        let endpoint = args.get("endpoint")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing required parameter: endpoint".to_string())?;

        let session_id = self.authenticate().await?;

        #[derive(Serialize)]
        struct RegisterRequest { endpoint: String }

        let resp = self.client
            .post(format!("{}/register", self.registry_url))
            .header("Authorization", format!("Bearer {}", session_id))
            .json(&RegisterRequest { endpoint: endpoint.to_string() })
            .send()
            .await
            .map_err(|e| format!("Failed to send register: {}", e))?;

        if !resp.status().is_success() {
            let error: ErrorResponse = resp.json().await
                .unwrap_or(ErrorResponse { error: "Unknown error".to_string() });
            return Err(error.error);
        }

        Ok(format!("✓ Registered {} at endpoint: {}", self.key.did(), endpoint))
    }

    async fn handle_lookup(&self, args: serde_json::Map<String, serde_json::Value>) -> Result<String, String> {
        let did = args.get("did")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing required parameter: did".to_string())?;

        let encoded_did = urlencoding::encode(did);
        let resp = self.client
            .get(format!("{}/lookup/{}", self.registry_url, encoded_did))
            .send()
            .await
            .map_err(|e| format!("Failed to lookup: {}", e))?;

        if resp.status().as_u16() == 404 {
            return Err("Agent not found in registry".to_string());
        }

        if !resp.status().is_success() {
            let error: ErrorResponse = resp.json().await
                .unwrap_or(ErrorResponse { error: "Unknown error".to_string() });
            return Err(error.error);
        }

        let lookup: LookupResponse = resp.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(format!("✓ Found {}\n  Endpoint: {}", lookup.did, lookup.endpoint))
    }

    async fn handle_deregister(&self) -> Result<String, String> {
        let session_id = self.authenticate().await?;

        let resp = self.client
            .delete(format!("{}/deregister", self.registry_url))
            .header("Authorization", format!("Bearer {}", session_id))
            .send()
            .await
            .map_err(|e| format!("Failed to deregister: {}", e))?;

        if !resp.status().is_success() {
            let error: ErrorResponse = resp.json().await
                .unwrap_or(ErrorResponse { error: "Unknown error".to_string() });
            return Err(error.error);
        }

        *self.session.write().await = None;

        Ok(format!("✓ Deregistered {}", self.key.did()))
    }

    async fn handle_status(&self) -> Result<String, String> {
        let did = self.key.did().to_string();
        let encoded_did = urlencoding::encode(&did);

        let resp = self.client
            .get(format!("{}/lookup/{}", self.registry_url, encoded_did))
            .send()
            .await
            .map_err(|e| format!("Failed to check status: {}", e))?;

        if resp.status().as_u16() == 404 {
            return Ok(format!("○ Not registered\n  DID: {}", did));
        }

        if resp.status().is_success() {
            let lookup: LookupResponse = resp.json().await
                .map_err(|e| format!("Failed to parse response: {}", e))?;
            return Ok(format!("✓ Registered\n  DID: {}\n  Endpoint: {}", lookup.did, lookup.endpoint));
        }

        Err("Failed to check status".to_string())
    }

    async fn handle_whoami(&self) -> Result<String, String> {
        Ok(format!("Your DID: {}", self.key.did()))
    }
}

impl ServerHandler for ReachMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "agent-reach-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("Agent discovery registry MCP server".to_string()),
        }
    }

    fn list_tools(
        &self,
        _params: PaginatedRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        async move {
            let tools = vec![
                Tool {
                    name: "reach_register".into(),
                    description: "Register your endpoint in the discovery registry".into(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "endpoint": {"type": "string", "description": "Endpoint URL"}
                        },
                        "required": ["endpoint"]
                    }).as_object().cloned().unwrap().into(),
                },
                Tool {
                    name: "reach_lookup".into(),
                    description: "Look up an agent's endpoint by DID".into(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "did": {"type": "string", "description": "DID to look up"}
                        },
                        "required": ["did"]
                    }).as_object().cloned().unwrap().into(),
                },
                Tool {
                    name: "reach_deregister".into(),
                    description: "Remove your registration".into(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {}
                    }).as_object().cloned().unwrap().into(),
                },
                Tool {
                    name: "reach_status".into(),
                    description: "Check your registration status".into(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {}
                    }).as_object().cloned().unwrap().into(),
                },
                Tool {
                    name: "reach_whoami".into(),
                    description: "Show your DID".into(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {}
                    }).as_object().cloned().unwrap().into(),
                },
            ];
            Ok(ListToolsResult { tools, next_cursor: None })
        }
    }

    fn call_tool(
        &self,
        params: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        let this = self.clone();
        async move {
            let args = params.arguments.unwrap_or_default();

            let result = match params.name.as_ref() {
                "reach_register" => this.handle_register(args).await,
                "reach_lookup" => this.handle_lookup(args).await,
                "reach_deregister" => this.handle_deregister().await,
                "reach_status" => this.handle_status().await,
                "reach_whoami" => this.handle_whoami().await,
                _ => Err(format!("Unknown tool: {}", params.name)),
            };

            match result {
                Ok(text) => Ok(CallToolResult {
                    content: vec![Content::text(text)],
                    is_error: Some(false),
                }),
                Err(e) => Ok(CallToolResult {
                    content: vec![Content::text(e)],
                    is_error: Some(true),
                }),
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    info!("Starting agent-reach-mcp...");

    let key = load_identity().context(
        "Failed to load identity. Run agent-id-mcp and use 'identity_generate' first."
    )?;

    info!(did = %key.did(), "Loaded identity");

    let server = ReachMcpServer::new(key);

    info!("MCP server ready");

    let transport = stdio();
    let running = server.serve(transport).await?;
    running.waiting().await?;

    Ok(())
}
