use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use nostr_sdk::ToBech32;
use serde::{Deserialize, Serialize};
use serde_json::json;

use common::TimestampToken;

use crate::dkg;
use crate::error::CoordinatorError;
use crate::frost_bridge;
use crate::session;
use crate::state::AppState;

// -- Request / Response types ------------------------------------------------

#[derive(Deserialize)]
pub struct TimestampRequest {
    pub hash: String,
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub token: TimestampToken,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub valid: bool,
}

// -- Status types ------------------------------------------------------------

#[derive(Serialize)]
pub struct StatusResponse {
    pub healthy: bool,
    pub active_sessions: usize,
    pub k: u16,
    pub n: u16,
    pub group_public_key: String,
    pub signers: Vec<StatusSigner>,
    pub relay_urls: Vec<String>,
}

#[derive(Serialize)]
pub struct StatusSigner {
    pub signer_id: u16,
    pub npub: String,
}

// -- Handlers ----------------------------------------------------------------

pub async fn get_status(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, CoordinatorError> {
    tracing::debug!("GET /api/v1/status");
    let pubkey_guard = state.public_key_package.read().await;
    let group_key = match pubkey_guard.as_ref() {
        Some(pkg) => frost_bridge::verifying_key_to_x_only_hex(pkg)?,
        None => String::new(),
    };
    drop(pubkey_guard);

    let signers: Vec<StatusSigner> = state
        .config
        .signers
        .iter()
        .map(|s| StatusSigner {
            signer_id: s.signer_id,
            npub: s.npub.clone(),
        })
        .collect();

    Ok(Json(StatusResponse {
        healthy: true,
        active_sessions: state.sessions.len(),
        k: state.config.frost.k,
        n: state.config.frost.n,
        group_public_key: group_key,
        signers,
        relay_urls: state.config.relays.urls.clone(),
    }))
}

pub async fn health() -> impl IntoResponse {
    tracing::debug!("GET /health");
    Json(json!({ "status": "ok" }))
}

pub async fn get_pubkey(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, CoordinatorError> {
    tracing::info!("GET /api/v1/pubkey");
    let pubkey_guard = state.public_key_package.read().await;
    let group_key = match pubkey_guard.as_ref() {
        Some(pkg) => frost_bridge::verifying_key_to_x_only_hex(pkg)?,
        None => String::new(),
    };
    drop(pubkey_guard);

    let coordinator_npub = state.keys.public_key().to_bech32().map_err(|e| {
        CoordinatorError::InternalError(format!("failed to encode npub: {e}"))
    })?;

    Ok(Json(json!({
        "group_public_key": group_key,
        "k": state.config.frost.k,
        "n": state.config.frost.n,
        "coordinator_npub": coordinator_npub,
    })))
}

// -- DKG types ---------------------------------------------------------------

#[derive(Serialize)]
pub struct DkgResponse {
    pub group_public_key: String,
    pub success: bool,
}

pub async fn post_dkg(
    State(state): State<Arc<AppState>>,
) -> Result<Json<DkgResponse>, CoordinatorError> {
    tracing::info!("POST /api/v1/dkg");

    let outcome = dkg::run_dkg_session(state).await?;

    Ok(Json(DkgResponse {
        group_public_key: outcome.group_public_key,
        success: true,
    }))
}

pub async fn post_timestamp(
    State(state): State<Arc<AppState>>,
    Json(body): Json<TimestampRequest>,
) -> Result<Json<TimestampToken>, CoordinatorError> {
    tracing::info!(hash = %body.hash, "POST /api/v1/timestamp");

    // Guard: check that DKG has been run
    {
        let pkg = state.public_key_package.read().await;
        if pkg.is_none() {
            return Err(CoordinatorError::NoDkgKeysYet);
        }
    }

    state.event_emitter.emit(
        None,
        format!("timestamp request received for hash {}", body.hash),
    );

    // Validate hash format: must be 64 hex chars (SHA-256)
    if body.hash.len() != 64 || hex::decode(&body.hash).is_err() {
        tracing::warn!(hash = %body.hash, "rejected: invalid hash format");
        return Err(CoordinatorError::BadRequest(
            "hash must be a 64-character hex-encoded SHA-256 digest".into(),
        ));
    }

    let start = std::time::Instant::now();
    let result = session::run_signing_session(state, body.hash.clone()).await;
    let elapsed = start.elapsed();

    match &result {
        Ok(token) => {
            tracing::info!(
                hash = %body.hash,
                serial = token.serial_number,
                elapsed_ms = elapsed.as_millis() as u64,
                "timestamp issued successfully"
            );
        }
        Err(e) => {
            tracing::error!(
                hash = %body.hash,
                elapsed_ms = elapsed.as_millis() as u64,
                error = %e,
                "timestamp request failed"
            );
        }
    }

    Ok(Json(result?))
}

pub async fn post_verify(
    Json(body): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, CoordinatorError> {
    tracing::info!(
        serial = body.token.serial_number,
        file_hash = %body.token.file_hash,
        "POST /api/v1/verify"
    );

    let valid = body
        .token
        .verify()
        .map_err(|e| CoordinatorError::BadRequest(format!("verification failed: {e}")))?;

    tracing::info!(serial = body.token.serial_number, valid, "verify result");

    Ok(Json(VerifyResponse { valid }))
}
