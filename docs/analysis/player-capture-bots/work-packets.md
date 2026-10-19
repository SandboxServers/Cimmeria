# Player capture work packets: rules, contract, index

> Type: work packets. Ledger and decisions: [README.md](README.md). Packets live in one file per wave:
>
> - [Wave 1](work-packets-w1.md): PC-01, PC-02, PC-06, PC-10, PC-11
> - [Wave 2](work-packets-w2.md): PC-03, PC-04, PC-12, PC-13, PC-14a, PC-14b
> - [Wave 3a, DLL camera](work-packets-w3-camera.md): PC-15, PC-16, PC-16b
> - [Wave 3a, join keys](work-packets-w3-join.md): PC-26, PC-27, PC-17
> - [Wave 3b and the live gates](work-packets-w3b-docs-gates.md): PC-05, PC-18, plus PC-07, PC-19, PC-07b and the release procedure
> - [Wave 4, tools](work-packets-w4-tools.md): PC-20, PC-21, PC-21b, PC-21c, PC-22, PC-23, PC-24, plus PC-25 and PC-09

## Dispatch rules (for the orchestrator)

- **Worktrees.** One per packet: `pwsh tools/build-lane/mk-worktree.ps1 player-capture-bots/<packet>-<slug> pc-<nn>`. A worktree lands at `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-<nn>`. PC-14a's worktree is `pc-14a`, and the same pattern applies to every lettered packet.
- **Reports.** Each agent writes its report to `<worktree>\.claude\reports\player-capture-bots\<packet>-<role>.md` and returns one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.
- **Stacked packets.** These pairs ship as one PR from the child's worktree: PC-01→PC-02, PC-12→PC-13, PC-14a→PC-14b, PC-15→PC-16→PC-16b, PC-27→PC-17, PC-20→PC-21→PC-21b→PC-21c and PC-23→PC-24. The child's first command fast-forwards onto the parent's branch. Retire the parent worktree after the merge.
- **Wave 1.**
  - PC-01, PC-06, PC-10 and PC-11 go at once.
  - PC-02 goes when PC-01 returns.
  - One packet-review-fixer reviews the wave.
- **Ship wave 1** as four PRs: PC-01+02, PC-06, PC-10 and PC-11.
- **Wave 2 starts from `origin/main` after wave 1 merges.**
  - PC-03, PC-04, PC-12 and PC-14a go at once.
  - PC-13 goes after PC-12, and PC-14b after PC-14a.
- **Wave 3a starts after wave 2 merges and PC-06 has returned (D-PC21).**
  - Give PC-15, PC-16 and PC-18 the S-CAM verdict line from PC-06's report: `S-CAM: A` or `S-CAM: B`.
  - PC-15, PC-26 and PC-27 go at once. PC-16 goes after PC-15, PC-16b after PC-16, and PC-17 after PC-27.
  - Three PRs: PC-15+16+16b from `pc-16b`, PC-26 alone, and PC-27+17 from `pc-17`. One packet-review-fixer reviews the wave; server-authority-enforcer gives PC-27 and PC-17 a read-only pass.
