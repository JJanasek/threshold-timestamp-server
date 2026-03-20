use serde::{Deserialize, Serialize};

// -- Shared API types (used by both client & server) -------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub healthy: bool,
    pub active_sessions: usize,
    pub k: u16,
    pub n: u16,
    pub group_public_key: String,
    pub signers: Vec<StatusSigner>,
    pub relay_urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusSigner {
    pub signer_id: u16,
    pub npub: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubkeyResponse {
    pub group_public_key: String,
    pub k: u16,
    pub n: u16,
    pub coordinator_npub: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampRequest {
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyRequest {
    pub token: common::TimestampToken,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResponse {
    pub valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
}

// -- Client-side fetch wrappers (hydrate only) -------------------------------

#[cfg(feature = "hydrate")]
pub mod client {
    use super::*;
    use gloo_net::http::Request;

    pub async fn get_status() -> Result<StatusResponse, String> {
        let resp = Request::get("/api/v1/status")
            .send()
            .await
            .map_err(|e| format!("Network error: {e}"))?;
        if resp.ok() {
            resp.json().await.map_err(|e| format!("Parse error: {e}"))
        } else {
            let err: ApiError = resp
                .json()
                .await
                .unwrap_or(ApiError { error: "Unknown error".into() });
            Err(err.error)
        }
    }

    pub async fn get_pubkey() -> Result<PubkeyResponse, String> {
        let resp = Request::get("/api/v1/pubkey")
            .send()
            .await
            .map_err(|e| format!("Network error: {e}"))?;
        if resp.ok() {
            resp.json().await.map_err(|e| format!("Parse error: {e}"))
        } else {
            let err: ApiError = resp
                .json()
                .await
                .unwrap_or(ApiError { error: "Unknown error".into() });
            Err(err.error)
        }
    }

    pub async fn post_timestamp(hash: &str) -> Result<common::TimestampToken, String> {
        let body = TimestampRequest { hash: hash.to_string() };
        let resp = Request::post("/api/v1/timestamp")
            .json(&body)
            .map_err(|e| format!("Serialize error: {e}"))?
            .send()
            .await
            .map_err(|e| format!("Network error: {e}"))?;
        if resp.ok() {
            resp.json().await.map_err(|e| format!("Parse error: {e}"))
        } else {
            let err: ApiError = resp
                .json()
                .await
                .unwrap_or(ApiError { error: "Unknown error".into() });
            Err(err.error)
        }
    }

    pub async fn post_verify(token: &common::TimestampToken) -> Result<VerifyResponse, String> {
        let body = VerifyRequest { token: token.clone() };
        let resp = Request::post("/api/v1/verify")
            .json(&body)
            .map_err(|e| format!("Serialize error: {e}"))?
            .send()
            .await
            .map_err(|e| format!("Network error: {e}"))?;
        if resp.ok() {
            resp.json().await.map_err(|e| format!("Parse error: {e}"))
        } else {
            let err: ApiError = resp
                .json()
                .await
                .unwrap_or(ApiError { error: "Unknown error".into() });
            Err(err.error)
        }
    }
}

// -- Server-side fetch wrappers (SSR - call coordinator directly) ------------

#[cfg(feature = "ssr")]
pub mod server {
    use super::*;

    pub async fn get_status_ssr(
        state: &std::sync::Arc<coordinator::state::AppState>,
    ) -> Result<StatusResponse, String> {
        let group_key = coordinator::frost_bridge::verifying_key_to_x_only_hex(
            &state.public_key_package,
        )
        .map_err(|e| e.to_string())?;

        let signers = state
            .config
            .signers
            .iter()
            .map(|s| StatusSigner {
                signer_id: s.signer_id,
                npub: s.npub.clone(),
            })
            .collect();

        Ok(StatusResponse {
            healthy: true,
            active_sessions: state.sessions.len(),
            k: state.config.frost.k,
            n: state.config.frost.n,
            group_public_key: group_key,
            signers,
            relay_urls: state.config.relays.urls.clone(),
        })
    }
}
