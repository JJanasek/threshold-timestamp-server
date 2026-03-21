use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use common::CollectorEvent;

use crate::error::CollectorError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct EventQuery {
    pub node_name: Option<String>,
    pub session_id: Option<String>,
}

pub async fn post_event(
    State(state): State<Arc<AppState>>,
    Json(event): Json<CollectorEvent>,
) -> Result<impl IntoResponse, CollectorError> {
    tracing::debug!(
        node = %event.node_name,
        message = %event.message,
        "received event"
    );
    state.push(event).await;
    Ok(Json(json!({ "status": "ok" })))
}

pub async fn get_events(
    State(state): State<Arc<AppState>>,
    Query(query): Query<EventQuery>,
) -> Result<impl IntoResponse, CollectorError> {
    let events = state
        .query(query.node_name.as_deref(), query.session_id.as_deref())
        .await;
    Ok(Json(events))
}

pub async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}
