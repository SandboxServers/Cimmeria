# Player capture work packets: wave 4 (tools) and the close-out

> Type: work packets. Rules, contract and index: [work-packets.md](work-packets.md). Ledger: [README.md](README.md).

Wave 4 turns exported rows into lab inputs. It starts **after wave 2 merges and PC-07 confirms that the live row shape matches the contract**; any drift goes back to the architect first. It needs PC-14a's `human::rows`. Every test uses inline rows, so it can run beside wave 3. PC-21b and PC-21c also wait for PC-07b to confirm the camera rows' shape and join shares.

The lab-cli packets all edit `commands/mod.rs` and `human/mod.rs`. To keep them from conflicting, the wave runs in two halves, each reviewed by one packet-review-fixer.

| Half | Packets | Start |
|---|---|---|
| 4a | PC-20, PC-22, PC-23, in parallel | `origin/main` after wave 2 |
| 4b | PC-21, then PC-21b, then PC-21c, stacked (one PR); PC-24 in parallel | `origin/main` after 4a merges |

PC-25 (data runs) and PC-09 (close-out) follow once the live gates have passed.

## Rules for every code packet in this file

1. **PowerShell only**, never bash. Start every shell call with `Set-Location <your worktree>;`, using the absolute path.
2. **Never run a compiling `cargo` directly.** `cargo fmt` may run bare. `clippy` and `nextest` go through `pwsh tools/build-lane/lane.ps1 cargo ...`; live-DB tests go through `pwsh tools/build-lane/live-db-test.ps1`. Run them in the **foreground**, and commit as soon as they pass.
3. **Git limits.** Never use `git stash`, `git worktree prune`, `--no-verify`, push or force. Commit once, locally, with the given subject and your brief's attribution lines.
4. **File size.** Soft cap 500 lines, hard cap 700. `crates/lab-cli/src/commands/mod.rs` is about 422 lines after PC-14b; each new command adds about 8.
5. **lab-cli help tests.** Each new subcommand needs three things, or `commands` tests fail:
   - a `COMMANDS` entry whose `name` equals the clap name;
   - an `after_help` Examples block whose lines start with `lab <name>`, parse, and stay within 76 columns;
   - `long_about` text.

   Copy the shape of `commands/friction_drafts/mod.rs` (PC-14b).
6. **No network and no SigNoz.** The tools read JSONL files only (D-PC15). No host, IP, account or character name goes into code, tests, fixtures or output.
7. **Tests.** Unit tests with inline JSONL strings (`human::rows::parse_jsonl`), and no files except where `tempfile` is named. Each test fails when its change is reverted, and says why in a one-line doc comment.
8. **Output.** Print through `anstream::println!`. Errors go through `ctx.ui.error(&msg, None)` and return exit code 1. Successful runs return 0.
9. **Docs** under `docs/` are CRLF. **Scope:** edit only the named files. If an assumption is false, return `blocked` with the reason.

---

## PC-20 Human pose statistics (wave 4a)

- **Agent:** packet-coder.
- **Worktree:** `pc-20`. **Branch:** `player-capture-bots/pc-20-pose-stats`. **Crate:** `cimmeria-lab-cli`.
- **Commit subject:** `feat(lab-cli): per-target human pose statistics with circular bearing`.
- **Files:**
  - `crates/lab-cli/src/human/stats.rs` (new)
  - `crates/lab-cli/src/human/stats_tests.rs` (new)
  - `crates/lab-cli/src/human/mod.rs` (add `pub mod stats; #[cfg(test)] mod stats_tests;`)

**Change.** Write `human/stats.rs`. It reads pose rows only (`interaction.pose` fields, contract § `interaction.pose`). The idea 7 check, facing error, is reported as its own spread.

