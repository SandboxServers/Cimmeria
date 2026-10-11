# Lab row fixtures and rewind: work packets FX-06 to FX-13

> Type: work packets. Audience: `packet-coder` workers (Haiku),
> `documentation-writer` for FX-09 and FX-13, the coordinator for FX-12,
> and `packet-reviewer` reviewers (Sonnet). The dispatch header (lane
> commands, Rust rules) and the **Contract** these packets build against
> are in [work-packets.md](work-packets.md); FX-01 to FX-05 are there too.
> Ledger, findings and decisions: [README.md](README.md). Revised
> 2026-10-10 for one session per fixture row (D-FX5) and fixture mode
> (D-FX10).

## Contents

- [FX-06 Runner sessions and fixture mode](#fx-06-runner-sessions-and-fixture-mode)
- [FX-07 Runner verify step](#fx-07-runner-verify-step)
- [FX-08 first-session.toml fixtures](#fx-08-first-sessiontoml-fixtures)
- [FX-09 Docs](#fx-09-docs)
- [FX-10 Row-boundary snapshots and parity](#fx-10-row-boundary-snapshots-and-parity)
- [FX-11 lab rewind (deferred)](#fx-11-lab-rewind-deferred)
- [FX-12 Live UAT acceptance](#fx-12-live-uat-acceptance)
- [FX-13 Close-out](#fx-13-close-out)

## FX-06 Runner sessions and fixture mode

**Implementer:** packet-coder. **Size:** L. **Wave:** 3. **Depends on:** FX-05 (and FX-04's tool contract).
**Branch:** `lab-fixtures/fx06-sessions`. **Worktree:** `fx06`.
**Subject:** `feat(lab): FX-06 fixture rows run in their own session from a staged fixture`
**Reviewers:** packet-reviewer, testing-validation-engineer.

Why: README Purpose, F7, F8, F24; D-FX5, D-FX6, D-FX10.

Files:

1. `crates/lab/src/uat/runner/mod.rs`:
   - `Runner` gains `pub(crate) last_row: Option<LastRow>` with
     `pub(crate) struct LastRow { section: String, row_id: String, passed: bool }`,
     set wherever the run loop has a finished row's `RowEvidence`
     (`passed` = result is Pass).
   - `RowCtx` gains `pub(crate) fixture_mode: Option<&'static str>`
     (`"fixture"` or `"chain"`), written into the row's vars as
     `fixture_mode` so the row JSON shows which path ran.
   - `drive_row`, before `ensure_state`:

     ```rust
     if let Some(f) = &row.fixture {
         if self.applies_fixture(spec, row) {
             ctx.fixture_mode = Some("fixture");
             if let Err(e) = self.establish_fixture(spec, f, ctx).await {
                 ctx.blocked.push(format!("fixture not established: {e}"));
                 let _ = self.end_session(ctx).await;
                 return;
             }
         } else {
             // Continue the chain: the fixture names the state the chain
             // should have produced (parity, FX-10).
             ctx.fixture_mode = Some("chain");
             if let Some(n) = self.in_world_as.clone() {
                 ctx.vars.insert(FIXTURE_CHARACTER_VAR.into(), json!(n));
             }
         }
     }
     ```

     Skip `ensure_state` and `ctx.character = self.character_value(spec)`
     when the fixture path ran (the client is in world as the fixture
     character, and `ctx.character` already names it). At the end of
     `drive_row`, after teardown, when `ctx.fixture_mode == Some("fixture")`:
     `let _ = self.end_session(ctx).await;` (the session ends with the row,
     D-FX5).
   - `fn applies_fixture(&self, spec, row) -> bool`: `Never` → false;
     `Always` → true; `Auto` → false only when `self.last_row` is this
     section's row immediately before `row` in the spec's row order, it
     passed, and `self.in_world_as` is Some; true otherwise.
2. New `crates/lab/src/uat/runner/fixture.rs` (`mod fixture;` in
   `runner/mod.rs`):
   - `end_session(ctx)`: `lab_client_status`; when running: `lab_logout`
     if `login_state == "in_world"`, then `lab_client_stop`;
     `self.in_world_as = None`. Then, when `self.server` is set and a name
     was in play, poll `server_sessions` every 1 s for up to
     `SESSION_DRAIN_MS = 30_000` until no session row's `name` contains it
     (case-insensitive); timeout is Err
     `"server still lists <name> after 30 s"` (F24).
   - `establish_fixture(spec, f, ctx)`:
     1. No `self.server`: Err `"fixture needs the server lab MCP (CIMMERIA_LAB_MCP_URL/_TOKEN)"`.
     2. `name = fixture_character_name(&f.character, self.req.instance.as_deref().unwrap_or("default"))`;
        `ctx.vars[FIXTURE_CHARACTER_VAR] = name`;
        `ctx.character = json!({ "name": name })`.
     3. `self.end_session(ctx)?` (any earlier session, including a chain).
     4. `self.ensure_state(spec, "char_select", ctx)?`; `lab_characters`.
        When no listed character's name contains `name`
        (case-insensitive): `lab_ensure_character_slot { free_slots: 1, protect: [lab character] }`,
        `lab_create_character { first, last: name, alignment, archetype, gender }`
        from `spec.section.profiles[f.character]`, then the **bootstrap
        session**: `lab_play_character { name }`, wait
        `FIRST_LOGIN_HOLD_MS = 17_000` (a `const` with a comment: the
        server's 16 s cinematic hold; lab-spec-vocab replaces it with a
        server-event wait; skipped when `self.req.no_settle`, recorded as a
        `wait` action either way), `lab_finish_dialog` best effort,
        `end_session`, `ensure_state(spec, "char_select")`.
     5. `server_stage_fixture` with `stage_args(f, &name)`. Err
        `"stage: <message>"`.
     6. `lab_play_character { name }`; `self.in_world_as = Some(name)`.
     7. Resolve the entity id (`self.tap_entity(server, ctx)`; make it and
        `server_call` in `packet.rs` `pub(crate)` if they are not) and
        store it in `ctx.vars["player_entity_id"]`.
     Every call goes through `setup_call` (client tools) or `server_call`
     (server tools), so it is on the row's record; set `kind = "fixture"`
     on those records (`ActionRecord.kind`) so lab-golden can strip them.
3. `crates/lab/src/uat/runner/session.rs`: make `setup_call` `pub(crate)`.

Tests (unit with the scripted invokers in `runner/tests.rs` and the fake
server of `runner/packet_tests.rs`, TESTING.md type 1; new file
`runner/fixture_tests.rs`, registered under `#[cfg(test)]`):

- `a_fixture_row_runs_in_its_own_session_in_order`: assert the recorded
  tool order: `lab_client_stop` (earlier session), `lab_client_start`,
  `lab_login`, `lab_characters`, `server_stage_fixture`,
  `lab_play_character`, then after the row `lab_logout`,
  `lab_client_stop`, `server_sessions`. Fails if the stage moves after
  Play or the session is not ended.
- `a_new_fixture_character_gets_one_bootstrap_session`: two rows with the
  same profile; the client lists no character first, then lists it.
  Exactly one `lab_create_character` and one recorded first-login wait
  across both rows.
- `auto_mode_continues_a_passing_chain`: rows FS-A (no fixture, passes)
  then FS-B (fixture): FS-B records no `server_stage_fixture` and its
  `fixture_mode` var is `chain`, and `fixture_character` is the chain's
  character.
- `auto_mode_uses_the_fixture_after_a_failed_row`: FS-A fails; FS-B
  stages. Fails if `applies_fixture` ignores `passed`.
- `always_mode_stages_even_after_a_pass` and `never_mode_never_stages`.
- `a_refused_stage_blocks_the_row_and_runs_no_setup`: the fake server
  answers `stage refused: character X is online`; the row is BLOCKED
  with that text, no setup or step ran, and the session was ended.
- `no_lab_endpoint_blocks_a_fixture_row`.
- `the_session_drain_times_out_with_the_name`: the fake `server_sessions`
  keeps listing the character (use a short drain const under `cfg(test)`
  or `no_settle`).

Checks: the three lane commands, `-p cimmeria-lab`.

Docs owed: none (FX-09).

Reviewer focus: the stage happens at character select and before the one
Play; the entity id is resolved after Play; `end_session` runs on every
fixture exit path (success, BLOCKED, teardown); no call bypasses the
row's action record.

## FX-07 Runner verify step

**Implementer:** packet-coder. **Size:** S. **Wave:** 4. **Depends on:** FX-06.
**Branch:** `lab-fixtures/fx07-verify`. **Worktree:** `fx07`.
**Subject:** `feat(lab): FX-07 verify a row's fixture with read-only SQL before it runs`
**Reviewers:** packet-reviewer, testing-validation-engineer.

Why: D-FX6. The base applies the fixture with no reply to the runner, and
logs an error and carries on if a part fails (FX-02).

Files:

1. `crates/lab/src/uat/runner/fixture.rs`: `verify_fixture(f, ctx)`: for
   each `verify_checks(f, name)` check, call `server_db_query { sql }`
   through `server_call`; compare the pointer's value with `want` (numbers
   numerically). When `world` is set, the session's `world`
   (`server_sessions`) must equal it; when `position` is set, the entity's
   position (`server_entity_get` on `player_entity_id`) must be within
   `POSITION_TOLERANCE_M = 2.0` of it. Retry the whole set every 500 ms for
   up to `VERIFY_TIMEOUT_MS = 5_000`; on timeout Err listing every failing
   check as `"<what>: got <v>, want <w>"`, joined with `; `, capped at 300
   characters.
2. Call it at the end of `establish_fixture`, after the entity id.

Tests (unit, `runner/fixture_tests.rs`, TESTING.md type 1):

- `a_mission_on_the_wrong_step_blocks_with_got_and_want`: the fake
  `server_db_query` returns step 2113 for a fixture wanting 80623; the row
  is BLOCKED and the reason contains `got 2113, want 80623`. Fails if the
  verify call is removed from `establish_fixture`.
- `verify_retries_until_the_state_matches`.
- `a_wrong_world_blocks`.
- `the_blocked_reason_is_capped`.

Checks: the three lane commands, `-p cimmeria-lab`.

Docs owed: none (FX-09).

## FX-08 first-session.toml fixtures

**Implementer:** packet-coder. **Size:** S. **Wave:** 5. **Depends on:** FX-04, FX-07 (and D-FX3).
**Branch:** `lab-fixtures/fx08-first-session`. **Worktree:** `fx08`.
**Subject:** `feat(uat-specs): FX-08 fixtures make FS-P3, FS-P4 and FS-P5 independent`

Why: README F1, F10 to F12, F14. Rebase on lab-spec-vocab's edits to this
file first if they have landed (README, "Contract collisions").

A fixture row must work in both modes (D-FX10): from its fixture in a fresh
session, and continuing the chain. So the fixtures carry no `position`: each
row keeps its own setup positioning (`/gmgotoxyz`, or SV-'s `@stand` if
SV-13 has landed), which is right in both modes (a fresh entity has no
selection, README F15).

Edits to `docs/guides/uat-specs/first-session.toml`:

1. `[section.profiles.praxis]`: `alignment = "praxis"`,
   `archetype = "Soldier"`, `gender = "male"`, `first = "Uat"`.
2. FS-P3:

   ```toml
   [row.fixture]
   character = "praxis"
   world = "Castle_CellBlock"
   missions = [{ id = 622, step = 2113 }, { id = 1360, state = "not_active" }]
   items = [{ id = 3730, count = 0, container = "mission" }, { id = 55, count = 0 }]
   ```

   Make the `@finish_dialog` setup action `optional = true` (README F12:
   no accept dialog when 622 is already active at entry).
3. FS-P4: fixture with `world = "Castle_CellBlock"`,
   `missions = [{ id = 622, step = 80623 }, { id = 1360, step = 4037 }]`,
   `items = [{ id = 55, count = 0 }]`, `tutorials_unseen = [5882]`. Add
   `{ chat = "/gmsetgodmode 1", tier = "G" }` to its setup (it no longer
   inherits FS-P3's). Camera: if lab-spec-vocab's absolute camera has
   landed, use it in FS-P4's setup; otherwise leave the existing
   relative pitch and note in the header that FS-P4's camera is
   calibrated for the chain (FX-12 calibrates the fixture path).
4. FS-P5: fixture with `world = "Castle_CellBlock"`,
   `missions = [{ id = 622, step = 80622 }]`, `items = [{ id = 55, count = 1 }]`.
5. Every SQL clause in FS-P3, FS-P4 and FS-P5: replace `%px${run_id}%`
   with `%${fixture_character}%` (the clauses use `ILIKE`; in chain mode
   the runner sets the var to the chain's character).
6. The header: replace "How the rows are built" bullet 1 and the whole
   "Resuming a run that stopped part-way" block with: FS-01, FS-02, FS-P1
   and FS-P2 are a chain (FS-P2 is the fresh-login row); FS-P3, FS-P4 and
   FS-P5 carry fixtures: in a full run they continue the chain and check
   parity, and run alone (or after a failed row) they start from their
   fixture in their own session on the instance's reusable `Fxpraxis...`
   character (`lab uat first-session -Rows FS-P4 -RunsPerLease 20`); the
   SGU rows are still a chain. Keep the calibration table.

Tests: `uat::tests::committed_specs_parse_and_validate`
(`crates/lab/src/uat/mod.rs:126`) must pass:
`pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab committed_specs`.
It fails if a fixture names a missing profile or a bad state.

Docs owed: none beyond the spec header (FX-09 covers the guide).

## FX-09 Docs

**Implementer:** documentation-writer. **Size:** S. **Wave:** 5. **Depends on:** FX-04, FX-07.
**Branch:** `lab-fixtures/fx09-docs`. **Worktree:** `fx09`.
**Subject:** `docs(lab): FX-09 document row fixtures, fixture sessions and server_stage_fixture`

Doc-update-map row: "Live research lab ... the lab scripts and `lab`
command" (row 45). Files:

1. `docs/guides/automated-uat.md`: a new `### Row fixtures` under "Write a
   row spec", after "Two-player rows": the TOML shape, every field, the
   profile table, `${fixture_character}`, fixture mode (`auto`, `always`,
   `never`) and parity, what a fixture session does (end the previous
   session, log in, bootstrap a new character once, stage, Play, verify,
   run, end the session), the BLOCKED reasons, and the rules (one player,
   `state` `any` or `in_world`, no `position` when the row positions in
   setup). Add the fixture rows to "Spec coverage".
2. `docs/guides/live-research-lab.md`: `server_stage_fixture` in the
   endpoint's tool list: lab-only, offline GM-account character, applied
   at the next Play, taken once, 10-minute expiry.
3. `docs/architecture/live-research-lab.md` (the ADR): one paragraph on the
   staged-fixture path and its gates (D-FX2), if the ADR lists the
   endpoint's write surface; otherwise skip and say so in the PR.

Checks: `pwsh -NoProfile -File tools/lint-md.ps1` on the changed files
(warn-only). Keep CRLF line endings in the existing files.

## FX-10 Row-boundary snapshots and parity

**Implementer:** packet-coder. **Size:** M. **Wave:** 6. **Depends on:** FX-07.
**Branch:** `lab-fixtures/fx10-parity`. **Worktree:** `fx10`.
**Subject:** `feat(lab): FX-10 check chained rows against their fixtures (parity)`
**Reviewers:** packet-reviewer, testing-validation-engineer.

Why: D-FX4, D-FX10. A fixture that drifts from what the player route
produces would make isolated runs pass from a state no player reaches.

Files:

1. New `crates/lab/src/uat/snapshot.rs` (pure; `pub mod snapshot;` in
   `uat/mod.rs`):

   ```rust
   #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
   pub struct RowSnapshot {
       pub schema: u32,                    // 1
       pub character: String,              // letters only
       pub world: Option<String>,
       pub missions: Vec<SnapMission>,     // { id, status, step: Option<i32> }
       pub items: Vec<SnapItem>,           // { id, container, count }
       pub tutorials_seen: Vec<i32>,
       pub host_ms: i64,
   }
   /// Three read-only statements (missions, carried items, tutorials).
   pub fn snapshot_sql(name: &str) -> Result<[String; 3], String>;
   pub fn from_rows(name: &str, missions: &Value, items: &Value, tutorials: &Value) -> Result<RowSnapshot, String>;
   #[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
   pub struct Parity { pub mismatches: Vec<String>, pub unspecified: Vec<String> }
   /// Every mission, item count, tutorial and the world the fixture names
   /// must match `snap`; missions in `snap` the fixture does not name are
   /// `unspecified`.
   pub fn parity(f: &FixtureSpec, snap: &RowSnapshot) -> Parity;
   ```

   Mismatch text: `"mission 622: chain has step 2113, fixture wants 80623"`,
   `"item 55: chain carries 0, fixture wants 1"`,
   `"tutorial 5882: chain has seen it, fixture wants it unseen"`.
2. New `crates/lab/src/uat/runner/snapshot.rs`: `snapshot_row(ctx) -> Result<RowSnapshot, String>`
   runs the three queries and `server_sessions` (world) through
   `server_call`, and writes `rows/<section>/<row>.snapshot.json` beside
   the row JSON (a `RunDir` path helper next to `row_json` in
   `evidence.rs`).
3. `runner/mod.rs`: in `drive_row`, on the chain path
   (`fixture_mode == Some("chain")`), before setup: snapshot, compute
   `parity`, store it on the context. `RowEvidence` gains
   `#[serde(skip_serializing_if = "Option::is_none")] pub fixture_parity: Option<Parity>`.
   In `run_row`, after grading: when parity has mismatches and the result
   is Pass, the result becomes Fail with the reason
   `"fixture parity: <mismatches joined with ; >"` (capped at 300). A
   snapshot error is a reason that does not change the result.

Tests (unit, TESTING.md type 1):

- `parity_reports_a_wrong_step_item_count_and_seen_tutorial`.
- `parity_lists_unnamed_missions_as_unspecified_without_failing`.
- `a_chained_row_with_a_parity_mismatch_fails`: scripted run, FS-A passes,
  FS-B (fixture wanting 80623) chains, the fake `server_db_query` reports
  2113; FS-B is Fail with the parity reason. Fails if the Pass-to-Fail
  step is removed.
- `a_fixture_row_records_no_parity`.
- `from_rows_reads_the_server_db_query_shape`.

Checks: the three lane commands, `-p cimmeria-lab`.

Docs owed: a "Parity" paragraph in `docs/guides/automated-uat.md`'s Row
fixtures section (FX-09 has merged by then).

## FX-11 lab rewind (deferred)

Deferred by D-FX8 until FX-12 has run. When it is picked up: `lab rewind
<run> <row>` builds a fixture from the row's `.snapshot.json` (FX-10, which
then has to snapshot fixture rows too) and runs the row in a new session
with `fixtures = always`. Rewrite this section from the FX-12 worknote
before dispatch; LW-02's `Get-LabMcpUrl` is still a dependency.

## FX-12 Live UAT acceptance

**Implementer:** the coordinator. **Size:** S. **Wave:** 7. **Depends on:** FX-08 and FX-10 merged and deployed to the lab server.
**Needs the user's OK** before any lab use, at a time they choose, with the
lab free (memory: ask before the lab, never lend Haiku a lease). No code.

Steps, with `lab uat` (inferenceless; no lab-driver agent):

1. `lab uat first-session -Rows FS-P3 -RunsPerLease 5`
2. `lab uat first-session -Rows FS-P4 -RunsPerLease 5` (calibrate FS-P4's
   camera for the fixture path first if SV-'s absolute camera has not
   landed)
3. `lab uat first-session -Rows FS-P5 -RunsPerLease 5`
4. `lab uat first-session -Rows FS-P5,FS-P3,FS-P4 -Fixtures always` once
   (each row's fixture undoes the previous row's progress).
5. The whole section once (`auto`): FS-01 to FS-P2 create and enter a fresh
   character with the 16 s first-login hold; FS-P3 to FS-P5 chain, pass,
   and every parity check is green.
6. Watch the server log for the session ends: every fixture session's
   `lab_client_stop` is followed by the account leaving `server_sessions`
   within 30 s (F24), and no login is refused as already connected.

Pass: 5/5 for each of steps 1 to 3, and steps 4 to 6 green. Record each
batch's summary line and run dirs in `worknotes/FX-12.md`. A BLOCKED
"fixture not established" is a fixture bug, not a row failure: fix forward.
Then bring D-FX8 (rewind) back to the owner.

## FX-13 Close-out

**Implementer:** documentation-writer. **Size:** S. **Wave:** 8. **Depends on:** all.
**Branch:** `lab-fixtures/fx13-closeout`. **Worktree:** `fx13`.
**Subject:** `docs(lab-fixtures): FX-13 close out the lab fixtures campaign`

1. This ledger: every packet's status, the review outcomes, and the
   follow-ups: the native `/gmmissionreset` (cell method 117, README F3);
   fixtures for the SGU rows once lab-spec-vocab calibrates them;
   swapping `FIRST_LOGIN_HOLD_MS` for lab-spec-vocab's server-event wait
   if not done; the D-FX8 rewind decision; a graceful client quit in place
   of `lab_client_stop` if FX-12 shows stale sessions.
2. `docs/guides/unified-uat.md`: the first-session section's resume note
   now points at fixtures and fixture mode; owner UAT steps still pending.
3. `docs/gap-analysis/` (the lab or infrastructure area file) and
   `docs/project-status.md`: one line each, once.
4. Retire every worker worktree:
   `pwsh -NoProfile -File tools/build-lane/rm-worktree.ps1 --merged`.
5. Board: a handoff post in this campaign's subcategory.
