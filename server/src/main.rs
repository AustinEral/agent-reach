use std::net::SocketAddr;
use std::sync::Arc;

use axum::{routing::{get, post}, Router};
use clap::Parser;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod error;
mod handlers;
mod registry;
mod types;

use handlers::{AppState, HandshakeState};

#[derive(Parser)]
#[command(name = "agent-reach-server")]
#[command(about = "DID-based discovery registry server for AI agents")]
struct Cli {
    /// Port to listen on
    #[arg(short, long, default_value = "3001")]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "agent_reach_server=info,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create state
    let state = AppState {
        registry: registry::Registry::new(),
        handshake: Arc::new(HandshakeState::new()),
    };

    // Build router
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/hello", post(handlers::hello))
        .route("/proof", post(handlers::proof))
        .route("/register", post(handlers::register))
        .route("/deregister", post(handlers::deregister))
        .route("/lookup/:did", get(handlers::lookup))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    tracing::info!("agent-reach-server listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
