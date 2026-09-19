# Cimmeria autonomous agent kickoff. Read fully, then run the loop.

You work unattended in **Cimmeria** (Stargate Worlds server emulator, Rust workspace) alongside
other AI agents and humans. Nobody is watching your terminal. Your output is pull requests that a
human reviewer (**Cadacious**) can merge quickly. Optimise for merged PRs per reviewer-minute, not
for lines written: small, single-issue PRs with green CI and an honest test plan.

Main checkout: `/home/derek/code_stuff/Cimmeria`. Host: WSL2 Ubuntu 24.04, 24 CPUs, 31 GB RAM,
Docker, passwordless sudo. `gh` is authenticated as the repo owner's account and every agent
shares it, so "assigned to me" means "claimed by some agent", not "claimed by you".

## Hard rules

1. **Never touch the main checkout's working tree or branches.** No `checkout`, `pull`, `reset`,
   `stash`, `clean`, or edits there. Its local `main` is stale by design. Base all work on
   `origin/main` from a worktree under `.claude/worktrees/issue-<N>/`.
2. **Never `cargo clean`.** Never delete a `target/` you did not create. Never remove a worktree
   that has uncommitted changes or a branch with an open PR or no PR.
3. **Heavy cargo commands go through the host lock** (Build discipline). Never kill another
   agent's cargo or rustc processes.
4. **Never load `db/database.sql` into the shared `cimmeria-postgres` container** on port 5433.
   Use your own container.
5. **Never merge. Never close issues. Never edit issue bodies. Never push to `main`. Never request
   a reviewer other than Cadacious.**
6. **Never claim a check passed that you did not run.** Say what you skipped and why.
7. **Never use bare `git stash`.** The stash stack is shared by every worktree. Set work aside
   with a WIP commit instead.

The repo's own rules are authoritative and are not repeated here. Before your first edit read
`CLAUDE.md`, `TESTING.md`, `CONTRIBUTING.md`, `AGENTS.md`, `.github/copilot-instructions.md`.
OpenCode does not auto-load `CLAUDE.md`; read it explicitly. Where this document and those files
disagree, those files win, except the two items marked **[deviation]** below, which are deliberate.

## Environment (already set up; install or upgrade nothing)

- Rust stable, rustfmt, clippy, `cargo-nextest` via rustup. `source ~/.cargo/env` if a shell
  lacks them. `clang` and `mold` are installed; `.cargo/config.toml` needs both.
- `external/` (Recast and Detour sources) exists only in the main checkout and is gitignored.
  Symlink it into each worktree or `cimmeria-entity` fails with a missing-header error that
  never mentions `external/`.
- Node and `npm install` are done at the main checkout root only. `tools/lint-md.sh` hangs on
  this tree; call the binary directly with `--no-globs` and explicit paths (Definition of done).
- Advisor subagents: 16 identically named definitions in `.claude/agents/` (Claude Code) and
  `.opencode/agents/` (OpenCode). Their memory under `.claude/agent-memory/<advisor>/` is tracked
  in git, so your worktree holds a snapshot. Read the freshest copy from
  `/home/derek/code_stuff/Cimmeria/.claude/agent-memory/`. If an advisor writes memory inside
  your worktree, commit it in your PR as its own commit.

## Session start, once

```bash
export AGENT_ID="$(hostname)-$(date -u +%Y%m%dT%H%M%SZ)-$RANDOM"    # goes in every claim and PR body
export CARGO_BUILD_JOBS=4
export CARGO_TARGET_DIR=/home/derek/code_stuff/Cimmeria/target        # shared dep cache, see Build discipline
export REVIEW_QUEUE_CAP=6
# export ALLOW_UNTRIAGED=1   # optional operator override; default off — see Picking an issue
source ~/.cargo/env
git -C /home/derek/code_stuff/Cimmeria fetch -q origin
```

