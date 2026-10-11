#!/usr/bin/env bash
# Registers the runner on first start (RUNNER_TOKEN required then), restores its
# registration from the state volume afterwards, waits for the paired dind daemon,
# and runs the listener. See tools/ci-runners/README.md.
set -euo pipefail

# A named volume first mounted by the root-only dind sidecar comes up root-owned.
sudo chown runner:runner /home/runner/_work /home/runner/state /home/runner/sccache \
    "$CARGO_HOME/registry" "$CARGO_HOME/git"

cd /home/runner/actions-runner
STATE=/home/runner/state
FILES=(.runner .credentials .credentials_rsaparams)

: "${REPO_URL:?REPO_URL is required}"
: "${RUNNER_NAME:?RUNNER_NAME is required}"
: "${RUNNER_LABELS:=cimmeria-linux}"

if [ -f "$STATE/.runner" ]; then
    for f in "${FILES[@]}"; do cp "$STATE/$f" .; done
elif [ ! -f .runner ]; then
    RUNNER_TOKEN="$(tr -d '\r\n' < /run/runner-token 2> /dev/null || true)"
    if [ -z "$RUNNER_TOKEN" ]; then
        echo "entrypoint: $RUNNER_NAME is not registered and RUNNER_TOKEN is empty;" >&2
        echo "entrypoint: run tools/ci-runners/deploy.ps1 to mint a registration token." >&2
        exit 64
    fi
    ./config.sh --unattended --replace \
        --url "$REPO_URL" --token "$RUNNER_TOKEN" \
        --name "$RUNNER_NAME" --labels "$RUNNER_LABELS" \
        --work /home/runner/_work
    for f in "${FILES[@]}"; do cp "$f" "$STATE/"; done
fi
unset RUNNER_TOKEN

# Environment every job step sees (the runner reads .env at start).
cat > .env <<EOF
LANG=C.UTF-8
PATH=$PATH
CARGO_HOME=$CARGO_HOME
RUSTUP_HOME=$RUSTUP_HOME
DOCKER_HOST=${DOCKER_HOST:-}
RUSTC_WRAPPER=sccache
SCCACHE_DIR=/home/runner/sccache
SCCACHE_CACHE_SIZE=${SCCACHE_CACHE_SIZE:-20G}
SCCACHE_IDLE_TIMEOUT=0
CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-}
ACTIONS_RUNNER_HOOK_JOB_STARTED=/home/runner/hooks/job-started.sh
ACTIONS_RUNNER_HOOK_JOB_COMPLETED=/home/runner/hooks/job-completed.sh
EOF
# The shared compile cache (cache.compose.yml), when this node is given one. sccache
# prefers Redis over its local SCCACHE_DIR whenever the endpoint is set, so set it only
# when it is non-empty.
if [ -n "${CACHE_ENDPOINT:-}" ]; then
    echo "SCCACHE_REDIS_ENDPOINT=$CACHE_ENDPOINT" >> .env
    echo "SCCACHE_REDIS_PASSWORD=${CACHE_PASSWORD:-}" >> .env
fi
unset CACHE_PASSWORD
# The routing proxy (compose.yml `egress`), for the runner and every job step.
for v in HTTPS_PROXY https_proxy NO_PROXY no_proxy; do
    if [ -n "${!v:-}" ]; then echo "$v=${!v}" >> .env; fi
done
# An empty CARGO_BUILD_JOBS would be an error for cargo.
sed -i '/^CARGO_BUILD_JOBS=$/d' .env

# Quick runners (compose.yml `quick*`) have no dind and leave DOCKER_HOST empty.
if [ -n "${DOCKER_HOST:-}" ]; then
    for _ in $(seq 1 60); do
        docker info > /dev/null 2>&1 && break
        sleep 2
    done
    docker info > /dev/null 2>&1 || { echo "entrypoint: dind at $DOCKER_HOST never came up" >&2; exit 69; }
fi

exec ./run.sh
