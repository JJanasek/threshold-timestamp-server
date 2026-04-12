use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use frost_secp256k1_tr::keys::KeyPackage;
use nostr_sdk::prelude::*;
use nostr_sdk::RelayPoolNotification;
use tokio::sync::RwLock;

use common::event_client::EventEmitter;
use common::{
    TimestampToken,
    KIND_ROUND2_PAYLOAD, KIND_SESSION_ANNOUNCE,
    KIND_DKG_ANNOUNCE, KIND_DKG_ROUND1_BROADCAST, KIND_DKG_ROUND2,
};

/// Maximum allowed drift between the coordinator's announced timestamp and the
/// signer's local clock. Requests outside this window are silently rejected to
/// prevent a malicious coordinator from back- or post-dating tokens.
const MAX_CLOCK_DRIFT_SECS: i64 = 60;
use frost_core::secp256k1::{
    Secp256k1, Nonce,
    dkg_part1, dkg_part2, dkg_part3,
    dkg_round1_package_to_json, dkg_round1_package_from_json,
    dkg_round2_package_to_json, dkg_round2_package_from_json,
};
use frost_core::ThresholdScheme;
use nostr_transport::events::{
    build_partial_signature, build_round1_commitment, parse_round2_payload, parse_session_announce,
    build_dkg_round1, build_dkg_round2, build_dkg_result,
    parse_dkg_announce, parse_dkg_round1_broadcast, parse_dkg_round2,
};
use nostr_transport::relay::NostrRelay;
use nostr_transport::types::{
    PartialSignature, Round1Commitment,
    DkgRound1, DkgRound2, DkgResult,
};

use crate::config;
use crate::dkg_state::DkgState;
use crate::nonce_map::NonceMap;

/// Shared mutable signing identity — updated in-place after DKG.
pub struct SigningIdentity {
    pub key_package: Option<KeyPackage>,
    pub signer_id: Option<u16>,
}

