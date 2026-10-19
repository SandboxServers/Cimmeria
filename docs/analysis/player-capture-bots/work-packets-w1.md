# Player capture work packets: wave 1

> Type: work packets. Rules, contract and index: [work-packets.md](work-packets.md). Ledger: [README.md](README.md).

Wave 1 contains PC-01, PC-02 (stacked on PC-01), PC-06, PC-10 and PC-11. Everything starts from `origin/main`. One packet-review-fixer reviews the whole wave.

## Rules for every packet in this file

1. **PowerShell only**, never bash. Start every shell call with `Set-Location <your worktree>;`, using the absolute path.
2. **Never run a compiling `cargo` directly.** `cargo fmt` may run bare. `clippy` and `nextest` go through `pwsh tools/build-lane/lane.ps1 cargo ...`, in the **foreground**. Commit as soon as they pass.
3. **Git limits.** Never use `git stash`, `git worktree prune`, `--no-verify`, push or force. Commit once, locally, with the given subject and your brief's attribution lines.
4. **File size.** Soft cap 500 lines, hard cap 700. Never grow a file over 700; edit inside existing lines instead.
5. **Id fields.** Every new `tracing` field ending in `_id` is paired with a `_name` field or carries a same-line `// nt:id-only <reason>`. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-server unpaired_id` stays green. Never set `NT_BASELINE_BLESS`.
6. **Privacy (D-PC2).** No `account_name`, `player_name`, character `entity_name` or IP on `interaction.pose`.
7. **Tests.** Unit tests (`#[test]` or `#[tokio::test]`, no live DB). Each one fails when its change is reverted, and says why in a one-line doc comment. Use a distinct entity id per test, because the throttles are process-global.
8. **Docs.** Docs under `docs/` are CRLF.
9. **Scope.** Edit only the named files. Find moved lines by their quoted code. If an assumption is false, return `blocked` and give the reason in the report.

---

## PC-01 Pose geometry and types

- **Agent:** packet-coder.
- **Worktree:** `pc-01`. **Branch:** `player-capture-bots/pc-01-pose-geometry`. **Crate:** `cimmeria-cell-world`.
- **Commit subject:** `feat(cell-world): interaction pose geometry (bearing, radius, facing error)`.
- **Files:**
  - `crates/cell-world/src/cell/mod.rs` (one line)
  - `crates/cell-world/src/cell/interaction_pose/mod.rs` (new)
  - `crates/cell-world/src/cell/interaction_pose/geometry.rs` (new)
  - `crates/cell-world/src/cell/interaction_pose/geometry_tests.rs` (new)

**Contract excerpt.**

- Server yaw is `atan2(dx, dz)`, measured from +z toward +x.
- Bearing (D-SV3) is `atan2(pz - tz, px - tx)` in degrees, wrapped to `[0, 360)`. The stand point is the target plus r·(cos b, 0, sin b).
- Labels: `InteractionKind` is `"interact"` or `"use_item"`. `InteractionRoute` is `"trainer"`, `"chain_tag"`, `"chain_template"`, `"fallback"` or `"item_used"`.

**Change.**

1. In `crates/cell-world/src/cell/mod.rs`, add `pub mod interaction_pose;` in alphabetical order. It goes next to `pub mod playtest_friction;`'s neighbours.
2. `interaction_pose/mod.rs`:
   ```rust
   //! `interaction.pose`: where a player stood, and which way they faced, when an
   //! interaction succeeded. Feeds the lab's stand-off poses (D-SV3) from real play.
   //! Campaign: docs/analysis/player-capture-bots/.
   mod geometry;
   #[cfg(test)]
   mod geometry_tests;

   pub use geometry::{
       pose_geometry, wrap_deg_180, wrap_deg_360, InteractionKind, InteractionRoute, PoseGeometry,
   };
   ```
