#!/usr/bin/env bash
# Docker Compose management for Cimmeria services.
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

docker_up() {
    log_step "STARTING DOCKER SERVICES"

    if ! command -v docker &>/dev/null; then
        die "Docker not found. Install Docker Desktop or docker CLI."
    fi

    cd "$PROJECT_ROOT"
    docker compose up -d

    log_status "Waiting for PostgreSQL to be healthy..."
    local timeout=30
    local deadline=$(($(date +%s) + timeout))
    while [[ $(date +%s) -lt $deadline ]]; do
        if docker compose exec -T postgres pg_isready -U w-testing -d sgw &>/dev/null; then
            log_ok "PostgreSQL is ready (port 5433)."
            return 0
        fi
        sleep 1
    done
    die "PostgreSQL did not become healthy within ${timeout}s."
}

docker_down() {
    log_step "STOPPING DOCKER SERVICES"
    cd "$PROJECT_ROOT"
    docker compose down
    log_ok "Docker services stopped."
}

docker_status() {
    cd "$PROJECT_ROOT"
    docker compose ps
}

# Allow direct invocation: ./docker.sh up|down|status
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    case "${1:-}" in
        up)     docker_up ;;
        down)   docker_down ;;
        status) docker_status ;;
        *)      echo "Usage: $0 {up|down|status}" ;;
    esac
fi
