# Player capture work packets: wave 3a, DLL camera (PC-15, PC-16, PC-16b)

> Type: work packets. Rules, contract and index: [work-packets.md](work-packets.md). Ledger: [README.md](README.md). The rest of wave 3a: [work-packets-w3-join.md](work-packets-w3-join.md). Wave 3b and the live gates: [work-packets-w3b-docs-gates.md](work-packets-w3b-docs-gates.md).

Wave 3a starts from `origin/main` **after wave 2 merges and PC-06 has returned**. It has six code packets in three chains:

| Chain | Packets | Ships as |
|---|---|---|
| DLL camera (this file) | PC-15, then PC-16, then PC-16b | one PR from `pc-16b` |
| Server seq (w3-join) | PC-26 | its own PR |
| admin-api (w3-join) | PC-27, then PC-17 | one PR from `pc-17` |

- The orchestrator copies the last line of PC-06's report (`S-CAM: A` or `S-CAM: B`) into the briefs of PC-15, PC-16 and PC-18.
- The three chains run in parallel. One packet-review-fixer reviews the wave. Add `server-authority-enforcer` as a read-only second pass on PC-27 and PC-17, which handle client-supplied data.
- **Consent (owner, 2026-10-18):** the "Send game diagnostics while playing" opt-in covers the camera ("telemetry is telemetry"). No packet changes the launcher disclosure. The DLL hash is compiled into the launcher, so the new DLL reaches players only through a launcher release, which ops runs after the merge (pre-authorized; see PC-07b).

## Rules for every code packet in this file

1. **PowerShell only**, never bash. Start every shell call with `Set-Location <your worktree>;`, using the absolute path.
2. **Never run a compiling `cargo` directly.** `cargo fmt` may run bare. `clippy` and `nextest` go through `pwsh tools/build-lane/lane.ps1 cargo ...`, in the **foreground**. Commit as soon as they pass.
3. **Git limits.** Never use `git stash`, `git worktree prune`, `--no-verify`, push or force. Commit once, locally, with the given subject and your brief's attribution lines.
4. **File size.** Soft cap 500 lines, hard cap 700. Never grow a file past 700. The line counts below are from 2026-10-18.
5. **Id fields.** Every new `tracing` field ending in `_id` is paired with a `_name` field or carries a same-line `// nt:id-only <reason>`. DLL bag fields are JSON, not `tracing` fields, so this applies to admin-api only.
6. **Privacy (D-PC2).** No `account_name`, `player_name`, character `entity_name`, IP or host on any new row.
7. **Tests.** Unit tests, no live DB. Each one fails when its change is reverted, and says why in a one-line doc comment.
8. **The DLL crate builds for 32-bit.** For `cimmeria-client-telemetry`, every clippy and nextest call adds `--target i686-pc-windows-msvc`, and each runs twice, with and without `--features lab-bridge`. A host-only run is a false green.
9. **Docs** under `docs/` are CRLF. **Scope:** edit only the named files. If an assumption is false, return `blocked` with the reason.

---

## PC-15 DLL camera reader

- **Agent:** packet-coder.
- **Worktree:** `pc-15`. **Branch:** `player-capture-bots/pc-15-dll-camera-reader`. **Crate:** `cimmeria-client-telemetry`.
- **Commit subject:** `feat(client-telemetry): read the player camera and pawn pose in process`.
- **Brief input:** the S-CAM line. With **`S-CAM: B`**, skip every pawn item below (`PAWN_*`, `read_pawn_pose` and its tests).
- **Files:**
  - `crates/client-telemetry/src/hooks.rs` (166 lines; add one module declaration)
  - `crates/client-telemetry/src/hooks/camera/mod.rs` (new)
  - `crates/client-telemetry/src/hooks/camera/layout.rs` (new)
  - `crates/client-telemetry/src/hooks/camera/read.rs` (new)
  - `crates/client-telemetry/src/hooks/camera/tests.rs` (new)

**Source of truth for the layout.** The lab's readers in `crates/lab/src/supervisor/world/memory.rs`: `find_player_camera` (:290), `camera_state` (:388), `decode_pose` (:65) and `real_camera_pose` (:427). They are proven live (findings `cegui-mouse-input-feed.md` §10). The DLL runs in process: there is **no slide**, so use the VAs as they are. Do not edit the lab crate.

**Change.**

