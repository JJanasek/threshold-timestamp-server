# Nostr MPC Timestamping Server

A decentralized, trusted digital timestamping server built in Rust. This project implements a $k$-of-$n$ Multi-Party Computation (MPC) architecture to issue BIP-340 Schnorr signatures over the Nostr network.

## 🛠️ Tech Stack

* **Language:** Rust
* **API & Runtime:** `axum` & `tokio` (for the Coordinator REST API and async execution)
* **Cryptography (From Scratch):** * `k256` (secp256k1 elliptic curve arithmetic for threshold math)
  * `sha2` (SHA-256 for document hashing and BIP-340 challenges)
  * `rand` (Cryptographically secure random number generation)
* **Networking / Message Bus:** `nostr-sdk` (for decentralized communication between nodes)
* **Serialization:** `serde` & `serde_json`

## 🚀 How to Start
 * clone this repo
 ```bash
    cd threshold-timestamp-server
    cargo run
 ```

