use serde::{Deserialize, Serialize};

/// Registration request
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    /// The agent's DID (did:key:...)
    pub did: String,
    /// Where to reach this agent (any URI format)
    pub endpoint: String,
    /// Time-to-live in seconds (default: 3600)
    #[serde(default = "default_ttl")]
    pub ttl: u64,
    /// Signature of the registration payload
    pub signature: String,
}

fn default_ttl() -> u64 {
    3600
}

/// Registration response
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub ok: bool,
    pub did: String,
    pub expires_at: i64,
}

/// Lookup response
#[derive(Debug, Serialize)]
pub struct LookupResponse {
    pub did: String,
    pub endpoint: String,
    pub status: AgentStatus,
    pub registered_at: i64,
    pub expires_at: i64,
}

/// Deregistration request
#[derive(Debug, Deserialize)]
pub struct DeregisterRequest {
    pub did: String,
    pub signature: String,
}

/// Deregistration response
#[derive(Debug, Serialize)]
pub struct DeregisterResponse {
    pub ok: bool,
}

/// Agent status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Online,
    Expired,
}

/// Internal registry entry
#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub did: String,
    pub endpoint: String,
    pub registered_at: i64,
    pub expires_at: i64,
}

impl RegistryEntry {
    pub fn status(&self) -> AgentStatus {
        let now = chrono::Utc::now().timestamp();
        if now > self.expires_at {
            AgentStatus::Expired
        } else {
            AgentStatus::Online
        }
    }
}