```rust
use super::rows::Row;
#[derive(Debug, Clone, PartialEq)]
pub struct Spread { pub n: usize, pub median: f64, pub p10: f64, pub p90: f64 }
/// Sorts a copy. The median is the mean of the two middle values for an even n.
/// The p-th percentile is nearest-rank: sorted[ceil(p*n) - 1], with the index clamped to 0..n.
pub fn spread(values: &[f64]) -> Option<Spread>; // None when empty
#[derive(Debug, Clone, PartialEq)]
pub struct Circular { pub n: usize, pub mean_deg: f64, pub resultant: f64, pub spread_deg: f64 }
/// mean_deg = atan2(mean sin, mean cos) in degrees, wrapped to [0, 360);
/// resultant R = hypot(mean sin, mean cos), in 0..=1;
/// spread_deg = sqrt(-2 ln R) in degrees (the circular standard deviation; 180.0 when R < 1e-9).
pub fn circular(degrees: &[f64]) -> Option<Circular>;
#[derive(Debug, Clone, PartialEq)]
pub struct TagStats {
    pub world: String, pub tag: String, pub kind: String, // kind: "interact" or "use_item"
    pub n: usize, pub players: usize,                     // players = distinct player_id
    pub radius: Spread, pub dy: Spread, pub bearing: Circular, pub facing_error: Spread,
}
#[derive(Debug, Clone, Default)]
pub struct StatsFilter { pub session_kind: Option<String>, pub world: Option<String>, pub min_n: usize }
/// Groups pose rows by (world, target_tag, kind). A row is used only when it has target_tag,
/// radius_m, bearing_deg and dy_m. It must also match session_kind (`cimmeria.session_kind`)
/// and world when those filters are set. Groups with n < min_n are dropped.
/// Sorted by n descending, then by tag ascending.
pub fn pose_stats(rows: &[Row], f: &StatsFilter) -> Vec<TagStats>;
```

A row missing `facing_error_deg` still counts. The `facing_error` spread uses only the rows that have it, and when no row has it, its `n` is 0 with every value 0.0.

**Tests.** `stats_tests.rs`:
- `median_of_even_count_is_mid_mean`: `[1, 2, 3, 4]` gives median 2.5, p10 1 and p90 4.
- `bearing_mean_crosses_north`: `[350, 10]` gives a `mean_deg` within 1e-6 of 0 (or 360) and `resultant > 0.98`. A linear mean would give 180, which proves the circular mean.
- `opposed_bearings_have_wide_spread`: `[0, 180]` gives `spread_deg == 180.0`.
- `groups_by_world_tag_kind`: two worlds and two tags give four groups, sorted by n.
- `session_filter_drops_lab_rows`: with the `player` filter, a `lab` row is not counted.
- `min_n_drops_thin_groups`: a group of two is dropped with `min_n = 3`.
- `players_counts_distinct_player_ids`.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-20;`:

1. `cargo fmt -p cimmeria-lab-cli --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-lab-cli --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab-cli human`
4. `git diff --check`

---

## PC-22 `lab coverage` (wave 4a)

- **Agent:** packet-coder.
- **Worktree:** `pc-22`. **Branch:** `player-capture-bots/pc-22-lab-coverage`. **Crate:** `cimmeria-lab-cli`.
- **Commit subject:** `feat(lab-cli): lab coverage diffs human interaction tags against the UAT specs`.
- **Files:**
  - `crates/lab-cli/src/commands/coverage/mod.rs` (new)
  - `crates/lab-cli/src/commands/coverage/scan.rs` (new)
  - `crates/lab-cli/src/commands/coverage/tests.rs` (new)
  - `crates/lab-cli/src/commands/mod.rs` (about 8 lines added)

**Purpose (idea 6).** Show which targets players actually use that no UAT spec touches, and which pairs of targets players visit back to back.

**Change.**

1. `commands/mod.rs`:
   - add `pub mod coverage;`;
   - add a `COMMANDS` entry `coverage`, in the group PC-14b used, with the summary "Human interaction tags no UAT spec covers";
   - add the enum variant and the dispatch arm.
2. `coverage/scan.rs` holds pure functions:
   ```rust
   /// Every `tag = "<value>"` in the text, via regex r#"\btag\s*=\s*"([^"]+)""#.
   pub fn spec_tags(toml_text: &str) -> std::collections::BTreeSet<String>;
   /// Reads every *.toml in dir and in dir/drafts (not recursive further; golden/ is skipped).
   pub fn spec_tags_in(dir: &std::path::Path) -> Result<std::collections::BTreeSet<String>, String>;
   #[derive(Debug, Clone, PartialEq)]
   pub struct TagUse { pub world: String, pub tag: String, pub n: usize, pub players: usize, pub covered: bool }
   pub fn tag_uses(rows: &[Row], covered: &BTreeSet<String>) -> Vec<TagUse>; // uncovered first, then n desc, then tag
   #[derive(Debug, Clone, PartialEq)]
   pub struct Pair { pub from: String, pub to: String, pub n: usize, pub covered: bool } // covered = both tags covered
   /// For each player_id, sort rows by pose_seq. Each two consecutive rows with distinct
   /// target_tag in the same world make a pair. Count pairs; sort by n desc, then from, then to.
   pub fn tag_pairs(rows: &[Row], covered: &BTreeSet<String>) -> Vec<Pair>;
   ```
   Only rows with `cimmeria.session_kind == "player"` count, unless `--all-sessions` is given.
3. `coverage/mod.rs`. Usage: `lab coverage <poses.jsonl> [--specs-dir DIR] [--top N] [--all-sessions] [--json]`.
   - `--specs-dir` defaults to `<checkout>/docs/guides/uat-specs`. Find the checkout the same way `commands/calibrate/preflight.rs` does; if none is found, error "give --specs-dir".
   - `--top` defaults to 20.
   - Plain output:
     - a header line with the row count and the number of spec tags;
     - one line per `TagUse`, shaped `MISSING  <world>  <tag>  n=<n> players=<p>` or `covered  ...`;
     - then `pairs:`, with the top N `Pair` lines.
   - `--json` prints one object, `{rows, spec_tags, tags: [..], pairs: [..]}`.

**Tests.** `coverage/tests.rs`:
- `spec_tags_reads_pose_lines`: the Frost pose line from `docs/guides/automated-uat.md`, inlined, gives `{"ArmYourself_FrostBody"}`.
- `uncovered_tags_sort_first`.
- `lab_rows_are_ignored_by_default`.
- `pairs_follow_pose_seq_per_player`: two players with interleaved `pose_seq` give per-player pairs, never one across players.
- `spec_dir_skips_golden`: use `tempfile`, with a tag only in `golden/x.toml`, and that tag is not in the set.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-22;`:

1. `cargo fmt -p cimmeria-lab-cli --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-lab-cli --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab-cli`, the whole crate, because the help tests cover every command.
4. `git diff --check`

**Docs owed:** none in this packet; PC-09 adds the lab-cli guide rows for all three commands.

---

## PC-23 wireclient replay cases (wave 4a)

- **Agent:** packet-coder.
- **Worktree:** `pc-23`. **Branch:** `player-capture-bots/pc-23-replay-cases`. **Crate:** `cimmeria-wireclient`.
- **Commit subject:** `feat(wireclient): load recorded interaction poses and approach trails as replay cases`.
- **Files:**
  - `crates/wireclient/src/replay/mod.rs` (new)
  - `crates/wireclient/src/replay/case.rs` (new)
  - `crates/wireclient/src/replay/tests.rs` (new)
  - `crates/wireclient/src/lib.rs` (add `pub mod replay;` to the `pub mod` list)

**Purpose (idea 9, server-only).** A replay case is a recorded human interaction: where they came from, where they stood and what they clicked. PC-24 drives one case against a test server. This proves that **the server** accepts the recorded stand-off. It does not prove that a real client's click lands; that is `lab calibrate`'s job.

**Change.** `replay/case.rs`, parsing with `serde_json`. Wireclient cannot depend on lab-cli, so write a private helper `num(obj, key) -> Option<f64>` that accepts a number or a numeric string.

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReplayPoint { pub pos: [f32; 3], pub yaw_deg: f32, pub age_s: f32 }
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayCase {
    pub pose_seq: u64, pub kind: String, pub world: Option<String>,
    pub target_tag: Option<String>, pub target_template_id: Option<i32>,
    pub player: [f32; 3], pub player_yaw_deg: f32,
    pub radius_m: f32, pub bearing_deg: f32, pub dy_m: f32,
    pub target: [f32; 3],          // derived, D-PC9: [px - r·cos b, py - dy, pz - r·sin b]
    pub approach: Vec<ReplayPoint>, // oldest first; empty when no approach row matched
}
/// The inverse of ApproachTrail::encode: "x,y,z,yaw,age;..." -> points. An empty string gives an empty Vec.
pub fn decode_points(s: &str) -> Result<Vec<ReplayPoint>, String>; // "point N: <why>"
/// pose_jsonl: interaction.pose rows. approach_jsonl: interaction.approach rows (may be empty).
/// Joins on pose_seq. A pose row missing pose_seq, player_x/y/z, radius_m, bearing_deg or dy_m
/// is an error "line N: missing <key>". An approach row without a matching pose is ignored.
pub fn parse_cases(pose_jsonl: &str, approach_jsonl: &str) -> Result<Vec<ReplayCase>, String>;
/// The path to walk: the approach points, then the recorded stand (player, player_yaw_deg).
/// A point within 0.05 m of the one before it is dropped.
pub fn waypoints(case: &ReplayCase) -> Vec<([f32; 3], f32)>;
```

`replay/mod.rs` has the module doc, which carries the server-only scope sentence above, plus `mod case; pub use case::*; #[cfg(test)] mod tests;`.

