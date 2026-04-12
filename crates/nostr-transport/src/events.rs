use nostr_sdk::prelude::*;
use uuid::Uuid;

use crate::encrypt::{decrypt_payload, encrypt_payload, EncryptError};
use crate::types::*;
use crate::{
    KIND_PARTIAL_SIG, KIND_ROUND1_COMMITMENT, KIND_ROUND2_PAYLOAD, KIND_SESSION_ANNOUNCE,
    KIND_TIMESTAMP_TOKEN, KIND_DKG_ANNOUNCE, KIND_DKG_ROUND1, KIND_DKG_ROUND1_BROADCAST,
    KIND_DKG_ROUND2, KIND_DKG_RESULT,
};

#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("encryption error: {0}")]
    Encrypt(#[from] EncryptError),
    #[error("unexpected event kind: expected {expected}, got {got}")]
    WrongKind { expected: u16, got: u16 },
}

// ---------------------------------------------------------------------------
// Tag helpers
// ---------------------------------------------------------------------------

fn session_tag(session_id: &Uuid) -> Tag {
    Tag::custom(
        TagKind::SingleLetter(SingleLetterTag::lowercase(Alphabet::S)),
        [session_id.to_string()],
    )
}

fn recipient_tag(pubkey: &PublicKey) -> Tag {
    Tag::public_key(*pubkey)
}

// ---------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------

pub fn build_session_announce(
    sender_keys: &Keys,
    recipient_pubkey: &PublicKey,
    payload: &SessionAnnounce,
) -> Result<EventBuilder, EventError> {
    let encrypted = encrypt_payload(sender_keys, recipient_pubkey, payload)?;
    Ok(EventBuilder::new(
        Kind::from(KIND_SESSION_ANNOUNCE),
        encrypted,
        [
            recipient_tag(recipient_pubkey),
            session_tag(&payload.session_id),
        ],
    ))
}

pub fn build_round1_commitment(
    sender_keys: &Keys,
    coordinator_pubkey: &PublicKey,
    payload: &Round1Commitment,
) -> Result<EventBuilder, EventError> {
    let encrypted = encrypt_payload(sender_keys, coordinator_pubkey, payload)?;
    Ok(EventBuilder::new(
        Kind::from(KIND_ROUND1_COMMITMENT),
        encrypted,
        [
            recipient_tag(coordinator_pubkey),
            session_tag(&payload.session_id),
        ],
    ))
}

pub fn build_round2_payload(
    sender_keys: &Keys,
    recipient_pubkey: &PublicKey,
    payload: &Round2Payload,
) -> Result<EventBuilder, EventError> {
    let encrypted = encrypt_payload(sender_keys, recipient_pubkey, payload)?;
    Ok(EventBuilder::new(
        Kind::from(KIND_ROUND2_PAYLOAD),
        encrypted,
        [
            recipient_tag(recipient_pubkey),
            session_tag(&payload.session_id),
        ],
    ))
}

pub fn build_partial_signature(
    sender_keys: &Keys,
    coordinator_pubkey: &PublicKey,
    payload: &PartialSignature,
) -> Result<EventBuilder, EventError> {
    let encrypted = encrypt_payload(sender_keys, coordinator_pubkey, payload)?;
    Ok(EventBuilder::new(
        Kind::from(KIND_PARTIAL_SIG),
        encrypted,
        [
            recipient_tag(coordinator_pubkey),
            session_tag(&payload.session_id),
        ],
    ))
}

pub fn build_timestamp_token(content: &str) -> EventBuilder {
    EventBuilder::new(Kind::from(KIND_TIMESTAMP_TOKEN), content, [])
}

// ---------------------------------------------------------------------------
// DKG Builders
// ---------------------------------------------------------------------------

pub fn build_dkg_announce(
    sender_keys: &Keys,
    recipient_pubkey: &PublicKey,
    payload: &DkgAnnounce,
) -> Result<EventBuilder, EventError> {
    let encrypted = encrypt_payload(sender_keys, recipient_pubkey, payload)?;
    Ok(EventBuilder::new(
        Kind::from(KIND_DKG_ANNOUNCE),
        encrypted,
        [
            recipient_tag(recipient_pubkey),
            session_tag(&payload.session_id),
        ],
    ))
}

pub fn build_dkg_round1(
    sender_keys: &Keys,
    coordinator_pubkey: &PublicKey,
    payload: &DkgRound1,
) -> Result<EventBuilder, EventError> {
    let encrypted = encrypt_payload(sender_keys, coordinator_pubkey, payload)?;
    Ok(EventBuilder::new(
        Kind::from(KIND_DKG_ROUND1),
        encrypted,
        [
            recipient_tag(coordinator_pubkey),
            session_tag(&payload.session_id),
        ],
    ))
}

