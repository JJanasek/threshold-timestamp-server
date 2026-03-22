//! Ciphersuite: `FROST(secp256k1-tr, SHA-256)` — BIP-340 / Nostr compatible.

use std::collections::BTreeMap;

use frost_secp256k1_tr::{
    self as frost,
    keys::{IdentifierList, KeyPackage, PublicKeyPackage, dkg},
    round1::{SigningCommitments, SigningNonces},
    round2::SignatureShare,
    Identifier, Signature, SigningPackage,
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

// -----------------------------------------------------------------------------
// DKG Wrappers
// -----------------------------------------------------------------------------

/// DKG Round 1: Generate the participant's secret package and public package.
pub fn dkg_part1(
    identifier: Identifier,
    max_signers: u16,
    min_signers: u16,
) -> Result<(dkg::round1::SecretPackage, dkg::round1::Package), CryptoError> {
    dkg::part1(identifier, max_signers, min_signers, thread_rng())
        .map_err(|e| CryptoError::Frost(format!("dkg part1 failed: {e}")))
}

/// DKG Round 2: Process received round 1 packages and produce round 2 packages.
pub fn dkg_part2(
    secret_package: dkg::round1::SecretPackage,
    round1_packages: &BTreeMap<Identifier, dkg::round1::Package>,
) -> Result<(dkg::round2::SecretPackage, BTreeMap<Identifier, dkg::round2::Package>), CryptoError> {
    dkg::part2(secret_package, round1_packages)
        .map_err(|e| CryptoError::Frost(format!("dkg part2 failed: {e}")))
}

/// DKG Round 3: Finalize the protocol and produce KeyPackage + PublicKeyPackage.
pub fn dkg_part3(
    round2_secret_package: &dkg::round2::SecretPackage,
    round1_packages: &BTreeMap<Identifier, dkg::round1::Package>,
    round2_packages: &BTreeMap<Identifier, dkg::round2::Package>,
) -> Result<(KeyPackage, PublicKeyPackage), CryptoError> {
    dkg::part3(round2_secret_package, round1_packages, round2_packages)
        .map_err(|e| CryptoError::Frost(format!("dkg part3 failed: {e}")))
}

/// Serialize a DKG round 1 package to JSON.
pub fn dkg_round1_package_to_json(pkg: &dkg::round1::Package) -> Result<serde_json::Value, CryptoError> {
    serde_json::to_value(pkg)
        .map_err(|e| CryptoError::Frost(format!("failed to serialize round1 package: {e}")))
}

/// Deserialize a DKG round 1 package from JSON.
pub fn dkg_round1_package_from_json(value: serde_json::Value) -> Result<dkg::round1::Package, CryptoError> {
    serde_json::from_value(value)
        .map_err(|e| CryptoError::Frost(format!("failed to deserialize round1 package: {e}")))
}

/// Serialize a DKG round 2 package to JSON.
pub fn dkg_round2_package_to_json(pkg: &dkg::round2::Package) -> Result<serde_json::Value, CryptoError> {
    serde_json::to_value(pkg)
        .map_err(|e| CryptoError::Frost(format!("failed to serialize round2 package: {e}")))
}

/// Deserialize a DKG round 2 package from JSON.
pub fn dkg_round2_package_from_json(value: serde_json::Value) -> Result<dkg::round2::Package, CryptoError> {
    serde_json::from_value(value)
        .map_err(|e| CryptoError::Frost(format!("failed to deserialize round2 package: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dkg_3_participants_then_sign_verify() {
        let n = 3u16;
        let k = 2u16;

        // Create identifiers for 3 participants
        let id1 = Identifier::try_from(1u16).unwrap();
        let id2 = Identifier::try_from(2u16).unwrap();
        let id3 = Identifier::try_from(3u16).unwrap();
        let ids = [id1, id2, id3];

        // === Round 1 ===
        let (secret1, pkg1) = dkg_part1(id1, n, k).unwrap();
        let (secret2, pkg2) = dkg_part1(id2, n, k).unwrap();
        let (secret3, pkg3) = dkg_part1(id3, n, k).unwrap();

        // Verify round 1 serde round-trip
        let pkg1_json = dkg_round1_package_to_json(&pkg1).unwrap();
        let pkg1_back = dkg_round1_package_from_json(pkg1_json).unwrap();
        assert_eq!(
            serde_json::to_value(&pkg1).unwrap(),
            serde_json::to_value(&pkg1_back).unwrap()
        );

        // Each participant receives packages from the others
        let r1_for_1: BTreeMap<_, _> = [(id2, pkg2.clone()), (id3, pkg3.clone())].into();
        let r1_for_2: BTreeMap<_, _> = [(id1, pkg1.clone()), (id3, pkg3.clone())].into();
        let r1_for_3: BTreeMap<_, _> = [(id1, pkg1.clone()), (id2, pkg2.clone())].into();

        // === Round 2 ===
        let (r2_secret1, r2_packages1) = dkg_part2(secret1, &r1_for_1).unwrap();
        let (r2_secret2, r2_packages2) = dkg_part2(secret2, &r1_for_2).unwrap();
        let (r2_secret3, r2_packages3) = dkg_part2(secret3, &r1_for_3).unwrap();

        // Verify round 2 serde round-trip
        let r2_pkg = r2_packages1.get(&id2).unwrap();
        let r2_json = dkg_round2_package_to_json(r2_pkg).unwrap();
        let r2_back = dkg_round2_package_from_json(r2_json).unwrap();
        assert_eq!(
            serde_json::to_value(r2_pkg).unwrap(),
            serde_json::to_value(&r2_back).unwrap()
        );

        // Each participant receives round 2 packages from others
        let r2_for_1: BTreeMap<_, _> = [
            (id2, r2_packages2.get(&id1).unwrap().clone()),
            (id3, r2_packages3.get(&id1).unwrap().clone()),
        ].into();
        let r2_for_2: BTreeMap<_, _> = [
            (id1, r2_packages1.get(&id2).unwrap().clone()),
            (id3, r2_packages3.get(&id2).unwrap().clone()),
        ].into();
        let r2_for_3: BTreeMap<_, _> = [
            (id1, r2_packages1.get(&id3).unwrap().clone()),
            (id2, r2_packages2.get(&id3).unwrap().clone()),
        ].into();

        // === Round 3 ===
        let (key_pkg1, pub_pkg1) = dkg_part3(&r2_secret1, &r1_for_1, &r2_for_1).unwrap();
        let (key_pkg2, pub_pkg2) = dkg_part3(&r2_secret2, &r1_for_2, &r2_for_2).unwrap();
        let (_key_pkg3, pub_pkg3) = dkg_part3(&r2_secret3, &r1_for_3, &r2_for_3).unwrap();

        // All participants should agree on the group public key
        assert_eq!(
            pub_pkg1.verifying_key().serialize().unwrap(),
            pub_pkg2.verifying_key().serialize().unwrap()
        );
        assert_eq!(
            pub_pkg2.verifying_key().serialize().unwrap(),
            pub_pkg3.verifying_key().serialize().unwrap()
        );

        // === Sign with participants 1 & 2 (threshold k=2) ===
        let msg = crate::sha256(b"test message for DKG signing");

        // Generate nonces
        let nonce1 = Secp256k1::generate_nonce(&key_pkg1);
        let nonce2 = Secp256k1::generate_nonce(&key_pkg2);

        let comm1 = Secp256k1::nonce_commitment(&nonce1);
        let comm2 = Secp256k1::nonce_commitment(&nonce2);

        let commitments = [comm1.clone(), comm2.clone()];

        // Partial sign
        let psig1 = Secp256k1::partial_sign(&key_pkg1, &nonce1, &GroupKey(pub_pkg1.clone()), &commitments, &msg).unwrap();
        let psig2 = Secp256k1::partial_sign(&key_pkg2, &nonce2, &GroupKey(pub_pkg2.clone()), &commitments, &msg).unwrap();

        // Aggregate
        let sig = Secp256k1::aggregate(&[psig1, psig2], &commitments, &GroupKey(pub_pkg1.clone()), &msg).unwrap();

        // Verify
        assert!(Secp256k1::verify(&GroupKey(pub_pkg1.clone()), &msg, &sig));

        // Verify KeyPackageWrapper round-trip for DKG-generated keys
        let kp_bytes = key_pkg1.serialize().unwrap();
        let id_bytes = ids[0].serialize();
        let vk_bytes = pub_pkg1.verifying_key().serialize().unwrap();
        let wrapper = KeyPackageWrapper {
            identifier: hex::encode(id_bytes),
            secret_share: hex::encode(&kp_bytes),
            public_key: hex::encode(&vk_bytes),
        };
        let restored = wrapper.to_key_package().unwrap();
        assert_eq!(restored.serialize().unwrap(), kp_bytes);
    }
}
