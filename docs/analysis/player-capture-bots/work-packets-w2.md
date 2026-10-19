# Player capture work packets: wave 2

> Type: work packets. Rules, contract and index: [work-packets.md](work-packets.md). Ledger: [README.md](README.md).

Wave 2 contains PC-03, PC-04, PC-12, PC-13 (stacked on PC-12), PC-14a and PC-14b (stacked on PC-14a). It starts from `origin/main` **after wave 1 merges**; PC-03, PC-04 and PC-12 need PC-01 and PC-02 on main. PC-14a has no code dependency. One packet-review-fixer reviews the whole wave.

## Rules for every packet in this file

1. **PowerShell only**, never bash. Start every shell call with `Set-Location <your worktree>;`, using the absolute path.
2. **Never run a compiling `cargo` directly.** `cargo fmt` may run bare. `clippy` and `nextest` go through `pwsh tools/build-lane/lane.ps1 cargo ...`, in the **foreground**. Commit as soon as they pass.
3. **Git limits.** Never use `git stash`, `git worktree prune`, `--no-verify`, push or force. Commit once, locally, with the given subject and your brief's attribution lines.
4. **File size.** Soft cap 500 lines, hard cap 700. Never grow a file over 700. `interaction/mod.rs` (871) and `base_messages/mod.rs` (728) are off limits.
5. **Id fields.** Every new `tracing` field ending in `_id` is paired with a `_name` field or carries a same-line `// nt:id-only <reason>`. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-server unpaired_id` stays green. Never set `NT_BASELINE_BLESS`.
6. **Privacy (D-PC2).** No `account_name`, `player_name`, character `entity_name`, IP or host on any row or tool output this wave adds.
7. **Tests.** Unit tests (`#[test]` or `#[tokio::test]`, no live DB). Each one fails when its change is reverted, and says why in a one-line doc comment. Use a distinct entity id per test, because the pose throttle is process-global.
8. **Docs.** Docs under `docs/` are CRLF.
9. **Scope.** Edit only the named files. Find moved lines by their quoted code. If an assumption is false, return `blocked` and give the reason in the report.

---

## PC-03 Interact call site

- **Agent:** packet-coder.
- **Worktree:** `pc-03`. **Branch:** `player-capture-bots/pc-03-interact-site`. **Crate:** `cimmeria-cell-methods`.
- **Commit subject:** `feat(cell-methods): write interaction.pose on an accepted interact`.
- **Files:**
  - `crates/cell-methods/src/cell/cell_methods/player/interaction/interact.rs` (442 lines; stays under 500)
  - `crates/cell-methods/src/cell/cell_methods/player/interaction/interact_pose_tests.rs` (new)
  - **Do not edit `interaction/mod.rs`.**

**Contract excerpt.**
- `capture_interaction_pose(mgr, entity_id, target_entity_id) -> Option<InteractionPose>` returns `None` for a non-player actor, for self, and for a missing entity.
- `emit_interaction_pose(&pose, kind, route, PoseExtra { dialog_id, item_type_id }) -> Option<u64>`: at most one row per actor per 250 ms. Ignore the return value.
- Routes: `Trainer`, `ChainTag`, `ChainTemplate`, `Fallback`.

**Change** in `handle_interact`, after the line `crate::cell::interactions::pin_interaction_target(space_mgr, entity_id, target_entity_u32);`:

1. Add this at the top:
   ```rust
   use cimmeria_cell_world::cell::interaction_pose::{self, InteractionKind, InteractionRoute, PoseExtra};
   ```
2. Capture before any route runs, because a chain may teleport the player:
   ```rust
   // Where the player stood when the click was accepted (player-capture-bots).
   let pose = interaction_pose::capture_interaction_pose(space_mgr, entity_id, target_entity_u32);
   let mut route = InteractionRoute::Trainer;
   let mut dialog_id: Option<i32> = None;
   ```
