# Nostr MPC Timestamping Server

A decentralized, trusted digital timestamping service built in Rust. The signing key is never held by any single party — it is split into *n* Shamir shares using the FROST threshold Schnorr signature scheme. Signing requires *k*-of-*n* nodes to cooperate over the Nostr protocol.

## Architecture

```
┌────────────┐    HTTP     ┌──────────────┐   Nostr (NIP-44)   ┌──────────────┐
│ Client CLI │ ──────────► │ Coordinator  │ ◄────────────────►  │ Signer Nodes │
│  (mpc-cli) │             │  (HTTP API)  │                    │  (k-of-n)    │
└────────────┘             └──────────────┘                    └──────────────┘
```

- **Coordinator** — orchestrates FROST signing sessions and exposes an HTTP API
- **Signer Nodes** — each holds one key share, participates in signing via Nostr
- **Client CLI** — submits documents for timestamping and verifies tokens

## Repository Structure

```
threshold-timestamp-server/
├── Cargo.toml                  # Workspace manifest
├── SPEC.md                     # Detailed technical specification
├── TASKS.md                    # Implementation task breakdown
├── crates/
│   ├── common/                 # Shared types: TimestampToken, event kinds, configs
│   ├── frost-core/             # FROST crypto wrapper (keygen, signing, verification)
│   ├── nostr-transport/        # Nostr event builders, NIP-44 encryption, relay pool
│   └── client-cli/             # CLI binary: keygen, timestamp, verify, inspect
└── configs/                    # Generated key configs (gitignored)
```

## Prerequisites

- **Rust** 1.70+ (install via [rustup](https://rustup.rs/))
- A running **Nostr relay** (e.g. local relay on `ws://localhost:8080` or a public one like `wss://relay.damus.io`)

## Getting Started

### 1. Clone and Build

```bash
git clone <repo-url>
cd threshold-timestamp-server
cargo build --workspace
```

### 2. Generate Key Shares

Generate a 2-of-3 threshold key split using the trusted dealer:

```bash
cargo run -p mpc-cli -- keygen --k 2 --n 3 --out ./configs
```

This creates:
- `configs/coordinator.toml` — coordinator configuration with the group public key
- `configs/signer_1.toml` through `configs/signer_3.toml` — each signer's key share and config

> **Warning:** This uses trusted-dealer key generation. Replace with DKG for production deployments.

After generation, update each config file with the correct Nostr identities (`npub`/`nsec`) and relay URLs.

### 3. Start Signer Nodes

Each signer node loads its config and listens for signing requests over Nostr. Start each one in a separate terminal (or on separate machines):

```bash
cargo run -p signer-node -- --config ./configs/signer_1.toml
cargo run -p signer-node -- --config ./configs/signer_2.toml
cargo run -p signer-node -- --config ./configs/signer_3.toml
```

### 4. Start the Coordinator

The coordinator exposes an HTTP API (default port 8000) and communicates with signers via Nostr:

```bash
cargo run -p coordinator -- --config ./configs/coordinator.toml
```

### 5. Timestamp a Document

```bash
cargo run -p mpc-cli -- timestamp ./document.pdf --coordinator http://localhost:8000
```

This sends the file hash to the coordinator, which orchestrates a FROST signing session. The resulting timestamp token is saved as `document.tst`.

### 6. Verify a Token

Verify a timestamp token against its original file:

```bash
# Online verification (checks against coordinator)
cargo run -p mpc-cli -- verify ./document.pdf ./document.tst --coordinator http://localhost:8000

# Offline inspection (verifies signature locally, no network)
cargo run -p mpc-cli -- inspect ./document.tst
```

## CLI Reference

```
mpc-cli <COMMAND>

Commands:
  keygen     Generate key shares for the servers (offline)
  timestamp  Request a timestamp for a file
  verify     Verify a timestamp token against a file
  inspect    Inspect a token and verify its signature locally (offline)
```

| Command | Key Options | Description |
|---------|-------------|-------------|
| `keygen` | `--k <threshold>` `--n <total>` `--out <dir>` | Generate FROST key shares (default: 2-of-3) |
| `timestamp` | `<file>` `--coordinator <url>` | Request a timestamp from the coordinator |
| `verify` | `<file>` `<token>` `--coordinator <url>` | Verify token matches a file |
| `inspect` | `<token>` | Decode and verify a token offline |

## Configuration

### Signer Config (`signer_X.toml`)

```toml
key_package = "<JSON-serialized FROST KeyPackage>"
coordinator_npub = "npub1..."
relay_urls = ["wss://relay.damus.io", "wss://nos.lol"]
```

Keep signer configs secret (file mode `0600`) — they contain the signing key share.

### Coordinator Config (`coordinator.toml`)

```toml
group_public_key = "<hex-encoded group public key>"
port = 8000
relay_urls = ["wss://relay.damus.io", "wss://nos.lol"]
signers_npubs = ["npub1alice...", "npub1bob...", "npub1carol..."]
```

## How It Works

1. **Client** sends a file hash to the coordinator via HTTP
2. **Coordinator** announces a signing session to *k* signers over Nostr (NIP-44 encrypted)
3. **Round 1** — each signer generates a nonce commitment and sends it back
4. **Round 2** — coordinator builds a `SigningPackage` and sends it to each signer; signers compute partial signatures
5. **Aggregate** — coordinator combines partial signatures into a single BIP-340 Schnorr signature
6. **Result** — the `TimestampToken` (hash, timestamp, signature, group public key) is returned to the client and published to Nostr relays

All inter-node communication uses ephemeral Nostr events (kinds 20001–20004) with NIP-44 encryption. The final token is published as a kind 1 event (plaintext).

## TimestampToken Format

```json
{
  "serial_number": 1,
  "timestamp": 1700000000,
  "file_hash": "sha256:abcdef...",
  "signature": "<64-byte hex Schnorr signature>",
  "group_public_key": "<32-byte hex X-only pubkey>"
}
```

## Tech Stack

| Component | Crate |
|-----------|-------|
| FROST threshold signatures | `frost-secp256k1-tr` |
| Nostr client & relay pool | `nostr-sdk` |
| HTTP API | `axum` |
| Async runtime | `tokio` |
| Hashing | `sha2` |
| CLI parsing | `clap` |
| Serialization | `serde`, `serde_json`, `toml` |

## Development

```bash
# Build everything
cargo build --workspace

# Build only the CLI
cargo build -p mpc-cli

# Run tests
cargo test --workspace

# Check without building
cargo check --workspace
```

## License

See [LICENSE](LICENSE) for details.