3. `geometry.rs`:
   - The two enums derive `Debug, Clone, Copy, PartialEq, Eq` and have `pub const fn as_str(self) -> &'static str`.
   - `PoseGeometry` derives `Debug, Clone, Copy, PartialEq` and has the fields from the contract.
   - The functions:
   ```rust
   pub fn wrap_deg_360(deg: f32) -> f32 {
       let w = deg.rem_euclid(360.0);
       if w >= 360.0 { 0.0 } else { w } // rem_euclid can round up to 360.0
   }
   pub fn wrap_deg_180(deg: f32) -> f32 {
       let w = wrap_deg_360(deg);
       if w > 180.0 { w - 360.0 } else { w }
   }
   pub fn pose_geometry(player: [f32; 3], player_yaw_rad: f32, target: [f32; 3]) -> PoseGeometry {
       let (dx, dy, dz) = (player[0] - target[0], player[1] - target[1], player[2] - target[2]);
       let radius_m = (dx * dx + dz * dz).sqrt();
       let yaw_to_target_deg = wrap_deg_360((-dx).atan2(-dz).to_degrees()); // atan2(tx-px, tz-pz)
       PoseGeometry {
           radius_m,
           dist_3d_m: (dx * dx + dy * dy + dz * dz).sqrt(),
           bearing_deg: wrap_deg_360(dz.atan2(dx).to_degrees()), // D-SV3
           dy_m: dy,
           yaw_to_target_deg,
           facing_error_deg: wrap_deg_180(yaw_to_target_deg - player_yaw_rad.to_degrees()),
       }
   }
   ```
   Every item gets a doc comment stating its convention. A positive facing error means the target is clockwise of the heading, seen from above with +y up.

**Tests.** `geometry_tests.rs`, unit tests, starting with `use super::geometry::*;`.

- `frost_stand_off_matches_the_lab_pose`. Player `[-325.0, 73.6, -212.8]`, target `[-328.30, 73.472, -210.27]`, yaw 0.
  - `radius_m` is within 0.01 of 4.158, `bearing_deg` within 0.1 of 322.5, and `dy_m` within 0.001 of 0.128.
  - `dist_3d_m >= radius_m`.
  - Swapping the `atan2` arguments gives about 307.5, so the test fails.
- `facing_the_target_gives_zero_error`. Use `player_yaw_rad = g.yaw_to_target_deg.to_radians()` from a first call. Then `|facing_error_deg| < 0.01`.
- `facing_error_wraps_across_north`. Player `[0,0,0]`, target `[0,0,5]`. Yaw 20° gives -20, and yaw 340° gives +20, each ±0.01.
- `cardinal_bearings`. The target is at the origin. Each result is ±0.01:

  | Player | Bearing |
  |---|---|
  | `[3,0,0]` | 0 |
  | `[0,0,3]` | 90 |
  | `[-3,0,0]` | 180 |
  | `[0,0,-3]` | 270 |
- `wrap_bounds`:
  - `wrap_deg_360(-37.5) == 322.5` and `wrap_deg_360(360.0) == 0.0`.
  - `wrap_deg_180(190.0) == -170.0`, `wrap_deg_180(-190.0) == 170.0` and `wrap_deg_180(180.0) == 180.0`.
- `labels_are_stable`. Every `as_str` value equals its contract string.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-01;`:

1. `cargo fmt -p cimmeria-cell-world --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-world --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-world interaction_pose`
4. `git diff --check`

**Docs owed:** none; PC-05 writes them.

---

## PC-02 Capture, throttle and emitter (row v2)

- **Agent:** packet-coder.
- **Worktree:** `pc-02`. **Branch:** `player-capture-bots/pc-02-pose-emit`. **Crate:** `cimmeria-cell-world`.
- **Stacked on PC-01.** First command: `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-02; git merge --ff-only player-capture-bots/pc-01-pose-geometry`. If the fast-forward fails, return `blocked`.
- **Commit subject:** `feat(cell-world): interaction.pose row with per-player throttle`.
- **Files:** in `crates/cell-world/src/cell/interaction_pose/`:
  - `mod.rs`
  - `capture.rs` (new)
  - `emit.rs` (new)
  - `emit_tests.rs` (new)

**Contract excerpt.** The row has target `"interaction.pose"` at INFO, with `event = "interaction_pose"` and the message `"interaction.pose: player pose at a successful interaction"`. Its fields, **in this order** (29 fields plus the message; `tracing` allows 32 including the message):

1. `event`, `kind`, `route`, `cimmeria.session_kind`
2. `entity_id`, `account_id`, `player_id`
3. `space_id`, `world_id`, `world`
4. `player_x`, `player_y`, `player_z`, `player_yaw_deg`, `wire_pitch_deg`
5. `target_entity_id`, `target_template_id`, `target_template_name`, `target_tag`, `target_yaw_deg`
6. `radius_m`, `bearing_deg`, `dy_m`, `facing_error_deg`
7. `dialog_id`, `dialog_name`, `item_type_id`, `item_name`
8. `pose_seq`

Never emit `dist_3d_m`, `yaw_to_target_deg`, target x/y/z or any target name except the template name.

`emit_interaction_pose_at` returns `Option<u64>`: the row's `pose_seq`, or `None` when the row was throttled.

**Change.**

1. `mod.rs`: add `mod capture; mod emit; #[cfg(test)] mod emit_tests;`, then:
   ```rust
   pub use capture::{capture_interaction_pose, InteractionPose};
   pub use emit::{emit_interaction_pose, emit_interaction_pose_at, PoseExtra, POSE_MIN_INTERVAL};
   ```
