#!/usr/bin/env bash
# Container login smoke: start the server image, wait for it, then log in the
# way a client does with the login probe (issue #1291). Shared by
# .github/workflows/pr-container.yml and release-container.yml; runs the same
# on a workstation with Docker.
#
# Usage:
#   tools/container-smoke.sh <image> <login-probe binary> [log dir]
#
# The probe binary comes from the image build's `login-probe` target:
#   docker buildx build -f docker/Dockerfile --target login-probe \
#     --output type=local,dest=/tmp/probe .
#
# Checks, in order (any failure exits non-zero):
#   1. Postgres answers, the SOAP login port is bound and the BaseApp UDP
#      port is bound, all probed from INSIDE the container (a host-side probe
#      of a published port always connects: docker-proxy accepts the TCP
#      handshake itself). ~120 s budget.
#   2. login-probe, from the host through the published ports, as a client:
#      a wrong password is rejected as a bad password; the seeded `test`
#      account logs in (SOAP Phase 1 + 2) with the account id the database
#      holds; Phase 2 advertises BASE_EXTERNAL:BASE_PORT as the container
#      is actually configured; and the Mercury baseAppLogin handshake
#      succeeds against that advertised endpoint.
#
# Container state, `docker logs` and the server's log files (copied to
# [log dir], default ./container-smoke-logs) are kept on every run so a
# failure can be read after the container is gone.
#
# Env:
#   SMOKE_CONTAINER       container name (default cimmeria-smoke)
#   SMOKE_DOCKER_RUN_ARGS extra `docker run` arguments, e.g. "-e BASE_EXTERNAL=..."
#   SMOKE_USER / SMOKE_PASSWORD  seeded account to log in as (default test/test)

set -uo pipefail

IMAGE="${1:?usage: container-smoke.sh <image> <login-probe binary> [log dir]}"
PROBE="${2:?usage: container-smoke.sh <image> <login-probe binary> [log dir]}"
LOG_DIR="${3:-container-smoke-logs}"
NAME="${SMOKE_CONTAINER:-cimmeria-smoke}"
USER_NAME="${SMOKE_USER:-test}"
PASSWORD="${SMOKE_PASSWORD:-test}"

err() { echo "::error::$*"; }

if [ ! -f "$PROBE" ]; then
  err "login probe binary not found: $PROBE"
  exit 1
fi
chmod +x "$PROBE"
mkdir -p "$LOG_DIR"

# The image's own configuration, so the ports we publish are the ports the
# server binds. A -e override in SMOKE_DOCKER_RUN_ARGS wins inside the
# container; the advertised endpoint is read back from the running
# container below, not from here.
image_env() {
  docker image inspect -f '{{range .Config.Env}}{{println .}}{{end}}' "$IMAGE" \
    | sed -n "s/^$1=//p" | head -1
}
LOGON_PORT="$(image_env LOGON_PORT)"
BASE_PORT_IMG="$(image_env BASE_PORT)"
if [ -z "$LOGON_PORT" ] || [ -z "$BASE_PORT_IMG" ]; then
  err "image $IMAGE does not set LOGON_PORT / BASE_PORT"
  exit 1
fi

docker rm -f "$NAME" >/dev/null 2>&1 || true
# Not `--rm`: if the container exits early, --rm deletes it before we can
# read its logs. Removed explicitly in cleanup().
# shellcheck disable=SC2086  # SMOKE_DOCKER_RUN_ARGS is a word list on purpose
if ! docker run -d --name "$NAME" \
      -p "${LOGON_PORT}:${LOGON_PORT}" \
      -p "${BASE_PORT_IMG}:${BASE_PORT_IMG}/udp" \
      ${SMOKE_DOCKER_RUN_ARGS:-} \
      "$IMAGE" >/dev/null; then
  err "docker run failed for $IMAGE"
  exit 1
fi

