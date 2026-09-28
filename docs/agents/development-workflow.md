# Development Workflow for AI-Assisted Work

> **Last updated**: 2026-09-27
> **Audience**: Contributors doing AI-assisted work, and their agents
> **Type**: How-to

How a change moves from ticket to merged PR in this repo when an AI harness is doing the work. It is harness-neutral: the steps apply whether you drive them through Claude Code subagents, another tool's equivalents, or by hand.

The repo already ships the pieces. Anyone who clones it with Claude Code gets them automatically:

- [`CLAUDE.md`](../../CLAUDE.md): build rules, pre-PR checklist, test policy, doc-update map, file organization.
- [`AGENTS.md`](../../AGENTS.md) and [`.github/copilot-instructions.md`](../../.github/copilot-instructions.md): the same policy for other harnesses and for review bots.
- [`.claude/agents/`](../../.claude/agents/): sixteen domain subagents (roster below).
- `.claude/agent-memory/<agent>/`: what those agents learned on earlier runs, and `.claude/agent-memory/main-session/`: what top-level sessions learned. Committed on purpose; see [Project memory](#project-memory).
- [`TESTING.md`](../../TESTING.md): the test-type picker and the review gotchas.
- [`rules-and-gotchas.md`](rules-and-gotchas.md): decisions already made and traps already hit.

## The pipeline

1. **Reconcile the premise.** Read the ticket, then check its claims against `docs/` as described in [`domain.md`](domain.md). Stop here if the ticket and the docs disagree and you cannot verify which is right.
2. **Consult the domain advisor** for the area (roster below). Advisors know the failure modes of their system and what the client expects. Ask before designing, not after.
3. **Classify client impact.** "Free" (server-authoritative, reuses messages the client already speaks) or "needs a client patch". Do not scope a client-patch change casually. See [`rules-and-gotchas.md`](rules-and-gotchas.md).
4. **Pick the test type first**, using the picker in `TESTING.md`. Write the guard so it reproduces the bug shape.
5. **Implement** with `rust-gameserver-dev` (or directly), iterating with `cargo check -p <crate>` on the crate you changed, through the build lane (see [Builds, worktrees and test databases](#builds-worktrees-and-test-databases)). The review rules for code under `crates/services/` are in [`.github/instructions/rust-services.instructions.md`](../../.github/instructions/rust-services.instructions.md); content chains have their own in [`content-chains.instructions.md`](../../.github/instructions/content-chains.instructions.md).
6. **Ask "what if the client lies?"** Run `server-authority-enforcer` over any handler that takes client-supplied data into server state.
7. **Prove the guard.** First commit your work (a WIP commit is fine). Then undo only the fix by editing it out of the one file, run the test, and confirm it fails. Restore with `git checkout HEAD -- <that one file>`. Never restore with `git checkout .`, `git reset --hard`, or `git stash`: other sessions may share the checkout and the stash. `testing-validation-engineer` does this review.
8. **Update the docs** named by the `CLAUDE.md` doc-update map, preferably with `documentation-writer`, and keep `docs/readme.md` and the section `README.md` indexes in sync. Leave generated blocks and the status docs alone; see [Shared docs without conflicts](#shared-docs-without-conflicts).
9. **Run the pre-PR checklist** from `CLAUDE.md`. `rust-toolchain.toml` pins the toolchain CI uses, so your clippy run is CI's.
10. **Open the PR** with the template filled in, including what you could not test.
11. **Commit project memory.** If a subagent or the main session wrote findings under `.claude/agent-memory/`, stage them with the change. Check the root checkout too: subagents that worked in a worktree have been seen writing their memory files into the root checkout's `.claude/agent-memory/` instead (observed through 2026-09). Rules for what belongs there: [Project memory](#project-memory).

Fix warranted adjacent problems in the same pass: a file your change pushed over the 500-line cap, another instance of the bug a reviewer flagged, a doc your change made stale. Say what you fixed beyond the ask and why. Leave risky or judgment-heavy changes as a flagged follow-up.

**Unattended sessions are stricter.** An agent running without a human at the terminal follows [`docs/guides/autonomous-agent-kickoff.md`](../guides/autonomous-agent-kickoff.md): it picks only `ready-for-agent` issues, never fixes drive-by findings in the same PR (it files an issue instead), and stops rather than implementing an issue whose premise the docs contradict.

## Agent roster

| Area | Agent |
|---|---|
| Implementing server systems, wire handlers, persistence code | `rust-gameserver-dev` |
| BigWorld conventions, cell/base split, Mercury, what the client assumes | `bigworld-engine-advisor` |
| Who-sees-whom, witness lists, appearance rebroadcast, enter/leave AoI | `aoi-witness-broadcast` |
| Position updates, teleports, ring transports, speed validation | `movement-teleport-advisor` |
| Damage, abilities, cooldowns, effects, threat, combat state | `combat-systems-advisor` |
| Mob AI, spawners, leashing, patrols, ability selection | `npc-ai-spawn-advisor` |
| Inventory, bandolier, loot, vendors, item use | `items-systems-advisor` |
| Missions, objectives, content-engine chains, dialog | `mission-systems-advisor` |
| Guilds, mail, contacts, trade, duels, black market | `social-systems-engineer` |
| SmartFox minigame protocol and results | `minigame-systems-advisor` |
| Schema, queries, seeds, pooling, transactions | `database-persistence` |
| Auth, sessions, encryption, login flow | `network-security-auth` |
| Trust-boundary review of any handler | `server-authority-enforcer` |
| Test strategy, guard validity, suite audits | `testing-validation-engineer` |
| Docs of any kind (Diátaxis-aware) | `documentation-writer` |
| Ghidra work against `SGW.exe`, intent reconstruction | `game-archaeology-specialist` |

Definitions and trigger descriptions are in [`.claude/agents/`](../../.claude/agents/). If your harness has no subagents, read the agent's definition file and its `MEMORY.md` as briefing material.

## Running agents

- **Parallel writers need isolated worktrees.** Two implementation agents in one checkout race on git state: one agent's `git checkout` reverts the other's edits. Give each its own worktree under `.claude/worktrees/` (ignored by git), with its own test database. Read-only agents do not need one. How to create one that builds is in the next section.
- **Do not switch branches in a checkout someone else is using.** Do integration work from a dedicated worktree.
- **Run the advisors on the model they were defined for.** They are tuned for judgment-heavy review; routing them to a smaller model to save tokens costs more in rework than it saves.
- **Agent memory is a deliverable.** Files under `.claude/agent-memory/` are committed alongside the work that produced them. `.claude/settings.local.json` and `.mcp.json` are per-machine and are not. (`settings.local.json` is still tracked by mistake; see #845.)

## Project memory

What agents learn lives in two tiers with different bars:

| Tier | Where | What goes there | Bar |
|---|---|---|---|
| Project memory | `.claude/agent-memory/<agent>/` for subagents, `.claude/agent-memory/main-session/` for top-level sessions | Traps, file locations, measurements, the state of an open investigation, what an external handoff contained | Dated and sourced. Not necessarily verified end to end. |
| Documentation | `docs/` | Confirmed behaviour, decisions, RE findings | Cited evidence per [`evidence-standards.md`](../reverse-engineering/evidence-standards.md) |

Subagents with `memory: project` in their definition write to their own folder automatically. A top-level session keeps its own memory outside the repo by default, so the main-session folder is opt-in by rule. `CLAUDE.md` imports its index, which makes it load every session. Follow these rules:

- **Write project and reference facts to the repo.** Anything you learned from researching or writing code that the next contributor, or their agent, would otherwise have to rediscover goes in `.claude/agent-memory/main-session/`: one fact per file, with `name`, `description` and `type` frontmatter, plus a one-line pointer in its `MEMORY.md`. Commit it with the change that produced it, or on its own if there's no change.
- **Keep personal and machine state out.** User preferences, local absolute paths, machine-specific tool setup, and the branch, worktree or PR status of work in flight stay in your personal memory. The same goes for anything true only for your session.
- **This repo is public.** Never write IP addresses, hostnames of private infrastructure, credentials or tokens, player or tester account names, or anyone's personal details into a memory file. Name the doc or the operator that holds them instead.
- **Date what can go stale.** When a memory stops being true, add a dated status line rather than silently rewriting it, and reword its index line so the always-loaded index asserts nothing stale.
- **Promote, then trim.** When a memory's claim is verified to `docs/` standard, move it into the right doc (a finding, an ADR, [`rules-and-gotchas.md`](rules-and-gotchas.md)) in the same PR, and reduce the memory to a pointer or delete it.
- **Merge indexes, never overwrite them.** `MEMORY.md` files change often and deliberately list one file under several sections. When landing memory from another branch, apply the diff 3-way (`git diff … | git apply --3way`) and don't copy an index over or de-duplicate it.

## Shared docs without conflicts

Parallel PRs used to conflict mostly in docs, not code: each one bumped the same test count, findings count or gap-analysis total, and each packet edited the same status rows. Three rules prevent that.

- **Generated blocks belong to `tools/docs-gen/regen.py`.** Counts and tables between `<!-- gen:NAME -->` and `<!-- /gen:NAME -->` markers, and the crate graph between the `crate-graph` markers, are generated. The `regen-docs` workflow runs the script on `main` after every merge and commits whatever changed, so a PR never bumps them. Run `python tools/docs-gen/regen.py` locally to see the numbers, and commit its output only when your PR adds a marker. [`tools/docs-gen/README.md`](../../tools/docs-gen/README.md) lists the generators.
- **Status docs change once per campaign.** `docs/gap-analysis.md` and `docs/project-status.md` are updated in the campaign's close-out or release packet. Record per-packet progress in the campaign's own ledger under `docs/analysis/<campaign>/`.
- **Append-only lists merge by union.** [`.gitattributes`](../../.gitattributes) marks the `.claude/agent-memory/*/MEMORY.md` indexes, `docs/reverse-engineering/findings/README.md` and `docs/readme.md` with `merge=union`, so two PRs that append rows at the same spot both keep their rows instead of conflicting. Union applies only to local merges and rebases: GitHub's own conflict check ignores it, so rebase locally when GitHub reports a conflict. When both sides edit the same line, union keeps both versions, so read the result. Files whose rows are edited in place, such as the gap analysis, the test inventory tables and `crates/README.md`, are left on the normal merge driver.

Tip: `git config rerere.enabled true` makes git remember how you resolved a conflict and replay it on the next rebase.

## Builds, worktrees and test databases

Development builds run natively on Windows, from PowerShell or Git Bash. The tools below live under `tools/build-lane/`, `tools/dev-drive/`, `tools/build-hygiene/` and `tools/build-metrics/`; why they exist and what they measured is in [`docs/architecture/build-system.md`](../architecture/build-system.md).

### Create a worktree that builds

Run `bash tools/build-lane/mk-worktree.sh <branch> <name>` from any checkout. It creates `.claude/worktrees/<name>` on a new branch off `origin/main`, junctions `external/` in, and, when `CIMMERIA_TARGET_ROOT` points at a Dev Drive, seeds the new target dir from a warm one.

A worktree without `external/` does not build. `external/` is populated by `setup.ps1` and is not in git, and `crates/entity/build.rs` reads `../../external/recast`. If you create a worktree by hand, link `external/` with a junction (Windows) or a symlink. On a Linux host without `setup.ps1` (CI's case), reproduce the `hydrate external/recast` step from [`.github/workflows/test.yml`](../../.github/workflows/test.yml), which downloads the pinned Recast release into `external/recast`. First-time setup is otherwise in [`docs/building.md`](../building.md).

### Retire it when its PR merges

A worktree holds a target dir (several GB, on the Dev Drive when one is set up), a test database and a junction. On 2026-09-26, target dirs left behind by merged work filled the 150 GB Dev Drive, and every lane build on it failed with "no space left on device". So the session that created a worktree retires it the day its PR merges:

```bash
bash tools/build-lane/rm-worktree.sh <name>              # one worktree
bash tools/build-lane/rm-worktree.sh --dry-run --merged  # show what a sweep would do
bash tools/build-lane/rm-worktree.sh --merged            # every merged, idle worktree
```

The script deletes the target dir, unlinks `external/`, removes the worktree, deletes the local branch, and drops the worktree's `sgw_<name>` test database and its live-DB slot clones (`sgw_<name>_0`, `sgw_<name>_1`, ...). It refuses a worktree when:

- a lane job is building in it (nothing overrides this);
- it has uncommitted changes, or it is locked (an agent may still be using it);
- its branch's PR is still open or was closed unmerged, or the branch has unpushed commits.

`--force` overrides everything except a running build. `--merged` also skips anything committed to, checked out or built in the last 30 minutes, and deletes Dev Drive target dirs whose worktree is already gone.

**Orchestrators own their workers' worktrees.** A session that dispatched workers retires each worker's worktree when that worker's PR merges, not at the end of the campaign. That covers `isolation: "worktree"` agents (`agent-*`), workflow worktrees (`wf_*`) and `mk-worktree.sh` packets.

If you ever remove a worktree by hand on Windows, remove the junction first with plain `cmd /c rmdir <worktree>\external` (this removes only the link). Never use `rmdir /s`, `rm -rf`, or `Remove-Item -Recurse` on the junction or on a worktree that still contains it: a recursive delete can follow the link and empty the real `external/` directory.

### Build through the lane

Every agent or worker `cargo` call that compiles goes through the build lane:

```bash
bash tools/build-lane/lane.sh cargo check -p cimmeria-cell
bash tools/build-lane/lane.sh --exclusive cargo nextest run --profile=ci --workspace ...   # workspace-wide or measurement runs
```

- **The lane is per machine, not per worktree.** It is a counting semaphore: at most as many builds run as there are slots, and `--exclusive` takes all of them. The slot count is read from `%LOCALAPPDATA%\cimmeria-build\lane\SLOTS` (currently 4). A slot whose holder died is freed by the next caller.
- **It sets up the build environment.** `CARGO_BUILD_JOBS` defaults to cores ÷ slots (at least 4). Workspace crates build incrementally, and sccache, when installed, caches third-party crates in one shared cache, so a third-party crate one worktree compiled is a cache hit in the next. Each worktree builds into its own target dir: `<worktree>\target`, or `$CIMMERIA_TARGET_ROOT\<worktree>` on the Dev Drive. The lane runs sccache through a small wrapper that hides that per-worktree `CARGO_TARGET_DIR` from it; set `RUSTC_WRAPPER` yourself and you lose that.
- **It refuses to start on a nearly full disk.** Below `LANE_MIN_FREE_GB` free (default 10) on the target dir's drive, a job exits with code 28 and names the cleanup commands, instead of failing part-way with "os error 112". See [troubleshooting](../troubleshooting.md#lane-refuses-to-start-lane-refusing-to-start-n-gb-free-or-builds-fail-with-os-error-112).
- **It prunes stale incremental sessions after each job** in its own worktree, unless another lane job is building there. rustc keeps the previous session of every unit next to the newest one, and never reads it again. `LANE_PRUNE=0` turns this off.
- **Don't set `CARGO_INCREMENTAL=1`.** sccache refuses to run under it, so the lane drops sccache for that job. Workspace crates already build incrementally without it.
- **Every job is logged** to `%LOCALAPPDATA%\cimmeria-build\metrics\jobs.jsonl`: wait and run time, exit code, worktree, commit, settings, the lowest free RAM, the sccache hits and misses, the free disk at the start and the MB of incremental sessions it pruned. `python tools/build-lane/lane_stats.py` reports on it; `--recent 20` lists the last jobs, `--kind` and `--match` compare like with like, `--html` draws run time over time, and `--csv` exports every field. `LANE_METRICS=0` turns the log off.
- **Iterate per crate.** `cimmeria-services` is a small facade over about 20 crates, so `-p cimmeria-services` no longer covers the code you changed. Name the crate you edited: `-p cimmeria-cell`, `-p cimmeria-cell-content`, `-p cimmeria-base-methods`, and so on.

### Dev Drive (optional, recommended)

A Windows Dev Drive holds build output: the per-worktree target dirs and the sccache cache. Source code stays where it is.

1. From an elevated PowerShell, run `tools/dev-drive/New-CimmeriaDevDrive.ps1`. It creates a dynamically sized VHDX (by default `C:\DevDrives\cimmeria-build.vhdx`, up to 400 GB, mounted as `B:`), formats it as a Dev Drive, marks it trusted, and sets the user variables `CIMMERIA_TARGET_ROOT` and `CIMMERIA_SCCACHE_DIR`. It needs Windows 11 22H2 or later; block cloning needs 24H2.
2. Confirm Defender scans it asynchronously: Windows Security → **Virus & threat protection → Manage settings → Dev Drive protection** should say "Asynchronous scanning is on" for the drive. Don't rely on `(Get-MpPreference).PerformanceModeStatus`, which is not a per-volume reading.
3. New worktrees build on the Dev Drive from then on. `mk-worktree.sh` calls `tools/dev-drive/Copy-WarmTarget.ps1`, which seeds the new target dir from the most recently modified one by ReFS block cloning, in seconds and at almost no disk cost. Only third-party crates are reused; workspace crates rebuild once per worktree.

A worktree that already has a local `target\` keeps building there, so a job in flight never turns cold. Set `CIMMERIA_FORCE_DEV_DRIVE=1` to move it anyway.

### One test database per worktree

Live-DB tests reload and mutate the database, so each worktree uses its own on the bundled Postgres (`:5433`). Never reload a database another run is using.

- `bash tools/build-lane/reload-db.sh`, run from the worktree root, drops and reloads the worktree's database from `db/database.sql` and prints the `DATABASE_URL` to use. The database is `sgw_<worktree name>`; the main checkout keeps `sgw`. `CIMMERIA_TEST_DB=<name>` overrides it.
- `bash tools/build-lane/live-db-test.sh <test-name filter>` reloads that database, then runs the live-DB tier (`tools/test-live-db.sh`, the same crates and profile as CI) in one lane slot. The tier clones the database into one copy per live-DB slot, `sgw_<worktree name>_0` .. `_<N-1>`, and each running live-DB test uses its slot's copy.

Starting the bundled Postgres is documented in [`docs/architecture/integration-test-infra.md`](../architecture/integration-test-infra.md).

### Cleanup and measurement

- `tools/build-lane/rm-worktree.sh --merged` retires every merged, idle worktree and deletes orphaned Dev Drive target dirs (see [Retire it when its PR merges](#retire-it-when-its-pr-merges)). Run it before `sweep.ps1`: whole target dirs of merged work free far more space than trimming stale artifacts.
- `tools/build-hygiene/sweep.ps1` trims every target dir on the machine: the main checkout, `.claude/worktrees/*` and the Dev Drive. Through `cargo-sweep` (`cargo install --locked cargo-sweep`) it keeps only artifacts from the pinned toolchain and drops those unused for `-Days` (default 14). It also deletes stale incremental sessions, incremental caches not compiled for `-IncrementalHours` (default 24), and feature variants no build has read for `-VariantHours` (default 24), such as a crate's builds from before a rebase changed its dependencies. `-DryRun` prints what it would free per target dir, and `-Only <worktree>` limits it to one. It skips target dirs a lane job is building in. **Don't run it while anything else builds:** it can delete files a running build is about to use.
- `tools/build-metrics/measure-build.ps1` gives controlled numbers (cold build, edit loop, `cargo check`, peak memory, target size). Run it under `lane.sh --exclusive` from a worktree with an empty target dir, and re-measure before changing the slot count or `CARGO_BUILD_JOBS`.

## Definition of done

- The guard fails with the fix reverted, and you checked.
- The doc-update map rows are updated and the indexes are in sync.
- The pre-PR checklist passes on the CI toolchain.
- The PR body says what was not tested.
- For anything with a client UI element: the player gets visible feedback on the first press (see `rules-and-gotchas.md`). Otherwise it is not done, whatever the original server did.
- After the merge: the worktree is retired with `rm-worktree.sh`, and so is every worker worktree the session dispatched for it.
