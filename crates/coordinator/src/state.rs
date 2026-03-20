use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use dashmap::DashMap;
use frost_secp256k1_tr::keys::PublicKeyPackage;
use nostr_sdk::prelude::*;
use nostr_sdk::RelayPoolNotification;
use tokio::sync::mpsc;
use uuid::Uuid;

use nostr_transport::relay::NostrRelay;

use crate::config::CoordinatorAppConfig;

pub struct SignerInfo {
    pub signer_id: u16,
    pub pubkey: PublicKey,
}

pub struct SessionHandle {
    pub event_tx: mpsc::Sender<Event>,
}

pub struct AppState {
    pub config: CoordinatorAppConfig,
    pub relay: NostrRelay,
    pub keys: Keys,
    pub sessions: DashMap<Uuid, SessionHandle>,
    pub serial_counter: AtomicU64,
    pub active_hashes: DashMap<String, Uuid>,
    pub public_key_package: PublicKeyPackage,
}

/// Extract session UUID from the "s" tag of a Nostr event.
fn extract_session_id(event: &Event) -> Option<Uuid> {
    event.tags.iter().find_map(|tag| {
        let v = tag.as_vec();
        if v.first().map(|s| s.as_str()) == Some("s") {
            v.get(1).and_then(|s| Uuid::parse_str(s).ok())
        } else {
            None
        }
    })
}

/// Spawn a background task that routes incoming Nostr events to per-session channels.
pub fn spawn_event_listener(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut notifications = state.relay.notifications();
        loop {
            match notifications.recv().await {
                Ok(RelayPoolNotification::Event { event, .. }) => {
                    let kind = event.kind().as_u16();
                    if let Some(session_id) = extract_session_id(&event) {
                        if let Some(handle) = state.sessions.get(&session_id) {
                            tracing::debug!(
                                session_id = %session_id,
                                kind,
                                "routing event to session"
                            );
                            if handle.event_tx.send((*event).clone()).await.is_err() {
                                tracing::debug!(
                                    session_id = %session_id,
                                    "session channel closed, dropping event"
                                );
                            }
                        } else {
                            tracing::debug!(
                                session_id = %session_id,
                                kind,
                                "event arrived for unknown session"
                            );
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("event listener lagged by {n} events");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    tracing::info!("notification channel closed, stopping listener");
                    break;
                }
                _ => {}
            }
        }
    });
}
