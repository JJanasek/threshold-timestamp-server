# TASKS.md

Parallel implementation streams for the FROST Threshold Timestamp Authority.

Streams 1, 2, 3, and 7 have zero cross-dependencies and can start immediately.
Streams 4, 5, 6 depend on type signatures from 1+2+3 but can be developed against mocks in parallel with each other.

---

## Stream 1: `common` crate

- [ ] Create `crates/common/` with Cargo.toml
- [ ] Define `TimestampToken` struct with serde Serialize/Deserialize
- [ ] Implement signing message construction (`SHA-256("FROST-TIMESTAMP-V1\x00" || ...)`)
- [ ] Implement `TimestampToken::verify()` (recompute hash, reconstruct message, verify signature)
- [ ] Define Nostr event kind constants (20001–20004, 1)
- [ ] Define shared config structs (signer config, coordinator config) with TOML parsing
- [ ] Define shared error types with `thiserror`

## Stream 2: `frost-core` crate (thin FROST wrapper)

- [ ] Create `crates/frost-core/` with Cargo.toml (depends on `frost-secp256k1`)
- [ ] Wrap `frost_secp256k1::keys::generate_with_dealer` for keygen
- [ ] Wrap `round1::commit` → `(SigningNonces, SigningCommitments)`
- [ ] Wrap `round2::sign` → `SignatureShare`
- [ ] Wrap `aggregate` → `Signature`
- [ ] Wrap `verify_group_signature`
- [ ] JSON-serializable wrapper types for `SigningCommitments`, `SignatureShare`, `KeyPackage`, `PublicKeyPackage`
- [ ] Unit tests: keygen → sign → aggregate → verify round-trip

## Stream 3: `nostr-transport` crate

- [ ] Create `crates/nostr-transport/` with Cargo.toml (depends on `nostr-sdk`)
- [ ] NIP-44 encrypt/decrypt helpers
- [ ] Event builder functions for each kind (20001–20004, 1)
- [ ] Event parser functions (decrypt + deserialize content)
- [ ] Relay pool connection management
- [ ] Subscription filter builders (coordinator filters, signer filters)

## Stream 4: `client-cli` crate

Depends on: Stream 1 (types), Stream 2 (keygen)

- [ ] Create `crates/client-cli/` with Cargo.toml (depends on `common`, `frost-core`)
- [ ] Clap CLI argument parsing with subcommands
- [ ] `keygen --k <k> --n <n> --out <dir>` — generate key shares, write config files, print group pubkey
- [ ] `timestamp <file> [--coordinator <url>]` — POST file, write `.tst` token file
- [ ] `verify <file> <token.tst> [--coordinator <url>]` — verify via coordinator API
- [ ] `inspect <token.tst>` — decode and verify token locally (no network)
- [ ] `pubkey [--coordinator <url>]` — print group public key in hex and bech32

## Stream 5: `coordinator` binary

Depends on: Stream 1 (types), Stream 2 (FROST ops), Stream 3 (Nostr transport)

- [ ] Create `crates/coordinator/` with Cargo.toml
- [ ] Config loading from TOML file
- [ ] Axum HTTP routes: `GET /health`
- [ ] Axum HTTP routes: `GET /api/v1/pubkey`
- [ ] Axum HTTP routes: `POST /api/v1/timestamp`
- [ ] Axum HTTP routes: `POST /api/v1/verify`
- [ ] Session state machine (`DashMap<session_id, SessionState>`)
- [ ] Signing orchestration: session announce (kind 20001)
- [ ] Signing orchestration: collect round1 commitments (kind 20002, 30s timeout)
- [ ] Signing orchestration: send signing packages (kind 20003)
- [ ] Signing orchestration: collect partial signatures (kind 20004, 30s timeout)
- [ ] Signing orchestration: aggregate + verify + publish token (kind 1)
- [ ] Error handling: 400 bad input, 503 timeout (report unresponsive signers), 500 internal
- [ ] Serial number management (monotonic counter)
- [ ] One-session-per-document-hash guard

## Stream 6: `signer-node` binary

Depends on: Stream 1 (types), Stream 2 (FROST ops), Stream 3 (Nostr transport)

- [ ] Create `crates/signer-node/` with Cargo.toml
- [ ] Config loading (KeyPackage, nsec, coordinator npub, relay URLs)
- [ ] Relay pool connection + subscription to kind 20001 and 20003
- [ ] Event loop: handle kind 20001 → decrypt, parse, call `round1::commit`, publish kind 20002
- [ ] Event loop: handle kind 20003 → decrypt, lookup nonces, call `round2::sign`, publish kind 20004
- [ ] Nonce map (`HashMap<session_id, SigningNonces>`) with creation timestamps
- [ ] Background task: evict expired nonces (TTL 5min, sweep every 60s)
- [ ] `--interactive` flag for manual approval prompt
- [ ] Resilience: log and recover on bad events, never crash

## Stream 7: Infrastructure

- [ ] Workspace `Cargo.toml` with all crate members
- [ ] `docker-compose.yml`: coordinator, signer1, signer2, signer3 services
- [ ] Docker compose: keygen profile service (run once, write configs, exit)
- [ ] Coordinator exposes port 8000; signers have no exposed ports
- [ ] All services mount `./configs` read-only
- [ ] Dockerfiles for coordinator and signer-node binaries
- [ ] `.gitignore` configs/ except examples
