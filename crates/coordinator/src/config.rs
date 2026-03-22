use serde::Deserialize;

use crate::error::CoordinatorError;

#[derive(Debug, Clone, Deserialize)]
pub struct CoordinatorAppConfig {
    pub coordinator: CoordinatorSection,
    pub frost: FrostSection,
    pub signers: Vec<SignerEntry>,
    pub relays: RelaySection,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoordinatorSection {
    pub nsec: String,
    pub http_host: String,
    pub http_port: u16,
    pub collector_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FrostSection {
    pub k: u16,
    pub n: u16,
    /// Hex-encoded serialized `frost_secp256k1_tr::keys::PublicKeyPackage`.
    /// Optional: absent before DKG has been run.
    pub public_key_package: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SignerEntry {
    pub npub: String,
    pub signer_id: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RelaySection {
    pub urls: Vec<String>,
}

pub fn load_config(path: &str) -> Result<CoordinatorAppConfig, CoordinatorError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| CoordinatorError::ConfigError(format!("failed to read config: {e}")))?;

    let config: CoordinatorAppConfig = toml::from_str(&content)
        .map_err(|e| CoordinatorError::ConfigError(format!("failed to parse config: {e}")))?;

    // Validate thresholds
    if config.frost.k < 1 || config.frost.k > config.frost.n {
        return Err(CoordinatorError::ConfigError(format!(
            "invalid threshold: k={} must satisfy 1 <= k <= n={}",
            config.frost.k, config.frost.n
        )));
    }

    if (config.signers.len() as u16) < config.frost.n {
        return Err(CoordinatorError::ConfigError(format!(
            "not enough signers: have {}, need at least n={}",
            config.signers.len(),
            config.frost.n
        )));
    }

    Ok(config)
}
