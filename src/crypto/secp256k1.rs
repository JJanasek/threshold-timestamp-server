//! ThresholdScheme implementation for secp256k1.

use k256::{ProjectivePoint, Scalar};
use k256::elliptic_curve::ff::Field;

use crate::crypto::{CryptoError, Secp256k1, ThresholdScheme};


#[derive(Clone)]
pub struct Secp256k1PublicKey {
    pub bytes: [u8; 32],
    pub(crate) point: ProjectivePoint,
}

impl AsRef<[u8]> for Secp256k1PublicKey {
    fn as_ref(&self) -> &[u8] { &self.bytes }
}

pub struct Secp256k1Share {
    pub index: u32,
    pub(crate) secret: Scalar,
}

pub struct Secp256k1Nonce {
    pub index: u32,
    pub(crate) secret: Scalar,
    pub commitment: ProjectivePoint,
}

#[derive(Clone)]
pub struct Secp256k1NonceCommitment {
    pub index: u32,
    pub point: [u8; 33], // compressed SEC1
}

impl AsRef<[u8]> for Secp256k1NonceCommitment {
    fn as_ref(&self) -> &[u8] { &self.point }
}

pub struct Secp256k1PartialSig {
    pub index: u32,
    pub(crate) s: Scalar,
}

pub struct Secp256k1Signature(pub [u8; 64]);

impl AsRef<[u8]> for Secp256k1Signature {
    fn as_ref(&self) -> &[u8] { &self.0 }
}


impl ThresholdScheme for Secp256k1 {
    type PublicKey        = Secp256k1PublicKey;
    type SecretShare      = Secp256k1Share;
    type Nonce            = Secp256k1Nonce;
    type NonceCommitment  = Secp256k1NonceCommitment;
    type PartialSignature = Secp256k1PartialSig;
    type Signature        = Secp256k1Signature;

    fn generate_shares(n: u32, k: u32) -> Result<(Vec<Self::SecretShare>, Self::PublicKey), CryptoError> {
        todo!()
    }

    fn generate_nonce(index: u32) -> Self::Nonce {
        let secret = Scalar::random(&mut rand::thread_rng());
        let commitment = ProjectivePoint::GENERATOR * secret;
        Secp256k1Nonce { index, secret, commitment }
    }

    fn nonce_commitment(nonce: &Self::Nonce) -> Self::NonceCommitment {
        todo!()
    }

    fn partial_sign(
        share: &Self::SecretShare,
        nonce: &Self::Nonce,
        pubkey: &Self::PublicKey,
        commitments: &[Self::NonceCommitment],
        msg: &[u8; 32],
    ) -> Result<Self::PartialSignature, CryptoError> {
        todo!()
    }

    fn aggregate(
        partial_sigs: &[Self::PartialSignature],
        commitments: &[Self::NonceCommitment],
    ) -> Result<Self::Signature, CryptoError> {
        todo!()
    }

    fn verify(pubkey: &Self::PublicKey, msg: &[u8; 32], sig: &Self::Signature) -> bool {
        todo!()
    }
}
