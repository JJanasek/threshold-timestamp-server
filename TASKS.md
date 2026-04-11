# TASKS.md

Parallel implementation streams for the FROST Threshold Timestamp Authority.

Streams 1, 2, 3, and 7 have zero cross-dependencies and can start immediately.
Streams 4, 5, 6 depend on type signatures from 1+2+3 but can be developed against mocks in parallel with each other.

---

## Stream 1: `common` crate

- [x] Create `crates/common/` with Cargo.toml
- [x] Define `TimestampToken` struct with serde Serialize/Deserialize
- [x] Implement signing message construction (`SHA-256("FROST-TIMESTAMP-V1\x00" || ...)`)
- [x] Implement `TimestampToken::verify()` (recompute hash, reconstruct message, verify signature)
- [x] Define Nostr event kind constants (20001–20004, 1)
- [x] Define shared config structs (signer config, coordinator config) with TOML parsing
- [x] Define shared error types with `thiserror`

## Stream 2: `frost-core` crate (thin FROST wrapper)

- [x] Create `crates/frost-core/` with Cargo.toml (depends on `frost-secp256k1`)
- [x] Wrap `frost_secp256k1::keys::generate_with_dealer` for keygen
- [x] Wrap `round1::commit` → `(SigningNonces, SigningCommitments)`
- [x] Wrap `round2::sign` → `SignatureShare`
- [x] Wrap `aggregate` → `Signature`
- [x] Wrap `verify_group_signature`
- [x] JSON-serializable wrapper types for `SigningCommitments`, `SignatureShare`, `KeyPackage`, `PublicKeyPackage`
- [x] Unit tests: keygen → sign → aggregate → verify round-trip

## Stream 3: `nostr-transport` crate

- [x] Create `crates/nostr-transport/` with Cargo.toml (depends on `nostr-sdk`)
- [x] NIP-44 encrypt/decrypt helpers
- [x] Event builder functions for each kind (20001–20004, 1)
- [x] Event parser functions (decrypt + deserialize content)
- [x] Relay pool connection management
- [x] Subscription filter builders (coordinator filters, signer filters)

## Stream 4: `client-cli` crate

Depends on: Stream 1 (types), Stream 2 (keygen)

- [x] Create `crates/client-cli/` with Cargo.toml (depends on `common`, `frost-core`)
- [x] Clap CLI argument parsing with subcommands
- [x] `keygen --k <k> --n <n> --out <dir>` — generate key shares, write config files, print group pubkey
- [x] `timestamp <file> [--coordinator <url>]` — POST file, write `.tst` token file
- [x] `verify <file> <token.tst> [--coordinator <url>]` — verify via coordinator API
- [x] `inspect <token.tst>` — decode and verify token locally (no network)
- [x] `pubkey [--coordinator <url>]` — print group public key in hex and bech32

## Stream 5: `coordinator` binary

Depends on: Stream 1 (types), Stream 2 (FROST ops), Stream 3 (Nostr transport)

- [x] Create `crates/coordinator/` with Cargo.toml
- [x] Config loading from TOML file
- [x] Axum HTTP routes: `GET /health`
- [x] Axum HTTP routes: `GET /api/v1/pubkey`
- [x] Axum HTTP routes: `POST /api/v1/timestamp`
- [x] Axum HTTP routes: `POST /api/v1/verify`
- [x] Session state machine (`DashMap<session_id, SessionState>`)
- [x] Signing orchestration: session announce (kind 20001)
- [x] Signing orchestration: collect round1 commitments (kind 20002, 30s timeout)
- [x] Signing orchestration: send signing packages (kind 20003)
- [x] Signing orchestration: collect partial signatures (kind 20004, 30s timeout)
- [x] Signing orchestration: aggregate + verify + publish token (kind 1)
- [x] Error handling: 400 bad input, 503 timeout (report unresponsive signers), 500 internal
- [x] Serial number management (monotonic counter)
- [x] One-session-per-document-hash guard

## Stream 6: `signer-node` binary

Depends on: Stream 1 (types), Stream 2 (FROST ops), Stream 3 (Nostr transport)

- [x] Create `crates/signer-node/` with Cargo.toml
- [x] Config loading (KeyPackage, nsec, coordinator npub, relay URLs)
- [x] Relay pool connection + subscription to kind 20001 and 20003
- [x] Event loop: handle kind 20001 → decrypt, parse, call `round1::commit`, publish kind 20002
- [x] Event loop: handle kind 20003 → decrypt, lookup nonces, call `round2::sign`, publish kind 20004
- [x] Nonce map (`HashMap<session_id, SigningNonces>`) with creation timestamps
- [x] Background task: evict expired nonces (TTL 5min, sweep every 60s)
- [x] `--interactive` flag for manual approval prompt
- [x] Resilience: log and recover on bad events, never crash

## Stream 7: Infrastructure

- [x] Workspace `Cargo.toml` with all crate members
- [x] `docker-compose.yml`: coordinator, signer1, signer2, signer3 services
- [x] Docker compose: keygen profile service (run once, write configs, exit)
- [x] Coordinator exposes port 8000; signers have no exposed ports
- [x] All services mount `./configs` read-only
- [x] Dockerfiles for coordinator and signer-node binaries
- [x] `.gitignore` configs/ except examples
