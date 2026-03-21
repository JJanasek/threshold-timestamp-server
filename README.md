# Threshold Timestamp Server

A decentralized digital timestamping service built in Rust. Uses **FROST threshold Schnorr signatures** (k-of-n) over the **Nostr** network to produce BIP-340-compatible timestamp tokens without any single party holding the signing key.

## Architecture

```
                          Nostr Relay (ws://relay:8080)
                         ┌──────────┐
                         │          │
              ┌──────────┤  Relay   ├──────────┐──────────┐
              │          │          │          │          │
              ▼          └──────────┘          ▼          ▼
       ┌─────────────┐               ┌──────────┐ ┌──────────┐
       │ Coordinator │               │ Signer 1 │ │ Signer N │
       │  (Axum API) │               │          │ │   ...    │
       │  :8000      │               └──────────┘ └──────────┘
       └──────┬──────┘
              │ HTTP
     ┌────────┼────────┐
     ▼        ▼        ▼
  Next.js   CLI     Any HTTP
  Web UI   Tool     Client
  :3000
```

**Coordinator** — Axum REST API that orchestrates FROST signing sessions. Receives timestamp requests, selects k random signers, runs the two-round FROST protocol over encrypted Nostr messages, aggregates partial signatures, and returns the final token.

**Signer Nodes** — Each holds one secret key share. Listen for signing requests from the coordinator via Nostr, generate nonces (Round 1), compute partial signatures (Round 2), and send them back. No HTTP server; purely Nostr-based.

**Nostr Relay** — Message bus between coordinator and signers. All protocol messages are NIP-04 encrypted and use ephemeral event kinds (20001-20004), so relays don't store them.

**Web UI** — Next.js app for timestamping, verification, and admin status. Talks to the coordinator's REST API.

**CLI** — Command-line tool for key generation, timestamping files, verifying tokens, and inspecting token contents.

## Signing Flow

1. Client sends a SHA-256 hash to `POST /api/v1/timestamp`
2. Coordinator creates a session, selects k-of-n signers at random
3. **Round 1**: Coordinator announces session to selected signers via Nostr. Each signer generates a nonce and returns a commitment (30s timeout)
4. **Round 2**: Coordinator builds a FROST SigningPackage and sends it to signers. Each computes a partial signature and returns it (30s timeout)
5. Coordinator aggregates partial signatures into a single Schnorr signature, verifies it, and returns the `TimestampToken`
6. Token is also published on Nostr as a kind-1 note

## Quick Start

### Prerequisites

- Rust 1.75+ with `cargo`
- Node.js 22+ with `npm` (for web UI)
- Docker & Docker Compose (for containerized deployment)

### 1. Generate Keys

```bash
cargo run -p mpc-cli -- keygen --k 2 --n 3 --out ./configs
```

This generates:
- `configs/coordinator.toml` — coordinator config with FROST public key package and signer list
- `configs/signer_1.toml`, `configs/signer_2.toml`, `configs/signer_3.toml` — each with a unique key share

### 2a. Run with Docker Compose (recommended)

```bash
docker compose up --build
```

This starts:
| Service      | Port  | Description           |
|-------------|-------|-----------------------|
| relay       | 8081  | Nostr relay           |
| coordinator | 8000  | REST API              |
| signer-1..3 | —     | Threshold signers     |
| web-ui      | 3000  | Next.js web interface |

Open http://localhost:3000 for the web UI, or use the API directly on http://localhost:8000.

### 2b. Run Locally (development)

Start the Nostr relay (or use an existing one):

```bash
docker run -p 8080:8080 scsibug/nostr-rs-relay
```

Start the coordinator and signers in separate terminals:

```bash
cargo run -p coordinator -- --config configs/coordinator.toml
cargo run -p signer-node -- --config configs/signer_1.toml
cargo run -p signer-node -- --config configs/signer_2.toml
cargo run -p signer-node -- --config configs/signer_3.toml
```

Start the web UI:

```bash
cd web-ui
npm install
npm run dev
```

The web UI runs on http://localhost:3000 and the coordinator API on http://localhost:8000.

## CLI Usage

Build the CLI:

```bash
cargo build -p mpc-cli
```

### Timestamp a file

```bash
cargo run -p mpc-cli -- timestamp document.pdf --coordinator http://localhost:8000
```

Computes the SHA-256 hash, requests a threshold signature, and saves the token to `document.pdf.tst`.

### Verify a token

```bash
cargo run -p mpc-cli -- verify document.pdf document.pdf.tst
```

Checks that the file hash matches the token and verifies the Schnorr signature.

