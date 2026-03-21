use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use nostr_sdk::prelude::*;
use tokio::sync::mpsc;
use uuid::Uuid;

use common::{KIND_DKG_ROUND1, KIND_DKG_RESULT};
use nostr_transport::events::{
    build_dkg_announce, build_dkg_round1_broadcast,
    parse_dkg_round1, parse_dkg_result,
};
use nostr_transport::types::{DkgAnnounce, DkgParticipant, DkgRound1Broadcast};

use crate::error::CoordinatorError;
use crate::frost_bridge;
use crate::state::{AppState, SessionHandle, SignerInfo};

const DKG_ROUND1_TIMEOUT: Duration = Duration::from_secs(60);
const DKG_RESULT_TIMEOUT: Duration = Duration::from_secs(120);

pub struct DkgOutcome {
    pub group_public_key: String,
}

pub async fn run_dkg_session(
    state: Arc<AppState>,
) -> Result<DkgOutcome, CoordinatorError> {
    let session_id = Uuid::new_v4();
    let k = state.config.frost.k;
    let n = state.config.frost.n;

    tracing::info!(
        session_id = %session_id,
        k, n,
        "starting DKG session"
    );

    // Build participant list from config signers
    let all_signers = resolve_all_signers(&state)?;
    let participants: Vec<DkgParticipant> = all_signers
        .iter()
        .map(|s| DkgParticipant {
            signer_id: s.signer_id,
            npub: state.config.signers.iter()
                .find(|entry| entry.signer_id == s.signer_id)
                .map(|e| e.npub.clone())
                .unwrap_or_default(),
        })
        .collect();

    let signer_ids: HashSet<u16> = all_signers.iter().map(|s| s.signer_id).collect();

    // Register session channel for receiving DKG events
    let (tx, mut rx) = mpsc::channel(100);
    state.sessions.insert(session_id, SessionHandle { event_tx: tx });

    let cleanup = || {
        state.sessions.remove(&session_id);
    };

    // Step 1: Send DkgAnnounce to each signer
    let announce = DkgAnnounce {
        session_id,
        k,
        n,
        participants: participants.clone(),
    };

    for signer in &all_signers {
        let builder = build_dkg_announce(&state.keys, &signer.pubkey, &announce)
            .map_err(|e| CoordinatorError::NostrError(format!("build dkg announce: {e}")))?;
        state.relay
            .send_event_builder(builder)
            .await
            .map_err(|e| CoordinatorError::NostrError(format!("send dkg announce: {e}")))?;
    }

    tracing::info!(session_id = %session_id, "DKG announcements sent to {} signers", all_signers.len());

    // Step 2: Collect n DkgRound1 packages
    let round1_packages = match collect_dkg_round1(
        &state, session_id, &mut rx, n, &signer_ids,
    ).await {
        Ok(pkgs) => pkgs,
        Err(e) => {
            cleanup();
            return Err(e);
        }
    };

    tracing::info!(session_id = %session_id, "collected all {} DKG round 1 packages", n);

    // Step 3: Build and send DkgRound1Broadcast to each signer
    let broadcast = DkgRound1Broadcast {
        session_id,
        packages: round1_packages,
    };

    for signer in &all_signers {
        let builder = build_dkg_round1_broadcast(&state.keys, &signer.pubkey, &broadcast)
            .map_err(|e| CoordinatorError::NostrError(format!("build dkg round1 broadcast: {e}")))?;
        state.relay
            .send_event_builder(builder)
            .await
            .map_err(|e| CoordinatorError::NostrError(format!("send dkg round1 broadcast: {e}")))?;
    }

    tracing::info!(session_id = %session_id, "DKG round 1 broadcast sent");

    // Step 4: Wait for n DkgResult confirmations
    let results = match collect_dkg_results(
        &state, session_id, &mut rx, n, &signer_ids,
    ).await {
        Ok(r) => r,
        Err(e) => {
            cleanup();
            return Err(e);
        }
    };

    tracing::info!(session_id = %session_id, "collected all {} DKG results", n);

    // Step 5: Verify all group_pubkey_hash values match
    let hashes: HashSet<&str> = results.values().map(|(hash, _)| hash.as_str()).collect();
    if hashes.len() != 1 {
        cleanup();
        return Err(CoordinatorError::DkgError(format!(
            "group pubkey hash mismatch: got {} distinct hashes: {:?}",
            hashes.len(), hashes
        )));
    }

    // Step 6: Parse PublicKeyPackage from first signer's result
    let (_, (_, pubkey_pkg_hex)) = results.iter().next().unwrap();
    let pub_key_package = frost_bridge::public_key_package_from_hex(pubkey_pkg_hex)?;

    // Verify the hash matches
    let vk_bytes = pub_key_package.verifying_key().serialize()
        .map_err(|e| CoordinatorError::FrostError(format!("failed to serialize verifying key: {e}")))?;
    let coordinator_hash = hex::encode(frost_core::sha256(&vk_bytes));
    let signer_hash = hashes.into_iter().next().unwrap();
    if coordinator_hash != signer_hash {
        cleanup();
        return Err(CoordinatorError::DkgError(format!(
            "PublicKeyPackage hash mismatch: computed={}, reported={}",
            coordinator_hash, signer_hash
        )));
    }

    let group_key = frost_bridge::verifying_key_to_x_only_hex(&pub_key_package)?;

    // Step 7: Update state with new public key package
    {
        let mut pkg = state.public_key_package.write().await;
        *pkg = Some(pub_key_package);
    }

    cleanup();

    tracing::info!(
        session_id = %session_id,
        group_public_key = %group_key,
        "DKG complete! Group public key established."
    );

    Ok(DkgOutcome { group_public_key: group_key })
}

