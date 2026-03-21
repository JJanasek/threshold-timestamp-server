use serde::Deserialize;

use crate::error::CollectorError;

#[derive(Debug, Clone, Deserialize)]
pub struct CollectorConfig {
    pub host: String,
    pub port: u16,
    pub max_events: usize,
}

pub fn load_config(path: &str) -> Result<CollectorConfig, CollectorError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| CollectorError::ConfigError(format!("failed to read config: {e}")))?;

    let config: CollectorConfig = toml::from_str(&content)
        .map_err(|e| CollectorError::ConfigError(format!("failed to parse config: {e}")))?;

    Ok(config)
}
