#!/usr/bin/env bash
# Start/stop Cimmeria game servers.
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

CONFIGURATION="${CIMMERIA_CONFIG:-Debug}"
BIN_DIR="$BUILD_DIR/bin"

_suffix() {
    if [[ "$CONFIGURATION" == "Debug" ]]; then echo "_d"; else echo ""; fi
}

_ensure_pid_dir() {
    mkdir -p "$PID_DIR"
}

_start_process() {
    local name="$1"
    local exe="$2"
    local port="${3:-}"
    local timeout="${4:-15}"

    if [[ ! -x "$exe" ]]; then
        die "Executable not found: $exe. Run 'make build' first."
    fi

    log_status "Starting $name..."
    cd "$BIN_DIR"
    "$exe" &
    local pid=$!
    echo "$pid" > "$PID_DIR/${name}.pid"

    if [[ -n "$port" ]]; then
        if wait_for_port "$port" "$timeout"; then
            log_ok "$name ready (port $port, PID $pid)."
        else
            log_warn "$name did not respond on port $port within ${timeout}s (PID $pid)."
            if kill -0 "$pid" 2>/dev/null; then
                log_warn "  Process is alive — may still be initializing."
            else
                die "$name failed to start."
            fi
        fi
    else
        if kill -0 "$pid" 2>/dev/null; then
            log_ok "$name started (PID $pid)."
        else
            die "$name failed to start."
        fi
    fi
}

_stop_process() {
    local name="$1"
    local pidfile="$PID_DIR/${name}.pid"

    if [[ -f "$pidfile" ]]; then
        local pid
        pid=$(cat "$pidfile")
        if kill -0 "$pid" 2>/dev/null; then
            log_status "Stopping $name (PID $pid)..."
            kill "$pid" 2>/dev/null || true
            for i in $(seq 1 10); do
                if ! kill -0 "$pid" 2>/dev/null; then break; fi
                sleep 0.5
            done
            if kill -0 "$pid" 2>/dev/null; then
                kill -9 "$pid" 2>/dev/null || true
            fi
        fi
        rm -f "$pidfile"
    else
        pkill -f "$name" 2>/dev/null || true
    fi
}

server_start() {
    log_step "STARTING CIMMERIA SERVER"

    if ! nc -z 127.0.0.1 "${CIMMERIA_DB_PORT:-5433}" 2>/dev/null; then
        log_status "PostgreSQL not running, starting Docker..."
        source "$(dirname "${BASH_SOURCE[0]}")/docker.sh"
        docker_up
    fi

    _ensure_pid_dir

    local sfx
    sfx=$(_suffix)

    _start_process "AuthenticationServer" "$BIN_DIR/AuthenticationServer${sfx}" 8081 15
    _start_process "BaseApp" "$BIN_DIR/BaseApp${sfx}" 32832 15
    _start_process "CellApp" "$BIN_DIR/CellApp${sfx}" "" ""

    printf "\n"
    printf "${GREEN}=============================================${NC}\n"
    printf "${GREEN} Cimmeria Server Running${NC}\n"
    printf "${GREEN}=============================================${NC}\n"
    printf " Auth server:   http://localhost:8081 (client login)\n"
    printf " Shard server:  localhost:32832 (game client)\n"
    printf " PostgreSQL:    localhost:${CIMMERIA_DB_PORT:-5433}\n"
    printf " Test account:  test / test\n"
    printf "${GREEN}=============================================${NC}\n"
    printf "\n"
    printf " Stop with: make stop\n"
    printf "${GREEN}=============================================${NC}\n"
}

server_stop() {
    log_step "STOPPING CIMMERIA SERVER"

    _stop_process "CellApp"
    _stop_process "BaseApp"
    _stop_process "AuthenticationServer"

    log_ok "All game servers stopped."
}

trap 'echo ""; server_stop; exit 0' INT TERM

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    case "${1:-}" in
        start) server_start ;;
        stop)  server_stop ;;
        *)     echo "Usage: $0 {start|stop}" ;;
    esac
fi