cleanup() {
  echo "::group::container state"
  docker inspect -f 'Status: {{.State.Status}}  ExitCode: {{.State.ExitCode}}  Error: {{.State.Error}}' "$NAME" || true
  echo "::endgroup::"
  echo "::group::container logs (docker logs)"
  docker logs "$NAME" 2>&1 | tee "$LOG_DIR/docker-logs.txt" || true
  echo "::endgroup::"
  # The server's per-subsystem log files (auth.log, base.log, ...).
  docker cp "$NAME:/var/log/cimmeria/." "$LOG_DIR/server-logs" >/dev/null 2>&1 || true
  echo "server log files copied to $LOG_DIR/server-logs"
  docker rm -f "$NAME" >/dev/null 2>&1 || true
}
trap cleanup EXIT

in_container() { docker exec "$NAME" "$@"; }

# Variables as the running container sees them (image defaults plus -e).
BASE_EXTERNAL="$(in_container printenv BASE_EXTERNAL || true)"
BASE_PORT="$(in_container printenv BASE_PORT || true)"
if [ -z "$BASE_EXTERNAL" ] || [ -z "$BASE_PORT" ]; then
  err "could not read BASE_EXTERNAL / BASE_PORT from the running container"
  exit 1
fi
BASE_PORT_HEX="$(printf '%04X' "$BASE_PORT")"

# 1. Readiness. Auth starts before the BaseApp, so the login port alone is not
# enough: wait for the BaseApp's UDP socket too (/proc/net/udp{,6} list bound
# ports in hex).
ok=0
for i in $(seq 1 60); do
  if in_container pg_isready -h 127.0.0.1 -U w-testing -d sgw >/dev/null 2>&1 \
     && in_container timeout 2 bash -c "exec 3<>/dev/tcp/127.0.0.1/${LOGON_PORT}" 2>/dev/null \
     && in_container bash -c "cat /proc/net/udp /proc/net/udp6 2>/dev/null | awk '{print \$2}' | grep -qi ':${BASE_PORT_HEX}\$'"; then
    echo "container ready at iteration $i (postgres, login port ${LOGON_PORT}/tcp, BaseApp ${BASE_PORT}/udp)"
    ok=1
    break
  fi
  if [ "$(docker inspect -f '{{.State.Status}}' "$NAME" 2>/dev/null)" = "exited" ]; then
    err "container exited before becoming ready"
    break
  fi
  sleep 2
done
if [ "$ok" -ne 1 ]; then
  err "never saw postgres + login listener + BaseApp socket within 120s"
  exit 1
fi

# Expected values come from the container itself, not from constants: the
# seeded account's id from its database, the shard the server registers
# from its `shards` table (orchestrator::query_all_shards).
psql_in() {
  in_container env PGPASSWORD=w-testing psql -h 127.0.0.1 -U w-testing -d sgw -tAX -v ON_ERROR_STOP=1 -c "$1"
}
ACCOUNT_ID="$(psql_in "SELECT account_id FROM account WHERE account_name = '${USER_NAME}'")"
SHARD="$(psql_in "SELECT name FROM shards ORDER BY shard_id LIMIT 1")"
if [ -z "$ACCOUNT_ID" ] || [ -z "$SHARD" ]; then
  err "seeded account '${USER_NAME}' or a shard row is missing from the image's database"
  exit 1
fi

# 2. The login probe, as a client on the host. It dials the BaseApp at the
# address Phase 2 advertises; the published UDP port carries it in.
echo "login-probe: user=${USER_NAME} shard=${SHARD} expect account_id=${ACCOUNT_ID} base=${BASE_EXTERNAL}:${BASE_PORT}"
if ! LOGIN_PROBE_PASSWORD="$PASSWORD" "$PROBE" \
      --auth-url "http://127.0.0.1:${LOGON_PORT}" \
      --user "$USER_NAME" \
      --shard "$SHARD" \
      --expect-account-id "$ACCOUNT_ID" \
      --expect-base "${BASE_EXTERNAL}:${BASE_PORT}"; then
  err "login probe failed against $IMAGE (container and server logs follow)"
  exit 1
fi
echo "container smoke passed"
