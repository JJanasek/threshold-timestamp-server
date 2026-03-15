use nostr_sdk::prelude::*;
use uuid::Uuid;

use crate::{KIND_PARTIAL_SIG, KIND_ROUND1_COMMITMENT, KIND_ROUND2_PAYLOAD, KIND_SESSION_ANNOUNCE};

/// Filter for events that a **coordinator** needs to receive:
/// - Round 1 commitments (20002) from signers
/// - Partial signatures (20004) from signers
///
/// Optionally scoped to a specific session via the `"s"` tag.
pub fn coordinator_filter(
    coordinator_pubkey: &PublicKey,
    session_id: Option<Uuid>,
) -> Filter {
    let mut f = Filter::new()
        .kinds([
            Kind::from(KIND_ROUND1_COMMITMENT),
            Kind::from(KIND_PARTIAL_SIG),
        ])
        .custom_tag(
            SingleLetterTag::lowercase(Alphabet::P),
            [coordinator_pubkey.to_string()],
        );
    if let Some(sid) = session_id {
        f = f.custom_tag(
            SingleLetterTag::lowercase(Alphabet::S),
            [sid.to_string()],
        );
    }
    f
}

/// Filter for events that a **signer** needs to receive:
/// - Session announcements (20001) from the coordinator
/// - Round 2 payloads (20003) from the coordinator
pub fn signer_filter(signer_pubkey: &PublicKey) -> Filter {
    Filter::new()
        .kinds([
            Kind::from(KIND_SESSION_ANNOUNCE),
            Kind::from(KIND_ROUND2_PAYLOAD),
        ])
        .custom_tag(
            SingleLetterTag::lowercase(Alphabet::P),
            [signer_pubkey.to_string()],
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::*;
    use crate::types::*;

    /// Build a signed event for testing filter matching.
    fn signed_event(builder: EventBuilder, keys: &Keys) -> Event {
        builder.to_event(keys).unwrap()
    }

    #[test]
    fn coordinator_filter_matches_round1_and_partial_sig() {
        let coordinator = Keys::generate();
        let signer = Keys::generate();
        let sid = Uuid::new_v4();

        let filter = coordinator_filter(&coordinator.public_key(), Some(sid));

        // Round 1 commitment → should match
        let r1 = Round1Commitment {
            session_id: sid,
            signer_id: 1,
            commitment: serde_json::json!({}),
        };
        let r1_event = signed_event(
            build_round1_commitment(&signer, &coordinator.public_key(), &r1).unwrap(),
            &signer,
        );
        assert!(filter.match_event(&r1_event));

        // Partial signature → should match
        let ps = PartialSignature {
            session_id: sid,
            signer_id: 1,
            signature_share: serde_json::json!({}),
        };
        let ps_event = signed_event(
            build_partial_signature(&signer, &coordinator.public_key(), &ps).unwrap(),
            &signer,
        );
        assert!(filter.match_event(&ps_event));
    }

    #[test]
    fn coordinator_filter_rejects_wrong_session() {
        let coordinator = Keys::generate();
        let signer = Keys::generate();
        let sid = Uuid::new_v4();
        let other_sid = Uuid::new_v4();

        let filter = coordinator_filter(&coordinator.public_key(), Some(sid));

        let r1 = Round1Commitment {
            session_id: other_sid,
            signer_id: 1,
            commitment: serde_json::json!({}),
        };
        let event = signed_event(
            build_round1_commitment(&signer, &coordinator.public_key(), &r1).unwrap(),
            &signer,
        );
        assert!(!filter.match_event(&event));
    }

    #[test]
    fn coordinator_filter_rejects_wrong_recipient() {
        let coordinator = Keys::generate();
        let other_coordinator = Keys::generate();
        let signer = Keys::generate();
        let sid = Uuid::new_v4();

        let filter = coordinator_filter(&coordinator.public_key(), Some(sid));

        // Event tagged to a different coordinator
        let r1 = Round1Commitment {
            session_id: sid,
            signer_id: 1,
            commitment: serde_json::json!({}),
        };
        let event = signed_event(
            build_round1_commitment(&signer, &other_coordinator.public_key(), &r1).unwrap(),
            &signer,
        );
        assert!(!filter.match_event(&event));
    }

    #[test]
    fn coordinator_filter_without_session_matches_any_session() {
        let coordinator = Keys::generate();
        let signer = Keys::generate();

        let filter = coordinator_filter(&coordinator.public_key(), None);

        let r1 = Round1Commitment {
            session_id: Uuid::new_v4(),
            signer_id: 1,
            commitment: serde_json::json!({}),
        };
        let event = signed_event(
            build_round1_commitment(&signer, &coordinator.public_key(), &r1).unwrap(),
            &signer,
        );
        assert!(filter.match_event(&event));
    }

    #[test]
    fn signer_filter_matches_announce_and_round2() {
        let coordinator = Keys::generate();
        let signer = Keys::generate();

        let filter = signer_filter(&signer.public_key());

        // Session announce → should match
        let sa = SessionAnnounce {
            session_id: Uuid::new_v4(),
            message: "m".into(),
            k: 2,
            n: 3,
        };
        let sa_event = signed_event(
            build_session_announce(&coordinator, &signer.public_key(), &sa).unwrap(),
            &coordinator,
        );
        assert!(filter.match_event(&sa_event));

        // Round 2 → should match
        let r2 = Round2Payload {
            session_id: Uuid::new_v4(),
            signing_package: serde_json::json!({}),
        };
        let r2_event = signed_event(
            build_round2_payload(&coordinator, &signer.public_key(), &r2).unwrap(),
            &coordinator,
        );
        assert!(filter.match_event(&r2_event));
    }

    #[test]
    fn signer_filter_rejects_coordinator_bound_events() {
        let coordinator = Keys::generate();
        let signer = Keys::generate();

        let filter = signer_filter(&signer.public_key());

        // Round 1 commitment (kind 20002, tagged to coordinator) → should NOT match signer filter
        let r1 = Round1Commitment {
            session_id: Uuid::new_v4(),
            signer_id: 1,
            commitment: serde_json::json!({}),
        };
        let event = signed_event(
            build_round1_commitment(&signer, &coordinator.public_key(), &r1).unwrap(),
            &signer,
        );
        assert!(!filter.match_event(&event));
    }
}
