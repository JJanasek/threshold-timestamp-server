use nostr_sdk::prelude::*;
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EncryptError {
    #[error("serialization failed: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("encryption failed: {0}")]
    Encrypt(String),
    #[error("decryption failed: {0}")]
    Decrypt(String),
    #[error("no secret key available")]
    NoSecretKey,
}

/// Serialize `payload` as JSON, then NIP-44 encrypt it for `recipient`.
pub fn encrypt_payload<T: Serialize>(
    sender_keys: &Keys,
    recipient_pubkey: &PublicKey,
    payload: &T,
) -> Result<String, EncryptError> {
    let json = serde_json::to_string(payload)?;
    let secret_key = sender_keys.secret_key().map_err(|_| EncryptError::NoSecretKey)?;
    let encrypted = nip44::encrypt(secret_key, recipient_pubkey, json, nip44::Version::V2)
        .map_err(|e| EncryptError::Encrypt(e.to_string()))?;
    Ok(encrypted)
}

/// NIP-44 decrypt, then deserialize the JSON payload.
pub fn decrypt_payload<T: DeserializeOwned>(
    receiver_keys: &Keys,
    sender_pubkey: &PublicKey,
    encrypted: &str,
) -> Result<T, EncryptError> {
    let secret_key = receiver_keys.secret_key().map_err(|_| EncryptError::NoSecretKey)?;
    let json = nip44::decrypt(secret_key, sender_pubkey, encrypted)
        .map_err(|e| EncryptError::Decrypt(e.to_string()))?;
    let payload: T = serde_json::from_str(&json)?;
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SessionAnnounce;
    use uuid::Uuid;

    #[test]
    fn encrypt_decrypt_round_trip() {
        let sender = Keys::generate();
        let receiver = Keys::generate();

        let original = SessionAnnounce {
            session_id: Uuid::new_v4(),
            message: "hello FROST".into(),
            k: 2,
            n: 3,
        };

        let ciphertext =
            encrypt_payload(&sender, &receiver.public_key(), &original).unwrap();

        // Ciphertext should not contain the plaintext
        assert!(!ciphertext.contains("hello FROST"));

        let decrypted: SessionAnnounce =
            decrypt_payload(&receiver, &sender.public_key(), &ciphertext).unwrap();

        assert_eq!(original, decrypted);
    }

    #[test]
    fn decrypt_with_wrong_key_fails() {
        let sender = Keys::generate();
        let receiver = Keys::generate();
        let wrong_receiver = Keys::generate();

        let original = SessionAnnounce {
            session_id: Uuid::new_v4(),
            message: "secret".into(),
            k: 2,
            n: 3,
        };

        let ciphertext =
            encrypt_payload(&sender, &receiver.public_key(), &original).unwrap();

        let result: Result<SessionAnnounce, _> =
            decrypt_payload(&wrong_receiver, &sender.public_key(), &ciphertext);

        assert!(result.is_err());
    }

    #[test]
    fn pubkey_only_keys_cannot_encrypt() {
        let receiver = Keys::generate();
        let pubkey_only = Keys::from_public_key(receiver.public_key());

        let payload = SessionAnnounce {
            session_id: Uuid::new_v4(),
            message: "test".into(),
            k: 1,
            n: 1,
        };

        let result = encrypt_payload(&pubkey_only, &receiver.public_key(), &payload);
        assert!(matches!(result, Err(EncryptError::NoSecretKey)));
    }
}
