# How a request is attributed to a PR

> Type: reference. Audience: whoever works on `tools/token-profile/`, and readers of the per-PR stats comments.
> Contract version 4 (schema 4), 2026-10-03. Issue [#957](https://github.com/SandboxServers/Cimmeria/issues/957). Ledger: [docs/analysis/token-usage/](../../docs/analysis/token-usage/README.md).

The per-PR stats comments (TP-10) and the retro cost study (TP-11) both depend on charging each API request to the PR it was spent on. This page is the rule set. The ingest writes the result to `pr_attribution` in [`schema.sql`](schema.sql), and the transcript fields it uses are described in [`transcript-format.md`](transcript-format.md).

## Invariants

- **Every request is fully accounted for.** The weights of a request's `pr_attribution` rows sum to 1.
- **Nothing is forced onto a PR.** What no rule can place gets a row with `pr_number` NULL and method `unattributed`. Reports show the unattributed share next to every total.
- **A PR carries only its own spend (D-TP7).** Packet work that reached `main` through an integration branch, without a PR of its own, is charged to its campaign (method `campaign`, `pr_number` NULL, `campaign` set), never to the integration PR. A campaign's cost is the sum over its PRs plus those rows; see [Campaigns](#campaigns).
- **Each row says how it was placed.** `method` names the rule and `confidence` says how sure it is. A per-PR comment carries the weighted mix of both.
- **Only merged, closed and open PRs in the `prs` table are targets.** A branch with no PR is not a target until a PR for it exists.

## Rules

The first rule that places a request wins. A main-session request tries A1, then A2 and A3, then A5. A subagent's request tries A2 and A3 on its work branch, then A5's agent-wide case, then A4, then A5's turn case. Each rule says how a request's weight is split.

### Work branch

A2 and A3 place a request by the branch its work was on.

- **A main-session request:** its record's `gitBranch`.
- **A subagent's request:** the branch its own tool calls show it working on. Its record's `gitBranch` is never used. Claude Code writes the branch of the process's checkout there, which the coordinator and parallel agents move, whatever worktree the subagent is in. An in-process teammate's records carry the coordinator's cwd and branch. On 2026-10-03, for requests after a subagent's first git call, the record branch matched the subagent's own git output 4% of the time for teammates and 30% for subagents in their own worktree.

A subagent's evidence is, per tool call (schema version 4):

1. `tool_calls.branch_seen`: the one branch a git command's output names: `On branch X`, `## X...origin/X` (`git status -sb`), `[X abc1234]` (`git commit`), `Switched to branch 'X'`, `Successfully rebased and updated refs/heads/X`, and a ref update under `To <remote>` (`git push`). A ref update under `From <remote>` is a fetch and names other people's branches, so it is ignored, and so is output naming more than one branch.
2. `tool_calls.worktree`: the one `.claude/worktrees/<name>` the call's path or command names, resolved through `worktree_branches` to the branch that worktree had at the time (the observation nearest the call, within 14 days). `worktree_branches` is filled from the build lane's job log (worktree and branch of every build), `git worktree list` output seen in a tool result or taken at ingest, a git command's output paired with the worktree the same command names, and main-session requests run in a worktree. A subagent's record is not a source, for the reason above.

The request takes the branch of the subagent's latest evidence at or before it, else its first evidence after it. A subagent with no evidence has no work branch, so A2 and A3 don't place it. A request placed through evidence has method `worktree`, confidence 0.95.

### A1: trigger (coordinator sessions only)

A coordinator is a session that spawned at least one agent or received a notification from one, or a background completion of its own shell command (`sessions.is_coordinator`). When a coordinator's turn was started by an event about a specific piece of work, the turn is charged to that work's PR, whatever branch the coordinator was parked on:

- a `background_completion` or `idle_notification` from an agent, or a `teammate_message` or `agent_message` from one: where that agent's own requests went (their weighted mix, by request count), PRs and campaigns alike;
- a `background_completion` for a background shell command: the PR named in the command's description if exactly one `#NNN` appears, otherwise no match.

The agent is found by `triggers.source_ref`: an agent id (task notifications name it as the task id), or else an agent name in the same session (teammate messages name the teammate). Only the agent's requests placed by A2, A3 or the agent-wide case of A5 count, so A1 and A4 never feed each other. A shell command is found through the `tool_calls.task_id` its result carried, and its PR through `tool_calls.pr_ref`.

Method `trigger` (or `campaign` for the campaign share), confidence 0.9. This rule exists because the first quick pass on 2026-10-03 charged every coordinator turn to the docs branch the coordinator sat on, which inflated PRs such as #647 and #674.

### A2: branch

The work branch is the head branch of exactly one PR in `prs`, and the request's time falls between 24 hours before that PR was created and the moment it was merged or closed (`prs.closed_at`; an open PR's window has no end). When two PRs reused the branch name, the time window picks the one; if windows still overlap, the most recently created PR wins. Method `branch` (a main-session request) or `worktree` (a subagent's), confidence 1.0 or 0.95.

`main` and the default branch never match.

### A3: ancestry

The work branch has no PR of its own, but its commits reached `main` through another PR. The test is `git merge-base --is-ancestor <commit> <PR head commit>` for PRs merged after the request, and not more than 60 days after it; when several qualify, the earliest merged wins.

- **On the PR's first-parent chain: the PR's.** The branch is the PR's own line of work under another name (a local branch pushed under a different one). Method `ancestry`, confidence 0.8.
- **Merged in from the side: the campaign's (D-TP7).** The commit reached the PR's head through a merge, so the branch is a packet merged into an integration branch. Its requests are the packet's spend, not the integration PR's: they get a `campaign` row (confidence 0.8), named by [Campaigns](#campaigns) from the packet branch, else the integration PR's campaign, else `pr-<N>` after the integration PR. An integration PR with no campaign of its own is tagged with that one. A packet merged by fast-forward lands on the first-parent chain and so still counts as the PR's.
- **The PR's head commit, not its merge commit.** `main` takes PRs as squash merges, and a squash commit has none of the packet's commits as ancestors, so testing the merge commit would almost never match.
- **Work `main` already had is not the PR's.** A commit that is also an ancestor of the merge commit's first parent (`main` just before the merge) is skipped for that PR. Otherwise a branch with no commits of its own, whose head is a `main` commit, would be charged to the next PR that merged `main`.

The ingest answers these questions from one `git rev-list --parents --all` of the checkout, plus the PR head and merge commits fed to it on stdin, rather than one `git merge-base` per question.

The commit comes from `branch_heads`, never from resolving the branch name at report time: `rm-worktree.sh` deletes a merged packet branch, so a backfill run later may find no ref. `branch_heads` is filled from three sources, and the latest commit observed for the branch within 14 days after the request is used, since the work a request did is committed after it:

1. `ref-snapshot`: every ingest run records the head of every local and remote-tracking branch. Its `observed_at` is the head commit's committer date, not the run time: the commit was at the head from then until now, and the earlier date lets a backfill place requests made long before the run.
2. `lane-log`: the build lane's job log records the worktree and commit of every build; joined to the request through its worktree and time.
3. `merge-subject`: a merge commit on `main` or an integration branch whose subject names the branch (`Merge branch '<name>'`) supplies its second parent.

A request whose branch has no commit in any source is not placed by A3.

### A4: parent session (subagents only)

A subagent that A2, A3 and A5's agent-wide case don't place inherits the attribution of the parent turn that spawned it: the main-session request that made the `Agent` call (found through the call's `tool_calls.task_id`). The parent's PR rows become `parent-session` rows with the same weights; its campaign and unattributed shares stay as they are. A parent with no PR or campaign rows doesn't place the subagent. Method `parent-session`, confidence 0.7.

### A5: pr-link

Two cases, both confidence 0.6.

- **Agent-wide (subagents).** A subagent whose own calls created exactly one PR (`gh pr create`, `tool_calls.pr_verb = 'create'`) is charged to it for every request its work branch doesn't place, in any turn: a worker's first turns usually come before the turn that opens its PR. Method `pr-link`.
- **Turn.** A request in a turn that created a PR, wrote a `pr-link` record for one, or ran another `gh pr` command naming one (`merge`, `checks`, `view`, `diff`, `comment`, `edit`, `review`, `ready`, `close`), is charged to it. When the turn created a PR, only the PRs it created count: a worker that read another PR for reference before opening its own is charged to its own. Several PRs share the turn equally, method `split`; one PR gets method `pr-link`.

A turn is a trigger. A `pr-link` record belongs to the main-session turn open at its timestamp. `gh pr create` names no number of its own (a number in its title or body is not the PR's), so its number comes from the `/pull/N` in its result. A background command doesn't count here: its PR is only known from its description, and A1 charges its completion.

### A6: unattributed

Everything else: `pr_number` NULL, method `unattributed`, confidence 0. This includes research sessions that never produced a PR, playtest log mining, coordinator turns triggered by humans on `main` with no PR activity, and subagents that never touched git or a PR whose parent turn is unattributed.

## Campaigns

A campaign is a ledger folder under `docs/analysis/<campaign>/`. [`campaigns.json`](campaigns.json) tags PRs and branches with one, and the ingest writes `prs.campaign` (schema version 4). A PR's campaign is the first match of:

1. its head branch: an exact branch in the campaign's `branches`, else the longest matching entry in its `prefixes` (`npcai/`, `craft/`, `harset/`, ...);
2. a tracking issue its title names (`issues`), for campaigns whose branches carry no common prefix (`(#957)`);
3. its base branch, matched the same way, or the campaign of the PR whose head that base is: a packet PR into an integration branch belongs to the integration PR's campaign.

An integration PR (one other PRs target, or one A3 finds packets merged into) that no rule names forms its own campaign, `pr-<N>`. Branch names never reach a report, so a campaign is never named after one.

Campaign cost is the sum across the campaign's PRs, each carrying only its own spend, plus its `campaign` rows: packet work with no PR. The report's "Cost per campaign" section shows both parts. `campaigns.json` is data the data supports: on 2026-10-03, 207 of 804 PRs matched by branch prefix, exact branch or tracking issue. A campaign added to it needs a ledger folder of the same name; a test checks that.

## What the per-PR comment reports

A PR's totals are the weighted sums of its rows. Its machine block carries:

- `attribution.method`: the method with the largest weighted share, or `split` when none has more than half;
- `attribution.confidence`: the weighted mean confidence;
- `attribution.unattributed_share`: unattributed spend in the PR's sessions over its window, divided by that unattributed spend plus the PR's own total. It is always between 0 and 1; a high value means the PR's own number is probably low;
- `campaign`: the PR's campaign, or null;
- `cost_state`: Claude Code's own total cannot be split by PR (D-TP6), so `usd` is always null and `reason` says why. `sessions_gap_share` is how far the profiler fell short of the cost-state totals of the sessions the PR's spend came from, and `covered_share` how much of the PR's spend those cost-states cover.

## Validating the rules

`python tools/token-profile/validate --db <db> [--labels <labels.csv>] [--truth-db <db>]` scores the attribution against ground truth, weighted by estimated USD and by request count: precision (spend on the right PR over spend on any PR), recall (spend on the right PR over all labelled spend), and the wrong, campaign and unattributed shares, with the wrong share per method. A rule that misplaces more than 10% of the labelled spend is fixed before the backfill.

There are two ground-truth sets, scored apart:

- **`labels`**: a local CSV of `name,date,pr` rows, an agent's teammate name and the UTC day of its first request. The worker naming convention gives an independent set: a worker named after its packet (`cr06`, `bv07`, `org-05`, `tp10`) whose packet code matches exactly one PR head branch opened within four days (`craft/cr06-*`), with `-resume` and `-rebase` workers labelled as the packet's. Workers that opened more than one PR are left out, since one label can't name both. No rule reads agent names, so this set is independent of the rules.
- **`authors`**: a subagent, or a main session run in one worktree, that created exactly one PR is taken to have spent all its requests on it. A5's agent-wide case uses the same fact, so this set measures misplacement better than coverage.

`--truth-db` derives the ground truth from another database, so an older database is scored on the same requests. The output holds shares and totals only; the labels file stays local. The 2026-10-03 results are in the [TP-05b worknote](../../docs/analysis/token-usage/worknotes/TP-05b.md).
