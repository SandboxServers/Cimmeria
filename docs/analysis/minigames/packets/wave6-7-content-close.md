# Waves 6-7: content, UAT and close-out

> Type: how-to. Part of the [minigames work packets](../work-packets.md); the contract (C1-C7) and the dispatch rules live there.
> Content packets read [content-map.md](../content-map.md) (MG-R5) and [spec-launch-flow.md](../spec-launch-flow.md) (MG-R7). The architect confirms every `<from MG-R5>` and `<from MG-R7>` value at G1, or at the latest before the packet dispatches.

## Content rules (inline for every wave-6 packet)

- PowerShell only, with `Set-Location <wt>;` on every command.
- Cargo runs only through `pwsh tools/build-lane/lane.ps1` (except `cargo fmt`). Live-DB tests run through `pwsh tools/build-lane/live-db-test.ps1 <filter>`, in the foreground.
- **Seeds are the source of truth.** Edit `db/resources/` only; never add `db/scripts/*.sql` migrations.
- A new seed file gets its `\ir resources/Content/Seed/<file>.sql` line in `db/database.sql`, after the last `Content/Seed` line.
- Chain ids come from the packet's C7 block only. Grep the range in `db/resources/` first; if any id is taken, stop and return `blocked`.
- The chain row shape follows the existing minigame pairs in `db/resources/Content/Seed/castle_cellblock_chains.sql`: a launcher chain with a trigger, conditions and a `start_minigame` action, plus a victory chain with no trigger row. Read one pair before writing.
- **Tests:** type 6 chain-replay, in `crates/cell-content/src/cell/content/chain_replay_tests/`, following `livewire_pairs.rs`. That means four assertions: the launcher resolves exactly one `StartMinigame`, it resolves nothing at the wrong step, `execute_actions` emits one `CellToBaseMsg::StartMinigame` with the authored fields, and the victory and failure chain action lists match the seed. Register the module in `chain_replay_tests/mod.rs`.
- Docs are CRLF. No `git stash`, prune or push; make one local commit with the brief's attribution. No lab.
- **Lanes:**

  ```powershell
  Set-Location <wt>; cargo fmt -p cimmeria-cell-content
  Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings
  Set-Location <wt>; pwsh tools/build-lane/live-db-test.ps1 <the packet's test module name>
  ```

- **Docs.** Add a row to [mission-chains.md](../../../content/mission-chains.md) for each mission touched, and update [zone-audit.md](../../../content/zone-audit.md) if a zone row changes. Doc-update-map row "Seeds / content chains".

Ships one PR per packet. Merge order: MG-62, MG-63, MG-61, MG-64, MG-65.

## MG-61: mission 582 step 2307, Converse with Marsh

- Agent: packet-coder · Host: windows · Wave 6 · Depends on: MG-R5, MG-CH3 (or the G1 target game's `<G>3`)
- Worktree `mg-61` · Branch `minigames/mg-61-m582-marsh` · Commit `MG-61: mission 582 step 2307 plays the Marsh conversation minigame`
- **Ids:** chains 24000-24009.
- **Inputs (from MG-R5, confirmed at G1):** the NPC `<from MG-R5>` (template id and spawn), the dialog `<from MG-R5>`, the game `<from MG-R5, expected ConverseBasicHumanoid>`, the tier `<from MG-R5>`, the objective(s) of step 2307 `<from MG-R5>`, and the next step `<from MG-R5>`.
- **Files:**
  - `db/resources/Missions/Seed/mission_steps.sql:5335`: change row 2307's disabled flag to enabled. Copy the column order from the row; G1 names the exact column.
  - New `db/resources/Content/Seed/minigames_m582_chains.sql`:
    - launcher 24000: on the Marsh dialog choice (or interact) while step 2307 is active, it runs `start_minigame { minigame_type, difficulty, on_victory_chains: [24001], on_failure_chains: [24002] }`;
    - victory chain 24001: complete objective `<id>`, then advance the step;
    - failure chain 24002: a player message (`<from MG-R5 dialog text, else "Marsh loses interest. Try again.">`); the step stays open.
  - The `\ir` line in `db/database.sql`.
- **Test:** new `chain_replay_tests/minigames_m582.rs` with the four assertions, plus "a step-2307 row is enabled" read from the seeded DB. On revert, the launcher resolves nothing because the step is disabled.
- **Docs:** [mission-chains.md](../../../content/mission-chains.md), mission 582.

## MG-62: Converse abilities 1327 and 2089

- Agent: packet-coder · Host: windows · Wave 6 · Depends on: MG-24, MG-CH3, G1
- Worktree `mg-62` · Branch `minigames/mg-62-converse-abilities` · Commit `MG-62: Converse abilities launch the conversation minigame`
- **Ids:** chains 24010-24019 (victory and failure outcomes, if MG-R7 names any).
- **Inputs (from MG-R7):**
  - effect 1643 (ability 1327) and effect 2813 (ability 2089): their `script_name` → `StartMinigame`, with the NVP params `minigame_type=<from MG-R7>` and `difficulty=<from MG-R7>`;
  - their victory and failure outcomes `<from MG-R7>`. Where the outcome is "none", no chain is needed.
- **Files:**
  - the effects seed (Grep `1643` under `db/resources/` for the file holding effect rows): set `script_name` and the NVP params, following an existing scripted effect row's shape;
  - if outcomes exist, new `db/resources/Content/Seed/minigames_converse_chains.sql` and its `\ir` line.
- **Test:** a type 3 live-DB guard in `crates/cell-effect-scripts/` beside `registry.rs`'s seeded-names test, or a new `start_minigame_seed_live_db_tests.rs` there: effects 1643 and 2813 load with `script_name = "StartMinigame"` and the expected params, and the registry resolves them. On revert, the script name is unset. Lane: `live-db-test.ps1 start_minigame_seed`, plus clippy on `cimmeria-cell-effect-scripts`.
- **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md) § ability launch, the seeded abilities list.

