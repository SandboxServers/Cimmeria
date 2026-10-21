# Cellblock boot lock: work packets

Ledger: [README.md](README.md). Resume card: [RESUME.md](RESUME.md). Designed 2026-10-20.

## Dispatch rules

- One `packet-coder` per packet, each in its own worktree with its own test database. `ops` creates each worktree with `pwsh tools/build-lane/mk-worktree.ps1 <branch> <name>`; worktrees live at `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\<name>`.
- `mk-worktree.ps1` always branches off `origin/main`. For a packet with a dependency, once the dependency is merged into the integration branch, `ops` runs `pwsh tools/build-lane/merge-noff.ps1 -C <wt> -Branch cellblock-boot-lock/integration` in that packet's worktree **before** the coder starts.
- **Wave 1 ships as one PR** from the integration worktree `cbl-integration` (branch `cellblock-boot-lock/integration`). `ops` merges each packet branch into it with `merge-noff.ps1`, in the order in § Waves. A packet merged into integration has no PR of its own.
- **Wave 2 ships one PR per packet.**
- Report path: `<worktree>\.claude\reports\cellblock-boot-lock\<packet>-<role>.md`, where role is `coder`, `review-<N>`, `fix-<N>`.
- Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.
- Commit messages end with these two lines:

  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01WCmH1WPLJdizxaciRv4aJM
  ```
- Packet loop, then wave loop, then **BL-U1 (lab, ask the user first)**, then the wave-1 PR. The PR does not merge before BL-U1 passes (README D-BL10).

## Contract

Every packet builds against these names exactly. Do not rename them.

### C1. Content-engine type (BL-04): `crates/content-engine/src/movement_lock.rs`

```rust
//! What an `Action::MovementLock` does (cellblock-boot-lock D-BL3).
use serde::{Deserialize, Serialize};

/// `max_distance` when the row omits it, in world units (horizontal).
pub const DEFAULT_MAX_DISTANCE: f32 = 3.0;
/// The largest `max_distance` the loader accepts.
pub const MAX_MAX_DISTANCE: f32 = 10.0;
/// The longest `message` the loader accepts, in chars.
pub const MAX_MESSAGE_CHARS: usize = 200;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MovementLockOp {
    /// Pin the acting player to `at` while the prop tagged `anchor_tag` is live
    /// in their space and they stand within `max_distance` of `at`.
    Hold {
        anchor_tag: String,
        at: [f32; 3],
        max_distance: f32,
        /// Also set the ref-counted `BSF_MOVEMENT_LOCK` state flag (Layer A).
        client_flag: bool,
        message: Option<String>,
    },
    /// Remove the hold (if any). `message` is said either way.
    Release { message: Option<String> },
}

impl MovementLockOp {
    /// The `op` param value, for logs.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hold { .. } => "hold",
            Self::Release { .. } => "release",
        }
    }
}
```

The `Action` variant goes in `crates/content-engine/src/actions.rs`, after `SpawnSet(SpawnSetOp),` (line 541):

```rust
    /// Pin the acting player in place, or release them
    /// (cellblock-boot-lock D-BL3); see [`crate::movement_lock::MovementLockOp`].
    MovementLock(crate::movement_lock::MovementLockOp),
```

### C2. Seed row shape (`content_actions`, `action_type = 'movement_lock'`)

`target_id` and `target_key` are `NULL`. `params`:

| Key | Op | Required | Rule |
|---|---|---|---|
| `op` | both | yes | `"hold"` or `"release"` |
| `anchor_tag` | hold | yes | non-empty string, at most 64 chars, ASCII graphic (no spaces) |
| `at` | hold | yes | a JSON array of exactly 3 finite numbers |
| `max_distance` | hold | no, default 3.0 | finite number, `> 0` and `<= 10` |
| `client_flag` | hold | no, default `false` | boolean |
| `message` | both | no | string; trimmed, it must be 1-200 chars |

Anything else drops the row with a `warn!` carrying `chain_id`, `chain_name`, `?params` and `reason = "malformed_movement_lock"`. Unknown extra keys are ignored.

### C3. Cell-world hold state (BL-02): `crates/cell-world/src/cell/space_manager/client_move_lock.rs`

```rust
/// A content movement hold (cellblock-boot-lock D-BL4), stored in
/// `CellEntity::extensions`. Lives as long as the cell entity; a relog
/// rebuilds the entity without it and the seed re-applies it.
#[derive(Debug, Clone)]
pub struct MovementHold {
    pub anchor: [f32; 3],
    pub space_id: u32,
    pub chain_id: i64,
    pub client_flag: bool,
    pub message: Option<String>,
    pub last_snap: Option<Instant>,
    pub last_feedback: Option<Instant>,
}

/// Horizontal (x/z) slack around the anchor, in world units.
pub const HOLD_RADIUS: f32 = 1.0;
/// At most one snap-back per this interval (each is a base DB write).
pub const HOLD_SNAP_INTERVAL: Duration = Duration::from_millis(500);
/// At most one repeat of the hold message per this interval.
pub const HOLD_FEEDBACK_INTERVAL: Duration = Duration::from_secs(5);

impl SpaceManager {
    /// `None`: no hold, or the move is inside the slack. `Some(Locked)`: refuse it.
    pub(super) fn check_movement_hold(
        &mut self,
        now: Instant,
        entity_id: u32,
        space_id: u32,
        proposed: Vector3,
    ) -> Option<ClientMoveOutcome>;
}
```

The new `ClientMoveOutcome` variant:

```rust
    /// A content hold pins this player near `anchor` (see `client_move_lock`).
    /// Nothing was written and the snap-back budget is untouched. The caller
    /// snaps the client to `anchor` when `snap` is true, and sends `feedback`
    /// as one feedback-channel line when it is `Some`.
    Locked {
        anchor: [f32; 3],
        space_id: u32,
        snap: bool,
        feedback: Option<String>,
    },
