use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionAnnounce {
    pub session_id: Uuid,
    pub message: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_announce_round_trip() {
        let sa = SessionAnnounce {
            session_id: Uuid::new_v4(),
            message: "sign this".into(),
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
}