2. `capture.rs`:
   ```rust
   use super::geometry::{pose_geometry, wrap_deg_180, wrap_deg_360, PoseGeometry};
   use crate::cell::space_manager::SpaceManager;
   const UNKNOWN_WORLD: &str = "unknown";
   #[derive(Debug, Clone, PartialEq)]
   pub struct InteractionPose {
       pub entity_id: u32,
       pub account_id: Option<u32>,
       pub player_id: Option<i32>,
       pub session_kind: Option<&'static str>,
       pub space_id: Option<u32>,
       pub world_id: Option<i32>,
       pub world: String,
       pub player_pos: [f32; 3],
       pub player_yaw_deg: f32, // wrap_deg_360(direction.y.to_degrees())
       pub wire_pitch_deg: f32, // wrap_deg_180(direction.x.to_degrees()); meaning: PC-06
       pub target_entity_id: u32,
       pub target_template_id: Option<i32>,
       pub target_tag: Option<String>,
       pub target_yaw_deg: f32, // wrap_deg_360(target.direction.y.to_degrees())
       pub geometry: PoseGeometry,
   }
   ```
   `capture_interaction_pose(mgr, entity_id, target_entity_id)` returns `None` in three cases: `entity_id == target_entity_id`, either `mgr.get_entity` misses, or the actor's `is_player` is false. Otherwise it fills the struct as follows.

   | Fields | Source |
   |---|---|
   | `account_id`, `player_id` | `let id = mgr.player_identity(entity_id);`. **Never read `id.account_name` or `id.player_name`.** |
   | `space_id` | `mgr.get_entity_space_id(entity_id)` |
   | `world_id` | `mgr.get_entity_world_id(entity_id)` |
   | `world` | `space_id.and_then(\|s\| mgr.world_name_for_space(s))`, else `UNKNOWN_WORLD` |
   | positions | `e.position.x/.y/.z` |
   | `geometry` | `pose_geometry(player_pos, e.direction.y, target_pos)` |
3. `emit.rs`:
   ```rust
   static LAST_ROW: LazyLock<Mutex<HashMap<u32, Instant>>> = LazyLock::new(Mutex::default);
   static POSE_SEQ: AtomicU64 = AtomicU64::new(1);
   const PRUNE_AT: usize = 2048;
   fn admit(entity_id: u32, now: Instant) -> bool {
       let mut map = LAST_ROW.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
       if map.len() >= PRUNE_AT {
           map.retain(|_, at| now.saturating_duration_since(*at) < POSE_MIN_INTERVAL);
       }
       match map.get(&entity_id) {
           Some(at) if now.saturating_duration_since(*at) < POSE_MIN_INTERVAL => false,
           _ => { map.insert(entity_id, now); true }
       }
   }
   ```
   `emit_interaction_pose_at` returns `None` without logging when `admit` refuses. Otherwise it takes `let seq = POSE_SEQ.fetch_add(1, Ordering::Relaxed);`, writes one `tracing::info!(target: "interaction.pose", ...)` with the fields in contract order, and returns `Some(seq)`.
   - `world = %pose.world`.
   - `entity_id`, `account_id`, `player_id` and `target_entity_id` each take a same-line `// nt:id-only privacy: no personal names on interaction.pose (D-PC2)`.
   - If the scan flags `world_id`, mark it `// nt:id-only named by world`.
   - `target_template_name = pose.target_template_id.and_then(|t| cimmeria_names::book().template(t))`. `dialog_name` and `item_name` work the same way, through `.dialog(d)` and `.item(i)`.
   - `Option` fields go in as they are, so an absent value stays absent and never becomes `0`.
   - The geometry fields come from `pose.geometry` (`radius_m`, `bearing_deg`, `dy_m`, `facing_error_deg`).
   - `emit_interaction_pose` calls `_at` with `Instant::now()`.
   - The doc comment states the volume budget: humans interact a few times a minute, and the throttle caps a click-spammer at 4 rows/s.

