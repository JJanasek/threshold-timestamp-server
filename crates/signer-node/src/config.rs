use anyhow::{Context, Result, bail};
use frost_secp256k1_tr::keys::KeyPackage;
use nostr_sdk::prelude::*;

use common::SignerConfig;
use frost_core::secp256k1::KeyPackageWrapper;

/// Fully resolved configuration ready for the signer event loop.
pub struct ResolvedConfig {
    pub key_package: Option<KeyPackage>,
    pub signer_id: Option<u16>,
    pub coordinator_pubkey: PublicKey,
    pub nostr_keys: Keys,
    pub relay_urls: Vec<String>,
    pub collector_url: Option<String>,
}

pub fn load(path: &str) -> Result<ResolvedConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config file: {path}"))?;

    let cfg: SignerConfig = toml::from_str(&content)
        .with_context(|| format!("failed to parse config file: {path}"))?;

    // Parse key_package if present (absent before DKG)
    let (key_package, signer_id) = if let Some(ref kp_str) = cfg.key_package {
        let wrapper: KeyPackageWrapper = serde_json::from_str(kp_str)
            .context("failed to parse key_package JSON")?;

        let key_package = wrapper
            .to_key_package()
            .map_err(|e| anyhow::anyhow!("failed to deserialize KeyPackage: {e}"))?;

        let id = cfg.signer_id.unwrap_or_else(|| {
            wrapper.to_identifier_u16().unwrap_or(0)
        });

        (Some(key_package), Some(id))
    } else {
        (None, cfg.signer_id)
    };

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

    if key_package.is_some() {
        tracing::info!(
            signer_id = ?signer_id,
            coordinator_npub = %cfg.coordinator_npub,
            relay_count = cfg.relay_urls.len(),
            nsec_source,
            "configuration loaded successfully"
        );
    } else {
        tracing::info!(
            coordinator_npub = %cfg.coordinator_npub,
            relay_count = cfg.relay_urls.len(),
            nsec_source,
            "configuration loaded (DKG-only mode — no key_package)"
        );
    }

    Ok(ResolvedConfig {
        key_package,
        signer_id,
        coordinator_pubkey,
        nostr_keys,
        relay_urls: cfg.relay_urls,
        collector_url: cfg.collector_url,
    })
}

/// Persist DKG results back to the signer config file.
/// Reads the current key_package from the shared SigningIdentity lock.
pub async fn save_dkg_result(
    config_path: &str,
    signer_id: u16,
    identity: &std::sync::Arc<tokio::sync::RwLock<crate::handler::SigningIdentity>>,
) -> Result<()> {
    let id = identity.read().await;
    let key_package = id.key_package.as_ref()
        .ok_or_else(|| anyhow::anyhow!("no key_package in identity to save"))?;

    let content = std::fs::read_to_string(config_path)
        .with_context(|| format!("failed to read config file for update: {config_path}"))?;

    let mut cfg: SignerConfig = toml::from_str(&content)
        .with_context(|| format!("failed to parse config file for update: {config_path}"))?;

    // Serialize KeyPackage to KeyPackageWrapper JSON
    let kp_bytes = key_package.serialize()
        .map_err(|e| anyhow::anyhow!("failed to serialize KeyPackage: {e}"))?;
    let id_bytes = key_package.identifier().serialize();
    let vk_bytes = key_package.verifying_key().serialize()
        .map_err(|e| anyhow::anyhow!("failed to serialize verifying key: {e}"))?;

    let wrapper = KeyPackageWrapper {
        identifier: hex::encode(id_bytes),
        secret_share: hex::encode(&kp_bytes),
        public_key: hex::encode(&vk_bytes),
    };

    let wrapper_json = serde_json::to_string(&wrapper)
        .context("failed to serialize KeyPackageWrapper")?;

    cfg.key_package = Some(wrapper_json);
    cfg.signer_id = Some(signer_id);

    let toml_str = toml::to_string_pretty(&cfg)
        .context("failed to serialize config to TOML")?;

    std::fs::write(config_path, toml_str)
        .with_context(|| format!("failed to write config file: {config_path}"))?;

    tracing::info!(config_path, signer_id, "DKG result saved to config");

    Ok(())
}
