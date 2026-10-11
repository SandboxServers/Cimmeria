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
> **Campaign status (2026-10-10): decisions recorded, wave 1 running.** The
> owner settled D-FX1 to D-FX9 on 2026-10-10 and asked for one session per
> row instead of relogs. That moved the fixture from an in-world console
> command to a fixture **staged** on the lab endpoint and applied by the base
> when the character next enters the world (F19 to F24). The packets below
> are the revised set; the first plan (`.fixture` console family, cell
> primitive, relog) is superseded. FX-01 is Writing.

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

A fixture row is its own **session**: the runner starts the client, logs in,
stages the fixture for the row's character at character select, presses
Play once, runs the row, then logs out and exits the client. The base applies
the staged state at Play, before it reads anything for world entry, so the
character enters exactly the way a returning player with that saved state
does: the `player_loaded` restore chains bind the dialogs, the space
instance is fresh, the camera is level. No relog, no in-world write.

When the row's chain predecessor ran and passed in the same run, the runner
does not apply the fixture: it continues the chain (the player route) and
checks that the state the chain produced matches the fixture (**parity**,
D-FX10). A full run is the player route with parity checks; a row run alone,
or after a failed row, starts from its fixture. Then
`lab uat first-session -Rows FS-P4 -RunsPerLease 20` means what it says.

**Rewind** (deferred, D-FX8) is the same mechanism fed from a snapshot.

Acceptance: FS-P3, FS-P4 and FS-P5 each pass when run alone, five times
each; a full run passes with every parity check green; and FS-P2 is still
the only row that needs a fresh first login (FX-12, which needs the user's
OK).

Out of scope: tag targeting, absolute camera and server-event waits (SV-);
golden fingerprints (lab-golden); fixtures for the SGU rows (FS-S3 to FS-S6
are uncalibrated; SV- calibrates them, and adding their fixtures is a
follow-up); restoring abilities, cash, XP, stats or bank contents.

## What was found

F1 to F18 against `main` @ `6996c9403`; F19 to F24 from the 2026-10-10
world-entry trace.

