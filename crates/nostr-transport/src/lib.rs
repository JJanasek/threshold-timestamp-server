pub mod encrypt;
pub mod events;
pub mod filters;
pub mod relay;
pub mod types;

pub use nostr_sdk;

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

/// Single-letter tag used for session ID filtering (relay-compatible).
pub const TAG_SESSION: &str = "s";
