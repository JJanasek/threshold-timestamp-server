mod config;
mod error;
mod routes;
mod state;

use std::sync::Arc;

use clap::Parser;
use tokio::net::TcpListener;

use crate::config::load_config;
use crate::state::AppState;

#[derive(Parser)]
#[command(name = "collector", about = "Event collector service for audit logging")]
struct Cli {
    /// Path to the collector TOML config file.
    #[arg(long, default_value = "configs/collector.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "collector=info".into()),
        )
        .init();

    let cli = Cli::parse();

    let config = load_config(&cli.config)?;
    tracing::info!(
        host = %config.host,
        port = config.port,
        max_events = config.max_events,
        "loaded config"
    );

    let bind_addr = format!("{}:{}", config.host, config.port);
    let state = Arc::new(AppState::new(config.max_events));

    let cors = tower_http::cors::CorsLayer::permissive();

    let router = axum::Router::new()
        .route("/health", axum::routing::get(routes::health))
        .route(
            "/api/v1/events",
            axum::routing::get(routes::get_events).post(routes::post_event),
        )
        .layer(cors)
        .with_state(state);

    let listener = TcpListener::bind(&bind_addr).await?;
    tracing::info!(addr = %bind_addr, "collector listening");
    axum::serve(listener, router).await?;

    Ok(())
}