| # | Finding | Packets |
|---|---|---|
| F1 | The rows are a chain by design. `first-session.toml:79-94` says so: "The rows are not idempotent: progress is in sgw_mission", resume "from the first row that did not pass", rerun FS-01, FS-02 and FS-P2 after a relog, and call `client_camera {pitch_counts: 200}` before FS-P4 because FS-P4 "starts from FS-P3's camera". | FX-08 |
| F2 | The native mission GM commands only move forward. `gmMissionAssign` goes through `accept_mission`, whose offer guard refuses a mission that is active, or completed at its repeat cap (`crates/cell-content/src/cell/missions/lifecycle.rs:76-100`). `gmMissionClear` abandons only an active mission (`lifecycle.rs:195-197`, `crates/cell-console/src/cell/console/gm/missions.rs:218-225`). So once FS-P5 completes 622, nothing in game can put 622 back on step 80622. | D-FX1, FX-02 |
| F3 | `gmMissionReset(WSTRING DesignID, INT32 step)` (cell method 117, "revert a mission to a step") exists in the def and is unbuilt: `docs/protocol/cell-method-dispatch-table.md:547` (NEW, "no revert primitive"), `docs/commands.md:378` ("Not yet"). Follow-up, not this campaign. | FX-13 |
| F4 | The `.`-console has no mission-set or item-grant command (`crates/cell-console/src/cell/console/registry/commands/progression.rs:45-58`; `docs/gap-analysis/server-infrastructure.md:130`). | D-FX1 |
| F5 | The lab endpoint runs in the server process with a bearer token, fail-closed (`crates/lab-mcp/src/lib.rs:8-18`), and already reaches the cell (`server_console_exec`, `crates/lab-mcp/src/tools/console.rs:34-51`). | D-FX2, FX-04 |
| F6 | `server_db_query` is read-only by construction (`crates/lab-mcp/src/tools/db.rs:1-22`). The fixture write is server code behind a typed tool, not SQL on the endpoint. | D-FX1, FX-04 |
| F7 | Interaction bindings are not saved. The Frost and Guard dialog sets are re-bound on `player_loaded` by chains 1006 (step 2113) and 1007 (step 80623) (`db/resources/Content/Seed/castle_cellblock_chains.sql:463-498`). State written in world leaves them unbound; state present at world entry binds them. | D-FX5, FX-02 |
| F8 | The corpse search bits are cleared per space instance by chains 1003 and 1005 (`castle_cellblock_chains.sql:296`, `:430`), and every login builds a fresh Castle_CellBlock instance (`:558-560`). One session per row gets a clean instance for free. | D-FX5 |
| F9 | `.gotolocation` into the subject's own world is a same-space snap, not a reload (`docs/analysis/legacy-command-parity/worknotes/p46.md:94-105`). | D-FX5 |
| F10 | Tutorial 5882 shows once per character: `sgw_player_tutorials` has the key `(player_id, tutorial_id)` and only the first insert displays it (`crates/base-world-entry/src/base/world_entry/shown_tutorials.rs:5-10`). FS-P4 waits for `TutorialWin` (`first-session.toml:394`). | FX-02, FX-08 |
| F11 | Each row's chain gates name the state its fixture needs. FS-P3: 622 active on 2113 and 1360 not active (chains 1003 and 1121, `castle_cellblock_chains.sql:290`, `:356-361`). FS-P4: 622 on 80623 (chain 1005, `:423`). FS-P5: 622 on 80622 and item 55 carried (chain 1004, `:375-378`). | FX-08 |
| F12 | With 622 already active at world entry, chain 1001 (accept 622, show dialog 2982) does not fire (`castle_cellblock_chains.sql:224`). FS-P3's setup calls `@finish_dialog` as a required action (`first-session.toml:268`). | FX-08 |
| F13 | The runner can already call the lab endpoint (`ServerInvoker`, `crates/lab/src/uat/invoke.rs:106-114`) and find an entity id through `server_sessions` (`runner/packet.rs:50-78`, `:131-152`). `RunRequest` has no instance field (`runner/mod.rs:53-77`), though `lab_uat_run` has it (`server/uat.rs:373`). | FX-05 |
| F14 | A fresh character's name comes from the run id (`runner/session.rs:21-42`), and the rows' SQL clauses key on `Px${run_id}` (`first-session.toml:250`, `:359`, `:368`, `:427`, `:479`). | D-FX3, FX-05, FX-08 |
| F15 | `.gotoxyz` moves the GM's selected target when there is one (`registry/commands/travel.rs:8-14`). No longer used: the staged fixture writes the position before world entry. | — |
| F16 | Abandoning a mission saves `MISSION_NOT_ACTIVE` with its `repeats`, and the base deletes the row only when `repeats` is 0 (`lifecycle.rs:204-208`; base side `crates/base-methods/src/base/world_entry/methods/missions/mod.rs:147-171`). | FX-02 |
| F17 | `DEVELOPER_MODE` is an auth bypass for a database-less server (`crates/common/src/config.rs:104-108`), not a lab switch. | D-FX2 |
| F18 | `lab` subcommands are discovered from `tools/lab/cli/*.ps1` (`tools/lab/lab.ps1:32-65`). | FX-11 |
| F19 | World entry reads state in two stages, both after `handle_play_character` (`crates/base-world-entry/src/base/world_entry/play_character.rs:33`): at Play, `query_world_entry` reads world and position (`crates/base-methods/src/base/world_entry/methods/world_entry_db.rs:41`, `:134-136`) and `query_player_load_data` caches the inventory (`play_character.rs:118`); at `onClientReady`, missions (`query_saved_missions`, `client_ready/mod.rs:191`), tutorials (`load_player_init_row`, `:255`) and bandolier (`:309`). The cell fires `player_loaded` last (`crates/cell/src/cell/service/player_init/mod.rs:461`). Writing the rows at the top of `handle_play_character`, after the `world_entry_sent` latch (`:48-83`), lands before every read. | FX-02 |
| F20 | The base needs no mission definitions to write a correct row: hydration rebuilds the objective roster from `resources.mission_objectives` and re-derives hidden flags (`mission_restore.rs:58-61`, `:131-138`, `:181-184`). Prior steps' `completed_step_ids` and `completed_objective_ids` must be filled for chains that read earlier objectives (`persist.rs:30-44`). `handle_mission_update` (`missions/mod.rs:105`) already persists with the F16 rule. | FX-02 |
| F21 | Gate travel and respawn rerun `onClientReady`, so the fixture must be taken from the store at Play and applied once. | FX-02 |
| F22 | The lab endpoint is in the server process (`crates/server/src/main.rs:290`) and depends on `cimmeria-services`, which re-exports `base` (`crates/services/src/lib.rs:26`). `crates/base-session/src/base/world_entry/gm_only_worlds.rs:146-189` is a precedent for a process-wide `OnceLock<Mutex<..>>` keyed by player, written on one path and taken on another. `GM_ACCESS_LEVEL` is 2 (`gm_only_worlds.rs:36`). | FX-02, FX-04 |
| F23 | Inventory grant helpers assume a live session (`persist_grant` queues an outbox row for a cell entity, `inventory/grant/persist.rs:46`; `remove_by_type.rs:38`). The fixture's item write is one SQL transaction: delete the design id from carried containers 1 to 15 (`crates/entity/src/inventory.rs:13-32`; 16 to 20 are buyback, bank, auction and the team and command banks), then insert with `reserve_free_inventory_slots` (`vendor/serializers.rs:159`). | FX-03 |
| F24 | `lab_client_stop` terminates `SGW.exe` (`crates/lab/src/supervisor/lifecycle.rs:57-99`); the server learns of it from the dropped connection. A session that ends at character select (after `lab_logout`) has already saved the character; the next session must still wait until the server no longer lists the account before it logs in. | FX-06 |

