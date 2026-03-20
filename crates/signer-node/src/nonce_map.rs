use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use frost_core::secp256k1::Nonce;
use tokio::sync::RwLock;
use uuid::Uuid;

const TTL: Duration = Duration::from_secs(300); // 5 minutes
const CLEANUP_INTERVAL: Duration = Duration::from_secs(60);

struct NonceEntry {
    nonce: Nonce,
    created_at: Instant,
}

#[derive(Clone)]
pub struct NonceMap {
    inner: Arc<RwLock<HashMap<Uuid, NonceEntry>>>,
}

impl NonceMap {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a nonce for the given session. Returns `false` if a nonce for this
    /// session already exists (duplicate rejection).
    pub async fn insert(&self, session_id: Uuid, nonce: Nonce) -> bool {
        let mut map = self.inner.write().await;
        if map.contains_key(&session_id) {
            return false;
        }
        map.insert(
            session_id,
            NonceEntry {
                nonce,
                created_at: Instant::now(),
            },
        );
        tracing::debug!(%session_id, active_count = map.len(), "nonce inserted");
        true
    }

    /// Remove and return the nonce for the given session.
    /// Guarantees single-use: a second call with the same session_id returns `None`.
    pub async fn take(&self, session_id: &Uuid) -> Option<Nonce> {
        let mut map = self.inner.write().await;
        let result = map.remove(session_id);
        tracing::debug!(%session_id, found = result.is_some(), "nonce take");
        result.map(|entry| entry.nonce)
    }

    /// Returns the number of active nonces.
    pub async fn active_count(&self) -> usize {
        self.inner.read().await.len()
    }

    /// Spawn a background task that evicts entries older than 5 minutes every 60 seconds.
    pub fn start_cleanup_task(&self) {
        let inner = self.inner.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(CLEANUP_INTERVAL).await;
                let mut map = inner.write().await;
                let before = map.len();
                map.retain(|_, entry| entry.created_at.elapsed() < TTL);
                let evicted = before - map.len();
                tracing::trace!(evicted, remaining = map.len(), "nonce_map cleanup tick");
                if evicted > 0 {
                    tracing::debug!(evicted, remaining = map.len(), "nonce_map cleanup");
                }
            }
        });
    }
}
