---
name: revert-to-verify-regression-guards
description: When auditing a PR's regression guards, revert the fix locally and rerun the test before accepting it
metadata:
  type: feedback
---

When auditing a PR that claims to add regression guards, the only reliable validation is to revert the fix in the worktree (via git stash or surgical edit) and rerun the test. A test that passes whether the fix is in place or reverted is theatre, no matter how good the docstring reads.

**Why:** PR #427 had two routing tests (`spend_applied_science_points_routes_to_crafting`, `craft_routes_to_crafting`) whose docstrings described exactly the bug shape they claimed to guard. Both passed cleanly with the outer-router fix reverted, because they called the inner submodule's dispatch directly and bypassed the outer router entirely. The bug was 100% invisible to the test suite until I pushed a LogCapture-based replacement to `dispatch.rs`'s test module.

**How to apply:** For any test labeled "regression guard" in a PR review:

1. Identify the *fix commit* the test claims to guard (usually the one immediately preceding the test commit).
2. Stash unrelated working-tree changes (`git stash push <file>`).
3. Revert the fix surgically — prefer `Edit` over `git revert` so the test commit stays intact and only the production code changes.
4. Run only the named test: `cargo test -p <crate> --lib <test::path>`.
5. If it passes with the fix reverted, the test is theatre. Restore the fix (`git checkout <file>`) and write a replacement.
6. Push the replacement as a new commit on the PR branch.

The high-value reverts to check: `if (X..=Y)` range narrowing (because submodules often have their own arms), `rows_affected() == 0` checks (because seed data variants can mask which path fires), `checked_mul` / bounds checks (because the dropped check might still panic under debug overflow checks).

For live-DB tests where the local env can't run them, document the static-analysis reasoning explicitly in the audit comment instead of skipping the check. CI on a reverted branch is the fallback verification.

Related: [[gotcha-inner-dispatch-bypasses-outer-router]], [[gotcha-social-arm-shadows-crafting-95]]
