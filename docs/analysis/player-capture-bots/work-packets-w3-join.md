# Player capture work packets: wave 3a, the join keys (PC-26, PC-27, PC-17)

> Type: work packets. Rules, contract and index: [work-packets.md](work-packets.md). Ledger: [README.md](README.md). Wave 3a's DLL packets and the wave plan: [work-packets-w3-camera.md](work-packets-w3-camera.md). Wave 3b and the live gates: [work-packets-w3b-docs-gates.md](work-packets-w3b-docs-gates.md).

These packets run in wave 3a, beside the DLL chain (PC-15, PC-16, PC-16b), from `origin/main` after wave 2 merges. PC-26 ships alone. PC-27 and PC-17 ship as one PR from `pc-17`. `server-authority-enforcer` gives PC-27 and PC-17 a read-only second pass.

**How a camera row finds its pose row (D-PC19, D-PC20).** Two keys, best first, and neither needs a wire change:

1. **Sequence.** The server already receives each cell method call with its Mercury packet sequence (`packet_seq` in `dispatch_cell_method`). PC-26 stamps it on the pose row as `mercury_seq`. PC-16b reports the sequence range of the packets that carried each camera send (`client.camera.sent_seq`, joined to the camera row by `send_id`).
2. **Clock.** The DLL stamps each upload with its send time (PC-16). PC-27 uses it to measure the client clock offset per session to within the upload's network delay, and PC-17 writes `server_ts_ms` (the row's time on the server clock) on each camera row.

## Rules for every code packet in this file

1. **PowerShell only**, never bash. Start every shell call with `Set-Location <your worktree>;`, using the absolute path.
2. **Never run a compiling `cargo` directly.** `cargo fmt` may run bare. `clippy` and `nextest` go through `pwsh tools/build-lane/lane.ps1 cargo ...`, in the **foreground**. Commit as soon as they pass.
3. **Git limits.** Never use `git stash`, `git worktree prune`, `--no-verify`, push or force. Commit once, locally, with the given subject and your brief's attribution lines.
4. **File size.** Soft cap 500 lines, hard cap 700. Never grow a file past 700. New tests go in sibling `*_tests.rs` files, as each packet says.
5. **Id fields.** Every new `tracing` field ending in `_id` is paired with a `_name` field or carries a same-line `// nt:id-only <reason>`. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-server unpaired_id` stays green. Never set `NT_BASELINE_BLESS`.
6. **Privacy (D-PC2).** No `account_name`, `player_name`, character `entity_name`, IP or host on any new row.
7. **Tests.** Unit tests, no live DB. Each one fails when its change is reverted, and says why in a one-line doc comment.
8. **Trust.** Every value these packets read from the client (`ts_ms`, the send stamp, bag fields) is a claim. It may place or label the client's own rows; it never changes game state, never refuses a chunk, and a bad value reads as absent.
9. **Docs** under `docs/` are CRLF. **Scope:** edit only the named files. If an assumption is false, return `blocked` with the reason.

---

## PC-26 `mercury_seq` on the pose row

- **Agent:** packet-coder.
- **Worktree:** `pc-26`. **Branch:** `player-capture-bots/pc-26-pose-mercury-seq`. **Crates:** `cimmeria-cell-world` and `cimmeria-cell`.
- **Commit subject:** `feat(cell): interaction.pose carries the Mercury sequence of the call that caused it`.
- **Files:**
  - `crates/cell-world/src/cell/interaction_pose/inbound.rs` (new)
  - `crates/cell-world/src/cell/interaction_pose/inbound_tests.rs` (new)
  - `crates/cell-world/src/cell/interaction_pose/mod.rs`
  - `crates/cell-world/src/cell/interaction_pose/emit.rs`
  - `crates/cell-world/src/cell/interaction_pose/emit_tests.rs`
  - `crates/cell/src/cell/dispatch/router.rs` (269 lines; 2 lines added)
  - `crates/cell/src/cell/dispatch/mod.rs` (one test `mod` line)
  - `crates/cell/src/cell/dispatch/pose_inbound_tests.rs` (new)

**Contract excerpt (addendum A1 in [work-packets.md](work-packets.md#addendum-a1-mercury_seq-on-interactionpose-pc-26)).**
- The pose row gains one field, `mercury_seq`, **after** `pose_seq`: 30 fields plus the message (the `tracing` cap is 32 including the message).
- `mercury_seq` is the `packet_seq` the router received with the client's `interact` (cell method 74) or `useItem` (cell method 39) call by the same actor. It is absent when no such call was seen in the last 5 s, or when the router had no sequence.
- `interact` is exact: the handler runs inside the router call. `useItem` is best effort: the pose row is written when the base's `ItemUsed` comes back, so it takes the newest `useItem` call by that actor within 5 s.

**Change.**

1. `interaction_pose/inbound.rs`:
   ```rust
   //! The Mercury sequence of the last interaction call per actor (PC-26, D-PC19).
   pub const INBOUND_MAX_AGE: Duration = Duration::from_secs(5);
   const PRUNE_AT: usize = 2048;
   static LAST_INBOUND: LazyLock<Mutex<HashMap<(u32, u8), (u32, Instant)>>> = LazyLock::new(Mutex::default);
   /// Note a client cell method call. Only `interact` and `useItem` are kept; a `None` seq is ignored.
   pub fn note_inbound(entity_id: u32, method_index: u16, packet_seq: Option<u32>);
   pub fn note_inbound_at(entity_id: u32, method_index: u16, packet_seq: Option<u32>, now: Instant);
   /// Take the noted seq for this actor and kind if it is younger than INBOUND_MAX_AGE. A seq is taken once.
   pub fn take_inbound(entity_id: u32, kind: InteractionKind, now: Instant) -> Option<u32>;
   ```
   - The map key is `(entity_id, slot)`, with slot 0 for `interact` and 1 for `useItem`. A newer call overwrites the older one.
   - Compare `method_index` with `cimmeria_wire::cell::cell_methods::player::INTERACT` (74) and `cimmeria_wire::cell::cell_methods::inventory::USE_ITEM` (39); both are `u16`. Never hard-code 74 or 39.
   - At `PRUNE_AT` entries, drop entries older than `INBOUND_MAX_AGE` first, as `emit.rs`'s `admit` prunes.
   - Lock with `unwrap_or_else(std::sync::PoisonError::into_inner)`, as `emit.rs` does.
   - `InteractionKind` is PC-01's enum in `geometry.rs`; import it the way `emit.rs` does.
2. `interaction_pose/mod.rs`: add `pub mod inbound;` and `#[cfg(test)] mod inbound_tests;`.
3. `interaction_pose/emit.rs`, in `emit_interaction_pose_at`:
   - **First**, before `admit`: `let mercury_seq = super::inbound::take_inbound(pose.entity_id, kind, now);`. A throttled row then still uses up its seq, so a later row never borrows an earlier call's seq.
   - Add `mercury_seq,` after `pose_seq` in the `tracing::info!`. It is an `Option<u32>`, so absent stays absent.