fn resolve_all_signers(state: &AppState) -> Result<Vec<SignerInfo>, CoordinatorError> {
    state.config.signers.iter().map(|entry| {
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
    }).collect()
}

async fn collect_dkg_round1(
    state: &AppState,
    session_id: Uuid,
    rx: &mut mpsc::Receiver<nostr_sdk::Event>,
    n: u16,
    expected_ids: &HashSet<u16>,
) -> Result<BTreeMap<u16, serde_json::Value>, CoordinatorError> {
    let mut packages = BTreeMap::new();
    let mut seen: HashSet<u16> = HashSet::new();

    let result = tokio::time::timeout(DKG_ROUND1_TIMEOUT, async {
        while packages.len() < n as usize {
            let event = rx.recv().await.ok_or_else(|| {
                CoordinatorError::InternalError("DKG session channel closed".into())
            })?;

            if event.kind().as_u16() != KIND_DKG_ROUND1 {
                continue;
            }

            let parsed = match parse_dkg_round1(&event, &state.keys) {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(session_id = %session_id, "failed to parse DKG round1: {e}");
                    continue;
                }
            };

            if parsed.session_id != session_id {
                continue;
            }

            if !expected_ids.contains(&parsed.signer_id) || seen.contains(&parsed.signer_id) {
                continue;
            }

            packages.insert(parsed.signer_id, parsed.package);
            seen.insert(parsed.signer_id);

            tracing::debug!(
                session_id = %session_id,
                signer_id = parsed.signer_id,
                "received DKG round 1 ({}/{})",
                packages.len(), n
            );
        }
        Ok::<_, CoordinatorError>(packages)
    }).await;

    match result {
        Ok(inner) => inner,
        Err(_) => {
            let missing: Vec<u16> = expected_ids
                .iter()
                .filter(|id| !seen.contains(id))
                .copied()
                .collect();
            Err(CoordinatorError::DkgTimeout {
                session_id,
                missing_signers: missing,
            })
        }
    }
}

/// Collects DKG results. Returns BTreeMap<signer_id, (group_pubkey_hash, public_key_package_hex)>.
async fn collect_dkg_results(
    state: &AppState,
    session_id: Uuid,
    rx: &mut mpsc::Receiver<nostr_sdk::Event>,
    n: u16,
    expected_ids: &HashSet<u16>,
) -> Result<BTreeMap<u16, (String, String)>, CoordinatorError> {
    let mut results = BTreeMap::new();
    let mut seen: HashSet<u16> = HashSet::new();

    let result = tokio::time::timeout(DKG_RESULT_TIMEOUT, async {
        while results.len() < n as usize {
            let event = rx.recv().await.ok_or_else(|| {
                CoordinatorError::InternalError("DKG session channel closed".into())
            })?;

            if event.kind().as_u16() != KIND_DKG_RESULT {
                continue;
            }

            let parsed = match parse_dkg_result(&event, &state.keys) {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(session_id = %session_id, "failed to parse DKG result: {e}");
                    continue;
                }
            };

            if parsed.session_id != session_id {
                continue;
            }

            if !expected_ids.contains(&parsed.signer_id) || seen.contains(&parsed.signer_id) {
                continue;
            }

            results.insert(parsed.signer_id, (parsed.group_pubkey_hash, parsed.public_key_package));
            seen.insert(parsed.signer_id);

            tracing::debug!(
                session_id = %session_id,
                signer_id = parsed.signer_id,
                "received DKG result ({}/{})",
                results.len(), n
            );
        }
        Ok::<_, CoordinatorError>(results)
    }).await;

    match result {
        Ok(inner) => inner,
        Err(_) => {
            let missing: Vec<u16> = expected_ids
                .iter()
                .filter(|id| !seen.contains(id))
                .copied()
                .collect();
            Err(CoordinatorError::DkgTimeout {
                session_id,
                missing_signers: missing,
            })
        }
    }
}