pub fn build_dkg_round1_broadcast(
    sender_keys: &Keys,
    recipient_pubkey: &PublicKey,
    payload: &DkgRound1Broadcast,
) -> Result<EventBuilder, EventError> {
    let encrypted = encrypt_payload(sender_keys, recipient_pubkey, payload)?;
    Ok(EventBuilder::new(
        Kind::from(KIND_DKG_ROUND1_BROADCAST),
        encrypted,
        [
            recipient_tag(recipient_pubkey),
            session_tag(&payload.session_id),
        ],
    ))
}

pub fn build_dkg_round2(
    sender_keys: &Keys,
    recipient_pubkey: &PublicKey,
    payload: &DkgRound2,
) -> Result<EventBuilder, EventError> {
    let encrypted = encrypt_payload(sender_keys, recipient_pubkey, payload)?;
    Ok(EventBuilder::new(
        Kind::from(KIND_DKG_ROUND2),
        encrypted,
        [
            recipient_tag(recipient_pubkey),
            session_tag(&payload.session_id),
        ],
    ))
}

pub fn build_dkg_result(
    sender_keys: &Keys,
    coordinator_pubkey: &PublicKey,
    payload: &DkgResult,
) -> Result<EventBuilder, EventError> {
    let encrypted = encrypt_payload(sender_keys, coordinator_pubkey, payload)?;
    Ok(EventBuilder::new(
        Kind::from(KIND_DKG_RESULT),
        encrypted,
        [
            recipient_tag(coordinator_pubkey),
            session_tag(&payload.session_id),
        ],
    ))
}

// ---------------------------------------------------------------------------
// Parsers
// ---------------------------------------------------------------------------

fn check_kind(event: &Event, expected: u16) -> Result<(), EventError> {
    let got = event.kind().as_u16();
    if got != expected {
        return Err(EventError::WrongKind { expected, got });
    }
    Ok(())
}

pub fn parse_session_announce(
    event: &Event,
    receiver_keys: &Keys,
) -> Result<SessionAnnounce, EventError> {
    check_kind(event, KIND_SESSION_ANNOUNCE)?;
    Ok(decrypt_payload(receiver_keys, &event.author(), event.content())?)
}

pub fn parse_round1_commitment(
    event: &Event,
    receiver_keys: &Keys,
) -> Result<Round1Commitment, EventError> {
    check_kind(event, KIND_ROUND1_COMMITMENT)?;
    Ok(decrypt_payload(receiver_keys, &event.author(), event.content())?)
}

pub fn parse_round2_payload(
    event: &Event,
    receiver_keys: &Keys,
) -> Result<Round2Payload, EventError> {
    check_kind(event, KIND_ROUND2_PAYLOAD)?;
    Ok(decrypt_payload(receiver_keys, &event.author(), event.content())?)
}

pub fn parse_partial_signature(
    event: &Event,
    receiver_keys: &Keys,
) -> Result<PartialSignature, EventError> {
    check_kind(event, KIND_PARTIAL_SIG)?;
    Ok(decrypt_payload(receiver_keys, &event.author(), event.content())?)
}

// ---------------------------------------------------------------------------
// DKG Parsers
// ---------------------------------------------------------------------------

pub fn parse_dkg_announce(
    event: &Event,
    receiver_keys: &Keys,
) -> Result<DkgAnnounce, EventError> {
    check_kind(event, KIND_DKG_ANNOUNCE)?;
    Ok(decrypt_payload(receiver_keys, &event.author(), event.content())?)
}

pub fn parse_dkg_round1(
    event: &Event,
    receiver_keys: &Keys,
) -> Result<DkgRound1, EventError> {
    check_kind(event, KIND_DKG_ROUND1)?;
    Ok(decrypt_payload(receiver_keys, &event.author(), event.content())?)
}

pub fn parse_dkg_round1_broadcast(
    event: &Event,
    receiver_keys: &Keys,
) -> Result<DkgRound1Broadcast, EventError> {
    check_kind(event, KIND_DKG_ROUND1_BROADCAST)?;
    Ok(decrypt_payload(receiver_keys, &event.author(), event.content())?)
}

pub fn parse_dkg_round2(
    event: &Event,
    receiver_keys: &Keys,
) -> Result<DkgRound2, EventError> {
    check_kind(event, KIND_DKG_ROUND2)?;
    Ok(decrypt_payload(receiver_keys, &event.author(), event.content())?)
}

