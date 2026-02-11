//! agent-reach-mcp: MCP server for agent-reach discovery registry
//!
//! Provides tools for agents to register, lookup, and manage their
//! presence in the agent-reach discovery registry.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use rmcp::{
    ServerHandler, ServiceExt,
    model::{ServerCapabilities, ServerInfo, Implementation},
    schemars, tool,
    transport::stdio,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, error};

use agent_id::RootKey;
use agent_id_handshake::protocol::Prover;

/// Default registry URL
const DEFAULT_REGISTRY_URL: &str = "https://reach.agent-id.ai";

/// Identity file location (same as agent-id-mcp)
fn identity_path() -> PathBuf {
    directories::ProjectDirs::from("ai", "agent-id", "agent-id")
        .map(|dirs| dirs.config_dir().join("identity.json"))
        .unwrap_or_else(|| PathBuf::from("~/.config/agent-id/identity.json"))
}

/// Stored identity format
#[derive(Serialize, Deserialize)]
struct StoredIdentity {
    secret_key: String,
}

/// Load identity from disk
fn load_identity() -> Result<RootKey> {
    let path = identity_path();
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read identity from {:?}", path))?;
    let stored: StoredIdentity = serde_json::from_str(&content)
        .context("Failed to parse identity file")?;
    let key = RootKey::from_secret_key_base64(&stored.secret_key)
        .context("Failed to load key from secret")?;
    Ok(key)
}

/// MCP Server state
struct ReachMcpServer {
    /// Agent's identity
    key: RootKey,
    /// HTTP client
    client: reqwest::Client,
    /// Registry URL
    registry_url: String,
    /// Current session (after successful auth)
    session: RwLock<Option<AuthSession>>,
}

#[derive(Clone)]
struct AuthSession {
    session_id: String,
    #[allow(dead_code)]
    did: String,
}

impl ReachMcpServer {
    fn new(key: RootKey) -> Self {
        Self {
            key,
            client: reqwest::Client::new(),
            registry_url: std::env::var("REACH_REGISTRY_URL")
                .unwrap_or_else(|_| DEFAULT_REGISTRY_URL.to_string()),
            session: RwLock::new(None),
        }
    }

    /// Perform handshake authentication, returns session_id
    async fn authenticate(&self) -> Result<String> {
        // Check if we have a valid session
        if let Some(session) = self.session.read().await.as_ref() {
            return Ok(session.session_id.clone());
        }

        info!("Authenticating with registry...");

        // Step 1: Send Hello
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as i64;

        let hello = serde_json::json!({
            "type": "hello",
            "version": "1.0",
            "did": self.key.did().to_string(),
            "protocols": [],
            "timestamp": timestamp
        });

        let resp = self.client
            .post(format!("{}/hello", self.registry_url))
            .json(&hello)
            .send()
            .await
            .context("Failed to send Hello")?;

        if !resp.status().is_success() {
            let error = resp.text().await.unwrap_or_default();
            anyhow::bail!("Hello failed: {}", error);
        }

        let challenge: ChallengeResponse = resp.json().await
            .context("Failed to parse Challenge")?;

        info!("Received challenge, signing proof...");

        // Step 2: Create and send Proof
        // Convert our challenge response to the format Prover expects
        let handshake_challenge = agent_id_handshake::Challenge {
            msg_type: challenge.r#type.clone(),
            version: challenge.version.clone(),
            nonce: challenge.nonce.clone(),
            timestamp: challenge.timestamp,
            audience: challenge.audience.clone(),
            issuer: challenge.issuer.clone(),
        };

        let prover = Prover::new(self.key.clone());
        let proof = prover.create_proof(&handshake_challenge)
            .context("Failed to create proof")?;

        let resp = self.client
            .post(format!("{}/proof", self.registry_url))
            .json(&proof)
            .send()
            .await
            .context("Failed to send Proof")?;

        if !resp.status().is_success() {
            let error = resp.text().await.unwrap_or_default();
            anyhow::bail!("Proof failed: {}", error);
        }

        let accepted: ProofAcceptedResponse = resp.json().await
            .context("Failed to parse ProofAccepted")?;

        info!("Authentication successful");

        // Store session
        let session = AuthSession {
            session_id: accepted.session_id.clone(),
            did: self.key.did().to_string(),
        };
        *self.session.write().await = Some(session);

        Ok(accepted.session_id)
    }
}

#[derive(Deserialize)]
struct ChallengeResponse {
    r#type: String,
    version: String,
    nonce: String,
    timestamp: i64,
    audience: String,
    issuer: String,
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

// ============================================================================
// MCP Tool Handlers
// ============================================================================

#[tool(
    name = "reach_register",
    description = "Register your agent's endpoint in the discovery registry. Other agents will be able to find you at this endpoint."
)]
async fn reach_register(
    #[doc = "The endpoint URL where your agent can be reached (e.g., https://example.com/agent/inbox)"]
    endpoint: String,
    #[tool(aggr)] server: Arc<ReachMcpServer>,
) -> String {
    match register_impl(&server, &endpoint).await {
        Ok(()) => format!("✓ Registered {} at endpoint: {}", server.key.did(), endpoint),
        Err(e) => format!("✗ Registration failed: {}", e),
    }
}

