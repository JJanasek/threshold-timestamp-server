use std::collections::VecDeque;

use tokio::sync::RwLock;

use common::CollectorEvent;

pub struct AppState {
    pub events: RwLock<VecDeque<CollectorEvent>>,
    pub max_events: usize,
}

impl AppState {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: RwLock::new(VecDeque::with_capacity(max_events)),
            max_events,
        }
    }

    pub async fn push(&self, event: CollectorEvent) {
        let mut events = self.events.write().await;
        if events.len() >= self.max_events {
            events.pop_front();
        }
        events.push_back(event);
    }

    pub async fn query(
        &self,
        node_name: Option<&str>,
        session_id: Option<&str>,
    ) -> Vec<CollectorEvent> {
        let events = self.events.read().await;
        events
            .iter()
            .filter(|e| {
                if let Some(name) = node_name {
                    if e.node_name != name {
                        return false;
                    }
                }
                if let Some(sid) = session_id {
                    match &e.session_id {
                        Some(s) if s == sid => {}
                        _ => return false,
                    }
                }
                true
            })
            .cloned()
            .collect()
    }
}
