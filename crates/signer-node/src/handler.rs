use anyhow::{Context, Result};
use frost_secp256k1_tr::keys::KeyPackage;
use nostr_sdk::prelude::*;
use nostr_sdk::RelayPoolNotification;

use common::{KIND_ROUND2_PAYLOAD, KIND_SESSION_ANNOUNCE};
use frost_core::secp256k1::{Secp256k1, Nonce};
use frost_core::ThresholdScheme;
use nostr_transport::events::{
    build_partial_signature, build_round1_commitment, parse_round2_payload, parse_session_announce,
};
use nostr_transport::relay::NostrRelay;
use nostr_transport::types::{PartialSignature, Round1Commitment};

use crate::nonce_map::NonceMap;

pub async fn run_event_loop(
    relay: &NostrRelay,
    key_package: &KeyPackage,
    signer_id: u16,
    coordinator_pubkey: &PublicKey,
    nonce_map: &NonceMap,
    interactive: bool,
) -> Result<()> {
    let mut notifications = relay.notifications();

    tracing::info!(signer_id, "event loop started, listening for signing sessions...");

    loop {
        match notifications.recv().await {
            Ok(RelayPoolNotification::Event { event, .. }) => {
                let kind = event.kind().as_u16();
                let sender = event.author();

                tracing::debug!(kind, %sender, "received event");

                // Only process events from the coordinator
                if sender != *coordinator_pubkey {
                    tracing::trace!(kind, %sender, "ignoring event from unknown sender");
                    continue;
                }

                let result = match kind {
                    KIND_SESSION_ANNOUNCE => {
                        handle_session_announce(
                            relay,
                            &event,
                            key_package,
                            signer_id,
                            coordinator_pubkey,
                            nonce_map,
                            interactive,
                        )
                        .await
                    }
                    KIND_ROUND2_PAYLOAD => {
                        handle_round2_payload(
                            relay,
                            &event,
                            key_package,
                            signer_id,
                            coordinator_pubkey,
                            nonce_map,
                        )
                        .await
                    }
                    _ => {
                        tracing::trace!(kind, "ignoring irrelevant event kind");
                        Ok(())
                    }
                };

                if let Err(e) = result {
                    tracing::error!(kind, error = %e, "handler error");
                }
            }
            Ok(RelayPoolNotification::Shutdown) => {
                tracing::warn!("relay connection shut down");
                break;
            }
            Ok(_) => {} // RelayPoolNotification::Message etc.
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

async fn handle_session_announce(
    relay: &NostrRelay,
    event: &Event,
    key_package: &KeyPackage,
    signer_id: u16,
    coordinator_pubkey: &PublicKey,
    nonce_map: &NonceMap,
    interactive: bool,
) -> Result<()> {
    let announce = parse_session_announce(event, relay.keys())
        .context("failed to parse SessionAnnounce")?;

    tracing::debug!(
        session_id = %announce.session_id,
        message = %announce.message,
        k = announce.k,
        n = announce.n,
        "decrypted SessionAnnounce"
    );

    // Interactive mode: ask user for approval
    if interactive {
        eprintln!(
            "\n--- Signing request ---\n  Session: {}\n  Message: {}\n  Threshold: {}/{}\nApprove? [y/N] ",
            announce.session_id, announce.message, announce.k, announce.n
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

    // Store nonce BEFORE sending commitment (prevent race with Round2)
    if !nonce_map.insert(announce.session_id, nonce).await {
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
    Ok(())
}

async fn handle_round2_payload(
    relay: &NostrRelay,
    event: &Event,
    key_package: &KeyPackage,
    signer_id: u16,
    coordinator_pubkey: &PublicKey,
    nonce_map: &NonceMap,
) -> Result<()> {
    let round2 = parse_round2_payload(event, relay.keys())
        .context("failed to parse Round2Payload")?;

    tracing::debug!(session_id = %round2.session_id, "decrypted Round2Payload");

    // Retrieve the stored nonce (single-use)
    let nonce = match nonce_map.take(&round2.session_id).await {
        Some(n) => n,
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
    Ok(())
}
