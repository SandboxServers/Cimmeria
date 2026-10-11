# Lab spec vocabulary: work packets (2 of 3)

> Type: work packets. The header rules, the lane commands and the
> [contract](work-packets.md#contract) are in
> [work-packets.md](work-packets.md); they apply here unchanged. SV-13 to
> SV-16: [work-packets-3.md](work-packets-3.md). Ledger:
> [README.md](README.md).

## Contents

- [SV-06 client_hover_probe](#sv-06-client_hover_probe)
- [SV-07 Runner target resolution and @stand](#sv-07-runner-target-resolution-and-stand)
- [SV-08 Server-event waits and polling clauses](#sv-08-server-event-waits-and-polling-clauses)
- [Calibration contract](#calibration-contract)
- [SV-09 Calibration core](#sv-09-calibration-core)
- [SV-10 lab_uat_calibrate](#sv-10-lab_uat_calibrate)
- [SV-11 Heal hint on a row summary](#sv-11-heal-hint-on-a-row-summary)
- [SV-12 lab calibrate and lab uat -Heal](#sv-12-lab-calibrate-and-lab-uat--heal)

## SV-06 client_hover_probe

**Implementer:** packet-coder. **Size:** M. **Wave:** 2. **Depends on:** SV-05 (the sim readout).
**Branch:** `lab-spec-vocab/sv06-hover-probe`. **Worktree:** `sv06`.
**Subject:** `feat(lab): SV-06 client_hover_probe scores a pose by hover, with no click`

Why: README F12, F13; D-SV6. The calibration sweep's only client primitive.

Files:

1. `crates/lab/src/supervisor/world/io.rs`: `WorldIo` gains
   `window_rects` (contract), default `Ok(vec![])`. `LiveWorld` implements
   it with one Lua call that walks the visible top-level windows (reuse
   `WALK_FN` and `getUnclippedPixelRect` exactly as
   `supervisor/ui/window_click.rs:41-46` does) and returns
   `{name, rect}` for each, leaving out any window whose rect covers 90% or
   more of the screen (the root and full-screen backdrops).
2. New `crates/lab/src/supervisor/world/hover.rs` (`pub mod hover;` in
   `world/mod.rs`, plus a module-doc bullet):
   - `HoverRequest` (contract). Exactly one of `entity_id` and `point`;
     `expect_entity` only with `point`.
   - Resolve like `click::run` does (entity: `find_entities` with
     `rendered_only`, `limit 1`; point: `PointArg::to_client`). Never turn
     the camera: an off-screen aim is `verdict: "off_screen"`.
   - Centre pixel: for an entity, the first `HOVER_OFFSETS` height whose
     hover is the entity (reuse `hover_once`; make it `pub(super)`); for a
     point, the projected point.
   - Verdict at a pixel: place the cursor, wait `HOVER_WAIT_MS`, read
     `Unit.MouseOver` and the rect containing the pixel. `ok` when (entity
     aim) mouse-over is the entity, or (point aim) mouse-over is 0 or
     `expect_entity`, and in both cases no window rect contains the pixel.
     Otherwise one of `"own"` (the player), `{"other_entity": id}`,
     `{"ui": "<window>"}`, `"nothing"` (entity aim).
   - Jitter: 8 points at `jitter_px` (default 12) around the centre, at 0,
     45, ..., 315 degrees; `jitter_hits` counts the `ok` ones.
   - `avatar_px`: the smallest pixel distance from the centre to the
     player's projected actor point at the `HOVER_OFFSETS` heights;
     `edge_px`: the distance to the nearest screen edge. Both rounded.
   - The cursor goes back where it was (`place_cursor` without a mouse
     move) at the end, also on failure.
   - Never presses a button. `steps.used(NativeLevel::RealInput)` for the
     cursor moves.
   - Result: `{ "verdict", "pixel": [x, y], "hover", "jitter_hits",
     "jitter": 8, "avatar_px", "edge_px" }` plus `Steps::finish`.
3. `crates/lab/src/server/world.rs`: register `client_hover_probe`
   ("Hover a target or a world point with the cursor, without clicking,
   and score it: whether the client's mouse-over (and no HUD window) is
   right at the aim pixel and at 8 points around it, and how far the pixel
   is from the avatar and the screen edge. For calibrating spec poses.") and
   add it to `world_tools_are_routed`.
4. `crates/lab/src/lease/policy.rs`: `"client_hover_probe"` in the
   world-actions group (it moves the shared cursor).
5. `crates/lab/src/supervisor/world/sim.rs`: `window_rects` returns a
   configurable `Vec` (a new field, empty by default), and the simulated
   mouse-over reports the player's own id when the cursor is within a
   configurable radius of the player's projected point (read how the sim
   computes mouse-over today first; extend, do not rewrite).

Tests (new `crates/lab/src/supervisor/world/hover_tests.rs`, `#[path]` from
`hover.rs`; unit with the simulated client):

- `an_open_target_scores_full_jitter`: an entity clear of the avatar and
  any window gives `verdict ok`, `jitter_hits 8`, and the sim saw no button
  press (assert its button log is empty). Fails if the probe clicks.
- `the_avatar_in_front_costs_the_pose`: the target's centre pixel within
  the avatar radius gives `hover "own"` and `verdict` not ok.
- `a_hud_window_over_the_point_is_a_miss`: a point aim under a configured
  window rect gives `hover {"ui": "<name>"}`.
- `a_point_aim_accepts_nothing_under_the_cursor`: point aim, mouse-over 0,
  no window: `ok`.
- `off_screen_is_reported_not_turned`: the camera did not move (compare
  the sim's yaw before and after).

Checks: the three lane commands for `-p cimmeria-lab`.

Docs owed: none here (SV-14 adds the tool to the lab guide's table).

Reviewer focus: no button calls on any path; the cursor is restored; the
lease policy lists the tool.

## SV-07 Runner target resolution and @stand

**Implementer:** rust-gameserver-dev. **Size:** M. **Wave:** 2. **Depends on:** SV-03 (SV-01 for the live server).
**Branch:** `lab-spec-vocab/sv07-targets`. **Worktree:** `sv07`.
**Subject:** `feat(lab): SV-07 resolve spec targets by server tag and stand at poses`

Why: README F2, F10, F11; D-SV1, D-SV3, D-SV11.

Files:

1. New `crates/lab/src/uat/runner/targets.rs` (`mod targets;` in
   `runner/mod.rs`), the contract's `ResolvedTag` and four methods:
   - `player_entity`: call `client_entity_find` on `who` with
     `{ include_player: true, limit: 1, project: false }` and read
     `/player/id`. Push the call into `rec.calls`.
   - `resolve_tag`: no server configured is an error ("tag <tag> needs the
     server lab MCP (CIMMERIA_LAB_MCP_URL/_TOKEN)"). Else
     `server_entity_get { entity_id: <player> }` for `/entity/space_id`, then
     `server_entity_query { space_id, tag }`; keep only the reply's
     `/entities/*` whose `tag` equals `tag` (D-SV1: an old server ignores
     the argument); exactly one is `Ok` (position from its `position`),
     zero is `"no entity with tag <tag> in space <id>"`, more is `"tag <tag>
     matches entities <ids> in space <id>"`. Each server call goes into
     `rec.calls` as `{tool, args, ok}`; the result as `{resolve: {pose,
     tag, entity_id, space_id}}` (`pose` when the action named one: SV-11's
     heal hint reads it there).
   - `expand_target_args(who, tool, args, ctx, rec)`: returns `args`
     unchanged unless it has `pose`, `tag` or `face_tag`. A `pose` is looked
     up in `ctx.poses` (unknown: error) and expands, with any explicit arg
     winning over the pose's value:
     - `client_world_click` / `client_target`: aim entity gives
       `entity_id`; aim point gives `point = {x, y, z}` from
       `aim_point(position)`, plus `expect_entity` is not passed (the click
       tool has no such field). `rotate_camera` defaults to `false` (the
       pose set the camera).
     - `client_camera`: aim entity gives `face_entity_id`; aim point gives
       `face_point`; plus `pitch_deg`, `yaw_offset_deg`, `zoom` from the
       pose when set.
     - `client_entity_find`: `entity_id`.
     - `uat_stand`: handled by `exec_stand` below, not here.
     A bare `tag` (or `face_tag`) resolves the same way; `aim`/`aim_dy_m`
     next to a bare `tag` behave as in a pose. The keys `pose`, `tag`,
     `face_tag`, `aim`, `aim_dy_m` are removed from what the client tool
     receives.
   - `stand_at(who, point, rec)`: send `/gmgotoxyz {x:.2} {y:.2} {z:.2}` with
     `send_chat`, then poll `client_entity_find { include_player: true,
     limit: 1, project: false }` every 250 ms until
     `/player/position/server` x and z are within 0.5 m of the point, at
     most 5 s; then `Ok`. Timeout: `"stand: still <d> m from <point> after
     5 s"`. Keep the poll results out of `rec.calls` except the last.
   - `exec_stand(a, who, ctx, rec)`: `pose` gives the stand point from the
     resolved target's position; `tag` + `radius_m` + `bearing_deg`
     (+ `dy_m`) builds a temporary `PoseSpec`; `point` is used as it is.
2. `crates/lab/src/uat/runner/mod.rs`: `RowCtx.poses` (filled from
   `row.pose.clone()` in `run_row`), and `mod targets;`.
3. `crates/lab/src/uat/runner/actions.rs` `exec_tool`: right after the
   `unresolved` check, `if name == STAND_TOOL { return self.exec_stand(..).await }`,
   then `let args = match self.expand_target_args(who, &name, args, ctx,
   rec).await { Ok(a) => a, Err(e) => return fail(rec, e) };` and set
   `rec.args = args.clone()` so the evidence shows what the tool got.
4. `crates/lab/src/uat/runner/checks.rs` `check_action`: a tool action
   that is `STAND_TOOL` or whose args carry `pose`, `tag` or `face_tag`,
   with no server configured, BLOCKs the row: `"<tool> by tag needs the
   server lab MCP (CIMMERIA_LAB_MCP_URL/_TOKEN)"`. Blocking up front beats
   failing half-way through a row.

Tests (new `crates/lab/src/uat/runner/targets_tests.rs`, `#[cfg(test)]
mod targets_tests;` in `runner/mod.rs`; unit with `Fake` and the moved
`FakeServer`, both with `answer`):

- `a_pose_click_reaches_the_tagged_entity_in_the_players_space`: the fake
  client's `client_entity_find` reports player 8; the fake server answers
  `server_entity_get` with `space_id 3` and `server_entity_query` with the
  tagged entity (id 100751) and one with another tag; assert the query was
  sent with `space_id 3`. The row's `@world_click { pose = "frost" }`
  reaches `client_world_click` with `entity_id 100751` and `rotate_camera
  false`, and with no `pose` key (read `Fake.log`). Fails if the tag is
  passed through or the space is not used.
- `an_old_server_that_ignores_tag_is_filtered`: `server_entity_query`
  returns three entities of which one carries the tag: the click still gets
  that one.
- `zero_or_two_matches_fail_naming_the_tag`: both errors appear in the
  row's action error and the client tool was never called.
- `a_point_pose_clicks_above_the_origin`: aim point with `aim_dy_m 0.28`
  sends `point.y = position.y + 0.28` (within 1e-6).
- `stand_types_the_teleport_and_waits_for_arrival`: canned
  `client_entity_find` replies far, far, then within 0.3 m: the typed line
  is `/gmgotoxyz -325.00 73.60 -212.80` for the frost pose at the F20
  target, and the action is ok. A run whose replies never arrive fails
  with the "still ... m" error. Use `no_settle` (the request helper sets
  it) and keep the test under a second: make the poll interval a constant
  the test can rely on, or drive time with `tokio::time::pause` as other
  runner tests do (check `ability_tests.rs` first).
- `no_server_blocks_a_tag_row`: a row with a `pose` click and no server is
  BLOCKED with the "by tag needs the server lab MCP" reason, and the fake
  client saw no `client_world_click`.

Checks: the three lane commands for `-p cimmeria-lab`.

Docs owed: none here (SV-14).

Reviewer focus: no new action records (only `calls`); explicit args win
over the pose; tags never reach a client tool; the arrival check compares
server metres with server metres.

## SV-08 Server-event waits and polling clauses

**Implementer:** packet-coder. **Size:** M. **Wave:** 2. **Depends on:** SV-02, SV-03.
**Branch:** `lab-spec-vocab/sv08-server-waits`. **Worktree:** `sv08`.
**Subject:** `feat(lab): SV-08 wait on server log events and poll tool and server clauses`

Why: README F8, F9; D-SV5.

Files:

1. New `crates/lab/src/uat/runner/server_wait.rs` (`mod server_wait;` in
   `runner/mod.rs`): `exec_server_wait` (contract).
   - No server: fail "wait_server needs the server lab MCP".
   - Substitute `${var}`s in `fields` (`actions::subst`). A field whose
     value is exactly `${player_entity_id}` and is unset is resolved first
     with `self.player_entity(who, rec)` (SV-07) when that method exists;
     if SV-07 has not merged yet, resolve it inline with the same
     `client_entity_find` call and leave a `// SV-07: use player_entity`
     comment for the coordinator. Store the value in `ctx.vars`.
   - Watermark: the newest server timestamp *before the previous action
     ran*. The runner records it: in `drive_steps` (`runner/mod.rs`),
     before each action whose next action is a `wait_server`, call
     `server_log_tail { limit: 1 }` and keep `newest_ms` in a new
     `RowCtx.log_mark: Option<u64>`. A wait with no mark (first action of a
     row) takes one itself, at its own start.
   - Poll every 250 ms: `server_log_tail { target, contains: log, since_ms:
     mark, limit: 50 }`; filter the `entries` again by target, message
     substring and fields (numbers compare numerically, as packet clauses
     do: reuse that comparison from `runner/packet.rs` or `clause.rs`
     rather than writing a new one). The first match passes:
     `rec.result = {"matched": {"timestamp_ms", "message", "fields": <only
     the fields the wait named>}}`. Each poll advances `mark` to the newest
     timestamp it saw, so an entry is examined once.
   - Timeout: fail `"no server log '<log>' <fields> within <n> ms"`.
   - `rec.kind = "wait_server"`, `rec.requested = "wait for server log
     '<log>'"`, no tier (it drives nothing).
2. `crates/lab/src/uat/runner/actions.rs` `exec`: the `ServerWait` arm
   calls `self.exec_server_wait(..)` instead of failing.
3. `crates/lab/src/uat/runner/clauses.rs`: for `Source::Tool` and
   `Source::Server` clauses with `timeout_ms`, evaluate, and while the
   result is a FAIL (not UNVERIFIED, not an error reaching the tool) and the
   time is not up, sleep 250 ms and evaluate again. Record `"polls": n` in
   the clause's observed detail when n > 1. `no_settle` (tests) makes the
   sleep zero but keeps the loop.

Tests (new `crates/lab/src/uat/runner/server_wait_tests.rs`; unit with
`Fake` and `FakeServer`):

- `a_wait_passes_on_the_release_for_this_player`: canned
  `server_log_tail` replies: first `newest_ms 100` (the mark), then entries
  for `witness_id 9` (another player) at 110 and `witness_id 8` at 120. The
  wait with `fields = { witness_id = "${player_entity_id}" }` (player 8
  from the fake client) passes on the 120 entry. Fails if fields are not
  compared (it would pass on 110) or the mark is ignored.
- `an_entry_before_the_mark_does_not_count`: the only matching entry is at
  90 with mark 100: the wait times out with the error text.
- `a_string_number_matches_a_number`: field `"8"` in the log equals `8`.
- `a_server_clause_polls_until_it_passes` (in `server_wait_tests.rs` too):
  canned `server_db_query` answers `80623`'s predecessor twice then
  `80623`; a clause with `timeout_ms = 2000` passes with `polls 3`; the
  same clause without `timeout_ms` fails (evaluated once). That is README
  F9's race.
- `an_unverified_server_clause_does_not_poll`: an unreachable server gives
  UNVERIFIED after one call (assert one `server_db_query` call).

Checks: the three lane commands for `-p cimmeria-lab`.

Docs owed: none here (SV-14).

Reviewer focus: the watermark is taken before the triggering action, not
after it (a fast release would be missed); polling never turns UNVERIFIED
into PASS; no busy loop without a sleep outside `no_settle`.

## Calibration contract

New directory `crates/lab/src/uat/calibrate/` (`pub mod calibrate;` in
`uat/mod.rs`). SV-09 writes `mod.rs`, `grid.rs`, `score.rs`, `edit.rs`;
SV-10 writes `drive.rs` and `crates/lab/src/server/calibrate.rs`.

```rust
// calibrate/grid.rs
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    /// Tried in this order (D-SV6 tie-break follows it).
    pub radii_m: Vec<f64>,             // default [3.75, 3.0, 4.5]
    pub bearing_step_deg: f64,         // default 30.0 (12 bearings)
    pub yaw_offsets_deg: Vec<f64>,     // default [25.0, -25.0, 0.0]
    /// None = keep the pitch the face lands on (and record it).
    pub pitches_deg: Vec<Option<f64>>, // default [None, Some(-35.0), Some(-22.0)]
}
impl Default for Grid { /* the defaults above */ }
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate { pub radius_m: f64, pub bearing_deg: f64, pub yaw_offset_deg: f64, pub pitch_deg: Option<f64> }
/// The current pose (when it has one) first, then radius-major, bearing
/// ascending from the current pose's bearing rounded to the step (else 0),
/// then yaw, then pitch. Deterministic: the same inputs give the same list.
pub fn candidates(current: Option<&PoseSpec>, grid: &Grid) -> Vec<Candidate>;

// calibrate/score.rs
#[derive(Debug, Clone, PartialEq)]
pub struct Probe { pub ok: bool, pub jitter_hits: u32, pub avatar_px: f64, pub edge_px: f64 }
/// From a `client_hover_probe` result; None when a field is missing.
pub fn parse_probe(v: &serde_json::Value) -> Option<Probe>;
/// D-SV6: None unless ok; else jitter_hits*1000 + min(avatar_px,400) + min(edge_px,200)/2, rounded.
pub fn score(p: &Probe) -> Option<u32>;
/// A confirmation re-probe passes when ok and jitter_hits >= CONFIRM_MIN_HITS.
pub const CONFIRM_MIN_HITS: u32 = 6;
pub const CONFIRM_RUNS: u32 = 3;
/// Index of the best score; ties go to the lower index.
pub fn best(scores: &[Option<u32>]) -> Option<usize>;

// calibrate/edit.rs
/// Replace the one line holding pose `new.id` inside row `row_id` with
/// `new.to_inline_toml()`, keeping its indentation, trailing comma,
/// trailing comment and line ending (CRLF or LF). Errors: no such row, no
/// such pose line, or more than one.
pub fn rewrite_pose(text: &str, row_id: &str, new: &PoseSpec) -> Result<String, String>;
/// A unified diff (`--- a/<path>`, `+++ b/<path>`, `@@ -l,n +l,n @@`, two
/// lines of context) that `git apply` accepts. Empty when old == new.
pub fn line_diff(path: &str, old: &str, new: &str) -> String;

// calibrate/mod.rs
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct CalibrateOutcome {
    pub ok: bool,
    pub section: String,
    pub row: String,
    pub pose: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub score: Option<u32>,
    pub confirmed: u32,
    /// Only the fields that changed: `{ "bearing_deg": [322.5, 330.0] }`.
    #[serde(skip_serializing_if = "serde_json::Map::is_empty")] pub changed: serde_json::Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")] pub diff_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub spec_path: Option<String>,
    pub probes: u32,
    #[serde(skip_serializing_if = "Option::is_none")] pub reason: Option<String>,
}
```

---

## SV-09 Calibration core

**Implementer:** packet-coder. **Size:** M. **Wave:** 2. **Depends on:** SV-03.
**Branch:** `lab-spec-vocab/sv09-calibrate-core`. **Worktree:** `sv09`.
**Subject:** `feat(lab): SV-09 calibration candidates, hover scoring and pose-line spec edits`

Why: D-SV2, D-SV6. Everything in the sweep that needs no client, so SV-10
is only the driving.

Files: `crates/lab/src/uat/calibrate/mod.rs` (module doc: what calibration
is, D-SV2 and D-SV6 in two sentences each, and that nothing here clicks or
commits; `pub mod edit; pub mod grid; pub mod score;` and
`CalibrateOutcome`), `grid.rs`, `score.rs`, `edit.rs`, all as the
calibration contract; `crates/lab/src/uat/mod.rs` (`pub mod calibrate;`
and a doc bullet).

`rewrite_pose` works on lines: find `[[row]]` headers, take the block whose
first `id = "..."` line is `row_id`, and within it the lines whose trimmed
text starts with `{ id = "<pose id>"`. Then re-parse the whole new text
with `spec::parse` (with the `@alias` resolution it does) and check that
the row's parsed pose with that id equals `new` after its own round trip;
a mismatch is an error, so a bad edit is never written.

Tests (unit, type 1, one `mod tests` per file):

- `grid.rs`: `default_grid_has_324_candidates_in_a_fixed_order` (3 x 12 x 3
  x 3, the first is the current pose when given, two calls agree);
  `bearings_start_at_the_current_pose` (current 322.5 gives a first sweep
  bearing of 330.0, then 0.0, 30.0 ... wrapping, each once).
- `score.rs`: `a_miss_scores_none`; `jitter_dominates` (8 hits at the
  avatar's edge beats 7 hits far from it); `ties_go_to_the_first`; the
  formula against two hand-computed values.
- `edit.rs`:
  - `rewrites_only_the_pose_line`: a two-row spec with poses in both
    rows; rewriting row 2's `frost` changes exactly one line, and row 1's
    `frost` line is untouched. Fails if the row scoping is lost.
  - `keeps_crlf_and_the_trailing_comment`: a CRLF input with
    an indented `{ id = "a", ... }, # keep me` line keeps its indentation,
    `\r\n` and the comment.
  - `refuses_a_missing_or_duplicated_pose_line`.
  - `the_diff_applies`: `line_diff` of a one-line change has the exact
    expected text (hunk header with counts, two context lines each side).
    Write the expected string out in the test.
- Each test fails if its rule is reverted: the edit tests by changing more
  than one line, the score tests by a changed weight.

Checks: the three lane commands for `-p cimmeria-lab`.

Docs owed: none here (SV-14).

Reviewer focus: no I/O in these modules; `rewrite_pose` cannot write a file
that fails to parse; the candidate order is total (no float sort ties left
to chance).

## SV-10 lab_uat_calibrate

**Implementer:** rust-gameserver-dev. **Size:** L. **Wave:** 3. **Depends on:** SV-05, SV-06, SV-07, SV-09, D-SV7.
**Branch:** `lab-spec-vocab/sv10-calibrate-drive`. **Worktree:** `sv10`.
**Subject:** `feat(lab): SV-10 lab_uat_calibrate sweeps stand-offs and camera poses by hover`

Why: the handoff's `lab calibrate`, as a daemon tool the CLI (SV-12) and
`-Heal` call. D-SV6, D-SV7, D-SV12.

Files:

1. New `crates/lab/src/uat/calibrate/drive.rs`:

   ```rust
   pub struct CalibrateRequest {
       pub spec: LoadedSpec,
       pub row: String,
       /// None: every pose a `client_world_click` step of the row names, in step order.
       pub pose: Option<String>,
       pub grid: Grid,
       pub budget: std::time::Duration,   // default 15 min, at most 30
       pub out_dir: PathBuf,               // the calibrate run folder
       pub stamp: String,                  // "2026-10-12" + server version when known
   }
   pub async fn calibrate<I: ToolInvoker>(runner: &Runner<'_, I>, req: &CalibrateRequest) -> Vec<CalibrateOutcome>;
   ```

   It drives through the `Runner`'s SV-07 methods (`resolve_tag`,
   `stand_at`, `player_entity`) with a scratch `ActionRecord` per call, so
   there is one implementation of resolution and standing. Per pose:

   1. Refuse, with `reason`, when the section's `account` is not `gm`, no
      server is configured, or `lab_client_status` is not `in_world`.
   2. `resolve_tag` the pose's tag: the target position.
   3. For each `grid::candidates(Some(&pose), &grid)` until the budget is
      spent: `stand_at(stand_point)` when the stand point changed (a
      failed stand marks every candidate at that point a miss and moves
      on); `client_camera` with the face target (entity or the aim point),
      `pitch_deg` (when the candidate has one), `yaw_offset_deg` and
      `zoom` (the pose's, else 250); when the camera call fails, a miss;
      else `client_hover_probe` with the entity id or the aim point
      (`expect_entity` the target's id). Score with `score::score`.
      Append one compact JSON line per candidate to
      `<out_dir>/calibrate.jsonl`: `{i, r, b, yaw, pitch, ok, hits, score}`
      (no nulls).
   4. `best`, then confirm `CONFIRM_RUNS` times: stand at the opposite
      bearing (b + 180), then back at the winner, camera, probe; each
      must pass `CONFIRM_MIN_HITS`. A failed confirmation drops that
      candidate and tries the next best, at most 3 winners.
   5. The new pose: radius and bearing from the winner, `pitch_deg` from
      the camera result's `absolute.pitch.got` (or the face's resulting
      readout when the candidate kept the face pitch), `yaw_offset_deg`,
      `zoom`, `calibrated = "<stamp> <n>/<n>"`. `edit::rewrite_pose` on the
      spec file's text, then write `<out_dir>/<file name>` and
      `<out_dir>/<section>.diff` (`line_diff` with the spec's repo-relative
      path), and fill the outcome. No pose passed: `ok: false`, `reason:
      "no candidate hovered the target (<probes> probes)"`.

   It never calls `client_world_click`, `client_target` or a button, and
   never writes the spec file itself.
2. New `crates/lab/src/server/calibrate.rs` (registered next to the UAT
   tools; read how `server/uat.rs` registers `lab_uat_run` and do the
   same): `lab_uat_calibrate { section, row, pose?, specs_dir?, instance?,
   budget_minutes?, radii_m?, bearing_step_deg? }`. It takes and releases
   its own lease exactly as `lab_uat_run` does (`server/uat.rs:340` on;
   make `RouterInvoker` and the lease helpers `pub(crate)` rather than
   copy them), builds a `Runner` over the same invokers with a
   `RunRequest` whose `root` is the UAT runs folder, and returns the
   outcomes as `{ "outcomes": [...] }` (compact, D-SV12).
3. `crates/lab/src/lease/policy.rs`: `"lab_uat_calibrate"` joins
   `OWN_LEASE`, and its test.
4. If D-SV7 is (b): refuse when the server lab MCP URL is not a loopback
   or private address, with the reason "calibration runs on a local server
   only (D-SV7)". If (a): no guard.

Tests (new `crates/lab/src/uat/calibrate/drive_tests.rs`; unit with `Fake`
and `FakeServer` canned replies, a 2 x 4 grid with one yaw and one pitch):

- `the_best_hovering_candidate_wins_and_is_confirmed`: canned
  `client_hover_probe` replies make candidate 5 the only 8-hit one; the
  outcome's `changed` has candidate 5's bearing, `confirmed 3`, and the
  written spec copy parses with that pose. Fails if the score is ignored
  (candidate 0 would win).
- `nothing_is_clicked`: `Fake.log` holds no `client_world_click`,
  `client_target`, `client_input_mouse` call.
- `a_failed_confirmation_falls_to_the_next_best`.
- `the_budget_stops_the_sweep_with_the_best_so_far` (a zero budget after
  the first stand: the outcome says `reason` "budget" and still proposes
  the best probed candidate when it passed).
- `a_non_gm_section_is_refused_before_anything_moves`.
- `crates/lab/src/server/lease_tests.rs`: `lab_uat_calibrate` without a
  lease id takes its own and releases it (mirror the `lab_uat_run` test at
  `lease_tests.rs:209-230`).

Checks: the three lane commands for `-p cimmeria-lab`.

Docs owed: none here (SV-14).

Reviewer focus: no click path; the lease is released on every exit; the
diff is written to the run folder, never over the spec; the budget is
checked between candidates, not only between stands.

## SV-11 Heal hint on a row summary

**Implementer:** packet-coder. **Size:** S. **Wave:** 3. **Depends on:** SV-07.
**Branch:** `lab-spec-vocab/sv11-heal-hint`. **Worktree:** `sv11`.
**Subject:** `feat(lab): SV-11 flag hover-miss failures with the pose to recalibrate`

Why: README F15; D-SV8.

Files:

1. New `crates/lab/src/uat/runner/heal.rs`:
   `pub(crate) fn heal_hint(ev: &RowEvidence) -> Option<HealHint>`. The
   first step action that failed (`role == Step`, `!ok`, not optional)
   decides: when its tool is `client_world_click`, `client_target` or
   `client_camera`, its `calls` hold a `resolve` entry with a `pose`
   (SV-07), and its result's `error_data.step` (`record_outcome` puts the
   world tool's `WorldError::to_json` there) is one of `hover`,
   `on_screen`, `face`, `abs_pitch`, `abs_yaw`: `Some(HealHint { pose,
   step })`. Anything else: `None`.
2. `crates/lab/src/uat/runner/mod.rs`: `HealHint` and `RowSummary.heal`
   (contract); `run_all` fills it with `heal::heal_hint(r)` for non-PASS
   rows.

Tests (unit, in `heal.rs`'s `mod tests`, on hand-built `RowEvidence`):

- `a_hover_miss_on_a_pose_click_names_the_pose`.
- `a_miss_without_a_pose_has_no_hint` (a `name` click cannot be
  recalibrated).
- `a_mission_clause_failure_has_no_hint` (all actions ok).
- `the_first_failure_decides` (a failed `@stand` before the click gives no
  hint).
- A runner test in `runner/targets_tests.rs`: a scripted
  `client_world_click` failure (`__error` with `__data.step = "hover"`)
  puts `heal: {pose: "frost", step: "hover"}` in `RunOutcome.rows`, and a
  passing row's summary serializes without a `heal` key.

Checks: the three lane commands for `-p cimmeria-lab`.

Docs owed: none here (SV-14).

## SV-12 lab calibrate and lab uat -Heal

**Implementer:** packet-coder. **Size:** M. **Wave:** 4. **Depends on:** SV-10, SV-11, D-SV8.
**Branch:** `lab-spec-vocab/sv12-calibrate-cli`. **Worktree:** `sv12`.
**Subject:** `feat(lab): SV-12 lab calibrate and lab uat -Heal propose spec diffs`

Why: the handoff's two commands. D-SV8, D-SV12. Written for D-SV8 (a); for
(b), `-Heal` only prints `lab calibrate <section> <row> -Pose <pose>` per
hint and exits 5, and the calibrate call below is dropped.

Files:

1. New `tools/lab/cli/calibrate.ps1` (auto-discovered by `lab.ps1`):
   `lab calibrate <section> <row> [-Pose <id>] [-Instance <name>]
   [-Prepare <row ids>] [-BudgetMinutes 1-30] [-Apply] [-Json] [-Quiet]`.
   - Comment-based help in the same shape as `uat.ps1`'s (synopsis,
     description, exit codes, examples).
   - Pre-flight as `uat.ps1` does it (daemon answering, the instance free,
     no foreign SGW.exe), reusing `uat-lib.ps1`'s functions by dot-sourcing
     it; argument checks first (exit 2).
   - `-Prepare`: one `lab_uat_run` on the instance with those rows first
     (the client then stands in world where the row starts); a prepare row
     that does not PASS stops with exit 3 naming it.
   - Then `lab_uat_calibrate`. Print one line per outcome (`frost: score
     8312, confirmed 3/3, bearing 322.5 -> 330.0; diff <path>`), then the
     diff text unless `-Quiet`.
   - `-Apply`: refuse (exit 2) when the spec file has uncommitted changes
     (`git -C <repo> diff --quiet -- <file>`); else run `git apply` with
     the diff in the checkout. Never `git add` or commit.
   - `-Json`: one object, under about 400 characters: `{ok, section, row,
     outcomes: [{pose, ok, score, confirmed, changed, diff}]}`, no nulls.
   - Exit codes: 0 every pose has a proposal (or is unchanged); 1 a pose
     found no candidate; 2 usage; 3 pre-flight or prepare failed.
   - Testable pieces go in new `tools/lab/cli/calibrate-lib.ps1`:
     `Test-CalibrateArguments`, `ConvertTo-CalibrateCompactJson`,
     `Format-CalibrateLine`.
2. `tools/lab/cli/uat.ps1` and `uat-lib.ps1`: a `-Heal` switch.
   - `-Heal` with `-Leases` above 1 is a usage error (exit 2).
   - After the batch (inside the existing `finally`, after the summary is
     written): `Get-UatHealHints $allRuns` returns the distinct
     `(row, pose)` pairs from the rows' `heal` fields. For each, call
     `lab_uat_calibrate` on the lane's instance, save the diff as
     `heal-<row>-<pose>.diff` in the batch folder, and print it.
   - The summary (`batch.json`, `summary.md`, the `-Json` object) gains
     `heal: [{row, pose, diff}]` only when a proposal was written.
   - Exit 5 when a proposal was written, else the batch's own code. Add 5
     to the help's exit-code list.

Tests (PowerShell check scripts; `lab.yml` runs every `test-*.ps1`):

- New `tools/lab/cli/test-calibrate.ps1`: the argument checks (no row,
  budget 0 and 31 refused, `-Apply` with `-Json` allowed);
  `ConvertTo-CalibrateCompactJson` on two outcomes is under 400 characters
  and contains no `null`; `Format-CalibrateLine` for a changed and an
  unchanged pose.
- `tools/lab/cli/test-uat.ps1`: `Get-UatHealHints` dedupes two runs'
  identical hints and ignores rows without one; `Test-UatArguments` (or
  the new heal check) refuses `-Heal -Leases 2`; `ConvertTo-UatCompactJson`
  leaves out `heal` when there is none.

Checks: `pwsh -NoProfile -File tools/lab/cli/test-calibrate.ps1` and
`pwsh -NoProfile -File tools/lab/cli/test-uat.ps1`. No cargo.

Docs owed: none here (SV-14 writes the `lab` command rows).

Reviewer focus: nothing commits; `-Apply` cannot clobber local edits;
`-Json` output stays compact on errors too (`{"ok":false,"exit":N,"error":...}`).
