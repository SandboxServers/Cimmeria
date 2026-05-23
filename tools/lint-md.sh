#!/usr/bin/env bash
# Run markdownlint-cli2 against the repo. Warn-only — exit code is reported
# but never fails CI; locally it lets you choose to fix or not.
#
# Usage:
#   tools/lint-md.sh                    # lint all *.md per .markdownlint-cli2.yaml
#   tools/lint-md.sh --fix              # auto-fix what's auto-fixable
#   tools/lint-md.sh path/to/file.md    # lint a single file
#
# Companion: tools/lint-md.ps1 (PowerShell equivalent).

set -uo pipefail

# Move to the repo root (one level up from this script). Bail loudly if the
# cd fails so we don't accidentally lint whatever the current directory
# happens to contain.
cd "$(dirname "$0")/.." || {
  echo "tools/lint-md.sh: could not cd into repo root from $(dirname "$0")/.." >&2
  exit 1
}

# Prefer the project-local node_modules if it's installed; fall back to npx,
# which fetches markdownlint-cli2 on demand. The version pin in package.json
# keeps both paths in sync.
if [ -x "node_modules/.bin/markdownlint-cli2" ]; then
  exec node_modules/.bin/markdownlint-cli2 "$@"
fi

if command -v npx >/dev/null 2>&1; then
  exec npx --yes markdownlint-cli2 "$@"
fi

echo "tools/lint-md.sh: neither node_modules/.bin/markdownlint-cli2 nor npx is available." >&2
echo "Install Node.js (https://nodejs.org), then run 'npm install' from the repo root." >&2
exit 1
