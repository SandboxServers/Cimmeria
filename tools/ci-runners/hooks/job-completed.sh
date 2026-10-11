#!/usr/bin/env bash
# Runs after every job, pass or fail, and keeps a runner's disk use bounded:
#   - the job's target/ (the bulk: tens of GB) goes now, not at the next checkout;
#   - RUNNER_TEMP and the runner's own /tmp files go;
#   - the dind daemon drops stopped containers, unused networks and volumes, and
#     images nobody used for a week (postgres:17.9 stays warm).
# sccache trims itself to SCCACHE_CACHE_SIZE; the cargo registry is small.
set -u
ws="${GITHUB_WORKSPACE:-}"
if [ -n "$ws" ] && [ -d "$ws" ]; then
    rm -rf "$ws/target" 2> /dev/null || sudo rm -rf "$ws/target"
fi
if [ -n "${RUNNER_TEMP:-}" ] && [ -d "$RUNNER_TEMP" ]; then
    find "$RUNNER_TEMP" -mindepth 1 -delete 2> /dev/null || sudo find "$RUNNER_TEMP" -mindepth 1 -delete
fi
sudo find /tmp -mindepth 1 -maxdepth 1 -mmin +1 -exec rm -rf {} + 2> /dev/null
docker container prune -f > /dev/null 2>&1
docker network prune -f > /dev/null 2>&1
docker volume prune -af > /dev/null 2>&1
docker image prune -af --filter until=168h > /dev/null 2>&1
exit 0
