//! [`ThresholdScheme`] for secp256k1-tr backed by the ZF FROST library.
//!
//! Ciphersuite: `FROST(secp256k1-tr, SHA-256)` — BIP-340 / Nostr compatible.
//! <https://docs.rs/frost-secp256k1-tr>

use std::collections::BTreeMap;

use frost_secp256k1_tr::{
    self as frost,
    keys::{IdentifierList, KeyPackage, PublicKeyPackage},
    round1::{SigningCommitments, SigningNonces},
    round2::SignatureShare,
    Signature, SigningPackage,
};
use rand::thread_rng;

use crate::crypto::{CryptoError, Secp256k1, ThresholdScheme};


/// Secret nonce pair for one signing session (never leave the node).
pub struct Nonce {
    pub identifier: frost::Identifier,
    pub(crate) nonces: SigningNonces,
    /// Pre-computed public half; extracted by [`nonce_commitment`].
    pub(crate) commitments: SigningCommitments,
}

/// Public nonce commitment broadcast to all participants in round 1.
#[derive(Clone)]
pub struct Commitment {
    pub identifier: frost::Identifier,
    pub(crate) inner: SigningCommitments,
}

impl AsRef<[u8]> for Commitment {
    fn as_ref(&self) -> &[u8] {
        todo!("wire serialisation — implement when HTTP transport is added")
    }
}

/// One signer's partial signature share (round 2 output).
pub struct PartialSig {
    pub identifier: frost::Identifier,
    pub(crate) inner: SignatureShare,
}

/// Aggregated BIP-340 Schnorr signature (64 bytes).
pub struct FrostSignature(pub Signature);

impl AsRef<[u8]> for FrostSignature {
    fn as_ref(&self) -> &[u8] {
        todo!("wire serialisation — implement when HTTP transport is added")
    }
}

/// Group public key package (verifying key + per-signer verifying keys).
#[derive(Clone)]
pub struct GroupKey(pub PublicKeyPackage);

impl AsRef<[u8]> for GroupKey {
    fn as_ref(&self) -> &[u8] {
        todo!("wire serialisation — implement when HTTP transport is added")
    }
}


impl ThresholdScheme for Secp256k1 {
    type PublicKey        = GroupKey;
    type SecretShare      = KeyPackage;
    type Nonce            = Nonce;
    type NonceCommitment  = Commitment;
    type PartialSignature = PartialSig;
    type Signature        = FrostSignature;

    /// Trusted dealer keygen via `frost::keys::generate_with_dealer`.
    /// Each `SecretShare` is converted to a `KeyPackage` in place.
    fn generate_shares(n: u32, k: u32) -> Result<(Vec<KeyPackage>, GroupKey), CryptoError> {
        if k < 1 || k > n {
            return Err(CryptoError::InvalidThreshold { k, n });
        }

        let (shares, pubkey_package) = frost::keys::generate_with_dealer(
            n as u16,
            k as u16,
            IdentifierList::Default,
            &mut thread_rng(),
        )
        .map_err(|e| CryptoError::Frost(e.to_string()))?;

        // Convert SecretShare → KeyPackage for each participant.
        let key_packages = shares
            .into_values()
            .map(|s| KeyPackage::try_from(s).map_err(|e| CryptoError::Frost(e.to_string())))
            .collect::<Result<Vec<_>, _>>()?;

        Ok((key_packages, GroupKey(pubkey_package)))
    }

    /// Round 1: `frost::round1::commit` binds the nonce to the signing share.
    fn generate_nonce(share: &KeyPackage) -> Nonce {
        let (nonces, commitments) = frost::round1::commit(share.signing_share(), &mut thread_rng());
        Nonce { identifier: *share.identifier(), nonces, commitments }
    }

    /// Extract the public commitment (safe to broadcast to coordinator).
    fn nonce_commitment(nonce: &Nonce) -> Commitment {
        Commitment { identifier: nonce.identifier, inner: nonce.commitments.clone() }
    }

    /// Round 2: build a `SigningPackage` from collected commitments + message,
    /// then call `frost::round2::sign`.
    fn partial_sign(
        share: &KeyPackage,
        nonce: &Nonce,
        _pubkey: &GroupKey,
        commitments: &[Commitment],
        msg: &[u8; 32],
    ) -> Result<PartialSig, CryptoError> {
        let commitments_map: BTreeMap<frost::Identifier, SigningCommitments> =
            commitments.iter().map(|c| (c.identifier, c.inner.clone())).collect();

        let signing_package = SigningPackage::new(commitments_map, msg);

        let sig_share = frost::round2::sign(&signing_package, &nonce.nonces, share)
            .map_err(|e| CryptoError::Frost(e.to_string()))?;

        Ok(PartialSig { identifier: nonce.identifier, inner: sig_share })
    }

    /// Aggregation: reconstruct `SigningPackage`, then call `frost::aggregate`.
    fn aggregate(
        partial_sigs: &[PartialSig],
        commitments: &[Commitment],
        pubkey: &GroupKey,
        msg: &[u8; 32],
    ) -> Result<FrostSignature, CryptoError> {
        let commitments_map: BTreeMap<frost::Identifier, SigningCommitments> =
            commitments.iter().map(|c| (c.identifier, c.inner.clone())).collect();

        let signing_package = SigningPackage::new(commitments_map, msg);

        let shares_map: BTreeMap<frost::Identifier, SignatureShare> =
            partial_sigs.iter().map(|ps| (ps.identifier, ps.inner.clone())).collect();

        let signature = frost::aggregate(&signing_package, &shares_map, &pubkey.0)
            .map_err(|e| CryptoError::Frost(e.to_string()))?;

        Ok(FrostSignature(signature))
    }

    /// Verify against the group verifying key.
    fn verify(pubkey: &GroupKey, msg: &[u8; 32], sig: &FrostSignature) -> bool {
        pubkey.0.verifying_key().verify(msg, &sig.0).is_ok()
    }
}