## Decisions

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-FX1 | **Decided** 2026-10-10 (owner: (a), revised by D-FX5) | **How the runner sets state:** a typed lab endpoint tool, `server_stage_fixture`, stages a fixture for an offline character; the base applies it at that character's next Play, through server code (missions through `handle_mission_update`; items, tutorials and position in one transaction). No `.fixture` console family, no SQL on the endpoint, no chat typing. | The owner chose server-side, invariant-keeping code over a DB write tool and over native GM commands (which cannot reset F2, F10). One session per row (D-FX5) means the state must be in place before the one world entry (F7, F19), which an in-world command cannot do. |
| D-FX2 | **Decided** 2026-10-10 (owner: (a), revised) | **Lab-only gates:** (1) staging exists only as a lab endpoint tool (bearer token, fail-closed); (2) the character's account has `accesslevel >= 2` (GM) when staged, and the session's access level is GM when applied, else the fixture is discarded with a WARN; (3) the character is offline when staged; (4) one fixture per character, taken at Play, applied once, expiring 10 minutes after staging. No chat or console surface. | The owner's three gates mapped onto staging: the endpoint gate is unchanged, "acting GM on itself" becomes "a GM account's character", and nothing in game can reach it. |
| D-FX3 | **Decided** 2026-10-10 (owner: (a)) | **A reusable character per profile and lab instance** (`Uat Fxpraxis<instance>`), created on first use and kept. FS-P1 and FS-P2 still create and enter this run's `Px<run id>`. | Creation and the 16 s first-login hold happen once per instance, ever. |
| D-FX4 | **Decided** 2026-10-10 (owner asked; parity added) | **Fixture writes are silent** (no content events, rewards, Discord or tutorials), and the gap is covered two ways: full runs keep playing the chain, and every chained row's state is checked against its fixture (D-FX10). | A fixture skips the route *into* a row's state; the row's own steps still run the player route. The chain covers the transition, parity catches a fixture drifting from what the route really produces. |
| D-FX5 | **Decided** 2026-10-10 (owner) | **One session per fixture row, no relog.** The runner ends any running session (logout, client exit, wait until the server drops the account), starts the client, logs in, stages, presses Play once, runs the row, and ends the session the same way. A newly created fixture character gets one bootstrap session (Play, first-login hold, logout, exit) before its first staged session, once per instance. | The owner: "run more sessions instead of logging in and out; once the objective of the session is done the run ends with a proper logout and disconnect and game exit." A fresh entry gives restore-chain bindings (F7), a fresh instance (F8) and a level camera. |
| D-FX6 | **Decided** 2026-10-10 (owner) | **Fail closed.** After Play the runner checks every mission, item count and tutorial with `server_db_query`, the world with `server_sessions` and the position with `server_entity_get`. A mismatch, a refused stage, or no lab endpoint makes the row BLOCKED ("fixture not established: ..."), never FAIL, and its steps do not run. | A row that ran from the wrong state proves nothing. |
| D-FX7 | **Decided** 2026-10-10 (owner) | **Exact item counts per listed design id.** For each `{ id, count, container }` the base deletes every carried instance (containers 1 to 15; never 16 to 20), then inserts `count` into `container` (`main` by default, or `mission`). Unlisted items are untouched. `count` is 0 to 10. | Stale items would make FS-P4 false-pass and FS-P5 ambiguous. |
| D-FX8 | **Deferred** 2026-10-10 (owner asked whether partial rewind is worth it) | **Rewind (FX-11) waits** until FX-12 shows how isolated reruns feel. The snapshot half (FX-10) is built now because parity needs it. | With staged fixtures a rewind is "snapshot to fixture, new session"; whether to build it is a call to make after using fixtures. |
| D-FX9 | **Decided** 2026-10-10 (owner) | **FS-01, FS-02, FS-P1 and FS-P2 stay chained**, and no fixture touches `sgw_player.first_login`. | FS-P2 is the first-login proof. |
| D-FX10 | PROPOSED (coordinator, 2026-10-10) | **Fixture mode `auto`:** a row's fixture is applied unless the previous row of the same section ran in this run, passed, and left the client in world; then the row continues the chain in that session, its `${fixture_character}` is the chain's character, and the runner records **parity**: each mission, item and tutorial the fixture names must match the chain's state at the row boundary (mismatch fails the parity check, reported in the row JSON and the summary), and missions the chain has that the fixture does not name are listed as `unspecified`. `RunRequest.fixtures` takes `auto` (default), `always` and `never`. | Answers the owner's D-FX4 concern without a second run: a full run is the player route plus parity, a row run alone or after a failure starts clean (no cascade, F1). |