```

`space_manager/mod.rs` re-exports `pub use client_move_lock::{MovementHold, HOLD_FEEDBACK_INTERVAL, HOLD_RADIUS, HOLD_SNAP_INTERVAL};`.

### C4. Ids, tag, texts

| What | Value |
|---|---|
| Prop template | `1250`, `template_name 'Cellblock - Prison Boot Control'`, `display_name 'Prison Boot Control'`, mesh `Ga-Props.GA-PuzzleStation00` |
| Prop spawn | `284`, world 12, `(-333.03, 73.472, -227.32)`, heading `0.0`, `is_stationary true` |
| Tag | `Cellblock_PrisonBootControl` |
| Hold anchor `at` | `[-334.231, 73.472, -228.026]` (stasis-chamber respawner) |
| Chains | 1023 (hold), 1024 (interact, Livewire), 1025 (victory, release), **1047** (restore click bits), **1048** (GM-completion release) |
| Hold text | `Your prison boot is locked to the floor. Right-click the boot control beside you and win the Livewire game to work it loose.` |
| Release text | `The prison boot unlocks. You can move again.` |
| Counter | `movement_hold_snaps_total`, label `"reason" => "content_hold"` |
| Log events | `event = "movement_lock.hold"` / `"movement_lock.release"` (target `content`); `movement.hold_snap` (target `movement.validation`) |

## Waves

| Wave | Packets | Ships as | Merge order into `cellblock-boot-lock/integration` |
|---|---|---|---|
| 1 | BL-01..BL-07, then gate BL-U1 | one PR from `cbl-integration` | BL-01 → BL-04 → BL-02 → BL-06 → BL-03 → BL-05 → BL-07 |
| 2 | BL-08, BL-U2, BL-09 | one PR per packet | BL-08 → (BL-U2) → BL-09 |

BL-01 and BL-04 start at once. BL-02 starts when BL-01 is integrated. BL-06 starts when BL-04 is integrated. BL-03 starts when BL-02 is integrated. BL-05 starts when BL-02 and BL-04 are integrated. BL-07 starts last.

---

## Wave 1

### BL-01: split `client_move.rs` (no behaviour change)

Host: windows · Branch `cellblock-boot-lock/BL-01-split-client-move` · Worktree `cbl-01` · Crate `cimmeria-cell-world` · Agent: packet-coder

**Why.** `crates/cell-world/src/cell/space_manager/client_move.rs` is 716 lines, over the 700 hard cap. BL-02 adds to it.

**Rules (inline).**
1. PowerShell only. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cbl-01;`.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo ...`, in the foreground.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Files: soft cap 500 lines, hard cap 700. Every file you touch must end at or under 700.
5. This is a pure code move. Change no logic, no doc text beyond the module headers, and no test.
6. Edit only the files listed below.

**Do.**
1. Create `space_manager/client_move_outcome.rs`. Move into it the `ClientMoveOutcome` enum with its doc comment and `#[derive(Debug)]` (now lines 51-109), verbatim. Header: `//! [`ClientMoveOutcome`]: what one client position update came to. Split out of `client_move.rs` (cellblock-boot-lock BL-01).` Imports it needs: `cimmeria_entity::movement_validation::{MovementReject, SpaceBounds}`.
2. Create `space_manager/client_move_recovery.rs`. Move `resolve_recovery_position` with its doc comment (now lines 598-698) into a new `impl SpaceManager { ... }` block there, verbatim, and make it `pub(super) fn`. Add the imports it needs (`cimmeria_common::Vector3`, `cimmeria_entity::movement_validation::{position_within_bounds, SpaceBounds}`, `use super::SpaceManager;`). Header: `//! The terminal-fallback resolver for an unusable snap target. Split out of `client_move.rs` (cellblock-boot-lock BL-01).`
3. In `client_move.rs`: delete the moved code, add `use super::ClientMoveOutcome;`, and drop the imports that are now unused (clippy names them).
4. In `space_manager/mod.rs`: replace line 15 `pub use client_move::ClientMoveOutcome;` with `pub use client_move_outcome::ClientMoveOutcome;`. Below `mod client_move;` (line 33), add `mod client_move_outcome;` and `mod client_move_recovery;`.

**Test.** No new test. The guard is the existing suite: `tests/movement_validation/*` exercises every outcome and the recovery resolver, and it must pass unchanged.

**Lane checks, in order.**
1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-world`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-world --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-world movement_validation`
4. `pwsh tools/build-lane/lane.ps1 cargo check -p cimmeria-cell`. This proves the `ClientMoveOutcome` path still resolves downstream.

Report the line count of all three files.

**Docs.** None.

**Commit.** `BL-01: split client_move.rs into outcome and recovery modules`

---

### BL-04: content-engine `movement_lock` type and loader

Host: windows · Branch `cellblock-boot-lock/BL-04-movement-lock-verb` · Worktree `cbl-04` · Crate `cimmeria-content-engine` · Agent: packet-coder

**Rules (inline).**
1. PowerShell only. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cbl-04;`.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo ...`, in the foreground.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Files: soft cap 500 lines, hard cap 700. `actions.rs` is 652 lines, so add only the variant there.
5. Tests are unit tests (`#[cfg(test)]` in the new loader file). Each must fail if the conversion it pins is reverted.
6. Edit only the files listed below.

**Contract.** C1 and C2 above, verbatim.

**Do.**
1. Create `crates/content-engine/src/movement_lock.rs` with C1's code and a module doc.
2. `crates/content-engine/src/lib.rs`: add `pub mod movement_lock;` to the module list (lines 28-34), in alphabetical order.
3. `crates/content-engine/src/actions.rs`: add the `MovementLock` variant (C1) after `SpawnSet(SpawnSetOp),` at line 541.
4. Create `crates/content-engine/src/loader/action_movement_lock.rs` with `pub(super) fn convert_movement_lock(row: &DbActionRow) -> Option<Action>`.
   - Copy the shape of `loader/action_loot.rs`: the module doc showing the param shape, and a `drop_row` closure that warns with `chain_id`, `chain_name = cimmeria_names::book().chain(row.chain_id)`, `?params`, `reason = "malformed_movement_lock"` and returns `None`.
   - Apply every rule in C2. Read `at` as `params.get("at").and_then(|v| v.as_array())`, which must have length 3, each value `as_f64()` and finite, then cast to `f32`.
   - Trim `message` before checking its length, and store the trimmed string.
5. `crates/content-engine/src/loader/mod.rs`: add `mod action_movement_lock;` between `mod action_mail;` and `mod action_spawn;` (lines 46-47).
6. `crates/content-engine/src/loader/action.rs`: add `"movement_lock" => super::action_movement_lock::convert_movement_lock(row),` on the line after the `"open_loot"` arm (line 305).

**Tests**, in `action_movement_lock.rs` `mod tests`. Build rows like `action_loot.rs`'s `row()` helper (`chain_id: 1023`, `action_type: "movement_lock"`). Convert through `super::super::action::convert_action(&row)`, not the inner function, so that deleting the dispatch arm also fails the tests.
- `converts_a_full_hold`: every key set. Expect `Action::MovementLock(MovementLockOp::Hold { .. })` with exact values.
- `hold_defaults`: only `op`, `anchor_tag` and `at`. Expect `max_distance == DEFAULT_MAX_DISTANCE`, `client_flag == false`, `message == None`.
- `converts_release_with_and_without_message`.
- `trims_the_message`: `"  hi  "` becomes `Some("hi")`.
- `malformed_rows_are_dropped`: a loop over each of these, expecting `None`:
  - a missing `op`;
  - `op: "freeze"`;
  - a hold without `anchor_tag`;
  - an `anchor_tag` of `""`;
  - an `anchor_tag` of `"has space"`;
  - a hold without `at`;
  - `at` with 2 elements;
  - `at` with a string element;
  - `max_distance` of 0;
  - `max_distance` of 10.5;
  - `max_distance` of `"3"`;
  - `client_flag` of `"yes"`;
  - `message` of `"   "`;
  - `message` of 201 chars;
  - `message` of `5`.

