use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use clap::Parser;
use dashmap::DashMap;
use nostr_sdk::prelude::*;
use nostr_sdk::ToBech32;
use tokio::net::TcpListener;
use tokio::sync::RwLock;

use common::event_client::EventEmitter;
use nostr_transport::filters::{coordinator_filter, coordinator_dkg_filter};
use nostr_transport::relay::NostrRelay;

use coordinator::config::load_config;
use coordinator::frost_bridge::public_key_package_from_hex;
use coordinator::routes;
use coordinator::state::{spawn_event_listener, AppState};

#[derive(Parser)]
#[command(name = "coordinator", about = "FROST threshold timestamp coordinator")]
struct Cli {
    /// Path to the coordinator TOML config file.
    #[arg(long, default_value = "configs/coordinator.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "coordinator=info".into()),
        )
        .init();

    let cli = Cli::parse();

    // Load config
    let config = load_config(&cli.config)?;
    tracing::info!(
        k = config.frost.k,
        n = config.frost.n,
        host = %config.coordinator.http_host,
        port = config.coordinator.http_port,
        "loaded config"
    );

    // Parse coordinator keys from nsec
    let keys = Keys::parse(&config.coordinator.nsec)
        .map_err(|e| anyhow::anyhow!("failed to parse nsec: {e}"))?;
    tracing::info!(npub = %keys.public_key().to_bech32()?, "coordinator identity");

    // Parse public key package (optional -- absent before DKG)
    let public_key_package = match &config.frost.public_key_package {
        Some(hex_str) if !hex_str.is_empty() => {
            let pkg = public_key_package_from_hex(hex_str)?;
            tracing::info!("loaded existing public key package");
            Some(pkg)
        }
        _ => {
            tracing::info!("no public_key_package in config — DKG required before signing");
            None
        }
    };

    // Connect to relays
    let relay = NostrRelay::new(keys.clone(), config.relays.urls.clone())
        .await
        .map_err(|e| anyhow::anyhow!("relay setup failed: {e}"))?;
    relay.connect().await;
    tracing::info!(relays = ?config.relays.urls, "connected to relays");

    // Subscribe to coordinator-bound events (signing + DKG)
    let signing_filter = coordinator_filter(&keys.public_key(), None);
    let dkg_filter = coordinator_dkg_filter(&keys.public_key(), None);
    let _sub_id = relay
        .subscribe(vec![signing_filter, dkg_filter])
        .await
        .map_err(|e| anyhow::anyhow!("subscription failed: {e}"))?;
    tracing::info!("subscribed to coordinator-bound Nostr events (signing + DKG)");

    // Build shared state
    let bind_addr = format!("{}:{}", config.coordinator.http_host, config.coordinator.http_port);
    let event_emitter = EventEmitter::from_optional(
        config.coordinator.collector_url.clone(),
        "coordinator".to_string(),
    );
    if config.coordinator.collector_url.is_some() {
        tracing::info!("event collector enabled");
    }
    let state = Arc::new(AppState {
        config,
        relay,
        keys,
        sessions: DashMap::new(),
        serial_counter: AtomicU64::new(0),
        active_hashes: DashMap::new(),
        public_key_package: RwLock::new(public_key_package),
        event_emitter,
    });

    // Spawn background event listener
    spawn_event_listener(state.clone());

    // CORS layer (permissive for dev; tighten for production)
    let cors = tower_http::cors::CorsLayer::permissive();

    // Build HTTP router
    let router = axum::Router::new()
        .route("/health", axum::routing::get(routes::health))
        .route("/api/v1/status", axum::routing::get(routes::get_status))
        .route("/api/v1/pubkey", axum::routing::get(routes::get_pubkey))
        .route(
            "/api/v1/timestamp",
            axum::routing::post(routes::post_timestamp),
        )
        .route("/api/v1/verify", axum::routing::post(routes::post_verify))
        .route("/api/v1/dkg", axum::routing::post(routes::post_dkg))
        .layer(cors)
        .with_state(state);

    // Start server
    let listener = TcpListener::bind(&bind_addr).await?;
    tracing::info!(addr = %bind_addr, "HTTP server listening");
    axum::serve(listener, router).await?;

    Ok(())
}
