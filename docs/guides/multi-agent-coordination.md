# Working in Parallel: Multi-Agent Coordination

> **Audience**: Anyone running an agent session (Claude Code, OpenCode, or a
> human contributor) against the shared repo host.
> **Type**: How-to. Applies to every session on the shared machine.
> **Tools**: `tools/agent-status.sh`, `tools/check-issue.sh`, `tools/claim.sh`,
> `tools/unclaim.sh`.

This repo is developed by **multiple agent sessions on one host**, all
pointing at the same checkout. The coordination failure modes are real and
have all happened:

- an agent picked an issue another agent was already working (found only by
  spotting the worktree name),
- an agent picked an issue that a merge had **already closed** mid-session,
- commits landed **directly on the main checkout's `main`**, leaving orphaned
  work and a main that drifted 4 ahead / 6 behind `origin/main`,
- a worktree was created **outside** the `.claude/worktrees/` convention.

This page makes the coordination state *visible* and the claim lifecycle
*mechanical*, so none of those require archaeology.

## The two hard rules (recap)

1. **Never work in the main checkout** — no `checkout`, `pull`, `reset`,
   `stash`, or `clean` on `/home/derek/code_stuff/Cimmeria`. It is the
   integration workspace and may hold another agent's uncommitted edits.
   All task work happens in a worktree under `.claude/worktrees/<slug>`.
   The one safe write is creating a branch ref to *rescue* work before the
   operator resets main (see [Main hygiene](#main-hygiene)).
2. **Never run cargo concurrently with another agent** — check
   `tools/agent-status.sh` (or `ps aux | grep -E "cargo|rustc"`) first, cap
   with `CARGO_BUILD_JOBS=4`, and wait if another session is building.

## The claim lifecycle

```text
check-issue → claim → work in worktree → PR → merge → unclaim + cleanup
```

### 1. Check the issue is clear

```bash
tools/check-issue.sh 616
```

Refuses (exit 1) when the issue is **closed on GitHub**, **already claimed**
in the registry, has a **worktree** under `.claude/worktrees/`, or has an
**open PR** referencing it. It errs on the side of refusing — investigate
and remove the stale claim/worktree before forced-starting.

### 2. Claim it and create the worktree

```bash
tools/claim.sh 616 fix movewaypoint-broadcast "cell/aoi, npc reposition"
#                    │    └─ slug: worktree dir + branch subject
#                    └─────── type: fix|feat|docs|chore|test|refactor
```

One deterministic step:

- refuses if the issue is already claimed or its worktree exists,
- creates `.claude/worktrees/<slug>` on branch `<type>/<issue>-<slug>` from
  `origin/main` (never from a possibly-polluted local `main`),
- symlinks `external/` into the worktree,
- appends a row to the shared registry: `<issue>  <slug>  <agent>  <branch>  <note>  <ts>`.

Then work in the worktree as usual:

```bash
cd .claude/worktrees/<slug> && git status -sb
```

### 3. Open the PR

Open the PR as normal (see the PR checklist in `CONTRIBUTING.md`). When you
do, optionally record the PR number in the claim row's `note` column so
`agent-status.sh` output correlates claims ↔ PRs (the registry is a plain
TSV — edit it directly):

```text
issue  slug        agent  branch                     note         ts
616    issue-616   derek  fix/616-movewaypoint-...   pr=707       2026-09-19T...
```

### 4. Merge → unclaim + cleanup

When the PR merges (a maintainer merges, never an agent), release the claim
and remove the worktree/branch, run from the **main checkout**:

```bash
tools/unclaim.sh 616
git worktree remove --force .claude/worktrees/<slug>
git branch -D <branch>
git push origin --delete <branch>      # only if it still exists remotely
```

## See who is doing what, always

```bash
tools/agent-status.sh
```

One command prints: the claim registry, every worktree (flagging any
outside the convention), open PRs, **main↔origin drift plus orphaned local
commits**, uncommitted files in the main checkout, running cargo/rustc, and
cross-checks (claims without worktrees, worktrees without claims).

## Main hygiene

The main checkout's `main` is **not** a workspace for commits. Symptoms of a
thrashed main `agent-status.sh` surfaces:

- `main ... [ahead N, behind M]` with N > 0 → orphaned local commits;
- dirty/untracked rows in the main checkout section.

**If your own work ever ends up committed on the main checkout's `main`**
(recoverable, not catastrophic):

```bash
# 1. Rescue the orphaned commits onto a branch so a reset can't lose them:
git branch rescue/<your-slug> HEAD      # in the main checkout — allowed: ref write only

# 2. Recover the real work: prefer rebasing the commits onto your worktree
#    branch, or copy the relevant portions across.

# 3. The operator (human) then re-syncs main:
git checkout main && git reset --hard origin/main
```

Branch from `origin/main`, never from the local `main` pointer, until the
above workflow is routine. `tools/claim.sh` already does this for you.

## Registry details

- File: `.claude/worktrees/claims.tsv` (tab-separated, 6 columns:
  `issue  slug  agent  branch  note  ts`).
- It sits under `.claude/worktrees/`, which is **gitignored** — it is
  host-local shared state, not a committable artifact. A fresh checkout
  starts with no claims file; the tools create it on first use.
- Treat the registry as the source of truth for "who is working on what".
  If you see a worktree with no claim, add the claim; if you see a claim
  with no worktree, ask before deleting either.
