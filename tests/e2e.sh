#!/usr/bin/env bash
# =============================================================================
# E2E Test Suite for Threshold Timestamp Server
#
# Runs against a Docker Compose stack. Expects services to be running and
# healthy before invocation, OR pass --up to start them automatically.
#
# Usage:
#   ./tests/e2e.sh              # services already running
#   ./tests/e2e.sh --up         # start services, run tests, tear down
# =============================================================================
set -euo pipefail

COORD="${COORDINATOR_URL:-http://localhost:8000}"
COLLECTOR="${COLLECTOR_URL:-http://localhost:9000}"
PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
COMPOSE="docker compose -f $PROJECT_DIR/docker-compose.yml"
MANAGE_STACK=false

PASSED=0
FAILED=0
TOTAL=0

# -- Helpers ------------------------------------------------------------------

pass() {
  PASSED=$((PASSED + 1))
  TOTAL=$((TOTAL + 1))
  echo "  [PASS] $1"
}

fail() {
  FAILED=$((FAILED + 1))
  TOTAL=$((TOTAL + 1))
  echo "  [FAIL] $1"
}

wait_for_url() {
  local url="$1"
  local label="$2"
  local max_attempts="${3:-30}"
  local attempt=0
  echo "  Waiting for $label ($url)..."
  while ! curl -sf "$url" > /dev/null 2>&1; do
    attempt=$((attempt + 1))
    if [[ $attempt -ge $max_attempts ]]; then
      echo "  ERROR: $label did not become ready after $max_attempts attempts"
      return 1
    fi
    sleep 2
  done
  echo "  $label is ready."
}

# -- Argument parsing ---------------------------------------------------------

for arg in "$@"; do
  case "$arg" in
    --up) MANAGE_STACK=true ;;
  esac
done

# -- Stack lifecycle ----------------------------------------------------------

cleanup() {
  if $MANAGE_STACK; then
    echo ""
    echo "Tearing down Docker Compose stack..."
    $COMPOSE down -v --remove-orphans 2>/dev/null || true
  fi
}
trap cleanup EXIT

if $MANAGE_STACK; then
  echo "=== Starting Docker Compose stack ==="

  # Generate keys if configs don't exist
  if [[ ! -f "$PROJECT_DIR/configs/coordinator.toml" ]]; then
    echo "  Generating keys (keygen profile)..."
    mkdir -p "$PROJECT_DIR/configs"
    $COMPOSE --profile keygen run --rm keygen
  fi

  # Generate collector config if missing
  if [[ ! -f "$PROJECT_DIR/configs/collector.toml" ]]; then
    printf 'host = "0.0.0.0"\nport = 9000\nmax_events = 10000\n' > "$PROJECT_DIR/configs/collector.toml"
  fi

  $COMPOSE up -d --build relay collector coordinator signer-1 signer-2 signer-3
  wait_for_url "$COORD/health" "coordinator"
fi

# =============================================================================
# GROUP 1: Setup & Basics (Acceptance Criteria #1)
# =============================================================================
echo ""
echo "=== GROUP 1: Setup & Basics ==="

# Test 1: Config files exist
if [[ -f "$PROJECT_DIR/configs/coordinator.toml" && \
      -f "$PROJECT_DIR/configs/signer_1.toml" && \
      -f "$PROJECT_DIR/configs/signer_2.toml" && \
      -f "$PROJECT_DIR/configs/signer_3.toml" ]]; then
  pass "T01: keygen produced coordinator.toml + 3 signer configs"
else
  fail "T01: config files missing"
fi

# Test 2: Health endpoint
HEALTH=$(curl -sf "$COORD/health" 2>&1 || echo "")
if echo "$HEALTH" | jq -e '.status == "ok"' > /dev/null 2>&1; then
  pass "T02: GET /health returns {status: ok}"
else
  fail "T02: GET /health unexpected: $HEALTH"
fi

# Test 3: Status endpoint
STATUS=$(curl -sf "$COORD/api/v1/status" 2>&1 || echo "")
S_K=$(echo "$STATUS" | jq -r '.k' 2>/dev/null || echo "")
S_N=$(echo "$STATUS" | jq -r '.n' 2>/dev/null || echo "")
S_SIGNERS=$(echo "$STATUS" | jq '.signers | length' 2>/dev/null || echo "")
if [[ "$S_K" == "2" && "$S_N" == "3" && "$S_SIGNERS" == "3" ]]; then
  pass "T03: GET /api/v1/status returns k=2, n=3, 3 signers"
else
  fail "T03: status unexpected: k=$S_K n=$S_N signers=$S_SIGNERS"
fi

# =============================================================================
# GROUP 2: Pubkey
# =============================================================================
echo ""
echo "=== GROUP 2: Pubkey ==="

PUBKEY_RESP=$(curl -sf "$COORD/api/v1/pubkey" 2>&1 || echo "")
GROUP_KEY=$(echo "$PUBKEY_RESP" | jq -r '.group_public_key' 2>/dev/null || echo "")
PK_K=$(echo "$PUBKEY_RESP" | jq -r '.k' 2>/dev/null || echo "")
PK_N=$(echo "$PUBKEY_RESP" | jq -r '.n' 2>/dev/null || echo "")

