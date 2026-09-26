# Development Workflow for AI-Assisted Work

> **Last updated**: 2026-09-26
> **Audience**: Contributors doing AI-assisted work, and their agents
> **Type**: How-to

How a change moves from ticket to merged PR in this repo when an AI harness is doing the work. It is harness-neutral: the steps apply whether you drive them through Claude Code subagents, another tool's equivalents, or by hand.

The repo already ships the pieces. Anyone who clones it with Claude Code gets them automatically:

- [`CLAUDE.md`](../../CLAUDE.md): build rules, pre-PR checklist, test policy, doc-update map, file organization.
- [`AGENTS.md`](../../AGENTS.md) and [`.github/copilot-instructions.md`](../../.github/copilot-instructions.md): the same policy for other harnesses and for review bots.
- [`.claude/agents/`](../../.claude/agents/): sixteen domain subagents (roster below).
- `.claude/agent-memory/<agent>/`: what those agents learned on earlier runs. Committed on purpose.
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
8. **Update the docs** named by the `CLAUDE.md` doc-update map, preferably with `documentation-writer`, and keep `docs/readme.md` and the section `README.md` indexes in sync.
9. **Run the pre-PR checklist** from `CLAUDE.md`. `rust-toolchain.toml` pins the toolchain CI uses, so your clippy run is CI's.
10. **Open the PR** with the template filled in, including what you could not test.
11. **Commit agent memory.** If an agent wrote findings under `.claude/agent-memory/`, stage them with the change.

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
- **Agent memory is a deliverable.** Files under `.claude/agent-memory/` are committed alongside the work that produced them. `.claude/settings.local.json` and `.mcp.json` are per-machine and are not.

## Builds, worktrees and test databases

Development builds run natively on Windows, from PowerShell or Git Bash. The tools below live under `tools/build-lane/`, `tools/dev-drive/`, `tools/build-hygiene/` and `tools/build-metrics/`; why they exist and what they measured is in [`docs/architecture/build-system.md`](../architecture/build-system.md).

### Create a worktree that builds

Run `bash tools/build-lane/mk-worktree.sh <branch> <name>` from any checkout. It creates `.claude/worktrees/<name>` on a new branch off `origin/main`, junctions `external/` in, and, when `CIMMERIA_TARGET_ROOT` points at a Dev Drive, seeds the new target dir from a warm one.

A worktree without `external/` does not build. `external/` is populated by `setup.ps1` and is not in git, and `crates/entity/build.rs` reads `../../external/recast`. If you create a worktree by hand, link `external/` with a junction (Windows) or a symlink. On a Linux host without `setup.ps1` (CI's case), reproduce the `hydrate external/recast` step from [`.github/workflows/test.yml`](../../.github/workflows/test.yml), which downloads the pinned Recast release into `external/recast`. First-time setup is otherwise in [`docs/building.md`](../building.md).

When deleting a worktree on Windows, remove the junction first with plain `cmd /c rmdir <worktree>\external` (this removes only the link). Never use `rmdir /s`, `rm -rf`, or `Remove-Item -Recurse` on the junction or on a worktree that still contains it: a recursive delete can follow the link and empty the real `external/` directory.

### Build through the lane

Every agent or worker `cargo` call that compiles goes through the build lane:

```bash
bash tools/build-lane/lane.sh cargo check -p cimmeria-cell
bash tools/build-lane/lane.sh --exclusive cargo nextest run --profile=ci --workspace ...   # workspace-wide or measurement runs
```

- **The lane is per machine, not per worktree.** It is a counting semaphore: at most as many builds run as there are slots, and `--exclusive` takes all of them. The slot count is read from `%LOCALAPPDATA%\cimmeria-build\lane\SLOTS` (currently 4). A slot whose holder died is freed by the next caller.
- **It sets up the build environment.** `CARGO_BUILD_JOBS` defaults to cores ÷ slots (at least 4). Workspace crates build incrementally, and sccache, when installed, caches third-party crates in one shared cache. Each worktree builds into its own target dir: `<worktree>\target`, or `$CIMMERIA_TARGET_ROOT\<worktree>` on the Dev Drive.
- **Don't set `CARGO_INCREMENTAL=1`.** sccache refuses to run under it, so the lane drops sccache for that job. Workspace crates already build incrementally without it.
- **Every job is logged** to `%LOCALAPPDATA%\cimmeria-build\metrics\jobs.jsonl`: wait and run time, exit code, worktree, commit, settings, the lowest free RAM, and the sccache hits and misses. `python tools/build-lane/lane_stats.py` reports on it; `--recent 20` lists the last jobs, `--kind` and `--match` compare like with like, `--html` draws run time over time, and `--csv` exports every field. `LANE_METRICS=0` turns the log off.
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
- `bash tools/build-lane/live-db-test.sh <test-name filter>` reloads that database, then runs the live-DB tier (`tools/test-live-db.sh`, the same crates, profile and serialisation as CI) in one lane slot.

Starting the bundled Postgres is documented in [`docs/architecture/integration-test-infra.md`](../architecture/integration-test-infra.md).

### Cleanup and measurement

- `tools/build-hygiene/sweep.ps1` runs `cargo-sweep` (`cargo install --locked cargo-sweep`) over every target dir on the machine: the main checkout, `.claude/worktrees/*` and the Dev Drive. It keeps only artifacts from the pinned toolchain and drops those unused for `-Days` (default 14). Try `-DryRun` first. **Don't run it while anything builds:** it can delete files a running build is about to use.
- `tools/build-metrics/measure-build.ps1` gives controlled numbers (cold build, edit loop, `cargo check`, peak memory, target size). Run it under `lane.sh --exclusive` from a worktree with an empty target dir, and re-measure before changing the slot count or `CARGO_BUILD_JOBS`.

## Definition of done

- The guard fails with the fix reverted, and you checked.
- The doc-update map rows are updated and the indexes are in sync.
- The pre-PR checklist passes on the CI toolchain.
- The PR body says what was not tested.
- For anything with a client UI element: the player gets visible feedback on the first press (see `rules-and-gotchas.md`). Otherwise it is not done, whatever the original server did.