pub fn parse_dkg_result(
    event: &Event,
    receiver_keys: &Keys,
) -> Result<DkgResult, EventError> {
    check_kind(event, KIND_DKG_RESULT)?;
    Ok(decrypt_payload(receiver_keys, &event.author(), event.content())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag_value(event: &Event, key: &str) -> Option<String> {
        event.tags.iter().find_map(|t| {
            let v = t.as_vec();
            if v.first().map(|s| s.as_str()) == Some(key) {
                v.get(1).cloned()
            } else {
                None
            }
        })
    }

    #[test]
    fn session_announce_build_parse_round_trip() {
        let coordinator = Keys::generate();
        let signer = Keys::generate();
        let payload = SessionAnnounce {
            session_id: Uuid::new_v4(),
            serial_number: 1,
            timestamp: 1_700_000_000,
            file_hash: "b".repeat(64),
            k: 3,
            n: 5,
        };

        let builder =
            build_session_announce(&coordinator, &signer.public_key(), &payload).unwrap();
        let event = builder.to_event(&coordinator).unwrap();

        // Verify kind
        assert_eq!(event.kind().as_u16(), KIND_SESSION_ANNOUNCE);

        // Verify tags
        assert_eq!(
            tag_value(&event, "p").unwrap(),
            signer.public_key().to_string()
        );
        assert_eq!(
            tag_value(&event, "s").unwrap(),
            payload.session_id.to_string()
        );

        // Parse back
        let parsed = parse_session_announce(&event, &signer).unwrap();
        assert_eq!(parsed, payload);
    }

    #[test]
    fn round1_commitment_build_parse_round_trip() {
        let signer = Keys::generate();
        let coordinator = Keys::generate();
        let payload = Round1Commitment {
            session_id: Uuid::new_v4(),
            signer_id: 42,
            commitment: serde_json::json!({"hiding": "aa", "binding": "bb"}),
        };

        let builder =
            build_round1_commitment(&signer, &coordinator.public_key(), &payload).unwrap();
        let event = builder.to_event(&signer).unwrap();

        assert_eq!(event.kind().as_u16(), KIND_ROUND1_COMMITMENT);
        let parsed = parse_round1_commitment(&event, &coordinator).unwrap();
        assert_eq!(parsed, payload);
    }

    #[test]
    fn round2_payload_build_parse_round_trip() {
        let coordinator = Keys::generate();
        let signer = Keys::generate();
        let payload = Round2Payload {
            session_id: Uuid::new_v4(),
            signing_package: serde_json::json!({"commitments": {}, "message": "cafe"}),
        };

        let builder =
            build_round2_payload(&coordinator, &signer.public_key(), &payload).unwrap();
        let event = builder.to_event(&coordinator).unwrap();

        assert_eq!(event.kind().as_u16(), KIND_ROUND2_PAYLOAD);
        let parsed = parse_round2_payload(&event, &signer).unwrap();
        assert_eq!(parsed, payload);
    }

    #[test]
    fn partial_signature_build_parse_round_trip() {
        let signer = Keys::generate();
        let coordinator = Keys::generate();
        let payload = PartialSignature {
            session_id: Uuid::new_v4(),
            signer_id: 7,
            signature_share: serde_json::json!({"share": "deadbeef"}),
        };

        let builder =
            build_partial_signature(&signer, &coordinator.public_key(), &payload).unwrap();
        let event = builder.to_event(&signer).unwrap();

        assert_eq!(event.kind().as_u16(), KIND_PARTIAL_SIG);
        let parsed = parse_partial_signature(&event, &coordinator).unwrap();
        assert_eq!(parsed, payload);
    }

    #[test]
    fn timestamp_token_is_plaintext_kind1() {
        let keys = Keys::generate();
        let builder = build_timestamp_token("signed timestamp data");
        let event = builder.to_event(&keys).unwrap();

        assert_eq!(event.kind().as_u16(), KIND_TIMESTAMP_TOKEN);
        assert_eq!(event.content(), "signed timestamp data");
    }

    #[test]
    fn parse_wrong_kind_returns_error() {
        let coordinator = Keys::generate();
        let signer = Keys::generate();
        let payload = SessionAnnounce {
            session_id: Uuid::new_v4(),
            serial_number: 2,
            timestamp: 1_700_000_000,
            file_hash: "c".repeat(64),
            k: 2,
            n: 3,
        };

        let builder =
            build_session_announce(&coordinator, &signer.public_key(), &payload).unwrap();
        let event = builder.to_event(&coordinator).unwrap();

        // Try to parse a kind-20001 event as round1_commitment (kind 20002)
        let result = parse_round1_commitment(&event, &signer);
        assert!(matches!(
            result,
            Err(EventError::WrongKind {
                expected: KIND_ROUND1_COMMITMENT,
                got: KIND_SESSION_ANNOUNCE,
            })
        ));
    }

    #[test]
    fn parse_with_wrong_receiver_fails() {
        let coordinator = Keys::generate();
        let signer = Keys::generate();
        let wrong_keys = Keys::generate();

        let payload = SessionAnnounce {
            session_id: Uuid::new_v4(),
            serial_number: 3,
            timestamp: 1_700_000_000,
            file_hash: "d".repeat(64),
            k: 2,
            n: 3,
        };

        let builder =
            build_session_announce(&coordinator, &signer.public_key(), &payload).unwrap();
        let event = builder.to_event(&coordinator).unwrap();

        let result = parse_session_announce(&event, &wrong_keys);
        assert!(result.is_err());
    }
}