1. `hooks.rs`: next to the other modules, add `#[cfg_attr(not(target_arch = "x86"), allow(dead_code))] pub(crate) mod camera;`.
2. `camera/layout.rs` holds the constants, each with a doc comment citing its lab constant:

   | Constant | Value |
   |---|---|
   | `GLEVEL_PTR_VA` | `0x01EE_2684` |
   | `WORLD_LEVEL_OFF` | `0x50` |
   | `LEVEL_ACTORS_OFF` | `0x3C` |
   | `MAX_LEVEL_ACTORS` | `8192` |
   | `PLAYER_CONTROLLER_VTABLE_VA` | `0x019E_2B2C` |
   | `PLAYER_CAMERA_VTABLE_VA` | `0x019E_1134` |
   | `CONTROLLER_CAMERA_OFF` | `0x2B0` |
   | `CAMERA_FLAGS_OFF` | `0x348` |
   | `CAMERA_GAIN_OFF` | `0x358` |
   | `CAMERA_DIST_OFF` | `0x360` |
   | `CAMERA_PITCH_OFF` | `0x368` |
   | `CAMERA_YAW_OFF` | `0x36C` |
   | `CAMERA_BLOCK_LEN` | `0x370 - 0x348` |
   | `ACTOR_LOCATION_OFF` | `0xDC` |
   | `POSE_LEN` | `0x18` |
   | `ENTITY_MANAGER_VA` | `0x01EF_244C` |
   | `MANAGER_LOCAL_ENTITY_OFF` | `0x0C` |
   | `ENTITY_PAWN_OFF` | `0x08` |

   The pawn path is `[[ENTITY_MANAGER_VA] + 0x0C] + 0x08`. If PC-06's report names a different pawn path, use that one and cite it.
3. `camera/read.rs`. The pure decoders and pure geometry:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq)]
   pub struct CameraState { pub flags: u32, pub gain: f32, pub distance: f32, pub pitch: i32, pub yaw: i32 }
   #[derive(Debug, Clone, Copy, PartialEq)]
   pub struct ActorPose { pub pos: [f32; 3], pub pitch: i32, pub yaw: i32 } // client units
   pub fn decode_camera(b: &[u8]) -> Option<CameraState>;   // None unless b.len() >= CAMERA_BLOCK_LEN
   pub fn decode_actor_pose(b: &[u8]) -> Option<ActorPose>; // f32 x3 at 0, pitch i32 at 0xC, yaw i32 at 0x10
   pub fn rot_deg(units: i32) -> f32;                       // units * 360 / 65536, not wrapped
   pub fn rot_deg_360(units: i32) -> f32;                   // wrapped to [0, 360)
   pub fn client_to_server(p: [f32; 3]) -> [f32; 3];        // [p[1]/100, p[2]/100, p[0]/100]
   ```
   The `Mem` readers. `Mem` is `crate::hooks::entity_trace::map::Mem`, with `u32_at` and `bytes_at`.
   ```rust
   pub fn find_player_camera(mem: &impl Mem) -> Option<u32>;           // the lab's scan, then the camera vtable check
   pub fn camera_ptr(mem: &impl Mem, cache: &AtomicU32) -> Option<u32>; // cached pointer if its vtable still matches, else rescan and store
   pub fn read_camera(mem: &impl Mem, camera: u32) -> Option<CameraState>;
   pub fn read_camera_pose(mem: &impl Mem, camera: u32) -> Option<ActorPose>; // None when pos == [0,0,0]
   pub fn read_pawn_pose(mem: &impl Mem) -> Option<ActorPose>;          // S-CAM A only; None on any null
   ```
   Never dereference a raw pointer; every read goes through `Mem`.
4. `camera/mod.rs` has the module doc (what is read, when, and that it rides the existing diagnostics opt-in, D-PC3 and F13), plus `pub mod layout; pub mod read; #[cfg(test)] mod tests;`. PC-16 adds the emitter.

**Tests.** `camera/tests.rs`, unit tests using `crate::hooks::entity_trace::map::fake::FakeMem` (`FakeMem::default()`, `set(addr, u32)`; store an f32 as `v.to_bits()`).

- `decode_camera_reads_each_field`: build the 0x28-byte block with known flags, gain, distance 400.0, pitch -2048 and yaw 16384. Each field is decoded, and a short buffer gives `None`.
- `rotator_units_to_degrees`: `rot_deg(16384) == 90.0`, `rot_deg(-2048) == -11.25` and `rot_deg_360(-16384) == 270.0`.
- `client_to_server_swaps_axes`: `[100, 200, 300]` maps to `[2, 3, 1]`.
- `finds_the_camera_through_the_level`:
  - A fake world with a level whose Actors array holds 3 actors; the second has the controller vtable, and its `+0x2B0` points at a camera with the camera vtable.
  - `find_player_camera` returns that camera.
  - With a wrong camera vtable, it returns `None`.
