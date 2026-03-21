use std::time::{SystemTime, UNIX_EPOCH};

use crate::CollectorEvent;

/// Fire-and-forget event emitter that sends audit events to the collector service.
pub struct EventEmitter {
    client: reqwest::Client,
    collector_url: Option<String>,
    node_name: String,
}

impl EventEmitter {
    /// Create an emitter that sends events to the given collector URL.
    pub fn new(collector_url: String, node_name: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            collector_url: Some(collector_url),
            node_name,
        }
    }

    /// Create a no-op emitter (when collector is not configured).
    pub fn noop(node_name: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            collector_url: None,
            node_name,
        }
    }

    /// Create an emitter from an optional URL.
    pub fn from_optional(collector_url: Option<String>, node_name: String) -> Self {
        match collector_url {
            Some(url) => Self::new(url, node_name),
            None => Self::noop(node_name),
        }
    }

    /// Emit an event. Spawns a background task; never blocks or fails the caller.
    pub fn emit(&self, session_id: Option<String>, message: String) {
        let Some(ref url) = self.collector_url else {
            return;
        };

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let event = CollectorEvent {
            node_name: self.node_name.clone(),
            session_id,
            message,
            timestamp,
        };

        let client = self.client.clone();
        let endpoint = format!("{}/api/v1/events", url);

        tokio::spawn(async move {
            if let Err(e) = client.post(&endpoint).json(&event).send().await {
                tracing::debug!(error = %e, "failed to send event to collector");
            }
        });
    }
}