Revert proof: deleting the `"movement_lock"` arm makes every positive test return `None`, so they fail. Deleting any one validation turns its malformed row into `Some`, so that test fails.

**Lane checks, in order.**
1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-content-engine`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-content-engine --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine movement_lock`
4. `pwsh tools/build-lane/lane.ps1 cargo check -p cimmeria-cell-content`. This proves the new variant breaks no downstream match; `dispatch.rs` has a catch-all.

**Docs.** None (BL-07 owns them).

**Commit.** `BL-04: content-engine movement_lock verb (type and loader)`

---

### BL-02: `MovementHold` and the `Locked` gate

Host: windows · Branch `cellblock-boot-lock/BL-02-hold-gate` · Worktree `cbl-02` · Crates `cimmeria-cell-world`, `cimmeria-cell` · Agent: packet-coder · Base: integration with BL-01 merged (`ops` runs `merge-noff.ps1` first)

**Rules (inline).**
1. PowerShell only. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cbl-02;`.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo ...`, in the foreground.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Files: soft cap 500 lines, hard cap 700. `space_manager/mod.rs` is about 669 lines after BL-01, so add exactly two lines there.
5. Tests are unit tests at the `SpaceManager` seam (TESTING.md). Each must fail when the gate is reverted.
6. Edit only the files listed below. Check `git log --oneline -3` first: BL-01's commit must be in the history. If it is not, return `blocked`.

**Contract.** C3 above, verbatim.

**Do.**
1. Create `crates/cell-world/src/cell/space_manager/client_move_lock.rs` with:
   - a module doc naming D-BL4;
   - the C3 struct and constants;
   - `check_movement_hold`, using this body shape:

   ```rust
   let entity = self.get_entity_mut(entity_id)?;
   let held_space = entity.extensions.get::<MovementHold>()?.space_id;
   if held_space != space_id {
       // The player left the held space (GM travel, world change): the hold is stale.
       entity.extensions.remove::<MovementHold>();
       return None;
   }
   let hold = entity.extensions.get_mut::<MovementHold>()?;
   let (dx, dz) = (proposed.x - hold.anchor[0], proposed.z - hold.anchor[2]);
   if dx * dx + dz * dz <= HOLD_RADIUS * HOLD_RADIUS {
       return None;
   }
   let snap = hold.last_snap.is_none_or(|t| now.saturating_duration_since(t) >= HOLD_SNAP_INTERVAL);
   if snap {
       hold.last_snap = Some(now);
   }
   let feedback = match &hold.message {
       Some(m) if hold.last_feedback.is_none_or(|t| now.saturating_duration_since(t) >= HOLD_FEEDBACK_INTERVAL) => {
           let m = m.clone();
           hold.last_feedback = Some(now);
           Some(m)
       }
       _ => None,
   };
   Some(ClientMoveOutcome::Locked { anchor: hold.anchor, space_id, snap, feedback })
   ```

   Only the horizontal distance counts, so jumping in place is never refused. If the borrow checker objects, restructure it, but keep this behaviour.
2. `client_move_outcome.rs` (from BL-01): add the C3 `Locked` variant just above `EntityMissing`.
3. `client_move.rs` `apply_client_position_update_at`: directly after the GM-bypass block (`if movement_unrestricted { return self.accept(...); }`), insert:

   ```rust
   // Content movement hold (cellblock-boot-lock D-BL4). After the GM bypass
   // so `/gmsetfly` frees a stuck player; before the layers so a held player
   // never spends the snap-back budget.
   if let Some(locked) = self.check_movement_hold(now, entity_id, space_id, proposed) {
       return locked;
   }
   ```
4. `space_manager/mod.rs`: add `mod client_move_lock;` after `mod client_move;`, and add the C3 `pub use` line next to `pub use client_move_outcome::ClientMoveOutcome;`.
5. `crates/cell/src/cell/service/base_messages/movement.rs`: in the `match outcome` (lines 126-219), add a placeholder arm so the crate compiles. BL-03 replaces it.

   ```rust
   // BL-03 replaces this with the snap-back and feedback line.
   ClientMoveOutcome::Locked { .. } => {}
   ```

**Tests.** Create `crates/cell-world/src/cell/space_manager/tests/movement_validation/locked.rs`, and register `mod locked;` in `movement_validation/mod.rs`, alphabetically between `mod kinematics;` and `mod navmesh;`. Imports: `std::time::{Duration, Instant}`, `crate::cell::space_manager::{ClientMoveOutcome, MovementHold}`, `super::super::make_manager`, `super::SPAWN_POS`.

Fixture: `make_manager()`, `create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])`. Then insert `MovementHold { anchor: SPAWN_POS, space_id: mgr.entity_space[&100], chain_id: 1023, client_flag: false, message: Some("held".into()), last_snap: None, last_feedback: None }` through `mgr.get_entity_mut(100).unwrap().extensions.insert(...)`.

- `a_move_outside_the_slack_is_locked_and_not_written`: a move to `[15.0, 0.0, 20.0]` returns `Locked { anchor: SPAWN_POS, snap: true, feedback: Some("held"), .. }`. `mgr.get_entity(100).unwrap().position` still equals `SPAWN_POS`.
- `a_move_inside_the_slack_falls_through`: a move to `[10.5, 0.0, 20.3]` returns `Accepted`.
- `vertical_movement_is_not_locked`: a move to `[10.0, 2.0, 20.0]` does not match `Locked`.
- `snaps_and_feedback_are_throttled`, driven with `apply_client_position_update_at` and an injected `t0`:
  - `t0` gives `snap: true` and `feedback: Some`;
  - `t0 + 100 ms` gives `snap: false` and `feedback: None`;
  - `t0 + 600 ms` gives `snap: true` and `feedback: None`;
  - `t0 + 5_100 ms` gives `feedback: Some`.