## MG-63: Prison Boot 1597

- Agent: rust-gameserver-dev · Host: windows · Wave 6 · Depends on: MG-R7, MG-24, G1 · Reviewer pass: plus combat-systems-advisor
- Worktree `mg-63` · Branch `minigames/mg-63-prison-boot` · Commit `MG-63: Prison Boot 1597 is escaped through its minigame`
- **Ids:** chains 24020-24029.
- **Design:** G1 copies MG-R7's design here: who applies 1597 and effect 1939; what the minigame removes on victory (the effect, the root or the stun); what happens on failure; and whether the target or the caster plays.
- **Files (expected):**
  - the effect seed row for 1939: `script_name`, plus params for the escape game;
  - if removal on victory needs a content action that does not exist (for example `remove_effect`), add it to the content-engine `Action` enum, its loader and the executor, with a loader test. Name it in the report.
  - new `db/resources/Content/Seed/minigames_prison_boot_chains.sql` and its `\ir` line.
- **Tests:**
  - type 6: the victory chain removes the effect;
  - type 1: the new action's loader parses it (if one was added);
  - type 3: the effect seed row resolves to `StartMinigame`.

  On revert, the effect stays applied after a victory.
- **Lanes:** the content rules' lanes, plus C2 for `cimmeria-content-engine` and `cimmeria-cell-effect-scripts` if they are touched.
- **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md) § ability launch; the content-action reference row if a new action was added.

## MG-64: shell-game content per D-MG3 (BlockedDecision)

- Agent: packet-coder · Host: windows · Wave 6 · Depends on: D-MG3, MG-R5, MG-04 (and MG-26 under B)
- Worktree `mg-64` · Branch `minigames/mg-64-shell-content` · Commit `MG-64: wire the shell minigames into their seeded content`
- **Ids:** chains 24030-24039.
- **Under A:** wire only what `content-map.md` names: objectives 4963 (mission 4291) and 789 (mission 652), and any template with interaction bits 512-4096 that a mission uses. Each gets a launcher chain naming the shell game, at tier `<from MG-R5>`, with a victory chain completing the objective. The shells stay client-declared (MG-04: 5 s minimum play).
- **Under B:** the same rows, but they name the aliased game.
- **Test:** type 6, as in the content rules, in `chain_replay_tests/minigames_shells.rs`.
- **Docs:** [mission-chains.md](../../../content/mission-chains.md) rows for 4291 and 652.

## MG-65: objective 4203 upgrade (only under D-MG6 B)

- Agent: packet-coder · Host: windows · Wave 6 · Depends on: D-MG6 = B, a ported game · **BlockedDecision**
- Worktree `mg-65` · Branch `minigames/mg-65-obj-4203` · Commit `MG-65: objective 4203 plays a real minigame instead of the interact`
- **Ids:** chains 24040-24049.
- **Change.** Replace Harset's D-H13 interact route for mission 1244, step 3619, objective 4203 with a launcher (game and tier named by the owner's answer) and a victory chain that completes 4203. Remove the D-H13 interact chain; Grep `4203` in `db/resources/Content/Seed/` to find it.
- **Test:** type 6. The old interact no longer completes 4203, and the minigame victory does. Update the existing Harset chain-replay test that pins D-H13, and name it in the report.
- **Docs:** [mission-chains.md](../../../content/mission-chains.md), mission 1244; a superseded note on Harset's D-H13 in `docs/analysis/` (Grep `D-H13`).

## Wave 7: UAT, docs and close-out

