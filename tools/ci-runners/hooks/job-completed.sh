#!/usr/bin/env bash
# Runs after every job, pass or fail, to record the job's cache use and keep the runner's
# disk use bounded. In detail, it records what the job did with the caches, then:
#   - one JSON line per job in state/jobs.jsonl: sccache requests, hits and misses
#     for this job, and the size of target/, sccache and the cargo registry
#     (tools/ci-runners/status.ps1 summarizes it);
#   - the job's target/ (the bulk: tens of GB) goes now, not at the next checkout;
#   - RUNNER_TEMP goes, and /tmp entries older than an hour (the listener's .NET files stay);
#   - the dind daemon drops stopped containers, unused networks and volumes and dangling
#     images; tagged images stay (postgres:17.9 stays warm). Unused tagged images go
#     only when the daemon's store passes 20 GB. (`image prune --filter until=` goes by
#     when an image was built, not last used, so it would delete postgres every job.)
# sccache trims itself to SCCACHE_CACHE_SIZE; the cargo registry is small.
#
# The runner runs this with `bash -e -o pipefail`. A hook must never fail the job, and
# on a quick runner (no dind) every docker command fails, so turn both off here.
set +e +o pipefail -u
ws="${GITHUB_WORKSPACE:-}"
# du's size in KB, or 0 when the path is missing (cut exits 0 on empty input, so a
# `|| echo 0` after the pipe would never fire).
kb() { local v; v=$(du -sk "$1" 2> /dev/null | cut -f1); echo "${v:-0}"; }
target_kb=0
[ -n "$ws" ] && target_kb=$(kb "$ws/target")

# Stats are per sccache server; stopping it resets them for the next job. Every value
# goes to Python as an argument, never spliced into its source (a branch name may
# contain quotes).
stats="$(sccache --show-stats --stats-format json 2> /dev/null || true)"
sccache --stop-server > /dev/null 2>&1 || true
python3 -c '
import json, sys, datetime
stats, runner, job, run, ref, target_kb, sccache_kb, registry_kb, free_kb = sys.argv[1:10]
try:
    s = json.loads(stats)["stats"]
except Exception:
    s = {}
def total(v):
    if isinstance(v, dict):
        return sum(total(x) for x in v.values())
    return v if isinstance(v, int) else 0
def num(v):
    try:
        return int(v)
    except ValueError:
        return 0
print(json.dumps({
    "at": datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds"),
    "runner": runner, "job": job, "run": run, "ref": ref,
    "requests": s.get("compile_requests", 0),
    "hits": total(s.get("cache_hits", {}).get("counts", {})),
    "misses": total(s.get("cache_misses", {}).get("counts", {})),
    "non_cacheable": s.get("non_cacheable_compilations", 0),
    "target_kb": num(target_kb), "sccache_kb": num(sccache_kb),
    "registry_kb": num(registry_kb), "work_free_kb": num(free_kb),
}))
' "$stats" "${RUNNER_NAME:-}" "${GITHUB_JOB:-}" "${GITHUB_RUN_ID:-}" "${GITHUB_REF:-}" \
    "$target_kb" "$(kb /home/runner/sccache)" "$(kb "$CARGO_HOME/registry")" \
    "$(df -k --output=avail /home/runner/_work | tail -1 | tr -d ' ')" \
    >> /home/runner/state/jobs.jsonl 2> /dev/null || true# Keep the log itself small.
tail -n 500 /home/runner/state/jobs.jsonl > /home/runner/state/jobs.jsonl.tmp 2> /dev/null \
    && mv /home/runner/state/jobs.jsonl.tmp /home/runner/state/jobs.jsonl

if [ -n "$ws" ] && [ -d "$ws" ]; then
    rm -rf "$ws/target" 2> /dev/null || sudo rm -rf "$ws/target"
fi
if [ -n "${RUNNER_TEMP:-}" ] && [ -d "$RUNNER_TEMP" ]; then
    find "$RUNNER_TEMP" -mindepth 1 -delete 2> /dev/null || sudo find "$RUNNER_TEMP" -mindepth 1 -delete
fi
sudo find /tmp -mindepth 1 -maxdepth 1 -mmin +60 ! -name '.dotnet' ! -name 'clr-debug-pipe-*' ! -name 'dotnet-diagnostic-*' ! -name 'CoreFxPipe_*' -exec rm -rf {} + 2> /dev/null
docker container prune -f > /dev/null 2>&1
docker network prune -f > /dev/null 2>&1
docker volume prune -af > /dev/null 2>&1
docker image prune -f > /dev/null 2>&1
images_bytes=$(docker system df --format '{{.Type}} {{.Size}}' 2> /dev/null | awk '$1=="Images"{print $2}' | numfmt --from=si --suffix=B 2> /dev/null | sed 's/B$//' || echo 0)
if [ "${images_bytes:-0}" -gt 21474836480 ] 2> /dev/null; then
    docker image prune -af > /dev/null 2>&1
fi
exit 0