- `no_message_means_no_feedback`: with `message: None`, the outcome is `Locked { feedback: None, snap: true, .. }`.
- `a_hold_from_another_space_is_dropped`: with `space_id: 999`, the move is not `Locked`, and `extensions.contains::<MovementHold>()` is now false.
- `the_gm_bypass_wins`: with `movement_unrestricted = true`, a move to `[15.0, 0.0, 20.0]` returns `Accepted`.
- `a_locked_move_spends_no_snap_back_budget`: send ten locked moves, then remove the hold and send one out-of-bounds move (`[-100_000.0, 0.0, 20.0]`). It must be `Rejected`, not `CorrectionSuppressed`, because the held moves never counted.

Revert proof: delete the gate call in `client_move.rs` and every `Locked` assertion fails (those moves come back `Accepted`).

**Lane checks, in order.**
1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-world`, then `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-world --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell --all-targets -- -D warnings`
4. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-world movement_validation`

**Docs.** None (BL-07).

**Commit.** `BL-02: content movement hold gate (ClientMoveOutcome::Locked)`

---

### BL-06: seeds and live-DB tests

Host: windows · Branch `cellblock-boot-lock/BL-06-seeds` · Worktree `cbl-06` · Crates `cimmeria-cell-content`, `cimmeria-cell-catalog`, `cimmeria-content-engine` · Agent: packet-coder · Base: integration with BL-04 merged

The owner may change the mesh, nameplate or texts (OQ-1, OQ-2). The orchestrator passes any override in the brief. Otherwise use C4 as written.

**Rules (inline).**
1. PowerShell only. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cbl-06;`.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo ...` and `pwsh tools/build-lane/live-db-test.ps1 <filter>`, in the foreground.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Seeds only, in `db/resources/`. Never `db/scripts/` migrations.
5. Every `content_*` row and every `set_interaction_type` row naming the tag stays on **one line**: the interact-tag linter scans line by line.
6. Docs under `docs/` are CRLF. Write them with the Edit tool, which keeps line endings.
7. Files: Rust soft cap 500 lines, hard cap 700.
8. Live-DB tests use `require_db_or_skip!()`. Each new test must fail with its seed edit reverted.
9. Edit only the files listed below. Check `git log --oneline -3` first: BL-04's commit must be in the history. If it is not, return `blocked`.

**Step 0: re-check the ids.** Run `git grep -n -E "\b(1047|1048)\b" -- db/resources/Content` and `git grep -n -E "VALUES \((1250|284)," -- db/resources`. Both must find nothing. If either finds a hit, return `blocked` and list the hits.

**Seeds.**
1. `db/resources/Entities/Seed/entity_templates.sql`: add one line after template 303's row. Copy the 36-column column list from `db/resources/Entities/Seed/entity_templates_beta_e1.sql:27` exactly. Values: `(1250, 'Ga-Props.GA-PuzzleStation00', 'GLB_Components.WorldObject_Small', NULL, 4, 0, NULL, 1, 0, 1, 7550, NULL, NULL, NULL, 'Cellblock - Prison Boot Control', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, NULL, 'Prison Boot Control')`.
   - Above it, add a comment: "cellblock-boot-lock D-BL1; the click bits are set at run time by chain 1047, so the seeded interaction_type stays 0".
   - Before you write the row, compare its column order against template 303's row and the beta_e1 header. If they disagree, follow the beta_e1 header and say so in the report.
2. `db/resources/Worlds/Seed/spawnlist.sql`: after the stasis-room hub rows 400-404 (lines 1080-1088), add a comment block and then this line:

   ```sql
   INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (284, -333.03, 73.472, -227.32, 0.0, 12, 1250, 'Cellblock_PrisonBootControl', NULL, true);
   ```

   The comment says: the Prison Boot Control is at the stasis-room centre, about 1.4 u from the player's spawn (respawner 8); it is cellblock-boot-lock D-BL1; it must stay `is_stationary`.
