use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use frost_secp256k1_tr::{self as frost, round1::SigningCommitments, round2::SignatureShare};
use tokio::sync::mpsc;
use uuid::Uuid;

use common::{KIND_PARTIAL_SIG, KIND_ROUND1_COMMITMENT, TimestampToken};
use nostr_transport::events::{
    build_session_announce, build_round2_payload, build_timestamp_token,
    parse_round1_commitment, parse_partial_signature,
};
use nostr_transport::types::{Round2Payload, SessionAnnounce};

use crate::error::CoordinatorError;
use crate::frost_bridge;
use crate::state::{AppState, SessionHandle, SignerInfo};

const ROUND_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn run_signing_session(
    state: Arc<AppState>,
    document_hash_hex: String,
) -> Result<TimestampToken, CoordinatorError> {
    let session_id = Uuid::new_v4();

    // Acquire dedup guard
    {
        use dashmap::mapref::entry::Entry;
        match state.active_hashes.entry(document_hash_hex.clone()) {
            Entry::Occupied(_) => {
                tracing::warn!(hash = %document_hash_hex, "dedup rejection: signing session already in progress");
                return Err(CoordinatorError::BadRequest(format!(
                    "signing session already in progress for hash {}",
                    document_hash_hex
                )));
            }
            Entry::Vacant(e) => {
                e.insert(session_id);
            }
        }
    }

    let result = run_session_inner(state.clone(), session_id, &document_hash_hex).await;

    if let Err(ref e) = result {
        state.event_emitter.emit(
            Some(session_id.to_string()),
            format!("session failed: {}", e),
        );
    }

    // Always cleanup
    state.active_hashes.remove(&document_hash_hex);
    state.sessions.remove(&session_id);
    tracing::info!(session_id = %session_id, hash = %document_hash_hex, "session cleaned up");

    result
}

