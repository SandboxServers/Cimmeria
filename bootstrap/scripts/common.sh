#!/usr/bin/env bash
# Common helpers for Cimmeria bootstrap scripts.
# Source this file; do not execute directly.

set -euo pipefail

# --- Colors (disabled if not a terminal) ---
if [[ -t 1 ]]; then
    RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[0;33m'
    CYAN='\033[0;36m'; GRAY='\033[0;90m'; BOLD='\033[1m'; NC='\033[0m'
else
    RED=''; GREEN=''; YELLOW=''; CYAN=''; GRAY=''; BOLD=''; NC=''
fi

# --- Project paths ---
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
EXTERNAL_DIR="$PROJECT_ROOT/external"
BUILD_DIR="$PROJECT_ROOT/build"
CONFIG_DIR="$PROJECT_ROOT/config"
DB_DIR="$PROJECT_ROOT/db"
SERVER_DIR="$PROJECT_ROOT/server"
PID_DIR="$SERVER_DIR/pids"

# --- Timing ---
_START_TIME=$(date +%s)

_elapsed() {
    local now=$(date +%s)
    local diff=$((now - _START_TIME))
    printf "%02d:%02d" $((diff / 60)) $((diff % 60))
}

# --- Logging ---
log_status() {
    local msg="$1"
    local color="${2:-$GRAY}"
    printf "${color}[$(_elapsed)]  %s${NC}\n" "$msg"
}

log_step() {
    local msg="$1"
    printf "\n${CYAN}[$(_elapsed)] === %s ===${NC}\n" "$msg"
}

log_ok()    { log_status "$1" "$GREEN"; }
log_warn()  { log_status "$1" "$YELLOW"; }
log_err()   { log_status "$1" "$RED"; }

die() {
    log_err "$1"
    exit 1
}

# --- Port checking ---
wait_for_port() {
    local port="$1"
    local timeout="${2:-15}"
    local deadline=$(($(date +%s) + timeout))

    while [[ $(date +%s) -lt $deadline ]]; do
        if nc -z 127.0.0.1 "$port" 2>/dev/null; then
            return 0
        fi
        sleep 0.5
    done
    return 1
}

# --- OS detection ---
detect_os() {
    case "$(uname -s)" in
        Darwin) echo "macos" ;;
        Linux)  echo "linux" ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
        *) echo "unknown" ;;
    esac
}

OS="$(detect_os)"
