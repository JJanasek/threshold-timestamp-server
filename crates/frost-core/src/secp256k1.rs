//! Ciphersuite: `FROST(secp256k1-tr, SHA-256)` — BIP-340 / Nostr compatible.

use std::collections::BTreeMap;
use frost_secp256k1_tr::{
    self as frost,
    keys::{IdentifierList, KeyPackage, PublicKeyPackage},
    round1::{SigningCommitments, SigningNonces},
    round2::SignatureShare,
    Signature, SigningPackage,
};
use rand::thread_rng;
use serde::{Deserialize, Serialize};

use crate::{CryptoError, ThresholdScheme}; 

/// The Marker Struct
pub struct Secp256k1;

// Re-export the frost crate so downstream crates (signer-node) can use its types.
pub use frost_secp256k1_tr;

// -----------------------------------------------------------------------------
// CLI Helpers (Moved here from lib.rs)
// -----------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct KeyPackageWrapper {
    pub identifier: String, // CHANGED from u16 to String to safely hold full Identifier
    pub secret_share: String, 
    pub public_key: String,   
}

impl KeyPackageWrapper {
    /// Deserialize the hex-encoded `secret_share` back into a live `KeyPackage`.
    pub fn to_key_package(&self) -> Result<KeyPackage, CryptoError> {
        let bytes = hex::decode(&self.secret_share)
            .map_err(|_| CryptoError::InvalidEncoding)?;
        KeyPackage::deserialize(&bytes)
            .map_err(|e| CryptoError::Frost(format!("failed to deserialize KeyPackage: {e}")))
    }

    /// Extract the signer ID as a `u16` from the hex-encoded `identifier`.
    pub fn to_identifier_u16(&self) -> Result<u16, CryptoError> {
        let bytes = hex::decode(&self.identifier)
            .map_err(|_| CryptoError::InvalidEncoding)?;
        // Validate that the bytes are a valid Identifier.
        let _id = frost::Identifier::deserialize(&bytes)
            .map_err(|e| CryptoError::Frost(format!("failed to deserialize Identifier: {e}")))?;
        // Identifiers created by IdentifierList::Default are 1..=n.
        // The serialized form is a 32-byte big-endian Scalar; extract the low u16.
        if bytes.len() >= 2 {
            let lo = u16::from_be_bytes([bytes[bytes.len() - 2], bytes[bytes.len() - 1]]);
            if lo > 0 {
                return Ok(lo);
            }
        }
        Err(CryptoError::Frost("cannot extract u16 signer id from identifier".into()))
    }
}

pub fn generate_with_dealer(n: u16, k: u16) -> (String, String, Vec<KeyPackageWrapper>) {
    let mut rng = thread_rng();
    let (shares, pubkey_package) = frost::keys::generate_with_dealer(
        n,
        k,
        IdentifierList::Default,
        &mut rng,
    ).expect("Keygen failed");

    let group_pubkey_bytes = pubkey_package.verifying_key().serialize().expect("Pubkey serialization failed");
    let group_pubkey = hex::encode(group_pubkey_bytes);

    let pubkey_package_bytes = pubkey_package.serialize().expect("PublicKeyPackage serialization failed");
    let pubkey_package_hex = hex::encode(pubkey_package_bytes);

    let packages = shares.into_iter().map(|(id, secret)| {
        let key_package = frost::keys::KeyPackage::try_from(secret).unwrap();
        let bytes = key_package.serialize().expect("Failed to serialize share");

        KeyPackageWrapper {
            identifier: hex::encode(id.serialize()),
            secret_share: hex::encode(bytes),
            public_key: group_pubkey.clone(),
        }
    }).collect();

    (group_pubkey, pubkey_package_hex, packages)
}

// -----------------------------------------------------------------------------
// Protocol Structs
// -----------------------------------------------------------------------------

pub struct Nonce {
    pub identifier: frost::Identifier,
    pub(crate) nonces: SigningNonces,
    pub(crate) commitments: SigningCommitments,
}

impl Nonce {
    /// Public accessor for the signing nonces (needed by `frost::round2::sign`).
    pub fn signing_nonces(&self) -> &SigningNonces {
        &self.nonces
    }

    /// Public accessor for the signing commitments.
    pub fn signing_commitments(&self) -> &SigningCommitments {
        &self.commitments
    }
}

#[derive(Clone)]
pub struct Commitment {
    pub identifier: frost::Identifier,
    pub(crate) inner: SigningCommitments,
}

impl Commitment {
    /// Serialize the inner `SigningCommitments` to a `serde_json::Value`.
    pub fn to_json(&self) -> Result<serde_json::Value, CryptoError> {
        serde_json::to_value(&self.inner)
            .map_err(|e| CryptoError::Frost(format!("failed to serialize commitment: {e}")))
    }
}

pub struct PartialSig {
    pub identifier: frost::Identifier,
    pub(crate) inner: SignatureShare,
}

impl PartialSig {
    /// Serialize the inner `SignatureShare` to a `serde_json::Value`.
    pub fn to_json(&self) -> Result<serde_json::Value, CryptoError> {
        serde_json::to_value(&self.inner)
            .map_err(|e| CryptoError::Frost(format!("failed to serialize signature share: {e}")))
    }
}

pub struct FrostSignature(pub Signature);

#[derive(Clone)]
pub struct GroupKey(pub PublicKeyPackage);

// -----------------------------------------------------------------------------
// Trait Implementation
// -----------------------------------------------------------------------------

impl ThresholdScheme for Secp256k1 {
    type PublicKey        = GroupKey;
    type SecretShare      = KeyPackage;
    type Nonce            = Nonce;
    type NonceCommitment  = Commitment;
    type PartialSignature = PartialSig;
    type Signature        = FrostSignature;

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

        let key_packages = shares
            .into_values()
            .map(|s| KeyPackage::try_from(s).map_err(|e| CryptoError::Frost(e.to_string())))
            .collect::<Result<Vec<_>, _>>()?;

        Ok((key_packages, GroupKey(pubkey_package)))
    }

    fn generate_nonce(share: &KeyPackage) -> Nonce {
        let (nonces, commitments) = frost::round1::commit(share.signing_share(), &mut thread_rng());
        Nonce { identifier: *share.identifier(), nonces, commitments }
    }

    fn nonce_commitment(nonce: &Nonce) -> Commitment {
        Commitment { identifier: nonce.identifier, inner: nonce.commitments.clone() }
    }

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

    fn verify(pubkey: &GroupKey, msg: &[u8; 32], sig: &FrostSignature) -> bool {
        pubkey.0.verifying_key().verify(msg, &sig.0).is_ok()
    }
}
