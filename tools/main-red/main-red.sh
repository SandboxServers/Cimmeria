#!/usr/bin/env bash
# Open, update or close the "main is red" issue for one post-merge check.
#
#   tools/main-red/main-red.sh fail <check> <run-url>
#   tools/main-red/main-red.sh pass <check> <run-url>
#
# One open issue per <check>, titled "main is red: <check>" and labelled `main-red`.
# `fail` opens it, or comments on the open one; `pass` closes it with a comment, or does
# nothing when none is open. tools/build-lane/ship.py refuses to merge code PRs while any
# `main-red` issue is open, so a broken main is found when it breaks, not on release day.
#
# Needs GH_TOKEN with issues: write, and GITHUB_REPOSITORY (set in every Actions job).
set -euo pipefail

usage() { echo "usage: main-red.sh fail|pass <check> <run-url>" >&2; exit 2; }
[ $# -eq 3 ] || usage
verdict=$1 check=$2 url=$3
repo=${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is not set}
title="main is red: $check"

# Label filter and an exact title match, not --search: the search index lags, and a
# fail right after a fail would open a second issue.
open=$(gh issue list --repo "$repo" --label main-red --state open --limit 100 \
         --json number,title --jq "[.[] | select(.title == \"$title\") | .number] | first // empty")

case "$verdict" in
  fail)
    if [ -n "$open" ]; then
      gh issue comment "$open" --repo "$repo" --body "Still failing: $url"
      echo "::error::$title (issue #$open, updated)"
    else
      body="The post-merge check **$check** failed on main: $url

Until this issue is closed, \`tools/build-lane/ship.py merge\` refuses code PRs
(\`--allow-red-main\` is for the PR that fixes it). The next passing run closes it.
See docs/agents/pre-pr-checks.md § Main is red."
      new=$(gh issue create --repo "$repo" --label main-red --title "$title" --body "$body")
      echo "::error::$title ($new)"
    fi
    ;;
  pass)
    if [ -n "$open" ]; then
      gh issue close "$open" --repo "$repo" --comment "Passing again: $url"
      echo "closed #$open: $title"
    fi
    ;;
  *) usage ;;
esac