- `cache_rechecks_the_vtable`:
  - After a first `camera_ptr`, overwrite the cached camera's vtable word and place a second valid camera.
  - `camera_ptr` returns the second camera, which proves the recheck.
- `zero_camera_pose_is_none`: a pose block of zeros gives `None`, and a non-zero one gives `Some`.
- `pawn_pose_follows_the_manager` (S-CAM A only): manager → entity → pawn → pose at `+0xDC` decodes. A null at any link gives `None`.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-15;`:

1. `cargo fmt -p cimmeria-client-telemetry --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-client-telemetry --target i686-pc-windows-msvc --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-client-telemetry --features lab-bridge --target i686-pc-windows-msvc --all-targets -- -D warnings`
4. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-client-telemetry --target i686-pc-windows-msvc camera`
5. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-client-telemetry --features lab-bridge --target i686-pc-windows-msvc camera`
6. `git diff --check`

---

## PC-16 Camera at interact: emitter, governor, send stamp

- **Agent:** packet-coder.
- **Worktree:** `pc-16`. **Branch:** `player-capture-bots/pc-16-camera-at-interact`. **Crate:** `cimmeria-client-telemetry` only.
- **Stacked on PC-15.** First command: `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-16; git merge --ff-only player-capture-bots/pc-15-dll-camera-reader`. If the fast-forward fails, return `blocked`.
- **Brief input:** the S-CAM line. With `S-CAM: B`, write no `pawn_*` field.
- **Commit subject:** `feat(client-telemetry): client.camera.at_interact on interact and useItem, and a send stamp on each upload`.
- **Files:**
  - `crates/client-telemetry/src/hooks/camera/emit.rs` (new)
  - `crates/client-telemetry/src/hooks/camera/emit_tests.rs` (new)
  - `crates/client-telemetry/src/hooks/camera/mod.rs`
  - `crates/client-telemetry/src/hooks/inline_hooks/net_out.rs` (234 lines)
  - `crates/client-telemetry/src/governor/classify.rs` (316 lines)
  - `crates/client-telemetry/src/governor/tests/classify.rs`
  - `crates/client-telemetry/src/uploader_post.rs` (165 lines)
  - `crates/client-telemetry/src/uploader_tests.rs` (327 lines)

**Contract excerpt.**
- Target `client.camera.at_interact`, level `info`, once per outbound `interact` or `useItem` (cell methods 74 and 39, `docs/protocol/cell-method-dispatch-table.md`).
- The bag fields are listed below. All angles are degrees.

  | Field | Value |
  |---|---|
  | `method` | `interact` or `useItem` |
  | `entity_id` | the local player id |
  | `send_id` | the DLL send counter shared with `client.ability.sent`; PC-16b fills it. PC-16 passes `None`, which omits the key. |
  | `zoom` | the camera distance |
  | `yaw_offset_deg`, `pitch_offset_deg` | `rot_deg` of the yaw and pitch offsets |
  | `camera_flags`, `camera_gain` | from the camera block |
  | `camera_pose_zero` | bool |
  | `camera_x`, `camera_y`, `camera_z`, `camera_yaw_deg` | `client_to_server` of the camera pose and `rot_deg_360` of its yaw; absent when the camera pose is zero |
  | `pawn_x`, `pawn_y`, `pawn_z`, `pawn_yaw_deg`, `pawn_pitch_deg` | **S-CAM A only**; absent when the pawn cannot be read |

- **Send stamp (D-PC18).** Every upload POST carries the header `X-Cimmeria-Client-Sent-Ms`: the DLL's wall clock in unix ms when the POST is built. This is the same clock as each row's `ts_ms` (`queue.rs` `now_ms`, `SystemTime`). admin-api uses it to measure the clock skew (PC-27).

**Change.**

1. `camera/emit.rs`:
   - A pure function, `pub fn camera_fields(method: &str, entity_id: Option<i32>, send_id: Option<u32>, cam: Option<CameraState>, cam_pose: Option<ActorPose>, pawn: Option<ActorPose>) -> crate::hooks::entity_trace::Fields`, which builds the bag above. `send_id` is pushed right after `entity_id`, only when `Some`.
   - It returns an empty bag (and the caller emits nothing) when `cam` is `None`. A zero camera pose gives `camera_pose_zero: true` and no `camera_*` position fields.
   - The live function:
     ```rust
     static CAMERA: AtomicU32 = AtomicU32::new(0);
     #[cfg(all(target_os = "windows", target_arch = "x86"))]
     pub fn at_interact(method: &str, entity_id: Option<i32>, send_id: Option<u32>) {
         let mem = LiveMem;
         let Some(cam) = read::camera_ptr(&mem, &CAMERA) else { return };
         let fields = camera_fields(method, entity_id, send_id, read::read_camera(&mem, cam),
             read::read_camera_pose(&mem, cam), read::read_pawn_pose(&mem));
         if fields.is_empty() { return; }
         let target = "client.camera.at_interact";
         let decision = trace::throttle(&format!("{target}:{method}"), entity_id.unwrap_or(0));
         if let Some(f) = trace::with_suppressed(fields, decision) {
             crate::hooks::emit::emit(target, "info", f);
         }
     }
     ```
     Under S-CAM B, pass `None` for the pawn.
   - Declare `pub mod emit; #[cfg(test)] mod emit_tests;` in `camera/mod.rs`.
