use anyhow::{Context, Result, bail};
use frost_secp256k1_tr::keys::KeyPackage;
use nostr_sdk::prelude::*;

use common::SignerConfig;
use frost_core::secp256k1::KeyPackageWrapper;

/// Fully resolved configuration ready for the signer event loop.
pub struct ResolvedConfig {
    pub key_package: KeyPackage,
    pub signer_id: u16,
    pub coordinator_pubkey: PublicKey,
    pub nostr_keys: Keys,
    pub relay_urls: Vec<String>,
}

pub fn load(path: &str) -> Result<ResolvedConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config file: {path}"))?;

    let cfg: SignerConfig = toml::from_str(&content)
        .with_context(|| format!("failed to parse config file: {path}"))?;

    // Parse key_package JSON → KeyPackageWrapper → live KeyPackage
    let wrapper: KeyPackageWrapper = serde_json::from_str(&cfg.key_package)
        .context("failed to parse key_package JSON")?;

    let key_package = wrapper
        .to_key_package()
        .map_err(|e| anyhow::anyhow!("failed to deserialize KeyPackage: {e}"))?;

    let signer_id = wrapper
        .to_identifier_u16()
        .map_err(|e| anyhow::anyhow!("failed to extract signer_id: {e}"))?;

    // Parse coordinator npub
    let coordinator_pubkey = PublicKey::from_bech32(&cfg.coordinator_npub)
        .map_err(|e| anyhow::anyhow!("invalid coordinator_npub: {e}"))?;

    // Nostr keys: parse nsec if present, otherwise generate fresh keys
    let nostr_keys = match cfg.nsec {
        Some(ref nsec) if !nsec.is_empty() => {
            let secret = SecretKey::from_bech32(nsec)
                .map_err(|e| anyhow::anyhow!("invalid nsec: {e}"))?;
            Keys::new(secret)
        }
        _ => {
            let keys = Keys::generate();
            tracing::warn!(
                npub = %keys.public_key().to_bech32().unwrap_or_default(),
                nsec = %keys.secret_key()
                    .map(|sk| sk.to_bech32().unwrap_or_default())
                    .unwrap_or_default(),
                "no nsec in config — generated ephemeral Nostr identity"
            );
            keys
        }
    };

    if cfg.relay_urls.is_empty() {
        bail!("relay_urls must not be empty");
    }

    let nsec_source = if cfg.nsec.as_ref().map_or(true, |s| s.is_empty()) {
        "generated"
    } else {
        "loaded"
    };

    tracing::info!(
        signer_id,
        coordinator_npub = %cfg.coordinator_npub,
        relay_count = cfg.relay_urls.len(),
        nsec_source,
        "configuration loaded successfully"
    );

    Ok(ResolvedConfig {
        key_package,
        signer_id,
        coordinator_pubkey,
        nostr_keys,
        relay_urls: cfg.relay_urls,
    })
}
