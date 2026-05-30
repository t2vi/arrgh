#!/usr/bin/env bash
set -euo pipefail

BASE=${BASE_URL:-http://localhost:8080}
RESULTS_DIR=${ALLURE_RESULTS_DIR:-./allure-results}
JUNIT_FILE=${JUNIT_FILE:-./junit.xml}
ADMIN_USER=${ADMIN_USER:-test-admin}
ADMIN_PASS=${ADMIN_PASS:-testpassword123}

mkdir -p "$RESULTS_DIR"

# ── Wait for server ───────────────────────────────────────────────────────────
echo "Waiting for server at $BASE..."
for i in $(seq 1 60); do
  if curl -sf "$BASE/api/auth/status" > /dev/null 2>&1; then
    echo "Server ready."
    break
  fi
  if [ "$i" -eq 60 ]; then
    echo "Server did not become ready within 60s" >&2
    exit 1
  fi
  sleep 1
done

# ── Register admin (idempotent — ignore 409/403) ──────────────────────────────
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE/api/auth/register" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"$ADMIN_USER\",\"password\":\"$ADMIN_PASS\"}")
if [ "$STATUS" != "200" ] && [ "$STATUS" != "409" ] && [ "$STATUS" != "403" ]; then
  echo "Register failed with HTTP $STATUS" >&2
  exit 1
fi

# ── Login → token ─────────────────────────────────────────────────────────────
TOKEN=$(curl -sf -X POST "$BASE/api/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"$ADMIN_USER\",\"password\":\"$ADMIN_PASS\"}" \
  | grep -o '"token":"[^"]*"' | cut -d'"' -f4)

if [ -z "$TOKEN" ]; then
  echo "Login failed — could not extract token" >&2
  exit 1
fi

echo "Token acquired."

# ── Run Hurl tests ─────────────────────────────────────────────────────────────
# Run in defined order; each file is independently runnable with base+token vars.
HURL_FILES=(
  tests/version.hurl
  tests/auth.hurl
  tests/settings.hurl
  tests/titles.hurl
  tests/sources.hurl
  tests/plugins.hurl
  tests/queue.hurl
  tests/logs.hurl
)

hurl \
  --variable "base=$BASE" \
  --variable "token=$TOKEN" \
  --report-junit "$JUNIT_FILE" \
  --test \
  "${HURL_FILES[@]}"

HURL_EXIT=$?

# ── Convert JUnit XML → Allure JSON with layer=api ────────────────────────────
node "$(dirname "$0")/junit-to-allure.mjs" "$JUNIT_FILE" "$RESULTS_DIR"

echo "Allure results written to $RESULTS_DIR"
exit $HURL_EXIT