3. `db/resources/Content/Seed/castle_cellblock_chains.sql`, inside the mission 689 block (lines 74-210):
   1. Rewrite the block header comment (lines 77-147) to say what is now true:
      - the boot lock is a server hold (`movement_lock`, D-BL3/D-BL4);
      - the minigame starts from a right-click on spawn 284 (`interact_tag`), not `item_use 3438`, which the client never sent (README F2);
      - abilities 1597 and 1598 are kept, but are no-ops (F3);
      - there is no archetype condition: Goa'uld (archetype 6) start on the same anchor and are held too (D-BL8, owner 2026-10-20).

      Point to `docs/analysis/cellblock-boot-lock/README.md` and keep it under 40 lines.
   2. Chain 1023. Keep its one condition (`mission_status neq completed`, sort 0) and add no other; there is no archetype condition (D-BL8). Turn the actions into a two-row `VALUES` list: the existing `launch_ability 1597` at sort 0, plus `(1023, 'movement_lock', NULL, NULL, '<hold params>', 0, 1)` with hold params `{"op": "hold", "anchor_tag": "Cellblock_PrisonBootControl", "at": [-334.231, 73.472, -228.026], "max_distance": 3.0, "client_flag": false, "message": "<C4 hold text>"}`. Update the chain description to `'689 - Zone load: (re-)apply Prison Boot hold while not cleared'`.
   3. Chain 1024. Replace the trigger line with `VALUES (1024, 'interact_tag', 'Cellblock_PrisonBootControl', 'player', false, 0);`. Keep its one condition (`mission_status neq completed`, sort 0) and add no other. Description: `'689 - Right-click the Prison Boot Control: start Livewire minigame'`. Rewrite the comment above it.
   4. Chain 1025. Its actions become five single-line rows, in this order:
      - `(1025, 'launch_ability', 1598, NULL, '{}', 0, 0)`
      - `(1025, 'movement_lock', NULL, NULL, '{"op": "release", "message": "<C4 release text>"}', 0, 1)`
      - `(1025, 'set_interaction_type', NULL, 'Cellblock_PrisonBootControl', '{"op": "~", "mask": 256}', 0, 2)`
      - `(1025, 'set_interaction_type', NULL, 'Cellblock_PrisonBootControl', '{"op": "~", "mask": 1073741824}', 0, 3)`
      - `(1025, 'complete_mission', 689, NULL, '{}', 0, 4)`
   5. New chain 1047, after 1025. Its comment explains that the interaction bits are cell-local, the same pattern as chain 1045 (F10).
      - `content_chains`: `(1047, '689 - Zone load: make the Prison Boot Control clickable while not cleared', 'mission', 689, true, 0)`
      - trigger: `(1047, 'player_loaded', 'Castle_CellBlock', 'player', false, 0)`
      - condition, the only one: `(1047, 'mission_status', 689, NULL, 'neq', 'completed', 0)`
      - actions, one line each:
        - `(1047, 'set_interaction_type', NULL, 'Cellblock_PrisonBootControl', '{"op": "|", "mask": 256}', 0, 0)`
        - `(1047, 'set_interaction_type', NULL, 'Cellblock_PrisonBootControl', '{"op": "|", "mask": 1073741824}', 0, 1)`
   6. New chain 1048. Its comment: a GM `complete_mission 689` (or chain 1025's own completion) releases the hold. The release is silent, because chain 1025 already said its line.
      - `content_chains`: `(1048, '689 - Mission completed: release any Prison Boot hold', 'mission', 689, true, 0)`
      - trigger: `(1048, 'mission_completed', '689', 'player', false, 0)`
      - action: `(1048, 'movement_lock', NULL, NULL, '{"op": "release"}', 0, 0)`
   7. File header. Remove `1047-1050` from the "next free inside 1001-1111" list (line 53) and put `1049-1050` in its place. Under the Mission 689 entry (lines 26-33), add: "cellblock-boot-lock (2026-10-20): chains 1047-1048, scanned free on main and on every branch updated in the last 30 days".
4. `docs/analysis/four-zone-planning/id-allocation.md` line 17, the Cellblock row: add `1047-1048 (cellblock-boot-lock)` to the in-use chain list, plus template `1250` and spawn `284`. Change nothing else.

**Rust test edits.**
1. `crates/cell-content/src/cell/content/chain_replay_tests/mission_689.rs`:
   - Delete `assert_item_use_3438_resolves` and its two tests (lines 130-201). Write two tests in their place using `super::livewire_pairs::resolve_interact(&pool, 1024, "Cellblock_PrisonBootControl", &[("mission_689_status", "active")])`. Model them on the other callers of `resolve_interact`: grep `resolve_interact(` under `chain_replay_tests/`.
     - `live_db_chain_1024_starts_livewire_on_right_click_while_locked`: a `StartMinigame` with `"Livewire"` and `on_victory_chains == vec![1025]` resolves.
     - `live_db_chain_1024_does_not_start_once_completed`: with `"completed"`, nothing resolves for 1024.
   - Update the module doc (lines 1-20) for the new 1024 trigger and the 1025 actions.
   - Keep the file under 300 lines.
2. Create `crates/cell-content/src/cell/content/chain_replay_tests/mission_689_boot_lock.rs` and register it in `chain_replay_tests/mod.rs` next to `mod mission_689;`. Use the same imports as `mission_689.rs`, plus `cimmeria_content_engine::movement_lock::MovementLockOp` and `cimmeria_content_engine::triggers::Trigger`. Write a local helper `fire_player_loaded(chain_id, status, archetype: Option<i64>) -> Vec<Action>`, a copy of `assert_player_loaded_resolves` that returns the fired actions and, when it is `Some`, sets `"archetype"` as a JSON number (`serde_json::json!(n)`; the `archetype` condition reads `as_i64`). Because `require_db_or_skip!` returns `()`, the helper takes the pool as an argument and each test calls `require_db_or_skip!()` itself. Tests:
   - `live_db_chain_1023_holds_at_the_stasis_anchor`, with `"active"` and archetype `Some(1)`. The fired actions contain `LaunchAbility { ability_id: 1597, .. }` **and** a `MovementLock(Hold { anchor_tag, at, max_distance, client_flag: false, message: Some(_) })` where `anchor_tag == "Cellblock_PrisonBootControl"`, `at == [-334.231, 73.472, -228.026]` and `max_distance == 3.0`.
   - `live_db_chain_1023_holds_goauld_too`: `"active"` and archetype `Some(6)` fire the same `LaunchAbility 1597` and `MovementLock(Hold { .. })` as above (D-BL8: no archetype exclusion).
   - `live_db_chain_1047_sets_click_bits_while_locked`: `"not_active"` with archetype `None`, and again with archetype `Some(6)`, each fire two `SetInteractionType { entity_tag: "Cellblock_PrisonBootControl", operation: "|", .. }` with masks 256 and 1073741824. `"completed"` fires nothing.
   - `live_db_chain_1025_releases_then_clears_then_completes`: load 1025. Its `actions` are, in order: `LaunchAbility 1598`, `MovementLock(Release { message: Some(_) })`, `SetInteractionType` `~` 256, `SetInteractionType` `~` 1073741824, `CompleteMission { mission_id: 689 }`.
   - `live_db_chain_1048_releases_on_mission_completed`: load 1048. `matches!(chain.trigger, Trigger::OnMissionCompleted { mission_id: 689 })` holds, and `actions == vec![Action::MovementLock(MovementLockOp::Release { message: None })]`.
3. Create `crates/cell-catalog/src/cell/spawner/tests/live_db_boot_control_spawn.rs` and register it in `spawner/tests/mod.rs`, alphabetically after `mod live_db_ammo_loot;`. Copy the shape of `live_db_debug_hub_stationary.rs` (`mod live_db { ... }`, `load_spawns_from_db`). Test `the_boot_control_stands_by_the_hold_anchor`:
   - exactly one Castle_CellBlock spawn has tag `Cellblock_PrisonBootControl`;
   - it has `template_id == 1250` and `is_stationary`;
   - its horizontal (x/z) distance to `const BOOT_ANCHOR_XZ: [f32; 2] = [-334.231, -228.026]` is at most 3.0. The doc comment says this must match chain 1023's `at` (pinned in `mission_689_boot_lock.rs`) and stay within the hold's `max_distance`.
4. `crates/content-engine/tests/it/onitemuse_remove_item_pairing.rs:26`: remove `3438` from `KNOWN_REUSABLES`, and add a one-line comment saying chain 1024 no longer uses `item_use`. If that test then fails because 3438 has another `item_use` chain, put 3438 back and say so in the report.

Revert proof: revert any seed edit and its named test fails:
- 1023's hold row → the hold test;
- 1024's trigger → the right-click test;
- 1025's order → the order test;
- spawn 284 → the cell-catalog test;
- an `archetype` condition added to 1023 or 1047 → `live_db_chain_1023_holds_goauld_too` or the `Some(6)` case of the 1047 test.

**Lane checks, in order.**
1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-content`, then `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-catalog`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-catalog --all-targets -- -D warnings`
4. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine`. This runs the interact-tag linter and the onitemuse pairing test.
5. `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::mission_689`. This covers both files.
6. `pwsh tools/build-lane/live-db-test.ps1 live_db_boot_control_spawn`
7. `pwsh tools/build-lane/live-db-test.ps1 every_stasis_room_spawn_is_stationary`
8. `pwsh tools/build-lane/live-db-test.ps1 live_db_seed_sequences`

**Docs.** `id-allocation.md` only (CRLF). BL-07 owns the rest.

**Commit.** `BL-06: Prison Boot Control prop and hold seeds (chains 1023-1025, 1047, 1048)`

---

### BL-03: `EntityMove` handler: snap-back and feedback for `Locked`

Host: windows · Branch `cellblock-boot-lock/BL-03-hold-snap` · Worktree `cbl-03` · Crate `cimmeria-cell` · Agent: packet-coder · Base: integration with BL-02 merged

**Rules (inline).**
1. PowerShell only. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cbl-03;`.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo ...`, in the foreground.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Files: soft cap 500 lines, hard cap 700. `movement.rs` is 306 lines.
5. Tests are async unit tests through `handle_base_message`, as in `base_messages/tests/movement.rs`. Each must fail with the arm reverted to `{}`.
6. Edit only the files listed below. Check `git log --oneline -3` first: BL-02's commit must be in the history. If it is not, return `blocked`.

**Contract.** C3 `Locked`; C4 counter and log names.

**Do.** In `crates/cell/src/cell/service/base_messages/movement.rs`:
1. Add the imports `use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};` and `use crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;`. `crates/cell/src/cell/service/base_messages/content_ability_grant.rs:22-24` shows the exact import paths, and its lines 175-179 show the send.
2. Replace BL-02's placeholder arm with:

   ```rust
   ClientMoveOutcome::Locked { anchor, space_id, snap, feedback } => {
       // Content movement hold (cellblock-boot-lock D-BL4): the player is
       // pinned to `anchor` until the hold is released. Throttled in the
       // gate, so this runs at most every 500 ms per player.
       if snap {
           cimmeria_observability::counter!(
               "movement_hold_snaps_total",
               "reason" => "content_hold",
           );
           let id = space_mgr.player_identity(entity_id);
           let world = space_mgr.world_name_for_space(space_id);
           tracing::info!(
               target: "movement.validation",
               event = "movement.hold_snap",
               entity_id,
               entity_name = id.player_name,
               account_id = id.account_id,
               account_name = id.account_name,
               player_id = id.player_id,
               player_name = id.player_name,
               space_id,
               world,
               client_x = position[0],
               client_z = position[2],
               anchor_x = anchor[0],
               anchor_z = anchor[2],
               reason = "content_hold",
               "movement.hold_snap: a content hold pinned the player; snapping back to the anchor"
           );
           send_snap_back(entity_id, space_id, world, anchor, id, tx).await;
       }
       if let Some(text) = feedback {
           send_feedback_line(entity_id, &text, tx).await;
       }
   }
   ```

   If `id` or `world` cannot be reused as written because of borrow or move rules, clone them. The log line and the send must keep these fields.
3. Add a private `async fn send_feedback_line(entity_id: u32, text: &str, tx: &mpsc::Sender<CellToBaseMsg>)`. It sends `CellToBaseMsg::EntityMethodCall { entity_id, method_index: ON_PLAYER_COMMUNICATION, args: serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text) }`. On a send error it logs `warn!` with `entity_id`, `error = %e` and `reason = "hold_feedback_send_failed"`. This matches `send_line` in `crates/cell-content/src/cell/content/executor/spawn_set.rs:105-115`.

**Tests.** Create `crates/cell/src/cell/service/base_messages/tests/movement_lock.rs`, and register `mod movement_lock;` in `tests/mod.rs` after `mod movement;`. Fixture: copy `entity_move_out_of_bounds_rejects...` from `tests/movement.rs` lines 54-70, then insert a `MovementHold { anchor: spawn_pos, space_id: mgr.get_entity_space_id(7777).unwrap(), chain_id: 1023, client_flag: false, message: Some("held".into()), last_snap: None, last_feedback: None }` into entity 7777's `extensions`. Import `MovementHold` from `crate::cell::space_manager`.
- `a_held_move_snaps_to_the_anchor_and_says_why`:
  - send `EntityMove` to `spawn_pos + [5, 0, 0]`;
  - drain `rx`: exactly one `CellToBaseMsg::TeleportPlayer { position, prev_pos, .. }`, with both equal to `spawn_pos`;
  - one `EntityMethodCall { method_index: ON_PLAYER_COMMUNICATION, .. }` whose args contain `"held"` UTF-16LE-encoded (copy `says()` from `crates/cell-content/src/cell/content/executor/tests/spawn_set.rs:64-74`);
  - the entity position is unchanged.
- `an_immediate_second_held_move_sends_nothing`: send two moves back to back. The second drain has no `TeleportPlayer` and no `EntityMethodCall`, because the gate throttles both.
- `a_move_inside_the_slack_is_written`: a move to `spawn_pos + [0.5, 0, 0]` sends no `TeleportPlayer`, and the position advances.

Revert proof: replace the arm with `{}` and the first test fails twice, once on the missing `TeleportPlayer` and once on the missing line.

**Lane checks, in order.**
1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell base_messages::tests::movement`