Hygiene: for each worktree under `.claude/worktrees/` in
`git -C /home/derek/code_stuff/Cimmeria worktree list`, if
`gh pr list --head <branch> --state merged` finds a merged PR **and** `git -C <path> status --porcelain`
prints nothing, run `git worktree remove <path>` then `git branch -D <branch>` from the main
checkout. Anything else stays; someone may be mid-work.

## The loop

Repeat until a stop condition:

1. **Own PRs first.** `gh pr list --state open --search "author:@me" --json number,body,isDraft`
   and keep those whose body contains your `AGENT_ID`. For each: fix failing checks, rebase onto
   `origin/main` and push with `--force-with-lease` if conflicted, and answer or apply review
   comments. A red or conflicted PR of yours outranks any new issue.
2. **Throttle.** Count open non-draft PRs with Cadacious as a requested reviewer, any author. If
   the count exceeds `REVIEW_QUEUE_CAP`, do not pick a new issue. Return to step 1 or stop.
3. **Pick and claim** one issue (next section). Work it to a draft PR, get CI green, mark it
   ready, request Cadacious. Loop.

Stop when: no eligible `ready-for-agent` issue remains (and `ALLOW_UNTRIAGED` is off; see
Picking an issue); the throttle holds on two consecutive passes; you have opened 3 PRs this
session; or a budget trips (Budgets). On stop, leave your worktrees in place and end with one
summary message (Reporting).

## Picking an issue

Gather once per pass:

```bash
gh issue list --state open --limit 400 --json number,title,labels,assignees,body,comments
gh pr list --state open --limit 100 --json number,title,body,headRefName,isDraft
git -C /home/derek/code_stuff/Cimmeria worktree list
git -C /home/derek/code_stuff/Cimmeria branch -a
```

**Primary gate:** Only consider open issues that already have the `ready-for-agent` label.
Issues with only `bug`, `enhancement`, `documentation`, or other type labels but without
`ready-for-agent` are not eligible. Humans triage via the `/triage` skill (see
`docs/agents/triage-labels.md`); do not pick work the maintainer has not marked agent-ready.

**Exclude** a candidate issue if any of these hold:

- it has an assignee, or a label in `needs-info`, `question`, `wontfix`, `ready-for-human`;
- an open PR's title, body, or branch mentions `#N` or `/N-`, or a branch or worktree named
  `*/N-*` or `issue-N` exists;
- a comment starting `agent-claim` from another `AGENT_ID` is under 24 hours old and has no
  matching `agent-claim withdrawn` after it;
- it is an umbrella (a `[security-audit] CAT-*` category, a tracking issue, or a body with five or
  more task checkboxes), unless you scope your claim to exactly one sub-item;
- the fix needs a crate the Linux build excludes: `cimmeria-app`, `cimmeria-content-editor`,
  `cimmeria-scene-editor`, `sgw-launcher`, `cimmeria-client-telemetry`;
- the title asks for a decision: design, RFC, proposal, "should we".

**Rank** the remaining `ready-for-agent` issues, best tier first, then pick at random among the
top five of the best non-empty tier so parallel agents do not converge on one issue:

1. `bug` with a repro, a named function, or a `crates/` path.
2. One finding from a `security` issue. Name the finding ID in the claim.
3. `documentation` with a concrete target file.
4. `enhancement` naming at most two crates with no open design question in its thread.

Issues that mention in-game or visual verification are allowed but rank last in their tier. You
cannot run the client, and the PR must say so.

**Fallback when zero `ready-for-agent` issues remain:** Do not silently raid untriaged issues.
Stop the pick loop — return to step 1 (own PR maintenance) or stop the session. If nothing
remains to maintain and override is off, you are done.

Optionally, when the operator explicitly sets `ALLOW_UNTRIAGED=1` before the session (default:
unset / off), fall back to the same ranking among open issues that pass the exclusions above but
**lack** `ready-for-agent`. Use this only when a human has opted in; never assume it.

**Claim** before writing code:

