use std::collections::BTreeMap;

use frost_secp256k1_tr::{
    Identifier,
    keys::dkg,
};
use nostr_sdk::PublicKey;
use uuid::Uuid;

/// Tracks ephemeral state for a single DKG session on this signer.
pub struct DkgState {
    pub session_id: Option<Uuid>,
    pub my_identifier: Option<Identifier>,
    pub my_signer_id: Option<u16>,
    /// Maps signer_id -> Nostr public key of each participant.
    pub participants: BTreeMap<u16, PublicKey>,
    pub round1_secret: Option<dkg::round1::SecretPackage>,
    /// All round 1 packages received (from broadcast), keyed by Identifier.
    pub round1_packages: Option<BTreeMap<Identifier, dkg::round1::Package>>,
    pub round2_secret: Option<dkg::round2::SecretPackage>,
    /// Round 2 packages received from peers, keyed by Identifier.
    pub round2_packages: BTreeMap<Identifier, dkg::round2::Package>,
    pub k: u16,
    pub n: u16,
}

impl DkgState {
    pub fn new() -> Self {
        Self {
            session_id: None,
            my_identifier: None,
            my_signer_id: None,
            participants: BTreeMap::new(),
            round1_secret: None,
            round1_packages: None,
            round2_secret: None,
            round2_packages: BTreeMap::new(),
            k: 0,
            n: 0,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Look up a participant's Nostr public key by signer_id.
    pub fn peer_pubkey(&self, signer_id: u16) -> Option<&PublicKey> {
        self.participants.get(&signer_id)
    }

    /// Check if we've collected all n-1 round 2 packages from peers.
    pub fn round2_complete(&self) -> bool {
        // We need packages from n-1 other participants
        let needed = if self.n > 1 { (self.n - 1) as usize } else { 0 };
        self.round2_packages.len() >= needed
    }
}
