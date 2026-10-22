# Lab navigation telemetry: work packets

Ledger: [README.md](README.md) · Resume: [RESUME.md](RESUME.md) · RE: [controller-mouse-input-model.md](../../reverse-engineering/findings/controller-mouse-input-model.md)

Design complete (architect pass, 2026-10-22). Owner decisions D-LN1 (yes, variant A), D-LN2 (b) and D-LN3 (b, lift the quarantine) were recorded in a second pass the same day; their packets (LN-12b, LN-20a..e, wave 1b) are in [work-packets-wave1b.md](work-packets-wave1b.md). Every packet below is prescriptive: if the code does not match what a packet says (a function moved, a line is gone), stop and return `blocked` with what you found. Do not redesign.

## Dispatch rules

- **Waves.** Wave 1 (LN-01..05, LN-11) touches no launcher file and runs in parallel with launcher-uat-fixes. Wave 2 code (LN-06, LN-07, LN-19) after wave 1 has merged; the RE packets LN-12 and LN-15 touch only docs and memory and can start any time. Wave 1b (LN-12b, LN-20a..e; [work-packets-wave1b.md](work-packets-wave1b.md)) runs beside wave 1; LN-12b and LN-15 start after LN-11 has merged (LN-11 commits the files they need). Wave 3 (LN-08, LN-09, LN-10, LN-16) after wave 2; D-LN1 is decided (A), so LN-17 is out unless the owner later picks B. Wave 4 (LN-13, LN-18, LN-14) last.
- **Shipping.** Wave 1: one PR per packet; merge order LN-11, LN-04, LN-05, LN-01, LN-02, LN-03 (LN-11 first: LN-12b and LN-15 need its files; LN-02 and LN-03 edit different hunks of `interact.rs`; LN-03 rebases after LN-02). Wave 2: LN-06 and LN-07 ship together from an integration worktree on branch `lab-nav-telemetry/wave2` (LN-07's worktree is a branch cut at LN-06's reviewed head); LN-15 and LN-19 ship one PR each, in that order, then the wave-2 PR (LN-12 is done and commits nothing: its memory entry rides LN-11, its fix is LN-12b). Wave 1b: see [work-packets-wave1b.md § Dispatch and shipping](work-packets-wave1b.md#dispatch-and-shipping) (LN-20a..e as one integration PR from `lab-nav-telemetry/wave1b`, LN-12b its own PR). Wave 3: one integration PR from `lab-nav-telemetry/wave3` (cut from `main` after the wave-2 PR merges), packets merged in the order LN-08, LN-09, LN-10, LN-16 (LN-17 last if D-LN1 = B). Wave 4: LN-18 and LN-14 one PR each.
- **Agents.** Code and docs packets: `packet-coder` (Haiku). LN-12, LN-15: `game-archaeology-specialist`. LN-17: `rust-gameserver-dev`. LN-13: the orchestrator with the `lab-uat` skill, only after the user says yes. LN-19's triage step: an agent with the `telemetry-triage` skill.
- **Host.** Every packet is `Host: windows`.
- **Report and return.** Report at `<worktree>\.claude\reports\lab-nav-telemetry\<packet>-coder.md`; return exactly `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`. The report states what was tested and what was not.

## Rules (every packet; each packet repeats the ones it needs)

- R1. PowerShell only, never Bash. Every cargo call goes through the lane, from your worktree: `Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo <cmd> -p <crate> ...`, in the order fmt, clippy, tests. Never call cargo directly. Run lane calls in the foreground. Read the lane summary and its failures file; don't rerun to see output.
- R2. One worktree and one `sgw_<name>` DB per packet. No `git stash`, no `git worktree prune`, no push. Commit locally once the checks are green, with the attribution lines from your brief.
- R3. Files: soft cap 500 lines, hard cap 700. Put new code in the new files named; don't grow a file past 700.
- R4. `docs/**/*.md` are CRLF. After editing one, convert with the absolute path: `$p='<abs path>'; $t=[IO.File]::ReadAllText($p) -replace "(?<!\r)\n","`r`n"; [IO.File]::WriteAllText($p,$t)`.
- R5. Privacy: never log key codes, characters, chat text or cursor coordinates. Mouse data is per transition or per 250 ms change, never per frame.
- R6. A runtime change needs a test that fails when the change is reverted ([TESTING.md](../../../TESTING.md)). Log rows are pinned with `crate::test_support::LogCapture` (`LogCapture::install()` before the action; default `#[tokio::test]`, never multi-thread; `capture.all()`, `c.has_field(k, v)`, `capture.find_event(Level, msg_substr, reason)`).
- R7. A hook or poll that cannot work logs one negative row and leaves the game running ([negative-logging-convention.md](../../architecture/negative-logging-convention.md)).
- R8. Edit only the files your packet names. Walk [doc-update-map.md](../../agents/doc-update-map.md) only for the rows your packet lists.

### Lane command sets

- **Server crate `<c>`:** `cargo fmt -p <c> --check` (on failure run `cargo fmt -p <c>` through the lane, then recheck), `cargo clippy -p <c> --all-targets -- -D warnings`, `cargo nextest run -p <c>`.
- **Client DLL (`cimmeria-client-telemetry`)**, all six, in order. A host-only run is a false green: the hooks compile only for i686.
  1. `cargo fmt -p cimmeria-client-telemetry --check`
  2. `cargo clippy -p cimmeria-client-telemetry --target i686-pc-windows-msvc --all-targets -- -D warnings`
  3. `cargo clippy -p cimmeria-client-telemetry --features lab-bridge --target i686-pc-windows-msvc --all-targets -- -D warnings`
  4. `cargo nextest run -p cimmeria-client-telemetry --target i686-pc-windows-msvc`
  5. `cargo nextest run -p cimmeria-client-telemetry --features lab-bridge --target i686-pc-windows-msvc`
  6. Both nextest runs must report a higher test count than before your change.
- **Live-DB:** `Set-Location <worktree>; pwsh tools/build-lane/live-db-test.ps1 <test-name substring>`.

## Contracts

### C-LN1: server rows (wave 1)

| Row | Level, target | Fields added or new | Packet |
|---|---|---|---|
| `cooked_data.miss_refused` (existing) | WARN, module path | `element_name` (Option, the cooked element's name when known), `display_age_ms` (Option, ms since the base sent `onDialogDisplay` for this key to this client; category 5 only) | LN-01 |
| `dialog.cooked_miss_after_display` (new) | WARN, module path | `addr`, `account_id`, `account_name`, `category_id`, `key`, `element_name`, `display_age_ms`; only when `display_age_ms < 1000` | LN-01 |
| `interact` (existing INFO, message `"interact"`) | INFO, module path | `heading_age_ms` (i64; ms since the cell processed the player's last position update; `-1` when none) | LN-02 |
| `interaction_pose` (existing) | INFO, `interaction.pose` | new route value `hostile_attack` for an RMB on a living hostile NPC | LN-03 |
| `interaction_hostiles` (new) | INFO, `interaction.pose` | `entity_id`, `pose_seq` (joins the pose row), `count` (1-3), `hostile_<n>_id`, `hostile_<n>_dist_m`, `hostile_<n>_bearing_deg` for n = 1..count; nearest first, radius 20 m; no row when none | LN-03 |
| `org_vault_open_rejected` (existing) | `not_in_org`: INFO; every other reason: WARN; target `bank` | unchanged | LN-05 |

### C-LN2: client rows (`cimmeria-client` service, waves 2-3)

All rows go through `crate::hooks::emit::emit(target, "info", fields)`. Angles in degrees, positions in server units, converted exactly as `hooks/camera/emit.rs::camera_fields` converts them.

| Target | When | Fields | Packet |
|---|---|---|---|
| `client.nav.pose` | checked every 250 ms; emitted only when the snapshot changed (thresholds in LN-07) | `zoom`, `yaw_offset_deg`, `pitch_offset_deg`, `pawn_ok` (bool), `pawn_x`, `pawn_y`, `pawn_z`, `pawn_yaw_deg` (when `pawn_ok`), `fwd`, `strafe`, `turn` (when PlayerInput was readable), `mode`, `buttons`, `hover_id` | LN-07 |
| `client.input.button` | each LMB/RMB edge | `button` (`lmb`/`rmb`), `edge` (`down`/`up`); on down `other_held` (bool); on up `hold_ms` (when the down was seen), `drag_x`, `drag_y`, `class` (`select`/`interact`/`drag`/`chord`), `entity_id` (hover id, only for `select`/`interact` and when non-zero); `ui_owned` (bool, only when known) | LN-08, LN-10 |
| `client.input.mode` | `mode` change, at most 8 rows per second | `mode`, `prev`, `mouselook`, `jump`, `autorun`, `mousewalk` (bools of `mode`), `suppressed` (changes not emitted since the last row) | LN-09 |
| `client.input.hover` | `hover_id` change, at most 4 rows per second | `entity_id` (0 = none), `prev`, `suppressed` | LN-09 |
| `client.input.mouse_window` | per 100 ms window with any mouse record (D-LN1 = B only) | `dx`, `dy`, `wheel`, `records`, `buttons`, `mode` | LN-17 |

Governor: `client.nav.` and `client.input.` prefixes are must-keep with `KeepReason::NavInput` (D-LN6), added once, by LN-07.

### C-LN3: the client `nav` module (`crates/client-telemetry/src/hooks/nav/`)

```rust
// controller.rs (LN-06). Offsets from the RE finding, section 2.
pub const PC_INPUT_PTR_OFF: u32 = 0x3f4;   // PlayerInput*
pub const PC_MODE_OFF: u32 = 0x498;        // mode bits
pub const PC_BUTTONS_OFF: u32 = 0x49c;     // button bits
pub const PC_DRAG_X_OFF: u32 = 0x4a0;      // i32 mouse counts, reset on press
pub const PC_DRAG_Y_OFF: u32 = 0x4a4;
pub const PC_HOVER_OFF: u32 = 0x4c0;       // hover entity id
pub const INPUT_TURN_OFF: u32 = 0xa4;      // f32
pub const INPUT_FWD_OFF: u32 = 0xa8;       // f32
pub const INPUT_STRAFE_OFF: u32 = 0xc0;    // f32
pub const MODE_MOUSELOOK: u32 = 0x1;
pub const MODE_JUMP: u32 = 0x2;
pub const MODE_AUTORUN: u32 = 0x4;
pub const MODE_MOUSEWALK: u32 = 0x8;
pub const MODE_MASK: u32 = 0xf;
pub const BUTTON_LMB: u32 = 0x1;
pub const BUTTON_RMB: u32 = 0x2;
pub const BUTTON_MASK: u32 = 0x3;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct InputAxes { pub turn: f32, pub fwd: f32, pub strafe: f32 }

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ControllerState {
    pub mode: u32,               // [pc + PC_MODE_OFF] & MODE_MASK
    pub buttons: u32,            // [pc + PC_BUTTONS_OFF] & BUTTON_MASK
    pub drag: [i32; 2],          // [pc + 0x4a0], [pc + 0x4a4] as i32
    pub hover_id: i32,           // [pc + PC_HOVER_OFF] as i32
    pub axes: Option<InputAxes>, // None when PlayerInput* is 0 or unreadable
}

/// None when any of mode, buttons, drag or hover cannot be read.
pub fn read_controller(mem: &impl Mem, pc: u32) -> Option<ControllerState>;

// hooks/camera/read.rs (LN-06)
pub fn find_player_controller(mem: &impl Mem) -> Option<u32>;

// frame.rs (LN-07)
pub const PC_RESCAN_MS: u64 = 1000;
pub struct NavFrame { pub now_ms: u64, pub ctrl: ControllerState, pub ui_owned: Option<bool> }
pub struct PcCache { pub ptr: u32, pub next_scan_ms: u64 }
pub fn controller_ptr(mem: &impl Mem, now_ms: u64, cache: &mut PcCache) -> Option<u32>;

// pose.rs (LN-07), buttons.rs (LN-08), transitions.rs (LN-09), ui.rs (LN-10).
// LN-07 creates these three stubs with exactly these signatures; wave 3 fills them:
pub(super) fn step(frame: &NavFrame) {}                              // buttons.rs, transitions.rs
pub(super) fn read_ui_owned(mem: &impl Mem) -> Option<bool> { None } // ui.rs
```

`Mem` is `crate::hooks::entity_trace::map::Mem` (`u32_at`, `bytes_at`); the test fake is `crate::hooks::entity_trace::map::fake::FakeMem` (`FakeMem::default()`, `mem.set(addr, value)`).

## Wave 1: server and docs (parallel, no launcher files)

### LN-01 Dialog cooked-miss: element name and display age

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-01-dialog-miss` · Worktree `ln-01` · Commit `LN-01: name the dialog and its display age on a refused cooked-data miss`

Why: F1. A refused miss for a dialog key looks like noise unless the row says which dialog it is and that the client was just shown it.

Rules: R1 (server sets for `cimmeria-base-session` and `cimmeria-base-world-entry`), R2, R3 (`aoi_dispatch.rs` is 642 lines: add at most 8 lines there), R4, R6, R8.

1. Check first: `docs/protocol/client-method-dispatch-table.md` must give `onDialogDisplay` index 105 on SGWPlayer, and `send_dialog_display` in `crates/cell-content/src/cell/interactions/dialog.rs` (around :35) must show where the dialog id sits in the args. If the index is not 105, return `blocked`. Note the byte offset of the dialog id (expected `args[4..8]`, little-endian).
2. New file `crates/base-session/src/base/cooked_sync/recent_display.rs` (under 120 lines including tests):
   - `pub const ON_DIALOG_DISPLAY_INDEX: u16 = 105;` with a comment citing the dispatch table.
   - `pub const DIALOGS_CATEGORY: u32 = 5;` unless `super::resources` already has a Dialogs category constant; then reuse that.
   - `static LAST: LazyLock<Mutex<HashMap<SocketAddr, (u32, Instant)>>>`, holding the last dialog id sent to each client. If the map holds more than 4096 entries on insert, clear it first.
   - `pub fn dialog_id_from_args(args: &[u8]) -> Option<u32>` reads the dialog id at the offset from step 1.
   - `pub fn note_dialog_display(addr: SocketAddr, dialog_id: u32, now: Instant)`.
   - `pub fn display_age_ms(addr: SocketAddr, dialog_id: u32, now: Instant) -> Option<u64>`: Some only when the last display to `addr` was `dialog_id`.
   - A poisoned mutex is recovered with `unwrap_or_else(|e| e.into_inner())`.
   - Inline `#[cfg(test)] mod tests`: same id gives Some, another id None, unknown addr None, short args None.
3. `crates/base-session/src/base/cooked_sync/mod.rs`: add `mod recent_display;` and `pub use recent_display::{dialog_id_from_args, note_dialog_display, ON_DIALOG_DISPLAY_INDEX};`. In `serve_miss`, the `MissOutcome::Refused { why, log }` arm (around :118):
   - compute `let element_name = super::cooked_data::element_name(category_id, key);` and `let display_age_ms = if category_id == DIALOGS_CATEGORY { recent_display::display_age_ms(addr, key, now) } else { None };`
   - add `element_name = element_name.as_deref(), display_age_ms,` to the existing `cooked_data.miss_refused` WARN (Option fields record only when Some);
   - after the `if let Some(suppressed) = log { ... }` block (not inside it, so the throttle does not hide it): when `display_age_ms` is `Some(age)` with `age < 1000`, `tracing::warn!` with `%addr`, `account_id`, `account_name` (same `identity_for_addr` call), `event = "dialog.cooked_miss_after_display"`, `category_id`, `key`, `element_name = element_name.as_deref()`, `display_age_ms = age`, message `"A dialog was displayed but its cooked data was refused: the client shows nothing"`.
4. `crates/base-world-entry/src/base/world_entry/cell_dispatch/aoi_dispatch.rs`, `entity_method_call` (around :419): right after `target_addr` is computed and before the `should_defer` check, add:
   ```rust
   if method_index == cooked_sync::ON_DIALOG_DISPLAY_INDEX {
       if let (Some(addr), Some(dialog_id)) = (target_addr, cooked_sync::dialog_id_from_args(&args)) {
           cooked_sync::note_dialog_display(addr, dialog_id, std::time::Instant::now());
       }
   }
   ```
   Reach `cooked_sync` by the path other base-world-entry code uses for `serve_miss` (grep `serve_miss(` under `crates/base-world-entry`).
5. Tests in `crates/base-session/src/base/cooked_sync/tests/misses.rs` (266 lines; follow `an_unknown_key_is_refused_and_repeats_are_throttled` at :105 for the rig):
   - `a_refused_dialog_miss_right_after_its_display_warns_with_the_age`: `note_dialog_display(<rig addr>, KEY, Instant::now())`, then a miss for `(DIALOGS, KEY)` with a key the rig's cache lacks (never 60100, 60101, 60104 or 60110: wave 1b serves them). Assert the `cooked_data.miss_refused` row has a `display_age_ms` field and exactly one `dialog.cooked_miss_after_display` WARN.
   - `a_refused_dialog_miss_without_a_display_has_no_correlation_row`: no `dialog.cooked_miss_after_display` row.
   - If you can find a dialog id that `cimmeria_names::book().dialog(id)` knows, use it as the refused key and also assert `element_name`; otherwise say so in the report.
   Revert proof: without step 3 both the field and the row vanish.
6. Docs: `docs/architecture/observability-target-catalog.md`: in the row for `cooked_data.miss_refused` (grep it) add the two fields; add a row for `dialog.cooked_miss_after_display` in the same table, same format. CRLF (R4).

Lanes: server set for `cimmeria-base-session`, then `cimmeria-base-world-entry`.

### LN-02 Interact: heading age

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-02-heading-age` · Worktree `ln-02` · Commit `LN-02: log heading age on interact`

Why: F4. The server's heading at an interact can be 3 s old; the row must say how old.

Rules: R1 (server sets for `cimmeria-entity`, `cimmeria-cell-methods`), R2, R3, R4, R6, R8. Edit only the `"interact"` INFO near :49 of `interact.rs`: LN-03 edits other hunks of the same file.

1. `crates/entity/src/movement_validation/mod.rs`: next to `touch_clock` (:269) add
   ```rust
   /// When the last client position update for `entity_id` was processed, if any.
   pub fn last_move_at(&self, entity_id: u32) -> Option<Instant> {
       self.move_clock.get(&entity_id).copied()
   }
   ```
   plus a unit test in that crate's existing movement-validation tests: `touch_clock` then `last_move_at` returns it; unknown id is None.
2. `crates/cell-methods/src/cell/cell_methods/player/interaction/interact.rs`:
   - add at the bottom
     ```rust
     /// Milliseconds since the last processed position update, or -1 when none was seen.
     pub(super) fn heading_age_ms(now: Instant, last: Option<Instant>) -> i64 {
         last.map_or(-1, |t| i64::try_from(now.saturating_duration_since(t).as_millis()).unwrap_or(i64::MAX))
     }
     ```
   - add `heading_age_ms = heading_age_ms(Instant::now(), space_mgr.movement_validator.last_move_at(entity_id)),` to the `tracing::info!(... "interact")` at :49-55 (`movement_validator` is a pub field of `SpaceManager`).
3. Test file `crates/cell-methods/src/cell/cell_methods/player/interaction/interact_heading_tests.rs`, registered in that folder's `mod.rs` the same way `interact_pose_tests` is. Copy the space-manager setup from `interact_pose_tests.rs`.
   - pure: `heading_age_ms(now, None) == -1`; 1,500 ms ago gives 1,500 (±50).
   - LogCapture: `mgr.movement_validator.touch_clock(PLAYER, Instant::now() - Duration::from_millis(1500))`, call `handle_interact`, assert the `"interact"` INFO row has `heading_age_ms` parsing to 1,500..5,000.
   Revert proof: without the field the LogCapture test fails.
4. Docs: `docs/operations/signoz/interaction-pose-views.md`: add a short section `## Heading age at interact` immediately before `## Friction is DEBUG now (D-PC11)`: the field, its meaning, `-1`, and that standing players send a position about every 3 s, so an age above 1,000 means the server heading may lag the client's turn. CRLF.

Lanes: server set for `cimmeria-entity`, then `cimmeria-cell-methods`.

### LN-03 Interact: hostile-click pose and nearby hostiles

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-03-hostile-pose` · Worktree `ln-03` · Commit `LN-03: pose row for hostile clicks and nearby hostiles at interact`

Why: F5. An RMB on a hostile leaves no pose row, and no row names the hostiles around an interaction.

Rules: R1 (server sets for `cimmeria-cell-world`, `cimmeria-cell-methods`), R2, R3, R4, R6, R8. Don't touch the `"interact"` INFO at :49 (LN-02's hunk). Merge after LN-02; rebase on conflict.

1. `crates/cell-world/src/cell/interaction_pose/geometry.rs`: add `InteractionRoute::HostileAttack`, whose `as_str` is `"hostile_attack"` (match the existing naming style).
2. New `crates/cell-world/src/cell/interaction_pose/hostiles.rs` (under 150 lines):
   - `pub const HOSTILE_CONTEXT_RADIUS_M: f32 = 20.0;` and `pub const HOSTILE_CONTEXT_MAX: usize = 3;`
   - `pub struct NearbyHostile { pub entity_id: u32, pub dist_m: f32, pub bearing_deg: f32 }`
   - `pub fn nearby_hostiles(mgr: &SpaceManager, pose: &InteractionPose) -> Vec<NearbyHostile>`: `mgr.npc_ids_near(pose.entity_id, HOSTILE_CONTEXT_RADIUS_M)`, keep entities where `crate::cell::combat::is_hostile_to_players(npc)` is true and the NPC is not dead (the same dead-state check `interact.rs` :80 uses), skip `pose.target_entity_id`, compute geometry with `pose_geometry(pose.player_pos, <player yaw in radians, as capture.rs computes it>, <npc position, read as capture.rs reads the target's>)`, `dist_m = geometry.radius_m`, `bearing_deg = geometry.bearing_deg`; sort by `dist_m`, truncate to `HOSTILE_CONTEXT_MAX`.
   - `pub fn emit_nearby_hostiles(mgr: &SpaceManager, pose: &InteractionPose, pose_seq: <the type emit_interaction_pose returns>)`: no row when the list is empty; else one `tracing::info!(target: "interaction.pose", event = "interaction_hostiles", entity_id, pose_seq, count, hostile_1_id, hostile_1_dist_m, hostile_1_bearing_deg, hostile_2_id = <Option>, ..., "interaction_hostiles")`.
   - `mod.rs`: `mod hostiles;`, `#[cfg(test)] mod hostiles_tests;`, `pub use hostiles::{emit_nearby_hostiles, nearby_hostiles, NearbyHostile, HOSTILE_CONTEXT_MAX, HOSTILE_CONTEXT_RADIUS_M};`
3. `crates/cell-methods/src/cell/cell_methods/player/interaction/interact.rs`:
   - in `if is_hostile { ... }` (:82), before `right_click_attack`: `if let Some(pose) = interaction_pose::capture_interaction_pose(space_mgr, entity_id, target_entity_u32) { if let Some(seq) = interaction_pose::emit_interaction_pose(&pose, InteractionKind::Interact, InteractionRoute::HostileAttack, PoseExtra { dialog_id: None, item_type_id: None }) { interaction_pose::emit_nearby_hostiles(space_mgr, &pose, seq); } }`
   - at :256, replace `let _ = interaction_pose::emit_interaction_pose(...)` with `if let Some(seq) = ... { interaction_pose::emit_nearby_hostiles(space_mgr, &pose, seq); }`, arguments unchanged.
4. Tests:
   - `crates/cell-world/src/cell/interaction_pose/hostiles_tests.rs` (build the manager as `approach_emit_tests.rs` or `emit_tests.rs` do): four hostiles at 3, 6, 9 and 12 m plus one non-hostile and one dead hostile; `nearby_hostiles` returns the three nearest in order and excludes the other two; LogCapture: one `interaction_hostiles` row with `count = 3` and `hostile_1_id`; an empty area gives no row.
   - append to `interact_pose_tests.rs`: an RMB on a living hostile (faction 10) emits `interaction_pose` with route `hostile_attack`.
   Revert proof: without step 3 neither row appears.
5. Docs: `docs/operations/signoz/interaction-pose-views.md`: add `## Hostile clicks and nearby hostiles` immediately before `## Approach trails`: the route value, the new row's fields, a ClickHouse query joining it to the pose row on `pose_seq` in the style of the file's other queries. CRLF.

Lanes: server set for `cimmeria-cell-world`, then `cimmeria-cell-methods`.

### LN-04 Launcher replay targets in the catalog (docs only)

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-04-launcher-catalog` · Worktree `ln-04` · Commit `LN-04: catalog the launcher replay targets`

Why: F6, D-LN7. The filter has no gap, but the catalog has no rows for five launcher targets, and nothing says the launcher's own rows stay local.

Rules: R2, R4, R8. Docs only: no lane run. Don't edit anything under `crates/`.

1. Read, don't edit: `crates/server/src/logging/filters.rs` L415-416 and the `OTLP_EXCLUDED_TARGETS` reason for `launcher.key_dump` (L431-446); `crates/server/src/otel.rs` `CLIENT_TARGETS` (L174-180); the four replay sites in `crates/admin-api/src/routes/telemetry/replay.rs` (L197, 214, 234, 246) and `launcher.bundle` in `bundle.rs` for each target's level and fields.
2. `docs/architecture/observability-target-catalog.md`: next to the `launcher.summary` and `launcher.ingest` rows (L185-186), add rows in the same format for `launcher.client_log`, `launcher.debug_log`, `launcher.session_meta` (client index), `launcher.bundle` (server index) and `launcher.key_dump` (never exported; the reason from `OTLP_EXCLUDED_TARGETS`).
3. Same file, in the launcher details section (around L572-589), add a paragraph "Launcher-local rows": the launcher's `launcher.update` and `cimmeria_launcher_*` rows reach only its local daily log (`crates/launcher/desktop/shell/src/telemetry.rs` L84-91); the only forwarded target is `adoption`, as `client.launcher.adoption`. Exporting `launcher.update` is LN-18.
4. CRLF (R4). Check every link you add resolves.

### LN-05 Org-vault `not_in_org` at INFO

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-05-org-vault-info` · Worktree `ln-05` · Commit `LN-05: log the org-vault not_in_org refusal at INFO`

Why: F12, D-LN10. A player clicking a Banker of an org they are not in is a normal refusal with a chat line, not a fault.

Rules: R1 (server set for `cimmeria-base-methods`; live-DB), R2, R3, R4, R6, R8. Use the live-DB script for the live tests.

1. `crates/base-methods/src/base/world_entry/methods/inventory/org_vault/open.rs`, `async fn refuse` (:310): replace the one `tracing::warn!` (:319-338) with a local `macro_rules! rejected { ($lvl:ident) => { tracing::$lvl!( <the same target, fields and message, unchanged> ) }; }` and `if matches!(refusal, OpenRefusal::NotInOrg) { rejected!(info) } else { rejected!(warn) }`. Fields and message stay byte-identical.
2. Tests: in `org_vault/tests/open.rs`, the helper near :176 asserts exactly one WARN per refusal. Give it a `Level` parameter, pass `Level::INFO` from `live_db_a_player_in_no_team_is_refused_not_in_org` (:201) and `Level::WARN` from every other caller, and assert in the `not_in_org` test that no WARN `org_vault_open_rejected` row exists. Then grep `crates/` for other tests asserting a WARN `org_vault_open_rejected` with `not_in_org` (for example the Debug Hub round trip `debug_hub_org_bankers_open_the_members_vault_and_refuse_others`) and switch only those assertions to INFO. Revert proof: going back to `warn!` fails the `not_in_org` test.
3. Docs (CRLF): `docs/gameplay/inventory-system.md` :279, `docs/architecture/negative-logging-convention.md` :534 and the `org_vault_open_rejected` row in `docs/architecture/observability-target-catalog.md`: say WARN, except `not_in_org` at INFO. Leave `docs/guides/unified-uat.md` alone (another campaign is editing it); its queries filter by `reason`, not level.

Lanes: server set for `cimmeria-base-methods`, then `pwsh tools/build-lane/live-db-test.ps1 org_vault`, then `live-db-test.ps1` with the Debug Hub test's name if you changed it.

### LN-11 Lab docs: camera chain and interact distance

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-11-lab-docs` · Worktree `ln-11` · Commit `LN-11: reconcile lab camera memory and the interact distance gotcha`

Why: F10, F11. Two memory entries seem to contradict each other on the camera, and three sheets give three stand-off distances.

Rules: R2, R4, R5, R8. No code, no lane run. Skills are edited only in `.claude/skills/`, then synced. Merge LN-11 first in wave 1: LN-12b and LN-15 wait for it.

1. `.claude/agent-memory/main-session/reference_lab_harness_findings_2026_10_12.md`:
   - line 11: say the camera *actor location* reads zero on this build; the camera *state* (yaw and pitch offsets, zoom, gains) is valid through the Actors scan (controller vtable `0x019e2b2c`, camera at `+0x2b0`); the old WorldInfo `+0x35c` chain is 0 in a normal game (`crates/client-telemetry/src/hooks/camera/read.rs`, `find_player_camera`).
   - line 12: append that static RE (2026-10-22, `docs/reverse-engineering/findings/controller-mouse-input-model.md`) says mouse movement with RMB *held* also turns the pawn (controller `+0x498` bit 0); the lab's `client_camera` mouse-look moves without a held button, so it only orbits; unconfirmed live (LN-13).
   - line 16: "Stand 3.5 to 4.5 m off" becomes "Stand 3 to 4 m off".
   - the `description:` front-matter: "zero camera pose" becomes "zero camera actor location".
2. `.claude/agent-memory/main-session/reference_lab_driving_recipes_2026_10_12.md` line 11: "Stand 3.5-4.5 m" becomes "Stand 3-4 m"; keep the 6.4 m and 3.4 m observations.
3. `.claude/agent-memory/main-session/MEMORY.md`: if the index line for the harness-findings file says "zero camera pose", make it "zero camera actor location".
4. `.claude/skills/lab-uat/SKILL.md` lines 59-61: keep "Stand 3-4 m away (server range 5.0 m)" and add "in 3D; humans interact at about 1.3-3.8 m. Click by entity_id: under about 3.4 m your own avatar can take a click by point".
5. Run `python tools/agent-skills/sync.py` and commit the synced `.agents/skills/lab-uat/SKILL.md` with it.
6. Commit the campaign's untracked files from the main checkout (they exist only there; LN-12b and LN-15 need them in git). Copy each with `Copy-Item` from the absolute path under `C:\Users\Steve\source\projects\Cimmeria\` to the same relative path in your worktree:
   - `docs\reverse-engineering\findings\controller-mouse-input-model.md`;
   - `.claude\agent-memory\game-archaeology-specialist\controller-mouse-input-model.md` and `.claude\agent-memory\game-archaeology-specialist\bm-overlay-clock-reopen-ln12.md`;
   - the campaign folder snapshot: `docs\analysis\lab-nav-telemetry\README.md`, `RESUME.md`, `work-packets.md`, `work-packets-wave1b.md`, and `worknotes\*` if it exists.
   - Index lines: never copy a `MEMORY.md`. In your worktree's `.claude\agent-memory\game-archaeology-specialist\MEMORY.md`, append the main checkout's index line for each of the two memory files (find them with `Select-String` on the main copy). If a line is missing there, write one: `- [<title>](<file>.md) — <one-line hook>`.
   - Run R4 on each copied `docs\**\*.md`. No IPs, credentials or account names in anything you commit (check with a quick read).

Not in this packet: `docs/guides/unified-uat.md` :1370 ("4 to 6 m") is fixed by LN-14, because another campaign has uncommitted edits in that file.

## Wave 2: client poll model and RE

### LN-06 Client: controller reads

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-06-controller-reads` · Worktree `ln-06` · Commit `LN-06: read the player controller's input state`

Why: D-LN5. Every input row is a poll of these fields; this packet adds only the reads, with no emit and no behaviour change.

Rules: R1 (the client DLL set), R2, R3, R6, R8. The contract is C-LN3; use its names exactly.

1. `crates/client-telemetry/src/hooks/camera/read.rs` (179 lines): split `find_player_camera` (:109) into `pub fn find_player_controller(mem: &impl Mem) -> Option<u32>` (the same scan; returns the first actor whose vtable is `PLAYER_CONTROLLER_VTABLE_VA`) and `find_player_camera`, which becomes `let pc = find_player_controller(mem)?;` then the existing camera read and vtable check. Behaviour of `find_player_camera` is unchanged: the existing `hooks/camera/tests.rs` must pass untouched.
2. New `crates/client-telemetry/src/hooks/nav/mod.rs`: `pub mod controller;` and `#[cfg(test)] mod controller_tests;`. Register `pub mod nav;` where `hooks/camera` is declared (grep `mod camera;` under `crates/client-telemetry/src/`).
3. New `hooks/nav/controller.rs`: the constants, `InputAxes`, `ControllerState` and `read_controller` from C-LN3. `read_controller`: read the four words with `mem.u32_at`; any `None` returns `None`; mask mode and buttons; drag words `as i32`; hover `as i32`; `axes`: read the PlayerInput pointer at `pc + PC_INPUT_PTR_OFF`; when non-zero, read the three words and convert with `f32::from_bits`; any failure gives `axes: None`.
4. New `hooks/nav/controller_tests.rs`, with a small local fixture (copy the shape of `build_world` and `controller` from `hooks/camera/tests.rs`; don't change that file):
   - `find_player_controller` finds the controller among other actors, and returns None with no controller;
   - `read_controller` masks `mode` (0xfff3 gives 0x3) and `buttons`, reads a negative drag (0xffff_fff6 gives -10), reads hover, reads axes from f32 bits, gives `axes: None` for a null PlayerInput, and returns None when the mode word is unreadable.
5. No docs in this packet (LN-07 documents the module).

Lanes: the client DLL set (six steps).

### LN-07 Client: nav tick and 4 Hz pose sampler

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-07-pose-sampler`, cut at LN-06's reviewed head · Worktree `ln-07` · Commit `LN-07: sample camera and pawn pose from the engine tick`

Why: F2, F3, D-LN4, D-LN6. A pose row whenever the camera, pawn or input changed, at most 4 per second, and the frame plumbing wave 3 fills.

Rules: R1 (the client DLL set), R2, R3 (`inline_hooks/mod.rs` is 602 lines: change at most one visibility keyword there), R4, R5, R6, R7, R8. The contract is C-LN2 and C-LN3.

1. `hooks/camera/emit.rs`: factor the conversion `camera_fields` uses for `zoom`, `yaw_offset_deg` and `pitch_offset_deg` into `pub(crate) fn camera_angles(cam: &CameraState) -> (f32, f32, f32)`, and the pawn conversion into `pub(crate) fn pawn_fields(pawn: &ActorPose) -> ([f32; 3], f32)` (server position, yaw in degrees); `camera_fields` calls both. `emit_tests.rs` must pass untouched.
2. New `hooks/nav/frame.rs`: `NavFrame`, `PcCache`, `PC_RESCAN_MS` and `controller_ptr` from C-LN3. `controller_ptr`: if `cache.ptr` still has the controller vtable, return it; else if `now_ms < cache.next_scan_ms`, return None; else scan with `find_player_controller`, store the result (0 on None), and on None set `next_scan_ms = now_ms + PC_RESCAN_MS`. Also `pub fn now_ms() -> u64`, milliseconds since a `static START: OnceLock<Instant>`.
3. New `hooks/nav/pose.rs` (pure, every target):
   ```rust
   pub const POSE_PERIOD_MS: u64 = 250;
   pub const ANGLE_EPS_DEG: f32 = 1.0;
   pub const ZOOM_EPS: f32 = 1.0;
   pub const POS_EPS: f32 = 0.10;   // server units
   pub const AXIS_EPS: f32 = 0.01;
   #[derive(Clone, Copy, Debug, PartialEq)]
   pub struct PoseSnapshot { pub zoom: f32, pub yaw_offset_deg: f32, pub pitch_offset_deg: f32,
       pub pawn: Option<([f32; 3], f32)>, pub axes: Option<InputAxes>, pub mode: u32, pub buttons: u32, pub hover_id: i32 }
   #[derive(Default)]
   pub struct PoseSampler { last_check_ms: Option<u64>, last_emitted: Option<PoseSnapshot> }
   impl PoseSampler {
       pub fn due(&self, now_ms: u64) -> bool;                        // None, or now - last >= POSE_PERIOD_MS
       pub fn offer(&mut self, now_ms: u64, snap: PoseSnapshot) -> bool; // sets last_check_ms; true (and stores snap) when first or changed
   }
   pub fn changed(a: &PoseSnapshot, b: &PoseSnapshot) -> bool;
   pub fn angle_delta_deg(a: f32, b: f32) -> f32;                     // shortest signed difference, in -180..180
   pub fn pose_fields(snap: &PoseSnapshot) -> Fields;                 // C-LN2 field names; pawn_ok = pawn.is_some()
   ```
   `changed` is true when an angle moved by `ANGLE_EPS_DEG` or more (by `angle_delta_deg`; the pawn yaw too), zoom by `ZOOM_EPS` or more, the pawn position by `POS_EPS` or more (Euclidean), the pawn went Some/None, an axis by `AXIS_EPS` or more or axes went Some/None, or mode, buttons or hover differ.
4. New stubs with the exact C-LN3 signatures and a one-line doc saying which packet fills them: `hooks/nav/buttons.rs` (LN-08), `hooks/nav/transitions.rs` (LN-09), `hooks/nav/ui.rs` (LN-10).
5. New `hooks/nav/tick.rs`, `#[cfg(all(target_os = "windows", target_arch = "x86"))]`: `pub(crate) fn on_tick()`:
   - On the first call, check whether `nav_poll` is disabled the way `install_one` checks `CIMMERIA_CLIENT_HOOKS_DISABLE` for a hook name (find that helper in `inline_hooks/mod.rs`; make it `pub(crate)` if it is private) and cache the answer in a `OnceLock<bool>`. Disabled: emit `client.hooks.inline.disabled` with `hook = "nav_poll"` once, the same way `install_one` does, and return on every call.
   - `let mem = LiveMem;` (as `camera/emit.rs` builds it), `let now = frame::now_ms();`, `controller_ptr` with a `static PC: Mutex<PcCache>`; None returns.
   - `read_controller`; None returns. Build `NavFrame { now_ms: now, ctrl, ui_owned: ui::read_ui_owned(&mem) }`.
   - Pose: with a `static SAMPLER: Mutex<PoseSampler>`, when `due(now)`: read the camera with `camera::read::camera_ptr` and `read_camera`, and the pawn with `read_pawn_pose`; with no camera, call `offer` with nothing (just advance `last_check_ms`: add `pub fn skip(&mut self, now_ms)` for that). Else build the snapshot through `camera_angles` and `pawn_fields`, and when `offer` returns true, `crate::hooks::emit::emit("client.nav.pose", "info", pose_fields(&snap))`.
   - Then `buttons::step(&frame); transitions::step(&frame);`.
   - Every `Mutex::lock` recovers poison with `unwrap_or_else(|e| e.into_inner())`.
6. `hooks/nav/mod.rs`: add `pub mod frame; pub mod pose; mod buttons; mod transitions; mod ui;`, `#[cfg(all(target_os = "windows", target_arch = "x86"))] pub(crate) mod tick;` and `#[cfg(test)] mod pose_tests;`.
7. `hooks/inline_hooks/engine_frame.rs`: in `engine_tick_detour`, inside the existing `catch_unwind` block and right after `crate::hooks::seams::frame_health::on_tick();` (:106), call `crate::hooks::nav::tick::on_tick();`.
8. Governor: `crates/client-telemetry/src/governor/classify.rs`: add `KeepReason::NavInput` (fix any match the compiler flags, giving it the label `"nav_input"` where reasons have labels) and two rows, `keep(Prefix("client.nav."), KeepReason::NavInput)` and `keep(Prefix("client.input."), KeepReason::NavInput)`, before any broader `client.` prefix row. Add both prefixes to the table in `governor/tests/classify.rs` (~:70).
9. Tests, new `hooks/nav/pose_tests.rs`: first offer emits; the same snapshot again doesn't; yaw 359.5 to 0.4 (0.9 deg) doesn't, 359.5 to 1.0 (1.5 deg) does; zoom +0.5 doesn't, +1.5 does; pawn Some to None does; hover change does; `due` is false at 249 ms and true at 250; `controller_ptr` with `FakeMem`: a failed scan returns None and does not rescan before 1,000 ms (count reads by making the level pointer unreadable, then readable before 1,000 ms: still None); a cached controller is reused.
10. Docs (CRLF): `docs/architecture/client-telemetry.md`: a new section `## Navigation pose: client.nav.pose` before `## Session tags on client.dll.attached`: what is polled (D-LN5), the fields (C-LN2), the 250 ms check and thresholds, nothing while idle, the `nav_poll` kill switch, privacy (R5), static-only status. In the governor table's Must-keep row add "`client.nav.*`, `client.input.*` (rate-capped at the source, D-LN6)".

Lanes: the client DLL set (six steps).

### LN-12 RE: Black Market reopen

Host: windows · Agent: game-archaeology-specialist · Branch `lab-nav-telemetry/ln-12-bm-reopen-re` · Worktree `ln-12` · Commit `LN-12: RE the Black Market reopen search path`

Status: Done (static, 2026-10-22). The answer is in `.claude/reports/lab-nav-telemetry/LN-12-re.md` (main checkout; the orchestrator copies it to `worknotes/ln-12.md`). The root cause is our overlay's clock, not the binary, so no separate finding file was written; LN-12b records it as row U13 in `black-market-client-io.md`. D-LN2 decided (b): LN-12b. The optional server push (LN-12a) is dropped.

Why: F8, D-LN2. On reopen after the My Auctions tab the client sends no `BMSearch`.

Static only (the `re-lookup` skill order: repo docs and dispatch tables first, then Ghidra). No lab, no x64dbg, no code. Start with `docs/protocol/*-dispatch-table.md` (BM methods), `.claude/agent-memory/` (grep `BMSearch`, `black market`) and any `docs/reverse-engineering/findings/*black*`.

Find: what sends `BMSearch` when the BM window opens (Lua, Flash or native; the function or script and the address or file); what the My Auctions tab changes so a later open sends nothing; whether the client sends anything on reopen (a method, a CME event, a UI event that reaches the server) a server push could answer.

Output: `docs/reverse-engineering/findings/black-market-reopen-search.md` (CRLF); `docs/analysis/lab-nav-telemetry/worknotes/ln-12.md` with the D-LN2 options updated by the evidence (server push first, if a reopen signal exists); a memory entry under `.claude/agent-memory/game-archaeology-specialist/`. Mark every claim static or inferred.

### LN-15 RE: poll-model confirmations

Host: windows · Agent: game-archaeology-specialist · Branch `lab-nav-telemetry/ln-15-poll-model-re` · Worktree `ln-15` · Commit `LN-15: confirm the controller poll model statically`

Status: BlockedDependency (LN-11 merged: the finding file below is untracked until LN-11 commits it).

Why: D-LN5 rests on static readings; LN-08..LN-10 need them confirmed and LN-10 needs a field to poll.

Static only, same order as LN-12. Answer, each with addresses and a static/inferred mark:
1. `0x0056b890` (CEGUI owns the mouse): what it reads. Give a pollable chain (global VA, offsets, value type and the value meaning "CEGUI owns the mouse"), so LN-10 can read it with no call into the game.
2. Are `+0x4a0`/`+0x4a4` left untouched on release (helper `0x00e85540`, release branch), and do `0x00e84ac0`/`0x00e84b20` read them at release?
3. Do `0x00e84ac0`/`0x00e84b20` pick from the hover id `+0x4c0` or from a fresh trace?
4. Does PlayerTick `0x00e84d70` run inside `FEngineLoop::Tick` `0x00416ec0` (so a poll before the original Tick sees the previous frame)?
5. Are PlayerInput `+0xa4`/`+0xa8`/`+0xc0` floats, and are they cleared each tick?
6. Every writer of `+0x498` (xrefs), so LN-09's mode rows are complete.

Output: a section `## 6. Poll model (LN-15)` appended to `docs/reverse-engineering/findings/controller-mouse-input-model.md` (CRLF); update `.claude/agent-memory/game-archaeology-specialist/controller-mouse-input-model.md`. If an answer breaks D-LN5 (for example drag counters zeroed on release), say so first in the report.

### LN-19 `method_dropped` known-unbound names

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-19-known-unbound` · Worktree `ln-19` · Commit `LN-19: replay known-unbound method drops at INFO`

Status: BlockedDependency. **Pre-step (orchestrator):** dispatch an agent with the `telemetry-triage` skill to write `docs/analysis/lab-nav-telemetry/worknotes/ln-19-triage.md`: every `client.dispatch.method_dropped` row of the owner's 2026-10-22 Cellblock session, grouped by (`method_name`, `method_index`, `type_id`) with counts, and for each the verdict from `docs/protocol/client-method-dispatch-table.md`: bound on some class, or bound nowhere. No account names in the note.

Rules: R1 (server set for `cimmeria-admin-api`), R2, R4, R6, R8. D-LN8: by name only. Never add an `onBM*` method (it may be F8 evidence).

1. `crates/admin-api/src/routes/telemetry/replay_native.rs` :87: add to `KNOWN_UNBOUND_CLIENT_METHODS` each name the triage note marks "bound nowhere", with a doc bullet per name like the `onPlayerTeleport` one (index, why it is sent, the citation). A name marked "bound on some class" is not added; list it in the report as a real drop.
2. Test in `replay_tests.rs` next to `a_dropped_on_player_teleport_is_known_unbound_not_a_warn` (:164): one table-driven test that every added name replays at INFO with `drop_class = "known_unbound"`, and that an unlisted name still replays at WARN.
3. Docs (CRLF): `docs/architecture/client-telemetry.md`, the paragraph on `client.dispatch.method_dropped` (grep it): name the added methods.

Lanes: server set for `cimmeria-admin-api`.

## Wave 3: mouse input rows (after D-LN1)

LN-08, LN-09 and LN-10 each edit exactly one stub file from LN-07, with inline `#[cfg(test)] mod tests`, so they can run in parallel. None of them touches `tick.rs`, `mod.rs`, the governor or docs.

### LN-08 Client: button edges and click class

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-08-buttons` · Worktree `ln-08` · Commit `LN-08: emit mouse button edges with drag totals and click class`

Status: BlockedDependency (LN-06, LN-07; D-LN1 decided A). Rules: R1 (the client DLL set), R2, R3, R5, R6, R8. Edit only `crates/client-telemetry/src/hooks/nav/buttons.rs`.

```rust
pub const DRAG_THRESHOLD: i32 = 5;   // RE § 1: a drag under 5 counts on both axes is a click
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Button { Lmb, Rmb }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Edge { Down, Up }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum ClickClass { Select, Interact, Drag, Chord }
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonEvent { pub button: Button, pub edge: Edge, pub other_held: bool,
    pub hold_ms: Option<u64>, pub drag: Option<[i32; 2]>, pub class: Option<ClickClass>, pub entity_id: Option<i32> }
#[derive(Default)]
pub struct ButtonTracker { initialized: bool, prev: u32, down_at: [Option<u64>; 2], chord: [bool; 2] }
impl ButtonTracker { pub fn step(&mut self, now_ms: u64, ctrl: &ControllerState) -> Vec<ButtonEvent>; }
pub fn classify(button: Button, drag: [i32; 2], chord: bool) -> ClickClass;
pub fn button_fields(ev: &ButtonEvent, ui_owned: Option<bool>) -> Fields;
```

- `step`: the first call only records `prev = ctrl.buttons` (no events). Then per button, LMB first (bit `BUTTON_LMB`, index 0), then RMB (`BUTTON_RMB`, index 1): 0 to 1 gives `Down` (`down_at = now`, `chord` = other bit set now, `other_held` = the same); while held, the other bit set sets `chord = true`; 1 to 0 gives `Up` with `hold_ms = now - down_at` (None when unknown), `drag = Some(ctrl.drag)`, `class = classify(...)`, `entity_id = Some(ctrl.hover_id)` only for `Select`/`Interact` and a non-zero hover; then clear `down_at` and `chord`.
- `classify`: `Drag` when `|dx| >= 5 || |dy| >= 5`; else `Chord` when `chord`; else `Select` for LMB, `Interact` for RMB.
- `button_fields`: C-LN2 names; `class` as `select`/`interact`/`drag`/`chord`; `ui_owned` only when Some.
- `step(frame)`: a `static TRACKER: Mutex<ButtonTracker>` (poison recovered); emit each event as `client.input.button` through `crate::hooks::emit::emit`.
- Tests (inline): click LMB with drag (2, -3) gives Select with the hover id; RMB with drag (0, 7) gives Drag with no `entity_id`; RMB pressed while LMB is held, then both released, gives two Chord ups; a zero hover gives no `entity_id`; the first call emits nothing; `hold_ms` is the tick difference; `button_fields` omits `ui_owned` when None.

Lanes: the client DLL set (six steps).

### LN-09 Client: mode and hover transitions

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-09-transitions` · Worktree `ln-09` · Commit `LN-09: emit controller mode and hover transitions`

Status: BlockedDependency (LN-06, LN-07; D-LN1 decided A). Rules: R1 (the client DLL set), R2, R3, R5, R6, R8. Edit only `crates/client-telemetry/src/hooks/nav/transitions.rs`.

```rust
pub const MODE_CAP_PER_SEC: u32 = 8;
pub const HOVER_CAP_PER_SEC: u32 = 4;
#[derive(Clone, Copy, Debug, PartialEq)] pub struct Change<T> { pub value: T, pub prev: T, pub suppressed: u32 }
pub struct RateLimitedDiff<T> { cap: u32, initialized: bool, last_seen: T, last_emitted: T,
    window_start_ms: u64, emitted_in_window: u32, suppressed: u32 }
impl<T: Copy + PartialEq + Default> RateLimitedDiff<T> {
    pub fn new(cap: u32) -> Self;
    pub fn step(&mut self, now_ms: u64, value: T) -> Option<Change<T>>;
}
pub fn mode_fields(c: &Change<u32>) -> Fields;   // mode, prev, mouselook, jump, autorun, mousewalk, suppressed
pub fn hover_fields(c: &Change<i32>) -> Fields;  // entity_id, prev, suppressed
```

- `step`: the first call records the value (no event). A value differing from `last_seen` counts one change. The window is 1,000 ms from `window_start_ms` (reset when `now - start >= 1000`). When `value != last_emitted` and `emitted_in_window < cap`: return `Change { value, prev: last_emitted, suppressed }`, then set `last_emitted`, bump the count, zero `suppressed`. When over the cap, a counted change adds to `suppressed`. The catch-up is automatic: a later tick in a new window with `value != last_emitted` emits.
- `step(frame)`: two statics, `RateLimitedDiff::<u32>::new(MODE_CAP_PER_SEC)` fed `frame.ctrl.mode` and `RateLimitedDiff::<i32>::new(HOVER_CAP_PER_SEC)` fed `frame.ctrl.hover_id`; emit `client.input.mode` and `client.input.hover`.
- Tests (inline): the first value is silent; one change emits with the right `prev`; ten hover changes inside 1 s emit 4 and the next window's row carries the latest value and `suppressed`; a flip and flip-back inside the cap emit two rows; `mode_fields` decodes the four bools (0x9 gives mouselook and mousewalk).

Lanes: the client DLL set (six steps).

### LN-10 Client: `ui_owned`

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-10-ui-owned` · Worktree `ln-10` · Commit `LN-10: read whether CEGUI owns the mouse`

Status: BlockedDependency (LN-15, LN-07; D-LN1 decided A). **Before dispatch, a fresh architect fills the chain below from LN-15's § 6 answer 1.** If LN-15 found no pollable field, this packet is dropped and `ui_owned` stays absent.

Rules: R1 (the client DLL set), R2, R3, R6, R8. Edit only `crates/client-telemetry/src/hooks/nav/ui.rs`.

- `read_ui_owned(mem)`: follow `<UI_OWNER_GLOBAL_VA>` then `<offsets>`; any unreadable or null link gives None; return `Some(<value test>)`. Constants named and commented with the LN-15 citation.
- Tests (inline, `FakeMem`): owned, not owned, null global gives None, unreadable link gives None.

Lanes: the client DLL set (six steps).

### LN-16 Docs: mouse input rows

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-16-input-docs` · Worktree `ln-16` · Commit `LN-16: document the mouse input rows`

Status: BlockedDependency (LN-08, LN-09). Rules: R2, R4, R8. Docs only, no lane run.

1. `docs/architecture/client-telemetry.md`: a section `## Mouse input: client.input.*` right after LN-07's `## Navigation pose` section: the three (or four with LN-17) targets and fields from C-LN2, the click classes and the 5-count threshold, the rate caps and `suppressed`, `ui_owned` (only when LN-10 shipped), what is not captured (keys, text, cursor position), and that every address is static until LN-13.
2. `docs/operations/signoz/interaction-pose-views.md`: a section `## Mouse navigation` before `## Documentation debt`, with two ClickHouse queries in the file's style: one session's `client.input.button` and `client.nav.pose` rows in time order, and clicks by `class` with `entity_id`.
3. CRLF (R4). Check links.

### LN-17 Client: GetDeviceData aggregate in the player build (D-LN1 = B only)

Host: windows · Agent: rust-gameserver-dev · Branch `lab-nav-telemetry/ln-17-dinput-read` · Worktree `ln-17` · Commit `LN-17: read-only mouse aggregate in the player DLL`

Status: BlockedDecision (D-LN1 = B only). The owner answered "yes" to D-LN1 without choosing B, and the design needs only A, so this packet stays blocked and is dropped at close-out unless the owner explicitly picks B.

Rules: R1 (the client DLL set), R2, R3, R5, R6, R7, R8. Never modify a record or a return value.

1. Pure `crates/client-telemetry/src/hooks/input/mouse_window.rs`: aggregate `DIDEVICEOBJECTDATA` records (0x14 bytes; `dwOfs` 0 = X, 4 = Y, 8 = wheel, the rest ignored) into 100 ms windows: `dx`, `dy`, `wheel`, `records`. Flush on the first record after the window closes, or from the nav tick when 100 ms passed. Table tests over synthetic records.
2. Install, player build only (`#[cfg(not(feature = "lab-bridge"))]`): the same IAT `DirectInput8Create` interception `bridge/input/dinput.rs` uses (`IAT_DIRECT_INPUT8_CREATE`, :41; checked against `GetProcAddress` first), then a post-call swap of vtable slot 10 on the mouse device only; the detour calls the original, then feeds the records it returned to the aggregator. Under `lab-bridge`, install nothing new: `get_device_data_detour` (:417) feeds the same aggregator after its own work. A failed install logs one `client.hooks.inline.create_failed` row (R7).
3. Emit `client.input.mouse_window` with `buttons` and `mode` from the latest `NavFrame`.
4. Docs: one subsection in `client-telemetry.md` under the mouse input section.

Lanes: the client DLL set (six steps). The reviewer pass adds `server-authority-enforcer`-style scrutiny of "never modify" (a read-only diff of the detour).

## Wave 4: lab, launcher, close-out

### LN-13 Live confirmation (lab: ask the user first)

Status: BlockedDecision. The orchestrator asks the user before any lab use and waits until the lab is free. Run with the `lab-uat` skill and a lab-driver; grade against server and SigNoz evidence.

1. x64dbg non-freezing (log-only) breakpoints, fast resume off, on `0x00e85710`, `0x00e85860`, `0x00e83f40`, `0x00e84ac0`, `0x00e84b20`: record the hit order for an LMB click, an LMB drag, an RMB click on an NPC, an RMB drag, and both buttons held.
2. With a player DLL from the merged wave 3 (default features), perform the same five actions plus a zoom and an autorun toggle; check each gives the expected `client.input.*` and `client.nav.pose` rows (classes, `entity_id`, `mode` bits).
3. F3: record `pawn_ok` through a world entry and a zone change.
4. (Dropped: D-LN3 was decided (b), the quarantine is lifted by wave 1b, and the owner validates the debug-hub dialogs and the BM reopen after release, with no lab step.)

Output: `docs/analysis/lab-nav-telemetry/worknotes/ln-13.md` (which rows passed, which were never tested), and the RE finding's *inferred* rows updated.

### LN-18 Launcher: forward `launcher.update`

Host: windows · Agent: packet-coder · Branch `lab-nav-telemetry/ln-18-launcher-update` · Worktree `ln-18` · Commit `LN-18: forward launcher.update warnings to SigNoz`

Status: BlockedDependency (launcher-uat-fixes closed). **Before dispatch, a fresh architect re-checks the line numbers below against `main`,** since launcher-uat-fixes edits the launcher.

1. `crates/launcher/desktop/shell/src/telemetry.rs`: the forward layer (L124-129) passes `adoption` at INFO and above. Also pass target `launcher.update` at WARN and above, as `client.launcher.update`, the same way (`engine/.../game_telemetry/launcher_event.rs:12` names the adoption row).
2. `crates/admin-api/src/routes/telemetry/launcher_rows.rs:17`: accept `client.launcher.update` beside `client.launcher.adoption`, replayed the same way.
3. Tests: the forward filter passes `launcher.update` WARN and drops INFO; the server accepts and replays the new name.
4. Docs (CRLF): the LN-04 "Launcher-local rows" paragraph and a catalog row.

Lanes: the server set for `cimmeria-admin-api`; the launcher's own lane command from `docs/architecture/build-system.md` § Standalone desktop launcher validation.

### LN-14 Close-out

Host: windows · Agent: documentation-writer · Branch `lab-nav-telemetry/ln-14-close-out` · Worktree `ln-14` · Commit `LN-14: lab-nav-telemetry close-out`

After the campaign review loop. Status docs once (`docs/gap-analysis*`, `docs/project-status.md`); `docs/guides/unified-uat.md`: the owner's mouse session (what to do, the SigNoz queries from LN-16, the expected rows) and line 1370 "4 to 6 m" becoming "3 to 4 m"; the owner's post-release checks for wave 1b (after the overlay is re-signed and published: open the Black Market, close it, reopen it, and expect a second `BMSearch` in the server's `wire.in` and a working Search button; the debug-hub and Banker dialogs per LN-20e's K2/K3 text); the status-doc lines on the dialog quarantine (`docs/gap-analysis/stub-only-systems.md`, `docs/gap-analysis/core-gameplay.md`, `docs/project-status.md`: grep `quarantin`) and on the BM reopen (F8 resolved in the overlay); LN-17 marked dropped unless the owner picked B; the ledger's final statuses and the tested-vs-untested list; a board handoff through `ops`; `.claude/state/active-campaign` emptied only if it names this campaign.
