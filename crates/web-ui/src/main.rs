#[cfg(feature = "ssr")]
mod ssr {
    use std::sync::atomic::AtomicU64;
    use std::sync::Arc;

    use axum::routing::get;
    use clap::Parser;
    use dashmap::DashMap;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use nostr_sdk::prelude::*;
    use nostr_sdk::ToBech32;
    use tokio::net::TcpListener;

    use coordinator::config::load_config;
    use coordinator::frost_bridge::public_key_package_from_hex;
    use coordinator::routes as api_routes;
    use coordinator::state::{spawn_event_listener, AppState};

    use web_ui::app::{shell, App};

    #[derive(Parser)]
    #[command(name = "web-ui", about = "FROST threshold timestamp server with web UI")]
    struct Cli {
        /// Path to the coordinator TOML config file.
        #[arg(long, default_value = "configs/coordinator.toml")]
        config: String,
    }

    pub async fn run() -> anyhow::Result<()> {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "web_ui=info,coordinator=info".into()),
            )
            .init();

        let cli = Cli::parse();

        // Load coordinator config
        let config = load_config(&cli.config)?;
        tracing::info!(
            k = config.frost.k,
            n = config.frost.n,
            host = %config.coordinator.http_host,
            port = config.coordinator.http_port,
            "loaded config"
        );

        // Parse coordinator keys
        let keys = Keys::parse(&config.coordinator.nsec)
            .map_err(|e| anyhow::anyhow!("failed to parse nsec: {e}"))?;
        tracing::info!(npub = %keys.public_key().to_bech32()?, "coordinator identity");

        // Parse public key package
        let public_key_package = public_key_package_from_hex(&config.frost.public_key_package)?;

        // Connect to relays
        let relay = nostr_transport::relay::NostrRelay::new(keys.clone(), config.relays.urls.clone())
            .await
            .map_err(|e| anyhow::anyhow!("relay setup failed: {e}"))?;
        relay.connect().await;
        tracing::info!(relays = ?config.relays.urls, "connected to relays");

        // Subscribe to coordinator-bound events
        let filter = nostr_transport::filters::coordinator_filter(&keys.public_key(), None);
        relay
            .subscribe(vec![filter])
            .await
            .map_err(|e| anyhow::anyhow!("subscription failed: {e}"))?;
        tracing::info!("subscribed to coordinator-bound Nostr events");

        // Build shared state
        let bind_addr = format!("{}:{}", config.coordinator.http_host, config.coordinator.http_port);
        let state = Arc::new(AppState {
            config,
            relay,
            keys,
            sessions: DashMap::new(),
            serial_counter: AtomicU64::new(0),
            active_hashes: DashMap::new(),
            public_key_package,
        });

        // Spawn background event listener
        spawn_event_listener(state.clone());

        // Leptos configuration
        let leptos_options = LeptosOptions::builder()
            .output_name("web-ui")
            .site_root("target/site")
            .site_pkg_dir("pkg")
            .site_addr(bind_addr.parse().unwrap_or_else(|_| {
                std::net::SocketAddr::from(([0, 0, 0, 0], 8000))
            }))
            .build();

        let routes = generate_route_list(App);

        // Build API router (coordinator endpoints) - state consumed -> Router<()>
        let api_router = axum::Router::new()
            .route("/health", get(api_routes::health))
            .route("/api/v1/status", get(api_routes::get_status))
            .route("/api/v1/pubkey", get(api_routes::get_pubkey))
            .route(
                "/api/v1/timestamp",
                axum::routing::post(api_routes::post_timestamp),
            )
            .route(
                "/api/v1/verify",
                axum::routing::post(api_routes::post_verify),
            )
            .with_state(state);

        // Build Leptos router with state consumed -> Router<()>
        let leptos_router = axum::Router::new()
            .leptos_routes(&leptos_options, routes, {
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            })
            .fallback(leptos_axum::file_and_error_handler(shell))
            .with_state(leptos_options);

        // Merge: API first (takes priority), then Leptos as fallback
        let app = api_router.merge(leptos_router);

        // Start server
        let listener = TcpListener::bind(&bind_addr).await?;
        tracing::info!(addr = %bind_addr, "web-ui server listening");
        axum::serve(listener, app).await?;

        Ok(())
    }
}

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    ssr::run().await
}

#[cfg(not(feature = "ssr"))]
fn main() {
    // This binary requires the `ssr` feature.
}
