# Lab row fixtures and rewind

> Type: ledger. Audience: the coordinator, packet workers and reviewers.
> Opened 2026-10-10 against `main` @ `6996c9403`. Prefix `FX-`. Source brief:
> [lab-roadmap/handoff.md](../lab-roadmap/handoff.md) effort 2, "Row fixtures
> and rewind", and its section "Explaining fixtures and rewind". Packet specs:
> [work-packets.md](work-packets.md) (contract, FX-01 to FX-05) and
> [work-packets-runner-rewind.md](work-packets-runner-rewind.md) (FX-06 to
> FX-13). Sibling campaigns that touch the same
> files: lab-spec-vocab (`SV-`, `spec.rs` and `first-session.toml`),
> lab-golden, lab-record, lab-watch, client-limits and lab-chaos.
>
> **Campaign status (2026-10-10): planned, nothing built.** Three owner
> decisions (D-FX1, D-FX2, D-FX3) gate the server packets and the spec
> rewrite. FX-01 can start now.

## Purpose

Make UAT rows independent. Today a spec is a chain: FS-P4 works only after
FS-P3 advanced mission 622 on the same character, one failure turns every
later row into noise, and "run FS-P4 twenty times" means replaying create,
login, Frost and the Guard twenty times.

A **fixture** puts a row's starting state in the row:

```toml
[row.fixture]
character = "praxis"            # a [section.profiles] key
world = "Castle_CellBlock"
missions = [{ id = 622, step = 80623 }, { id = 1360, step = 4037 }]
items = [{ id = 55, count = 0 }]
tutorials_unseen = [5882]
position = [-319.0, 73.6, -212.5]
```

The runner establishes that state before the row's setup, through
server-authoritative, GM-gated, lab-only commands, then checks it with
read-only SQL. Then `lab uat first-session -Rows FS-P4 -RunsPerLease 20`
means what it says.

**Rewind** (a later wave) is the same mechanism fed from a snapshot: the
runner saves a character's mission, item and tutorial rows at each row
boundary, and `lab rewind <run> FS-P4` restores the snapshot taken before
FS-P4 and reruns that row, without replaying the chain.