2. `net_out.rs`, in `observe`, after the `versionInfoRequest` block:
   ```rust
   // player-capture-bots: the camera as the player clicked (D-PC3, diagnostics opt-in).
   #[cfg(all(target_os = "windows", target_arch = "x86"))]
   if matches!(name_key, "interact" | "useItem") {
       crate::hooks::camera::emit::at_interact(name_key, local_id, None);
   }
   ```
   `local_id` is the local player's id, already computed above. PC-16b moves this call into `route_detour`; keep it in `observe` here.
3. `governor/classify.rs`:
   - Add the variant `InteractionCamera` to `KeepReason`, with `as_str` returning `"interaction_camera"`.
   - Add `keep(Prefix("client.camera."), KeepReason::InteractionCamera),` to the rules list, next to `keep(Prefix("client.ability."), KeepReason::AbilityTrace)`. A prefix, because PC-16b adds `client.camera.sent_seq`.
   - In `governor/tests/classify.rs`, add `("client.camera.at_interact", KeepReason::InteractionCamera)` to the existing `(target, KeepReason)` table, next to the `client.ability.*` rows.
4. `uploader_post.rs`:
   - Add, below `CLIENT_PID_HEADER`, a doc-commented `pub(super) const CLIENT_SENT_MS_HEADER: &str = "X-Cimmeria-Client-Sent-Ms";` ("The DLL's wall clock when this POST was built, unix ms, on the same clock as each row's `ts_ms`. admin-api measures the clock skew from it.").
   - In `post_batch_within`, after `let body = encode_batch(batch)?;`, compute the stamp with `SystemTime::now().duration_since(UNIX_EPOCH)` as `i64` ms (0 on error, as `queue.rs` `now_ms` does) and add `.set(CLIENT_SENT_MS_HEADER, &sent_ms.to_string())` to the request chain. Each retry calls `post_batch_within` again, so a retry carries a fresh stamp.

**Tests.**

`camera/emit_tests.rs`, unit tests:
- `fields_carry_zoom_and_offsets`: a `CameraState` with distance 400, yaw 16384 and pitch -2048 gives `zoom == 400`, `yaw_offset_deg == 90` and `pitch_offset_deg == -11.25`.
- `zero_camera_pose_omits_position`: `cam_pose = None` gives `camera_pose_zero == true` and no `camera_x`.
- `camera_pose_is_in_server_units`: a client pos of `[100, 200, 300]` gives `camera_x == 2`, `camera_y == 3` and `camera_z == 1`.
- `no_camera_no_row`: `cam = None` gives an empty bag.
- `send_id_only_when_given`: `Some(77)` gives `send_id == 77`; `None` gives no `send_id` key.
- `pawn_fields_follow_s_cam`: with S-CAM A, a pawn gives `pawn_yaw_deg`. With S-CAM B this test asserts that no `pawn_*` key is ever present.
- `no_personal_names`: the bag has no `entity_name`, `player_name` or `account_name`.

The governor table row fails if the rule is removed.

`uploader_tests.rs`: add `upload_carries_the_send_stamp`. Copy the `tiny_http` setup of `uploader_ships_batch_to_local_server`, but keep `header_value(req.headers(), "X-Cimmeria-Client-Sent-Ms")` and the decoded body. Assert that the header parses as `i64`, that it is within 60 000 of the test's own `SystemTime` unix ms, and that it is `>=` every `ts_ms` in the decoded NDJSON (parse each line with `serde_json::Value` and read `ClientNative.ts_ms`, or the key the existing test reads). Removing the header fails the parse.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-16;`:

1. `cargo fmt -p cimmeria-client-telemetry --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-client-telemetry --target i686-pc-windows-msvc --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-client-telemetry --features lab-bridge --target i686-pc-windows-msvc --all-targets -- -D warnings`
4. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-client-telemetry --target i686-pc-windows-msvc`
5. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-client-telemetry --features lab-bridge --target i686-pc-windows-msvc`
6. `git diff --check`

**Ship:** with PC-15 and PC-16b, as one PR from `pc-16b`.

---

## PC-16b Send id and Mercury sequence for camera sends

- **Agent:** packet-coder.
- **Worktree:** `pc-16b`. **Branch:** `player-capture-bots/pc-16b-camera-seq-join`. **Crate:** `cimmeria-client-telemetry`.
- **Stacked on PC-16.** First command: `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-16b; git merge --ff-only player-capture-bots/pc-16-camera-at-interact`. If the fast-forward fails, return `blocked`.
- **Commit subject:** `feat(client-telemetry): join camera sends to their Mercury packet sequence`.
- **Files:**
  - `crates/client-telemetry/src/hooks/inline_hooks/ability/route.rs` (338 lines; add about 45)
  - `crates/client-telemetry/src/hooks/inline_hooks/net_out.rs`
  - `crates/client-telemetry/src/hooks/ability_trace/seq_join.rs` (266 lines)
  - `crates/client-telemetry/src/governor/tests/classify.rs`
  - `crates/client-telemetry/src/hooks/inline_hooks/ability/tests/mod.rs` (one `mod reach_probe;` line)
  - `crates/client-telemetry/src/hooks/inline_hooks/ability/tests/reach_probe.rs` (new)

**Why (D-PC19).** The DLL already joins each ability send to the Mercury packet sequence numbers that carried it (`seq_join.rs`, AB-C1): the router hook notes a `SentTag`, `Channel::send` tags the bundle, and `Nub::send` reports `client.ability.sent_seq` with the range. The server sees the same number as `packet_seq` on every cell method call. PC-26 stamps it on the pose row as `mercury_seq`. This packet puts camera sends through the same joiner, so a camera row joins its pose row by sequence number, not by time. No wire change.

**Contract excerpt.**
- `client.camera.at_interact` gains `send_id` (u32, unique per DLL process; the same counter as `client.ability.sent`).
- New target `client.camera.sent_seq`, level `info`, one per camera send whose bundle went out. Fields: `send_id`, `method`, `mercury_seq_first`, `mercury_seq_last`, `packets`, `seq_bits` (28), and `evicted_unmatched` on the first row of a bundle when non-zero. No `press_id` and no `ability_id`.
- A pose row's `mercury_seq` joins when `seq_in_range(mercury_seq, first, last)` (28-bit mask, may wrap).

**Change.**

1. `ability/route.rs`, after `impl Drop for RouteProbe`. `route.rs` is a child of `ability`, so it can call the private `super::next_send_id()` and `super::with_joiner(..)`. Add:
   ```rust
   /// A camera method's router call (player-capture-bots, D-PC19): a send id
   /// before the original runs, and a joiner tag if a `start*Message` ran.
   pub(in crate::hooks::inline_hooks) struct ReachProbe {
       method: &'static str,
       send_id: u32,
       prev_in_route: bool,
       prev_reached: bool,
   }
   impl ReachProbe {
       pub(in crate::hooks::inline_hooks) fn begin(method: &'static str) -> Self;
       pub(in crate::hooks::inline_hooks) fn send_id(&self) -> u32;
       /// After the original: notes `SentTag { send_id, press_id: None, method, ability_id: None }`
       /// when `REACHED_START` is set. Returns whether it noted one.
       pub(in crate::hooks::inline_hooks) fn finish(self) -> bool;
   }
   impl Drop for ReachProbe { /* restore IN_ROUTE and REACHED_START, as RouteProbe does */ }
   ```
   `begin` takes `next_send_id()`, then saves and sets the thread-locals exactly as `RouteProbe::begin` does (`IN_ROUTE` to true, `REACHED_START` to false). `finish` reads `REACHED_START` with `try_with(Cell::get).unwrap_or(false)`, and on true calls `with_joiner(|j| j.note_sent(tag))`. `interact` and `useItem` are outside the ability allowlist, so `RouteProbe::begin` returns `None` for them and the two probes never nest.