4. `crates/cell/src/cell/dispatch/router.rs`, in `dispatch_cell_method`, on the line right after the `super::ability_receipt::log_receipt(...)` call (before the GM gate):
   ```rust
   // player-capture-bots: the pose row's mercury_seq (PC-26).
   cimmeria_cell_world::cell::interaction_pose::inbound::note_inbound(entity_id, method_index, packet_seq);
   ```
   Use the parameter names the function really has.

**Tests.**

`inbound_tests.rs` (actor ids 9221 to 9226, so they never meet PC-02's 9201-9207):
- `interact_seq_is_taken_once`: `note_inbound_at(9221, INTERACT, Some(555), t0)`; `take_inbound(9221, Interact, t0 + 10ms) == Some(555)`, and a second take is `None`.
- `kinds_do_not_cross`: a `useItem` note does not answer an `Interact` take, and the reverse.
- `old_seq_expires`: a take at `t0 + INBOUND_MAX_AGE + 1ms` is `None`.
- `other_methods_and_none_are_ignored`: method 0 with `Some(1)`, and `INTERACT` with `None`, leave nothing to take.
- `newer_call_wins`: two notes, `Some(1)` then `Some(2)`, give `Some(2)`.

`emit_tests.rs`, one new test, `row_carries_mercury_seq_once` (actor 9227, the existing scene helper): note `Some(4242)` for `INTERACT`, emit an `Interact` row (it has `mercury_seq == 4242`), then emit again 300 ms later (that row has no `mercury_seq` key). Keep `row_has_at_most_31_fields` green.

`crates/cell/src/cell/dispatch/pose_inbound_tests.rs`, modelled on `ability_receipt_tests.rs` (its `world()` and `route()` helpers; copy them with a caller id of 9228):
- `router_notes_the_interact_seq`: route `CM_INTERACT` with 4 zero bytes of args and `Some(777)`, then `take_inbound(9228, InteractionKind::Interact, Instant::now()) == Some(777)`. No target exists, so the handler writes no pose row and cannot take the seq first. Removing the router line fails this test.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-26;`:

1. `cargo fmt -p cimmeria-cell-world -p cimmeria-cell --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-world -p cimmeria-cell --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-world interaction_pose`
4. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell dispatch`
5. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-server unpaired_id`
6. `git diff --check`

---

## PC-27 admin-api: measure the clock from the send stamp

- **Agent:** packet-coder.
- **Worktree:** `pc-27`. **Branch:** `player-capture-bots/pc-27-chunk-clock`. **Crate:** `cimmeria-admin-api`.
- **Commit subject:** `feat(admin-api): measure the client clock from the upload send stamp and keep it per chunk`.
- **Files:** all under `crates/admin-api/src/routes/telemetry/`:
  - `chunk.rs` (328 lines)
  - `client_pid_tests.rs` (83 lines)
  - `entity_labels.rs` (427 lines; add at most 45)
  - `entity_labels_tests.rs` (286 lines)
  - `replay.rs` (258 lines)
  - `replay_native.rs` (380 lines; 2 lines added)

**Why (D-PC18).** NT-40 places client rows on the server clock with a per-session offset: the smallest `recv - newest row ts_ms` seen in a 5-minute window. That is the true offset plus the smallest flush delay, and the DLL flushes on a cadence, so the error is up to one flush interval. With the DLL's send stamp (header `x-cimmeria-client-sent-ms`, PC-16), the sample becomes `recv - sent`: the true offset plus the network delay. No wire change; the stamp rides the existing HTTP upload.

**Change.**

1. `chunk.rs`:
   - Below `client_pid_header`, add `pub(super) const CLIENT_SENT_MS_HEADER: &str = "x-cimmeria-client-sent-ms";` and `pub(super) fn client_sent_ms_header(headers: &HeaderMap) -> Option<i64>`, parsed like `client_pid_header`: absent, non-numeric or `<= 0` reads as `None`, and never refuses the chunk.
   - In `chunk_flow`, next to `let client_pid = client_pid_header(&parts.headers);`, add `let client_sent_ms = client_sent_ms_header(&parts.headers);` and `let recv = SystemTime::now();`. Taking `recv` before the body is read keeps the body transfer out of the sample.
   - Pass `recv` and `client_sent_ms` to `name_chunk` in place of the inline `SystemTime::now()`.
2. `entity_labels.rs`:
   - Add:
     ```rust
     /// One chunk's clock: the session offset after this chunk, and when it arrived.
     #[derive(Debug, Clone, Copy, PartialEq, Eq)]
     pub(super) struct ChunkClock { pub(super) offset_ms: i64, pub(super) recv_ms: i64 }
     impl ChunkClock {
         /// A client `ts_ms` on the server clock, never later than the receive time.
         pub(super) fn server_ts_ms(&self, ts_ms: i64) -> i64 { ts_ms.saturating_add(self.offset_ms).min(self.recv_ms) }
     }
     ```
   - `NamingPlan::build` gains a last parameter `client_sent_ms: Option<i64>`. The sample stamp is `client_sent_ms.filter(|s| *s >= newest).unwrap_or(newest)`, passed to `clock.observe(recv_ms, stamp)`. A stamp older than the newest row is incoherent and is ignored.
   - `NamingPlan` gains `clock: Option<ChunkClock>`, set after `observe` from `clock.offset_ms()` and `recv_ms` (`None` when there is no offset). Use `ChunkClock::server_ts_ms` for the existing `at` computation, so there is one formula.
   - `EntityLabels` gains `clock: Option<ChunkClock>`, copied over in `answered`, with `pub(super) fn clock(&self) -> Option<ChunkClock>`. `EntityLabels::none()` keeps `None`.
   - `name_chunk` gains `client_sent_ms: Option<i64>` after `recv`, passed to `build`.
   - Update the module doc's clock paragraph in place: one sentence saying the send stamp, when present, replaces the newest row's `ts_ms` as the sample.
3. `replay.rs`: `replay_ndjson_named` passes `None` for the stamp. In `replay_events_from_client`, the `ReplayNames` literal gains `clock: labels.clock(),`.
4. `replay_native.rs`: `ReplayNames` gains `pub clock: Option<super::entity_labels::ChunkClock>,` with a one-line doc comment ("The chunk's clock, for rows that carry `server_ts_ms`"). Nothing reads it yet; PC-17 does.

**Tests.**

`entity_labels_tests.rs`: the `plan` helper and the direct `NamingPlan::build` call at about line 166 pass `None`. Add:
- `sent_stamp_removes_flush_age`: a fresh `SessionClock`, one row at `ts_ms` 10 000, `recv = at(1_015_000)`, stamp `Some(14_900)`. The offset is 1 000 100, not 1 005 000. Reverting the stamp fails it.
- `incoherent_stamp_falls_back_to_newest`: stamp `Some(9_000)` (older than the row) gives 1 005 000.
- `plan_carries_the_chunk_clock`: after a build, the plan's clock (add a `#[cfg(test)] pub(super) fn clock(&self)` to `NamingPlan`, as `queries` is) equals `ChunkClock { offset_ms, recv_ms: 1_015_000 }`.
- `server_ts_is_capped_at_receive`: `ChunkClock { offset_ms: 100, recv_ms: 5_100 }` gives 1 100 for 1 000, and 5 100 for 5 050.

`client_pid_tests.rs`: `sent_ms_header_parses_or_reads_none`, covering a valid value, an absent header, `"abc"` and `"-5"`.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-27;`:

1. `cargo fmt -p cimmeria-admin-api --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-admin-api --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-admin-api telemetry`
4. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-server --all-targets -- -D warnings` (its tests call `replay_ndjson_named`, whose signature must not change)
5. `git diff --check`

**Ship:** with PC-17, as one PR from `pc-17`.

---

## PC-17 admin-api camera shape

- **Agent:** packet-coder.
- **Worktree:** `pc-17`. **Branch:** `player-capture-bots/pc-17-admin-camera-shape`. **Crate:** `cimmeria-admin-api`.
- **Stacked on PC-27.** First command: `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-17; git merge --ff-only player-capture-bots/pc-27-chunk-clock`. If the fast-forward fails, return `blocked`.
- **Commit subject:** `feat(admin-api): replay client.camera rows in their own id-only shape with server time`.
- **Files:** all under `crates/admin-api/src/routes/telemetry/`:
  - `replay_camera.rs` (new)
  - `replay_camera_tests.rs` (new)
  - `replay.rs` (about 5 lines changed)
  - `mod.rs` (add `mod replay_camera;` after `mod replay;`, and `#[cfg(test)] mod replay_camera_tests;` among the test mods)

**Why (D-PC14).** The game shape (`game_event!` in `replay_native.rs`) names `entity_id` with an `entity_name`, which is a character name. It is also at `tracing`'s 32-field cap. Camera rows get their own shape, with no names, the numbers lifted, and the row's server time.

**Contract excerpt.**
- `client.camera.at_interact` bag: `method`, `entity_id`, `send_id`, `zoom`, `yaw_offset_deg`, `pitch_offset_deg`, `camera_flags`, `camera_gain`, `camera_pose_zero`, `camera_x/y/z`, `camera_yaw_deg`, and under S-CAM A `pawn_x/y/z`, `pawn_yaw_deg`, `pawn_pitch_deg`.
- `client.camera.sent_seq` bag: `send_id`, `method`, `mercury_seq_first`, `mercury_seq_last`, `packets`, `seq_bits`, and sometimes `evicted_unmatched`.
- Both replay under target `client.native` at INFO, with `client_target` set to the DLL target.

**Change.**

1. `replay_camera.rs`:
   ```rust
   pub(super) const CAMERA_TARGET_PREFIX: &str = "client.camera.";
   pub(super) const SENT_SEQ_TARGET: &str = "client.camera.sent_seq";
   pub(super) fn is_camera_target(target: &str) -> bool { target.starts_with(CAMERA_TARGET_PREFIX) }
   #[derive(Debug, Default, Clone, PartialEq)]
   pub(super) struct CameraLift { /* Option<f64> per numeric bag field, Option<bool> camera_pose_zero,
       Option<String> method, Option<i64> entity_id, Option<u32> send_id, Option<u32> camera_flags,
       Option<u32> mercury_seq_first, Option<u32> mercury_seq_last, Option<u32> packets */ }
   impl CameraLift { pub(super) fn from_fields(f: &serde_json::Map<String, Value>) -> Self; } // numbers or numeric strings
   pub(super) fn replay_camera(claims: &TokenClaims, e: ClientNativeEvent, names: &ReplayNames<'_>);
   ```
   `replay_camera` computes `server_ts_ms = names.clock.map(|c| c.server_ts_ms(e.ts_ms))` and `clock_offset_ms = names.clock.map(|c| c.offset_ms)`, then writes one `tracing::info!(target: "client.native", ...)`, always at INFO, in one of two shapes.

   **Shape A, every camera target except `sent_seq`.** Fields, in order:
   1. The identity block, copied from `game_event!`: `session_id` and `install_id` with their `nt:id-only` comments, `cimmeria.session_kind`, `lab`, `ts_ms`, `seq`, `client_pid`, `client_target`.
   2. `server_ts_ms`, `clock_offset_ms`.
   3. `method`; `entity_id = l.entity_id, // nt:id-only privacy: no character name on camera rows (D-PC2, D-PC14)`; `send_id = l.send_id, // nt:id-only DLL send counter, names nothing`.
   4. `zoom`, `yaw_offset_deg`, `pitch_offset_deg`, `camera_flags`, `camera_gain`, `camera_pose_zero`.
   5. `camera_x`, `camera_y`, `camera_z`, `camera_yaw_deg`.
   6. `pawn_x`, `pawn_y`, `pawn_z`, `pawn_yaw_deg`, `pawn_pitch_deg`.
   7. `fields = %Value::Object(bag)`, then the message `"{client_target}"`.

   That is 29 fields plus the message. `client_level` and `names_source` are left out: the level is always INFO, and nothing is named.

   **Shape B, `client.camera.sent_seq`.** The identity block, `server_ts_ms`, `clock_offset_ms`, `method`, `send_id` (same comment), `mercury_seq_first`, `mercury_seq_last`, `packets`, `fields`, then the message. 16 fields plus the message.

   Take every value from `CameraLift`. Absent stays absent.
2. `replay.rs`, in `replay_event`'s `TelemetryEvent::ClientNative(e)` arm:
   ```rust
   TelemetryEvent::ClientNative(e) if super::replay_camera::is_camera_target(&e.target) => {
       super::replay_camera::replay_camera(claims, e, names);
   }
   ```
   placed before the existing arm, which stays as it is. Adjust it to the real types if `e.target` is not a `String`. `replay_native.rs` is not touched.

**Tests.** `replay_camera_tests.rs`, unit tests. Use `super::replay_tests::{capture, claims}`, and copy the `ClientNativeEvent` builder from `replay_tests.rs` (`fn native`) or `replay_names_tests.rs` (`fn row`). Drive rows through `replay_event` with a `ReplayNames` you build in the test, so the routing in `replay.rs` is covered.

- `camera_row_is_replayed_without_names`:
  - A `client.camera.at_interact` event with `{method:"interact", entity_id:4242, send_id:9, zoom:400.0, yaw_offset_deg:90.0, camera_pose_zero:false, camera_x:1.5}`, and an `entity_label` that names 4242.
  - One captured row, target `client.native`, level INFO, with `client_target == "client.camera.at_interact"`.
  - `zoom`, `send_id` and `camera_x` are lifted.
  - There is **no `entity_name` key**.
- `camera_row_stays_info_whatever_the_client_level`: client level `warn` still gives INFO.
- `non_camera_target_keeps_the_game_shape`: a `client.net.out` event still has `method_name`, which proves the guard is scoped.
- `server_ts_comes_from_the_chunk_clock`: with `clock: Some(ChunkClock { offset_ms: 100, recv_ms: 5_100 })` and `ts_ms` 1 000, the row has `server_ts_ms == 1100` and `clock_offset_ms == 100`; with `clock: None`, neither key is present.
- `sent_seq_row_lifts_the_range`: a `client.camera.sent_seq` event with `{send_id:9, method:"interact", mercury_seq_first:100, mercury_seq_last:101, packets:2}` gives one INFO row with those lifted, and no `zoom` key.
- `lift_reads_numeric_strings`: `CameraLift::from_fields` with `"zoom": "400"` gives `Some(400.0)`.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-17;`:

1. `cargo fmt -p cimmeria-admin-api --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-admin-api --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-admin-api telemetry`
4. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-server unpaired_id`
5. `git diff --check`

**Ship:** one PR covering PC-27 and PC-17, from `pc-17`. It reaches colo through a colo release, which ops runs after the merge (PC-07b). **Docs owed:** none; PC-18 writes them.