pub async fn run_event_loop(
    relay: &NostrRelay,
    identity: Arc<RwLock<SigningIdentity>>,
    coordinator_pubkey: &PublicKey,
    nonce_map: &NonceMap,
    interactive: bool,
    config_path: &str,
    dkg_state: Arc<RwLock<DkgState>>,
    emitter: &EventEmitter,
) -> Result<()> {
    let mut notifications = relay.notifications();

    {
        let id = identity.read().await;
        if id.key_package.is_some() {
            tracing::info!(signer_id = ?id.signer_id, "event loop started, listening for signing sessions...");
        } else {
            tracing::info!("event loop started in DKG-only mode, waiting for DKG announcement...");
        }
    }

    loop {
        match notifications.recv().await {
            Ok(RelayPoolNotification::Event { event, .. }) => {
                let kind = event.kind().as_u16();
                let sender = event.author();

                tracing::debug!(kind, %sender, "received event");

                let result = match kind {
                    // Signing protocol events (coordinator only)
                    KIND_SESSION_ANNOUNCE if sender == *coordinator_pubkey => {
                        let id = identity.read().await;
                        if let Some(ref kp) = id.key_package {
                            let sid = id.signer_id.unwrap_or(0);
                            handle_session_announce(
                                relay, &event, kp, sid,
                                coordinator_pubkey, nonce_map, interactive, emitter,
                            ).await
                        } else {
                            tracing::debug!("ignoring signing session (no key_package, DKG-only mode)");
                            Ok(())
                        }
                    }
                    KIND_ROUND2_PAYLOAD if sender == *coordinator_pubkey => {
                        let id = identity.read().await;
                        if let Some(ref kp) = id.key_package {
                            let sid = id.signer_id.unwrap_or(0);
                            handle_round2_payload(
                                relay, &event, kp, sid,
                                coordinator_pubkey, nonce_map, emitter,
                            ).await
                        } else {
                            tracing::debug!("ignoring round2 payload (no key_package, DKG-only mode)");
                            Ok(())
                        }
                    }

                    // DKG protocol events
                    KIND_DKG_ANNOUNCE if sender == *coordinator_pubkey => {
                        handle_dkg_announce(
                            relay, &event, coordinator_pubkey, &dkg_state,
                        ).await
                    }
                    KIND_DKG_ROUND1_BROADCAST if sender == *coordinator_pubkey => {
                        handle_dkg_round1_broadcast(
                            relay, &event, coordinator_pubkey, &dkg_state,
                        ).await
                    }
                    KIND_DKG_ROUND2 => {
                        // Round 2 comes from peer signers, not coordinator
                        handle_dkg_round2_package(
                            relay, &event, coordinator_pubkey,
                            &dkg_state, config_path, &identity,
                        ).await
                    }

                    _ => {
                        tracing::trace!(kind, "ignoring irrelevant event kind");
                        Ok(())
                    }
                };

                if let Err(ref e) = result {
                    tracing::error!(kind, error = %e, "handler error");
                    emitter.emit(None, format!("handler error (kind={}): {}", kind, e));
                }
            }
            Ok(RelayPoolNotification::Shutdown) => {
                tracing::warn!("relay connection shut down");
                break;
            }
            Ok(_) => {}
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!(n, "notification receiver lagged, some events may be lost");
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                tracing::warn!("notification channel closed");
                break;
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Signing handlers
// ---------------------------------------------------------------------------

/// Validates a SessionAnnounce's pre-image fields and recomputes the canonical
/// message hash the signer will commit to. Returns the 32-byte hash on success
/// or a human-readable rejection reason on failure.
///
/// Checks performed:
/// 1. `file_hash` is exactly 64 hex characters.
/// 2. `timestamp` is within ±`max_drift_secs` of `now_secs`.
/// 3. The canonical message hash is derived via `TimestampToken::compute_message_hash()`.
///
/// Split out as a pure function so it can be unit-tested without mocking Nostr
/// events, relays, or clocks.
fn validate_and_recompute_message(
    serial_number: u64,
    timestamp: u64,
    file_hash: &str,
    now_secs: u64,
    max_drift_secs: i64,
) -> std::result::Result<[u8; 32], String> {
    if file_hash.len() != 64 || hex::decode(file_hash).is_err() {
        return Err("file_hash is not a 64-char hex string".to_string());
    }

    let drift = now_secs as i64 - timestamp as i64;
    if drift.abs() > max_drift_secs {
        return Err(format!(
            "timestamp drift {}s exceeds {}s limit",
            drift, max_drift_secs
        ));
    }

    // Reuse the client-side canonical hash so signer, coordinator and
    // verifier all agree on exactly which bytes get signed.
    let preimage = TimestampToken {
        serial_number,
        timestamp,
        file_hash: file_hash.to_string(),
        signature: String::new(),
        group_public_key: String::new(),
    };
    preimage
        .compute_message_hash()
        .map_err(|e| format!("failed to recompute message hash: {}", e))
}

async fn handle_session_announce(
    relay: &NostrRelay,
    event: &Event,
    key_package: &KeyPackage,
    signer_id: u16,
    coordinator_pubkey: &PublicKey,
    nonce_map: &NonceMap,
    interactive: bool,
    emitter: &EventEmitter,
) -> Result<()> {
    let announce = parse_session_announce(event, relay.keys())
        .context("failed to parse SessionAnnounce")?;

    tracing::debug!(
        session_id = %announce.session_id,
        serial = announce.serial_number,
        timestamp = announce.timestamp,
        file_hash = %announce.file_hash,
        k = announce.k,
        n = announce.n,
        "decrypted SessionAnnounce"
    );

    emitter.emit(
        Some(announce.session_id.to_string()),
        "session announce received".to_string(),
    );

    // --- Signer-side policy checks ------------------------------------------
    //
    // The coordinator is not trusted to dictate the message; we recompute the
    // hash ourselves from the pre-image and validate the embedded timestamp
    // against our local clock. This protects against back-/post-dated tokens
    // and against signers being tricked into signing arbitrary 32-byte values.
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let expected_msg = match validate_and_recompute_message(
        announce.serial_number,
        announce.timestamp,
        &announce.file_hash,
        now_secs,
        MAX_CLOCK_DRIFT_SECS,
    ) {
        Ok(m) => m,
        Err(reason) => {
            tracing::warn!(
                session_id = %announce.session_id,
                reason = %reason,
                "rejecting session announce"
            );
            emitter.emit(
                Some(announce.session_id.to_string()),
                format!("rejected: {}", reason),
            );
            return Ok(());
        }
    };

    // Interactive mode: ask user for approval
    if interactive {
        let drift = now_secs as i64 - announce.timestamp as i64;
        eprintln!(
            "\n--- Signing request ---\n  Session: {}\n  Serial:  {}\n  Time:    {} (drift {}s)\n  Doc:     {}\n  Threshold: {}/{}\nApprove? [y/N] ",
            announce.session_id,
            announce.serial_number,
            announce.timestamp,
            drift,
            announce.file_hash,
            announce.k,
            announce.n
        );
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            tracing::info!(session_id = %announce.session_id, "user rejected signing request");
            return Ok(());
        }
    }

    // Generate nonce and commitment
    tracing::debug!(session_id = %announce.session_id, "generating nonce");
    let nonce: Nonce = Secp256k1::generate_nonce(key_package);
    let commitment = Secp256k1::nonce_commitment(&nonce);
    let commitment_json = commitment
        .to_json()
        .map_err(|e| anyhow::anyhow!("failed to serialize commitment: {e}"))?;

    // Store nonce BEFORE sending commitment (prevent race with Round2).
    // The expected message hash is bound to the nonce so Round 2 can verify
    // the SigningPackage carries exactly what we committed to.
    if !nonce_map
        .insert(announce.session_id, nonce, expected_msg)
        .await
    {
        tracing::warn!(session_id = %announce.session_id, "duplicate session, ignoring");
        return Ok(());
    }
    tracing::debug!(
        session_id = %announce.session_id,
        active_nonces = nonce_map.active_count().await,
        "stored nonce in map"
    );

    // Build and send Round1Commitment
    let payload = Round1Commitment {
        session_id: announce.session_id,
        signer_id,
        commitment: commitment_json,
    };

    tracing::debug!(session_id = %announce.session_id, "building Round1Commitment event");
    let builder = build_round1_commitment(relay.keys(), coordinator_pubkey, &payload)
        .map_err(|e| anyhow::anyhow!("build round1 commitment: {e}"))?;

    relay
        .send_event_builder(builder)
        .await
        .map_err(|e| anyhow::anyhow!("send round1 commitment: {e}"))?;

    tracing::info!(session_id = %announce.session_id, "sent Round1Commitment");

    emitter.emit(
        Some(announce.session_id.to_string()),
        "round 1 commitment sent".to_string(),
    );

    Ok(())
}

async fn handle_round2_payload(
    relay: &NostrRelay,
    event: &Event,
    key_package: &KeyPackage,
    signer_id: u16,
    coordinator_pubkey: &PublicKey,
    nonce_map: &NonceMap,
    emitter: &EventEmitter,
) -> Result<()> {
    let round2 = parse_round2_payload(event, relay.keys())
        .context("failed to parse Round2Payload")?;

    tracing::debug!(session_id = %round2.session_id, "decrypted Round2Payload");

    emitter.emit(
        Some(round2.session_id.to_string()),
        "round 2 payload received".to_string(),
    );

    // Retrieve the stored nonce and the message hash we committed to in Round 1
    // (single-use: a second take() for the same session returns None).
    let (nonce, expected_msg) = match nonce_map.take(&round2.session_id).await {
        Some(entry) => entry,
        None => {
            tracing::warn!(
                session_id = %round2.session_id,
                "no nonce found for session (expired or already used)"
            );
            return Ok(());
        }
    };

    tracing::debug!(session_id = %round2.session_id, "retrieved stored nonce");

    // Deserialize the SigningPackage from JSON
    tracing::debug!(session_id = %round2.session_id, "deserializing SigningPackage");
    let signing_package: frost_secp256k1_tr::SigningPackage =
        serde_json::from_value(round2.signing_package.clone())
            .context("failed to deserialize SigningPackage")?;

    // Cross-check: the SigningPackage must carry the exact same message hash
    // that was announced in Round 1. A malicious coordinator could otherwise
    // swap the message between rounds.
    if signing_package.message().as_slice() != expected_msg.as_slice() {
        tracing::warn!(
            session_id = %round2.session_id,
            "rejecting Round2: SigningPackage message does not match Round1 announcement"
        );
        return Ok(());
    }

    // Compute partial signature
    tracing::debug!(session_id = %round2.session_id, "computing partial signature");
    let sig_share =
        frost_secp256k1_tr::round2::sign(&signing_package, nonce.signing_nonces(), key_package)
            .map_err(|e| anyhow::anyhow!("partial_sign failed: {e}"))?;

    let sig_share_json = serde_json::to_value(&sig_share)
        .context("failed to serialize SignatureShare")?;

    // Build and send PartialSignature
    tracing::debug!(session_id = %round2.session_id, "building PartialSignature event");
    let payload = PartialSignature {
        session_id: round2.session_id,
        signer_id,
        signature_share: sig_share_json,
    };

    let builder = build_partial_signature(relay.keys(), coordinator_pubkey, &payload)
        .map_err(|e| anyhow::anyhow!("build partial signature: {e}"))?;

    relay
        .send_event_builder(builder)
        .await
        .map_err(|e| anyhow::anyhow!("send partial signature: {e}"))?;

    tracing::info!(session_id = %round2.session_id, "sent PartialSignature");

    emitter.emit(
        Some(round2.session_id.to_string()),
        "partial signature sent".to_string(),
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// DKG handlers
// ---------------------------------------------------------------------------

async fn handle_dkg_announce(
    relay: &NostrRelay,
    event: &Event,
    coordinator_pubkey: &PublicKey,
    dkg_state: &Arc<RwLock<DkgState>>,
) -> Result<()> {
    let announce = parse_dkg_announce(event, relay.keys())
        .context("failed to parse DkgAnnounce")?;

    tracing::info!(
        session_id = %announce.session_id,
        k = announce.k,
        n = announce.n,
        participants = announce.participants.len(),
        "received DKG announcement"
    );

    let my_pubkey = relay.public_key();

    // Find our entry in the participant list
    let my_entry = announce.participants.iter().find(|p| {
        PublicKey::from_bech32(&p.npub)
            .map(|pk| pk == my_pubkey)
            .unwrap_or(false)
    });

    let my_entry = match my_entry {
        Some(e) => e,
        None => {
            tracing::warn!(session_id = %announce.session_id, "not in participant list, ignoring");
            return Ok(());
        }
    };

    let my_signer_id = my_entry.signer_id;
    let my_identifier = frost_secp256k1_tr::Identifier::try_from(my_signer_id)
        .map_err(|e| anyhow::anyhow!("invalid signer id {my_signer_id}: {e}"))?;

    // Build participant map (signer_id -> nostr pubkey)
    let mut participants = BTreeMap::new();
    for p in &announce.participants {
        let pk = PublicKey::from_bech32(&p.npub)
            .map_err(|e| anyhow::anyhow!("invalid npub for signer {}: {e}", p.signer_id))?;
        participants.insert(p.signer_id, pk);
    }

    // DKG Part 1: generate our round 1 package
    let (round1_secret, round1_package) = dkg_part1(my_identifier, announce.n, announce.k)
        .map_err(|e| anyhow::anyhow!("DKG part1 failed: {e}"))?;

    let round1_json = dkg_round1_package_to_json(&round1_package)
        .map_err(|e| anyhow::anyhow!("failed to serialize round1 package: {e}"))?;

    // Update DKG state
    {
        let mut state = dkg_state.write().await;
        state.reset();
        state.session_id = Some(announce.session_id);
        state.my_identifier = Some(my_identifier);
        state.my_signer_id = Some(my_signer_id);
        state.participants = participants;
        state.round1_secret = Some(round1_secret);
        state.k = announce.k;
        state.n = announce.n;
    }

    // Send round 1 package to coordinator
    let payload = DkgRound1 {
        session_id: announce.session_id,
        signer_id: my_signer_id,
        package: round1_json,
    };

    let builder = build_dkg_round1(relay.keys(), coordinator_pubkey, &payload)
        .map_err(|e| anyhow::anyhow!("build dkg round1: {e}"))?;

    relay
        .send_event_builder(builder)
        .await
        .map_err(|e| anyhow::anyhow!("send dkg round1: {e}"))?;

    tracing::info!(
        session_id = %announce.session_id,
        signer_id = my_signer_id,
        "sent DKG Round 1 package to coordinator"
    );

    Ok(())
}

async fn handle_dkg_round1_broadcast(
    relay: &NostrRelay,
    event: &Event,
    _coordinator_pubkey: &PublicKey,
    dkg_state: &Arc<RwLock<DkgState>>,
) -> Result<()> {
    let broadcast = parse_dkg_round1_broadcast(event, relay.keys())
        .context("failed to parse DkgRound1Broadcast")?;

    let mut state = dkg_state.write().await;

    // Verify session
    if state.session_id != Some(broadcast.session_id) {
        tracing::warn!(
            expected = ?state.session_id,
            got = %broadcast.session_id,
            "DKG round1 broadcast for wrong session"
        );
        return Ok(());
    }

    let my_signer_id = state.my_signer_id.unwrap();

    tracing::info!(
        session_id = %broadcast.session_id,
        packages = broadcast.packages.len(),
        "received DKG round 1 broadcast"
    );

    // Build round1_packages map (Identifier -> Package), excluding our own
    let mut round1_packages = BTreeMap::new();
    for (&sid, pkg_json) in &broadcast.packages {
        if sid == my_signer_id {
            continue; // Skip our own
        }
        let identifier = frost_secp256k1_tr::Identifier::try_from(sid)
            .map_err(|e| anyhow::anyhow!("invalid signer id {sid}: {e}"))?;
        let package = dkg_round1_package_from_json(pkg_json.clone())
            .map_err(|e| anyhow::anyhow!("failed to deserialize round1 package from signer {sid}: {e}"))?;
        round1_packages.insert(identifier, package);
    }

    // DKG Part 2: process round 1 packages
    let round1_secret = state.round1_secret.take()
        .ok_or_else(|| anyhow::anyhow!("no round1 secret stored (out of order?)"))?;

    let (round2_secret, round2_packages) = dkg_part2(round1_secret, &round1_packages)
        .map_err(|e| anyhow::anyhow!("DKG part2 failed: {e}"))?;

    // Store for part 3
    state.round1_packages = Some(round1_packages);
    state.round2_secret = Some(round2_secret);

    // Send round 2 packages to each peer (peer-to-peer, NIP-44 encrypted)
    for (&target_id, pkg) in &round2_packages {
        // Extract signer_id from Identifier (serialized as 32-byte big-endian scalar)
        let id_bytes = target_id.serialize();
        let target_signer_id = if id_bytes.len() >= 2 {
            u16::from_be_bytes([id_bytes[id_bytes.len() - 2], id_bytes[id_bytes.len() - 1]])
        } else {
            return Err(anyhow::anyhow!("cannot extract signer_id from identifier"));
        };

        let peer_pubkey = state.peer_pubkey(target_signer_id)
            .ok_or_else(|| anyhow::anyhow!("no pubkey for signer {target_signer_id}"))?;

        let pkg_json = dkg_round2_package_to_json(pkg)
            .map_err(|e| anyhow::anyhow!("failed to serialize round2 package: {e}"))?;

        let payload = DkgRound2 {
            session_id: broadcast.session_id,
            sender_id: my_signer_id,
            package: pkg_json,
        };

        let builder = build_dkg_round2(relay.keys(), peer_pubkey, &payload)
            .map_err(|e| anyhow::anyhow!("build dkg round2: {e}"))?;

        relay
            .send_event_builder(builder)
            .await
            .map_err(|e| anyhow::anyhow!("send dkg round2 to signer {target_signer_id}: {e}"))?;

        tracing::debug!(
            session_id = %broadcast.session_id,
            target_signer_id,
            "sent DKG round 2 package to peer"
        );
    }

    tracing::info!(
        session_id = %broadcast.session_id,
        sent_to = round2_packages.len(),
        "sent all DKG round 2 packages to peers"
    );

    Ok(())
}

async fn handle_dkg_round2_package(
    relay: &NostrRelay,
    event: &Event,
    coordinator_pubkey: &PublicKey,
    dkg_state: &Arc<RwLock<DkgState>>,
    config_path: &str,
    identity: &Arc<RwLock<SigningIdentity>>,
) -> Result<()> {
    let round2_msg = parse_dkg_round2(event, relay.keys())
        .context("failed to parse DkgRound2")?;

    let mut state = dkg_state.write().await;

    // Verify session
    if state.session_id != Some(round2_msg.session_id) {
        tracing::warn!(
            expected = ?state.session_id,
            got = %round2_msg.session_id,
            "DKG round2 package for wrong session"
        );
        return Ok(());
    }

    // Verify sender is in participant list (NOT coordinator)
    let sender_pubkey = event.author();
    let sender_in_list = state.participants.values().any(|pk| *pk == sender_pubkey);
    if !sender_in_list {
        tracing::warn!(
            session_id = %round2_msg.session_id,
            sender = %sender_pubkey,
            "DKG round2 from unknown sender, ignoring"
        );
        return Ok(());
    }

    let sender_id = frost_secp256k1_tr::Identifier::try_from(round2_msg.sender_id)
        .map_err(|e| anyhow::anyhow!("invalid sender id {}: {e}", round2_msg.sender_id))?;

    let package = dkg_round2_package_from_json(round2_msg.package)
        .map_err(|e| anyhow::anyhow!("failed to deserialize round2 package from signer {}: {e}", round2_msg.sender_id))?;

    state.round2_packages.insert(sender_id, package);

    tracing::debug!(
        session_id = %round2_msg.session_id,
        from_signer = round2_msg.sender_id,
        collected = state.round2_packages.len(),
        needed = state.n - 1,
        "received DKG round 2 package from peer"
    );

    // Check if we have all n-1 packages
    if !state.round2_complete() {
        return Ok(());
    }

    tracing::info!(
        session_id = %round2_msg.session_id,
        "all round 2 packages received, running DKG part 3"
    );

    // DKG Part 3: finalize
    let round2_secret = state.round2_secret.as_ref()
        .ok_or_else(|| anyhow::anyhow!("no round2 secret stored"))?;
    let round1_packages = state.round1_packages.as_ref()
        .ok_or_else(|| anyhow::anyhow!("no round1 packages stored"))?;

    let (key_package, pub_key_package) = dkg_part3(round2_secret, round1_packages, &state.round2_packages)
        .map_err(|e| anyhow::anyhow!("DKG part3 failed: {e}"))?;

    // Compute group pubkey hash for verification
    let vk_bytes = pub_key_package.verifying_key().serialize()
        .map_err(|e| anyhow::anyhow!("failed to serialize verifying key: {e}"))?;
    let group_pubkey_hash = hex::encode(frost_core::sha256(&vk_bytes));

    // Serialize the PublicKeyPackage so coordinator can use it
    let pubkey_pkg_bytes = pub_key_package.serialize()
        .map_err(|e| anyhow::anyhow!("failed to serialize PublicKeyPackage: {e}"))?;
    let pubkey_pkg_hex = hex::encode(&pubkey_pkg_bytes);

    let session_id = state.session_id.unwrap();
    let my_signer_id = state.my_signer_id.unwrap();

    // Send result to coordinator
    let payload = DkgResult {
        session_id,
        signer_id: my_signer_id,
        group_pubkey_hash: group_pubkey_hash.clone(),
        public_key_package: pubkey_pkg_hex,
    };

    let builder = build_dkg_result(relay.keys(), coordinator_pubkey, &payload)
        .map_err(|e| anyhow::anyhow!("build dkg result: {e}"))?;

    relay
        .send_event_builder(builder)
        .await
        .map_err(|e| anyhow::anyhow!("send dkg result: {e}"))?;

    tracing::info!(
        session_id = %session_id,
        group_pubkey_hash = %group_pubkey_hash,
        "sent DKG result to coordinator"
    );

    // Hot-swap the in-memory signing identity so signing works immediately
    {
        let mut id = identity.write().await;
        id.key_package = Some(key_package);
        id.signer_id = Some(my_signer_id);
    }

    tracing::info!(
        session_id = %session_id,
        signer_id = my_signer_id,
        "DKG complete! New keys active in memory."
    );

    // Persist KeyPackage to config file (best-effort — may fail in read-only containers)
    match config::save_dkg_result(config_path, my_signer_id, &identity).await {
        Ok(()) => tracing::info!(signer_id = my_signer_id, "DKG keys saved to config file"),
        Err(e) => tracing::warn!(signer_id = my_signer_id, error = %e, "could not save DKG keys to config (keys are active in memory)"),
    }

    // Reset DKG state
    state.reset();

    Ok(())
}

// ---------------------------------------------------------------------------
// Unit tests for signer-side policy checks
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_HASH: &str =
        "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
    const NOW: u64 = 1_700_000_000;

    #[test]
    fn accepts_timestamp_within_drift_window() {
        let out = validate_and_recompute_message(1, NOW, VALID_HASH, NOW, 60);
        assert!(out.is_ok(), "expected OK, got: {:?}", out);
        let msg = out.unwrap();
        // Canonical hash must be deterministic for the same pre-image.
        let again = validate_and_recompute_message(1, NOW, VALID_HASH, NOW, 60).unwrap();
        assert_eq!(msg, again);
    }

    #[test]
    fn accepts_timestamp_at_boundary() {
        // exactly +60s and -60s from "now" must still be accepted.
        assert!(validate_and_recompute_message(1, NOW + 60, VALID_HASH, NOW, 60).is_ok());
        assert!(validate_and_recompute_message(1, NOW - 60, VALID_HASH, NOW, 60).is_ok());
    }

    #[test]
    fn rejects_backdated_timestamp() {
        // One hour in the past must be rejected.
        let err = validate_and_recompute_message(1, NOW - 3600, VALID_HASH, NOW, 60)
            .unwrap_err();
        assert!(err.contains("drift"), "unexpected error: {}", err);
    }

    #[test]
    fn rejects_postdated_timestamp() {
        // Five minutes in the future must be rejected.
        let err = validate_and_recompute_message(1, NOW + 300, VALID_HASH, NOW, 60)
            .unwrap_err();
        assert!(err.contains("drift"), "unexpected error: {}", err);
    }

    #[test]
    fn rejects_bad_file_hash_length() {
        let short = "abcd";
        let err = validate_and_recompute_message(1, NOW, short, NOW, 60).unwrap_err();
        assert!(err.contains("64-char"), "unexpected error: {}", err);
    }

    #[test]
    fn rejects_non_hex_file_hash() {
        let non_hex = "z".repeat(64);
        let err =
            validate_and_recompute_message(1, NOW, &non_hex, NOW, 60).unwrap_err();
        assert!(err.contains("hex"), "unexpected error: {}", err);
    }

    #[test]
    fn recomputed_hash_matches_client_side_verify() {
        // The signer's recomputed hash must equal what a TimestampToken client
        // (e.g. the CLI) computes for the same pre-image. If these ever drift
        // apart, tokens would fail verification.
        let signer_hash =
            validate_and_recompute_message(42, NOW, VALID_HASH, NOW, 60).unwrap();
        let client_token = TimestampToken {
            serial_number: 42,
            timestamp: NOW,
            file_hash: VALID_HASH.to_string(),
            signature: String::new(),
            group_public_key: String::new(),
        };
        let client_hash = client_token.compute_message_hash().unwrap();
        assert_eq!(signer_hash, client_hash);
    }
}
