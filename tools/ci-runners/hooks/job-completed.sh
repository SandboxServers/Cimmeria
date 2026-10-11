#!/usr/bin/env bash
# Runs after every job, pass or fail. It records what the job did with the caches,
# then keeps the runner's disk use bounded:
#   - one JSON line per job in state/jobs.jsonl: sccache requests, hits and misses
#     for this job, and the size of target/, sccache and the cargo registry
#     (tools/ci-runners/status.ps1 summarizes it);
#   - the job's target/ (the bulk: tens of GB) goes now, not at the next checkout;
#   - RUNNER_TEMP and the runner's own /tmp files go;
#   - the dind daemon drops stopped containers, unused networks and volumes and dangling
#     images; tagged images stay (postgres:17.9 stays warm). Unused tagged images go
#     only when the daemon's store passes 20 GB. (`image prune --filter until=` goes by
#     when an image was built, not last used, so it would delete postgres every job.)
# sccache trims itself to SCCACHE_CACHE_SIZE; the cargo registry is small.
set -u
ws="${GITHUB_WORKSPACE:-}"
kb() { du -sk "$1" 2> /dev/null | cut -f1 || echo 0; }

# Stats are per sccache server; stopping it resets them for the next job.
stats="$(sccache --show-stats --stats-format json 2> /dev/null || true)"
sccache --stop-server > /dev/null 2>&1 || true
python3 - "$stats" <<EOF >> /home/runner/state/jobs.jsonl 2> /dev/null || true
import json, sys, datetime
try:
    s = json.loads(sys.argv[1])["stats"]
except Exception:
    s = {}
def total(v):
    if isinstance(v, dict):
        return sum(total(x) for x in v.values())
    return v if isinstance(v, int) else 0
print(json.dumps({
    "at": datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds"),
    "runner": "${RUNNER_NAME:-}",
    "job": "${GITHUB_JOB:-}",
    "run": "${GITHUB_RUN_ID:-}",
    "ref": "${GITHUB_REF:-}",
    "requests": s.get("compile_requests", 0),
    "hits": total(s.get("cache_hits", {}).get("counts", {})),
    "misses": total(s.get("cache_misses", {}).get("counts", {})),
    "non_cacheable": s.get("non_cacheable_compilations", 0),
    "target_kb": $( [ -n "$ws" ] && kb "$ws/target" || echo 0 ),
    "sccache_kb": $(kb /home/runner/sccache),
    "registry_kb": $(kb "$CARGO_HOME/registry"),
    "work_free_kb": $(df -k --output=avail /home/runner/_work | tail -1 | tr -d ' '),
}))
EOF
# Keep the log itself small.
tail -n 500 /home/runner/state/jobs.jsonl > /home/runner/state/jobs.jsonl.tmp 2> /dev/null \
    && mv /home/runner/state/jobs.jsonl.tmp /home/runner/state/jobs.jsonl

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
docker image prune -f > /dev/null 2>&1
images_bytes=$(docker system df --format '{{.Type}} {{.Size}}' 2> /dev/null | awk '$1=="Images"{print $2}' | numfmt --from=si --suffix=B 2> /dev/null | sed 's/B$//' || echo 0)
if [ "${images_bytes:-0}" -gt 21474836480 ] 2> /dev/null; then
    docker image prune -af > /dev/null 2>&1
fi
exit 0