3. Track the route in the existing code:
   - After `handled = crate::cell::content::fire_interact_tag(...).await;`, add `if handled { route = InteractionRoute::ChainTag; }`.
   - After `handled = crate::cell::content::fire_interact_template(...).await;`, add `if handled { route = InteractionRoute::ChainTemplate; }`.
   - In the final `if !handled {` block, set `route = InteractionRoute::Fallback;`. Change `let dialog_id = crate::cell::interactions::handle_interact(...)` so it assigns the outer `dialog_id`. Leave the `if let Some(did) = dialog_id` body unchanged.
4. At the very end of the function:
   ```rust
   if let Some(pose) = pose {
       let _ = interaction_pose::emit_interaction_pose(
           &pose, InteractionKind::Interact, route, PoseExtra { dialog_id, item_type_id: None },
       );
   }
   ```
   The early returns (malformed args, dead actor, hostile reroute, out of range) stay before the capture, so they write no row.
5. At the bottom of `interact.rs`, add `#[cfg(test)] #[path = "interact_pose_tests.rs"] mod pose_tests;`.

**Tests.** `interact_pose_tests.rs`, unit tests with `#[tokio::test]`.

- Call `super::handle_interact(PLAYER, &args, &tx, &mut mgr, &engine).await`, with `args = (npc as i32).to_le_bytes()`.
- Copy the scene setup and the `tx` and `engine` construction from the existing tests in `interact.rs` or `interaction/mod.rs`:
  1. `mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])`, then set `is_player = true`, `player_id = Some(12)`, `account_id = Some(6)` and `session_kind = Some("player")`.
  2. `spawn_npc` the target, then set `faction = 1` and call `clear_all_state_flags()`.
- Use a distinct `PLAYER` id per test, 9301 to 9304.
- Count rows with `capture.all().into_iter().filter(|c| c.target == "interaction.pose")`.

The tests:

- `accepted_interact_writes_one_fallback_row`:
  - Neutral NPC at `[3.0, 0.0, 0.0]`, no tag, empty `ChainEngine::new()`.
  - Expect exactly 1 row with `route == "fallback"`, `kind == "interact"`, `radius_m` ≈ 3.0 and `bearing_deg` ≈ 180.0 (the player is on the target's -x side).
  - Removing the emit fails it.
- `chain_tag_interact_reports_chain_tag`:
  - NPC `tag = Some("PoseProbe".into())`. The engine registers a `Trigger::OnInteractTag { entity_tag: "PoseProbe".into() }` chain; copy the `Chain { .. }` literal from `dialog_choice_gate_tests.rs` (the `MissingIdProbe` chain) and give it a new `id`.
  - Expect 1 row with `route == "chain_tag"`.
- `out_of_range_interact_writes_no_row`: NPC at `[100.0, 0.0, 0.0]`. Expect 0 rows.
- `hostile_reroute_writes_no_row`: a live NPC with `faction = 10`. Expect 0 rows.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-03;`:

1. `cargo fmt -p cimmeria-cell-methods --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-methods --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-methods`
4. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-server unpaired_id`
5. `git diff --check`

---

## PC-04 Item-used call site

- **Agent:** packet-coder.
- **Worktree:** `pc-04`. **Branch:** `player-capture-bots/pc-04-item-used-site`. **Crate:** `cimmeria-cell`.
- **Commit subject:** `feat(cell): write interaction.pose when an item is used on an entity`.
- **Files:**
  - `crates/cell/src/cell/service/base_messages/inventory_events.rs` (172 lines)
  - `crates/cell/src/cell/service/base_messages/tests/item_used_pose.rs` (new)
  - `crates/cell/src/cell/service/base_messages/tests/mod.rs` (add `mod item_used_pose;` in alphabetical order, after `mod item_events;`)
  - **Do not edit `base_messages/mod.rs`.**

**Contract excerpt.** As in PC-03, with `InteractionKind::UseItem` and `InteractionRoute::ItemUsed`. D-PC4: write a row only when `target_id` names a live entity other than the user.

**Change** in `handle_item_used` (around line 120). Insert this after the `let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {` block ends and before `content::fire_item_use(`, so a chain that teleports cannot skew the row:

```rust
// D-PC4: only a use aimed at another live entity has a stand-off pose.
if let Ok(target) = u32::try_from(target_id) {
    if let Some(pose) = interaction_pose::capture_interaction_pose(space_mgr, entity_id, target) {
        let _ = interaction_pose::emit_interaction_pose(
            &pose,
            InteractionKind::UseItem,
            InteractionRoute::ItemUsed,
            PoseExtra { dialog_id: None, item_type_id: Some(type_id) },
        );
    }
}
```

Import `use cimmeria_cell_world::cell::interaction_pose::{self, InteractionKind, InteractionRoute, PoseExtra};`. If `type_id` is not an `i32` in this function, convert it with `i32::try_from(..).ok()` and say so in the report.

**Tests.** `tests/item_used_pose.rs`, unit tests with `#[tokio::test]`, starting with `use super::*;` plus `crate::test_support::LogCapture`.

- Build the scene like `item_used_drops_event_when_no_player_id` in `tests/item_events.rs`:
  1. The Castle space.
  2. The player, with `is_player = true`, `player_id = Some(12)` and `session_kind = Some("player")`, then `connect_entity`.
  3. `let npc = mgr.allocate_npc_id(); mgr.spawn_npc(npc, "Castle", [2.0, 0.0, 0.0], [0.0; 3])`.
- Drive it through `handle_base_message(BaseToCellMsg::ItemUsed { entity_id, instance_id: 1001, type_id: 42, target_id }, &tx, &mut mgr, &engine, &[])`, copying the exact call shape from `item_events.rs`.
- Use distinct player ids, 9401 to 9403.

The tests:

- `item_used_on_npc_writes_use_item_row`: `target_id = npc as i32`. Expect 1 row with `kind == "use_item"`, `route == "item_used"`, `item_type_id == "42"` and `radius_m` ≈ 2.0.
- `item_used_without_target_writes_no_row`: `target_id = 0`. Expect 0 rows.
- `item_used_on_self_writes_no_row`: `target_id` is the player's own id. Expect 0 rows.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-04;`:

1. `cargo fmt -p cimmeria-cell --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell base_messages`
4. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-server unpaired_id`
5. `git diff --check`

---

## PC-12 Approach ring

- **Agent:** packet-coder.
- **Worktree:** `pc-12`. **Branch:** `player-capture-bots/pc-12-approach-ring`. **Crate:** `cimmeria-cell-world`.
- **Commit subject:** `feat(cell-world): keep each player's last 8 accepted positions for approach trails`.
- **Files:** all under `crates/cell-world/src/cell/space_manager/`:
  - `movement_telemetry/approach.rs` (new)
  - `movement_telemetry/approach_tests.rs` (new)
  - `movement_telemetry/mod.rs` (342 lines)
  - `movement_telemetry/position_sample.rs` (one call)
  - `mod.rs` (666 lines; edit the existing `pub use movement_telemetry::{...}` only)
  - `tests/movement_validation/telemetry_sampling.rs` (one test)

**Contract excerpt.** These are the `interaction.approach` names from [work-packets.md § Contract](work-packets.md#interactionapproach-pc-12-pc-13):

```rust
pub const APPROACH_POINTS: usize = 8;
pub const APPROACH_MIN_INTERVAL: Duration = Duration::from_secs(1);
pub const APPROACH_MAX_AGE: Duration = Duration::from_secs(30);
pub const APPROACH_JUMP_M: f32 = 10.0;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApproachPoint { pub pos: [f32; 3], pub yaw_deg: f32, pub age_s: f32 }
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ApproachTrail { pub points: Vec<ApproachPoint> } // oldest first
```

**Change.**

1. `approach.rs`. Its module doc says the ring feeds `interaction.approach` (player-capture-bots, idea 2b), is server-side, and drops a trail across a teleport.
   - The four constants, the two types, and `impl ApproachTrail`:
     - `span_s()` is `first.age_s - last.age_s`, or 0.0 when there are fewer than 2 points.
     - `path_len_m()` is the sum of 3D distances between consecutive points.
     - `encode()` joins points with `;`. Each point is `format!("{:.2},{:.2},{:.2},{:.1},{:.1}", x, y, z, yaw_deg, age_s)`.
   - `impl super::MovementTelemetry`:
     ```rust
     pub(crate) fn note_approach_point(&mut self, entity_id: u32, pos: [f32; 3], yaw_deg: f32, now: Instant) {
         let ring = self.approach.entry(entity_id).or_default();
         if let Some(&(at, last, _)) = ring.back() {
             if now.saturating_duration_since(at) < APPROACH_MIN_INTERVAL { return; }
             if dist3(last, pos) > APPROACH_JUMP_M { ring.clear(); } // a teleport breaks the trail
         }
         ring.push_back((now, pos, yaw_deg));
         while ring.len() > APPROACH_POINTS { ring.pop_front(); }
     }
     pub fn approach_trail(&self, entity_id: u32, now: Instant) -> Option<ApproachTrail> {
         // points no older than APPROACH_MAX_AGE, oldest first,
         // age_s = now - at in seconds; None when fewer than 2 points remain
     }
     ```
     `dist3` is a private fn in this file.
2. `movement_telemetry/mod.rs`:
   - After `mod teleport;`, add `mod approach;` and `#[cfg(test)] mod approach_tests;`.
   - Next to the existing `pub use reject::...`, add `pub use approach::{ApproachPoint, ApproachTrail};`.
   - In the `use std::collections::HashMap;` line, also import `VecDeque`.
   - Add a field to `MovementTelemetry`: `/// Approach ring per player, see [`approach`].` and then `approach: HashMap<u32, VecDeque<(Instant, [f32; 3], f32)>>,`.
   - In `forget`, add `self.approach.remove(&entity_id);`. In `tracked`, add `+ self.approach.len()`.
3. `space_manager/mod.rs`: add `ApproachPoint, ApproachTrail` to the existing `pub use movement_telemetry::{HardReject, LogThrottle, MovementTelemetry, ...};` list, in alphabetical order. The file must stay under 700 lines.
4. `position_sample.rs`, in `sample_accepted_position_at`: right after the `if !is_player { return; }` block, and **before** the 5-second sample throttle, add:
   ```rust
   // Approach ring (player-capture-bots): 1 Hz, independent of the 5 s sample.
   let yaw_deg = self.get_entity(entity_id).map_or(0.0, |e| {
       crate::cell::interaction_pose::wrap_deg_360(e.direction.y.to_degrees())
   });
   self.movement_telemetry.note_approach_point(entity_id, position, yaw_deg, now);
   ```

**Tests.** Unit tests.

`approach_tests.rs` starts with `use super::approach::*; use super::MovementTelemetry;`, and uses `MovementTelemetry::default()` and `t0 = Instant::now()`.

- `ring_keeps_the_last_eight`:
  - Feed 12 points at `t0 + i s`, with `x = i`.
  - `approach_trail(1, t0 + 11 s)` has 8 points, x from 4 to 11, oldest first; the last `age_s` is 0.0.
- `points_closer_than_one_second_are_skipped`: points at `t0` and `t0 + 500 ms` give a trail of `None` (only 1 point).
- `a_jump_over_ten_metres_clears_the_ring`:
  - Points at x = 0, 1 and 2, then x = 50, at 1 s spacing. The trail is `None`.
  - Add one more point at x = 51; the trail then has exactly 2 points.
- `old_points_age_out`: 3 points, then `approach_trail` at `last + 31 s` is `None`.
- `encode_span_and_length`:
  - Two points `[1,2,3]` and `[2,2,3]` at yaw 90, 1 s apart.
  - `encode()` equals `"1.00,2.00,3.00,90.0,1.0;2.00,2.00,3.00,90.0,0.0"`, `span_s() == 1.0` and `path_len_m() == 1.0`.
- `forget_drops_the_ring`: after 2 points, `forget(1)` makes `tracked()` drop by 1.

In `telemetry_sampling.rs`, add `the_accept_path_feeds_the_approach_ring`:
- `player_in(&mut mgr, 7120, "Agnos")`.
- Three `mgr.sample_accepted_position_at(7120, [11.0 + i, 0.0, 20.0], t0 + i s)` calls.
- `mgr.movement_telemetry.approach_trail(7120, t0 + 2 s)` has 3 points.
- An NPC (`create_entity` with no `connect_entity`, id 7121) gets `None`.

**Revert proof.**
- Removing the call in `position_sample.rs` fails the sampling test.
- Removing the `forget` line fails `forget_drops_the_ring`, and also the existing lifecycle test that checks `tracked()`.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-12;`:

1. `cargo fmt -p cimmeria-cell-world --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-world --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-world`
4. `git diff --check`

---

## PC-13 `interaction.approach` row

- **Agent:** packet-coder.
- **Worktree:** `pc-13`. **Branch:** `player-capture-bots/pc-13-approach-row`. **Crate:** `cimmeria-cell-world`.
- **Stacked on PC-12.** First command: `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-13; git merge --ff-only player-capture-bots/pc-12-approach-ring`. If the fast-forward fails, return `blocked`.
- **Commit subject:** `feat(cell-world): interaction.approach row joined to interaction.pose by pose_seq`.
- **Files:** in `crates/cell-world/src/cell/interaction_pose/`:
  - `capture.rs`
  - `emit.rs`
  - `approach_emit_tests.rs` (new)
  - `mod.rs` (one `#[cfg(test)] mod approach_emit_tests;` line)

**Contract excerpt.**
- The row has target `"interaction.approach"` at INFO, with the message `"interaction.approach: the player's path to a successful interaction"`.
- It is written right after an **admitted** pose row whose capture had a trail. It is never written when the pose row was throttled.
- Fields, in this order:
  1. `event = "interaction_approach"`, `pose_seq`, `cimmeria.session_kind`
  2. `entity_id`, `player_id`, `target_entity_id`, all id-only
  3. `world`, `target_tag`, `target_template_id`, `target_template_name`
  4. `n_points`, `span_s`, `path_len_m`, `points` (the `encode()` string)

**Change.**

1. `capture.rs`:
   - Add `pub approach: Option<ApproachTrail>` to `InteractionPose`, importing it from `crate::cell::space_manager::ApproachTrail`.
   - In `capture_interaction_pose`, set `approach: mgr.movement_telemetry.approach_trail(entity_id, std::time::Instant::now())`.
   - Fix every struct literal of `InteractionPose` in the existing tests (`approach: None`).
2. `emit.rs`: in `emit_interaction_pose_at`, after the pose row and before `Some(seq)`, write the approach row when the trail is present:
   ```rust
   if let Some(trail) = pose.approach.as_ref() {
       tracing::info!(
           target: "interaction.approach",
           event = "interaction_approach",
           pose_seq = seq,
           cimmeria.session_kind = pose.session_kind,
           entity_id = pose.entity_id, // nt:id-only privacy: no personal names (D-PC2)
           player_id = pose.player_id, // nt:id-only privacy: no personal names (D-PC2)
           target_entity_id = pose.target_entity_id, // nt:id-only privacy: no personal names (D-PC2)
           world = %pose.world,
           target_tag = pose.target_tag.as_deref(),
           target_template_id = pose.target_template_id,
           target_template_name = /* same lookup as the pose row */,
           n_points = trail.points.len(),
           span_s = trail.span_s(),
           path_len_m = trail.path_len_m(),
           points = %trail.encode(),
           "interaction.approach: the player's path to a successful interaction"
       );
   }
   ```
   Update the doc comment of `emit_interaction_pose_at` to mention the second row.

**Tests.** `approach_emit_tests.rs`, unit tests. Use `crate::test_support::{make_space_manager, LogCapture}` and the scene from `emit_tests.rs`, with actor ids 9211 to 9213.

- `pose_row_with_trail_writes_a_joined_approach_row`:
  1. Seed the ring through `mgr.sample_accepted_position_at(id, p, t)` three times. Use `t = Instant::now().checked_sub(Duration::from_secs(3 - i)).unwrap()` and `p` 1 m apart.
  2. Capture and emit.
  3. Expect one `interaction.approach` row whose `pose_seq` equals the pose row's `pose_seq`, with `n_points == "3"` and a `points` string containing two `;`.
  4. There are no `account_name`, `player_name` or `entity_name` keys.
- `no_trail_writes_no_approach_row`: a player with no samples gives one pose row and zero approach rows.
- `throttled_pose_writes_no_approach_row`: emit twice within 100 ms with a trail. Expect one pose row and one approach row.

**Revert proof.** Removing the approach block fails the first test. Moving it before the throttle check fails the third.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-13;`:

1. `cargo fmt -p cimmeria-cell-world --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-world --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-world`
4. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-server unpaired_id`
5. `git diff --check`

**Ship:** one PR covering PC-12 and PC-13, from `pc-13`.

---

## PC-14a JSONL rows and friction clustering

- **Agent:** packet-coder.
- **Worktree:** `pc-14a`. **Branch:** `player-capture-bots/pc-14a-friction-clusters`. **Crate:** `cimmeria-lab-cli`.
- **Commit subject:** `feat(lab-cli): read exported telemetry rows and cluster human friction`.
- **Files:**
  - `crates/lab-cli/src/lib.rs` (one line: `pub mod human;`, in alphabetical order after `pub mod help;`)
  - `crates/lab-cli/src/human/mod.rs` (new)
  - `crates/lab-cli/src/human/rows.rs` (new)
  - `crates/lab-cli/src/human/friction.rs` (new)
  - `crates/lab-cli/src/human/tests.rs` (new)

**Contract excerpt (D-PC15, export format).**
- The input is JSONL: one flat JSON object per line.
- The keys are the tracing field names plus `timestamp`.
- A number may be a JSON number or a numeric string. A missing key means absent.
- Blank lines are skipped.
- No tool queries SigNoz, and no host goes into the repo.

**Change.**

1. `human/mod.rs` gets the module doc "Offline readers for telemetry exported from ClickHouse as JSONL (player-capture-bots, D-PC15)", then `pub mod friction; pub mod rows; #[cfg(test)] mod tests;`.
2. `human/rows.rs`:
   ```rust
   #[derive(Debug, Clone, PartialEq)]
   pub struct Row(pub serde_json::Map<String, serde_json::Value>);
   impl Row {
       pub fn str(&self, key: &str) -> Option<&str>;   // a JSON string only
       pub fn f64(&self, key: &str) -> Option<f64>;    // a number, or a string that parses
       pub fn i64(&self, key: &str) -> Option<i64>;    // same, integral
       pub fn text(&self, key: &str) -> Option<String>;// a string, or a number as text
   }
   pub fn parse_jsonl(text: &str) -> Result<Vec<Row>, String>; // "line N: <serde error>"; a non-object line is an error
   pub fn read_jsonl(path: &std::path::Path) -> Result<Vec<Row>, String>; // std::fs::read_to_string, then parse_jsonl
   ```
3. `human/friction.rs`:
   ```rust
   pub const PERSONAL_KEYS: &[&str] = &["account_name", "player_name", "entity_name", "target_entity_name"];
   #[derive(Debug, Clone, PartialEq)]
   pub struct FrictionCluster {
       pub signal: String, pub reason: String, pub world: String, pub subject: String,
       pub count: usize, pub players: usize,
       pub first_seen: Option<String>, pub last_seen: Option<String>,
       pub sample: Row, // the first row of the cluster with PERSONAL_KEYS removed
   }
   pub fn cluster_friction(rows: &[Row]) -> Vec<FrictionCluster>;
   ```
   `cluster_friction` works as follows:
   - **Keep** rows where `signal` is present and `cimmeria.session_kind` is `"player"`. Lab and unknown sessions are dropped (D-PC1).
   - **Key** each row by `signal`, `reason` (or `"-"`), `world` (or `"-"`) and `subject`. `subject` is the first present of `tag`, `template_name`, `item_id`, `step_id`, `mission_id`, `region_tag` and `dialog_id`, through `text()`, else `"-"`.
   - **Count** distinct players by `player_id` through `text()`. A row without one counts as its own player.
   - **Times:** `first_seen` and `last_seen` are the min and max `timestamp` through `text()`, compared as strings.
   - **Sort** by `count` descending, then by key, so the order is stable.

**Tests.** `human/tests.rs`, unit tests with inline JSONL strings and no files.

- `numbers_may_be_strings`: `{"radius_m":"4.15","n":3}` gives `f64("radius_m") == Some(4.15)` and `i64("n") == Some(3)`.
- `bad_line_names_its_number`: the second line `{oops` gives an `Err` that contains `"line 2"`.
- `blank_lines_are_skipped`.
- `friction_clusters_by_signal_world_subject`:
  - Five player rows: three `repeat_interact_no_effect` / `Agnos` / `tag=Frost` from two players, and two `step_stalled` / `Agnos` / `step_id=77`.
  - Plus one `lab` row for Frost.
  - Expect 2 clusters. Frost is first, with `count == 3` and `players == 2`. The lab row is excluded.
- `sample_drops_personal_names`: a row carrying all four `PERSONAL_KEYS` yields a cluster `sample` that has none of them.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-14a;`:

1. `cargo fmt -p cimmeria-lab-cli --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-lab-cli --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab-cli human`
4. `git diff --check`

---

## PC-14b `lab friction-drafts`

- **Agent:** packet-coder.
- **Worktree:** `pc-14b`. **Branch:** `player-capture-bots/pc-14b-friction-drafts`. **Crate:** `cimmeria-lab-cli`.
- **Stacked on PC-14a.** First command: `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-14b; git merge --ff-only player-capture-bots/pc-14a-friction-clusters`. If the fast-forward fails, return `blocked`.
- **Commit subject:** `feat(lab-cli): lab friction-drafts turns human friction into bug cards`.
- **Files:**
  - `crates/lab-cli/src/commands/friction_drafts/mod.rs` (new)
  - `crates/lab-cli/src/commands/friction_drafts/card.rs` (new)
  - `crates/lab-cli/src/commands/friction_drafts/tests.rs` (new)
  - `crates/lab-cli/src/commands/mod.rs` (414 lines; about 8 lines added)

**Contract excerpt (D-PC12).**
- One markdown bug card per friction cluster, written to `--out`.
- A card holds the facts and a commented, **UNCONFIRMED** TOML row skeleton in the shape of `crates/lab/src/uat/record/fixtures/fs_p3_draft.toml`. Read that file first.
- Nothing is written into `docs/guides/uat-specs/`.
- No personal names in a card.

**Change.**

1. `commands/mod.rs`:
   - Add `pub mod friction_drafts;` in alphabetical order.
   - Add a `CommandInfo` with `name: "friction-drafts"`, `group: Group::RunAndRecord` and `summary: "Turn exported human friction rows into bug cards"`, after `calibrate`.
   - Add `/// Turn exported human friction rows into bug cards` and `FrictionDrafts(friction_drafts::Args),` to `Command`, plus the dispatch arm `Command::FrictionDrafts(args) => friction_drafts::run(args, ctx),`.
2. `friction_drafts/mod.rs`: copy the `calibrate` pattern (`const LONG`, `const EXAMPLES`, `#[derive(clap::Args, Debug)] #[command(about, long_about = LONG, after_help = EXAMPLES)]`).
   - Args:
     - `rows: PathBuf` (positional): the JSONL export.
     - `--out <dir>`, required.
     - `--top <n>`, default 20.
     - `--min-count <n>`, default 2.
     - `--json`.
   - `EXAMPLES` is `"Examples:\n  lab friction-drafts friction.jsonl --out drafts\n  lab friction-drafts friction.jsonl --out drafts --top 5 --json"`. Each line is at most 76 columns, and the first word after `lab` is `friction-drafts`.
   - `LONG` explains the following. Read it to the exact meaning:
     - export with the ClickHouse `JSONEachRow` recipe in `docs/operations/signoz/interaction-pose-views.md`;
     - only `session_kind = player` rows count;
     - cards are proposals, and every TOML line in them is commented out;
     - it never writes into the spec tree.
   - `pub fn run(args: &Args, ctx: &Ctx) -> u8`:
     1. `read_jsonl`, then `cluster_friction`. Keep clusters with `count >= min_count`, and take the first `top`.
     2. `create_dir_all(out)`, then write `bug-<nn>-<signal>.md` with `nn` 2-digit and 1-based.
     3. Print one line per card through `anstream::println!`, or with `--json` one JSON array of `{file, signal, world, subject, count, players}`.
     4. Return 0. On a read or write error, call `ctx.ui.error(&msg, None)` and return 1.
3. `friction_drafts/card.rs` has `pub fn render_card(c: &FrictionCluster) -> String`, a pure function. The card:
   - `# Friction: <signal> at <subject> (<world>)`
   - A facts list:
     - signal, reason, world and subject;
     - count and distinct players;
     - first and last seen;
     - the sample's `x`, `z`, `template_id`, `template_name`, `mission_name`, `step_name`, `item_id` and `interaction_flags_names` when present.
   - `## Expected` with `TODO: what should have happened`.
   - `## Draft row (UNCONFIRMED)`, then a fenced `toml` block in which **every line starts with `# `**:
     ```text
     # [[row]]
     # id = "FR-<nn>"
     # title = "<signal> at <subject>"
     # setup = [{ step = "stand", tag = "<subject>" }]   # adjust: see fs_p3_draft.toml
     # step = []
     # [[row.expect]]
     # TODO
     ```
     Copy the key names from `fs_p3_draft.toml`, not from this sketch, if they differ.

**Tests.** `friction_drafts/tests.rs`, unit tests.

- `card_has_facts_and_a_commented_row`: render a cluster built in the test. The card contains the signal, world, subject and count. Every line inside the `toml` fence starts with `# `.
- `card_has_no_personal_names`: a cluster whose `sample` was built from a row with the names already stripped by PC-14a, plus a hand-added `player_name` key. The card output contains no `player_name` text. `render_card` must print only the listed keys.
- `writes_one_card_per_cluster`:
  - Write a 6-row JSONL into `tempfile::tempdir()` (2 clusters ≥ 2, 1 singleton), then call `run` with `--out` inside the tempdir.
  - Build the `Ctx` the way the `calibrate` tests do. If none do, use `Ctx { ui: crate::ui::Ui::plain(), command: "friction-drafts".into(), raw: vec![] }`, or whatever constructor `ui/mod.rs` offers.
  - Expect exactly 2 files: `bug-01-*.md` and `bug-02-*.md`.
- The existing `commands::tests` (the table matches clap, the examples parse and fit 76 columns) must pass. They fail if the `COMMANDS` row or `EXAMPLES` is wrong.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-14b;`:

1. `cargo fmt -p cimmeria-lab-cli --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-lab-cli --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab-cli`
4. `git diff --check`

**Docs owed:** a `friction-drafts` line in the lab CLI command reference, if `crates/lab-cli/README.md` lists the commands. Add it in this packet, inside that file only.

**Ship:** one PR covering PC-14a and PC-14b, from `pc-14b`.