async fn run_session_inner(
    state: Arc<AppState>,
    session_id: Uuid,
    document_hash_hex: &str,
) -> Result<TimestampToken, CoordinatorError> {
    let k = state.config.frost.k;
    let n = state.config.frost.n;

    // 1. Build token skeleton
    let serial_number = state.serial_counter.fetch_add(1, Ordering::SeqCst) + 1;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut token = TimestampToken {
        serial_number,
        timestamp,
        file_hash: document_hash_hex.to_string(),
        signature: String::new(),
        group_public_key: String::new(),
    };

    // 2. Compute the message to sign
    let message_hash = token
        .compute_message_hash()
        .map_err(|e| CoordinatorError::InternalError(format!("failed to compute message hash: {e}")))?;

    // 3. Select k random signers
    let selected_signers = select_signers(&state, k)?;
    let selected_ids: HashSet<u16> = selected_signers.iter().map(|s| s.signer_id).collect();

    tracing::info!(
        session_id = %session_id,
        signers = ?selected_ids,
        "starting signing session"
    );

    state.event_emitter.emit(
        Some(session_id.to_string()),
        format!("session started, selected signers: {:?}", selected_ids),
    );

    // 4. Register session channel
    let (tx, mut rx) = mpsc::channel(100);
    state.sessions.insert(
        session_id,
        SessionHandle { event_tx: tx },
    );

    // 5. Round 1 out: Send SessionAnnounce to each signer.
    //    Signers recompute the message hash from (serial, timestamp, file_hash)
    //    and validate the timestamp against their own clock before committing.
    let announce = SessionAnnounce {
        session_id,
        serial_number: token.serial_number,
        timestamp: token.timestamp,
        file_hash: token.file_hash.clone(),
        k: k as usize,
        n: n as usize,
    };

    for signer in &selected_signers {
        let builder = build_session_announce(&state.keys, &signer.pubkey, &announce)
            .map_err(|e| CoordinatorError::NostrError(format!("build announce: {e}")))?;
        state
            .relay
            .send_event_builder(builder)
            .await
            .map_err(|e| CoordinatorError::NostrError(format!("send announce: {e}")))?;
    }

    tracing::info!(session_id = %session_id, "round 1 announcements sent");

    // 6. Round 1 in: Collect k commitments
    let round1_start = std::time::Instant::now();
    let commitments = collect_commitments(
        &state, session_id, &mut rx, k, &selected_ids,
    )
    .await?;

    tracing::info!(
        session_id = %session_id,
        elapsed_ms = round1_start.elapsed().as_millis() as u64,
        "round 1 commitments collected"
    );

    state.event_emitter.emit(
        Some(session_id.to_string()),
        format!("round 1 commitments collected ({}ms)", round1_start.elapsed().as_millis()),
    );

    // 7. Build SigningPackage
    let signing_package = frost::SigningPackage::new(commitments.clone(), &message_hash);

    // 8. Round 2 out: Send SigningPackage to each signer
    let sp_json = frost_bridge::signing_package_to_json(&signing_package)?;

    for signer in &selected_signers {
        let payload = Round2Payload {
            session_id,
            signing_package: sp_json.clone(),
        };
        let builder = build_round2_payload(&state.keys, &signer.pubkey, &payload)
            .map_err(|e| CoordinatorError::NostrError(format!("build round2: {e}")))?;
        state
            .relay
            .send_event_builder(builder)
            .await
            .map_err(|e| CoordinatorError::NostrError(format!("send round2: {e}")))?;
    }

    tracing::info!(session_id = %session_id, "round 2 payloads sent");

    // 9. Round 2 in: Collect k signature shares
    let round2_start = std::time::Instant::now();
    let shares = collect_signature_shares(
        &state, session_id, &mut rx, k, &selected_ids,
    )
    .await?;

    tracing::info!(
        session_id = %session_id,
        elapsed_ms = round2_start.elapsed().as_millis() as u64,
        "round 2 shares collected"
    );

    state.event_emitter.emit(
        Some(session_id.to_string()),
        format!("round 2 shares collected ({}ms)", round2_start.elapsed().as_millis()),
    );

    // 10. Aggregate
    tracing::info!(session_id = %session_id, "aggregating signature shares");
    let pubkey_pkg_guard = state.public_key_package.read().await;
    let pubkey_pkg = pubkey_pkg_guard.as_ref()
        .ok_or(CoordinatorError::NoDkgKeysYet)?;

    let signature = frost::aggregate(&signing_package, &shares, pubkey_pkg)
        .map_err(|e| CoordinatorError::FrostError(format!("aggregation failed: {e}")))?;

    // 11. Verify aggregated signature
    pubkey_pkg
        .verifying_key()
        .verify(&message_hash, &signature)
        .map_err(|e| CoordinatorError::FrostError(format!("signature verification failed: {e}")))?;

    tracing::info!(session_id = %session_id, "signature aggregated and verified");

    state.event_emitter.emit(
        Some(session_id.to_string()),
        "signature aggregated and verified".to_string(),
    );

    // 12. Fill token
    token.signature = frost_bridge::signature_to_hex(&signature)?;
    token.group_public_key = frost_bridge::verifying_key_to_x_only_hex(pubkey_pkg)?;
    drop(pubkey_pkg_guard);

    // 13. Publish token as Nostr kind:1 event
    tracing::info!(session_id = %session_id, serial = serial_number, "publishing timestamp token to Nostr");
    let token_json = serde_json::to_string(&token)
        .map_err(|e| CoordinatorError::InternalError(format!("serialize token: {e}")))?;
    let token_builder = build_timestamp_token(&token_json);
    state
        .relay
        .send_event_builder(token_builder)
        .await
        .map_err(|e| CoordinatorError::NostrError(format!("publish token: {e}")))?;

    tracing::info!(session_id = %session_id, serial = serial_number, "timestamp token published to Nostr");

    state.event_emitter.emit(
        Some(session_id.to_string()),
        format!("timestamp token published (serial={})", serial_number),
    );

    Ok(token)
}

fn select_signers(
    state: &AppState,
    k: u16,
) -> Result<Vec<SignerInfo>, CoordinatorError> {
    use rand::seq::SliceRandom;
    use nostr_sdk::prelude::*;

    let mut entries: Vec<_> = state.config.signers.iter().collect();
    entries.shuffle(&mut rand::thread_rng());

    entries
        .into_iter()
        .take(k as usize)
        .map(|entry| {
            let pubkey = PublicKey::from_bech32(&entry.npub).map_err(|e| {
                CoordinatorError::ConfigError(format!(
                    "invalid npub for signer {}: {e}",
                    entry.signer_id
                ))
            })?;
            Ok(SignerInfo {
                signer_id: entry.signer_id,
                pubkey,
            })
        })
        .collect()
}