Acceptance: FS-P3, FS-P4 and FS-P5 each pass when run alone, five times
each, and FS-P2 is still the only row that needs a fresh first login
(FX-12, which needs the user's OK).

Out of scope: tag targeting, absolute camera and server-event waits (SV-);
golden fingerprints (lab-golden); fixtures for the SGU rows (FS-S3 to FS-S6
are uncalibrated; SV- calibrates them, and adding their fixtures is a
follow-up); restoring abilities, cash, XP, stats or bank contents.

## What was found

Against `main` @ `6996c9403`.

| # | Finding | Packets |
|---|---|---|
| F1 | The rows are a chain by design. `first-session.toml:79-94` says so: "The rows are not idempotent: progress is in sgw_mission", resume "from the first row that did not pass", rerun FS-01, FS-02 and FS-P2 after a relog, and call `client_camera {pitch_counts: 200}` before FS-P4 because FS-P4 "starts from FS-P3's camera". | FX-08 |
| F2 | The native mission GM commands only move forward. `gmMissionAssign` goes through `accept_mission`, whose offer guard refuses a mission that is active, or completed at its repeat cap (`crates/cell-content/src/cell/missions/lifecycle.rs:76-100`). `gmMissionClear` abandons only an active mission (`lifecycle.rs:195-197`, `crates/cell-console/src/cell/console/gm/missions.rs:218-225`). So once FS-P5 completes 622, nothing can put 622 back on step 80622 on that character, and FS-P5 cannot run twice. | D-FX1, FX-02 |
| F3 | `gmMissionReset(WSTRING DesignID, INT32 step)` (cell method 117, "revert a mission to a step") exists in the def and is unbuilt: `docs/protocol/cell-method-dispatch-table.md:547` (NEW, "no revert primitive"), `docs/commands.md:378` ("Not yet"). It is the native twin of what a fixture needs, so the primitive FX-02 writes can back it later. | FX-02, FX-13 |
| F4 | The `.`-console has no mission-set or item-grant command. Its mission family is `.missionfail` and `.missionrewards` only (`crates/cell-console/src/cell/console/registry/commands/progression.rs:45-58`); a dot `.giveitem` is "not yet built" (`docs/gap-analysis/server-infrastructure.md:130`). The native `gmGiveItem` grants to the caller and is reached only by typing `/gmgiveitem` in the client (`gm/give.rs:96-104`). | D-FX1, FX-04 |
| F5 | The lab endpoint already runs `.`-console lines server-side: `server_console_exec` sends `BaseToCellMsg::LabConsoleExec`, the cell refuses a non-GM acting entity and tees the feedback lines back (`crates/lab-mcp/src/tools/console.rs:34-51`, `crates/cell/src/cell/service/base_messages/lab_console.rs:41-97`). No chat typing, no client focus, a structured reply. The endpoint itself starts only with `CIMMERIA_LAB_MCP_BIND` and a 32-byte token (`crates/lab-mcp/src/lib.rs:8-18`). | D-FX1, D-FX2, FX-04 |
| F6 | `server_db_query` is read-only by construction (`crates/lab-mcp/src/tools/db.rs:1-22`, `read_only_query`). A DB-row fixture needs a new write tool on that endpoint. | D-FX1 |
| F7 | Interaction bindings are not saved. The Frost and Guard dialog sets are re-bound on `player_loaded` by chains 1006 (step 2113) and 1007 (step 80623) (`db/resources/Content/Seed/castle_cellblock_chains.sql:463-498`). An in-world `gmMissionAdvance` replays step regions only (`gm/missions.rs:323-347`), so after a mission write in world the Guard's dialog would be unbound. A fresh world entry runs the restore chains. | D-FX5, FX-06 |
| F8 | The corpse search bits are cleared per space instance by chains 1003 and 1005 (`castle_cellblock_chains.sql:296`, `:430`), and "every relog builds a fresh Castle_CellBlock instance" (`:558-560`). A second FS-P3 in the same instance finds Frost unsearchable; after a relog it does not. | D-FX5, FX-06 |
| F9 | `.gotolocation` into the subject's own world is a same-space snap, not a reload (`docs/analysis/legacy-command-parity/worknotes/p46.md:94-105`). The only fresh entry the runner has is a relog (`lab_logout`, then `lab_play_character`). | D-FX5, FX-06 |
| F10 | Tutorial 5882 shows once per character: `sgw_player_tutorials` has the key `(player_id, tutorial_id)` and only the first insert displays it (`crates/base-world-entry/src/base/world_entry/shown_tutorials.rs:5-10`). FS-P4 waits for `TutorialWin` as a required step (`first-session.toml:394`), so its second run on one character fails. | FX-03, FX-08 |
| F11 | Each row's chain gates name the state its fixture needs. FS-P3: 622 active on 2113 and 1360 not active (chains 1003 and 1121, `castle_cellblock_chains.sql:290`, `:356-361`). FS-P4: 622 on 80623 (chain 1005, `:423`). FS-P5: 622 on 80622 and item 55 carried (chain 1004, `:375-378`). | FX-08 |
| F12 | After a fixture relog, chain 1001 (accept 622, show dialog 2982) does not fire because 622 is already active (`castle_cellblock_chains.sql:224`). FS-P3's setup calls `@finish_dialog` as a required action (`first-session.toml:268`). | FX-08 |
| F13 | The runner can already call the lab endpoint (`ServerInvoker`, `crates/lab/src/uat/invoke.rs:106-114`) and find a character's entity id through `server_sessions` (`runner/packet.rs:50-78`, `:131-152`). It does not know its lab instance: `RunRequest` has no instance field (`runner/mod.rs:53-77`), though `lab_uat_run` has it (`server/uat.rs:373`, `self.supervisor.instance()`). | FX-05 |
| F14 | A fresh character's name comes from the run id (`runner/session.rs:21-42`), so no character survives from one run to the next, and every `-RunsPerLease` run pays creation plus the 16 s first-login hold. The rows' SQL clauses key on `Px${run_id}` (`first-session.toml:250`, `:359`, `:368`, `:427`, `:479`). | D-FX3, FX-05, FX-08 |
| F15 | `.gotoxyz` moves the GM's selected target when there is one (`registry/commands/travel.rs:8-14`). Positioning right after a relog is safe (a fresh entity has no selection); positioning mid-row is not. | FX-06 |
| F16 | Abandoning a mission saves `MISSION_NOT_ACTIVE` with its `repeats`, and the base deletes the row only when `repeats` is 0 (`lifecycle.rs:204-208`). A fixture that clears a mission must zero `repeats` or the row survives. `MissionTracker::remove_mission` removes an instance of any status (`crates/entity/src/missions.rs:170-172`). | FX-02 |
| F17 | `DEVELOPER_MODE` is an auth bypass for a database-less server (`crates/common/src/config.rs:104-108`), not a lab switch. Lab-only has to mean "reached through the lab endpoint". | D-FX2 |
| F18 | `lab` subcommands are discovered from `tools/lab/cli/*.ps1` (`tools/lab/lab.ps1:32-65`), so `tools/lab/cli/rewind.ps1` is `lab rewind`. | FX-11 |

## Decisions

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-FX1 | **BlockedDecision** (owner). Recommended: **(a)** | **How the runner sets state.** (a) A lab-only `.fixture` console family (`.fixture mission`, `.fixture item`, `.fixture tutorial`), run through the existing `server_console_exec`, built on the cell's own mission primitive (FX-02) and two base messages (FX-03); position by `.gotoxyz`. (b) Seeded DB rows written through a new lab-mcp write tool (`sgw_mission`, `sgw_inventory`, `sgw_player_tutorials`, position) while the character is at character select. (c) Existing native GM commands only, typed into chat (`/gmmissionassign`, `/gmmissionadvance`, `/gmgiveitem`). | (a) is server-authoritative, keeps the cell's invariants (offer guard bypass is explicit and logged, step ownership is checked, persistence goes through `MissionUpdate`), adds no write tool and no SQL, and needs no client typing. (b) puts a write path on a token endpoint, and SQL that bypasses the cell would have to re-implement every invariant (hidden missions, objective lists, repeats). (c) cannot reset a completed mission or a seen tutorial (F2, F10), so FS-P5 and FS-P4 would still not repeat. |
| D-FX2 | **BlockedDecision** (owner). Recommended: **(a)** | **What "lab-only" means for `.fixture`.** (a) Three gates: the line arrives through `LabConsoleExec` (the lab endpoint, bearer token, fail-closed), the acting entity is a GameMaster, and the command acts on that entity only (no target, no name argument). The chat path refuses `.fixture` with one feedback line and changes nothing. The colo gets it wherever its lab endpoint runs (WireGuard-only port). (b) As (a), plus an env switch `CIMMERIA_LAB_FIXTURES=1` the operator sets per server. | (a) uses the gate the endpoint already enforces (F5) and keeps the blast radius to a GM's own character, which a GM can already move, grant and advance with native commands. (b) is defence in depth at the cost of one more env row to forget on the colo. `DEVELOPER_MODE` is not a fit (F17). |
| D-FX3 | **BlockedDecision** (owner). Recommended: **(a)** | **Which character a fixture row plays.** (a) A reusable character per profile and lab instance (`Uat Fxpraxis<instance>`), created on first use and kept. FS-P1 and FS-P2 still create and enter this run's `Px<run id>`, so a full run still proves a brand-new character's creation and first login, and FS-P3 to FS-P5 prove mission 622 from established state. (b) This run's `Px<run id>`, with the fixture applied to it. | (a) is what makes `-RunsPerLease 20` cheap: creation and the 16 s hold happen once per instance, ever. (b) keeps "one character's whole first session" in a full run, but a row run alone must create a character first, every run, and the lab slots fill with Px characters. The change in what the smoke proves is the owner's call. |
| D-FX4 | PROPOSED (coordinator) | **Fixture writes are silent.** `.fixture` fires no content events (`mission_accepted`, step regions), no Discord line, no rewards, no tutorial. The relog that follows (D-FX5) runs the `player_loaded` restore chains, which rebuild exactly what a returning player gets. | A fixture is test scaffolding: firing accept chains would display dialogs and grant items the fixture did not ask for, and make the state depend on chain side effects. The restore chains are the relog path players already depend on (F7). |
| D-FX5 | PROPOSED (coordinator) | **Every fixture that writes state ends with a relog**, then positions the player with `.gotoxyz`. A fixture with only `world` or `position` skips the relog. | Only a fresh entry re-binds interactions (F7), resets per-instance corpse bits (F8) and levels the camera, so every row starts from the same client state. `.gotolocation` cannot force it (F9). The relog costs one logout and one Play, still far less than replaying the chain. |
| D-FX6 | PROPOSED (coordinator) | **Fail closed.** After establishing, the runner checks every mission, item count and tutorial with `server_db_query` and the world with `server_sessions`. A mismatch, a refused `.fixture` line, or no lab endpoint makes the row BLOCKED ("fixture not established: ..."), never FAIL, and its steps do not run. | A row that ran from the wrong state proves nothing; BLOCKED keeps it out of the pass/fail counts and names the cause. |
| D-FX7 | PROPOSED (coordinator) | **Item semantics are exact per listed design id.** For each `{ id, count, container }` the base removes every carried instance of that design id (bags, bandolier, mission inventory; never a vault or bank), then grants `count` into `container` (`main`, the backpack, by default; or `mission`). Unlisted items are untouched. `count` is capped at 10. | FS-P4's `pistol-in-pack` clause would false-pass on a stale pistol, and FS-P5 must find exactly one unequipped pistol. Remove-then-grant is one rule with no special cases. |
| D-FX8 | PROPOSED (coordinator) | **Rewind restores only what a fixture can express**: missions, the item counts of every design id in either snapshot, unseen tutorials, world and position. Abilities, cash, XP, stats and bank contents are not snapshotted. A tutorial seen in the snapshot but not now is reported, not restored. | One mechanism for both features (the brief's design), and every field has a server path that already exists after FX-02 to FX-04. |
| D-FX9 | PROPOSED (coordinator) | **FS-01, FS-02, FS-P1 and FS-P2 stay chained**, and no fixture touches `sgw_player.first_login`. FS-P2 remains the only fresh-login row. | It is the acceptance line, and the first-login hold (16 s, cinematic AoI) is exactly what FS-P2 exists to cover. |

## Packets

| ID | Packet | Implementer | Size | Wave | Depends on | Status |
|---|---|---|---|---|---|---|
| FX-01 | Spec schema: `[row.fixture]`, `[section.profiles]`, validation | packet-coder | S | 1 | none | Ready |
| FX-02 | Cell primitive `force_mission_state` | rust-gameserver-dev | M | 1 | D-FX1, D-FX4 | BlockedDecision |
| FX-03 | Base messages: set an item count, forget tutorials | rust-gameserver-dev | M | 1 | D-FX1, D-FX7 | BlockedDecision |
| FX-04 | Lab-only `.fixture` console family | rust-gameserver-dev | M | 2 | FX-02, FX-03, D-FX2 | BlockedDecision |
| FX-05 | Runner plan: names, lines, replies, verify SQL; `RunRequest.instance` | packet-coder | M | 2 | FX-01, D-FX3 | BlockedDecision |
| FX-06 | Runner establish step | packet-coder | L | 3 | FX-05 | BlockedDependency |
| FX-07 | Runner verify step and BLOCKED reasons | packet-coder | S | 4 | FX-06 | BlockedDependency |
| FX-08 | `first-session.toml`: fixtures for FS-P3, FS-P4, FS-P5 | packet-coder | S | 5 | FX-04, FX-07 | BlockedDependency |
| FX-09 | Docs: authoring guide, commands, console channel, lab guide | documentation-writer | S | 5 | FX-04, FX-07 | BlockedDependency |
| FX-10 | Row-boundary snapshots and `snapshot_to_fixture` | packet-coder | M | 6 | FX-07 | BlockedDependency |
| FX-11 | `lab rewind` and `lab_uat_run { rewind }` | packet-coder | M | 7 | FX-10 | BlockedDependency |
| FX-12 | Live UAT acceptance (needs the user's OK) | coordinator | S | 6 | FX-08 | BlockedDependency |
| FX-13 | Close-out | documentation-writer | S | 8 | all | BlockedDependency |

Size: S under about 40k tokens, M 40k to 70k, L 70k to 100k.

Waves (packets in one wave touch disjoint files and run in parallel):

1. FX-01 (`crates/lab/src/uat/spec*.rs`), FX-02 (`crates/cell-content/src/cell/missions/`), FX-03 (`crates/wire`, `crates/base-world-entry`).
2. FX-04 (`crates/cell-console/src/cell/console/`, `crates/cell/.../lab_console.rs`, `crates/lab-mcp/src/tools/console.rs`), FX-05 (`crates/lab/src/uat/fixture.rs`, `server/uat.rs`, `runner/mod.rs` request only).
3. FX-06 (`runner/fixture.rs`, `runner/mod.rs` `drive_row`).
4. FX-07 (`runner/fixture.rs` verify half).
5. FX-08 (the spec), FX-09 (docs).
6. FX-10 (`runner/snapshot.rs`, `uat/snapshot.rs`), FX-12 (live, no files besides the ledger and a worknote).
7. FX-11.
8. FX-13.

## Contract collisions with lab-spec-vocab

Both campaigns add to `crates/lab/src/uat/spec.rs` and edit
`docs/guides/uat-specs/first-session.toml`.

- **`spec.rs`:** FX-01 adds two fields (`SectionMeta::profiles`,
  `RowSpec::fixture`) and the fixture types in one block headed
  `// ── Row fixtures (lab-fixtures FX-01) ──`, below `EvidenceSpec`. SV-
  adds step vocabulary to `ActionSpec`. The edits do not overlap; whichever
  lands second rebases.
- **`first-session.toml`:** SV- recalibrates FS-P3 and FS-P4 (tag targets,
  absolute camera) and replaces the 17 s sleep with a server-event wait.
  FX-08 adds fixtures, changes the clauses' character and rewrites the
  header's resume rules. If SV- has landed, FX-08 keeps its steps and drops
  the `pitch_counts = 200` prelude FX-08 would otherwise add to FS-P4.
- **The first-login wait inside the establish step** (FX-06's
  `FIRST_LOGIN_HOLD_MS`) is a fixed 17 s until SV-'s server-event wait
  lands; FX-13 records the swap as a follow-up if it has not been made.
- **lab-golden** strips setup actions from fingerprints; fixture actions are
  recorded as `Role::Setup` with `kind = "fixture"` (FX-06), so they fall
  out with the rest of setup.

## Dispatch rules

- **Workers.** One packet each, in its own worktree and test database:
  `pwsh -NoProfile -File tools/build-lane/mk-worktree.ps1 lab-fixtures/<packet>-<slug> <worktree>`.
  `packet-coder` (Haiku) for packets marked so; `rust-gameserver-dev` for
  FX-02, FX-03 and FX-04; `documentation-writer` for FX-09 and FX-13. The
  brief carries the worktree path, the packet section of
  [work-packets.md](work-packets.md) or
  [work-packets-runner-rewind.md](work-packets-runner-rewind.md), the
  contract section of work-packets.md, and the commit
  subject with the attribution lines.
- **Review.** Each finished packet gets a Sonnet `packet-reviewer` on its
  commit range. FX-02 also gets `mission-systems-advisor`; FX-03
  `items-systems-advisor`; FX-02, FX-03 and FX-04 get
  `server-authority-enforcer` (the write paths). FX-06 and FX-07 get
  `testing-validation-engineer` (do the guards fail when reverted). Review
  fixes go to a fresh worker or the coordinator, never back to the
  implementer.
- **Shell.** PowerShell only: no bash, WSL or Git Bash, no direct `cargo`,
  no `git worktree prune`, no `git stash`. Every compiling command goes
  through `pwsh -NoProfile -File tools/build-lane/lane.ps1`.
- **The lab.** No packet but FX-12 touches the lab, and FX-12 runs only
  after the user says yes, at a time they pick.
- **Ship.** `python tools/build-lane/ship.py pr -C <worktree> -m <msg>`, then
  `python tools/build-lane/ship.py merge <PR> --retire <worktree>` once the
  build-proving CI jobs pass. Update this table and write
  `worknotes/<packet>.md` when anything is left over.
- **Shared files.** Only FX-13 edits `docs/gap-analysis*`,
  `docs/project-status.md` and `docs/guides/unified-uat.md`. FX-09 owns the
  other doc edits; earlier packets record their doc deltas in their
  worknote.

## Review outcomes

None yet. Where merged code differs from the packet specs, record it here;
the code is then the reference, not the spec.
