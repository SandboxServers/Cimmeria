---
name: workflow-revert-audit
description: Per-PR revert+run audit workflow that has caught test theatre on past PRs — the gold-standard verification method
metadata:
  type: feedback
---

The audit workflow:

1. **Each open PR has its own worktree** at `.claude/worktrees/agent-<hash>` (locked, named after `gh pr view N --json headRefName`). PR #432 was an exception (no worktree pre-existed) — I created `audit-pr432` ad-hoc, junction-linked `external/`, ran verification, and pruned.
2. **For each trap guard**: Edit the production code to revert (NOT git stash — use Edit tool with explicit `REVERTED for audit:` comment). Run the test. Confirm the failure shape matches the agent's claim. Use Edit to restore. End with `git diff --stat` showing only `.claude/settings.local.json` drift.
3. **Each cargo test run that needs `LogCapture` requires `-- --test-threads=1`** (PR #433 matrix used this). The tracing subscriber install in `LogCapture::install()` races under in-process parallel tests.
4. **Live-DB rollback tests should NOT be run unless local Postgres is up.** They self-skip via `require_db_or_skip!`. PR #438 had 7 of these — I validated by static read of assertion exactness rather than runtime revert.
5. **Cooked-asset-dependent tests silently self-skip** when assets aren't present — `castle_cellblock_walks_static_mesh_actors` "passes" but actually returned early. Flag this in audit output.

**Why:** The user explicitly warned "Don't trust the agents' self-reports that they verified their tests under revert — the prior round caught real theatre on PR #427 even though the agent claimed it had verified." Revert + run is the only audit method that surfaces theatre.

**How to apply:** Run on highest-trap-risk PRs first (trading state machine, security bounds checks, dispatch routing matrices). Lower-risk PRs (pure deletions, asset-dependent tests) can be static-read for assertion exactness.

Related: [[finding_log_capture_serial]], [[finding_external_junction]]