### Inspect a token

```bash
cargo run -p mpc-cli -- inspect document.pdf.tst
```

Prints token fields (serial, timestamp, hash, signature, public key) and verifies the signature.

### Generate keys

```bash
cargo run -p mpc-cli -- keygen --k 2 --n 3 --out ./configs
```

| Flag    | Default    | Description                |
|---------|-----------|----------------------------|
| `--k`   | 2         | Signing threshold          |
| `--n`   | 3         | Total number of signers    |
| `--out` | `./configs` | Output directory for configs |

## REST API

Base URL: `http://localhost:8000`

| Method | Endpoint             | Description                      |
|--------|---------------------|----------------------------------|
| GET    | `/health`           | Health check                     |
| GET    | `/api/v1/status`    | Coordinator status, signer list  |
| GET    | `/api/v1/pubkey`    | Group public key and parameters  |
| POST   | `/api/v1/timestamp` | Request a timestamp signature    |
| POST   | `/api/v1/verify`    | Verify a timestamp token         |

### POST /api/v1/timestamp

```json
// Request
{ "hash": "a1b2c3...64-char-hex-sha256" }

// Response
{
  "serial_number": 1,
  "timestamp": 1711036800,
  "file_hash": "a1b2c3...",
  "signature": "...128-char-hex...",
  "group_public_key": "...64-char-hex..."
}
```

### POST /api/v1/verify

```json
// Request
{
  "token": {
    "serial_number": 1,
    "timestamp": 1711036800,
    "file_hash": "a1b2c3...",
    "signature": "...",
    "group_public_key": "..."
  }
}

// Response
{ "valid": true }
```

## Web UI

The web UI at http://localhost:3000 provides three pages:

- **Sign** (`/`) — Upload a file or paste a SHA-256 hash to request a threshold-signed timestamp token. Download the result as a `.tst` file.
- **Verify** (`/verify`) — Paste or upload a `.tst` token to verify its signature.
- **Admin** (`/admin`) — Live dashboard showing coordinator health, threshold parameters, signer nodes, and relay URLs. Auto-refreshes every 5 seconds.

## Project Structure

```
threshold-timestamp-server/
├── crates/
│   ├── client-cli/       # CLI tool (keygen, timestamp, verify, inspect)
│   ├── common/           # Shared types (TimestampToken, Nostr event kinds, configs)
│   ├── coordinator/      # Axum REST API + FROST session orchestration
│   ├── frost-core/       # FROST threshold crypto (keygen, sign, aggregate, verify)
│   ├── nostr-transport/  # Nostr message types, NIP-04 encryption, relay wrapper
│   └── signer-node/      # Threshold signer daemon (Nostr event loop)
├── web-ui/               # Next.js web interface
│   ├── app/              # Pages (signing, verify, admin)
│   ├── components/       # React components (hand-drawn design system)
│   └── lib/              # API client
├── configs/              # Generated TOML configs (coordinator + signers)
├── Dockerfile            # Multi-stage build for coordinator + signer-node
├── docker-compose.yml    # Full stack: relay, coordinator, 3 signers, web UI
└── Cargo.toml            # Workspace root
```

## Configuration

### Coordinator (`configs/coordinator.toml`)

```toml
[coordinator]
nsec = "nsec1..."           # Coordinator Nostr secret key
http_host = "0.0.0.0"
http_port = 8000

[frost]
k = 2                       # Signing threshold
n = 3                       # Total signers
public_key_package = "..."  # Hex-encoded FROST PublicKeyPackage

[[signers]]
npub = "npub1..."
signer_id = 1

[[signers]]
npub = "npub1..."
signer_id = 2

[[signers]]
npub = "npub1..."
signer_id = 3

[relays]
urls = ["ws://relay:8080"]
```

### Signer (`configs/signer_N.toml`)

```toml
key_package = '{"identifier":"...","secret_share":"...","public_key":"..."}'
coordinator_npub = "npub1..."
relay_urls = ["ws://relay:8080"]
nsec = "nsec1..."
```

All config files are generated by `mpc-cli keygen`. Relay URLs must match across coordinator and signers.

## Tech Stack

- **Rust** — coordinator, signers, CLI, and crypto core
- **FROST** (`frost-secp256k1-tr`) — threshold Schnorr signatures (BIP-340 compatible)
- **Nostr** (`nostr-sdk`) — decentralized encrypted messaging between nodes
- **Axum** + **Tokio** — async HTTP API
- **Next.js** + **Tailwind CSS** — web UI
- **Docker** — containerized deployment
