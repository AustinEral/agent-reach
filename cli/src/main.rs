//! agent-reach CLI - interact with agent-reach registry servers
//!
//! Usage:
//!   agent-reach auth <server>                     # Handshake, output session
//!   agent-reach register <server> -e <endpoint>   # Register endpoint
//!   agent-reach lookup <server> <did>             # Look up agent by DID
//!   agent-reach deregister <server>               # Remove registration

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use reqwest::Client;
use serde::Deserialize;
use std::path::PathBuf;

use agent_id::RootKey;
use agent_id_handshake::{
    messages::{Hello, ProofAccepted},
    protocol::sign_proof,
    Challenge,
};

#[derive(Parser)]
#[command(name = "agent-reach")]
#[command(about = "CLI client for agent-reach registry")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with a server (handshake), output session ID
    Auth {
        /// Server URL (e.g., http://localhost:3001)
        server: String,
        /// Path to identity file
        #[arg(short, long)]
        identity: Option<PathBuf>,
    },
    /// Register endpoint with a server
    Register {
        /// Server URL
        server: String,
        /// Endpoint where you can be reached
        #[arg(short, long)]
        endpoint: String,
        /// TTL in seconds
        #[arg(short, long, default_value = "3600")]
        ttl: u64,
        /// Session ID (or use SESSION env var)
        #[arg(short, long, env = "SESSION")]
        session: String,
    },
    /// Look up an agent by DID
    Lookup {
        /// Server URL
        server: String,
        /// DID to look up
        did: String,
    },
    /// Remove registration
    Deregister {
        /// Server URL
        server: String,
        /// Session ID (or use SESSION env var)
        #[arg(short, long, env = "SESSION")]
        session: String,
    },
}

// ============================================================================
// Identity loading (same format as agent-id CLI)
// ============================================================================

#[derive(Deserialize)]
struct StoredIdentity {
    #[serde(with = "hex_bytes")]
    secret_key: [u8; 32],
}

mod hex_bytes {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("invalid key length"))
    }
}

fn load_identity(path: Option<PathBuf>) -> Result<RootKey> {
    let path = match path {
        Some(p) => p,
        None => {
            let proj_dirs = directories::ProjectDirs::from("ai", "agent-id", "agent-id")
                .context("Could not determine config directory")?;
            proj_dirs.config_dir().join("identity.json")
        }
    };

    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("Could not read identity file: {}", path.display()))?;

    let stored: StoredIdentity =
        serde_json::from_str(&contents).context("Invalid identity file format")?;

    let root_key = RootKey::from_bytes(&stored.secret_key)?;
    Ok(root_key)
}

// ============================================================================
// Commands
// ============================================================================

async fn cmd_auth(server: String, identity: Option<PathBuf>) -> Result<()> {
    let key = load_identity(identity)?;
    let did = key.did();
    let client = Client::new();

    eprintln!("Authenticating with {}...", server);
    eprintln!("  DID: {}", did);

    // Hello
    let hello = Hello::new(did.to_string());
    let challenge: Challenge = client
        .post(format!("{}/hello", server))
        .json(&hello)
        .send()
        .await?
        .error_for_status()
        .context("Hello failed")?
        .json()
        .await?;

    eprintln!("  ✓ Received challenge");

    // Proof
    let proof = sign_proof(&challenge, &did, &key, Some(challenge.issuer.clone()))?;
    let accepted: ProofAccepted = client
        .post(format!("{}/proof", server))
        .json(&proof)
        .send()
        .await?
        .error_for_status()
        .context("Proof failed")?
        .json()
        .await?;

    eprintln!("  ✓ Handshake complete");
    eprintln!();

    // Output session ID to stdout
    println!("{}", accepted.session_id);

    Ok(())
}

#[derive(Deserialize)]
struct RegisterResponse {
    ok: bool,
    did: String,
}

async fn cmd_register(server: String, endpoint: String, ttl: u64, session: String) -> Result<()> {
    let client = Client::new();

    eprintln!("Registering endpoint...");
    eprintln!("  Endpoint: {}", endpoint);
    eprintln!("  TTL: {}s", ttl);

    let response: RegisterResponse = client
        .post(format!("{}/register", server))
        .header("Authorization", format!("Bearer {}", session))
        .json(&serde_json::json!({
            "endpoint": endpoint,
            "ttl": ttl
        }))
        .send()
        .await?
        .error_for_status()
        .context("Register failed")?
        .json()
        .await?;

    if response.ok {
        eprintln!("  ✓ Registered");
        println!("{}", response.did);
    } else {
        anyhow::bail!("Registration failed");
    }

    Ok(())
}

#[derive(Deserialize)]
struct LookupResponse {
    did: String,
    endpoint: String,
    status: String,
    expires_at: i64,
}

async fn cmd_lookup(server: String, did: String) -> Result<()> {
    let client = Client::new();

    let encoded_did = urlencoding::encode(&did);
    let response: LookupResponse = client
        .get(format!("{}/lookup/{}", server, encoded_did))
        .send()
        .await?
        .error_for_status()
        .context("Lookup failed")?
        .json()
        .await?;

    // Endpoint to stdout
    println!("{}", response.endpoint);

    // Details to stderr
    eprintln!("DID: {}", response.did);
    eprintln!("Status: {}", response.status);
    eprintln!("Expires: {}", response.expires_at);

    Ok(())
}

#[derive(Deserialize)]
struct DeregisterResponse {
    ok: bool,
}

async fn cmd_deregister(server: String, session: String) -> Result<()> {
    let client = Client::new();

    eprintln!("Deregistering...");

    let response: DeregisterResponse = client
        .post(format!("{}/deregister", server))
        .header("Authorization", format!("Bearer {}", session))
        .send()
        .await?
        .error_for_status()
        .context("Deregister failed")?
        .json()
        .await?;

    if response.ok {
        eprintln!("  ✓ Deregistered");
    } else {
        eprintln!("  (was not registered)");
    }

    Ok(())
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Auth { server, identity } => cmd_auth(server, identity).await,
        Commands::Register { server, endpoint, ttl, session } => {
            cmd_register(server, endpoint, ttl, session).await
        }
        Commands::Lookup { server, did } => cmd_lookup(server, did).await,
        Commands::Deregister { server, session } => cmd_deregister(server, session).await,
    }
}