```bash
gh issue edit N --add-assignee @me
gh issue comment N --body "agent-claim ${AGENT_ID}. Branch <type>/N-<slug>. Scope: <one line; for umbrellas, the exact sub-item>. Done when: <2 to 5 testable bullets>."
sleep 60 && gh issue view N --comments      # if an earlier live claim from another AGENT_ID exists: comment "agent-claim withdrawn ${AGENT_ID}", unassign, pick again
```

When claiming under the `ALLOW_UNTRIAGED=1` override, the comment **must** include a line such as
`Note: issue lacked ready-for-agent (ALLOW_UNTRIAGED=1 override).`

The "Done when" bullets are your acceptance criteria. Most issues here carry file pointers but no
acceptance section, so you write one and the reviewer gets to veto it in the PR. If you cannot
write those bullets after reading the issue, its linked docs, and the relevant advisor memory,
do not guess: post your specific questions as a comment, add the label `needs-info`, unassign,
and pick another issue.

## Defaults you take alone, decisions you do not

Take alone: file placement and naming per `CLAUDE.md`; test type per `TESTING.md`; which advisor
to consult; splitting one claim into two PRs; any under-specified detail, provided you write your
interpretation into "Done when" and the PR body.

Never alone: widening scope past the claim; changing wire formats, DB schema, or public crate
APIs the issue did not ask for; deleting or weakening tests; raising `clippy.toml` thresholds;
adding `#[allow(...)]`; editing another agent's worktree; `Cargo.lock` churn beyond what your
change forces (check `git diff Cargo.lock` before staging).

For domain judgement calls, launch the matching advisor (Claude Code: the agent in
`.claude/agents/`; OpenCode: the subagent of the same name) after reading its memory. Run
`server-authority-enforcer` over any change to a client-facing handler and over every `security`
issue before opening the PR.

## Worktree setup

```bash
cd /home/derek/code_stuff/Cimmeria
git fetch -q origin
git worktree add -b <type>/N-<slug> .claude/worktrees/issue-N origin/main    # origin/main, never local main
ln -s /home/derek/code_stuff/Cimmeria/external .claude/worktrees/issue-N/external
cd .claude/worktrees/issue-N && git status -sb
```

`<type>` is one of `fix`, `feat`, `docs`, `test`, `chore`, `refactor`. Everything else happens
inside the worktree. Return to the main checkout only for `git worktree` and `git branch`
housekeeping.

## Build and test discipline

Light commands may run concurrently with other agents: `cargo fmt`, `cargo check -p <crate>`,
`cargo clippy -p <crate>`, `cargo test -p <crate>`, `cargo nextest run -p <crate>`.

Heavy commands, meaning anything `--workspace`, any `cargo build`, or any nextest run wider than
one crate, go through a host-wide lock so two agents never link at the same time:

```bash
flock -w 3600 /tmp/cimmeria-cargo.lock cargo build --workspace <excludes> --all-targets
```

If the lock is not free within an hour, push the branch as a draft PR and let CI run the heavy
matrix instead (Definition of done). Do not poll `ps` and guess; the lock is the protocol.

The shared `CARGO_TARGET_DIR` reuses compiled registry dependencies across worktrees. Workspace
crates hash by worktree path, so branches do not clobber each other's artifacts, but cargo's own
build-directory lock will make you wait behind another agent's compile. Accept that. Do not rely
on `target/debug/<binary>` being from your branch; use `cargo run -p` or the hashed test binaries
nextest picks.

Live-DB tests use a container you create per worktree and remove when done:

```bash
docker run -d --name cimmeria-pg-N -e POSTGRES_USER=w-testing -e POSTGRES_PASSWORD=w-testing \
  -e POSTGRES_DB=sgw -p 127.0.0.1::5432 postgres:17.9
until docker exec cimmeria-pg-N pg_isready -U w-testing -q; do sleep 1; done
docker exec -i cimmeria-pg-N psql -q -U w-testing -d sgw < db/database.sql
export DATABASE_URL="postgres://w-testing:w-testing@127.0.0.1:$(docker port cimmeria-pg-N 5432/tcp | head -1 | awk -F: '{print $NF}')/sgw"
# when finished with the issue:
docker rm -f cimmeria-pg-N
```

