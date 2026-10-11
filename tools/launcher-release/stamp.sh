#!/usr/bin/env bash
# Stamp a launcher release: the first step of launcher-release.yml's
# desktop-windows job. Writes tag, epoch, version and same_day to
# $GITHUB_OUTPUT.
#
# The tag and the build time, as before: a manifest's `min_launcher` compares
# them. The compiled version is D-LX8's `YYYY.M.D` from the same date, which
# the updater compares as SemVer. When dated releases already exist for that
# day this one is `YYYY.M.D-N`, N its place among the day's releases by
# creation time (the second is -2). A re-run of a run that already published
# its tag keeps that tag's place; the same commit dispatched again on the same
# day would need the same tag, so it is refused here, before anything is
# built. The stable channel refuses a prerelease, so a same-day build is never
# offered as an update; it is published as its dated release only and
# launcher-current keeps the day's first release.
#
# Environment: GH_TOKEN, GH_REPO and the GITHUB_* variables Actions sets.
set -euo pipefail

SHORT_SHA="${GITHUB_SHA:0:7}"
EPOCH=$(date -u +%s)
DATE=$(date -u -d "@${EPOCH}" +%Y%m%d)
TAG="launcher-${DATE}-${SHORT_SHA}"
BASE="$(date -u -d "@${EPOCH}" +%Y).$(date -u -d "@${EPOCH}" +%-m).$(date -u -d "@${EPOCH}" +%-d)"
# Two lines: this tag's 0-based place among the day's dated releases, oldest
# first (-1 when it has none), and their count. Captured first, so a failed
# listing fails the step instead of silently counting as "no releases today".
LISTING="$(gh release list --limit 200 --json tagName,createdAt --jq "
  [.[] | select(.tagName | startswith(\"launcher-${DATE}-\"))]
  | sort_by(.createdAt) | map(.tagName)
  | ((to_entries | map(select(.value == \"${TAG}\")) | .[0].key // -1), length)")"
mapfile -t DAY <<<"${LISTING}"
PLACE="${DAY[0]:-}"; COUNT="${DAY[1]:-}"
[[ "${PLACE}" =~ ^-?[0-9]+$ && "${COUNT}" =~ ^[0-9]+$ ]] \
  || { echo "::error::could not read today's releases: ${LISTING}"; exit 1; }
RUN_URL="${GITHUB_SERVER_URL}/${GITHUB_REPOSITORY}/actions/runs/${GITHUB_RUN_ID}"
if [ "${PLACE}" -ge 0 ]; then
  # The dated release's notes name the run that published it.
  PUBLISHED_BODY="$(gh release view "${TAG}" --json body --jq .body)"
  if grep -qF "${RUN_URL}" <<<"${PUBLISHED_BODY}"; then
    N=$((PLACE + 1))
    echo "re-run: ${TAG} was published by this run; it keeps its place (${N})"
  else
    echo "::error::${TAG} is already released by another run: commit ${SHORT_SHA} was released today. A second release of the same commit on the same day would need the same tag. Merge another commit first, or re-run the run that published ${TAG}."
    exit 1
  fi
else
  N=$((COUNT + 1))
fi
if [ "${N}" -eq 1 ]; then
  VERSION="${BASE}"; SAME_DAY=false
else
  VERSION="${BASE}-${N}"; SAME_DAY=true
fi
[[ "${VERSION}" =~ ^[0-9]{4}\.[1-9][0-9]?\.[1-9][0-9]?(-[0-9]+)?$ ]] \
  || { echo "::error::computed version ${VERSION} is not D-LX8 SemVer"; exit 1; }
{
  echo "tag=${TAG}"
  echo "epoch=${EPOCH}"
  echo "version=${VERSION}"
  echo "same_day=${SAME_DAY}"
} >> "$GITHUB_OUTPUT"
echo "tag ${TAG}, version ${VERSION}, dated releases today: ${COUNT}"
