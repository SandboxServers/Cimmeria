# How a request is attributed to a PR

> Type: reference. Audience: whoever works on `tools/token-profile/`, and readers of the per-PR stats comments.
> Contract version 1, 2026-10-03. Issue [#957](https://github.com/SandboxServers/Cimmeria/issues/957). Ledger: [docs/analysis/token-usage/](../../docs/analysis/token-usage/README.md).

The per-PR stats comments (TP-10) and the retro cost study (TP-11) both depend on charging each API request to the PR it was spent on. This page is the rule set. The ingest writes the result to `pr_attribution` in [`schema.sql`](schema.sql), and the transcript fields it uses are described in [`transcript-format.md`](transcript-format.md).

## Invariants

- **Every request is fully accounted for.** The weights of a request's `pr_attribution` rows sum to 1.
- **Nothing is forced onto a PR.** What no rule can place gets a row with `pr_number` NULL and method `unattributed`. Reports show the unattributed share next to every total.
- **Each row says how it was placed.** `method` names the rule and `confidence` says how sure it is. A per-PR comment carries the weighted mix of both.
- **Only merged, closed and open PRs in the `prs` table are targets.** A branch with no PR is not a target until a PR for it exists.

## Rules

The first rule that places a request wins. Each rule says how a request's weight is split.

### A1: trigger (coordinator sessions only)

A coordinator is a session that spawned at least one agent or received a notification from one (`sessions.is_coordinator`). When a coordinator's turn was started by an event about a specific piece of work, the turn is charged to that work's PR, whatever branch the coordinator was parked on:

- a `background_completion` or `idle_notification` from an agent, or a `teammate_message` or `agent_message` from one: the PR that agent's own requests are attributed to (their weighted mix, by request count);
- a `background_completion` for a background shell command: the PR named in the command's description if exactly one `#NNN` appears, otherwise no match.

Method `trigger`, confidence 0.9. This rule exists because the first quick pass on 2026-10-03 charged every coordinator turn to the docs branch the coordinator sat on, which inflated PRs such as #647 and #674.

### A2: branch

The request's `gitBranch` (or the branch of the worktree in its `cwd`) is the head branch of exactly one PR in `prs`, and the request's time falls between 24 hours before that PR was created and the moment it was merged or closed. When two PRs reused the branch name, the time window picks the one. Method `branch`, confidence 1.0.

`main` and the default branch never match.

### A3: ancestry

The request's branch has no PR of its own, but its commits reached `main` through another PR: a packet branch merged into an integration branch whose PR merged. The test is `git merge-base --is-ancestor <commit> <PR merge commit>` for PRs merged after the request; when several qualify, the earliest merged wins. Method `ancestry`, confidence 0.8.

The commit comes from `branch_heads`, never from resolving the branch name at report time: `rm-worktree.sh` deletes a merged packet branch, so a backfill run later may find no ref. `branch_heads` is filled from three sources, and the latest commit observed for the branch within 14 days after the request is used, since the work a request did is committed after it:

1. `ref-snapshot`: every ingest run records the head of every local and remote-tracking branch.
2. `lane-log`: the build lane's job log records the worktree and commit of every build; joined to the request through its worktree and time.
3. `merge-subject`: a merge commit on `main` or an integration branch whose subject names the branch (`Merge branch '<name>'`) supplies its second parent.

A request whose branch has no commit in any source is not placed by A3.

### A4: parent session (subagents only)

A subagent whose own requests match none of A2 and A3 inherits the attribution of the parent turn that spawned it: the main-session request that made the `Agent` call. Method `parent-session`, confidence 0.7.

### A5: pr-link

A request in a turn that wrote a `pr-link` record for PR N, or whose tool calls ran `gh pr create`, `gh pr merge`, `gh pr checks` or `gh pr view` naming N, is charged to N. Several distinct PRs in one turn share it equally, method `split`; one PR gets method `pr-link`. Confidence 0.6.

### A6: unattributed

Everything else: `pr_number` NULL, method `unattributed`, confidence 0. This includes research sessions that never produced a PR, playtest log mining, and coordinator turns triggered by humans on `main` with no PR activity.

## What the per-PR comment reports

A PR's totals are the weighted sums of its rows. Its machine block carries:

- `attribution.method`: the method with the largest weighted share, or `split` when none has more than half;
- `attribution.confidence`: the weighted mean confidence;
- `attribution.unattributed_share`: unattributed spend in the PR's sessions over its window, divided by that unattributed spend plus the PR's own total. It is always between 0 and 1; a high value means the PR's own number is probably low.

## Validating the rules

TP-05 checks the rules before the backfill is posted. It hand-labels a sample of at least 30 PRs (workers in worktrees, packets merged through integration branches, coordinator-only PRs and docs PRs), compares their attribution with the labels, and records the agreement in the ledger. A rule that misplaces more than 10% of the labelled spend is fixed before the backfill.