**Tests.** `emit_tests.rs`, using `crate::test_support::{make_space_manager, LogCapture}` and `tracing::Level`. Each test uses its own actor id, from 9201 to 9207.

Scene helper:
1. `let mut mgr = make_space_manager();`
2. `mgr.create_entity(id, "Agnos", [3.3, 0.128, -2.53], [0.0; 3])`, then on `get_entity_mut(id)` set `is_player = true`, `account_id = Some(6)`, `player_id = Some(12)`, `session_kind = Some("player")`, and `direction.y` = a yaw in radians.
3. `let npc = mgr.allocate_npc_id(); mgr.spawn_npc(npc, "Agnos", [0.0; 3], [0.0; 3])`, then set `tag = Some("PoseProbe".into())` and `template_id = Some(700)`.

The tests:

- `capture_reads_both_poses`:
  - `radius_m` is about 4.158 and `bearing_deg` about 322.5.
  - `target_tag == Some("PoseProbe")`, `session_kind == Some("player")` and `account_id == Some(6)`.
  - `world_id == mgr.get_entity_world_id(id)`.
- `capture_skips_npc_actor_self_and_missing`: capture returns `None` for an actor with `is_player = false`, for a target that is the actor, and for a missing target.
- `row_carries_pose_and_no_names`:
  - `LogCapture::install()`, then `emit_interaction_pose_at(..., Instant::now())` returns `Some(_)`.
  - Find the row with `target == "interaction.pose"`. It is INFO and has `event`, `kind` and `route`. Its `fields["cimmeria.session_kind"] == "player"`, and `radius_m` parses to about 4.158.
  - None of `account_name`, `player_name`, `entity_name`, `target_entity_name`, `target_x` or `dist_3d_m` is a key.
- `throttle_drops_rows_inside_250ms`: emit at `t0` (`Some`), `t0+100ms` (`None`) and `t0+300ms` (`Some`, with a larger seq). Two rows for that id.
- `extra_fields_are_named`: `PoseExtra { dialog_id: Some(1), item_type_id: Some(42) }` writes both keys. `PoseExtra::default()` writes neither.
- `row_has_at_most_31_fields`: the captured row's `fields.len()` (excluding the message) is ≤ 31. This guards the `tracing` cap.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-02;`:

1. `cargo fmt -p cimmeria-cell-world --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-world --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-world`
4. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-server unpaired_id`
5. `git diff --check`

**Ship:** one PR covering PC-01 and PC-02, from `pc-02`.

---

## PC-06 RE: what rotation the client sends, and the S-CAM verdict

- **Agent:** game-archaeology-specialist.
- **Worktree:** `pc-06`. **Branch:** `player-capture-bots/pc-06-wire-rotation-re`.
- **Tools:** Ghidra only. No lab, and no x64dbg on a live client.
- **Commit subject:** `docs(re): avatarUpdate yaw and pitch source (pawn or view rotation)`.
- **Files:**
  - `docs/reverse-engineering/findings/avatar-update-rotation-source.md` (new, CRLF)
  - `docs/protocol/position-updates.md`: at most 30 added lines, as a "Rotation source" subsection linking the finding
  - one fact file in `.claude/agent-memory/game-archaeology-specialist/`, with no addresses beyond function VAs

**Read first:**
- `docs/protocol/position-updates.md`
- `docs/reverse-engineering/findings/position-movement-wire-formats.md`
- `.claude/agent-memory/`: grep `addMove`, `de1720` and `camera`
- `crates/lab/src/supervisor/world/memory.rs:240-429`: the controller and camera vtables, the camera offsets, and `decode_pose` (pawn rotation at actor +0xE8)

**Answer, with function VAs and decompile excerpts:**

1. Which rotator reaches `ServerConnection::addMove` (0x00dd9330) and the packer 0x00de1720: the pawn's, the controller's (view) or the camera's? Name the caller chain.
2. Is the pitch byte ever non-zero for a walking avatar?
3. Does the camera yaw offset (`ASGWCamera_Player` +0x36C) feed the sent yaw?
4. Does the client send `avatarUpdateExplicit` while standing still and only turning the camera?
5. Does the pawn's rotation (`pawn + 0xE8`, `[[[0x01ef244c]+0x0c]+0x08]` is the pawn) match the sent yaw? PC-15 reads it there.

**Output.**
- A table with one row per wire byte: client source, meaning, and confidence (proven or inferred).
- A paragraph titled "What `player_yaw_deg` and `wire_pitch_deg` mean on `interaction.pose`".
- **A last line of the report, exactly `S-CAM: A` or `S-CAM: B`.**
  - A means the wire carries the pawn rotation, so the camera aim is unknown to the server.
  - B means the wire carries the view rotation.
  - If the evidence is inconclusive, write `S-CAM: A (inconclusive)` and name the one lab probe that would settle it. Do not run it.

**Checks:**
- `git diff --check`
- CRLF on both docs: `[IO.File]::ReadAllText(<abs path>).Contains("`r`n")`

---

## PC-10 `session_kind` on `movement.position_sample`

- **Agent:** packet-coder.
- **Worktree:** `pc-10`. **Branch:** `player-capture-bots/pc-10-position-sample-kind`. **Crate:** `cimmeria-cell-world`.
- **Commit subject:** `feat(cell-world): tag movement.position_sample with cimmeria.session_kind`.
- **Files:**
  - `crates/cell-world/src/cell/space_manager/movement_telemetry/position_sample.rs` (120 lines)
  - `crates/cell-world/src/cell/space_manager/tests/movement_validation/telemetry_sampling.rs` (331 lines)

**Why.** Lab clients teleport and walk scripted lines. The walked-surface map must be able to drop them with `cimmeria.session_kind = 'player'`, the same filter every other session row uses (TG-NET-17).

**Change.**

1. In `sample_accepted_position_at`, after `let identity = self.player_identity(entity_id);`, add:
   ```rust
   let session_kind = self.get_entity(entity_id).and_then(|e| e.session_kind);
   ```
2. In the `tracing::debug!(target: "movement.position_sample", ...)` call, add `cimmeria.session_kind = session_kind,` right after `event = "position_sample",`. Change nothing else on the row.

**Test** in `telemetry_sampling.rs`:
- Name: `position_sample_carries_session_kind`. Its doc comment says that without the field, lab walks pollute the walked-surface map.
- Copy the shape of the test that calls `player_in(&mut mgr, 7103, "Agnos")` (around line 185), using id **7110**.
- After `player_in`, set `mgr.get_entity_mut(7110).unwrap().session_kind = Some("lab");`, then sample once at `[11.0, 0.0, 20.0]`.
- Assert the one `sample_rows(&capture)` row has `fields.get("cimmeria.session_kind") == Some("lab")`.
- In a second block, a player with `session_kind = None` (id 7111) has no such key.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-10;`:

1. `cargo fmt -p cimmeria-cell-world --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-world --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-world telemetry_sampling`
4. `git diff --check`

**Docs owed:** none; PC-05 notes it in the catalog.

---

## PC-11 Demote `playtest.friction` from WARN to DEBUG

- **Agent:** packet-coder.
- **Worktree:** `pc-11`. **Branch:** `player-capture-bots/pc-11-friction-debug`.
- **Crates:** `cimmeria-cell-world`, `cimmeria-server`, `cimmeria-cell`, `cimmeria-cell-content` and `cimmeria-cell-methods`.
- **Commit subject:** `fix(telemetry): playtest.friction at DEBUG so it stops posting to Discord`.