# Test 4: Pubkey returns valid group key
if [[ ${#GROUP_KEY} -eq 64 && "$PK_K" == "2" && "$PK_N" == "3" ]]; then
  pass "T04: GET /api/v1/pubkey returns 64-char key, k=2, n=3"
else
  fail "T04: pubkey unexpected: key_len=${#GROUP_KEY} k=$PK_K n=$PK_N"
fi

# =============================================================================
# GROUP 3: Happy Path (Acceptance Criteria #2, #3)
# =============================================================================
echo ""
echo "=== GROUP 3: Happy Path ==="

# Test 5: Timestamp via API
HASH1=$(echo -n "e2e-test-document-1" | sha256sum | awk '{print $1}')
TOKEN1=$(curl -sf -X POST "$COORD/api/v1/timestamp" \
  -H "Content-Type: application/json" \
  -d "{\"hash\": \"$HASH1\"}" 2>&1 || echo "")
T1_SIG=$(echo "$TOKEN1" | jq -r '.signature' 2>/dev/null || echo "")
T1_SERIAL=$(echo "$TOKEN1" | jq -r '.serial_number' 2>/dev/null || echo "")

if [[ -n "$T1_SIG" && "$T1_SIG" != "null" && -n "$T1_SERIAL" && "$T1_SERIAL" != "null" ]]; then
  pass "T05: POST /api/v1/timestamp returns token (serial=$T1_SERIAL)"
else
  fail "T05: timestamp failed: $TOKEN1"
fi

# Test 6: Verify token via API
VERIFY1=$(curl -sf -X POST "$COORD/api/v1/verify" \
  -H "Content-Type: application/json" \
  -d "{\"token\": $TOKEN1}" 2>&1 || echo "")
V1_VALID=$(echo "$VERIFY1" | jq -r '.valid' 2>/dev/null || echo "")

if [[ "$V1_VALID" == "true" ]]; then
  pass "T06: POST /api/v1/verify returns valid=true"
else
  fail "T06: verify returned: $VERIFY1"
fi

# Test 7: Second timestamp has higher serial number
HASH2=$(echo -n "e2e-test-document-2" | sha256sum | awk '{print $1}')
TOKEN2=$(curl -sf -X POST "$COORD/api/v1/timestamp" \
  -H "Content-Type: application/json" \
  -d "{\"hash\": \"$HASH2\"}" 2>&1 || echo "")
T2_SERIAL=$(echo "$TOKEN2" | jq -r '.serial_number' 2>/dev/null || echo "")

if [[ -n "$T2_SERIAL" && "$T2_SERIAL" != "null" && "$T2_SERIAL" -gt "$T1_SERIAL" ]]; then
  pass "T07: Serial numbers increase monotonically ($T1_SERIAL -> $T2_SERIAL)"
else
  fail "T07: serial not increasing: $T1_SERIAL -> $T2_SERIAL"
fi

# =============================================================================
# GROUP 4: Tamper Detection (Acceptance Criteria #5)
# =============================================================================
echo ""
echo "=== GROUP 4: Tamper Detection ==="

# Test 8: Tampered signature is rejected
BAD_SIG="0000000000000000000000000000000000000000000000000000000000000000"
BAD_TOKEN=$(echo "$TOKEN1" | jq --arg s "$BAD_SIG" '.signature = $s')
BAD_HTTP=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$COORD/api/v1/verify" \
  -H "Content-Type: application/json" \
  -d "{\"token\": $BAD_TOKEN}")

if [[ "$BAD_HTTP" == "400" ]]; then
  pass "T08: Tampered signature rejected with HTTP 400"
else
  fail "T08: tampered signature returned HTTP $BAD_HTTP (expected 400)"
fi

# Test 9: Tampered file_hash is rejected
WRONG_HASH="ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
BAD_TOKEN2=$(echo "$TOKEN1" | jq --arg h "$WRONG_HASH" '.file_hash = $h')
BAD_HTTP2=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$COORD/api/v1/verify" \
  -H "Content-Type: application/json" \
  -d "{\"token\": $BAD_TOKEN2}")

if [[ "$BAD_HTTP2" == "400" ]]; then
  pass "T09: Tampered file_hash rejected with HTTP 400"
else
  fail "T09: tampered file_hash returned HTTP $BAD_HTTP2 (expected 400)"
fi

# =============================================================================
# GROUP 5: Input Validation
# =============================================================================
echo ""
echo "=== GROUP 5: Input Validation ==="

# Test 10: Invalid hash (too short) returns 400
SHORT_HTTP=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$COORD/api/v1/timestamp" \
  -H "Content-Type: application/json" \
  -d '{"hash": "abc123"}')

if [[ "$SHORT_HTTP" == "400" ]]; then
  pass "T10: Hash too short returns HTTP 400"
else
  fail "T10: short hash returned HTTP $SHORT_HTTP (expected 400)"
fi

# Test 11: Invalid hash (non-hex) returns 400
NONHEX_HTTP=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$COORD/api/v1/timestamp" \
  -H "Content-Type: application/json" \
  -d '{"hash": "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"}')

if [[ "$NONHEX_HTTP" == "400" ]]; then
  pass "T11: Non-hex hash returns HTTP 400"
else
  fail "T11: non-hex hash returned HTTP $NONHEX_HTTP (expected 400)"
fi

# =============================================================================
# GROUP 6: Fault Tolerance (Acceptance Criteria #4, #7)
# =============================================================================
echo ""
echo "=== GROUP 6: Fault Tolerance ==="

# Test 12: Kill 1 signer (2-of-3 alive), signing still works.


echo "  Stopping signer-3..."
$COMPOSE stop signer-3
sleep 3

FT_SIG=""
for attempt in 1 2 3 4 5 6 7 8 9 10; do
  HASH3=$(echo -n "fault-tolerance-2of3-$attempt" | sha256sum | awk '{print $1}')
  FT_RESP=$(curl -s --max-time 35 -X POST "$COORD/api/v1/timestamp" \
    -H "Content-Type: application/json" \
    -d "{\"hash\": \"$HASH3\"}" 2>&1 || echo "")
  FT_SIG=$(echo "$FT_RESP" | jq -r '.signature' 2>/dev/null || echo "")
  if [[ -n "$FT_SIG" && "$FT_SIG" != "null" ]]; then
    break
  fi
  echo "    attempt $attempt picked stopped signer (coordinator timed out), retrying..."
done

if [[ -n "$FT_SIG" && "$FT_SIG" != "null" ]]; then
  pass "T12: Signing succeeds with 2-of-3 signers (signer-3 stopped)"
else
  fail "T12: signing with 2-of-3 failed after 10 attempts"
fi

# Test 13: Kill 2nd signer (1-of-3 alive), signing fails 503 within 35s
echo "  Stopping signer-2..."
$COMPOSE stop signer-2
sleep 3

HASH4=$(echo -n "fault-tolerance-1of3" | sha256sum | awk '{print $1}')
START_TS=$(date +%s)
TIMEOUT_HTTP=$(curl -s -o /dev/null -w "%{http_code}" --max-time 40 -X POST "$COORD/api/v1/timestamp" \
  -H "Content-Type: application/json" \
  -d "{\"hash\": \"$HASH4\"}" 2>&1 || echo "timeout")
END_TS=$(date +%s)
ELAPSED=$((END_TS - START_TS))

if [[ "$TIMEOUT_HTTP" == "503" && "$ELAPSED" -le 35 ]]; then
  pass "T13: Below-threshold returns HTTP 503 in ${ELAPSED}s (<=35s)"
else
  fail "T13: below-threshold: HTTP=$TIMEOUT_HTTP elapsed=${ELAPSED}s (expected 503 <=35s)"
fi

# Test 14: Restart killed signers, signing recovers.
# With all 3 signers alive again, signing should succeed on the first try,
# but we still allow a small retry budget for the restart to settle.
echo "  Restarting signer-2 and signer-3..."
$COMPOSE start signer-2 signer-3
sleep 5

REC_SIG=""
for attempt in 1 2 3; do
  HASH5=$(echo -n "recovery-test-$attempt" | sha256sum | awk '{print $1}')
  REC_RESP=$(curl -s --max-time 35 -X POST "$COORD/api/v1/timestamp" \
    -H "Content-Type: application/json" \
    -d "{\"hash\": \"$HASH5\"}" 2>&1 || echo "")
  REC_SIG=$(echo "$REC_RESP" | jq -r '.signature' 2>/dev/null || echo "")
  if [[ -n "$REC_SIG" && "$REC_SIG" != "null" ]]; then
    break
  fi
  sleep 2
done

if [[ -n "$REC_SIG" && "$REC_SIG" != "null" ]]; then
  pass "T14: Signing recovers after restarting killed signers"
else
  fail "T14: recovery failed after 3 attempts"
fi

# =============================================================================
# GROUP 7: Collector / Events
# =============================================================================
echo ""
echo "=== GROUP 7: Collector ==="

# Test 15: Collector health
COLL_HEALTH=$(curl -sf "$COLLECTOR/health" 2>&1 || echo "")
if echo "$COLL_HEALTH" | jq -e '.status == "ok"' > /dev/null 2>&1; then
  pass "T15: Collector GET /health returns {status: ok}"
else
  fail "T15: collector health: $COLL_HEALTH"
fi

# Test 16: Collector received events from signing sessions
EVENTS=$(curl -sf "$COLLECTOR/api/v1/events" 2>&1 || echo "[]")
EVENT_COUNT=$(echo "$EVENTS" | jq 'length' 2>/dev/null || echo "0")

if [[ "$EVENT_COUNT" -gt 0 ]]; then
  pass "T16: Collector has $EVENT_COUNT events from signing sessions"
else
  fail "T16: collector has no events"
fi

# =============================================================================
# Summary
# =============================================================================
echo ""
echo "========================================"
echo "  E2E Results: $PASSED passed, $FAILED failed (of $TOTAL)"
echo "========================================"

if [[ "$FAILED" -gt 0 ]]; then
  exit 1
fi
