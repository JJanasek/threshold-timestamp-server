pub mod secp256k1;

use std::fmt;

/// Hash arbitrary bytes to a 32-byte SHA-256 digest.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    sha2::Sha256::digest(data).into()
}

// ─────────────────────────── Error type ─────────────────────────────────────

#[derive(Debug, PartialEq, Eq)]
pub enum CryptoError {
    /// n < 1 or k > n or k < 1.
    InvalidThreshold { k: u32, n: u32 },
    /// Participant indices must be distinct positive integers.
    InvalidIndices,
    /// Not enough partial signatures or commitments provided.
    InsufficientShares { got: usize, need: u32 },
    /// A scalar, point, or key could not be decoded.
    InvalidEncoding,
    /// Signature verification failed.
    VerificationFailed,
    /// Underlying FROST library error.
    Frost(String),
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidThreshold { k, n } =>
                write!(f, "invalid threshold: need 1 ≤ k ≤ n, got k={k}, n={n}"),
            Self::InvalidIndices =>
                write!(f, "participant indices must be distinct positive integers"),
            Self::InsufficientShares { got, need } =>
                write!(f, "insufficient shares: need {need}, got {got}"),
            Self::InvalidEncoding =>
                write!(f, "could not decode scalar, point, or key"),
            Self::VerificationFailed =>
                write!(f, "signature verification failed"),
            Self::Frost(e) =>
                write!(f, "FROST error: {e}"),
        }
    }
}

impl std::error::Error for CryptoError {}

// ──────────────────────── Core trait ────────────────────────────────────────

/// Threshold Schnorr signature scheme over an arbitrary curve.
pub trait ThresholdScheme {
    type PublicKey: AsRef<[u8]> + Clone;
    type SecretShare;
    type Nonce;
    type NonceCommitment: AsRef<[u8]> + Clone;
    type PartialSignature;
    type Signature: AsRef<[u8]>;

    /// Trusted dealer: generate `n` key shares with signing threshold `k`.
    /// Returns `(shares, group_public_key)`. Share indices are `1..=n`.
    fn generate_shares(n: u32, k: u32) -> Result<(Vec<Self::SecretShare>, Self::PublicKey), CryptoError>;

    /// Generate a fresh random nonce for this signing session.
    /// Takes the share because FROST binds the nonce to the signer's key.
    /// Never reuse a nonce across sessions.
    fn generate_nonce(share: &Self::SecretShare) -> Self::Nonce;

    /// Extract the public nonce commitment from a nonce (safe to broadcast).
    fn nonce_commitment(nonce: &Self::Nonce) -> Self::NonceCommitment;

    /// Compute a partial signature.
    /// `commitments` is the full set of k participants' nonce commitments from round 1.
    fn partial_sign(
        share: &Self::SecretShare,
        nonce: &Self::Nonce,
        pubkey: &Self::PublicKey,
        commitments: &[Self::NonceCommitment],
        msg: &[u8; 32],
    ) -> Result<Self::PartialSignature, CryptoError>;

    /// Combine k partial signatures into a final signature.
    /// Needs `pubkey` and `msg` to reconstruct the FROST SigningPackage.
    fn aggregate(
        partial_sigs: &[Self::PartialSignature],
        commitments: &[Self::NonceCommitment],
        pubkey: &Self::PublicKey,
        msg: &[u8; 32],
    ) -> Result<Self::Signature, CryptoError>;

    /// Verify a signature. Returns `true` iff valid.
    fn verify(pubkey: &Self::PublicKey, msg: &[u8; 32], sig: &Self::Signature) -> bool;
}

// ─────────────────────── Curve marker ───────────────────────────────────────

/// Marker for the secp256k1-tr ciphersuite (BIP-340 Schnorr / Nostr-compatible).
pub struct Secp256k1;