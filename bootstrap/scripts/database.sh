#!/usr/bin/env bash
# Database initialization for Cimmeria.
# Requires: Docker PostgreSQL running, psql on PATH.
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

DB_PORT="${CIMMERIA_DB_PORT:-5433}"
DB_USER="w-testing"
DB_NAME="sgw"
DB_HOST="localhost"

_psql() {
    PGPASSWORD="$DB_USER" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" "$@"
}

# Admin connection — w-testing is the superuser in our Docker setup
# (POSTGRES_USER=w-testing in docker-compose.yml)
_psql_admin() {
    PGPASSWORD="$DB_USER" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" "$@"
}

db_init() {
    local force="${1:-false}"
    log_step "INITIALIZING DATABASE"

    # Ensure Docker is running
    if ! nc -z "$DB_HOST" "$DB_PORT" 2>/dev/null; then
        log_status "PostgreSQL not running, starting Docker..."
        source "$(dirname "${BASH_SOURCE[0]}")/docker.sh"
        docker_up
    fi

    # Wait for PostgreSQL to accept queries
    log_status "Waiting for PostgreSQL to accept queries..."
    local ready=false
    for i in $(seq 1 15); do
        if _psql_admin -tAc "SELECT 1" &>/dev/null; then
            ready=true
            break
        fi
        sleep 1
    done
    if [[ "$ready" != "true" ]]; then
        die "PostgreSQL not accepting queries after 15s."
    fi

    # Force mode: drop and recreate
    if [[ "$force" == "true" ]]; then
        log_warn "Force mode: dropping and recreating database '$DB_NAME'..."
        _psql_admin -c "DROP DATABASE IF EXISTS $DB_NAME" 2>/dev/null || true
        _psql_admin -c "CREATE DATABASE $DB_NAME OWNER \"$DB_USER\"" || die "Failed to recreate database."
        log_ok "Database '$DB_NAME' recreated."
    fi

    # Check if schemas already loaded
    local table_check
    table_check=$(_psql -d "$DB_NAME" -tAc "SELECT 1 FROM information_schema.tables WHERE table_name='account'" 2>/dev/null || echo "")
    if [[ "$table_check" == *"1"* ]] && [[ "$force" != "true" ]]; then
        log_status "Schemas already loaded (account table exists). Use --force to reload." "$GRAY"
        return 0
    fi

    # Load schemas
    local db_sql="$DB_DIR/database.sql"
    if [[ ! -f "$db_sql" ]]; then
        die "Schema file not found: $db_sql"
    fi

    log_status "Loading database.sql..."
    # Disable errexit so PIPESTATUS is reachable after the pipeline
    set +e
    _psql -d "$DB_NAME" -v ON_ERROR_STOP=1 -f "$db_sql" 2>&1 | while IFS= read -r line; do
        if [[ "$line" == *ERROR* ]] || [[ "$line" == *FATAL* ]]; then
            log_err "  $line"
        fi
    done
    local psql_exit=${PIPESTATUS[0]}
    set -e

    if [[ $psql_exit -ne 0 ]]; then
        die "database.sql failed. Aborting."
    fi
    log_ok "database.sql loaded."

    # Verify key tables
    log_status "Verifying schema..."
    local all_good=true
    for table in account sgw_player; do
        local check
        check=$(_psql -d "$DB_NAME" -tAc "SELECT 1 FROM information_schema.tables WHERE table_name='$table'" 2>/dev/null || echo "")
        if [[ "$check" == *"1"* ]]; then
            log_ok "  [OK] $table"
        else
            log_err "  [--] $table"
            all_good=false
        fi
    done

    if [[ "$all_good" != "true" ]]; then
        die "Schema verification failed — some tables are missing."
    fi

    log_ok "Database initialized."
    log_status "  PostgreSQL: ${DB_HOST}:${DB_PORT}" "$GRAY"
    log_status "  Database:   $DB_NAME" "$GRAY"
    log_status "  Role:       $DB_USER" "$GRAY"
    log_status "  Test login: test / test (SHA1 hashed)" "$GRAY"
}

# Allow direct invocation
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    case "${1:-}" in
        init)   db_init false ;;
        reset)  db_init true ;;
        *)      echo "Usage: $0 {init|reset}" ;;
    esac
fi