## Packets

| ID | Packet | Implementer | Size | Wave | Depends on | Status |
|---|---|---|---|---|---|---|
| FX-01 | Spec schema: `[row.fixture]`, `[section.profiles]`, validation | packet-coder | S | 1 | none | Writing |
| FX-02 | Base: fixture store, apply at Play (missions, tutorials, world, position) | rust-gameserver-dev | L | 1 | D-FX1, D-FX2 | Ready |
| FX-03 | Base: exact item counts in the staged apply | rust-gameserver-dev | M | 2 | FX-02 | BlockedDependency |
| FX-04 | Lab endpoint tool `server_stage_fixture` | rust-gameserver-dev | M | 2 | FX-02 | BlockedDependency |
| FX-05 | Runner plan: names, stage args, verify SQL, `RunRequest.instance` and `fixtures` | packet-coder | M | 2 | FX-01 | BlockedDependency |
| FX-06 | Runner sessions: end, bootstrap, stage, Play; fixture mode | packet-coder | L | 3 | FX-05 | BlockedDependency |
| FX-07 | Runner verify step and BLOCKED reasons | packet-coder | S | 4 | FX-06 | BlockedDependency |
| FX-08 | `first-session.toml`: fixtures for FS-P3, FS-P4, FS-P5 | packet-coder | S | 5 | FX-04, FX-07 | BlockedDependency |
| FX-09 | Docs: authoring guide, lab guide, endpoint tool | documentation-writer | S | 5 | FX-04, FX-07 | BlockedDependency |
| FX-10 | Row-boundary snapshots and parity | packet-coder | M | 6 | FX-07 | BlockedDependency |
| FX-11 | `lab rewind` | packet-coder | M | — | D-FX8 | Deferred |
| FX-12 | Live UAT acceptance (needs the user's OK) | coordinator | S | 7 | FX-08, FX-10 | BlockedDependency |
| FX-13 | Close-out | documentation-writer | S | 8 | all | BlockedDependency |

Size: S under about 40k tokens, M 40k to 70k, L 70k to 100k.

Waves (packets in one wave touch disjoint files and run in parallel):

1. FX-01 (`crates/lab/src/uat/spec*.rs`), FX-02 (`crates/base-session`, `crates/base-world-entry`).
2. FX-03 (`base-world-entry .../lab_fixture/items.rs`, one line in its `mod.rs`), FX-04 (`crates/lab-mcp/src/tools/`), FX-05 (`crates/lab/src/uat/fixture.rs`, `server/uat.rs`, `runner/mod.rs` request only).
3. FX-06 (`runner/fixture.rs`, `runner/session.rs`, `runner/mod.rs` `drive_row`).
4. FX-07 (`runner/fixture.rs` verify half).
5. FX-08 (the spec), FX-09 (docs).
6. FX-10 (`runner/snapshot.rs`, `uat/snapshot.rs`).
7. FX-12 (live, no files besides the ledger and a worknote).
8. FX-13.

## Contract collisions with lab-spec-vocab

Both campaigns add to `crates/lab/src/uat/spec.rs` and edit
`docs/guides/uat-specs/first-session.toml`.

- **`spec.rs`:** FX-01 adds two fields (`SectionMeta::profiles`,
  `RowSpec::fixture`) and the fixture types in one block headed
  `// ── Row fixtures (lab-fixtures FX-01) ──`, below `EvidenceSpec`. SV-
  adds step vocabulary to `ActionSpec`. Whichever lands second rebases.
- **`first-session.toml`:** SV- recalibrates FS-P3 and FS-P4 (tag targets,
  absolute camera) and replaces the 17 s sleep with a server-event wait.
  FX-08 adds fixtures, changes the clauses' character and rewrites the
  header's resume rules.
- **The first-login wait in the bootstrap session** (FX-06's
  `FIRST_LOGIN_HOLD_MS`) is a fixed 17 s until SV-'s server-event wait
  lands; FX-13 records the swap as a follow-up if it has not been made.
- **lab-golden** strips setup actions from fingerprints; fixture actions are
  recorded as `Role::Setup` with `kind = "fixture"` (FX-06).

## Dispatch rules

- **Workers.** One packet each, in its own worktree and test database:
  `pwsh -NoProfile -File tools/build-lane/mk-worktree.ps1 lab-fixtures/<packet>-<slug> <worktree>`.
  `packet-coder` (Haiku) for packets marked so; `rust-gameserver-dev` for
  FX-02, FX-03 and FX-04; `documentation-writer` for FX-09 and FX-13. The
  brief carries the worktree path, the packet section, the contract section
  of work-packets.md, and the commit subject with the attribution lines.
- **Review.** Each finished packet gets a Sonnet `packet-reviewer` on its
  commit range. FX-02 also gets `mission-systems-advisor`; FX-03
  `items-systems-advisor`; FX-02, FX-03 and FX-04 get
  `server-authority-enforcer` (the write paths). FX-06, FX-07 and FX-10 get
  `testing-validation-engineer`. Review fixes go to a fresh worker or the
  coordinator, never back to the implementer.
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