**Tests.** `replay/tests.rs`:
- `decode_points_round_trips_the_contract_format`: `"1.00,2.00,3.00,90.0,4.5;5.00,6.00,7.00,180.0,0.5"` gives two points.
- `target_is_derived_from_bearing`: radius 2, bearing 90 and player `[10, 0, 10]` give target `[10, 0, 8]` (within 1e-4).
- `approach_joins_on_pose_seq`: two poses and one approach row; only the matching case has points.
- `missing_key_names_line_and_key`.
- `numbers_may_be_strings`: `"radius_m": "4.16"` parses.
- `waypoints_end_at_the_stand_and_drop_duplicates`.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-23;`:

1. `cargo fmt -p cimmeria-wireclient --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-wireclient --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-wireclient replay`
4. `git diff --check`

---

## PC-21 `lab pose-library` (wave 4b)

- **Agent:** packet-coder.
- **Worktree:** `pc-21`, from `origin/main` after PC-20 and PC-22 merge. **Branch:** `player-capture-bots/pc-21-pose-library`. **Crate:** `cimmeria-lab-cli`.
- **Commit subject:** `feat(lab-cli): lab pose-library proposes a spec pose line from human rows`.
- **Files:**
  - `crates/lab-cli/src/commands/pose_library/mod.rs` (new)
  - `crates/lab-cli/src/commands/pose_library/line.rs` (new)
  - `crates/lab-cli/src/commands/pose_library/tests.rs` (new)
  - `crates/lab-cli/src/commands/mod.rs` (about 8 lines added)

**Contract excerpt.** These are the spec pose keys (`docs/guides/automated-uat.md` § pose):

| Key | Meaning |
|---|---|
| `id` | the row id |
| `tag` | the target's tag |
| `radius_m` | 1.0 to 8.0 |
| `bearing_deg` | 0 to under 360 |
| `dy_m` | the height offset |
| `pitch_deg` | optional; -78.75 to 78.75 |
| `yaw_offset_deg` | optional; -180 to 180 |
| `zoom` | optional; 100 to 775 |
| `calibrated` | one provenance line |

**Change.**

1. `commands/mod.rs`:
   - add `pub mod pose_library;`;
   - add a `COMMANDS` entry `pose-library`, in the group PC-14b used, with the summary "Propose a spec pose line from human rows";
   - add the variant and the dispatch arm.
2. `pose_library/line.rs` holds pure functions:
   ```rust
   #[derive(Debug, Clone, PartialEq)]
   pub struct PoseLine { pub id: String, pub tag: String, pub radius_m: f64, pub bearing_deg: f64, pub dy_m: f64,
       pub pitch_deg: Option<f64>, pub yaw_offset_deg: Option<f64>, pub zoom: Option<f64>, pub calibrated: String }
   /// From TagStats: radius = median clamped to 1.0..=8.0, bearing = circular mean, dy = median,
   /// each rounded to 2 dp (bearing to 1 dp). id = the tag lowercased, with runs of
   /// non-alphanumerics turned into one '-'. calibrated = "human <date> n=<n> players=<p>".
   pub fn pose_line(s: &TagStats, date: &str) -> PoseLine;
   /// One TOML inline table in the spec's key order, omitting None keys:
   /// `{ id = "...", tag = "...", radius_m = 4.16, bearing_deg = 322.5, dy_m = 0.13, calibrated = "..." }`
   pub fn render(line: &PoseLine) -> String;
   /// Comment lines printed above the table: the spreads (radius p10/p90, bearing spread_deg,
   /// facing_error median) and a warning when bearing spread_deg > 45 ("players stand all around it").
   pub fn notes(s: &TagStats) -> Vec<String>;
   ```
3. `pose_library/mod.rs`. Usage: `lab pose-library <poses.jsonl> --tag T [--world W] [--min-n 5] [--all-sessions] [--json]`.
   - Call `pose_stats` with `session_kind = Some("player")` unless `--all-sessions` is given.
   - Keep the groups with `tag == T`. If none, error "no rows for <T> with at least <min-n> samples".
   - If more than one world is left and no `--world` was given, error "<T> is in several worlds: <list>; give --world".
   - Use the `interact` group before the `use_item` group.
   - Print `notes`, then `render`. `--json` prints the `PoseLine` and the stats as one object.
   - Take the date from `chrono::Utc::now().format("%Y-%m-%d")`. A private `run_with_date` lets tests pin it.

**Tests.** `pose_library/tests.rs`:
- `renders_in_spec_key_order`: the rendered line matches an exact expected string.
- `radius_is_clamped`: a median of 0.4 gives 1.0.
- `id_is_slugged_from_tag`: `ArmYourself_FrostBody` gives `armyourself-frostbody`.
- `wide_bearing_warns`: opposed bearings give the warning note.
- `several_worlds_need_world_flag`: `run_with_date` on a `tempfile` JSONL returns 1, and the error names both worlds.
- `calibrated_names_the_sample`: it has the `n=` and `players=` values.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-21;`:

1. `cargo fmt -p cimmeria-lab-cli --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-lab-cli --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab-cli`
4. `git diff --check`

---

## PC-21b Joining camera rows to pose rows (wave 4b, stacked on PC-21)

- **Agent:** packet-coder.
- **Worktree:** `pc-21b`. **Branch:** `player-capture-bots/pc-21b-camera-join`. **Crate:** `cimmeria-lab-cli`.
- **First command:** `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-21b; git merge --ff-only player-capture-bots/pc-21-pose-library`. If the fast-forward fails, return `blocked`.
- **Commit subject:** `feat(lab-cli): join client camera rows to interaction poses by Mercury sequence, then server time`.
- **Files:**
  - `crates/lab-cli/src/human/camera.rs` (new)
  - `crates/lab-cli/src/human/camera_tests.rs` (new)
  - `crates/lab-cli/src/human/mod.rs` (`pub mod camera;` and `#[cfg(test)] mod camera_tests;`)

**Contract excerpt** ([work-packets.md](work-packets.md), the camera section and addendum A1).
- Camera rows are `client.native` rows. `client_target == "client.camera.at_interact"` rows carry `session_id`, `ts_ms` (client clock), `server_ts_ms` (when admin-api measured the clock), `method` (`interact` or `useItem`), `entity_id`, `send_id`, `zoom`, `pitch_offset_deg`, `yaw_offset_deg`, `camera_flags`, `camera_gain`, `camera_pose_zero`, and `camera_x/y/z`, `camera_yaw_deg` (absent when the camera pose is zero).
- `client_target == "client.camera.sent_seq"` rows carry `session_id`, `send_id`, `method`, `mercury_seq_first`, `mercury_seq_last`.
- Pose rows (`interaction.pose`) carry `timestamp` (server time: integer ms or RFC 3339), `entity_id`, `kind` (`interact` or `use_item`) and, from PC-26, `mercury_seq`.
- Sequence numbers wrap at 28 bits: `seq_in_range(seq, first, last) = (seq.wrapping_sub(first) & 0x0fff_ffff) <= (last.wrapping_sub(first) & 0x0fff_ffff)`. This is the rule in `crates/client-telemetry/src/hooks/ability_trace/seq_join.rs`. Copy it; do not import that crate (it is the i686 DLL).

**Change.** `human/camera.rs`:

```rust
//! Joins client camera rows to interaction.pose rows (player-capture-bots, D-PC20).
pub const CAMERA_TARGET: &str = "client.camera.at_interact";
pub const SENT_SEQ_TARGET: &str = "client.camera.sent_seq";
pub const SEQ_MASK: u32 = 0x0fff_ffff;
pub fn seq_in_range(seq: u32, first: u32, last: u32) -> bool;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinMethod { Seq, ServerTime, ClientTime } // as_str: "seq", "server_time", "client_time"
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JoinWindows { pub server_ms: i64, pub client_ms: i64, pub seq_guard_ms: i64 }
impl Default for JoinWindows { /* 500, 1000, 60_000 */ }
/// A row's `timestamp` in ms: an integer, or RFC 3339 parsed with chrono.
pub fn timestamp_ms(r: &Row) -> Option<i64>;
/// "interact" matches "interact"; "useItem" matches "use_item".
pub fn kind_matches(method: &str, kind: &str) -> bool;
/// The camera row for one pose row, and how it was joined.
pub fn join_camera<'a>(pose: &Row, cams: &'a [Row], sent: &[Row], w: &JoinWindows)
    -> Option<(&'a Row, JoinMethod)>;
```

`join_camera` keeps only `cams` rows whose `client_target` is `CAMERA_TARGET`, and only `sent` rows whose `client_target` is `SENT_SEQ_TARGET`. A camera row is a candidate only with the same `entity_id` as the pose and a `method` that `kind_matches` the pose's `kind`. Then, in order:

1. **Seq.** If the pose has `mercury_seq`: find `sent` rows whose `method` matches the kind and whose range holds it (`seq_in_range`). For each, take the candidate with the same `session_id` and `send_id`. Keep it only if `|timestamp_ms(pose) - cam time| <= seq_guard_ms`, where the cam time is `server_ts_ms`, or `ts_ms` if that is absent. The guard stops a sequence reused after a relogin from matching. Nearest first.
2. **ServerTime.** Candidates with `server_ts_ms` and `|timestamp_ms(pose) - server_ts_ms| <= server_ms`. Nearest first.
3. **ClientTime.** Candidates **without** `server_ts_ms` and `|timestamp_ms(pose) - ts_ms| <= client_ms`. Nearest first.

In tiers 2 and 3, skip a candidate when the pose has `mercury_seq` and the candidate has a `sent` row (same `session_id` and `send_id`) whose range does not hold it. Both sides had a sequence and they disagreed, so that camera belongs to another call. Ties go to the earlier row in `cams`.

**Tests.** `human/camera_tests.rs` builds rows with `parse_jsonl`.
- `seq_join_beats_a_nearer_clock_match`: the pose has `mercury_seq` 100. Camera A (send 9) has a `sent` range 100..=101 and is 40 s away; camera B (send 10) has no `sent` row and its `server_ts_ms` is 100 ms away. The result is A with `Seq`.
- `seq_range_wraps_at_28_bits`: `seq_in_range(1, 0x0fff_fffe, 2)` is true and `seq_in_range(3, 0x0fff_fffe, 2)` is false.
- `seq_guard_rejects_a_reused_sequence`: the matching `sent` row's camera is 90 s away, so the result falls through to the time tiers (here, none: `None`).
- `server_time_window_is_500_ms`: with no `mercury_seq`, a camera with `server_ts_ms` 400 ms away joins with `ServerTime`, and one 600 ms away does not.
- `client_time_only_for_rows_without_server_time`: a camera with no `server_ts_ms` 800 ms away by `ts_ms` joins with `ClientTime`; a camera with `server_ts_ms` 2 s away and `ts_ms` 100 ms away does not join.
- `disagreeing_sequences_never_clock_join`: the pose has `mercury_seq` 100; the only camera is 50 ms away, with a `sent` range of 300..=300. The result is `None`.
- `join_needs_same_entity_and_kind`: a `useItem` camera does not join an `interact` pose, and a camera for another `entity_id` does not join.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-21b;`:

1. `cargo fmt -p cimmeria-lab-cli --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-lab-cli --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab-cli camera`
4. `git diff --check`

---

## PC-21c Camera keys for `lab pose-library` (wave 4b, stacked on PC-21b)

- **Agent:** packet-coder.
- **Worktree:** `pc-21c`. **Branch:** `player-capture-bots/pc-21c-pose-camera-keys`. **Crate:** `cimmeria-lab-cli`.
- **First command:** `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-21c; git merge --ff-only player-capture-bots/pc-21b-camera-join`. If the fast-forward fails, return `blocked`.
- **Commit subject:** `feat(lab-cli): pose-library proposes pitch, yaw offset and zoom from joined camera rows`.
- **Files:**
  - `crates/lab-cli/src/human/camera.rs` (adds `CameraKeys` and `camera_keys`)
  - `crates/lab-cli/src/human/camera_keys_tests.rs` (new)
  - `crates/lab-cli/src/human/mod.rs` (one test `mod` line)
  - `crates/lab-cli/src/commands/pose_library/mod.rs`
  - `crates/lab-cli/src/commands/pose_library/line.rs`
  - `crates/lab-cli/src/commands/pose_library/tests.rs`

**Contract excerpt.**
- The camera row fields and `join_camera` are PC-21b's (above).
- Server yaw is `atan2(dx, dz)`. The target position is derived (D-PC9): `tx = player_x - r·cos b`, `tz = player_z - r·sin b`, where `r` is `radius_m` and `b` is `bearing_deg` in radians.
- The spec keys: `pitch_deg` -78.75 to 78.75, `yaw_offset_deg` -180 to 180, `zoom` 100 to 775.

**Change.**

1. `human/camera.rs`, append:
   ```rust
   #[derive(Debug, Clone, PartialEq)]
   pub struct CameraKeys { pub pitch_deg: f64, pub yaw_offset_deg: Option<f64>, pub zoom: f64 }
   /// pitch_deg = pitch_offset_deg rounded to 1 dp, clamped to -78.7..=78.7.
   /// zoom = zoom clamped to 100..=775.
   /// yaw_offset_deg only when camera_x/z and camera_yaw_deg are present:
   ///   want = atan2(tx - camera_x, tz - camera_z) in degrees;
   ///   off = wrap to (-180, 180] of (want - camera_yaw_deg);
   ///   negated unless camera_gain < 0; rounded to 1 dp.
   pub fn camera_keys(pose: &Row, cam: &Row) -> Option<CameraKeys>; // None without zoom or pitch_offset_deg
   ```
   This mirrors lab record (`crates/lab/src/uat/record/pose.rs:63-85`, and `yaw_error` in `crates/lab/src/supervisor/world/geometry.rs:96`). Read them, but do not import them: `cimmeria-lab` is a bin crate (F12).
2. `line.rs`: add `pub fn with_camera(line: PoseLine, keys: &[CameraKeys], min_n: usize) -> (PoseLine, Vec<String>)`.
   - With at least `min_n` keys: `pitch_deg` = median, `zoom` = median, and `yaw_offset_deg` = the circular mean of the present values, wrapped to (-180, 180]. Set `yaw_offset_deg` only if at least `min_n` values are present.
   - Otherwise leave the line unchanged and add the note "camera: only <k> joined rows".
   - Append `camera=<k>` to `calibrated`.
3. `pose_library/mod.rs`: add `--camera <camera.jsonl>` and `--join-window-ms <ms>` (default 1000; it sets `JoinWindows::client_ms`). Add one `after_help` example line using `--camera`, within 76 columns.
   - The camera file holds both camera targets, as PC-05's export writes them; `join_camera` filters by `client_target`, so pass the same rows as `cams` and `sent`.
   - With `--camera`, join every pose row of the chosen group, collect the `CameraKeys`, and call `with_camera`.
   - Print a note with the join counts per method: "camera joined <k> of <n> (seq <a>, server_time <b>, client_time <c>)". `--json` adds the same counts as `camera_join`.

**Tests.**
- `human/camera_keys_tests.rs`:
  - `zero_camera_pose_gives_no_yaw_offset`: `camera_pose_zero` with no `camera_x` gives `yaw_offset_deg == None`, while pitch and zoom are present.
  - `yaw_offset_points_at_derived_target`: a camera at the stand looking exactly at the target (`camera_yaw_deg = want`) gives 0. Adding 20° to `camera_yaw_deg` gives +20 for a positive gain and -20 for a gain below 0.
  - `pitch_and_zoom_are_clamped`.
- `pose_library/tests.rs`:
  - `with_camera_needs_min_n`: below `min_n`, the line is unchanged and the note is added.
  - `camera_note_counts_each_method`: `run_with_date` on `tempfile` pose and camera files, with one seq join and one server-time join, prints `seq 1, server_time 1, client_time 0`.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-21c;`:

1. `cargo fmt -p cimmeria-lab-cli --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-lab-cli --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab-cli`
4. `git diff --check`

**Ship:** one PR covering PC-21, PC-21b and PC-21c, from `pc-21c`.

---

## PC-24 Server-only pose replay, live-DB (wave 4b)

- **Agent:** rust-gameserver-dev, because it adapts the `praxis_start_live_db` setup.
- **Worktree:** `pc-24`, from `origin/main` after PC-23 merges. **Branch:** `player-capture-bots/pc-24-pose-replay-live-db`. **Crate:** `cimmeria-wireclient` (tests only).
- **Commit subject:** `test(wireclient): replay a recorded interaction pose against a live server`.
- **Files:**
  - `crates/wireclient/tests/it/pose_replay_live_db.rs` (new; split it into a folder if it passes 400 lines)
  - `crates/wireclient/tests/it/fixtures/pose_replay/frost_pose.jsonl` (new)
  - `crates/wireclient/tests/it/fixtures/pose_replay/frost_approach.jsonl` (new)
  - `crates/wireclient/tests/it/main.rs` (one `mod` line)

**Change.**

1. **Fixtures.** Build them by hand from the Frost row in `docs/guides/automated-uat.md`:
   - `tag = "ArmYourself_FrostBody"`, `radius_m = 4.16`, `bearing_deg = 322.5`, `dy_m = 0.13`.
   - Take Frost's seed position from `db/resources/` (the praxis test finds Frost by template; reuse its template id).
   - Compute `player_x/y/z` from D-PC9 inverted: `p = t + [r·cos b, dy, r·sin b]`.
   - Use one approach row with 3 points walking in from 12 m.
   - Use only ids and content names: `session_kind = "player"`, `pose_seq = 1`, and no personal names.
