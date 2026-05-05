#!/usr/bin/env bash
set -euo pipefail

# Cimmeria Issue Resolution Apply Script
# Resolves GitHub issues: 207, 205, 191, 190, 167, 166, 149
#
# Usage:
#   1. Save this script as apply-cimmeria-fixes.sh
#   2. chmod +x apply-cimmeria-fixes.sh
#   3. Place the 6 .patch files in the same directory
#   4. cd /path/to/your/Cimmeria/clone
#   5. ../apply-cimmeria-fixes.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== Cimmeria Issue Resolution Script ==="
echo ""

# Verify we're in a Cimmeria repo
if [[ ! -f "Cargo.toml" ]] || ! grep -q "cimmeria" Cargo.toml 2>/dev/null; then
    echo "ERROR: Run this script from the root of your Cimmeria clone."
    exit 1
fi

# Verify patches exist
PATCHES=(
    "issue-207-codecov.patch"
    "issue-149-space-manager-helper.patch"
    "issue-190-191-npc-ai-tests.patch"
    "issue-166-purchase-test-cleanup.patch"
    "issue-205-service-tests.patch"
    "issue-167-db-tests.patch"
)

for p in "${PATCHES[@]}"; do
    if [[ ! -f "$SCRIPT_DIR/$p" ]]; then
        echo "ERROR: Missing patch file: $p (expected in $SCRIPT_DIR)"
        exit 1
    fi
done

echo "Target repo: $(git remote get-url origin 2>/dev/null || echo 'no origin')"
echo "Current branch: $(git branch --show-current)"
echo ""

read -p "Create a new branch for these fixes? [Y/n] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]] || [[ -z $REPLY ]]; then
    BRANCH_NAME="fix/issues-207-205-191-190-167-166-149-$(date +%Y%m%d)"
    git checkout -b "$BRANCH_NAME"
    echo "Created branch: $BRANCH_NAME"
else
    echo "Applying to current branch: $(git branch --show-current)"
fi

echo ""

# Helper to apply a patch and commit
apply_and_commit() {
    local patch_file="$1"
    local commit_msg="$2"
    local issue_nums="$3"

    echo "→ Applying $patch_file ..."
    if git apply --check "$SCRIPT_DIR/$patch_file" 2>/dev/null; then
        git apply "$SCRIPT_DIR/$patch_file"
        git add -A
        git commit -m "$commit_msg" -m "Resolves: $issue_nums"
        echo "  ✓ Committed"
    else
        echo "  ⚠ Patch check failed — attempting with 3-way merge context..."
        if git apply --3way "$SCRIPT_DIR/$patch_file" 2>/dev/null; then
            git add -A
            git commit -m "$commit_msg" -m "Resolves: $issue_nums"
            echo "  ✓ Committed (with 3-way merge)"
        else
            echo "  ✗ FAILED to apply $patch_file — resolve manually and run:"
            echo "      git apply --reject $SCRIPT_DIR/$patch_file"
            return 1
        fi
    fi
    echo ""
}

apply_and_commit \
    "issue-207-codecov.patch" \
    "ci(codecov): ratchet targets + component statuses + ignore deferred files" \
    "#207"

apply_and_commit \
    "issue-149-space-manager-helper.patch" \
    "refactor(tests): extract shared make_space_manager helper" \
    "#149"

apply_and_commit \
    "issue-190-191-npc-ai-tests.patch" \
    "test(npc-ai): top-threat selection, NaN handling, leash witness wire ordering" \
    "#190, #191"

apply_and_commit \
    "issue-166-purchase-test-cleanup.patch" \
    "test(vendor): fix outbox cleanup with bound entity_id parameter" \
    "#166"

apply_and_commit \
    "issue-205-service-tests.patch" \
    "test(services): cover ticks, base_messages, resources, world_entry, startup" \
    "#205"

apply_and_commit \
    "issue-167-db-tests.patch" \
    "test(auth+character): malformed XML + delete_character no-DB short-circuit" \
    "#167"

echo "=== All patches applied successfully ==="
echo ""
echo "Next steps:"
echo "  1. Review the commits:  git log --oneline -6"
echo "  2. Run tests:             cargo test --workspace --exclude cimmeria-app"
echo "  3. Push the branch:       git push -u origin $(git branch --show-current)"
echo "  4. Open PR(s) on GitHub and reference the issues in the description"
echo ""
