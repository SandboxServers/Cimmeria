# Lab row fixtures and rewind: work packets FX-06 to FX-13

> Type: work packets. Audience: `packet-coder` workers (Haiku),
> `documentation-writer` for FX-09 and FX-13, the coordinator for FX-12,
> and `packet-reviewer` reviewers (Sonnet). The dispatch header (lane
> commands, Rust rules) and the **Contract** these packets build against
> are in [work-packets.md](work-packets.md); FX-01 to FX-05 are there too.
> Ledger, findings and decisions: [README.md](README.md).

## Contents

- [FX-06 Runner establish step](#fx-06-runner-establish-step)
- [FX-07 Runner verify step](#fx-07-runner-verify-step)
- [FX-08 first-session.toml fixtures](#fx-08-first-sessiontoml-fixtures)
- [FX-09 Docs](#fx-09-docs)
- [FX-10 Row-boundary snapshots](#fx-10-row-boundary-snapshots)
- [FX-11 lab rewind](#fx-11-lab-rewind)
- [FX-12 Live UAT acceptance](#fx-12-live-uat-acceptance)
- [FX-13 Close-out](#fx-13-close-out)

## FX-06 Runner establish step

**Implementer:** packet-coder. **Size:** L. **Wave:** 3. **Depends on:** FX-05 (and FX-04's reply contract).
**Branch:** `lab-fixtures/fx06-establish`. **Worktree:** `fx06`.
**Subject:** `feat(lab): FX-06 the UAT runner establishes a row's fixture before setup`
**Reviewers:** packet-reviewer, testing-validation-engineer.

Why: README Purpose, F7 to F9, F15; D-FX5, D-FX6.

Files:

1. New `crates/lab/src/uat/runner/fixture.rs` (`mod fixture;` in
   `runner/mod.rs`). `establish_fixture(spec, f, ctx)`:
   1. No `self.server`: Err `"fixture needs the server lab MCP (CIMMERIA_LAB_MCP_URL/_TOKEN)"`.
   2. `name = fixture_character_name(&f.character, self.req.instance.as_deref().unwrap_or("default"))`;
      `ctx.vars[FIXTURE_CHARACTER_VAR] = name`;
      `ctx.character = json!({ "name": name })`.
   3. If `self.in_world_as != Some(name)`: `self.ensure_state(spec, "char_select", ctx)`;
      `lab_characters`; when no listed character's name contains `name`
      (case-insensitive), `lab_ensure_character_slot { free_slots: 1, protect: [lab character] }`
      then `lab_create_character { first, last: name, alignment, archetype, gender }`
      from the profile, and remember `created = true`. Then
      `lab_play_character { name }`; `self.in_world_as = Some(name)`. When
      created: sleep `FIRST_LOGIN_HOLD_MS = 17_000` (a `const` with a
      comment: the server's 16 s cinematic hold; lab-spec-vocab replaces it
      with a server-event wait), then `lab_finish_dialog` best effort.
   4. Resolve the entity id: `self.tap_entity(server, ctx)` (make it and
      `server_call` in `packet.rs` `pub(crate)` if they are not).
   5. `world` set and different from the session's `world`
      (`server_sessions` row): `server_console_exec { entity_id, line: ".gotolocation <world>" }`,
      then poll `server_sessions` every 1 s for up to 60 s until the
      character's `world` equals it; re-resolve the entity id.
   6. For each `console_lines(f)` line: `server_console_exec { entity_id, line }`.
      Ok only when one `output` line matches the line's reply regex; a line
      matching `FIXTURE_ERROR_REPLY`, or no match, is Err
      `"fixture: <line> -> <output>"`.
   7. `f.writes_state()`: wait `RELOG_SETTLE_MS = 1_000` (the base applies
      FX-03's messages in order before the logout it receives next), then
      `lab_logout` and `lab_play_character { name }`; remove
      `ctx.vars["player_entity_id"]` and re-resolve the entity id; store it
      back in that var.
   8. `position` set: `server_console_exec { entity_id, line: ".gotoxyz x y z" }`
      (after the relog the entity has no selection, README F15), then sleep
      `POSITION_SETTLE_MS = 2_500` (the specs' existing wait).
   Every call goes through the existing `setup_call` (client tools) or
   `server_call` (server tools) so it is recorded on the row; set
   `kind = "fixture"` on those records (`ActionRecord.kind`) so lab-golden
   can strip them.
2. `crates/lab/src/uat/runner/mod.rs` `drive_row`: after `ensure_state` and
   before the `players == 2` block:

   ```rust
   if let Some(f) = &row.fixture {
       if let Err(e) = self.establish_fixture(spec, f, ctx).await {
           ctx.blocked.push(format!("fixture not established: {e}"));
           return;
       }
   }
   ```

   and in `run_row`, `ctx.character` keeps the fixture name (do not
   overwrite it with `character_value(spec)` after establish).
3. `crates/lab/src/uat/runner/session.rs`: make `setup_call` `pub(crate)`.

Tests (unit with the scripted invokers in `runner/tests.rs` and the fake
server of `runner/packet_tests.rs`, TESTING.md type 1; new file
`runner/fixture_tests.rs`, registered in `runner/mod.rs` under
`#[cfg(test)]`):

- `a_fixture_creates_the_character_once_and_reuses_it`: two rows with the
  same profile; the client script lists no character first, then lists it.
  Assert exactly one `lab_create_character` across both rows, and that the
  second row calls no `lab_play_character` before its relog (already in
  world as the character). Skip the 17 s hold when `self.req.no_settle`
  (tests only), and record the hold as a `wait` action either way so the
  test can count it: exactly one.
- `fixture_lines_then_relog_then_position_in_that_order`: assert the
  recorded tool order: `server_console_exec` x N, `lab_logout`,
  `lab_play_character`, `server_sessions`, `server_console_exec .gotoxyz`.
  Fails if the relog (step 7) is removed or moved after the position.
- `a_refused_fixture_line_blocks_the_row_and_runs_no_setup`: the fake
  server answers `.fixture mission: step 4037 belongs to mission 1360`;
  the row is BLOCKED with that text and no setup or step action ran.
- `no_lab_endpoint_blocks_a_fixture_row`.
- `a_position_only_fixture_does_not_relog`.

Checks: the three lane commands, `-p cimmeria-lab`.

Docs owed: none (FX-09).

Reviewer focus: the entity id is re-resolved after every relog and world
change (a stale id would send `.fixture` to nobody, or the packet tap to a
dead session); no call bypasses the row's action record.

## FX-07 Runner verify step

**Implementer:** packet-coder. **Size:** S. **Wave:** 4. **Depends on:** FX-06.
**Branch:** `lab-fixtures/fx07-verify`. **Worktree:** `fx07`.
**Subject:** `feat(lab): FX-07 verify a row's fixture with read-only SQL before it runs`
**Reviewers:** packet-reviewer, testing-validation-engineer.

Why: D-FX6. FX-03's item and tutorial messages are applied by the base
after the console reply, so the reply alone proves nothing.

Files:

1. `crates/lab/src/uat/runner/fixture.rs`: `verify_fixture(f, ctx)`: for
   each `verify_checks(f, name)` check, call `server_db_query { sql }`
   through `server_call`; compare the pointer's value with `want` (numbers
   numerically). Retry the whole set every 500 ms for up to
   `VERIFY_TIMEOUT_MS = 5_000`; on timeout Err listing every failing check
   as `"<what>: got <v>, want <w>"`, joined with `; `, capped at 300
   characters. When `world` is set, also check the session's `world`.
2. Call it at the end of `establish_fixture`, after the position.

Tests (unit, `runner/fixture_tests.rs`, TESTING.md type 1):

- `a_mission_on_the_wrong_step_blocks_with_got_and_want`: the fake
  `server_db_query` returns step 2113 for a fixture wanting 80623; the row
  is BLOCKED and the reason contains `got 2113, want 80623`. Fails if the
  verify call is removed from `establish_fixture`.
- `verify_retries_until_the_base_catches_up`: the first answer is wrong,
  the second right; the row runs.
- `the_blocked_reason_is_capped`.

Checks: the three lane commands, `-p cimmeria-lab`.

Docs owed: none (FX-09).

## FX-08 first-session.toml fixtures

**Implementer:** packet-coder. **Size:** S. **Wave:** 5. **Depends on:** FX-04, FX-07 (and D-FX3).
**Branch:** `lab-fixtures/fx08-first-session`. **Worktree:** `fx08`.
**Subject:** `feat(uat-specs): FX-08 fixtures make FS-P3, FS-P4 and FS-P5 independent`

Why: README F1, F10 to F12, F14. Rebase on lab-spec-vocab's edits to this
file first if they have landed (README, "Contract collisions").

Edits to `docs/guides/uat-specs/first-session.toml`:

1. `[section.profiles.praxis]`: `alignment = "praxis"`,
   `archetype = "Soldier"`, `gender = "male"`, `first = "Uat"`.
2. FS-P3: add

   ```toml
   [row.fixture]
   character = "praxis"
   world = "Castle_CellBlock"
   missions = [{ id = 622, step = 2113 }, { id = 1360, state = "not_active" }]
   items = [{ id = 3730, count = 0, container = "mission" }, { id = 55, count = 0 }]
   position = [-325.0, 73.6, -212.8]
   ```

   Remove its `/gmgotoxyz` setup line (the fixture positions). Keep
   `/gmsetgodmode 1`. Make the `@finish_dialog` setup action
   `optional = true` (README F12).
3. FS-P4: fixture with `missions = [{ id = 622, step = 80623 }, { id = 1360, step = 4037 }]`,
   `items = [{ id = 55, count = 0 }]`, `tutorials_unseen = [5882]`,
   `position = [-319.0, 73.6, -212.5]`. Remove its `/gmgotoxyz` setup.
   Add `{ chat = "/gmsetgodmode 1", tier = "G" }` to setup (it no longer
   inherits FS-P3's). Unless lab-spec-vocab's absolute camera has landed,
   add `{ tool = "@camera", args = { pitch_counts = 200 } }` as its first
   step: after the fixture relog the camera is always level, which is what
   the header's resume rule compensated for.
4. FS-P5: fixture with `missions = [{ id = 622, step = 80622 }]` and
   `items = [{ id = 55, count = 1 }]`; no position (the equip needs none).
5. Every SQL clause in FS-P3, FS-P4 and FS-P5: replace `%px${run_id}%`
   with `%${fixture_character}%` (lower-case is fine: the clauses use
   `ILIKE`).
6. The header: replace "How the rows are built" bullet 1 and the whole
   "Resuming a run that stopped part-way" block with: FS-01, FS-02, FS-P1
   and FS-P2 are a chain (FS-P2 is the fresh-login row); FS-P3, FS-P4 and
   FS-P5 each carry a fixture, play the instance's reusable `Fxpraxis...`
   character, and run alone, in any order, or many times
   (`lab uat first-session -Rows FS-P4 -RunsPerLease 20`); the SGU rows
   are still a chain. Keep the calibration table.

Tests: `uat::tests::committed_specs_parse_and_validate`
(`crates/lab/src/uat/mod.rs:126`) must pass with the new fields; run it:
`pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab committed_specs`.
It fails if a fixture names a missing profile or a bad state, which is the
regression this packet can introduce.
Also `lab uat first-session -PlanOnly -Rows FS-P3,FS-P4,FS-P5` must report
the three rows ready, if a lab daemon is running and the user allows it;
otherwise leave it to FX-12.

Docs owed: none beyond the spec header (FX-09 covers the guide).

## FX-09 Docs

**Implementer:** documentation-writer. **Size:** S. **Wave:** 5. **Depends on:** FX-04, FX-07.
**Branch:** `lab-fixtures/fx09-docs`. **Worktree:** `fx09`.
**Subject:** `docs(lab): FX-09 document row fixtures and the lab-only .fixture commands`

Doc-update-map rows: "Live research lab ... the lab scripts and `lab`
command" (row 45) and "Developer how-to guides" (none needed). Files:

1. `docs/guides/automated-uat.md`: a new `### Row fixtures` under "Write a
   row spec", after "Two-player rows": the TOML shape, every field, the
   profile table, `${fixture_character}`, what the runner does (character,
   world, `.fixture` lines, relog, position, verify), the BLOCKED reasons,
   and the rule that a fixture row is `players = 1` and `state` `any` or
   `in_world`. Add the fixture rows to "Spec coverage".
2. `docs/commands.md`: a short "Lab-only console commands" table for
   `.fixture mission|item|tutorial`, stating they are refused from chat.
3. `docs/architecture/dev-console-channel.md`: one paragraph on the
   lab-only entry `handle_lab_console_command` and the three gates (D-FX2).
4. `docs/guides/live-research-lab.md`: one line under the endpoint's tools
   that `server_console_exec` also runs `.fixture`, and that
   `server_console_list` marks it `lab_only`.
5. `docs/protocol/cell-method-dispatch-table.md:547`: row 117's primitive
   cell becomes `cell-content missions/forced.rs force_mission_state
   (lab fixtures)`, status unchanged (NEW: the native binding is not built).

Checks: `pwsh -NoProfile -File tools/lint-md.ps1` on the changed files
(warn-only). Keep CRLF line endings in the existing files.

## FX-10 Row-boundary snapshots

**Implementer:** packet-coder. **Size:** M. **Wave:** 6. **Depends on:** FX-07.
**Branch:** `lab-fixtures/fx10-snapshots`. **Worktree:** `fx10`.
**Subject:** `feat(lab): FX-10 snapshot a character's server state at each row boundary`

Why: D-FX8. Rewind feeds a fixture from a snapshot.

Files:

1. New `crates/lab/src/uat/snapshot.rs` (pure; `pub mod snapshot;` in
   `uat/mod.rs`):

   ```rust
   #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
   pub struct RowSnapshot {
       pub schema: u32,               // 1
       pub character: String,         // letters only
       pub world: Option<String>,
       pub position: Option<[f32; 3]>,
       pub missions: Vec<SnapMission>,     // { id, status, step: Option<i32> }
       pub items: Vec<SnapItem>,           // { id, container, count }
       pub tutorials_seen: Vec<i32>,
       pub host_ms: i64,
   }
   /// The SQL the runner reads a snapshot with (three statements, one per table).
   pub fn snapshot_sql(name: &str) -> Result<[String; 3], String>;
   /// Rows of the three `server_db_query` results into a snapshot.
   pub fn from_rows(name: &str, missions: &Value, items: &Value, tutorials: &Value) -> Result<RowSnapshot, String>;
   /// The fixture that turns `now` back into `before` (D-FX8). Returns the
   /// fixture and any warnings (a tutorial seen in `before` but not `now`).
   pub fn snapshot_to_fixture(before: &RowSnapshot, now: &RowSnapshot) -> (FixtureSpec, Vec<String>);
   ```

   `snapshot_to_fixture`: missions in `before` keep their status (1 →
   step, 2 → completed); missions only in `now` become `not_active`; every
   item design id in either snapshot gets `before`'s summed carried count
   (0 when absent) and the container holding most of it in `before`
   (`main` when absent); tutorials in `now` but not `before` go to
   `tutorials_unseen`; world and position from `before`. `character` is
   set to the profile-less marker `"@snapshot"` (FX-11 handles it).
2. `crates/lab/src/uat/runner/snapshot.rs` (`mod snapshot;` in
   `runner/mod.rs`): `pub(crate) async fn snapshot_row(&mut self, ctx)`:
   when `self.server` is set and the row's character name is known, run
   the three queries plus `server_sessions` (world) and `server_entity_get`
   (position), and write `rows/<section>/<row>.snapshot.json` through
   `self.run.write_json` beside the row JSON (add a `RunDir` path helper
   next to `row_json` in `evidence.rs`). Never fail the row; on error
   write `{ "error": ... }`.
3. `runner/mod.rs` `drive_row`: call `snapshot_row` after the fixture and
   the anchor, before the tap starts.

Tests (unit, TESTING.md type 1):

- `snapshot_to_fixture_restores_missions_items_and_tutorials`: `before` has
  622 on 80623, one pistol, no 5882; `now` has 622 completed, 1360 active,
  the pistol and 5882; assert the fixture is 622 step 80623, 1360
  not_active, item 55 count 1, tutorials_unseen [5882]. Fails if missions
  only in `now` are not cleared (the bug shape: a rewind that leaves later
  progress behind).
- `a_tutorial_seen_before_but_not_now_is_a_warning`.
- `from_rows_reads_the_server_db_query_shape`: feed literal
  `server_db_query` JSON.
- `the_snapshot_file_is_written_next_to_the_row`: a scripted run writes
  `rows/first-session/FS-P4.snapshot.json`.

Checks: the three lane commands, `-p cimmeria-lab`.

## FX-11 lab rewind

**Implementer:** packet-coder. **Size:** M. **Wave:** 7. **Depends on:** FX-10.
**Branch:** `lab-fixtures/fx11-rewind`. **Worktree:** `fx11`.
**Subject:** `feat(lab): FX-11 lab rewind reruns one row from its snapshot`

Files:

1. `crates/lab/src/server/uat.rs`: `UatRunArgs` gains
   `#[serde(default)] pub rewind: bool` (doc: "with `run_dir` and exactly
   one row: restore the snapshot taken before that row, then run it");
   pass it into a new `RunRequest.rewind: bool`. Reject `rewind` without
   `run_dir` or with other than one row (`invalid_params`).
2. `crates/lab/src/uat/runner/mod.rs` `run_row`: when `self.req.rewind`,
   read `rows/<section>/<row>.snapshot.json` from the run dir (missing or
   `error`: BLOCKED "no snapshot for <row> in this run"), read the current
   state with the same queries (`snapshot_row` logic, factored into a
   function that returns the snapshot), build the fixture with
   `snapshot_to_fixture`, and establish it in place of `row.fixture`. For
   the `"@snapshot"` character, `establish_fixture` plays the snapshot's
   `character` by name and never creates one (Err "character <n> no longer
   exists" when it is missing). Record the warnings as row reasons that do
   not fail it.
3. New `tools/lab/cli/rewind.ps1` (`lab rewind`): `param([string]$RunDir, [string]$Row, [int]$Runs = 1, [switch]$Json)`;
   resolve `$RunDir` (`latest` means the newest run under the UAT root),
   find the row's section from its row JSON, then call `lab_uat_run`
   `{ sections = @($section); rows = @($Row); run_dir = $RunDir; rewind = $true }`
   `$Runs` times, reusing `uat-lib.ps1`'s session and verdict helpers, and
   print one line per run (`-Json`: `{ ok, row, runs, passed }`, no nulls,
   under 300 characters).
4. `tools/lab/cli/test-uat.ps1`: tests for the argument checks and the
   `-Json` shape (no daemon), in the file's existing style.

Tests:

- Rust (unit, `runner/fixture_tests.rs`):
  `rewind_establishes_the_snapshot_fixture_and_runs_the_row`, and
  `rewind_without_a_snapshot_blocks`.
- PowerShell: `pwsh -NoProfile -File tools/lab/cli/test-uat.ps1`.

Checks: the three lane commands, `-p cimmeria-lab`; the PowerShell test.

Docs owed (this packet, since FX-09 is done): the `lab rewind` row in
`docs/guides/live-research-lab.md#commands` and the `lab/` row in
`tools/README.md` (doc-update-map row 45), and a "Rewind a row" paragraph in
`docs/guides/automated-uat.md`.

## FX-12 Live UAT acceptance

**Implementer:** the coordinator. **Size:** S. **Wave:** 6. **Depends on:** FX-08 merged and deployed to the lab server.
**Needs the user's OK** before any lab use, at a time they choose, with the
lab free (memory: ask before the lab, never lend Haiku a lease). No code.

Steps, with `lab uat` (inferenceless; no lab-driver agent):

1. `lab uat first-session -Rows FS-P3 -RunsPerLease 5`
2. `lab uat first-session -Rows FS-P4 -RunsPerLease 5`
3. `lab uat first-session -Rows FS-P5 -RunsPerLease 5`
4. `lab uat first-session -Rows FS-P5,FS-P3,FS-P4` once (order
   independence; the runner keeps file order, so this proves only that
   each row's fixture undoes the previous row's progress).
5. The whole section once: FS-01 to FS-P2 still create and enter a fresh
   character with the 16 s first-login hold, and FS-P3 to FS-P5 pass on the
   fixture character.
6. After FX-11: one full run, then `lab rewind latest FS-P4 -Runs 3`.

Pass: 5/5 for each of steps 1 to 3, and steps 4 and 5 green. In step 5,
FS-P2 is the only row that waits out a first login; FS-P3 to FS-P5 record
no `lab_create_character`, except the fixture character's one-time
creation on an instance that never had one. Record each batch's summary line and run dirs in
`worknotes/FX-12.md`. A BLOCKED "fixture not established" is a
fixture bug, not a row failure: fix forward.

## FX-13 Close-out

**Implementer:** documentation-writer. **Size:** S. **Wave:** 8. **Depends on:** all.
**Branch:** `lab-fixtures/fx13-closeout`. **Worktree:** `fx13`.
**Subject:** `docs(lab-fixtures): FX-13 close out the lab fixtures campaign`

1. This ledger: every packet's status, the review outcomes, and the
   follow-ups: the native `/gmmissionreset` (cell method 117) binding over
   `force_mission_state` (README F3); fixtures for the SGU rows once
   lab-spec-vocab calibrates them; swapping `FIRST_LOGIN_HOLD_MS` for
   lab-spec-vocab's server-event wait if not done.
2. `docs/guides/unified-uat.md`: the first-session section's resume note
   now points at fixtures and `lab rewind`; owner UAT steps still pending.
3. `docs/gap-analysis/` (the lab or infrastructure area file) and
   `docs/project-status.md`: one line each, once.
4. Retire every worker worktree:
   `pwsh -NoProfile -File tools/build-lane/rm-worktree.ps1 --merged`.
5. Board: a handoff post in this campaign's subcategory.