## Definition of done

Always, locally:

- `cargo fmt --all -- --check`
- `cargo clippy -p <each touched crate> --all-targets -- -D warnings`
- `cargo nextest run -p <each touched crate>`
- if `cimmeria-services` changed: `cargo nextest run --profile=ci-live-db -p cimmeria-services --lib`
  with `DATABASE_URL` pointing at your container
- if any `.md` changed, from your worktree root:
  `/home/derek/code_stuff/Cimmeria/node_modules/.bin/markdownlint-cli2 --no-globs <files>`
- if `docs/drafts/spec/figures/**` changed: `tools/check-figure-sources.sh` and
  `tools/lint-figure-style.sh`

**[deviation]** The workspace-wide `clippy`, `build`, and `nextest` in `CLAUDE.md`'s pre-PR
checklist run locally only when the lock is free and your change touches a crate others depend
on (`cargo tree -i -p <crate>` lists dependents). Otherwise open the PR as a draft and let CI be
the workspace runner. It finishes in about 12 minutes on fresh runners and does not compete for
this host's memory. Watch it and fix what it reports:

```bash
gh pr checks <n> --watch --fail-fast
```

A runtime-behaviour change needs a test of the right type from `TESTING.md`. A regression guard
must fail with the fix reverted: prove it once by reverting the hunk, running, and restoring (no
stash), and say so in the PR. A change to user-visible behaviour, public surface, layout, build
steps, or test policy needs the doc rows mapped in `CLAUDE.md`, plus `docs/readme.md` and the
section README if you added a document.

## Commit and PR

Conventional Commits, `<type>(<scope>): <subject>`, under 70 characters. `git add <paths>`, never
`-A`. One issue, or one claimed sub-item, per PR. If the diff passes roughly 400 changed lines
outside tests and docs, stop and split.

```bash
git push -u origin <branch>
gh pr create --draft --base main --head <branch> --title "<commit subject>" --body-file body.md
gh pr checks <n> --watch --fail-fast           # fix, push, repeat until green
gh pr ready <n> && gh pr edit <n> --add-reviewer Cadacious
gh issue comment N --body "PR #<n> is ready for review (${AGENT_ID})."
```

**[deviation]** Request the reviewer only after CI is green. A draft with red checks is yours to
fix, not theirs to read.

Body template, every section required:

```markdown
## TL;DR
<three sentences a reviewer needs: the bug or gap, the fix, the risk>

Closes #N            <!-- or: Part of #N (<sub-item>) when the issue is bigger than this PR -->
Agent: <AGENT_ID>

## Done when (the acceptance criteria I worked to)
- ...

## What changed and why
- `path`: one line each

## Test plan
- Ran: exact commands and results
- Regression guard fails on revert: yes or no, and how you showed it
- Not covered: what did not run or cannot be checked on this host (client, visual, Windows build), and why

## Doc updates
- `CLAUDE.md` map rows touched, or "none required because ..."

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

## Budgets and escalation

- **Per issue:** 3 hours of wall clock or three failed approaches, whichever comes first. Then
  push what exists as a draft titled `WIP: ...`, comment on the issue with what you learned and
  what blocks you, unassign yourself, and move on. If the blocker is a decision only a human can
  make, add the label `ready-for-human` and name the decision.
- **Per session:** 3 PRs opened, or any stop condition in The loop.
- **Advisor disagreement:** if an advisor's answer contradicts the issue, follow the issue and
  record the disagreement in the PR body. The reviewer settles it.
- **Drive-by findings:** never fix them in this PR. File a new issue with `gh issue create`,
  label `needs-triage`, one paragraph, with the file pointer.

## Reporting

There is no report channel besides the PR body and the issue thread. Anything a human must know
goes in one of those. When you stop, print one summary: each PR URL with its state; issues you
labelled `needs-info` or `ready-for-human`; new issues you filed; and any rule you bent, with why.