2. `net_out.rs`:
   - `observe` returns `Option<(&'static str, Option<i32>)>`: `Some(("interact", local_id))` or `Some(("useItem", local_id))` when `name_key` is one of them, else `None`. Every early `return;` becomes `return None;`. Remove the PC-16 `at_interact` call from `observe`.
   - `route_detour` becomes:
     ```rust
     let camera = guarded(|| observe(entity, method)).flatten();
     let reach = camera.map(|(m, _)| super::ability::route::ReachProbe::begin(m));
     #[cfg(all(target_os = "windows", target_arch = "x86"))]
     if let (Some((m, local)), Some(r)) = (camera, reach.as_ref()) {
         guarded(|| crate::hooks::camera::emit::at_interact(m, local, Some(r.send_id())));
     }
     let probe = super::ability::route::RouteProbe::begin(entity, method, args);
     original(entity, desc, method, args);
     if let Some(p) = probe {
         p.finish();
     }
     if let Some(r) = reach {
         r.finish();
     }
     ```
     The camera is read before the original runs, so it is the camera as the player clicked. The existing `route_detour_forwards_all_four_arguments` test passes an unreadable method, so `observe` returns `None` there; it must stay green.
3. `ability_trace/seq_join.rs`:
   - Add `pub(crate) const TARGET_CAMERA_SENT_SEQ: &str = "client.camera.sent_seq";` and `pub(crate) fn is_camera_method(m: &str) -> bool { matches!(m, "interact" | "useItem") }`.
   - In `sent_seq_outs`, pick `target` per tag: `TARGET_CAMERA_SENT_SEQ` when `is_camera_method(t.method)`, else `TARGET_SENT_SEQ`. For a camera tag, leave out `press_id` and `ability_id`. The key is `format!("{target}:{}", t.method)`.
   - Add a unit test in its `mod tests`: `camera_tags_get_their_own_target`. One ability tag and one `interact` tag in one bundle give one `client.ability.sent_seq` row and one `client.camera.sent_seq` row, both with the same `mercury_seq_first` and `mercury_seq_last`, and the camera row has no `press_id` key.
4. `governor/tests/classify.rs`: add `("client.camera.sent_seq", KeepReason::InteractionCamera)` to the table. PC-16's prefix rule already keeps it.

**Tests.** `ability/tests/reach_probe.rs`, with `use super::*;` as `router_probe.rs` has. Each test takes `let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());` first, then drains the shared joiner: `with_joiner(|j| { j.tag_bundle(0xDEAD); j.take_bundle(0xDEAD); });`. Read what a probe noted by tagging a fake bundle, as `router_probe.rs` does: `with_joiner(|j| { assert!(j.tag_bundle(0x1000)); j.take_bundle(0x1000).unwrap() })`. The router state helpers are `super::super::route::test_access::{enter_route, mark_reached, leave_route}`.

- `reached_probe_notes_a_camera_tag`: `ReachProbe::begin("interact")`, `mark_reached()`, then `finish()` returns true, and the bundle holds one tag with that probe's `send_id`, `method == "interact"`, `press_id == None` and `ability_id == None`.
- `unreached_probe_notes_nothing`: `begin("useItem")`, then `finish()` returns false, and `tag_bundle(0x1000)` returns false.
- `probe_restores_the_thread_locals`: `enter_route()`, `mark_reached()`, then begin and drop a probe without finishing it; `leave_route()` still returns true.
- `send_ids_are_unique`: two probes get different `send_id` values.

**Lane checks.** Prefix each with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\pc-16b;`:

1. `cargo fmt -p cimmeria-client-telemetry --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-client-telemetry --target i686-pc-windows-msvc --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-client-telemetry --features lab-bridge --target i686-pc-windows-msvc --all-targets -- -D warnings`
4. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-client-telemetry --target i686-pc-windows-msvc`
5. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-client-telemetry --features lab-bridge --target i686-pc-windows-msvc`
6. `git diff --check`

**Caveat for the report.** The `Channel::send` and `Nub::send` hooks are not yet proven live (AB-C1 ships them; no live run has graded `client.ability.sent_seq`). PC-19 proves them. If they fail live, the join falls back to the clock (PC-27), and nothing here breaks.

**Ship:** one PR covering PC-15, PC-16 and PC-16b, from `pc-16b`. The PR body says that the new DLL reaches players only through a launcher release, because the DLL hash is compiled into the launcher, and that ops runs that release after the merge (PC-07b).
