# Lab record: work packets (1 of 2)

> Type: work packets. Audience: `packet-coder` workers (Haiku),
> `rust-gameserver-dev` for LR-06 and LR-10, `documentation-writer` for LR-12
> and LR-17, and `packet-reviewer` reviewers (Sonnet). Ledger, findings (F1
> to F16) and decisions (D-LR1 to D-LR17): [README.md](README.md). LR-06 to
> LR-17: [work-packets-2.md](work-packets-2.md).
>
> PowerShell only. Every compiling command goes through the lane, in this
> order, from the worktree root:
>
> ```powershell
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo fmt -p cimmeria-lab
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo clippy -p cimmeria-lab --all-targets -- -D warnings
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab
> ```
>
> Read the lane's summary and its failures file; do not rerun a build to see
> the output. No packet but LR-13 and LR-16 uses the lab, a lab MCP tool or a
> running client.
>
> Rust rules for every packet: no `unwrap()` outside tests; comments say why,
> not what; `#[cfg(test)] mod tests` last in a file, or a sibling `*_tests.rs`
> through `#[path]`; no new file over 500 lines; test names say what they
> prove. JSON results follow the lab's compact rule: no nulls, no empty
> arrays, floats to 2 decimals.

## Contents

- [Contract](#contract)
- [LR-01 Record log types and the FS-P3 fixture](#lr-01-record-log-types-and-the-fs-p3-fixture)
- [LR-02 Inbound decoder and row segmenter](#lr-02-inbound-decoder-and-row-segmenter)
- [LR-03 Pose and camera derivation](#lr-03-pose-and-camera-derivation)
- [LR-04 Clauses and timeouts](#lr-04-clauses-and-timeouts)
- [LR-05 TOML emitter and the drafts guard](#lr-05-toml-emitter-and-the-drafts-guard)
- LR-06 to LR-17: [work-packets-2.md](work-packets-2.md)

## Contract

Parallel packets build against these names. A packet that needs to change
one stops and tells the coordinator.

### Module layout

```text
crates/lab/src/uat/
  mod.rs                  + pub mod record;  (LR-01)
  record/
    mod.rs                module doc, pub use, `#![cfg_attr(not(test), allow(dead_code))]` (LR-01; LR-07 removes the allow)
    log.rs                RECORD_SCHEMA, TICK_MS, RecordLine and its parts, read_log, write_line   (LR-01)
    decode.rs             Call, decode_call                                                        (LR-02)
    segment.rs            AnchorKind, Anchor, RecordedRow, segment                                 (LR-02)
    pose.rs               derive_pose, PoseError                                                   (LR-03)
    clauses.rs            Diff, diff_snapshots, row_clauses, timeouts                              (LR-04)
    emit.rs               TransformOptions, Draft, transform, draft_file_name                      (LR-05)
    fixture.rs            row_fixture                                                              (LR-09)
    sampler.rs            ClientProbe, Sampler, SamplerConfig                                      (LR-06)
    rolling.rs            Rolling                                                                  (LR-14)
    bug_spec.rs           BugSpecTrigger, find_bug_spec                                            (LR-15)
    fixtures/             fs_p3_record.jsonl (LR-01), fs_p3_draft.toml (LR-05)
crates/lab/src/supervisor/world/camera_read.rs   Supervisor::camera_read            (LR-06)
crates/lab/src/supervisor/record_slot.rs         RecordSlot, RecordHandle           (LR-06)
crates/lab/src/server/record.rs                  lab_record_start/status/stop       (LR-07)
tools/lab/cli/record.ps1, record-lib.ps1, test-record.ps1                           (LR-08)
docs/guides/uat-specs/drafts/README.md                                              (LR-05)
```

LR-01 creates every `record/*.rs` file (the later ones as a one-line `//!`
stub naming their packet) and declares them all in `record/mod.rs`, so later
packets never edit `mod.rs` except LR-07.

### The raw log (`record/log.rs`, LR-01)

One JSON object per line: a `start`, then one `tick` per 500 ms, then a
`stop`. Absent optional fields are left out (`skip_serializing_if`).

```rust
pub const RECORD_SCHEMA: u32 = 1;
pub const TICK_MS: i64 = 500;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RecordLine { Start(RecordStart), Tick(Tick), Stop(RecordStop) }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordStart {
    pub schema: u32,
    pub recording_id: String,        // "20261012-143000-p2"
    pub section: String,             // the section the person says they are playing
    pub instance: String,            // "default", "p2", ...
    pub character: String,           // the character being played
    pub player_id: i32,              // sgw_player.player_id
    pub player_entity: u32,
    pub host_ms: i64,
    pub date: String,                // "2026-10-12", local date at start
    #[serde(default, skip_serializing_if = "Option::is_none")] pub rolling_min: Option<u32>, // LR-14
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Tick {
    pub seq: u32,
    pub host_ms: i64,
    /// Drained tap rows, verbatim from `server_packet_tap_read` `messages`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub tap: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "is_zero")] pub tap_dropped: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub camera: Option<CameraSample>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub player: Option<EntitySample>,
    /// Visible top-level windows, sorted; None when the UI read failed.
    #[serde(default, skip_serializing_if = "Option::is_none")] pub windows: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub dialog: Option<DialogSample>,
    /// Entities named by an inbound call for the first time this tick.
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub entities: Vec<EntitySample>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub snapshot: Option<DbSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub server_events: Vec<String>, // KNOWN_SERVER_EVENTS keys
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub clicks: Vec<UiClick>,       // LR-10
    /// At most 4, each cut to 120 characters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraSample {
    pub pitch_deg: f64,   // readout pitch_offset_deg
    pub zoom: f64,        // readout zoom
    pub yaw_gain: f64,    // readout gain, negated when the yaw-invert flag is set
    pub yaw_deg: f64,     // camera actor world yaw (pose.yaw_deg)
    pub pos: [f64; 3],    // camera actor server position (pose.server)
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntitySample {
    pub entity_id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub template_id: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub name: Option<String>,
    pub pos: [f64; 3],    // server metres
    pub space_id: u32,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DialogSample {
    #[serde(default, skip_serializing_if = "Option::is_none")] pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub buttons: Vec<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DbSnapshot { pub missions: Vec<SnapMission>, pub player: Option<SnapPlayer>, pub items: Vec<SnapItem> }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapMission { pub mission_id: i32, pub status: i32, pub step: Option<i32>, pub repeats: i32 }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapPlayer { pub level: i32, pub exp: i64, pub naquadah: i64, pub world_location: String }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapItem { pub type_id: i32, pub container_id: i32, pub count: i32, #[serde(default, skip_serializing_if = "Option::is_none")] pub name: Option<String> }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiClick { pub window: String, pub button: String }                // LR-10
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordStop { pub host_ms: i64, pub ticks: u32, pub reason: String } // stop | max_minutes | size_cap | lease_lost | bug_spec:<id>

/// Parse a whole log. Errors name the 1-based line. The first line must be
/// `start` with `schema == RECORD_SCHEMA`; a missing `stop` is allowed (a
/// crashed recording) and reported by `has_stop`.
pub fn read_log(text: &str) -> Result<RecordLog, String>;
pub struct RecordLog { pub start: RecordStart, pub ticks: Vec<Tick>, pub stop: Option<RecordStop> }
/// One line, `\n`-terminated.
pub fn write_line(line: &RecordLine) -> String;
```

### Snapshot SQL (`sampler.rs`, LR-06; the column names LR-04 reads)

`<P>` is the integer `player_id` from the start line (never a name):

```sql
SELECT mission_id, status, current_step_id AS step, repeats FROM sgw_mission WHERE player_id = <P> ORDER BY mission_id
SELECT level, exp, naquadah, world_location FROM sgw_player WHERE player_id = <P>
SELECT i.type_id, i.container_id, count(*)::int AS count, min(t.name) AS name FROM sgw_inventory i LEFT JOIN items t ON t.item_id = i.type_id WHERE i.character_id = <P> GROUP BY i.type_id, i.container_id ORDER BY 1, 2
```

### Anchors and the mapping table (D-LR4, D-LR5)

Inbound calls (`dir = "in"`) by `msg_name`, decoded from `args_hex`:

| Call | `method_index` | Args after the 4-byte entity prefix and the `0xBD` sub-slot byte | Role |
|---|---|---|---|
| `setTargetID` | 0 | target INT32 | remembered as the current target |
| `moveItem` | 38 | item, bag, slot, qty INT32 | anchor |
| `useItem` | 39 | item, target INT32 | anchor |
| `useAbility` | 68 | ability, target INT32 | anchor |
| `interact` | 74 | target INT32 (0: the current target) | anchor |
| `dialogButtonChoice` | 75 | dialog, button INT32 | in-row |
| `triggerClientHintedGenericRegion` | 85 | region INT32, entering UINT8 | anchor when a diff follows |
| `gmGotoXYZ` | 163 | 3 x FLOAT | dropped (the pose replaces it) |
| `sendPlayerCommunication` | base `0xC2` | channel UINT8, target WSTRING, text WSTRING | dot lines into the next row's setup; `.bug` lines dropped |

WSTRING: u32 count of UTF-16 code units, then the units little-endian.
Noise, dropped before anything else: `DEFAULT_IGNORE` from lab-golden
(`avatarUpdate*`, `perfStats`, `requestEntityUpdate`). Any other inbound
call is a `# TODO c2s <msg_name> (method <i>)` line in its row. Outbound
messages make clauses only (`onDialogDisplay`), never TODOs.

What a row becomes (pose id: the tag lower-cased, `_` and `.` to `-`):

| Fact | Emitted |
|---|---|
| `interact` on a tagged entity | a pose line; setup `@stand { pose }`; steps `@camera { pose }` (label `aim`), `@world_click { pose, button = "right" }` (label `interact`, plus `expect = "window"` when `DialogWin` opened in the row) |
| `DialogWin` opened in the row | step `@wait_for { lua_condition = "DialogWin ~= nil and DialogWin:isVisible()", timeout_ms }` after the click |
| `dialogButtonChoice(d, -1)` | step `@finish_dialog {}` (label `close`) |
| `dialogButtonChoice(d, b >= 0)` | `@finish_dialog { accept = true }` when the dialog's buttons in the tick before included `Accept`; else TODO naming the buttons |
| `useItem` | `@item_action { action = "use", name = <items.name> }` |
| `moveItem` into bag 3 to 14 | `@item_action { action = "equip", name }`; from bag 3 to 14 into 1: `action = "unequip"`; other bags: TODO |
| `useAbility(a, t)` | `@use_ability { ability_id = a }`; with `t` a tagged entity other than the player, first `@target { tag }`; untagged `t`: TODO |
| region anchor | step `@stand { point = { x, y, z } }` at the player's position in that tick |
| known server event (`KNOWN_SERVER_EVENTS`) | `wait_server` step, label the key |
| window opened or closed with no call (and no LR-10 click) | TODO naming the window |
| dot chat line | setup `{ chat = "<text>", tier = "G" }` |

Bag numbers are `crates/entity/src/inventory.rs:13-26` (`INV_MAIN = 1`,
`INV_BANDOLIER = 3` to `INV_ARTIFACT2 = 14`), copied as constants with that
citation: `cimmeria-lab` does not depend on `cimmeria-entity`.

```rust
/// key, tracing target, `event` field, log message substring (lab-spec-vocab F8).
pub const KNOWN_SERVER_EVENTS: &[(&str, &str, &str, &str)] = &[
    ("cinematic-hold", "aoi.cinematic_hold", "hold_released", "Cinematic AoI hold: released"),
];
```

### Clauses (D-LR5, D-LR7)

In this order, ids unique in the row:

- `dialog-<d>`: `source = "packet"`, `message = "onDialogDisplay"`,
  `direction = "to_client"`, `match_fields = { dialog_id = d }`, `min_rows = 1`,
  one per distinct dialog id, in the order shown.
- `mission-<m>-status` (a new mission, or a changed status) and
  `mission-<m>-step` (same status, changed step), by mission id. A mission
  that disappeared: `pointer = "/rows/0"`, `op = "absent"`.
- `item-<type>-<container>` (count changed), by type then container.
- `player-level`, `player-exp`, `player-naquadah`, `player-world`.

Server clauses use `tool = "server_db_query"`, `op = "eq"` and
`timeout_ms`, with the character written as `${character}` (or
`TransformOptions::character_expr`):

```sql
SELECT m.status FROM sgw_mission m JOIN sgw_player p ON p.player_id = m.player_id WHERE p.player_name = '${character}' AND m.mission_id = 622
SELECT m.current_step_id FROM sgw_mission m JOIN sgw_player p ON p.player_id = m.player_id WHERE p.player_name = '${character}' AND m.mission_id = 622
SELECT count(*)::int AS n FROM sgw_inventory i JOIN sgw_player p ON p.player_id = i.character_id WHERE p.player_name = '${character}' AND i.type_id = 55 AND i.container_id = 1
SELECT p.level FROM sgw_player p WHERE p.player_name = '${character}'
```

Timeouts, rounded up to a multiple of 250: polled clause
`clamp(max(2t, t + 3000), 3000, 60000)`; dialog wait
`clamp(max(2t, t + 2000), 2000, 30000)`; `wait_server`
`clamp(max(2t, t + 5000), 5000, 120000)`. `t` is the host time from the
anchor's tick to the first tick that shows the effect.

### The FS-P3 draft (the LR-05 golden output, abridged)

```toml
# Recorded draft: UNCONFIRMED until `lab golden record` agrees on it 5 times
# (docs/analysis/lab-record/README.md, D-LR12).
# recording 20261012-143000-p2 on p2, 2026-10-12, 42 ticks, transform 1
schema = 1

[section]
id = "rec-first-session-20261012-1430"
# ... system, guide, ledger, account, character copied from first-session

[[row]]
id = "REC-01"
title = "Interact with ArmYourself_FrostBody"
expected = "Dialog 3995 opens; mission 622 step 2113 -> 80623; mission 1360 -> status 1."
state = "any"
notes = "recorded 2026-10-12, ticks 32-41"
pose = [
  { id = "armyourself-frostbody", tag = "ArmYourself_FrostBody", radius_m = 4.16, bearing_deg = 322.5, dy_m = 0.13, pitch_deg = -28.8, yaw_offset_deg = 25.3, zoom = 250.0, calibrated = "recorded 2026-10-12" },
]
setup = [
  { tool = "@stand", args = { pose = "armyourself-frostbody" }, label = "stand" },
]
step = [
  { tool = "@camera", args = { pose = "armyourself-frostbody" }, label = "aim" },
  { tool = "@world_click", args = { pose = "armyourself-frostbody", button = "right", expect = "window" }, label = "interact" },
  { tool = "@wait_for", args = { lua_condition = "DialogWin ~= nil and DialogWin:isVisible()", timeout_ms = 2000 } },
  { tool = "@finish_dialog", args = {}, label = "close" },
]
[[row.expect]]
id = "dialog-3995"
# ... then mission-622-step (value 80623, timeout_ms = 3500) and mission-1360-status (value 1, timeout_ms = 3500)
```

### Transform entry points

```rust
// emit.rs (LR-05)
pub struct TransformOptions {
    pub base: Option<SectionMeta>,           // the named section's meta, when it exists
    pub account: String, pub character: String, // fallbacks when `base` is None
    pub character_expr: Option<String>,      // replaces the recorded name in SQL
    pub profile: Option<String>,             // LR-09
    pub max_actions: Option<usize>,          // LR-15: 30
    pub max_bytes: Option<usize>,            // LR-15: 32 * 1024
    pub from_tick: Option<u32>,              // LR-15: the rolling window's first tick
}
pub struct Draft { pub file_name: String, pub text: String, pub rows: usize, pub actions: usize, pub todos: usize, pub dropped_rows: usize }
/// Pure: the same log and options give the same bytes.
pub fn transform(log: &RecordLog, opts: &TransformOptions) -> Result<Draft, String>;
/// "<section>-<yyyyMMdd-HHmm>.toml" from the start line's recording id.
pub fn draft_file_name(start: &RecordStart) -> String;
```

---

## LR-01 Record log types and the FS-P3 fixture

**Implementer:** packet-coder. **Size:** S. **Wave:** 1. **Depends on:** none.
**Branch:** `lab-record/lr01-log`. **Worktree:** `lr01`.
**Subject:** `feat(lab): LR-01 record log types and the FS-P3 recording fixture`

Why: D-LR1, D-LR3. Every later packet reads this format.

Files:

1. `crates/lab/src/uat/mod.rs`: `pub mod record;` and one line in the module
   doc list (`[`record`] — lab record: raw recordings and the draft-spec transform`).
2. `crates/lab/src/uat/record/mod.rs`: module doc (what a recording is, that
   the transform infers nothing, link to the ledger), the dead-code allow
   from the contract with a comment (`cimmeria-lab` is a binary crate; the
   items are dead until LR-07 wires the tools), `pub mod` for every file in
   the layout, `pub use log::*;`.
3. Stubs for `decode.rs`, `segment.rs`, `pose.rs`, `clauses.rs`, `emit.rs`,
   `fixture.rs`, `sampler.rs`, `rolling.rs`, `bug_spec.rs`: one `//!` line each
   naming the packet that fills it.
4. `log.rs`: the contract types, `read_log`, `write_line`, `RecordLog`.
5. `record/fixtures/fs_p3_record.jsonl`, built with this PowerShell from the
   worktree root (records 4 to 34 of the Praxis capture are FS-P3; F16):

   ```powershell
   $src = Get-Content -Raw crates/wireclient/tests/fixtures/praxis_start_tap.json | ConvertFrom-Json
   $msgs = @($src.messages[4..34]); $base = [int64]$msgs[0].ts_ms
   $start = [ordered]@{ kind='start'; schema=1; recording_id='20261012-143000-p2'; section='first-session'; instance='p2'
       character='Pxfixture'; player_id=4242; player_entity=8; host_ms=$base; date='2026-10-12' }
   $lines = @(($start | ConvertTo-Json -Compress -Depth 12))
   $m0 = @{ missions=@(@{mission_id=622;status=1;step=2113;repeats=0}); player=@{level=1;exp=0;naquadah=0;world_location='Castle_CellBlock'}; items=@() }
   $m1 = @{ missions=@(@{mission_id=622;status=1;step=80623;repeats=0}, @{mission_id=1360;status=1;step=4037;repeats=0}); player=$m0.player; items=@() }
   for ($t = 0; $t -le 41; $t++) {
       $tap = @($msgs | Where-Object { $t -gt 0 -and [math]::Floor(([int64]$_.ts_ms - $base) / 500) + 1 -eq $t })
       $tick = [ordered]@{ kind='tick'; seq=$t; host_ms=$base + 500 * $t }
       if ($tap.Count) { $tick.tap = $tap }
       $tick.camera = [ordered]@{ pitch_deg=-28.8; zoom=250.0; yaw_gain=20.0; yaw_deg=-27.2; pos=@(-325.0, 75.1, -212.8) }
       $pos = if ($t -eq 0) { @(-334.23, 73.47, -228.03) } else { @(-325.0, 73.6, -212.8) }
       $tick.player = [ordered]@{ entity_id=8; pos=$pos; space_id=65563 }
       $tick.windows = if ($t -ge 32 -and $t -le 35) { @('DialogWin', 'SelfStatusWin') } else { @('SelfStatusWin') }
       if ($t -ge 32 -and $t -le 35) { $tick.dialog = [ordered]@{ title='Corporal Frost' } }
       if ($t -eq 32) { $tick.entities = @([ordered]@{ entity_id=100751; tag='ArmYourself_FrostBody'; template_id=14; pos=@(-328.30, 73.472, -210.27); space_id=65563 }) }
       if ($t -in 0, 1, 2, 32) { $tick.snapshot = $m0 } elseif ($t -ge 33) { $tick.snapshot = $m1 }
       $lines += ($tick | ConvertTo-Json -Compress -Depth 12)
   }
   $lines += ([ordered]@{ kind='stop'; host_ms=$base + 500 * 42; ticks=42; reason='stop' } | ConvertTo-Json -Compress)
   Set-Content crates/lab/src/uat/record/fixtures/fs_p3_record.jsonl -Value $lines -Encoding utf8NoBOM
   ```

   The camera values are chosen so LR-03 gives the calibrated FS-P3 pose:
   the bearing from (-325.0, -212.8) to Frost is -52.5 deg in client yaw,
   and -27.2 + 52.5 = 25.3.

Tests (unit, in `log.rs`):

- `fs_p3_fixture_reads_with_42_ticks_and_a_stop`: `include_str!` the
  fixture; 42 ticks, `stop.reason == "stop"`, tick 32's tap holds an
  `interact` row and its `entities[0].tag` is `ArmYourself_FrostBody`.
- `a_line_round_trips_and_leaves_absent_fields_out`: a `Tick` with only
  `seq` and `host_ms` writes exactly `{"kind":"tick","seq":3,"host_ms":10}\n`
  and reads back equal. Fails if a field loses its `skip_serializing_if`.
- `a_bad_first_line_or_schema_is_an_error_naming_the_line`: a log starting
  with a tick, and a start with `schema: 2`, both fail with `line 1`.
- `a_log_without_stop_still_reads`: drop the last line; `stop` is `None`.

Docs: none (LR-12).

---

## LR-02 Inbound decoder and row segmenter

**Implementer:** packet-coder. **Size:** M. **Wave:** 2. **Depends on:** LR-01, GD-02.
**Branch:** `lab-record/lr02-segment`. **Worktree:** `lr02`.
**Subject:** `feat(lab): LR-02 decode recorded client calls and cut rows at anchors`

Why: D-LR4, F2, F3.

Files:

1. `record/decode.rs`:

   ```rust
   #[derive(Debug, Clone, PartialEq)]
   pub enum Call {
       SetTarget { target: u32 },
       MoveItem { item: i32, bag: i32, slot: i32, qty: i32 },
       UseItem { item: i32, target: u32 },
       UseAbility { ability: i32, target: u32 },
       Interact { target: u32 },
       DialogChoice { dialog: i32, button: i32 },
       Region { region: i32, entering: bool },
       GmGoto { pos: [f32; 3] },
       Chat { channel: u8, text: String },
       Noise,
       /// Any other inbound call, for a TODO line.
       Other { name: String, method_index: i32 },
   }
   /// One tap row (`dir = "in"` only; None for "out").
   pub fn decode_call(row: &serde_json::Value) -> Option<Call>;
   ```

   Use `golden::key_args::decode_inbound` for `interact`, `dialogButtonChoice`,
   `moveItem`, `gmGotoXYZ` and the region call (it skips the entity prefix and
   the `0xBD` sub-slot byte). Write the rest here with the same rules:
   little-endian, bounds-checked, no panics; a too-short payload is `Other`.
   `moveItem`'s item id is the first INT32, which `decode_inbound` skips on
   purpose, so read it here. `Noise` is a name matching any glob of
   `golden::normalise::DEFAULT_IGNORE` (if GD-03 is not merged, a local
   `const NOISE: &[&str] = &["avatarUpdate*", "perfStats", "requestEntityUpdate"];`
   with a comment to swap it). Match `sendPlayerCommunication` by name, and
   by `msg_id == 0xC2` when the name is `unknown`.
2. `record/segment.rs`:

   ```rust
   #[derive(Debug, Clone, PartialEq)]
   pub enum AnchorKind { Interact, UseItem, MoveItem, UseAbility, Region }
   #[derive(Debug, Clone, PartialEq)]
   pub struct Anchor { pub kind: AnchorKind, pub tick: u32, pub call: Call, pub target: Option<u32> }
   #[derive(Debug, Clone, PartialEq)]
   pub struct RecordedRow {
       pub anchor: Anchor,
       /// Ticks covered: the anchor's tick to the tick before the next anchor (or the last tick).
       pub first_tick: u32, pub last_tick: u32,
       /// Calls after the anchor in the row (dialog choices, others), with their tick.
       pub calls: Vec<(u32, Call)>,
       /// Dot chat lines seen since the previous anchor (setup of this row).
       pub setup_chat: Vec<String>,
       /// Outbound messages in the row: (tick, msg_name, decoded).
       pub server: Vec<(u32, String, serde_json::Value)>,
   }
   pub fn segment(log: &RecordLog) -> Vec<RecordedRow>;
   ```

   Rules (D-LR4): walk ticks in order and rows inside a tick in tap order.
   `SetTarget` updates the current target. `Interact { target: 0 }` takes the
   current target. A `Region { entering: true }` becomes an anchor only when
   some snapshot after it and before the next other anchor differs from the
   last snapshot before it (compare `DbSnapshot` with `==`). `GmGoto` and
   `Noise` vanish. `Chat` text starting `.bug` vanishes; other text starting
   `.` goes to the next row's `setup_chat`; other chat vanishes. Calls before
   the first anchor other than chat are dropped (nothing to attach them to).

Tests (unit, fixture-driven, `segment.rs`):

- `fs_p3_is_one_interact_row_on_frost_with_a_close`: `segment` of the
  fixture gives one row: `Interact`, target 100751, `first_tick == 32`,
  `last_tick == 41`, `calls == [(36, DialogChoice { dialog: 3995, button: -1 })]`,
  `server` holds `onDialogDisplay` with `dialog_id` 3995. Fails if the
  region call (record 8) or the teleport anchors a row.
- `interact_zero_uses_the_last_set_target`: a two-tick log with
  `setTargetID(100751)` then `interact(0)`.
- `a_region_with_no_following_diff_is_not_an_anchor` and
  `a_region_followed_by_a_diff_is_an_anchor`.
- `dot_chat_goes_to_the_next_rows_setup_and_bug_lines_vanish`.
- In `decode.rs`: `fs_p3_interact_and_close_decode` on records 19 and 33 of
  the fixture (target 100751; dialog 3995, button -1);
  `a_short_payload_is_other_not_a_panic`.

---

## LR-03 Pose and camera derivation

**Implementer:** packet-coder. **Size:** S. **Wave:** 1. **Depends on:** SV-03 (`PoseSpec`).
**Branch:** `lab-record/lr03-pose`. **Worktree:** `lr03`.
**Subject:** `feat(lab): LR-03 derive a recorded pose from positions and the camera`

Why: D-LR6, F7.

Files: `record/pose.rs`.

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum PoseError { RadiusOutOfRange(f64), NoTag, NoCamera, NoPosition }
/// The pose `@stand` + `@camera { pose }` would reproduce (D-SV3, D-SV4).
pub fn derive_pose(id: &str, target: &EntitySample, player: &EntitySample,
    camera: Option<&CameraSample>, date: &str) -> Result<PoseSpec, PoseError>;
```

- `dx = player.x - target.x`, `dz = player.z - target.z`;
  `radius_m = hypot(dx, dz)`; `bearing_deg = atan2(dz, dx)` in degrees,
  normalised to `[0, 360)`; `dy_m = player.y - target.y`.
- Radius outside 1.0 to 8.0 is `RadiusOutOfRange` (the emitter comments the row).
- With a camera: `pitch_deg = camera.pitch_deg`, `zoom = camera.zoom`, and
  `yaw_offset_deg = -yaw_error(server_to_client(cam.pos), cam.yaw_deg.to_radians(), server_to_client(target.pos)).to_degrees()`,
  negated again when `camera.yaw_gain < 0`. Use
  `supervisor::world::geometry::{yaw_error, server_to_client, Vec3}`; if the
  module is private, make `geometry` `pub(crate)` in
  `crates/lab/src/supervisor/world/mod.rs` (one line, nothing else there).
- Without a camera: `pitch_deg`, `yaw_offset_deg`, `zoom` are `None`.
- `aim = AimMode::Entity`, `aim_dy_m = None`, `calibrated = Some(format!("recorded {date}"))`.
- Round to the `to_inline_toml` precision before returning (radius 2 dp,
  bearing 1 dp, dy 2 dp, angles and zoom 1 dp), so the struct equals what
  is written.

Tests (unit):

- `fs_p3_positions_give_the_calibrated_frost_pose`: target (-328.30, 73.472,
  -210.27), player (-325.0, 73.6, -212.8), the fixture's camera: radius 4.16,
  bearing 322.5, dy 0.13, pitch -28.8, yaw 25.3, zoom 250.0. This is the
  hand-calibrated FS-P3 pose (lab-spec-vocab F20); a sign error in bearing
  or yaw gives 37.5 or -25.3 and fails.
- `inverting_the_yaw_gain_flips_the_offset`.
- `a_round_trip_through_stand_point_lands_on_the_player`:
  `pose.stand_point(target.pos)` is within 0.01 m of the player (SV-03's
  function; proves the inverse matches D-SV3).
- `a_far_click_is_radius_out_of_range`: 9.3 m.

---

## LR-04 Clauses and timeouts

**Implementer:** packet-coder. **Size:** M. **Wave:** 2. **Depends on:** LR-01, LR-02, SV-03.
**Branch:** `lab-record/lr04-clauses`. **Worktree:** `lr04`.
**Subject:** `feat(lab): LR-04 derive row clauses from server-state diffs`

Why: D-LR5, D-LR7, F5, F9.

Files: `record/clauses.rs`.

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Diff {
    MissionStatus { mission: i32, from: Option<i32>, to: Option<i32> },  // None: no row
    MissionStep { mission: i32, from: Option<i32>, to: Option<i32> },
    ItemCount { type_id: i32, container: i32, from: i32, to: i32, name: Option<String> },
    Player { field: &'static str, from: String, to: String },          // level, exp, naquadah, world
}
/// What changed between two snapshots, in the contract's clause order.
pub fn diff_snapshots(before: &DbSnapshot, after: &DbSnapshot) -> Vec<Diff>;
/// The last snapshot strictly before `tick`, and the last at or before `last_tick`.
pub fn row_snapshots(log: &RecordLog, row: &RecordedRow) -> Option<(&DbSnapshot, &DbSnapshot)>;
/// Host ms from the anchor's tick to the first tick whose snapshot shows `d`.
pub fn settle_ms(log: &RecordLog, row: &RecordedRow, d: &Diff) -> i64;
pub fn clause_timeout_ms(t: i64) -> u64;      // clamp(max(2t, t+3000), 3000, 60000), up to 250
pub fn dialog_timeout_ms(t: i64) -> u64;      // clamp(max(2t, t+2000), 2000, 30000)
pub fn server_wait_timeout_ms(t: i64) -> u64; // clamp(max(2t, t+5000), 5000, 120000)
/// One clause, ready for the emitter: id, text and the key/value pairs in
/// write order (`source`, `tool`, `args`, `pointer`, `op`, `value`, `timeout_ms`, ...).
pub struct ClauseOut { pub id: String, pub text: String, pub fields: Vec<(&'static str, Inline)> }
pub fn row_clauses(log: &RecordLog, row: &RecordedRow, character_sql: &str) -> Vec<ClauseOut>;
```

`Inline` is the emitter's value type, defined here (LR-05 uses it):

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Inline { Str(String), Int(i64), Float(f64, usize), Bool(bool), Table(Vec<(String, Inline)>), Array(Vec<Inline>) }
impl Inline { pub fn render(&self) -> String; }
```

`render`: strings through `toml::Value::String(s).to_string()` (escaping),
floats with their fixed decimal count, tables as `{ k = v, k = v }` (an
empty one as `{}`), arrays as `[a, b]`.

Rules: the contract's clause list and SQL. `character_sql` is already the
text that goes between the quotes (`${character}` or the expression). Item
clauses only for containers 1 (main), 2 (mission) and 3 to 14. Clause
`text`: `"onDialogDisplay 3995 to the player"`, `"sgw_mission: 622 on step 80623"`,
`"sgw_mission: 1360 status 1"`, `"carried SI 3 9mm Pistol (55) in bag 1: 1"`,
`"sgw_player: level 2"`.

Tests (unit):

- `fs_p3_row_gives_a_dialog_clause_and_two_mission_clauses`: on the
  fixture's one row: ids `dialog-3995`, `mission-622-step` (value 80623),
  `mission-1360-status` (value 1), both server clauses `timeout_ms = 3500`
  (settle 500 ms). Fails if the start snapshot is taken at or after the
  anchor's tick (tick 32 still holds the old state, on purpose).
- `timeouts_have_floors_ceilings_and_quarter_seconds`: `clause_timeout_ms(0)
  == 3000`, `(1600) == 4750`, `(40000) == 60000`; `dialog_timeout_ms(1) == 2000`;
  `server_wait_timeout_ms(17000) == 34000`.
- `a_removed_mission_is_an_absent_clause` and
  `an_item_count_change_is_one_clause_per_container`.
- `inline_strings_are_escaped`: a value with a quote and a backslash renders
  as valid TOML (parse it back with `toml::from_str`).

---

## LR-05 TOML emitter and the drafts guard

**Implementer:** packet-coder. **Size:** M. **Wave:** 3. **Depends on:** LR-02, LR-03, LR-04, SV-04.
**Branch:** `lab-record/lr05-emit`. **Worktree:** `lr05`.
**Subject:** `feat(lab): LR-05 write a recorded draft spec, TODOs and all`

Why: D-LR5, D-LR8, D-LR10.

Files:

1. `record/emit.rs`: `TransformOptions`, `Draft`, `transform`,
   `draft_file_name` (contract). Layout, in this order:
   - the header comments (contract example), then `schema = 1`;
   - `[section]`: `id = "rec-<section>-<yyyyMMdd-HHmm>"`, then `system`,
     `guide`, `ledger`, `account`, `character` from `opts.base` (never
     `fresh`), else `system = "recorded"`, `guide = "docs/guides/automated-uat.md"`,
     `ledger = "docs/analysis/lab-record/README.md"` and the fallbacks;
   - a `# TODO` header line for each fault: `tap dropped <n> messages`,
     `recording has no stop line`, `<n> UI read failures`;
   - one `[[row]]` per `RecordedRow` (`REC-01`, `REC-02`, ...), with `title`,
     `expected` (the clause texts joined with `; `, ending `.`), `state = "any"`,
     `notes = "recorded <date>, ticks a-b"`, `pose` (one line per pose via
     `PoseSpec::to_inline_toml`), `setup`, `step` and the clauses from LR-04,
     each action on one line as in the contract;
   - TODO lines inside the `step` array as `  # TODO ...` comment lines.
   A row whose pose fails (`PoseError`) or whose target has no tag is written
   with every line prefixed `# `, under `# TODO REC-0n not runnable: <reason>`.
   Strings go through `Inline::render`. The text ends with one `\n`.
   After building, `transform` parses its own text with `spec::parse`; a
   failure is an `Err` naming the problem (never write an unparsable draft).
2. `record/fixtures/fs_p3_draft.toml`: the expected output for the fixture
   with `opts.base` = the `[section]` of `docs/guides/uat-specs/first-session.toml`
   (load it in the test with `spec::parse`). Generate it once with the code,
   read it line by line against the contract and the README decisions, and
   commit it.
3. `docs/guides/uat-specs/drafts/README.md`: what the folder holds, that
   `lab uat` never loads it unless pointed at it (`-SpecsDir`), that a draft is
   UNCONFIRMED until `lab golden record` agrees 5 times, and how to promote
   one (move rows into the real spec, record its golden).
4. `crates/lab/src/uat/mod.rs` tests: `committed_drafts_parse_and_validate`:
   `load_sections(<specs>/drafts, None)` succeeds (zero files is fine), and
   for each, `seed_tags::check` against
   `SeedTags::load(<repo>/db/resources/Worlds/Seed)`.

Tests (unit, `emit.rs` through a sibling `emit_tests.rs`):

- `fs_p3_transform_matches_the_committed_draft`: byte-equal with
  `fs_p3_draft.toml`. This is the regression guard for the whole transform:
  any change to a rule shows as a diff of that file.
- `the_transform_is_deterministic`: two calls give identical text.
- `an_unknown_call_is_a_todo_line_and_the_draft_still_parses`: insert a tap
  row `{dir: "in", msg_name: "gmSetGodMode", method_index: 150, args_hex: "00000000"}`
  in tick 33; the text holds `# TODO c2s gmSetGodMode (method 150)` and parses.
- `a_far_pose_comments_the_row_out`: move the player to 9.3 m; the row is
  commented and the draft parses with zero rows.
- `the_character_expression_replaces_the_name_in_sql`: `character_expr =
  Some("Px${run_id}")`.

Docs: `drafts/README.md` (this packet). Doc-update map row: "new folder under
`docs/guides/`" (the README is the folder's index; LR-12 links it from the
authoring guide).