Order: MG-71, MG-73, MG-72, the campaign review loop (skill § 4), then MG-99. One PR per packet.

### MG-71: cross-game TCP session test and REPL UAT index

- Agent: packet-coder · Host: windows · Wave 7 · Depends on: waves 3-6
- Worktree `mg-71` · Branch `minigames/mg-71-session-uat` · Commit `MG-71: every server-validated game survives a real SFS session`
- **Files:** new `crates/minigame/src/minigame/server/ported_games_tests.rs`, registered from `server/mod.rs` under `#[cfg(test)]`. For each game whose `validation()` is `"server"`, the test drives a real loopback SFS session through the existing listener test harness (copy the setup from `server/listener_tests.rs`, the Hack flow): login with the ticket, `joinOK`, the first golden move, then a client `victory`, which must be refused with no result sent.
- **Docs:** new `docs/analysis/minigames/uat-repl.md` (CRLF). For each game: the golden scripts, the command that replays them (`pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-minigame --bin minigame_repl -- --script <file>`), and what a human checks in the transcript. This is the REPL-style logic UAT that [AGENTS.md](../../../../AGENTS.md) asks for.
- **Lanes:** C2 for `cimmeria-minigame`.

### MG-72: docs sweep

- Agent: documentation-writer · Host: windows · Wave 7 · Depends on: waves 1-6
- Worktree `mg-72` · Branch `minigames/mg-72-docs` · Commit `MG-72: minigame docs match the shipped games`
- **Scope:** walk [doc-update-map.md](../../../agents/doc-update-map.md) for the campaign's union of changes.
  - [minigame-system.md](../../../gameplay/minigame-system.md): the game table (validation, tiers, cheats), the launch flow, failure chains and the flags.
  - [minigame-wire-formats.md](../../../reverse-engineering/findings/minigame-wire-formats.md): the per-game message tables, linking the specs.
  - [CAT-K-minigame.md](../../../security-audit/2026-05-31-server-authority/findings/CAT-K-minigame.md): the status of each finding.
  - `crates/minigame/README.md`, if one exists, or the crate docs in [crates/README.md](../../../../crates/README.md).
  - The `minigame-systems-advisor` memory: `game-implementation-status.md`.
  - Then `python tools/docs-gen/regen.py --skip-crate-graph` and `--check`, and `pwsh tools/lint-md.ps1` on every touched file.
- **Not here:** `docs/gap-analysis*` and `docs/project-status.md` (MG-99 only).

### MG-73: lab UAT recipes (ask-user-first)

- Agent: documentation-writer · Host: windows · Wave 7 · Depends on: waves 3-6
- Worktree `mg-73` · Branch `minigames/mg-73-lab-uat` · Commit `MG-73: lab UAT recipes for the minigames`
- **Files:** new `docs/analysis/minigames/uat-lab.md` (CRLF). Add one recipe per playable path:
  - the Livewire cellblock pair;
  - each ported game through a GM launch (Grep the GM commands for a minigame launcher; if there is none, record that as a gap rather than inventing one);
  - mission 582/2307;
  - ability 1327 or 2089;
  - Prison Boot;
  - the shell 5 s refusal.

  Each recipe gives the steps, the expected client view, the server-log lines (`reason=` values), and the SigNoz query. It is written for a Haiku `lab-driver`, with the gotchas first: stand 4-6 m from targets, clear dialogs first, and verify the driver's claims against the server log.
- **Rule:** the first line of the file says **"Ask the user before any lab use."** This packet writes recipes only and runs nothing.
- **Docs:** [unified-uat.md](../../../guides/unified-uat.md) is MG-99's job; link to it from here.

### Campaign review

Run per the skill § 4, in an `ops` worktree named `minigames-review`, over everything the campaign merged. Fixes ship as one PR before MG-99.

### MG-99: close-out

- Agent: documentation-writer · Host: windows · Wave 7 · Depends on: the campaign review
- Worktree `mg-99` · Branch `minigames/mg-99-close-out` · Commit `MG-99: minigames campaign close-out`
- **Files:**
  - `docs/gap-analysis*` and `docs/project-status.md`, changed once: the minigame rows, saying what is server-validated, what stays client-declared, and what is owed;
  - [unified-uat.md](../../../guides/unified-uat.md): a "Minigames" section pointing at `uat-repl.md` and `uat-lab.md`, with the owner-run lab steps marked ask-first;
  - this campaign's README: every row's final status, the missing-content register with its final verdicts, and the open owner decisions;
  - `RESUME.md`: phase closed.
- **Then (ops):** retire the worktrees (`rm-worktree.ps1 --merged`, and `--merged-into <PR>` for the wave-0 integration worktrees); empty `.claude/state/active-campaign`; post the board handoff.
