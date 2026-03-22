use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[cfg(feature = "collector-client")]
pub mod event_client;

// -----------------------------------------------------------------------------
// Constants (Nostr Kinds & Tags)
// -----------------------------------------------------------------------------

/// Ephemeral event: coordinator announces a new signing session to signers.
pub const KIND_SESSION_ANNOUNCE: u16 = 20001;
/// Ephemeral event: signer sends Round 1 commitment to coordinator.
pub const KIND_ROUND1_COMMITMENT: u16 = 20002;
/// Ephemeral event: coordinator sends Round 2 signing package to signers.
pub const KIND_ROUND2_PAYLOAD: u16 = 20003;
/// Ephemeral event: signer sends partial signature to coordinator.
pub const KIND_PARTIAL_SIG: u16 = 20004;
/// Regular event: published timestamp token (NIP-01 kind 1 note).
pub const KIND_TIMESTAMP_TOKEN: u16 = 1;

// DKG protocol event kinds
/// Ephemeral event: coordinator announces a DKG session to signers.
pub const KIND_DKG_ANNOUNCE: u16 = 20005;
/// Ephemeral event: signer sends DKG round 1 package to coordinator.
pub const KIND_DKG_ROUND1: u16 = 20006;
/// Ephemeral event: coordinator broadcasts all round 1 packages to signers.
pub const KIND_DKG_ROUND1_BROADCAST: u16 = 20007;
/// Ephemeral event: signer sends DKG round 2 package to another signer (peer-to-peer).
pub const KIND_DKG_ROUND2: u16 = 20008;
/// Ephemeral event: signer sends DKG result confirmation to coordinator.
pub const KIND_DKG_RESULT: u16 = 20009;

/// Single-letter tag used for session ID filtering (relay-compatible).
pub const TAG_SESSION: &str = "s";

// -----------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum CommonError {
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Hex decoding error: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("Crypto error: {0}")]
    Crypto(String),
}

// -----------------------------------------------------------------------------
// Core Data Structures
// -----------------------------------------------------------------------------

/// The final product: A trusted timestamp token proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampToken {
    pub serial_number: u64,
    pub timestamp: u64,
    pub file_hash: String,        // Hex encoded SHA256 of the original document
    pub signature: String,        // Hex encoded Schnorr signature (64 bytes)
    pub group_public_key: String, // Hex encoded X-only public key (32 bytes)
}

impl TimestampToken {
    /// Reconstructs the message that was definitively signed.
    /// Format: SHA-256("FROST-TIMESTAMP-V1\x00" || serial_number || timestamp || file_hash_bytes)
    pub fn compute_message_hash(&self) -> Result<[u8; 32], CommonError> {
        let mut hasher = Sha256::new();
        hasher.update(b"FROST-TIMESTAMP-V1\x00");
        hasher.update(self.serial_number.to_be_bytes());
        hasher.update(self.timestamp.to_be_bytes());
        
        let file_hash_bytes = hex::decode(&self.file_hash)?;
        hasher.update(&file_hash_bytes);

        Ok(hasher.finalize().into())
    }

    /// Verify the validity of the timestamp using the group public key.
    /// Only available on non-WASM targets (requires secp256k1 C library).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn verify(&self) -> Result<bool, CommonError> {
        use secp256k1::{schnorr::Signature, XOnlyPublicKey};

        // 1. Recompute the message hash that the servers supposedly signed
        let msg_hash = self.compute_message_hash()?;
        let msg = secp256k1::Message::from_digest_slice(&msg_hash)
            .map_err(|e| CommonError::Crypto(e.to_string()))?;

        // 2. Parse the aggregated FROST signature
        let sig_bytes = hex::decode(&self.signature)?;
        let signature = Signature::from_slice(&sig_bytes)
            .map_err(|e| CommonError::Crypto(e.to_string()))?;

        // 3. Parse the Group Public Key
        let pk_bytes = hex::decode(&self.group_public_key)?;
        let public_key = XOnlyPublicKey::from_slice(&pk_bytes)
            .map_err(|e| CommonError::Crypto(e.to_string()))?;

        // 4. Perform Schnorr verification
        let secp = secp256k1::Secp256k1::verification_only();
        secp.verify_schnorr(&signature, &msg, &public_key)
            .map(|_| true)
            .map_err(|_| CommonError::InvalidSignature)
    }
}

// -----------------------------------------------------------------------------
// Collector Event
// -----------------------------------------------------------------------------

/// An audit event sent to the collector service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorEvent {
    pub node_name: String,
    pub session_id: Option<String>,
    pub message: String,
    pub timestamp: u64,
}

// -----------------------------------------------------------------------------
// Configuration
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignerConfig {
    pub key_package: Option<String>, // JSON serialized KeyPackage (secret share); None before DKG
    pub signer_id: Option<u16>,      // Assigned during DKG; derived from key_package if absent
    pub coordinator_npub: String,    // To know which events to listen to
    pub relay_urls: Vec<String>,
    pub nsec: Option<String>,        // Nostr secret key (bech32); generated if absent
    pub collector_url: Option<String>, // URL of the event collector service
}

