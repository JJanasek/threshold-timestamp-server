use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionAnnounce {
    pub session_id: Uuid,
    /// Monotonic serial number assigned by the coordinator.
    pub serial_number: u64,
    /// Unix timestamp (seconds) at which the coordinator opened the session.
    /// Signers validate this is within their configured drift window.
    pub timestamp: u64,
    /// Hex-encoded SHA-256 of the document being timestamped (64 hex chars).
    pub file_hash: String,
    pub k: usize,
    pub n: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Round1Commitment {
    pub session_id: Uuid,
    pub signer_id: u16,
    pub commitment: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Round2Payload {
    pub session_id: Uuid,
    pub signing_package: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PartialSignature {
    pub session_id: Uuid,
    pub signer_id: u16,
    pub signature_share: serde_json::Value,
}

// ---------------------------------------------------------------------------
// DKG Message Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DkgParticipant {
    pub signer_id: u16,
    pub npub: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DkgAnnounce {
    pub session_id: Uuid,
    pub k: u16,
    pub n: u16,
    pub participants: Vec<DkgParticipant>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DkgRound1 {
    pub session_id: Uuid,
    pub signer_id: u16,
    pub package: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DkgRound1Broadcast {
    pub session_id: Uuid,
    pub packages: BTreeMap<u16, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DkgRound2 {
    pub session_id: Uuid,
    pub sender_id: u16,
    pub package: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DkgResult {
    pub session_id: Uuid,
    pub signer_id: u16,
    pub group_pubkey_hash: String,
    /// Hex-encoded serialized PublicKeyPackage (all signers produce the same one).
    pub public_key_package: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_announce_round_trip() {
        let sa = SessionAnnounce {
            session_id: Uuid::new_v4(),
            serial_number: 42,
            timestamp: 1_700_000_000,
            file_hash: "a".repeat(64),
            k: 2,
            n: 3,
        };
        let json = serde_json::to_string(&sa).unwrap();
        let back: SessionAnnounce = serde_json::from_str(&json).unwrap();
        assert_eq!(sa, back);
    }

    #[test]
    fn round1_commitment_round_trip() {
        let r1 = Round1Commitment {
            session_id: Uuid::new_v4(),
            signer_id: 1,
            commitment: serde_json::json!({"hiding": "aabb", "binding": "ccdd"}),
        };
        let json = serde_json::to_string(&r1).unwrap();
        let back: Round1Commitment = serde_json::from_str(&json).unwrap();
        assert_eq!(r1, back);
    }

    #[test]
    fn round2_payload_round_trip() {
        let r2 = Round2Payload {
            session_id: Uuid::new_v4(),
            signing_package: serde_json::json!({"commitments": [], "message": "deadbeef"}),
        };
        let json = serde_json::to_string(&r2).unwrap();
        let back: Round2Payload = serde_json::from_str(&json).unwrap();
        assert_eq!(r2, back);
    }

    #[test]
    fn partial_signature_round_trip() {
        let ps = PartialSignature {
            session_id: Uuid::new_v4(),
            signer_id: 2,
            signature_share: serde_json::json!({"share": "0102030405"}),
        };
        let json = serde_json::to_string(&ps).unwrap();
        let back: PartialSignature = serde_json::from_str(&json).unwrap();
        assert_eq!(ps, back);
    }

    #[test]
    fn dkg_announce_round_trip() {
        let da = DkgAnnounce {
            session_id: Uuid::new_v4(),
            k: 2,
            n: 3,
            participants: vec![
                DkgParticipant { signer_id: 1, npub: "npub1abc".into() },
                DkgParticipant { signer_id: 2, npub: "npub1def".into() },
            ],
        };
        let json = serde_json::to_string(&da).unwrap();
        let back: DkgAnnounce = serde_json::from_str(&json).unwrap();
        assert_eq!(da, back);
    }

    #[test]
    fn dkg_round1_round_trip() {
        let r1 = DkgRound1 {
            session_id: Uuid::new_v4(),
            signer_id: 1,
            package: serde_json::json!({"commitment": "aabb"}),
        };
        let json = serde_json::to_string(&r1).unwrap();
        let back: DkgRound1 = serde_json::from_str(&json).unwrap();
        assert_eq!(r1, back);
    }

    #[test]
    fn dkg_round1_broadcast_round_trip() {
        let mut packages = BTreeMap::new();
        packages.insert(1u16, serde_json::json!({"pkg": "data1"}));
        packages.insert(2u16, serde_json::json!({"pkg": "data2"}));
        let rb = DkgRound1Broadcast {
            session_id: Uuid::new_v4(),
            packages,
        };
        let json = serde_json::to_string(&rb).unwrap();
        let back: DkgRound1Broadcast = serde_json::from_str(&json).unwrap();
        assert_eq!(rb, back);
    }

    #[test]
    fn dkg_round2_round_trip() {
        let r2 = DkgRound2 {
            session_id: Uuid::new_v4(),
            sender_id: 1,
            package: serde_json::json!({"share": "ccdd"}),
        };
        let json = serde_json::to_string(&r2).unwrap();
        let back: DkgRound2 = serde_json::from_str(&json).unwrap();
        assert_eq!(r2, back);
    }

    #[test]
    fn dkg_result_round_trip() {
        let dr = DkgResult {
            session_id: Uuid::new_v4(),
            signer_id: 1,
            group_pubkey_hash: "deadbeef".into(),
            public_key_package: "aabbccdd".into(),
        };
        let json = serde_json::to_string(&dr).unwrap();
        let back: DkgResult = serde_json::from_str(&json).unwrap();
        assert_eq!(dr, back);
    }
}
