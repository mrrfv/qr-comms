use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{filter::EnvFilter, fmt::Subscriber as FmtSubscriber};

mod channel_manager;
mod handlers;
mod models;

use crate::channel_manager::ChannelManager;
use crate::handlers::{healthcheck, websocket_handler};
use crate::models::AppConfig;

#[derive(Clone)]
struct AppState {
    pub channel_manager: ChannelManager,
    pub config: AppConfig,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    // Set RUST_LOG environment variable to control logging levels, e.g., RUST_LOG=debug
    FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .compact()
        .init();

    // Initialize app configuration
    let config: AppConfig = envy::from_env().expect("Environment variables not set");

    let channel_manager = ChannelManager::new();
    let state = AppState {
        channel_manager,
        config: config.clone(),
    };

    // Build our application with routes
    let app = Router::new()
        .route("/health", get(healthcheck)) // Basic HTTP health check route
        .route("/ws/{channel_id}", get(websocket_handler)) // WebSocket route
        .with_state(state); // Share state across handlers

    // Run our app
    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.server_port))
        .await
        .expect("Failed to bind to address");
    info!(
        "listening on {}",
        listener.local_addr().expect("Failed to get local address")
    );
    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