async fn register_impl(server: &ReachMcpServer, endpoint: &str) -> Result<()> {
    let session_id = server.authenticate().await?;

    #[derive(Serialize)]
    struct RegisterRequest {
        endpoint: String,
    }

    let resp = server.client
        .post(format!("{}/register", server.registry_url))
        .header("Authorization", format!("Bearer {}", session_id))
        .json(&RegisterRequest { endpoint: endpoint.to_string() })
        .send()
        .await
        .context("Failed to send register request")?;

    if !resp.status().is_success() {
        let error: ErrorResponse = resp.json().await
            .unwrap_or(ErrorResponse { error: "Unknown error".to_string() });
        anyhow::bail!("{}", error.error);
    }

    Ok(())
}

#[tool(
    name = "reach_lookup",
    description = "Look up another agent's endpoint by their DID. Returns the endpoint URL where they can be reached."
)]
async fn reach_lookup(
    #[doc = "The DID of the agent to look up (e.g., did:key:z6Mk...)"]
    did: String,
    #[tool(aggr)] server: Arc<ReachMcpServer>,
) -> String {
    match lookup_impl(&server, &did).await {
        Ok(endpoint) => format!("✓ Found {}\n  Endpoint: {}", did, endpoint),
        Err(e) => format!("✗ Lookup failed: {}", e),
    }
}

async fn lookup_impl(server: &ReachMcpServer, did: &str) -> Result<String> {
    let encoded_did = urlencoding::encode(did);
    let resp = server.client
        .get(format!("{}/lookup/{}", server.registry_url, encoded_did))
        .send()
        .await
        .context("Failed to send lookup request")?;

    if resp.status().as_u16() == 404 {
        anyhow::bail!("Agent not found in registry");
    }

    if !resp.status().is_success() {
        let error: ErrorResponse = resp.json().await
            .unwrap_or(ErrorResponse { error: "Unknown error".to_string() });
        anyhow::bail!("{}", error.error);
    }

    let lookup: LookupResponse = resp.json().await
        .context("Failed to parse lookup response")?;

    Ok(lookup.endpoint)
}

#[tool(
    name = "reach_deregister",
    description = "Remove your agent's registration from the discovery registry. Other agents will no longer be able to find you."
)]
async fn reach_deregister(
    #[tool(aggr)] server: Arc<ReachMcpServer>,
) -> String {
    match deregister_impl(&server).await {
        Ok(()) => format!("✓ Deregistered {}", server.key.did()),
        Err(e) => format!("✗ Deregistration failed: {}", e),
    }
}

async fn deregister_impl(server: &ReachMcpServer) -> Result<()> {
    let session_id = server.authenticate().await?;

    let resp = server.client
        .delete(format!("{}/deregister", server.registry_url))
        .header("Authorization", format!("Bearer {}", session_id))
        .send()
        .await
        .context("Failed to send deregister request")?;

    if !resp.status().is_success() {
        let error: ErrorResponse = resp.json().await
            .unwrap_or(ErrorResponse { error: "Unknown error".to_string() });
        anyhow::bail!("{}", error.error);
    }

    // Clear session
    *server.session.write().await = None;

    Ok(())
}

#[tool(
    name = "reach_status",
    description = "Check your current registration status in the discovery registry."
)]
async fn reach_status(
    #[tool(aggr)] server: Arc<ReachMcpServer>,
) -> String {
    let did = server.key.did().to_string();
    
    match lookup_impl(&server, &did).await {
        Ok(endpoint) => format!("✓ Registered\n  DID: {}\n  Endpoint: {}", did, endpoint),
        Err(_) => format!("○ Not registered\n  DID: {}", did),
    }
}

#[tool(
    name = "reach_whoami",
    description = "Show your agent's DID (decentralized identifier) used for the registry."
)]
async fn reach_whoami(
    #[tool(aggr)] server: Arc<ReachMcpServer>,
) -> String {
    format!("Your DID: {}", server.key.did())
}

// ============================================================================
// MCP Server Implementation
// ============================================================================

#[derive(Clone)]
struct ReachMcpHandler {
    server: Arc<ReachMcpServer>,
}

impl ServerHandler for ReachMcpHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            name: "agent-reach-mcp".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            instructions: Some(
                "MCP server for agent-reach discovery registry. \
                 Allows agents to register their endpoints, look up other agents, \
                 and manage their presence in the registry.".to_string()
            ),
            ..Default::default()
        }
    }

    fn get_capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            tools: Some(rmcp::model::ToolsCapability::default()),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .with_writer(std::io::stderr)
        .init();

    info!("Starting agent-reach-mcp...");

    // Load identity
    let key = load_identity().context(
        "Failed to load identity. Run agent-id-mcp and use 'generate_identity' first."
    )?;
    
    info!(did = %key.did(), "Loaded identity");

    // Create server
    let server = Arc::new(ReachMcpServer::new(key));
    let handler = ReachMcpHandler { server: server.clone() };

    // Register tools and run
    let service = handler
        .serve(reach_register)
        .serve(reach_lookup)
        .serve(reach_deregister)
        .serve(reach_status)
        .serve(reach_whoami);

    info!("MCP server ready");

    // Run on stdio
    let transport = stdio::StdioTransport::new();
    rmcp::serve(service, transport).await?;

    Ok(())
}
