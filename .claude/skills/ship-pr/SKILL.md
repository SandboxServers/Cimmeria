---
name: ship-pr
description: Take a change in Cimmeria from a fresh worktree to a merged PR and a retired worktree, the way this repo ships. Use when starting a packet or fix that will become a PR, when committing/pushing/opening a PR, waiting on CI, getting a review, merging, or cleaning up a worktree after merge. Triggers include "open a PR", "ship it", "push this", "wait for CI", "merge #1234", "retire the worktree", "is CI green". Covers mk-worktree, the pre-PR checks, ship.py pr/merge, watching CI without polling loops, the adversarial review, and same-day retirement.
---

# Ship a PR

The mechanical path is one script, `tools/build-lane/ship.py` (twins: `ship.ps1`, `ship.sh`).
It prints one line, `status=... key=value`, and a log path only on failure. Run everything
from PowerShell; do not use bash.

## 1. Start in your own worktree

```powershell
pwsh tools/build-lane/mk-worktree.ps1 <branch> <name>   # e.g. feat/vendor-repair vendor-repair
```

This creates `.claude/worktrees/<name>` off `origin/main`, junctions `external/` in and seeds
a warm target dir on the Dev Drive. One worktree and one test database per worker
([development-workflow.md](../../../docs/agents/development-workflow.md)).

## 2. Pre-PR checks

Read [docs/agents/pre-pr-checks.md](../../../docs/agents/pre-pr-checks.md) before your first
PR and whenever a check fails. Every compiling command goes through the lane (see the
`lane-build` skill).

- Blocking on every PR that touches more than Markdown, `.claude/` or doc images: `fmt`,
  `clippy`, `build-and-test`, `test-live-db`.
- Blocking only when figures or `docs/drafts/spec/` chapters change: `figure-sources-in-sync`,
  `figure-style-lint`.
- Warn-only: `markdownlint`, `spec-lint`.

Before opening the PR, also:

- Runtime behavior changed → a test was added that fails when the fix is reverted ([TESTING.md](../../../TESTING.md)).
- Walk [docs/agents/doc-update-map.md](../../../docs/agents/doc-update-map.md) and list the rows you touched in the PR body.
- Project/reference facts learned along the way go in `.claude/agent-memory/<agent>/` and are committed **with this change**.
- `docs/gap-analysis*` and `docs/project-status.md` change only in a campaign close-out or release packet.
- `docs/**/*.md` is stored CRLF. Keep it CRLF or a 10-line edit becomes a whole-file diff.
- Never hand-edit text between `<!-- gen:NAME -->` markers.

## 3. Commit, push, open the PR

```powershell
python tools/build-lane/ship.py pr -C .claude/worktrees/<name> -m "feat(vendor): ..." --body-file <body.md>
```

- `-C` must be a registered worktree on a non-main branch. Anything else is refused (exit 2).
- `-F FILE` instead of `-m`; `--paths P ...` to stage only some files; `--draft`; `--title`.
- Trailers come from `$SHIP_TRAILERS` and the PR footer from `$SHIP_PR_FOOTER`. The body must
  end with the harness's attribution footer.
- Re-running on a branch that already has an open PR just pushes.

## 4. Wait for CI without polling

Never write `sleep`/`until`/`Start-Sleep` loops around `gh pr checks`. Telemetry showed about
220 such polling calls in one week, each one a full model turn. Use one blocking call:

```powershell
gh pr checks <PR> --watch --fail-fast
```

Run it as a background task if the harness supports that, and you are notified when it
exits. Alternatively let `ship.py merge` do the waiting (step 6).

## 5. Adversarial review

Copilot reviews are not requested at the moment (owner decision, 2026-10-04: out of usage). Launch a **fresh, read-only reviewer
subagent** that did not write the code. Give it the PR number and the packet spec. Use the
`code-review` skill in `.github/skills/code-review/`, or the [`packet-reviewer`](../../agents/packet-reviewer.md) agent (in a harness without subagents, read its definition as the brief). Post its
verified findings as **one** PR comment. The coordinator routes the fixes, then ships them
with `ship.py pr` again.

## 6. Merge

The owner's rule is to merge on the minimum build-proving CI: `fmt`, `clippy`,
`build + nextest` and any change-specific lanes. `ship.py merge` waits for exactly those;
coverage and live-DB are not waited for.

```powershell
python tools/build-lane/ship.py merge <PR> --retire <name>              # waits up to --timeout 30m
python tools/build-lane/ship.py merge <PR> --retire <name> --no-wait    # local lane fmt/clippy/nextest already green
```

- Docs/memory-only PRs (`*.md` or `.claude/agent-memory/` only) merge at once: `ship.py merge` adds `--admin` to `gh pr merge` for them by itself.
- Code PRs are rebased (`rebase-pr`) only when GitHub reports BEHIND or DIRTY, then squash-merged.
- Serialize merges that share a baseline: merge one, then the next.
- Exit codes: 0 merged, 1 conflict or failed check, 2 refused, 3 timed out, 4 git/gh error,
  5 merged but the worktree was not retired. On 5, run step 7 by hand.

## 7. Retire the worktree the same day

`--retire` does this for you. By hand:

```powershell
pwsh tools/build-lane/rm-worktree.ps1 <name>          # one worktree
pwsh tools/build-lane/rm-worktree.ps1 --merged        # every merged, idle (30 min) one
```

It removes the target dir, the `external/` junction, the branch and the `sgw_<name>` test
database. A session that dispatched workers retires their worktrees too.

## Gotchas

- **Never run `git worktree prune`**, and never pass `--prune` to rm-worktree. On
  2026-10-03 a prune wiped about 22 live sessions' worktree registrations.
- Never delete or `--force`-retire a worktree a running worker is still using.
- If your session's cwd is inside a worktree you are retiring, move out first.
- `/release` and other ChatOps comments sent with `gh` from Git Bash get their paths mangled.
  Send them from PowerShell.
- Stale target dirs once filled the Dev Drive and stopped every lane build. Retiring is not optional.
