mod config;
mod handler;
mod nonce_map;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use nostr_sdk::prelude::ToBech32;
use nostr_transport::filters::signer_filter;
use nostr_transport::relay::NostrRelay;

use crate::nonce_map::NonceMap;

#[derive(Parser)]
#[command(name = "signer-node", about = "FROST threshold signer node")]
struct Cli {
    /// Path to the signer configuration file
    #[arg(long, default_value = "signer.toml")]
    config: String,

    /// Prompt for approval before participating in each signing session
    #[arg(long)]
    interactive: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing (respects RUST_LOG env var)
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    tracing::info!(config_path = %cli.config, "loading configuration");

    // Load and validate configuration
    let cfg = config::load(&cli.config)?;

    let coord_bech32 = cfg.coordinator_pubkey.to_bech32().unwrap_or_default();
    tracing::info!(
        signer_id = cfg.signer_id,
        npub = %cfg.nostr_keys.public_key().to_bech32().unwrap_or_default(),
        coordinator = %&coord_bech32[..coord_bech32.len().min(20)],
        interactive = cli.interactive,
        "signer node starting"
    );

    // Connect to Nostr relays
    for url in &cfg.relay_urls {
        tracing::info!(relay_url = %url, "adding relay");
    }

    let relay = NostrRelay::new(cfg.nostr_keys.clone(), cfg.relay_urls.clone()).await
        .map_err(|e| anyhow::anyhow!("failed to create relay client: {e}"))?;

    relay.connect().await;

    // Subscribe to events addressed to this signer
    let filter = signer_filter(&cfg.nostr_keys.public_key());
    relay
        .subscribe(vec![filter])
        .await
        .map_err(|e| anyhow::anyhow!("failed to subscribe: {e}"))?;

    tracing::info!("connected to relays and subscribed");

    // Nonce storage with background cleanup
    let nonce_map = NonceMap::new();
    nonce_map.start_cleanup_task();
    tracing::debug!("nonce cleanup background task started");

    // Run the event loop (blocks until relay shuts down)
    handler::run_event_loop(
        &relay,
        &cfg.key_package,
        cfg.signer_id,
        &cfg.coordinator_pubkey,
        &nonce_map,
        cli.interactive,
    )
    .await?;

    relay
        .disconnect()
        .await
        .map_err(|e| anyhow::anyhow!("disconnect error: {e}"))?;

    Ok(())
}
