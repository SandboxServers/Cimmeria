#!/usr/bin/env bash
# Smoke test: verify AuthenticationServer can start and listen.
# Validates: binary loads -> reads config -> connects to DB -> opens port.
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

BIN_DIR="$BUILD_DIR/bin"
AUTH_PORT=8081
TIMEOUT=10

cleanup() {
    if [[ -n "${AUTH_PID:-}" ]] && kill -0 "$AUTH_PID" 2>/dev/null; then
        kill "$AUTH_PID" 2>/dev/null || true
        wait "$AUTH_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

log_step "SMOKE TEST"

# --- Preconditions ---
if [[ ! -x "$BIN_DIR/AuthenticationServer" ]]; then
    die "AuthenticationServer not found. Run 'make build' first."
fi

# Ensure database is available
if ! nc -z 127.0.0.1 "${CIMMERIA_DB_PORT:-5433}" 2>/dev/null; then
    log_status "PostgreSQL not running, starting Docker..."
    source "$(dirname "${BASH_SOURCE[0]}")/docker.sh"
    docker_up
    source "$(dirname "${BASH_SOURCE[0]}")/database.sh"
    db_init
fi

# --- Ensure config is visible from build/bin (server looks for ../config/) ---
if [[ ! -e "$BUILD_DIR/config" ]]; then
    ln -s "$CONFIG_DIR" "$BUILD_DIR/config"
fi

# --- Start AuthenticationServer ---
log_status "Starting AuthenticationServer..."
cd "$BIN_DIR"
./AuthenticationServer &
AUTH_PID=$!

# --- Wait for port ---
log_status "Waiting for port $AUTH_PORT (${TIMEOUT}s timeout)..."
if wait_for_port "$AUTH_PORT" "$TIMEOUT"; then
    log_ok "AuthenticationServer listening on port $AUTH_PORT (PID $AUTH_PID)."
else
    if kill -0 "$AUTH_PID" 2>/dev/null; then
        log_err "Process alive but not listening on port $AUTH_PORT within ${TIMEOUT}s."
    else
        log_err "AuthenticationServer exited prematurely."
    fi
    die "FAIL: smoke test failed."
fi

# --- Verify process is still alive ---
if kill -0 "$AUTH_PID" 2>/dev/null; then
    log_ok "Process still alive after port check."
else
    die "FAIL: AuthenticationServer died after opening port."
fi

# --- Stop server ---
log_status "Stopping AuthenticationServer..."
kill "$AUTH_PID" 2>/dev/null || true
wait "$AUTH_PID" 2>/dev/null || true
AUTH_PID=""

printf "\n"
printf "${GREEN}=============================================${NC}\n"
printf "${GREEN} PASS: Smoke test passed.${NC}\n"
printf "${GREEN}=============================================${NC}\n"