**Docs.** None (BL-07).

**Commit.** `BL-03: snap a held player back to the hold anchor with a feedback line`

---

### BL-05: executor for `movement_lock`

Host: windows · Branch `cellblock-boot-lock/BL-05-movement-lock-executor` · Worktree `cbl-05` · Crate `cimmeria-cell-content` · Agent: packet-coder · Base: integration with BL-02 and BL-04 merged

**Rules (inline).**
1. PowerShell only. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cbl-05;`.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo ...`, in the foreground.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Files: soft cap 500 lines, hard cap 700. `dispatch.rs` is 628 lines, so add only the 3-line arm and one name in its `use` list.
5. `BSF_MOVEMENT_LOCK` is **ref-counted**. Only ever call `update_state_flag`; never write `state_field` raw. A hold sets it at most once, and its release unsets it exactly once.
6. Tests are async unit tests through `execute_actions`, as in `executor/tests/spawn_set.rs`. Each must fail when the behaviour it pins is reverted.
7. Edit only the files listed below. Check `git log --oneline -5` first: BL-02's and BL-04's commits must both be in the history. If either is missing, return `blocked`.

**Contract.** C1 (`MovementLockOp`), C3 (`MovementHold`), C4 (log events).

**Do.**
1. Create `crates/cell-content/src/cell/content/executor/movement_lock.rs` with `pub(super) async fn run(op: MovementLockOp, entity_id: u32, chain_id: i64, tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager)`.
   - Imports:
     - `cimmeria_content_engine::movement_lock::MovementLockOp`;
     - `crate::cell::space_manager::{MovementHold, SpaceManager}`;
     - `crate::cell::ring_transport::wire_helpers::{update_state_flag, BSF_MOVEMENT_LOCK}`;
     - `crate::cell::messages::CellToBaseMsg`;
     - the chat imports `send_line` uses in `spawn_set.rs:16-18`.
   - Copy `spawn_set.rs`'s private `send_line` (lines 104-120). Do not import it.
   - **Not a player** (`space_mgr.get_entity(entity_id).filter(|e| e.is_player)` is `None`): log `warn!` with `event = "movement_lock.hold"` or `"movement_lock.release"` (from `op`), `decision_outcome = "refused"`, `reason = "not_a_player"`, plus `entity_id`, `entity_name = space_mgr.entity_label(entity_id)`, `chain_id` and `chain_name = cimmeria_names::book().chain(chain_id)`. Then return.
   - **Hold**, in order:
     1. If `extensions.contains::<MovementHold>()`, log `debug!` with `decision_outcome = "already_held"` and return. Set nothing and send nothing.
     2. If `space_mgr.find_entity_by_tag(entity_id, &anchor_tag)` is `None`, log `info!` with `decision_outcome = "skipped"`, `reason = "hold_skipped_no_anchor"` and `anchor_tag`, then return. This is fail-open.
     3. Compute the horizontal (x/z) distance from the player's `position` to `at`. If it is more than `max_distance`, log `info!` with `reason = "hold_skipped_far_from_anchor"`, `distance` and `max_distance`, then return.
     4. Read `space_id = *space_mgr.entity_space.get(&entity_id)` (return if missing). Insert `MovementHold { anchor: at, space_id, chain_id, client_flag, message: message.clone(), last_snap: None, last_feedback: message.is_some().then(std::time::Instant::now) }`.
     5. If `client_flag`, call `update_state_flag(entity_id, BSF_MOVEMENT_LOCK, true, tx, space_mgr).await` once.
     6. If `message` is `Some`, call `send_line`.
     7. Log `info!` with `event = "movement_lock.hold"`, `decision_outcome = "held"`, `client_flag`, `anchor_tag`, the player identity fields (`let who = space_mgr.player_identity(entity_id);` as `spawn_set.rs:66-80` does), `chain_id` and `chain_name`.
   - **Release**:
     1. `let removed = extensions.remove::<MovementHold>();`. Check what `remove` returns. If it returns `Option<T>`, use the removed hold's `client_flag`. If it returns `bool`, read the hold with `get` first.
     2. If the removed hold had `client_flag`, call `update_state_flag(..., false, ...)` once.
     3. If `message` is `Some`, call `send_line` whether or not a hold existed. The release is the reward line for winning the minigame.
     4. Log `info!` with `event = "movement_lock.release"`, `decision_outcome = "released"` or `"nothing_held"`, and the identity fields.