- **Wave 3b starts after wave 3a merges:** PC-05 and PC-18 (documentation-writer), two PRs.
- **Wave 4 starts after PC-07 confirms the row shape on real data.** If PC-07 shows a field drift, the architect amends the contract first.
- **Live gates ([procedure](work-packets-w3b-docs-gates.md#release-procedure-pre-authorized-d-pc22)).**
  - **Releases are pre-authorized (owner, 2026-10-18; D-PC22).** Ops cuts the colo release (`/release`) and the launcher release (`/release-launcher`) once the PRs they carry are merged. Post the comment from PowerShell (`gh pr comment <PR> --body "/release"`), because Git Bash rewrites `/release` into a path. Then verify the comment and the workflow runs.
  - **Still ask the user first:** any lab use (PC-19, PC-25's calibrate step) and any colo data export (PC-25).
  - Workers never touch the lab, colo or SigNoz.
- **Model use.**
  - packet-coder runs on Haiku, and its packets are sized for about 40k working tokens.
  - PC-24 goes to rust-gameserver-dev, because L-size runner packets stall Haiku.
  - PC-06 goes to game-archaeology-specialist; PC-05 and PC-18 go to documentation-writer.

## Rules every code packet carries (packet-coder skips CLAUDE.md)

Each wave file repeats these rules at its top. They are listed here for reference.

1. **PowerShell only**, never bash. Start every shell call with `Set-Location <your worktree>;`, using the absolute path.
2. **Never run a compiling `cargo` directly.** `cargo fmt` may run bare. `clippy`, `nextest` and `check` go through `pwsh tools/build-lane/lane.ps1 cargo ...`. Run lane checks in the **foreground**, and commit as soon as they pass.
3. **Git limits.** Never use `git stash`, `git worktree prune`, `--no-verify`, push or force. Commit once, locally, with the subject given, ending with the attribution lines from your brief.
4. **File size.** Soft cap 500 lines, hard cap 700. Never grow a file that is already over 700; edit inside existing lines instead. Tests go in sibling files, as the packet says.
5. **Id fields.** Every new `tracing` field ending in `_id` is either paired with a `_name` field or carries a same-line `// nt:id-only <reason>` comment. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-server unpaired_id` stays green. Never set `NT_BASELINE_BLESS`.
6. **Privacy (D-PC2).** No `account_name`, `player_name`, character `entity_name`, IP, hostname or anything address-like on any row this campaign adds. Content names (template, dialog, item, world) are fine.
7. **Tests.** Unit tests are in-crate `#[test]` or `#[tokio::test]` with no live DB, unless the packet says live-DB. Each test fails if the change it guards is reverted; give it a one-line doc comment saying why.
8. **Docs.** Everything under `docs/` is CRLF; keep it CRLF. Never hand-edit a `<!-- gen:NAME -->` block.
9. **Scope.** Edit only the files the packet names. If a quoted line has moved, find it by its code, not its line number. If something the packet assumes is false, stop and return `blocked`, with the reason in the report.

## Contract

These names are shared across packets. Never rename them (D-PC6).

### `interaction.pose` (PC-01, PC-02; written by PC-03, PC-04)

| Thing | Value |
|---|---|
| Module | `crates/cell-world/src/cell/interaction_pose/`; path `cimmeria_cell_world::cell::interaction_pose` |
| Target, level | `"interaction.pose"`, INFO, `event = "interaction_pose"`, message `"interaction.pose: player pose at a successful interaction"` |
| Throttle | at most one row per actor `entity_id` per `POSE_MIN_INTERVAL` = 250 ms (D-PC5) |
| Yaw | server yaw = `atan2(dx, dz)`; `CellEntity.direction` is a `Vector3` with `.x` = pitch, `.y` = yaw, `.z` = roll, in radians |
| Bearing (D-SV3) | `bearing_deg = atan2(pz - tz, px - tx)` in degrees, wrapped to `[0, 360)` |
| Target position (derived, D-PC9) | `tx = player_x - radius_m·cos(bearing)`, `ty = player_y - dy_m`, `tz = player_z - radius_m·sin(bearing)` |
| `pose_seq` | process-global `AtomicU64`, starting at 1, assigned only when a row is admitted |

```rust
// geometry.rs (PC-01)
pub enum InteractionKind { Interact, UseItem }   // as_str: "interact", "use_item"
pub enum InteractionRoute { Trainer, ChainTag, ChainTemplate, Fallback, ItemUsed }
    // as_str: "trainer", "chain_tag", "chain_template", "fallback", "item_used"
pub struct PoseGeometry { pub radius_m: f32, pub dist_3d_m: f32, pub bearing_deg: f32,
    pub dy_m: f32, pub yaw_to_target_deg: f32, pub facing_error_deg: f32 }
pub fn pose_geometry(player: [f32; 3], player_yaw_rad: f32, target: [f32; 3]) -> PoseGeometry;
pub fn wrap_deg_360(deg: f32) -> f32; // [0, 360)
pub fn wrap_deg_180(deg: f32) -> f32; // (-180, 180]
// capture.rs (PC-02; PC-13 adds `approach`)
pub struct InteractionPose { /* PC-02 */ }
pub fn capture_interaction_pose(mgr: &SpaceManager, entity_id: u32, target_entity_id: u32)
    -> Option<InteractionPose>;
// emit.rs (PC-02)
pub const POSE_MIN_INTERVAL: Duration = Duration::from_millis(250);
#[derive(Debug, Clone, Copy, Default)]
pub struct PoseExtra { pub dialog_id: Option<i32>, pub item_type_id: Option<i32> }
pub fn emit_interaction_pose(pose: &InteractionPose, kind: InteractionKind,
    route: InteractionRoute, extra: PoseExtra) -> Option<u64>;              // Instant::now()
pub fn emit_interaction_pose_at(pose: &InteractionPose, kind: InteractionKind,
    route: InteractionRoute, extra: PoseExtra, now: Instant) -> Option<u64>; // None = throttled
```

**Row fields, in this order (29 fields plus the message):**

1. `event`, `kind`, `route`, `cimmeria.session_kind`
2. `entity_id`, `account_id`, `player_id` (all id-only)
3. `space_id`, `world_id`, `world`
4. `player_x`, `player_y`, `player_z`, `player_yaw_deg`, `wire_pitch_deg`
5. `target_entity_id` (id-only), `target_template_id`, `target_template_name`, `target_tag`, `target_yaw_deg`
6. `radius_m`, `bearing_deg`, `dy_m`, `facing_error_deg`
7. `dialog_id`, `dialog_name`, `item_type_id`, `item_name`
8. `pose_seq`

`dist_3d_m` and `yaw_to_target_deg` stay on `PoseGeometry`, but they are never emitted.

### `interaction.approach` (PC-12, PC-13)

```rust
// crates/cell-world/src/cell/space_manager/movement_telemetry/approach.rs (PC-12),
// re-exported as cimmeria_cell_world::cell::space_manager::{ApproachPoint, ApproachTrail}
pub const APPROACH_POINTS: usize = 8;
pub const APPROACH_MIN_INTERVAL: Duration = Duration::from_secs(1);
pub const APPROACH_MAX_AGE: Duration = Duration::from_secs(30);
pub const APPROACH_JUMP_M: f32 = 10.0;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApproachPoint { pub pos: [f32; 3], pub yaw_deg: f32, pub age_s: f32 }
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ApproachTrail { pub points: Vec<ApproachPoint> } // oldest first
impl ApproachTrail {
    pub fn span_s(&self) -> f32;      // first.age_s - last.age_s, 0 when < 2 points
    pub fn path_len_m(&self) -> f32;  // sum of 3D segment lengths
    pub fn encode(&self) -> String;   // "x,y,z,yaw,age;..." x/y/z 2 dp, yaw and age 1 dp
}
impl MovementTelemetry {
    pub(crate) fn note_approach_point(&mut self, entity_id: u32, pos: [f32; 3], yaw_deg: f32, now: Instant);
    pub fn approach_trail(&self, entity_id: u32, now: Instant) -> Option<ApproachTrail>;
}
```

The row has target `"interaction.approach"` and level INFO. It is written right after an admitted pose row whose capture had a trail. Message: `"interaction.approach: the player's path to a successful interaction"`.

Fields, in this order: `event = "interaction_approach"`, `pose_seq`, `cimmeria.session_kind`, `entity_id` (id-only), `player_id` (id-only), `world`, `target_entity_id` (id-only), `target_tag`, `target_template_id`, `target_template_name`, `n_points`, `span_s`, `path_len_m`, `points`.

### Addendum A1: `mercury_seq` on `interaction.pose` (PC-26)

The frozen list above stays as it is; PC-26 appends one field after it (D-PC19).

- **Field 30:** `mercury_seq`, after `pose_seq`. That makes 30 fields plus the message, inside `tracing`'s cap of 32.
- **Value:** the Mercury `packet_seq` the cell router received with the same actor's `interact` (cell method 74) or `useItem` (cell method 39) call. It is absent when no such call arrived in the last 5 s or the router had no sequence.
- **Exactness:** exact for `interact`, whose handler runs inside the router call. Best effort for `useItem`, whose pose row waits for the base's `ItemUsed` and takes the newest `useItem` call by that actor within 5 s.
- **Not an `_id` field**, so it needs no `nt:id-only` comment.

### `client.camera.at_interact` and `client.camera.sent_seq` (PC-15, PC-16, PC-16b, PC-17)

- The DLL emits `at_interact` at level `info`, once per outbound `interact` or `useItem`, throttled per (method, entity).
- The governor keeps every `client.camera.*` target (`KeepReason::InteractionCamera`, `"interaction_camera"`).
- **`send_id` (PC-16b):** a u32 on `at_interact`, unique per DLL process, drawn from the same counter as `client.ability.sent`. Absent until PC-16b.
- **`client.camera.sent_seq` (PC-16b):** level `info`, one per camera send whose bundle went out. Fields: `send_id`, `method`, `mercury_seq_first`, `mercury_seq_last`, `packets`, `seq_bits` (28), and `evicted_unmatched` when non-zero. No `press_id` or `ability_id`.
- **Upload stamp (PC-16, D-PC18):** every upload POST carries the header `X-Cimmeria-Client-Sent-Ms`, the DLL's wall clock in unix ms on the same clock as `ts_ms`.

Bag fields:

| Field | Meaning |
|---|---|
| `method` | `"interact"` or `"useItem"` |
| `entity_id` | the local player's entity id, which is also the server's |
| `zoom` | the camera distance, 100..775 |
| `yaw_offset_deg`, `pitch_offset_deg` | raw offset units × 360/65536. The pitch offset is unwrapped. |
| `camera_flags` | u32; 0x4 inverts yaw, 0x8 inverts pitch |
| `camera_gain` | f32 |
| `camera_pose_zero` | bool |
| `camera_x`, `camera_y`, `camera_z`, `camera_yaw_deg` | server metres and server yaw degrees; **absent when `camera_pose_zero`** |
| `pawn_x`, `pawn_y`, `pawn_z`, `pawn_yaw_deg`, `pawn_pitch_deg` | **S-CAM A only**; absent when the pawn cannot be read |

Client to server mapping: `server = (client.y/100, client.z/100, client.x/100)`. Rotator yaw degrees = `units·360/65536`, wrapped to [0, 360); it is already the server yaw convention.

- **Ingest (PC-17).** `admin-api` replays `client.camera.*` in its own shape under target `client.native`, at INFO.
- **Lifted fields.** It lifts the bag's numeric fields as attributes. The identity and stamp fields are the same as in the game shape. `entity_id` is id-only, and there is no `entity_name`.
- **Server time (PC-27, PC-17):** each replayed camera row carries `server_ts_ms` (its `ts_ms` on the server clock, capped at the chunk's receive time) and `clock_offset_ms` when admin-api could measure the session's offset. The offset comes from the upload stamp when present, else from the chunk's newest row.
- **Join to `interaction.pose` (D-PC20),** best first, each recording its method:
  1. **Sequence:** camera row → `sent_seq` row (same `session_id`, `send_id`) → pose row with the same `entity_id`, matching kind, `mercury_seq` inside the range (28-bit wrap, `seq_in_range`), and times within 60 s.
  2. **Server time:** nearest pose row with the same `entity_id` and kind, `timestamp` within 500 ms of `server_ts_ms`.
  3. **Fallback:** for rows without `server_ts_ms`, raw `ts_ms` within `--join-window-ms` (default 1000).

### Export format (PC-14a, PC-20 to PC-23)

The input is a JSONL file, one flat JSON object per line. The keys are the tracing field names above plus `timestamp` (RFC 3339 or integer ms). A number may arrive as a JSON number or a numeric string. A missing key means absent. ClickHouse produces it with `FORMAT JSONEachRow` (PC-05 documents the query). No tool queries SigNoz, and no host goes into the repo.

## Packet index

| Packet | File | Crate or area | Size |
|---|---|---|---|
| PC-01, PC-02 | w1 | `cimmeria-cell-world` | S, S |
| PC-06 | w1 | RE docs | domain |
| PC-10 | w1 | `cimmeria-cell-world` | S |
| PC-11 | w1 | `cimmeria-cell-world`, `cimmeria-server`, three test crates, docs | S |
| PC-03 | w2 | `cimmeria-cell-methods` | S |
| PC-04 | w2 | `cimmeria-cell` | S |
| PC-12, PC-13 | w2 | `cimmeria-cell-world` | S, S |
| PC-14a, PC-14b | w2 | `cimmeria-lab-cli` | S, S |
| PC-15, PC-16, PC-16b | w3-camera | `cimmeria-client-telemetry` (i686) | S, S, S |
| PC-26 | w3-join | `cimmeria-cell-world`, `cimmeria-cell` | S |
| PC-27, PC-17 | w3-join | `cimmeria-admin-api` | S, S |
| PC-05 | w3b | docs | domain |
| PC-18 | w3b | docs | domain |
| PC-20, PC-21, PC-21b, PC-21c | w4 | `cimmeria-lab-cli` | S, S, S, S |
| PC-22 | w4 | `cimmeria-lab-cli` | S |
| PC-23 | w4 | `cimmeria-wireclient` | S |
| PC-24 | w4 | `cimmeria-wireclient` live-DB | domain |
