use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use parking_lot::RwLock;
use tracing::info;

use agent_id::RootKey;
use agent_id_handshake::{
    messages::{Hello, Proof, ProofAccepted},
    protocol::Verifier,
    Challenge,
};

use crate::error::ReachError;
use crate::registry::Registry;
use crate::types::*;

/// Shared state for handshake sessions
pub struct HandshakeState {
    /// agent-reach's own identity
    pub key: RootKey,
    /// Pending challenges (challenge_hash -> (challenge, verifier))
    pub pending_challenges: RwLock<HashMap<String, (Challenge, Verifier)>>,
    /// Authenticated sessions (session_id -> did)
    pub sessions: RwLock<HashMap<String, AuthenticatedSession>>,
}

#[derive(Clone)]
pub struct AuthenticatedSession {
    pub did: String,
    pub created_at: i64,
}

impl HandshakeState {
    pub fn new() -> Self {
        // Generate identity for agent-reach service
        // TODO: Load from file for persistence
        let key = RootKey::generate();
        info!(did = %key.did(), "agent-reach identity generated");
        
        Self {
            key,
            pending_challenges: RwLock::new(HashMap::new()),
            sessions: RwLock::new(HashMap::new()),
        }
    }
}

/// App state combining registry and handshake state
#[derive(Clone)]
pub struct AppState {
    pub registry: Registry,
    pub handshake: Arc<HandshakeState>,
}

// ============================================================================
// Handshake Endpoints
// ============================================================================

/// POST /hello
/// 
/// First step of handshake. Returns a challenge.
pub async fn hello(
    State(state): State<AppState>,
    Json(hello): Json<Hello>,
) -> Result<Json<Challenge>, ReachError> {
    info!(did = %hello.did, "Received Hello");

    // Parse and validate DID
    let did: agent_id::Did = hello.did.parse()
        .map_err(|_| ReachError::InvalidDid)?;

    // Create verifier and generate challenge
    let verifier = Verifier::new(did);
    let challenge = verifier.handle_hello(&hello)
        .map_err(|e| ReachError::HandshakeError(e.to_string()))?;

    // Store challenge and verifier for verification
    let challenge_hash = agent_id_handshake::protocol::hash_challenge(&challenge)
        .map_err(|e| ReachError::Internal(e.to_string()))?;
    
    state.handshake.pending_challenges.write()
        .insert(challenge_hash, (challenge.clone(), verifier));

    info!(did = %hello.did, "Sent Challenge");

    Ok(Json(challenge))
}

/// POST /proof
/// 
/// Second step of handshake. Verifies proof, returns ProofAccepted with session.
pub async fn proof(
    State(state): State<AppState>,
    Json(proof): Json<Proof>,
) -> Result<Json<ProofAccepted>, ReachError> {
    info!(did = %proof.responder_did, "Received Proof");

    // Get the pending challenge and verifier
    let (challenge, verifier) = state.handshake.pending_challenges.write()
        .remove(&proof.challenge_hash)
        .ok_or(ReachError::InvalidChallenge)?;

    // Verify the proof
    verifier.verify_proof(&proof, &challenge)
        .map_err(|e| ReachError::HandshakeError(e.to_string()))?;

    info!(did = %proof.responder_did, "Proof verified");

    // Accept proof and generate counter-proof (mutual auth)
    let accepted = verifier.accept_proof(&proof, &state.handshake.key)
        .map_err(|e| ReachError::HandshakeError(e.to_string()))?;

    // Store authenticated session
    let session = AuthenticatedSession {
        did: proof.responder_did.clone(),
        created_at: chrono::Utc::now().timestamp(),
    };
    state.handshake.sessions.write()
        .insert(accepted.session_id.clone(), session);

    info!(did = %proof.responder_did, session = %accepted.session_id, "Session created");

    Ok(Json(accepted))
}

// ============================================================================
// Registration Endpoints (require authenticated session)
// ============================================================================

/// Extract session from Authorization header
fn get_session(headers: &HeaderMap, state: &AppState) -> Result<AuthenticatedSession, ReachError> {
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(ReachError::Unauthorized)?;

    let session_id = auth
        .strip_prefix("Bearer ")
        .ok_or(ReachError::Unauthorized)?;

    let sessions = state.handshake.sessions.read();
    let session = sessions
        .get(session_id)
        .cloned()
        .ok_or(ReachError::Unauthorized)?;

    // Check session age (expire after 5 minutes)
    let now = chrono::Utc::now().timestamp();
    if now - session.created_at > 300 {
        return Err(ReachError::SessionExpired);
    }

    Ok(session)
}

/// POST /register
/// 
/// Register endpoint for authenticated agent.
pub async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, ReachError> {
    // Verify session
    let session = get_session(&headers, &state)?;

    info!(did = %session.did, endpoint = %req.endpoint, "Registering endpoint");

    // Calculate expiration
    let now = chrono::Utc::now().timestamp();
    let expires_at = now + req.ttl as i64;

    // Store in registry
    let entry = RegistryEntry {
        did: session.did.clone(),
        endpoint: req.endpoint,
        registered_at: now,
        expires_at,
    };
    state.registry.register(entry);

    info!(did = %session.did, "Agent registered");

    Ok(Json(RegisterResponse {
        ok: true,
        did: session.did,
        expires_at,
    }))
}

/// GET /lookup/:did
/// 
/// Look up an agent by DID. No authentication required.
pub async fn lookup(
    State(state): State<AppState>,
    Path(did): Path<String>,
) -> Result<Json<LookupResponse>, ReachError> {
    // URL decode the DID
    let did = urlencoding::decode(&did)
        .map_err(|_| ReachError::InvalidDid)?
        .into_owned();

    let entry = state.registry.lookup(&did).ok_or(ReachError::NotFound)?;

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
/// Remove registration. Requires authenticated session.
pub async fn deregister(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<DeregisterResponse>, ReachError> {
    let session = get_session(&headers, &state)?;

    let existed = state.registry.deregister(&session.did);
    
    if existed {
        info!(did = %session.did, "Agent deregistered");
    }

    Ok(Json(DeregisterResponse { ok: existed }))
}