2. `executor/mod.rs`: add `mod movement_lock;` to the module list (lines 43-62), alphabetically.
3. `executor/dispatch.rs`: add `movement_lock` to the `use super::{...}` list (lines 13-16), alphabetically. After the `Action::SpawnSet(op)` arm (lines 454-456), add:

   ```rust
   Action::MovementLock(op) => {
       movement_lock::run(op, entity_id, chain_id, tx, space_mgr).await;
   }
   ```

**Tests.** Create `crates/cell-content/src/cell/content/executor/tests/movement_lock.rs`, and register it in `executor/tests/mod.rs` (lines 49-66), alphabetically, with a one-line `//!` entry in that file's doc list. Fixture:
- `make_space_mgr()`;
- the player `PLAYER = 31` created at `[0.0; 3]`, with `is_player = true`, `player_id = Some(4343)` and `connect_entity`;
- a prop entity `32`, `create_entity(32, "Agnos", [1.0, 0.0, 1.0], ...)`, with `get_entity_mut(32).unwrap().tag = Some("BootCtl".into())`.

Use `PLAYER`, `PLAYER_ID` and the `says()` helper as `spawn_set.rs` does. Write `fn hold(at, client_flag) -> MovementLockOp` with `anchor_tag "BootCtl"`, `max_distance 3.0` and message `Some("held")`.
- `hold_pins_the_player_and_says_why`: hold at `[0.0; 3]`. Entity 31 now has `MovementHold { anchor: [0.0; 3], chain_id: <the chain id you passed>, .. }`, and the line contains `"held"`.
- `hold_is_skipped_far_from_the_anchor`: hold at `[10.0, 0.0, 0.0]` inserts no hold, sends no line, and logs `reason = "hold_skipped_far_from_anchor"` (`LogCapture`).
- `hold_is_skipped_without_the_prop`: clear entity 32's tag first. No hold is inserted, and the log says `hold_skipped_no_anchor`.
- `a_second_hold_does_not_double_count_the_flag`:
  - hold with `client_flag: true` twice, then release once;
  - `get_entity(PLAYER).unwrap().state_field & BSF_MOVEMENT_LOCK == 0`.
  - Revert proof: drop the `contains` early return and the bit stays set.
- `release_leaves_another_holder_s_flag`:
  - hold with `client_flag: true`;
  - simulate a second holder with `update_state_flag(PLAYER, BSF_MOVEMENT_LOCK, true, &tx, &mut mgr).await`;
  - release;
  - the bit is still set.
- `release_removes_the_hold_and_says_the_line`: hold, then `Release { message: Some("free") }`. There is no `MovementHold`, and the line contains `"free"`.
- `release_without_a_hold_still_says_the_line`: one release, and the line is sent.
- `a_held_player_is_locked_end_to_end`: after a hold at `[0.0; 3]`, `mgr.apply_client_position_update(PLAYER, [5.0, 0.0, 0.0], [0; 3], [0.0; 3])` matches `ClientMoveOutcome::Locked { .. }`. After a release, the same call does not.

**Lane checks, in order.**
1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-content`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-content executor::tests::movement_lock`
4. `pwsh tools/build-lane/lane.ps1 cargo check -p cimmeria-cell`

**Docs.** None (BL-07).

**Commit.** `BL-05: execute movement_lock hold and release`

---

### BL-07: docs

Host: windows · Branch `cellblock-boot-lock/BL-07-docs` · Worktree `cbl-07` · Agent: documentation-writer · Base: integration with BL-01..BL-06 merged

