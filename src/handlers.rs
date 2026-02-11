use axum::{
    extract::{Path, State},
    Json,
};
use tracing::info;

use crate::error::ReachError;
use crate::registry::Registry;
use crate::types::*;

/// POST /register
/// 
/// Register an agent's current endpoint.
pub async fn register(
    State(registry): State<Registry>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, ReachError> {
    // Validate DID format
    if !req.did.starts_with("did:key:") {
        return Err(ReachError::InvalidDid);
    }

    // Verify signature
    // The signature should be over: did + endpoint + ttl
    let message = format!("{}:{}:{}", req.did, req.endpoint, req.ttl);
    verify_signature(&req.did, &message, &req.signature)?;

    // Calculate expiration
    let now = chrono::Utc::now().timestamp();
    let expires_at = now + req.ttl as i64;

    // Store in registry
    let entry = RegistryEntry {
        did: req.did.clone(),
        endpoint: req.endpoint,
        registered_at: now,
        expires_at,
    };
    registry.register(entry);

    info!(did = %req.did, "Agent registered");

    Ok(Json(RegisterResponse {
        ok: true,
        did: req.did,
        expires_at,
    }))
}

/// GET /lookup/:did
/// 
/// Look up an agent by DID.
pub async fn lookup(
    State(registry): State<Registry>,
    Path(did): Path<String>,
) -> Result<Json<LookupResponse>, ReachError> {
    // URL decode the DID (colons may be encoded)
    let did = urlencoding::decode(&did)
        .map_err(|_| ReachError::InvalidDid)?
        .into_owned();

    let entry = registry.lookup(&did).ok_or(ReachError::NotFound)?;

    let status = entry.status();
    if status == AgentStatus::Expired {
        return Err(ReachError::Expired);
    }

    Ok(Json(LookupResponse {
        did: entry.did,
        endpoint: entry.endpoint,
        status,
        registered_at: entry.registered_at,
        expires_at: entry.expires_at,
    }))
}

/// POST /deregister
/// 
/// Remove an agent's registration.
pub async fn deregister(
    State(registry): State<Registry>,
    Json(req): Json<DeregisterRequest>,
) -> Result<Json<DeregisterResponse>, ReachError> {
    // Validate DID format
    if !req.did.starts_with("did:key:") {
        return Err(ReachError::InvalidDid);
    }

    // Verify signature (sign the DID to prove ownership)
    verify_signature(&req.did, &req.did, &req.signature)?;

    // Remove from registry
    let existed = registry.deregister(&req.did);
    
    if existed {
        info!(did = %req.did, "Agent deregistered");
    }

    Ok(Json(DeregisterResponse { ok: existed }))
}

/// Verify a signature using agent-id
fn verify_signature(did: &str, message: &str, signature_b64: &str) -> Result<(), ReachError> {
    // Decode signature from base64
    let signature = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        signature_b64,
    )
    .map_err(|_| ReachError::InvalidSignature)?;

    // Use agent-id to verify
    // For now, we'll use the agent_id crate's verification
    agent_id::did::verify_signature(did, message.as_bytes(), &signature)
        .map_err(|_| ReachError::InvalidSignature)?;

    Ok(())
}
