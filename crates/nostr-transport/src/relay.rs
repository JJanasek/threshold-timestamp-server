use nostr_sdk::prelude::*;
use nostr_sdk::RelayPoolNotification;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RelayError {
    #[error("client error: {0}")]
    Client(String),
}

impl From<nostr_sdk::client::Error> for RelayError {
    fn from(e: nostr_sdk::client::Error) -> Self {
        RelayError::Client(e.to_string())
    }
}

/// Thin wrapper around [`nostr_sdk::Client`] scoped to our use-case.
pub struct NostrRelay {
    client: Client,
    keys: Keys,
}

impl NostrRelay {
    /// Create a new relay pool client with the given identity and relay URLs.
    pub async fn new(keys: Keys, relay_urls: Vec<String>) -> Result<Self, RelayError> {
        let client = Client::new(&keys);
        for url in relay_urls {
            client.add_relay(url).await.map_err(|e| RelayError::Client(e.to_string()))?;
        }
        Ok(Self { client, keys })
    }

    /// Connect to all configured relays.
    pub async fn connect(&self) {
        self.client.connect().await;
    }

    /// Subscribe to events matching `filters`.
    pub async fn subscribe(
        &self,
        filters: Vec<Filter>,
    ) -> Result<SubscriptionId, RelayError> {
        let output = self
            .client
            .subscribe(filters, None)
            .await
            .map_err(|e| RelayError::Client(e.to_string()))?;
        Ok(output.val)
    }

    /// Sign and send an event builder to all connected relays.
    pub async fn send_event_builder(
        &self,
        builder: EventBuilder,
    ) -> Result<EventId, RelayError> {
        let output = self
            .client
            .send_event_builder(builder)
            .await
            .map_err(|e| RelayError::Client(e.to_string()))?;
        Ok(output.val)
    }

    /// Get a receiver for relay pool notifications (new events, messages, etc.).
    pub fn notifications(&self) -> tokio::sync::broadcast::Receiver<RelayPoolNotification> {
        self.client.notifications()
    }

    /// The identity keys this client is using.
    pub fn keys(&self) -> &Keys {
        &self.keys
    }

    /// Convenience: the public key of this client.
    pub fn public_key(&self) -> PublicKey {
        self.keys.public_key()
    }

    /// Disconnect from all relays.
    pub async fn disconnect(&self) -> Result<(), RelayError> {
        self.client
            .disconnect()
            .await
            .map_err(|e| RelayError::Client(e.to_string()))?;
        Ok(())
    }
}
