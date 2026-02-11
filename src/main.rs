use std::net::SocketAddr;

use axum::{routing::get, Router};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod error;
mod handlers;
mod registry;
mod types;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "agent_reach=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create registry
    let registry = registry::Registry::new();

    // Build router
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/register", axum::routing::post(handlers::register))
        .route("/lookup/:did", get(handlers::lookup))
        .route("/deregister", axum::routing::post(handlers::deregister))
        .layer(TraceLayer::new_for_http())
        .with_state(registry);

    // Run server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("agent-reach listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
