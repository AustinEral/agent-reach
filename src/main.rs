use std::net::SocketAddr;
use std::sync::Arc;

use axum::{routing::{get, post}, Router};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod error;
mod handlers;
mod registry;
mod types;

use handlers::{AppState, HandshakeState};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "agent_reach=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create state
    let state = AppState {
        registry: registry::Registry::new(),
        handshake: Arc::new(HandshakeState::new()),
    };

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(|| async { "ok" }))
        // Handshake endpoints
        .route("/hello", post(handlers::hello))
        .route("/proof", post(handlers::proof))
        // Registration endpoints (require authenticated session)
        .route("/register", post(handlers::register))
        .route("/deregister", post(handlers::deregister))
        // Lookup (public)
        .route("/lookup/:did", get(handlers::lookup))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Run server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    tracing::info!("agent-reach listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