async fn collect_commitments(
    state: &AppState,
    session_id: Uuid,
    rx: &mut mpsc::Receiver<nostr_sdk::Event>,
    k: u16,
    selected_ids: &HashSet<u16>,
) -> Result<BTreeMap<frost::Identifier, SigningCommitments>, CoordinatorError> {
    let mut commitments = BTreeMap::new();
    let mut seen: HashSet<u16> = HashSet::new();

    let result = tokio::time::timeout(ROUND_TIMEOUT, async {
        while commitments.len() < k as usize {
            let event = rx.recv().await.ok_or_else(|| {
                CoordinatorError::InternalError("session channel closed".into())
            })?;

            if event.kind().as_u16() != KIND_ROUND1_COMMITMENT {
                continue;
            }

            let parsed = match parse_round1_commitment(&event, &state.keys) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(session_id = %session_id, "failed to parse commitment: {e}");
                    continue;
                }
            };

            if parsed.session_id != session_id {
                continue;
            }

            if !selected_ids.contains(&parsed.signer_id) || seen.contains(&parsed.signer_id) {
                continue;
            }

            let sc = frost_bridge::commitments_from_json(parsed.commitment)?;
            let id = frost_bridge::identifier_from_signer_id(parsed.signer_id)?;

            commitments.insert(id, sc);
            seen.insert(parsed.signer_id);

            tracing::debug!(
                session_id = %session_id,
                signer_id = parsed.signer_id,
                "received commitment ({}/{})",
                commitments.len(),
                k
            );
        }
        Ok::<_, CoordinatorError>(commitments)
    })
    .await;

    match result {
        Ok(inner) => inner,
        Err(_) => {
            let missing: Vec<u16> = selected_ids
                .iter()
                .filter(|id| !seen.contains(id))
                .copied()
                .collect();
            Err(CoordinatorError::SigningTimeout {
                session_id,
                missing_signers: missing,
            })
        }
    }
}

async fn collect_signature_shares(
    state: &AppState,
    session_id: Uuid,
    rx: &mut mpsc::Receiver<nostr_sdk::Event>,
    k: u16,
    selected_ids: &HashSet<u16>,
) -> Result<BTreeMap<frost::Identifier, SignatureShare>, CoordinatorError> {
    let mut shares = BTreeMap::new();
    let mut seen: HashSet<u16> = HashSet::new();

    let result = tokio::time::timeout(ROUND_TIMEOUT, async {
        while shares.len() < k as usize {
            let event = rx.recv().await.ok_or_else(|| {
                CoordinatorError::InternalError("session channel closed".into())
            })?;

            if event.kind().as_u16() != KIND_PARTIAL_SIG {
                continue;
            }

            let parsed = match parse_partial_signature(&event, &state.keys) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(session_id = %session_id, "failed to parse signature share: {e}");
                    continue;
                }
            };

            if parsed.session_id != session_id {
                continue;
            }

            if !selected_ids.contains(&parsed.signer_id) || seen.contains(&parsed.signer_id) {
                continue;
            }

            let ss = frost_bridge::signature_share_from_json(parsed.signature_share)?;
            let id = frost_bridge::identifier_from_signer_id(parsed.signer_id)?;

            shares.insert(id, ss);
            seen.insert(parsed.signer_id);

            tracing::debug!(
                session_id = %session_id,
                signer_id = parsed.signer_id,
                "received signature share ({}/{})",
                shares.len(),
                k
            );
        }
        Ok::<_, CoordinatorError>(shares)
    })
    .await;

    match result {
        Ok(inner) => inner,
        Err(_) => {
            let missing: Vec<u16> = selected_ids
                .iter()
                .filter(|id| !seen.contains(id))
                .copied()
                .collect();
            Err(CoordinatorError::SigningTimeout {
                session_id,
                missing_signers: missing,
            })
        }
    }
}