**Why (D-PC11).** Friction rows are behaviour signals, not errors, but at WARN they reach the Discord errors channel. The path:
1. `crates/server/src/logging/mod.rs:233-241` installs `DiscordLayer` behind `EnvFilter` `warn`.
2. `crates/discord/src/layer/mod.rs:92-96` returns early below WARN.
3. Colo enables warnings (`docker/compose.discord.yml:87`).
4. `crates/discord/src/router.rs:59` sends them to the errors channel.

At DEBUG they still reach SigNoz through their own `OTEL_FILTER` row. They leave `server.log`, which is INFO and up; nothing reads them there.

**Step 0: verify, don't edit.** Read the four places above.
- If `DiscordLayer` is **not** behind a WARN-or-higher filter, return `blocked: discord path differs`, with what you found.
- If any non-test code outside the two friction modules reads `playtest.friction` rows, also return `blocked: discord path differs`. Check with `git grep -n "playtest.friction" -- crates tools docker`; only comments and tests may match.

**Change.**

1. In `crates/cell-world/src/cell/playtest_friction.rs`, change the 4 `tracing::warn!(` calls (near lines 130, 170, 210 and 251) to `tracing::debug!(`.
2. In `crates/cell-world/src/cell/playtest_friction_watch/mod.rs` (605 lines), change the 6 `tracing::warn!(` calls (near lines 300, 326, 347, 368, 532 and 571) to `tracing::debug!(`. No other change to the row fields.
3. Module doc of `playtest_friction.rs`, line 19: change "WARN" to "DEBUG". Add one sentence: "DEBUG, not WARN, so the Discord layer never posts it (D-PC11); `OTEL_FILTER` exports it." If `playtest_friction_watch/mod.rs`'s module doc says WARN, fix it the same way.
4. In `crates/server/src/logging/filters.rs` (**734 lines, over the hard cap: add no line**), change the line `                movement.position_sample=debug,\` to `                movement.position_sample=debug,playtest.friction=debug,\`.
5. Tests that expect WARN for a friction row move to `Level::DEBUG`:
   - `crates/cell-world/src/cell/playtest_friction.rs`, around line 337. Rename `repeat_dead_end_interacts_warn_once` to `repeat_dead_end_interacts_log_once`, and update its doc comment ("one row, not zero and not 76").
   - `crates/cell-world/src/cell/playtest_friction_watch/tests.rs`, around line 324 (`friction_rows_carry_session_kind`).
   - `crates/cell/src/cell/service/base_messages/player_init/tests/session_start_kind.rs`, around line 144.
   - `crates/cell-content/src/cell/content/executor/tests/mission.rs`, around line 242: the `objective_open_at_mission_complete` filter. That is a friction row; confirm it with the test's target before changing it.
   - `crates/cell-methods/src/cell/cell_methods/player/interaction/dialog_choice_gate_tests.rs`, around line 388: the `find_message(Level::WARN, "friction: a dialog was replaced")` **negative** assertion. Switch it to DEBUG. Left at WARN, it passes vacuously.
   - Run each crate's tests (below). Fix any other friction test that still expects WARN, and list it in the report.
6. Docs, CRLF, editing inside existing lines only:
   - `docs/architecture/observability-target-catalog.md`:
     - line 104, the `playtest.friction` row: level `WARN` becomes `DEBUG`;
     - line 592: "every `playtest.friction` and `movement.navmesh` warning" becomes "every `playtest.friction` row (DEBUG since D-PC11) and `movement.navmesh` warning";
   - `docs/architecture/dev-console-channel.md` line 48: "is a WARN" becomes "is a DEBUG row".

**Revert proof.**
- Restoring `warn!` fails the five level-filtered tests.
- Dropping the `OTEL_FILTER` entry fails `every_source_target_reaches_signoz_at_its_level` (`crates/server/src/logging/target_scan_tests.rs:344`).

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-11;`:

1. `cargo fmt -p cimmeria-cell-world -p cimmeria-server -p cimmeria-cell -p cimmeria-cell-content -p cimmeria-cell-methods --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-world -p cimmeria-server --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-world friction`
4. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-server target_scan`
5. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell session_start_kind`
6. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-content mission`
7. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-methods dialog_choice_gate`
8. `git diff --check`

**Docs owed:**
- Doc-update-map rows: observability target catalog and dev console.
- The live SigNoz saved view "Playtest: friction" may filter on WARN. PC-07 checks it; this packet does not touch SigNoz.
