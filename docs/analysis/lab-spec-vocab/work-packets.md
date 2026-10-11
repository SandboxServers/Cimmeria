# Lab spec vocabulary: work packets (1 of 3)

> Type: work packets. Audience: `packet-coder` workers (Haiku),
> `rust-gameserver-dev` for SV-07 and SV-10, `documentation-writer` for SV-14
> and SV-16, and `packet-reviewer` reviewers (Sonnet). Ledger, findings (F1
> to F20) and decisions (D-SV1 to D-SV12): [README.md](README.md). Packets
> SV-06 to SV-12 and the calibration contract:
> [work-packets-2.md](work-packets-2.md); SV-13 to SV-16:
> [work-packets-3.md](work-packets-3.md).
>
> PowerShell only. Every compiling command goes through the lane, in this
> order, from the worktree root, with `-p` the packet's crate:
>
> ```powershell
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo fmt -p cimmeria-lab
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo clippy -p cimmeria-lab --all-targets -- -D warnings
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab
> ```
>
> Read the lane's summary and its failures file; do not rerun a build to
> see the output. No packet uses the lab, a lab MCP tool or a running client
> (SV-15 is the one exception, and only with the user's OK).
>
> Rust rules for every packet: no `unwrap()` outside tests; comments say
> why, not what; `#[cfg(test)] mod tests` last in a file, or a sibling
> `*_tests.rs` file through `#[path]` as the crate already does; no new file
> over 500 lines; test names say what they prove. Every new JSON result
> follows the lab's compact rule: no nulls, no empty arrays, floats to 2
> decimals.

## Contents

- [Contract](#contract)
- [SV-01 Server entity query by tag](#sv-01-server-entity-query-by-tag)
- [SV-02 server_log_tail filters](#sv-02-server_log_tail-filters)
- [SV-03 Spec schema](#sv-03-spec-schema)
- [SV-04 Seed tag index and spec tag validation](#sv-04-seed-tag-index-and-spec-tag-validation)
- [SV-05 Absolute camera](#sv-05-absolute-camera)
- SV-06 to SV-12: [work-packets-2.md](work-packets-2.md); SV-13 to SV-16: [work-packets-3.md](work-packets-3.md)

## Contract

Parallel packets, and the `lab-record` and `lab-fixtures` campaigns, build
against these names. A packet that needs to change one stops and tells the
coordinator.

### TOML a spec writes

```toml
[[row]]
id = "FS-P3"
# One pose per line, canonical key order (D-SV2). `lab calibrate` rewrites
# these lines and nothing else.
pose = [
  { id = "frost", tag = "ArmYourself_FrostBody", radius_m = 4.16, bearing_deg = 322.5, dy_m = 0.13, pitch_deg = -28.8, yaw_offset_deg = 25.3, zoom = 250.0, calibrated = "2026-10-10 hand" },
]
setup = [
  { tool = "@finish_dialog", args = {} },
  { chat = "/gmsetgodmode 1", tier = "G" },
  { tool = "@stand", args = { pose = "frost" }, label = "stand" },
]
step = [
  { tool = "@camera", args = { pose = "frost" }, label = "aim" },
  { tool = "@world_click", args = { pose = "frost", button = "right", expect = "window" }, label = "frost" },
  { tool = "@finish_dialog", args = {}, label = "close" },
]

[[row.expect]]
id = "step-80623"
text = "sgw_mission: 622 on step 80623"
source = "server"
tool = "server_db_query"
args = { sql = "SELECT ..." }
pointer = "/rows/0/current_step_id"
op = "eq"
value = 80623
timeout_ms = 5000          # NEW for tool/server clauses: poll until it passes

# A point-aimed pose (a body that never answers an entity hover):
#   { id = "guard", tag = "ArmYourself_GuardBody", radius_m = 4.41, bearing_deg = 322.7, dy_m = 0.13, pitch_deg = -28.8, yaw_offset_deg = 25.3, zoom = 250.0, aim = "point", aim_dy_m = 0.28 },

# A server-event wait (an action kind of its own):
#   { wait_server = { log = "Cinematic AoI hold: released", target = "aoi.cinematic_hold", fields = { witness_id = "${player_entity_id}" }, timeout_ms = 30000 }, label = "movie-over" },
```

Tag arguments without a pose are also valid:
`@world_click { tag = "...", aim = "point", aim_dy_m = 0.28 }`,
`@entity_find { tag = "..." }`, `@camera { face_tag = "...", pitch_deg = -28.8, yaw_offset_deg = 25.3, zoom = 250 }`,
`@stand { tag = "...", radius_m = 4.2, bearing_deg = 322.5 }` or
`@stand { point = { x = -325.0, y = 73.6, z = -212.8 } }`. Relative camera
counts (`pitch_counts`, `yaw_counts`) keep working.

### `crates/lab/src/uat/spec.rs`: the lab-spec-vocab block (SV-03)

Every addition sits between `// lab-spec-vocab (SV-03)` and
`// end lab-spec-vocab` comments, so `lab-fixtures` can add its `fixture`
field after the block without a conflict.

```rust
// RowSpec gains, after `evidence`:
    // lab-spec-vocab (SV-03)
    /// Interaction poses: where to stand and how to aim at a tagged target.
    #[serde(default)]
    pub pose: Vec<PoseSpec>,
    // end lab-spec-vocab

// ActionSpec gains, after `capture`/`regex`/`var`:
    // lab-spec-vocab (SV-03)
    /// Wait for a server event (a log line) instead of sleeping.
    #[serde(default)]
    pub wait_server: Option<ServerWait>,
    // end lab-spec-vocab

pub enum ActionKind { Tool, Chat, Wait, Capture, ServerWait }   // ServerWait is new

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AimMode {
    /// Hover-verified on the entity (`Unit.MouseOver` is the target).
    #[default]
    Entity,
    /// A world point `aim_dy_m` above the target's origin; for bodies that
    /// never answer an entity hover (README F13).
    Point,
}

#[derive(Debug, Clone, PartialEq, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct PoseSpec {
    /// Row-unique, `[a-z0-9-]+`; actions name it with `pose = "<id>"`.
    pub id: String,
    /// `spawnlist.tag` of the target (seed-checked, SV-04).
    pub tag: String,
    /// Horizontal stand-off from the target, metres (1.0 to 8.0).
    pub radius_m: f64,
    /// From the target, server x/z plane, 0 = +x, 90 = +z (0 <= b < 360).
    pub bearing_deg: f64,
    /// Stand height above the target's origin, metres (default 0.1).
    #[serde(default = "default_dy_m")]
    pub dy_m: f64,
    /// Absolute camera pitch after facing, readout degrees (D-SV4).
    #[serde(default)]
    pub pitch_deg: Option<f64>,
    /// Yaw after facing, degrees in the +yaw_counts direction (D-SV4).
    #[serde(default)]
    pub yaw_offset_deg: Option<f64>,
    /// Camera distance, 100 to 775.
    #[serde(default)]
    pub zoom: Option<f64>,
    #[serde(default)]
    pub aim: AimMode,
    /// `aim = "point"` only: metres above the target's origin.
    #[serde(default)]
    pub aim_dy_m: Option<f64>,
    /// Provenance, written by `lab calibrate` ("2026-10-12 5/5 b6a00db78")
    /// or by `lab record` ("recorded 2026-10-12").
    #[serde(default)]
    pub calibrated: Option<String>,
}

impl PoseSpec {
    /// D-SV3: `target + radius * (cos b, 0, sin b)`, `y = target.y + dy_m`.
    pub fn stand_point(&self, target: [f64; 3]) -> [f64; 3];
    /// The aim point for `aim = "point"`: target with `y + aim_dy_m`.
    pub fn aim_point(&self, target: [f64; 3]) -> Option<[f64; 3]>;
    /// One line, keys in this order, `None` fields left out:
    /// id, tag, radius_m (2 dp), bearing_deg (1 dp), dy_m (2 dp),
    /// pitch_deg (1 dp), yaw_offset_deg (1 dp), zoom (1 dp), aim (only
    /// when "point"), aim_dy_m (2 dp), calibrated.
    /// Shape: `{ id = "frost", tag = "...", radius_m = 4.16, ... }`.
    pub fn to_inline_toml(&self) -> String;
}

#[derive(Debug, Clone, PartialEq, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServerWait {
    /// A substring of the log message (`server_log_tail` `message`).
    pub log: String,
    /// The tracing target, exact (`aoi.cinematic_hold`).
    #[serde(default)]
    pub target: Option<String>,
    /// Every field must equal (numbers numerically, `${var}`s substituted).
    #[serde(default)]
    pub fields: Option<serde_json::Map<String, Value>>,
    /// Default 30000; at most 120000.
    #[serde(default = "default_server_wait_ms")]
    pub timeout_ms: u64,
}
```

`ExpectSpec.timeout_ms` keeps its field and its meaning for `wait` and
`client_event`; for `tool` and `server` clauses it now means "re-evaluate
every 250 ms until the clause passes or the time is up" (at most 60000).
Its doc comment says so.

### Validation rules (`spec_validate.rs`, SV-03)

Each problem names its row, as today:

- pose ids unique in the row and `[a-z0-9-]+`; `tag` non-empty and
  `[A-Za-z0-9_.]+`; `radius_m` in 1.0..=8.0; `bearing_deg` in 0.0..360.0;
  `dy_m` in -3.0..=3.0; `pitch_deg` in -78.75..=78.75; `yaw_offset_deg` in
  -180.0..=180.0; `zoom` in 100.0..=775.0; `aim = "point"` needs `aim_dy_m`,
  `aim = "entity"` forbids it;
- an action arg `pose = "<id>"` names a pose of this row, and only on
  `client_camera`, `client_world_click`, `client_target`,
  `client_entity_find` or `uat_stand` (after `@` resolution);
- `tag` only on `client_world_click`, `client_target`, `client_entity_find`
  and `uat_stand`; `face_tag` only on `client_camera`; neither together with
  `name`, `entity_id`, `point` (`face_name`, `face_entity_id`, `face_point`)
  or `pose`;
- `wait_server.log` non-empty and `timeout_ms` in 1..=120000;
- `timeout_ms` on a `tool` or `server` clause at most 60000;
- `ActionSpec::kind` counts five kinds (tool, chat, wait_ms, capture,
  wait_server): exactly one.

### Capability table (`tools.rs`, SV-03)

```rust
/// What `@stand` resolves to: a runner action, not a router tool (SV-07).
pub const STAND_TOOL: &str = "uat_stand";
// in CAPABILITIES, a new block "Spec vocabulary (lab-spec-vocab)":
drive("stand", STAND_TOOL, Tier::G),
drive("hover_probe", "client_hover_probe", Tier::N1),
```

### Runner names (SV-07, SV-08, SV-11)

```rust
// runner/targets.rs (SV-07)
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedTag { pub tag: String, pub entity_id: u32, pub space_id: u32, pub position: [f64; 3] }
impl<I: ToolInvoker> Runner<'_, I> {
    /// The client's own player entity id (`client_entity_find` `/player/id`).
    pub(crate) async fn player_entity(&self, who: Who, rec: &mut ActionRecord) -> Result<u32, String>;
    /// D-SV1: the one entity with `tag` in the player's space.
    pub(crate) async fn resolve_tag(&self, who: Who, tag: &str, rec: &mut ActionRecord) -> Result<ResolvedTag, String>;
    /// `pose` / `tag` / `face_tag` arguments rewritten to plain ones.
    pub(crate) async fn expand_target_args(&self, who: Who, tool: &str, args: Value,
        ctx: &RowCtx, rec: &mut ActionRecord) -> Result<Value, String>;
    /// D-SV11: `/gmgotoxyz`, then wait for arrival. `lab-fixtures` reuses it.
    pub(crate) async fn stand_at(&self, who: Who, point: [f64; 3], rec: &mut ActionRecord) -> Result<(), String>;
}
// RowCtx gains `pub poses: Vec<PoseSpec>` (filled from row.pose in run_row).

// runner/server_wait.rs (SV-08)
impl<I: ToolInvoker> Runner<'_, I> {
    pub(crate) async fn exec_server_wait(&self, w: &ServerWait, who: Who, ctx: &mut RowCtx, rec: &mut ActionRecord);
}

// runner/mod.rs (SV-11)
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealHint { pub pose: String, pub step: String }
// RowSummary gains: #[serde(skip_serializing_if = "Option::is_none")] pub heal: Option<HealHint>,
```

Every server read the runner makes for a tag goes into the action's
`calls` array as `{ "tool", "args", "ok" }`, and the resolution as
`{ "resolve": { "tag", "entity_id", "space_id" } }`. None of them becomes a
separate action record, so grading and action indices do not change.

### Client tools (SV-05, SV-06)

```rust
// supervisor/world/camera.rs: CameraRequest gains
    /// Absolute pitch after any face, readout degrees (D-SV4).
    #[serde(default)] pub pitch_deg: Option<f64>,
    /// Yaw after any face, degrees in the +yaw_counts direction.
    #[serde(default)] pub yaw_offset_deg: Option<f64>,
    /// Camera distance; the same as `zoom_to` (an error to give both).
    #[serde(default)] pub zoom: Option<f32>,
pub const ABS_TOLERANCE_DEG: f64 = 0.5;
pub const ABS_MAX_LOOKS: u32 = 4;
// result gains "absolute": { "pitch": { "want", "got", "looks" }, "yaw": { "want", "got", "looks", "open_loop"? } }

// supervisor/world/memory.rs
impl CameraState { pub fn yaw_gain(self) -> f32; }   // -gain when flag 0x4 is set

// supervisor/world/hover.rs (SV-06): `client_hover_probe`
#[derive(Debug, Clone, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct HoverRequest {
    #[serde(default)] pub entity_id: Option<u32>,   // entity aim
    #[serde(default)] pub point: Option<PointArg>,  // point aim
    /// Entity aim with a point: hover must be this entity at the point.
    #[serde(default)] pub expect_entity: Option<u32>,
    /// Jitter ring radius, px (default 12).
    #[serde(default)] pub jitter_px: Option<u32>,
}
// WorldIo gains: async fn window_rects(&mut self) -> Result<Vec<(String, [f64; 4])>, String>;
```

---

## SV-01 Server entity query by tag

**Implementer:** packet-coder. **Size:** S. **Wave:** 1. **Depends on:** none.
**Branch:** `lab-spec-vocab/sv01-entity-query-tag`. **Worktree:** `sv01`.
**Subject:** `feat(lab-mcp): SV-01 server_entity_query filters by spawn tag`

Why: README F2; D-SV1.

Files:

1. `crates/wire/src/cell/messages/lab.rs`: `LabEntityFilter` gains
   `pub tag: Option<String>` with the doc "Restrict to entities whose spawn
   `tag` equals this, exactly (case sensitive)." Update the struct doc's
   field list.
2. `crates/cell-world/src/cell/space_manager/lab_snapshots.rs`
   `lab_query_entities`: after the `class_id` test, skip an entity whose tag
   is not `filter.tag`. Find the entity's tag field the same way
   `snapshot_entity` fills `LabEntitySnapshot.tag` (read that function; do
   not guess the field name).
3. `crates/lab-mcp/src/tools/mod.rs`: `EntityQueryArgs` gains
   `#[serde(default)] tag: Option<String>` ("Restrict to entities with this
   spawn tag (`spawnlist.tag`), exact."); add `"tag"` to the audit JSON in
   `server_entity_query`; pass it on; add "tag" to the tool description's
   filter list.
4. `crates/lab-mcp/src/tools/entities.rs` `entity_query`: a `tag:
   Option<String>` parameter, set on the filter.
5. Every `LabEntityFilter { .. }` literal that does not use
   `..Default::default()`: `crates/cell/src/cell/service/base_messages/tests/lab_query.rs`
   (lines about 112, 138, 165, 200) gains `tag: None`. Grep
   `LabEntityFilter {` to find any other.

Tests:

- `crates/cell/src/cell/service/base_messages/tests/lab_query.rs` (unit,
  type 1), `lab_query_filters_by_tag`: two NPCs in one space with tags
  `"A_Body"` and `"B_Body"` (build them the way the file's existing
  template-filter test does), query `tag: Some("A_Body")`: exactly one
  snapshot, its `tag` is `A_Body`, `total_matched == 1`. Then
  `tag: Some("a_body")` matches nothing (case sensitive). Fails if the
  filter is dropped (both match).
- `crates/lab-mcp/src/tools/entities_tests.rs`: if the file tests argument
  plumbing, add the tag to that test; otherwise none.

Checks: the lane's `fmt` / `clippy` / `nextest` for `-p cimmeria-wire`,
`-p cimmeria-cell-world`, `-p cimmeria-cell` and `-p cimmeria-lab-mcp`.

Docs owed: none here (SV-14 adds the argument to the lab guide's tool table).

Reviewer focus: the tag is compared exactly; a `None` filter changes
nothing; no snapshot field changed.

## SV-02 server_log_tail filters

**Implementer:** packet-coder. **Size:** S. **Wave:** 1. **Depends on:** none.
**Branch:** `lab-spec-vocab/sv02-log-tail-filters`. **Worktree:** `sv02`.
**Subject:** `feat(lab-mcp): SV-02 server_log_tail filters by target, message text and time`

Why: README F8; D-SV5.

Files:

1. `crates/lab-mcp/src/tools/logs.rs` `log_tail`: becomes
   `log_tail(state, &LogFilter, limit)` with

   ```rust
   #[derive(Debug, Clone, Default)]
   pub struct LogFilter {
       pub level: Option<String>,
       /// Exact tracing target (`aoi.cinematic_hold`).
       pub target: Option<String>,
       /// Case-sensitive substring of the message.
       pub contains: Option<String>,
       /// Only entries with `timestamp_ms` strictly greater than this.
       pub since_ms: Option<u64>,
   }
   ```

   Apply all filters (AND) before taking the last `limit`. Move the
   filtering into `pub fn filter_entries(entries: Vec<LogEntry>, f:
   &LogFilter) -> Vec<LogEntry>` so it is testable without a `LabState`
   (check the entry type's name and module in
   `crates/admin-api/src/ws/broadcast_layer/mod.rs:56`). The reply gains
   `"newest_ms"`: the largest `timestamp_ms` in the whole buffer, before
   filtering, or left out when the buffer is empty. That is the watermark a
   wait starts from (D-SV5).
2. `crates/lab-mcp/src/tools/mod.rs` `LogTailArgs` gains `target`,
   `contains`, `since_ms` (`#[serde(default)]`, documented); the audit JSON
   and the description ("optionally filtered by level, target, message text
   and since_ms; newest_ms is the buffer's newest timestamp") follow.

Tests (new `crates/lab-mcp/src/tools/logs_tests.rs`, `#[path]` from
`logs.rs`; unit, type 1):

- `filters_combine_target_text_and_time`: five entries (two targets, three
  messages, timestamps 10 to 50); `target = aoi.cinematic_hold`,
  `contains = "released"`, `since_ms = 20` keeps exactly the entry at 40.
  Fails if any filter is ignored (more entries pass).
- `since_is_strictly_after`: `since_ms = 40` drops the entry at 40.
- `newest_ms_ignores_filters`: a filter that matches nothing still reports
  `newest_ms = 50`.

Checks: the lane commands for `-p cimmeria-lab-mcp`.

Docs owed: none here (SV-14).

Reviewer focus: the existing `level` behaviour is unchanged; `newest_ms` is
taken before filtering.

## SV-03 Spec schema

**Implementer:** packet-coder. **Size:** M. **Wave:** 1. **Depends on:** none.
**Branch:** `lab-spec-vocab/sv03-spec-schema`. **Worktree:** `sv03`.
**Subject:** `feat(lab): SV-03 spec schema for poses, server waits and tag arguments`

Why: the vocabulary every later packet and campaign builds on. README F1,
F9; D-SV2, D-SV3, D-SV10.

Files:

1. `crates/lab/src/uat/spec.rs`: the lab-spec-vocab block exactly as the
   contract (`AimMode`, `PoseSpec` with its three methods, `ServerWait`,
   `RowSpec.pose`, `ActionSpec.wait_server`, `ActionKind::ServerWait`,
   `default_dy_m` = 0.1, `default_server_wait_ms` = 30000). `kind()` counts
   five fields; its error texts name all five. Extend the `timeout_ms` doc.
   `to_inline_toml` writes strings with `toml`'s own escaping (serialize a
   one-key table with the `toml` crate, or escape `\` and `"` by hand) and
   floats with `format!("{:.N}")` at the contract's decimals and no other
   trimming (so `zoom = 250.0`, `radius_m = 4.16`).
2. `crates/lab/src/uat/spec_validate.rs`: the contract's rules, in a new
   `fn check_poses(row, errs)` and `fn check_target_args(row, a, errs)`,
   called from `validate`'s row loop. `check_clause`: a `tool` or `server`
   clause's `timeout_ms` over 60000 is an error.
3. `crates/lab/src/uat/tools.rs`: `STAND_TOOL` and the two capability rows
   (contract). Extend `an_alias_resolves_and_an_unknown_one_is_an_error`
   with `resolve("@stand") == "uat_stand"` and `lookup("@stand").floor ==
   Some(Tier::G)`.
4. The runner's exhaustive matches on `ActionKind`: in
   `runner/actions.rs` `exec`, `Ok(ActionKind::ServerWait) => { rec.kind =
   "wait_server".into(); fail(&mut rec, "wait_server: not implemented yet
   (SV-08)".into()) }`; in `runner/checks.rs` `check_action`,
   `Ok(ActionKind::ServerWait) => if self.server.is_none() { out.push("wait_server needs the server lab MCP (CIMMERIA_LAB_MCP_URL/_TOKEN)".into()) }`.
   `action_available` already returns true for it.
5. `runner/players.rs` `routed`: `STAND_TOOL` is routed when chat can be
   sent (the same test as the lab commands, without the reader) and
   `client_entity_find` is routed.
6. Test support for SV-07 and SV-08, so they need not both edit it:
   - `runner/tests.rs` `Fake` gains `canned: Mutex<HashMap<String,
     VecDeque<Value>>>` and `pub(super) fn answer(self, tool: &str,
     replies: Vec<Value>) -> Self`. `call` pops the next canned reply for
     that tool (the last one repeats) before its existing handling, and
     still logs the call. A canned reply of the shape `{ "__error": "<msg>",
     "__data": <json> }` becomes a failed `ToolOutcome` with that `error`
     and `error_data` (SV-10 and SV-11 script world-tool failures with it).
   - Move `FakeServer` from `runner/packet_tests.rs` into a new
     `runner/test_server.rs` (`#[cfg(test)] mod test_server;` in
     `runner/mod.rs`), `pub(super)`, unchanged, plus `canned:
     Mutex<HashMap<String, VecDeque<Value>>>` and `pub(super) fn
     answer(self, tool, replies) -> Self` with the same pop rule, checked
     before its `match name`. `packet_tests.rs` imports it.

Tests (`crates/lab/src/uat/spec_tests.rs`, unit, type 1):

- `a_pose_row_parses_and_its_stand_point_follows_the_bearing`: the
  contract's FS-P3 row parses; `stand_point([-328.30, 73.472, -210.27])` is
  within 0.02 m of `(-325.0, 73.6, -212.8)` (README F20). Fails if the
  bearing convention flips (x or z sign).
- `pose_lines_round_trip`: for the frost and guard poses,
  `toml::from_str` of `pose = [<to_inline_toml()>]` gives back an equal
  `PoseSpec`, and `to_inline_toml` of the frost pose equals the contract's
  line byte for byte. Fails if the key order or rounding drifts (calibrate
  would then rewrite lines it did not mean to).
- `pose_rules_name_the_row`: one spec with a duplicate pose id, a
  `radius_m = 0.5`, a `bearing_deg = 360`, `aim = "point"` without
  `aim_dy_m`, a `pose = "nope"` reference, `tag` with `name` on one action,
  and `face_tag` on `client_world_click`: `validate` errors contain each
  problem with the row id.
- `wait_server_is_a_fifth_action_kind`: `{ wait_server = { log = "x" } }`
  is `ActionKind::ServerWait`; with `wait_ms` as well it is the "more than
  one" error; `timeout_ms = 0` is refused.
- `a_server_clause_timeout_is_capped`: `timeout_ms = 60001` on a `server`
  clause is refused; `60000` passes.

Also: `runner/tests.rs` `committed_specs_plan_against_main_tools` and
`uat/mod.rs` `committed_specs_parse_and_validate` stay green (no committed
spec uses the new fields yet).

Checks: the three lane commands for `-p cimmeria-lab`.

Docs owed: none here (SV-14 documents the vocabulary).

Reviewer focus: the additions sit inside the marked block; `deny_unknown_fields`
still holds; the moved `FakeServer` is unchanged apart from `answer`.

## SV-04 Seed tag index and spec tag validation

**Implementer:** packet-coder. **Size:** S. **Wave:** 2. **Depends on:** SV-03.
**Branch:** `lab-spec-vocab/sv04-seed-tags`. **Worktree:** `sv04`.
**Subject:** `feat(lab): SV-04 reject spec tags the seed does not define`

Why: README F3, F16; D-SV10.

Files:

1. New `crates/lab/src/uat/seed_tags.rs`:

   ```rust
   /// Every (world_id, tag) the seed's spawnlist rows define.
   #[derive(Debug, Clone, Default)]
   pub struct SeedTags { pub tags: std::collections::BTreeMap<String, std::collections::BTreeSet<i32>> }
   impl SeedTags {
       /// Parse every `INSERT INTO spawnlist (<cols>) VALUES (...)[, (...)]*;`
       /// in every `*.sql` file directly under `dir`.
       pub fn load(dir: &Path) -> Result<SeedTags, String>;
       pub fn parse_sql(text: &str, into: &mut SeedTags) -> Result<(), String>;
       pub fn contains(&self, tag: &str) -> bool;
       /// `<specs_dir>/../../../db/resources/Worlds/Seed`, when it is a directory.
       pub fn beside_specs(specs_dir: &Path) -> Option<PathBuf>;
   }
   /// Every tag a section names: pose tags, and `tag` / `face_tag` in any
   /// action's args (setup, step, teardown, fallbacks).
   pub fn spec_tags(spec: &SectionSpec) -> Vec<(String, String)>; // (row id, tag)
   pub fn check(spec: &SectionSpec, seed: &SeedTags) -> Result<(), String>;
   ```

   The parser: find each `INSERT INTO spawnlist` (case-insensitive), read
   the parenthesised column list, find the `tag` and `world_id` positions,
   then read each value tuple with a small tokenizer that understands
   single-quoted strings (with `''` escapes), `NULL`, numbers and commas
   inside parentheses. A row whose tag is `NULL` adds nothing. An insert
   without a `tag` column adds nothing. A tuple with the wrong number of
   values is an error naming the file offset. `check`'s error: `"<row>:
   tag <tag> is not in the seed (db/resources/Worlds/Seed)"`, one per
   unknown tag, joined with `"; "`, plus the nearest seeded tag by
   case-insensitive equality when one exists ("did you mean
   SGC_W1_Tealc?").
2. `crates/lab/src/uat/mod.rs`: `pub mod seed_tags;` and a module-doc
   line. `load_sections` loads `SeedTags::beside_specs(dir)` once when it
   exists and runs `seed_tags::check` on each spec after `spec::parse`; an
   error fails the load like a parse error. Without the folder it checks
   nothing.

Tests:

- In `seed_tags.rs` (unit, type 1): `parses_single_and_multi_row_inserts`
  (a one-row insert, a three-row insert with a `NULL` tag, a quoted
  `'O''Neill_Body'` tag, and a different column order) gives exactly the
  expected `(tag, world)` pairs; `a_short_tuple_is_an_error`.
- `the_committed_seed_defines_the_first_session_targets`: `SeedTags::load`
  on `concat!(env!("CARGO_MANIFEST_DIR"), "/../../db/resources/Worlds/Seed")`
  contains the six tags of README F3 and not `SGC_W1_GenHammond` (the real
  one is `SGCW1_GenHammond`). Fails if the parser misses a file or a column.
- `every_committed_spec_tag_is_seeded`: `load_sections` on the committed
  specs succeeds (it now checks tags). Then, in a temp directory laid out
  as `docs/guides/uat-specs/` beside a copy of the seed folder, a section
  with `tag = "SGC_W1_GenHammond"` fails with the plain "not in the seed"
  error (the real tag differs by an underscore, so there is no hint), and
  `tag = "armyourself_frostbody"` fails with "did you mean
  ArmYourself_FrostBody?". Fails if `load_sections` skips the check.

Checks: the three lane commands for `-p cimmeria-lab`.

Docs owed: none here (SV-14).

Reviewer focus: the tokenizer never panics on malformed SQL (fuzz it by
hand with a truncated insert); `load_sections` with no seed beside the
specs behaves exactly as before.

## SV-05 Absolute camera

**Implementer:** packet-coder. **Size:** M. **Wave:** 1. **Depends on:** none.
**Branch:** `lab-spec-vocab/sv05-absolute-camera`. **Worktree:** `sv05`.
**Subject:** `feat(lab): SV-05 absolute camera pitch, yaw offset and zoom in client_camera`

Why: README F4 to F7; D-SV4.

Files:

1. `crates/lab/src/supervisor/world/memory.rs`: `CameraState::yaw_gain()`
   (`-gain` when `flags & 0x4`, else `gain`), and `"yaw_gain"` in
   `to_json` next to `pitch_gain`. Unit test beside the existing ones.
2. `crates/lab/src/supervisor/world/camera.rs`:
   - `CameraRequest` gains the contract's three fields. `zoom` and
     `zoom_to` together is an `arguments` failure; otherwise `zoom` is used
     as `zoom_to`.
   - After the face block in `run` (and before `camera_after` is read),
     `abs_pitch` then `abs_yaw`, each only when its field is set:

     ```rust
     /// Closed loop on the readout's pitch_offset_deg (D-SV4).
     async fn abs_pitch<W: WorldIo>(io: &mut W, steps: &mut Steps, want: f64) -> Result<Value, WorldError> {
         // up to ABS_MAX_LOOKS times: read camera_readout(); err = want - pitch_offset_deg;
         // stop when |err| <= ABS_TOLERANCE_DEG; counts = round(err * 65536/360 / pitch_gain);
         // pass the counts through memory::clamped_pitch_counts(pitch_units, gain, counts)
         // (read the units from the readout's degrees), look(0, counts, 0), sleep LOOK_SETTLE_MS.
         // A readout without pitch_offset_deg or pitch_gain fails step "abs_pitch".
         // Returns { "want", "got", "looks" }.
     }
     /// Closed loop on the camera actor's world yaw (D-SV4).
     async fn abs_yaw<W: WorldIo>(io: &mut W, steps: &mut Steps, want: f64) -> Result<Value, WorldError> {
         // y0 = camera_pose().yaw. First look: counts = round(want * 65536/360 / |yaw_gain|)
         // (gain from the readout; 9.1 counts per degree at gain 20). Measure
         // d = |wrap_pi(camera_pose().yaw - y0)| in degrees, signed with the counts' sign.
         // Then up to ABS_MAX_LOOKS - 1 corrections: err = want - got;
         // counts = round(err * |sent_total| / |got|) when got != 0.
         // Stop at |err| <= ABS_TOLERANCE_DEG. When camera_pose fails, send the first
         // look only and report "open_loop": true.
     }
     ```

     Each records a step (`"abs_pitch"`, `"abs_yaw"`) and calls
     `steps.used(level)` with what `look` returned. The result gains
     `"absolute": { "pitch": ..., "yaw": ... }` (only the parts that ran).
     The camera did not reach the pitch within the looks: fail step
     `"abs_pitch"` with the last readout (the same for yaw), so a spec never
     passes on a pose it did not get.
   - Update the module doc and the tool description in
     `crates/lab/src/server/world.rs` `client_camera` ("... and/or absolute
     pitch_deg, yaw_offset_deg (after any face) and zoom, closed loop").
3. `crates/lab/src/supervisor/world/sim.rs`: the simulated client gets a
   `camera_readout`: `pitch_offset_deg = -cam_pitch.to_degrees()` (the
   sim's +dy looks down and lowers `cam_pitch`, `sim.rs:318`, while the
   live readout rises with +dy), `zoom = 250.0` unless changed by the wheel
   (add a `zoom: f64` field: wheel notches move it by -30 per +120, clamped
   100 to 775), `gain` and `pitch_gain` in rotator units per count
   (`65536 / (2*PI) / self.gain`, signed as the sim's look applies it), and
   `yaw_offset_deg = 0`. Keep every existing sim behaviour.

Tests (`crates/lab/src/supervisor/world/tool_tests.rs`, unit with the
simulated client, type 1):

- `absolute_pitch_lands_from_any_start`: from level, from +30 deg and from
  -40 deg, `pitch_deg = -28.8` ends within 0.5 deg in at most 4 looks.
  Fails if the loop is open (one look misses with a wrong gain) or the sign
  is flipped (it diverges).
- `face_then_absolute_pitch_is_start_independent`: face a floor point from
  level and from +22 deg with `pitch_deg = -28.8`: both results' `got`
  agree within 0.5. This is README F4's bug shape; without `pitch_deg` the
  two faces end at different pitches (assert that too, so the test proves
  the field is what fixes it).
- `yaw_offset_turns_relative_to_the_face`: face an entity, then
  `yaw_offset_deg = 25.3`: the camera's world yaw moved 25.3 ± 0.5 deg from
  where the face left it, in the direction `+yaw_counts` turns.
- `zoom_and_zoom_to_together_is_refused`.
- Existing camera tests stay green (relative counts unchanged).

Checks: the three lane commands for `-p cimmeria-lab`.

Docs owed: none here (SV-14).

Reviewer focus: nothing changes when the new fields are absent; the pitch
clamp is applied; the loop cannot spin past `ABS_MAX_LOOKS`.
