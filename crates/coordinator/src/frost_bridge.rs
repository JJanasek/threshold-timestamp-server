use frost_secp256k1_tr::{
    keys::PublicKeyPackage, round1::SigningCommitments, round2::SignatureShare, Identifier,
    Signature, SigningPackage,
};
use serde_json::Value;

use crate::error::CoordinatorError;

pub fn commitments_from_json(value: Value) -> Result<SigningCommitments, CoordinatorError> {
    serde_json::from_value(value)
        .map_err(|e| CoordinatorError::FrostError(format!("failed to deserialize commitments: {e}")))
}

pub fn signature_share_from_json(value: Value) -> Result<SignatureShare, CoordinatorError> {
    serde_json::from_value(value)
        .map_err(|e| CoordinatorError::FrostError(format!("failed to deserialize signature share: {e}")))
}

pub fn signing_package_to_json(sp: &SigningPackage) -> Result<Value, CoordinatorError> {
    serde_json::to_value(sp)
        .map_err(|e| CoordinatorError::FrostError(format!("failed to serialize signing package: {e}")))
}

pub fn public_key_package_from_hex(hex_str: &str) -> Result<PublicKeyPackage, CoordinatorError> {
    let bytes = hex::decode(hex_str)
        .map_err(|e| CoordinatorError::FrostError(format!("invalid public_key_package hex: {e}")))?;
    PublicKeyPackage::deserialize(&bytes)
        .map_err(|e| CoordinatorError::FrostError(format!("failed to deserialize PublicKeyPackage: {e}")))
}

pub fn identifier_from_signer_id(id: u16) -> Result<Identifier, CoordinatorError> {
    Identifier::try_from(id)
        .map_err(|e| CoordinatorError::FrostError(format!("invalid signer id {id}: {e}")))
}

pub fn signature_to_hex(sig: &Signature) -> Result<String, CoordinatorError> {
    let bytes = sig
        .serialize()
        .map_err(|e| CoordinatorError::FrostError(format!("failed to serialize signature: {e}")))?;
    Ok(hex::encode(bytes))
}

/// Extract the x-only public key (32 bytes) from a PublicKeyPackage's verifying key.
pub fn verifying_key_to_x_only_hex(pkg: &PublicKeyPackage) -> Result<String, CoordinatorError> {
    let bytes = pkg
        .verifying_key()
        .serialize()
        .map_err(|e| CoordinatorError::FrostError(format!("failed to serialize verifying key: {e}")))?;
    // The serialized verifying key is a 33-byte compressed point (02/03 prefix + 32-byte x).
    // Strip the prefix to get the x-only key for BIP-340 compatibility.
    if bytes.len() == 33 {
        Ok(hex::encode(&bytes[1..]))
    } else {
        Ok(hex::encode(&bytes))
    }
}