**Rules (inline).**
1. PowerShell only. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cbl-07;`.
2. Files under `docs/` are **CRLF**. Edit them with the Edit tool. A new file must be converted to CRLF with `[IO.File]` and an absolute path.
3. Never hand-edit a `<!-- gen:NAME -->` block. If a page you touch has one, run `python tools/docs-gen/regen.py` and commit its output.
4. Do not touch `docs/gap-analysis*` or `docs/project-status.md`. BL-09 owns them.
5. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
6. Read the merged code before you describe it. The code is the truth; this file is the plan.

**Do.** Walk `docs/agents/doc-update-map.md` and list the rows you touched in the report. At minimum:
1. `docs/content/content-engine-vocabulary.md`:
   - a `movement_lock` row in the actions table (lines 146-191), next to `spawn_set`, with its seed count;
   - a params section after the `spawn_set` section (about line 213) with C2's table, what hold and release do, the fail-open skips, the `client_flag` ref-count rule, and a link to the executor and the gate.
2. `docs/architecture/movement-validation.md`: the content hold gate. Cover:
   - where it sits (after the GM bypass, before layer 1);
   - the 1 u horizontal slack;
   - the 500 ms snap and 5 s feedback throttles;
   - that it spends no correction budget;
   - the `movement_hold_snaps_total` counter and the `movement.hold_snap` row;
   - that a stale hold is dropped on a space change.
3. `docs/architecture/state-field-bits.md`: add `movement_lock` (with `client_flag: true`) to the list of `BSF_MOVEMENT_LOCK` writers. Say that it is off in seeds until BL-U2.
4. `docs/gameplay/mission-system.md:168` and `docs/content/consumable-via-onitemuse-pattern.md:150`: correct each mention of chain 1024's `item_use 3438` to say it is now a right-click on the Prison Boot Control.
5. `docs/content/debug-hub.md`: add spawn 284 (Prison Boot Control) to the stasis-room table (about line 38), noting that it is not a hub service.
6. `docs/analysis/castle-cellblock-rebuild/uat-guide.md` T25 (lines 107-139): rewrite the steps for the right-click flow, and give the expected hold and release lines.

**Test.** None (docs). Run `python tools/docs-gen/regen.py --check` if the repo has it (see `docs/agents/pre-pr-checks.md`), and a link check of the pages you touched, as that page describes.

**Commit.** `BL-07: document the movement_lock verb and the content hold gate`

---

### BL-U1: lab UAT of the integration build (gate before the wave-1 PR)

Host: windows · Agent: orchestrator, with a `lab-driver` · **Ask the user before any lab use, and wait until the lab is free.** The user builds and starts the server from the `cbl-integration` worktree. Never build it for them.

Steps for the lab-driver brief (front-load the gotchas):
- Stand 4-6 m off targets.
- Clear dialogs first.
- Check every driver claim against the server log (`movement_lock.*`, `movement.hold_snap`, `minigame.*`).

| # | Step | Pass |
|---|---|---|
| 1 | Create a fresh Praxis character (any class but Goa'uld; step 8 covers Goa'uld) and enter Castle_CellBlock. | The prop is visible at the room centre with the nameplate "Prison Boot Control" and the quest glow. The hold line shows once. The log has `movement_lock.hold decision_outcome=held`. |
| 2 | Walk away for 3 s. | The player snaps back at most every 500 ms. The hold line repeats at most every 5 s. `movement_hold_snaps_total` rises. |
| 3 | Right-click the prop. | Livewire opens on the first press. |
| 4 | Lose or cancel the game, then right-click again. | Livewire opens again. The player is still held. |
| 5 | Log out and back in mid-lock. | The hold line shows again. The prop still glows and is clickable. |
| 6 | Win Livewire. | The release line shows. The player walks freely. The glow is gone. The log has `movement_lock.release decision_outcome=released` and `complete_mission 689`. |
| 7 | Relog. | No hold and no glow. |
| 8 | Create a Praxis Goa'uld character (char defs 10/19 start in the Cellblock) and repeat steps 1, 3 and 6. | Held exactly like step 1: the prop glows, the hold line shows, walking away snaps back. Livewire opens on a right-click, and a win releases. The log has `movement_lock.hold decision_outcome=held` and then `movement_lock.release decision_outcome=released`. |

A failure at step 1 or 3 blocks the PR. The kill switch is to disable chain 1023 in the seed; send a failure to the architect for a ruling. Record the results in `docs/analysis/cellblock-boot-lock/worknotes/BL-U1.md` (CRLF).

---

## Wave 2

### BL-08: enable Layer A (`client_flag: true`)

Host: windows · Branch `cellblock-boot-lock/BL-08-client-flag` · Worktree `cbl-08` · Agent: packet-coder · **BlockedDecision (OQ-3)** · Base: `origin/main` after wave 1 merged

**Rules (inline).** The same as BL-06's rules 1-9, with worktree `cbl-08`.

**Step 0: decide whether this packet is still small.** Grep for the path that moves a player entity between spaces without destroying it. Start from `crates/cell-world/src/cell/space_manager/entities.rs` and grep `entity_space.insert`.
- If an entity can change space while keeping its `extensions`, the gate's stale-hold drop (BL-02) leaves `BSF_MOVEMENT_LOCK` set.
- In that case, return `blocked` with the call sites and do not change the seed. The architect then adds a packet that releases the flag on a space change.

**Do.**
1. Chain 1023's hold params: change `"client_flag": false` to `"client_flag": true`.
2. Change every `client_flag: false` assertion in `mission_689_boot_lock.rs` to `true` (the stasis-anchor test, and the Goa'uld test if it checks the flag).

**Lane checks.**
1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-content`
2. `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::mission_689`

**Commit.** `BL-08: set the Prison Boot hold's client flag (Layer A)`

### BL-U2: lab UAT of Layer A

Ask the user first. Use a server built from `cbl-08`. Repeat BL-U1 steps 1-6, and also check:
- Does the client stop movement input while held, so the player does not rubber-band?
- Does a right-click on the prop still open Livewire while the flag is set?

If the answer to either question is no, BL-08 does not merge.

### BL-09: close-out

Host: windows · Agent: documentation-writer · Branch `cellblock-boot-lock/BL-09-close-out` · Worktree `cbl-09` · Base: `origin/main`, after the campaign review loop's PR

- `docs/gap-analysis/core-gameplay.md:283`: mark the prison boot lock implemented. Cite README D-BL3/D-BL4, and say what was lab-tested and what was not.
- `docs/project-status.md`: one line.
- `docs/guides/unified-uat.md`:
  - K7 (about line 89) and T25 (about line 711): replace the steps with BL-U1's list;
  - mark any step the owner has not run as owed.
- `docs/analysis/four-zone-planning/cellblock-work-packets.md`: note that this campaign supersedes the "689 boot gate relog" probe (OQ-9).
- `ops` retires every `cbl-*` worktree: `rm-worktree.ps1 --merged-into <wave-1 PR> cbl-01 ... cbl-07`, then `--merged` for the integration worktree and the wave-2 worktrees.
- Write an empty `.claude/state/active-campaign`.
- Post a board handoff through `ops`.

**Commit.** `BL-09: cellblock-boot-lock close-out`
