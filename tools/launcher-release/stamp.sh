#!/usr/bin/env bash
# Reserve a stable UTC day-slot identity for every signed launcher release (D-LX15).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
# The run creation time remains fixed across reruns and midnight boundaries.
STAMP_EPOCH="$(gh api "repos/${GITHUB_REPOSITORY}/actions/runs/${GITHUB_RUN_ID}" --jq .created_at | xargs -I{} date -u -d '{}' +%s)"
export STAMP_EPOCH
# Slurp every page; an API failure must never masquerade as an empty day.
LISTING="$(gh api --paginate --slurp "repos/${GITHUB_REPOSITORY}/releases?per_page=100")"
python "$HERE/release_identity.py" stamp <<<"$LISTING"
