# Development Workflow for AI-Assisted Work

How a change moves from ticket to merged PR in this repo when an AI harness is doing the work. It is harness-neutral: the steps apply whether you drive them through Claude Code subagents, another tool's equivalents, or by hand.

The repo already ships the pieces. Anyone who clones it with Claude Code gets them automatically:

- [`CLAUDE.md`](../../CLAUDE.md): build rules, pre-PR checklist, test policy, doc-update map, file organisation.
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
5. **Implement** with `rust-gameserver-dev` (or directly), iterating with `cargo check -p <crate>`.
6. **Ask "what if the client lies?"** Run `server-authority-enforcer` over any handler that takes client-supplied data into server state.
7. **Prove the guard.** Revert the fix, confirm the test fails, restore the fix. `testing-validation-engineer` does this review. Commit your work before you revert anything: a verification `git checkout` wipes uncommitted edits.
8. **Update the docs** named by the `CLAUDE.md` doc-update map, preferably with `documentation-writer`, and keep `docs/readme.md` and the section `README.md` indexes in sync.
9. **Run the pre-PR checklist** from `CLAUDE.md`, including clippy on the toolchain CI uses (see "Build and CI" in `rules-and-gotchas.md`).
10. **Open the PR** with the template filled in, including what you could not test.
11. **Commit agent memory.** If an agent wrote findings under `.claude/agent-memory/`, stage them with the change.

Fix warranted adjacent problems in the same pass: a file your change pushed over the 500-line cap, another instance of the bug a reviewer flagged, a doc your change made stale. Say what you fixed beyond the ask and why. Leave risky or judgment-heavy changes as a flagged follow-up.

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

- **Parallel writers need isolated worktrees.** Two implementation agents in one checkout race on git state: one agent's `git checkout` reverts the other's edits. Give each its own worktree under `.claude/worktrees/` (ignored by git). Read-only agents do not need one.
- **A fresh worktree does not build until `external/` is linked in.** `external/` is populated by `setup.ps1` and is not in git, and `crates/entity/build.rs` reads `../../external/recast`. Link it with a junction (Windows) or symlink. When deleting such a worktree on Windows, remove the junction first with `cmd /c rmdir <worktree>\external`, or the recursive delete can follow it into the real directory.
- **One cargo at a time per machine.** A full link can take ~47 GB. Agents running in parallel must serialise their builds. Setting `RUSTC_WRAPPER=sccache` keeps per-worktree `target/` directories from recompiling every dependency.
- **One live test database per concurrent run.** Live-DB tests reload and mutate the database. Point each worktree at its own database on the bundled Postgres (`:5433`) through `DATABASE_URL`, and never reload a database another run is using.
- **Do not switch branches in a checkout someone else is using.** Do integration work from a dedicated worktree.
- **Run the advisors on the model they were defined for.** They are tuned for judgment-heavy review; routing them to a smaller model to save tokens costs more in rework than it saves.
- **Agent memory is a deliverable.** Files under `.claude/agent-memory/` are committed alongside the work that produced them. `.claude/settings.local.json` and `.mcp.json` are per-machine and are not.

## Definition of done

- The guard fails with the fix reverted, and you checked.
- The doc-update map rows are updated and the indexes are in sync.
- The pre-PR checklist passes on the CI toolchain.
- The PR body says what was not tested.
- For anything with a client UI element: the player gets visible feedback on the first press (see `rules-and-gotchas.md`). Otherwise it is not done, whatever the original server did.