2. **The test,** `pose_replay_frost_is_accepted_live_db`, starts with `require_db_or_skip!()`.
   - Use the next free sentinel account in the 0x7000_9C00..FF block. 0x01, 0x02, 0x04 and 0x05 are taken; take 0x7000_9C06 unless `git grep 0x7000_9C06` finds it in use.
   - Set it up as `praxis_start_live_db` does: `start_server`, an account with GM level for `gm_goto_xyz`, and a fresh Praxis character in the world.
   - Run `replay::parse_cases` on the fixtures.
   - `gm_goto_xyz` to the first waypoint, then send `avatar_update_explicit` along `replay::waypoints(&case)`.
   - Find the target through the mirror by template, nearest to `case.target`, within 2 m.
   - Send `interact`.
   - Assert the server's acceptance, using the same evidence the praxis test uses for Frost (the interaction or dialog it observes). Tear down with `cleanup_account`.
3. **Revert proof,** in the same test or a second one: move the stand to `radius_m = 6.5` (outside the server's 5.0 m 3D range), replay, and assert that no acceptance arrives within the praxis test's wait budget. A replay that ignored the recorded stand would pass both, so this proves the test is not theatre.
4. **Module doc:** say that this is server-only (idea 9). It does not cover client hit-testing, camera or UI; `lab calibrate` covers those.

**Lane:** `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-24; pwsh tools/build-lane/live-db-test.ps1 --wireclient pose_replay`. Also run `cargo fmt -p cimmeria-wireclient --check` and `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-wireclient --all-targets -- -D warnings`. Load the `lane-build` skill first. The test database is `sgw_pc_24`.

**Docs owed:** add one line to the test inventory row for wireclient live-DB tests in `TESTING.md` or its linked inventory, whichever lists `praxis_start_live_db`.

---

## PC-25 Data runs (after PC-07b, with the user's approval)

- **When:** waves 2-4 are merged and PC-07b has passed. Exporting reads colo ClickHouse, so **the orchestrator asks the user first** (releases are pre-authorized; exports and lab use are not).
- **Agent:** a general-purpose agent, read-only on the colo. Exports go to the scratchpad, **never into the repo**.
- **Steps:**
  1. Export the four JSONL inputs with the recipes in `docs/operations/signoz/interaction-pose-views.md`. The camera export holds both `client.camera.at_interact` and `client.camera.sent_seq` rows.
  2. Run `lab coverage`, then `lab pose-library --camera` for the 10 most-used tags, then `lab friction-drafts`. Record each tag's join counts per method.
  3. Write `docs/analysis/player-capture-bots/worknotes/PC-25.md` (CRLF, ids only) with:
     - the coverage table;
     - each proposed pose line next to the spec line it would replace;
     - the friction draft list.
- **Then:** the orchestrator shows the proposals to the user. Approved pose lines go into specs through a documentation-writer packet, and `lab calibrate` confirms them (lab use, so ask). Approved `.bug` drafts are filed by a separate packet.

---

## PC-09 Close-out

- **Agent:** documentation-writer.
- **Worktree:** `pc-09`. **Branch:** `player-capture-bots/pc-09-close-out`.
- **Commit subject:** `docs(player-capture-bots): close-out`.
- **Files:**
  1. `docs/guides/unified-uat.md`: add one step to the hand-written list (not a gen block) for each live check still owed: PC-07, PC-19, PC-07b and PC-25, each marked done or not run.
  2. `docs/project-status.md` and the matching `docs/gap-analysis*` row: update them once.
  3. The lab-cli guide that lists `lab` commands (find it with `git grep -n "friction-drafts" -- docs`): rows for `friction-drafts`, `coverage` and `pose-library`.
  4. `docs/analysis/player-capture-bots/README.md`: set every ledger row.
  5. `.claude/agent-memory/architect/player-capture-bots.md`: one fact note (the join shares per method and the seq-vs-clock error from PC-07b, whether the `Channel::send` hooks held live (PC-19), the S-CAM verdict, and the camera-row privacy shape).
- **Then:** the orchestrator writes an empty `.claude/state/active-campaign` and asks `ops` to retire every `pc-*` worktree (`rm-worktree.ps1 --merged`) and post the board handoff.
