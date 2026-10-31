# lab-autopilot wave 5: watch, detectors, walker

> **Type:** Reference
> **Audience:** Packet-coders and packet-reviewers of wave 5
> **Last updated:** 2026-10-31
> **Companions:** [work packets](work-packets.md) (rules R1-R10, checks), [contract](contract.md), [ledger](README.md), [wave 4](packets-wave4.md)

Wave 5 adds the things that watch the client while a run goes on, and makes the walker land where it was asked to:

- `client_watch` pulls the per-frame samples of the DLL watch (C6, AX-24) into the daemon (C15);
- `lab_detectors_read` runs the C12 detectors over those samples, the heartbeat ring and the `lab.call` log;
- screenshots say which client frames they span, and `client_batch` uses the DLL's same-frame `batch` (C7, AX-25) when it can;
- the test `Sim` learns input latency and coasting, and `client_move_to` learns to stop before it coasts past the goal;
- `client_trace_line` and the ledge guard (C14). The 2026-10-31 live pass met their gate; they no longer wait on AX-01.

Every packet carries R1-R10 of [work-packets.md § Rules](work-packets.md#rules-every-packet-carries). Daemon paths are under `crates/lab/src/`; DLL paths under `crates/client-telemetry/src/`.

**Dispatch:**

1. AX-50 first, from `lax-w5` (created by `ops` from main once the wave 4 PR merged).
2. When AX-50 has merged into `lax-w5`, dispatch AX-51, AX-52 and AX-53 at once, each at that head.
3. AX-54 waits until AX-53 has merged (it tests against the coasting `Sim`).
4. AX-55 touches only the DLL. It may branch from main as soon as the wave 2 PR has merged (it routes a method in `bridge/dispatch.rs`, which wave 2 edits), in parallel with everything above.
5. AX-56 waits until AX-54 and AX-55 have both merged into `lax-w5` (it edits the movement loop AX-54 edits, and needs AX-55's method to be worth shipping).

**Merge order:** AX-50, AX-51, AX-52, AX-53, AX-54, AX-55, AX-56. One PR from `lab-autopilot/wave5`. (The old `wave5b` fallback is gone: nothing here waits on AX-01 except AX-51's `actor_growth` detector, which stays off until AX-01 confirms the actor count.)

**Shared facts every packet here needs:**

- `Supervisor` is per instance; a `LabServer` tool reaches it as `self.supervisor`. Its private fields (`state`, `bridge`) are visible to child modules of `supervisor/`, so a new `supervisor/<x>.rs` can write `impl Supervisor { ... }` against them.
- `supervisor/mod.rs` is about 651 lines after AX-23. Wave 5 adds at most 4 lines (AX-50), 2 (AX-52) and 3 (AX-54): keep it under 670.
- Bridge calls from tools go through `Supervisor::bridge_call(method, params)` (lease-checked, journaled). `self.bridge.heartbeat()` (`client.rs`, `-> anyhow::Result<u64>`) is a read with no lease check. `self.bridge.call_meta(method, params)` (AX-22) returns `(result, Option<BridgeMeta>)`.
- Test fakes: `supervisor/events/fake_bridge.rs` `supervisor(responder)` and `supervisor_with_meta(responder)` (AX-22) answer bridge methods from a closure; `supervisor/world/sim.rs` `Sim` is the world fake.
- A bridge error for an unknown method contains `-32601`. Every "old DLL" branch in this wave tests for that text.

### AX-50 client_watch and the watch store

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-50-watch` · **Worktree:** `lax-50` (at `lax-w5`)
- **Commit:** `feat(lab): client_watch pulls per-frame samples into the daemon (AX-50)`
- **Rules:** R1-R4, R6-R9. Contract: [C15](contract.md#c15-client_watch), C6 (sample fields), C4 (`meta`).

Files and changes:

1. New `supervisor/watch_store.rs`:

```rust
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct WatchSample {
    pub i: u64, pub f: u64, pub ms: u64,
    #[serde(default)] pub wall_ms: i64,           // set by the store, never by the DLL
    #[serde(default)] pub t: Option<f64>,
    #[serde(default)] pub pos: Option<[f32; 3]>,
    #[serde(default)] pub rot: Option<[i32; 3]>,
    #[serde(default)] pub hover: Option<u32>,
    #[serde(default)] pub mode: Option<u32>,
    #[serde(default)] pub buttons: Option<u32>,
    #[serde(default)] pub actors: Option<u32>,
}
pub const STORE_CAP: usize = 4096;
#[derive(Debug, Default)]
pub struct WatchStore { samples: std::collections::VecDeque<WatchSample>, pub running: bool, last_i: Option<u64> }
impl WatchStore {
    /// Keep the samples of one `watch_read`. `read_wall_ms` is the daemon wall
    /// clock when the response arrived; `meta_mono_ms` its `meta.mono_ms`.
    pub fn ingest(&mut self, samples: Vec<WatchSample>, read_wall_ms: i64, meta_mono_ms: Option<u64>) -> usize;
    pub fn since(&self, wall_ms: i64) -> Vec<WatchSample>;
    pub fn clear(&mut self);
}
```

   - `ingest` sets `wall_ms = read_wall_ms - (meta_mono_ms - s.ms) as i64` for each sample. Without `meta`, `wall_ms = read_wall_ms` for all of them (the read is the best bound there is).
   - It skips a sample whose `i` is not above `last_i` (two readers, or a repeated `since`), so the store never holds a sample twice. It returns how many it kept.
   - Over `STORE_CAP` it drops the oldest.
   - `since` returns the samples with `wall_ms >= wall_ms`, oldest first.
2. The same file, `impl Supervisor`:
   - `pub async fn watch(&self, action: &str, params: Value) -> Result<Value, String>`.
   - `start`: `bridge_call("watch_start", params)`; on success `clear()` the store and set `running = true`.
   - `stop`: `bridge_call("watch_stop", {})`; set `running = false`. Keep the samples (the detectors still read them).
   - `read`:
     1. `crate::lease::permit::ensure("bridge watch_read")?`.
     2. Record the call in the journal exactly as `bridge_call_unguarded` does (`st.journal.record` before, `st.journal.complete` after).
     3. `self.bridge.call_meta("watch_read", params)`.
     4. Parse `result.samples` as `Vec<WatchSample>`, `ingest` them with `now_ms()` (the wall clock helper `supervisor/mod.rs` already uses) and `meta.map(|m| m.mono_ms)`.
     5. Copy `result.running` into `store.running`.
     6. Return the DLL result unchanged plus `"pulled": <kept>`.
   - Any other action is an error `action must be start, read or stop`.
   - An error containing `-32601` from any action becomes the error text `watch needs a lab DLL with watch_start (AX-24)`.
   - `pub async fn watch_samples_since(&self, wall_ms: i64) -> (bool, Vec<WatchSample>)` returns `running` and `since(wall_ms)`. AX-51 reads it.
3. `supervisor/mod.rs` (at most 4 lines): `pub mod watch_store;`, a field `watch: tokio::sync::Mutex<watch_store::WatchStore>` on `Supervisor`, and its `Default` in the constructor.
4. New `server/watch.rs`, with its own router, the import pattern of `server/world.rs` l.1-26:
   - `#[tool_router(router = watch_router, vis = "pub(super)")] impl LabServer { ... }` with one tool `client_watch`.
   - Args `WatchArgs { action: String, period_frames: Option<u32>, cap: Option<u32>, since: Option<u64>, max: Option<u32> }`, plus the `instance` and `lease_id` fields every LEASED tool's args carry (copy them from `BatchArgs` in `server/composite.rs` or wherever `BatchArgs` is defined).
   - It builds `params` from the set fields only (`period_frames`, `cap` for start; `since`, `max` for read) and calls `self.supervisor.watch(&a.action, params)`, mapping `Err(e)` to `json_result(json!({"error": e}))` with `is_error` the way `client_batch`'s neighbours report tool errors (grep `is_error` in `server/composite.rs` or `server/world.rs` and copy that shape).
   - Description: `Start, read or stop the lab DLL's per-frame watch (pose, rotation, hover, controller mode, buttons, game time). read returns the samples since an index and keeps them for lab_detectors_read. Returns the DLL result plus pulled; no evidence (a read).`
5. `server/mod.rs`: `mod watch;` with the other `mod` lines, and `+ Self::watch_router()` at the end of the router sum (after `+ Self::input_router()` from AX-15). Two lines.
6. `lease/policy.rs`: `client_watch` in `LEASED`. `proof/claims.rs`: `client_watch` in `NO_CLAIM`.
7. Tests: `supervisor/watch_store_tests.rs` (wired with `#[cfg(test)] #[path]` from `watch_store.rs`).

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `ingest_maps_mono_to_wall` | samples `ms` 1000 and 1500, `meta_mono_ms` 2000, `read_wall_ms` 50_000: `wall_ms` 49_000 and 49_500 | C15 mapping |
| `ingest_without_meta_uses_read_time` | `meta_mono_ms` `None`: both `wall_ms` 50_000 | C15 |
| `ingest_skips_seen_indexes` | ingest `i` 0..=4, then 3..=7: 8 kept in total, second call returns 3 | no duplicates |
| `store_caps_at_4096` | ingest 5000 samples: 4096 kept, the oldest kept `i` is 904 | C15 ring |
| `read_pulls_and_reports_pulled` | `fake_bridge::supervisor_with_meta` answers `watch_read` with 3 samples and `meta.mono_ms`: result has `samples` (3), `pulled: 3`; `watch_samples_since(0)` returns 3 | step 2 |
| `start_clears_the_store` | read 3, then `start`: `watch_samples_since(0)` is empty and `running` true | step 2 |
| `old_dll_gets_the_c15_error` | the fake errors with text containing `-32601`: the error is exactly `watch needs a lab DLL with watch_start (AX-24)` | C15 |
| `bad_action_is_refused` | `action: "pause"` errors `action must be start, read or stop` with no bridge call recorded | step 2 |
| `client_watch_is_classified` | `claim("client_watch") == Claim::NoClaim`; `every_routed_tool_is_classified` passes | R9 |

Checks: L1, L2. Report the `supervisor/mod.rs` and `server/mod.rs` line counts.

### AX-51 Detectors and lab_detectors_read

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-51-detectors` · **Worktree:** `lax-51` (at `lax-w5` after AX-50)
- **Commit:** `feat(lab): detectors for hitches, teleports, falls, stuck input and repeated errors (AX-51)`
- **Rules:** R1-R4, R6-R9. Contract: [C12](contract.md#c12-detectors). Decision D-AX6 (`out_of_world` threshold, provisional).

C12, copied here so the packet carries it. `lab_detectors_read { instance?, since_ms? }`, OPEN. Result `{ "watch": bool, "window_ms": u64, "fired": [ { "name", "at_ms", "frame"?, "detail" } ] }`.

| Name | Fires when |
|---|---|
| `hitch` | watch: `(ms[i] - ms[i-1]) > 250 * max(1, f[i] - f[i-1])`; heartbeat (no watch): two consecutive heartbeat observations at least 250 ms apart with the same `tick_count` |
| `teleport` | the pawn moved more than 30 m within 500 ms |
| `long_fall` | z decreasing for more than 3 s |
| `out_of_world` | z below -100000 UE3 units (D-AX6) |
| `nan` | any NaN or infinite position or rotation |
| `stuck_input` | a hold (C5) present for 3 s with the pawn moving less than 0.2 m |
| `actor_growth` | actor count up more than 50% within 60 s (only when samples carry `actors`) |
| `log_error` | a `lab.call` with outcome `error` for the same tool three times in 60 s |

Without samples in the window only `hitch` (heartbeat mode) and `log_error` run, and `watch` is `false`. `since_ms` defaults to 120 000; the window is `[now - since_ms, now]` in daemon wall ms; `window_ms` is `since_ms`.

Files and changes:

1. New `supervisor/detect/mod.rs`:
   - `#[derive(Debug, Clone, PartialEq, serde::Serialize)] pub struct Fired { pub name: &'static str, pub at_ms: i64, #[serde(skip_serializing_if = "Option::is_none")] pub frame: Option<u64>, pub detail: serde_json::Value }`.
   - `pub struct Inputs<'a> { pub samples: &'a [WatchSample], pub heartbeats: &'a [HeartbeatSample], pub holds: &'a [(String, i64)], pub calls: &'a [CallRow], pub now_ms: i64 }` (holds are key name and hold start, daemon wall ms). `HeartbeatSample` is `crate::timeline::client_events::HeartbeatSample { tick_count: u64, observed_ms: i64 }`, what `Supervisor::heartbeat_samples()` returns.
   - `pub fn run_all(i: &Inputs) -> Vec<Fired>`: runs every detector below in table order and sorts by `at_ms`. With `samples` empty it runs only `timing::hitch_heartbeat` and `calls::log_error`.
   - `pub mod motion; pub mod timing; pub mod input; pub mod load; pub mod calls;`.
2. New `supervisor/detect/motion.rs`: `teleport`, `long_fall`, `out_of_world`, `nan`, each `fn(&[WatchSample]) -> Vec<Fired>`.
   - Distances are client UE3 units; 30 m is 3000 UU (`geometry::UU_PER_METRE`). `teleport` compares each sample with the earliest sample at most 500 ms (`wall_ms`) before it, horizontal and vertical together (`distance_m`), and fires once per jump with `detail {"from", "to", "metres"}`.
   - `long_fall` fires once per run of strictly decreasing z longer than 3000 ms, at the run's end, `detail {"seconds", "drop_uu"}`.
   - `out_of_world`: `pub const OUT_OF_WORLD_Z: f32 = -100_000.0; // D-AX6, provisional`; fires on the first sample below it in each run.
   - `nan`: the first sample with a non-finite `pos` component (rotations are integers; `rot` cannot be NaN, so only `pos` is checked; say so in a comment).
   - Samples without `pos` are skipped by every motion detector.
3. New `supervisor/detect/timing.rs`:
   - `hitch_watch(&[WatchSample])`: the C12 rule on consecutive samples, `at_ms` the later sample's `wall_ms`, `frame` its `f`, `detail {"gap_ms", "frames"}`.
   - `hitch_heartbeat(&[HeartbeatSample])`: consecutive observations with the same `tick_count` and `observed_ms` at least 250 apart; `detail {"gap_ms", "tick"}`, `frame` the tick.
4. New `supervisor/detect/input.rs`: `stuck_input(samples, holds, now_ms)`: for each hold older than 3000 ms, take the samples since the hold started; fire when their first and last `pos` are less than 20 UU apart (horizontal) and they span at least 3000 ms. `detail {"key", "held_ms", "moved_m"}`.
5. New `supervisor/detect/load.rs`: `actor_growth` (any two samples with `actors` at most 60 000 ms apart where the later is above 1.5 times the earlier; once per window) and `log_error` lives in `calls.rs`.
6. New `supervisor/detect/calls.rs`:
   - `#[derive(Debug, Clone, PartialEq)] pub struct CallRow { pub ms: i64, pub instance: String, pub tool: String, pub outcome: &'static str }`.
   - A process-wide `static CALLS: std::sync::Mutex<VecDeque<CallRow>>` of at most 256 rows (oldest dropped).
   - `pub fn record(row: CallRow)` and `pub fn rows_for(instance: &str) -> Vec<CallRow>`.
   - `log_error(calls, now_ms)`: per tool, three `error` rows within 60 000 ms fires once at the third, `detail {"tool", "count"}`.
7. `server/handler.rs` `log_call`: one line after `outcome` is known: `crate::supervisor::detect::calls::record(CallRow { ms: <wall now ms>, instance: instance.into(), tool: tool.into(), outcome })`. Nothing else in the handler changes.
8. `supervisor/hold_book.rs` (AX-23): `Held` gains `since_ms: u64`. `pressed` sets it to `now` for a new key and keeps it for a key already held. Add `pub fn holds(&self) -> Vec<(String, u64)>` (name and `since_ms`). Fix the constructors the compiler points at; no other change. Check what clock `supervisor/held_keys.rs` passes as `now`: it must be daemon wall ms (`crate::supervisor::now_ms() as u64`, the clock `WatchStore` uses). If it passes another clock, change that one argument in `held_keys.rs` and say so in the report. Besides that argument, `held_keys.rs` gets only `hold_starts` (step 10). The tool casts `since_ms as i64` into `Inputs::holds`.
9. `supervisor/mod.rs`: `pub mod detect;` (one line; count it against the 670 target).
10. `server/watch.rs` (AX-50): add the tool `lab_detectors_read`:
    - Args `DetectArgs { since_ms: Option<u64> }` plus the `instance` field (no `lease_id`: OPEN).
    - It reads `watch_samples_since(now - since)`, `heartbeat_samples()` (keeping those with `observed_ms` in the window), the hold starts through a new `pub(crate) async fn hold_starts(&self) -> Vec<(String, u64)> { self.holds.lock().await.holds() }` added to the `impl Supervisor` block in `supervisor/held_keys.rs` (the `holds` field is private to `supervisor/`), and `calls::rows_for(self.supervisor.label())` (`Supervisor::label()`, `supervisor/mod.rs` l.341, is the instance label `log_call` records), calls `run_all`, and returns `{watch: !samples.is_empty(), window_ms, fired}`.
    - It makes no bridge call. Description: `Run the lab detectors (hitch, teleport, long_fall, out_of_world, nan, stuck_input, actor_growth, log_error) over what the daemon already holds: client_watch samples, the heartbeat history and recent tool calls. No client call; watch is false when no samples were pulled, and then only hitch and log_error run.`
11. `lease/policy.rs`: `lab_detectors_read` in `OPEN`. Do **not** list it in `NO_CLAIM` (AX-11's test forbids an OPEN tool there).
12. Tests: one `#[cfg(test)] mod tests` per detector file, plus `supervisor/detect/run_tests.rs` for `run_all` and the tool.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `hitch_watch_period_one` | `ms` 0, 16, 400 at `f` 0, 1, 2: one `hitch` at the third sample, `gap_ms` 384 | C12 rule |
| `hitch_watch_scales_with_frames` | `ms` 0, 400 at `f` 0, 2 (period 2): nothing fires; at `f` 0, 1: fires | `max(1, df)` |
| `hitch_heartbeat_same_tick` | heartbeats `(5, 0)`, `(5, 300)`: fires; `(5, 0)`, `(6, 300)`: nothing | C12 heartbeat mode |
| `teleport_over_30m_in_500ms` | two samples 3100 UU apart, 200 ms: fires; 2900 UU: nothing; 3100 UU over 900 ms: nothing | C12 |
| `long_fall_after_three_seconds` | z falling for 3500 ms: one `long_fall`; for 2500 ms: nothing | C12 |
| `out_of_world_threshold` | z -100001 fires; -99999 does not | D-AX6 |
| `nan_position` | `pos` `[NaN, 0, 0]` fires once | C12 |
| `stuck_input_needs_three_seconds_of_hold` | hold `W` since 0, samples 0..4000 ms moving 10 UU: fires; moving 500 UU: nothing; hold since 2000: nothing | C12, HoldBook start |
| `pressed_again_keeps_since` | `pressed(W, 0)`, `pressed(W, 5000)`: `holds()` gives since 0 | step 8 |
| `log_error_three_in_a_minute` | three `error` rows for one tool within 50 s: fires once; two: nothing; three over 70 s: nothing | C12 |
| `calls_ring_caps_at_256` | 300 `record`s: `rows_for` returns at most 256, newest kept | step 6 |
| `no_watch_runs_two_detectors` | `run_all` with empty samples, a stalled heartbeat pair and 3 errors: fired names are exactly `hitch` and `log_error` | C12 |
| `detectors_tool_is_open` | `lease::policy::OPEN` contains `lab_detectors_read`; `claims::NO_CLAIM` does not | R9 |

Checks: L1, L2. Report the line count of every file under `supervisor/detect/` and of `hold_book.rs`.

### AX-52 Frame-bracketed screenshots and the DLL batch path

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-52-frames` · **Worktree:** `lax-52` (at `lax-w5` after AX-50)
- **Commit:** `feat(lab): screenshots name their frames and client_batch reads in one frame (AX-52)`
- **Rules:** R1-R4, R6-R8. Contract: C4 (frame), C7 (allowed methods). Decision D-AX2 (frames stay supervisor-side: PrintWindow, bracketed by heartbeats).

Files and changes:

1. New `supervisor/capture_frames.rs`:
   - Pure: `pub fn bracket(before: Option<u64>, after: Option<u64>) -> Option<[u64; 2]>`, `Some` only when both are present and `after >= before`.
   - Pure: `pub fn frames_text(b: Option<[u64; 2]>) -> String`: `", frames 81230..81233"`, or `""` when `None`.
   - `impl Supervisor { pub async fn capture_bracketed(&self) -> Result<(screenshot::CapturedImage, Option<[u64; 2]>), String> }`: `self.bridge.heartbeat().await.ok()`, then `self.capture().await?`, then the heartbeat again, then `bracket`. A failed heartbeat never fails the capture.
   - Declare `pub mod capture_frames;` in `supervisor/mod.rs` (one line).
2. `server/mod.rs` `lab_screenshot`: use `capture_bracketed`, then `screenshot::encode_png` and `png_to_base64` as `Supervisor::screenshot` does, and the text `format!("client window {w}x{h}{}", frames_text(b))`. At most 8 lines changed; if `server/mod.rs` would pass 650, move `lab_screenshot` whole into `server/client_state.rs` instead (it has room) and say so.
3. `server/client_state.rs`:
   - `lab_screenshot_region`: `capture_bracketed`, text `format!("region {w}x{h} at ({}, {}){}", a.x, a.y, frames_text(b))`.
   - `lab_pixel_probe`: `capture_bracketed`; the JSON gains `"frames": [before, after]` when `Some`.
   - Update the three descriptions: append ` The text (or JSON frames) names the client frames the capture spans, from heartbeats before and after.`
4. New `supervisor/composite/batch_frame.rs`:
   - `pub fn dll_batchable(steps: &[Step]) -> bool`: true only when `1 <= steps.len() <= 16` and every step is `Op::MemRead` or `Op::Lua` whose fields contain no `$` reference (test with `resolve(v, &Map::new())`: a reference errors on an empty map; a literal returns unchanged).
   - `pub fn dll_calls(steps: &[Step]) -> Value`: the C7 `calls` array: `{"method": "mem_read", "params": {"addr": addr_text(addr)?, "len": len}}` (the same `len` rule as `batch_step`) or `{"method": "lua_eval", "params": {"chunk": chunk}}`.
   - `pub fn decode_results(steps: &[Step], results: &[Value]) -> Map<String, Value>`: per step, an `{"ok": v}` entry decodes like `batch_step` does (`decode_read(v["hex"], read_as)` for a mem read; `flows::ui_state::lua_results_of(v)` and the 0/1/many mapping for Lua, factored into `pub fn lua_value(r: Vec<String>) -> Value` here and called from `batch_step` too); an `{"error": e}` entry becomes `{"error": e.message}`; a step whose value is `Value::Null` is not inserted (as `batch` does).
   - `impl Supervisor { pub(super) async fn batch_in_frame(&self, steps: &[Step]) -> Result<Option<BatchOutcome>, String> }`: `bridge_call("batch", {"calls": dll_calls(steps)})`. An error containing `-32601` sets the flag below and returns `Ok(None)`; another error is `Err`; success returns `BatchOutcome { values: decode_results(..), stopped_at: None, ms, frame: result["frame"].as_u64() }`.
5. `supervisor/composite/batch.rs` (547 lines; at most 15 added):
   - `BatchOutcome` gains `pub frame: Option<u64>` (`None` everywhere it was built before). `to_json` adds `"frame"` when `Some`.
   - `batch()` first: when `dll_batchable(steps)` and the flag is not set, `if let Ok(Some(o)) = self.batch_in_frame(steps).await { return o; }`; otherwise the existing loop runs unchanged. `stop_on_error` does not apply to the DLL path (reads have no side effects; the per-step errors are all reported).
   - `batch_step`'s Lua arm calls `batch_frame::lua_value(r)`.
   - Declare `mod batch_frame;` in `composite/mod.rs`.
6. `supervisor/mod.rs`: a field `dll_batch_unsupported: std::sync::atomic::AtomicBool` and its default (2 lines). The flag is per connection in spirit; it resets only when the supervisor is rebuilt, which is acceptable (an old DLL stays old until relaunch, and the fallback is the current behaviour).
7. `server/composite.rs` `client_batch` description: append ` When every step is a literal mem_read or lua, the lab DLL runs them all in one client frame and the result carries that frame.`
8. Tests: `supervisor/capture_frames.rs` `#[cfg(test)] mod tests`, and `supervisor/composite/batch_frame_tests.rs`. `batch_tests.rs` keeps passing unchanged; add the new ones in the new file (`batch_tests.rs` is not to grow).

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `bracket_needs_both_in_order` | `(Some(5), Some(7))` gives `[5, 7]`; `(None, Some(7))`, `(Some(7), Some(5))` give `None` | step 1 |
| `frames_text_format` | `Some([81230, 81233])` gives `", frames 81230..81233"`; `None` gives `""` | step 1 |
| `literal_reads_are_batchable` | mem_read + lua literals: true; with a `$a` ref: false; with a `wait` or `call_native`: false; 17 reads: false | step 4 |
| `batch_goes_to_the_dll_once` | `fake_bridge::supervisor` answering `batch` with `{results: [{ok: {hex: "2a000000"}}, {ok: {ok: true, results: ["7"]}}], frame: 900}`: `batch()` returns values `{a: 42, b: "7"}` (as today's decoders give them), `frame: Some(900)`, and the fake saw exactly one call, `batch` | step 5 |
| `old_dll_falls_back_and_remembers` | the fake errors `-32601` on `batch` and answers `mem_read`: the outcome comes from the per-step path, `frame` `None`; a second `batch()` sends no `batch` call | step 4, 6 |
| `refs_use_the_step_path` | a `$a` reference: the fake sees `mem_read` calls, no `batch` | step 5 |
| `per_step_error_is_reported` | the DLL returns `[{error: {code: -32000, message: "bad addr"}}, {ok: ...}]`: values `{a: {"error": "bad addr"}, b: ...}` | step 4 |

Checks: L1, L2. Report the `batch.rs`, `server/mod.rs` and `supervisor/mod.rs` line counts.

### AX-53 The Sim learns latency and coasting

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-53-lagsim` · **Worktree:** `lax-53` (at `lax-w5` after AX-50)
- **Commit:** `test(lab): the movement Sim models input latency and coasting (AX-53)`
- **Rules:** R1-R4, R6. Test infrastructure only; no production code changes.

`Sim` lives in `supervisor/world/sim.rs` and, after AX-13, `sim_model.rs`. Grep for `fn advance` and `fn key` (the `WorldIo::key` impl for `Sim`) to find where they are; `advance(ms)` steps 10 ms at a time and moves the pawn while `self.held("W")` (or `S`) at `speed_uu_s` (500).

Files and changes:

1. New `supervisor/world/sim_lag.rs`, pure, `#[cfg(test)]`-only like `sim.rs`:

```rust
#[derive(Debug, Clone, Default)]
pub(crate) struct Lag {
    pub input_latency_ms: u64,                 // 0: keys apply at once (today)
    pub coast_s: f64,                          // 0: the pawn stops when W goes up (today)
    pending: Vec<(u64, String, bool)>,         // (due_ms, key, down), in arrival order
    coast_until_ms: Option<u64>,
}
impl Lag {
    /// A key change at `now`: Some((key, down)) to apply now, or None when queued.
    pub fn key(&mut self, now: u64, key: &str, down: bool) -> Option<(String, bool)>;
    /// Queued changes due at `now`, in order.
    pub fn due(&mut self, now: u64) -> Vec<(String, bool)>;
    /// Forward was released at `now`: start a coast of `coast_s`.
    pub fn released_forward(&mut self, now: u64);
    /// True while the pawn should keep walking forward with W up.
    pub fn coasting(&self, now: u64) -> bool;
    /// Forward was pressed again: stop any coast.
    pub fn pressed_forward(&mut self);
}
```

2. `Sim` gains `pub lag: sim_lag::Lag` (default: both 0, identical to today).
   - `key()`: route the change through `lag.key(now, key, down)`; apply what it returns exactly as today (`keys.insert`, the Space/`jump_clears_wall` rule). About 3 lines.
   - `advance()`: at each 10 ms step, first apply `lag.due(now)` the same way; when `W` goes from held to not held call `lag.released_forward(now)` and when it is pressed call `lag.pressed_forward()`; move forward when `held("W") || lag.coasting(now)`. About 4 lines.
   - Declare `mod sim_lag;` next to `mod sim;` in `world/mod.rs` (under the same `#[cfg(test)]`).
   - Keep `sim.rs` and `sim_model.rs` under their AX-13 sizes plus 10 lines.
3. Tests in `supervisor/world/sim_lag_tests.rs`: the physics only. No controller tests here (AX-54 owns them).

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `default_lag_changes_nothing` | the existing `move_to_tests.rs` suite passes unchanged (run it; no new assertion needed) and a `Sim` with default `lag` moves 50 UU in 100 ms of W, 0 after W up | defaults |
| `latency_delays_the_press` | `input_latency_ms` 150: after W down, `advance(100)` moves 0; `advance(100)` more moves 250 UU (50 ms late start) | step 2 |
| `latency_delays_the_release_too` | `input_latency_ms` 150: W down, `advance(500)`, W up, `advance(100)`: still moving | step 2 |
| `coast_after_release` | `coast_s` 0.3: W held 1 s, W up, `advance(1000)`: moved 150 UU after the up, not more | step 2 |
| `pressing_again_ends_the_coast` | `coast_s` 0.3: W up, `advance(100)`, W down, W up, `advance(1000)`: the second coast is 150 UU from the second up | step 2 |

Checks: L1, L2. Report the line counts of `sim.rs`, `sim_model.rs` and `sim_lag.rs`.

### AX-54 The walker stops before it coasts past

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-54-coast` · **Worktree:** `lax-54` (at `lax-w5` after AX-53)
- **Commit:** `feat(lab): move_to releases early by the learned coast and checks where it stopped (AX-54)`
- **Rules:** R1-R4, R6, R8. No contract section: this is internal to `client_move_to`; its JSON gains only `legs[].coast` (step 4).

Why: the pawn keeps moving after W goes up, so the walker's `Arrived` at the release point is a claim about a place the pawn then leaves. The fix releases early by a learned distance, then waits for the pawn to settle and only calls it arrived if it settled inside `arrival_m`.

Files and changes:

1. New `supervisor/world/coast.rs`:

```rust
pub const COAST_PRIOR_S: f64 = 0.12;   // replaced by the AX-02 median when that lands (one-line change)
pub const COAST_ALPHA: f64 = 0.3;
pub const COAST_MAX_S: f64 = 0.6;
pub const MIN_LEARN_SPEED_MPS: f64 = 0.5;
pub const MIN_HOLDOFF_MS: u64 = 300;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoastModel { pub coast_s: f64, pub samples: u32 }
impl Default for CoastModel { /* coast_s: COAST_PRIOR_S, samples: 0 */ }
impl CoastModel {
    pub fn lead_m(&self, speed_mps: f64) -> f64;            // speed * coast_s, 0 for a negative speed
    pub fn holdoff_ms(&self) -> u64;                        // max(MIN_HOLDOFF_MS, ceil(coast_s * 1000) + 100)
    /// The pawn moved `coasted_m` after a release at `speed_mps`.
    pub fn learn(&mut self, speed_mps: f64, coasted_m: f64);
    pub fn to_json(&self) -> Value;                         // {"coast_s", "samples"}
}

/// Horizontal speed from consecutive poses.
#[derive(Debug, Clone, Default)]
pub struct SpeedEst { last: Option<(Vec3, u64)>, pub mps: f64 }
impl SpeedEst { pub fn observe(&mut self, pos: Vec3, now_ms: u64); }  // ignores dt == 0; mps = horizontal_m / dt
```

   - `learn`: ignore it when `speed_mps < MIN_LEARN_SPEED_MPS`. The observed coast is `(coasted_m / speed_mps).clamp(0.0, COAST_MAX_S)`. The **first** sample replaces the prior; later ones blend with `COAST_ALPHA` (`coast_s += alpha * (obs - coast_s)`). `samples += 1`.
2. `supervisor/world/steer.rs` `MoveController` (at most 20 lines added):
   - Fields `lead_m: f64` (default 0) and `holdoff: Option<(u64, u64)>` (`(until_ms, holdoff_ms)`), plus `pub fn set_lead(&mut self, lead_m: f64, holdoff_ms: u64)` and `pub fn coasting(&self) -> bool`.
   - In `step`, before the existing `dist <= arrival_m` check:
     1. While `holdoff` is `Some((until, _))` and `now_ms < until`: return `out` with `forward: false`, `turn: Off` and no outcome (set `self.last = None`).
     2. When the hold-off has ended: clear it, set `lead_m = 0.0` (a second approach on this leg walks to `arrival_m` plainly, which ends any oscillation), and fall through to the existing checks (so `dist <= arrival_m` gives `Arrived` with the settled distance).
     3. After the existing arrival check: when `lead_m > 0.0 && dist <= arrival_m + lead_m`, set `holdoff = Some((now_ms + holdoff_ms, holdoff_ms))`, `self.last = None`, and return `out` with `forward: false`, no outcome.
   - With `lead_m` 0 (the default), the controller behaves exactly as today. The steer tests must pass unchanged.
3. `WorldIo` (`supervisor/world/io.rs`; at most 8 lines): two default methods, `fn coast_model(&self) -> CoastModel { CoastModel::default() }` and `fn store_coast_model(&mut self, _m: CoastModel) {}`. `LiveWorld` keeps it on the supervisor: a field `coast: std::sync::Mutex<CoastModel>` on `Supervisor` (`supervisor/mod.rs`, 2 lines plus the default), read and written by these two methods. `Sim` gets `pub coast_model: Option<CoastModel>`: `coast_model()` returns it or the default; `store_coast_model` sets it.
4. `supervisor/world/movement.rs` (at most 20 lines added; the glue goes in `coast.rs` as `pub struct CoastGlue { model, speed: SpeedEst, release: Option<(Vec3, f64)> }` with `on_pose`, `on_released`, `on_settled` and `leg_json`):
   - Before the leg loop: `let mut glue = CoastGlue::new(io.coast_model());`.
   - On the **final** leg only (waypoint legs keep walking through), every tick: `glue.on_pose(pose.pos, now)` and `ctl.set_lead(glue.lead_m(), glue.holdoff_ms())`.
   - When the controller enters its hold-off (`ctl.coasting()` turns true), `glue.on_released(pose.pos)` records the release point and speed.
   - When the hold-off ends (the first step after it), `glue.on_settled(pose.pos)` calls `learn(speed, horizontal_m(release, settled))`.
   - After the loop: `io.store_coast_model(glue.model)`; the final leg's report gains `"coast": {"coast_s", "samples", "lead_m", "settled_m"?}`.
   - The existing `if s.forward != forward` block releases W on the hold-off tick with no change.
5. Tests in `supervisor/world/coast_tests.rs` (pure and `Sim`); model `Sim` set-up on `move_to_tests.rs`.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `first_sample_replaces_the_prior` | default model, `learn(5.0, 1.5)`: `coast_s` 0.3; `learn(5.0, 0.5)`: 0.3 + 0.3 * (0.1 - 0.3) = 0.24 | step 1 |
| `slow_releases_teach_nothing` | `learn(0.3, 0.1)`: unchanged, `samples` 0 | step 1 |
| `coast_is_clamped` | `learn(1.0, 5.0)`: `coast_s` 0.6 | step 1 |
| `zero_lead_is_todays_controller` | `MoveController` with `set_lead(0.0, 300)` walking a scripted pose sequence gives the same `MoveStep`s as one without (compare the vectors) | step 2 |
| `lead_releases_early_then_judges_settled` | lead 1.5, arrival 0.5: at `dist` 1.9 the step has `forward: false` and no outcome; during the hold-off no outcome; after it, at `dist` 0.4, `Arrived { distance_m: 0.4 }`; at `dist` 0.8, no `Arrived` and a later step walks again | step 2 |
| `move_to_lands_inside_arrival_when_coasting` | `Sim` with `lag.coast_s` 0.3 and `coast_model: Some(CoastModel { coast_s: 0.3, samples: 1 })`, goal 10 m ahead, `arrival_m` 0.5: after `run` returns, `sim.advance(1000)`, the horizontal distance to the goal is at most 0.5 m | **the revert proof**: without the lead the walker reports `Arrived` at about 0.5 m and the pawn then coasts 1.5 m past |
| `move_to_learns_across_calls` | `Sim` with `lag.coast_s` 0.3 and no model: two `run`s to two goals; `sim.coast_model` after the first has `samples: 1` and `coast_s` within 0.05 of 0.3; the second lands within 0.5 m after `advance(1000)` | steps 3-4 |
| `waypoint_legs_do_not_release` | two-leg route with coasting `Sim`: W is never released between the legs (the `Sim` key log shows one W down, one W up) | step 4 |

Checks: L1, L2. Report the line counts of `steer.rs`, `movement.rs`, `io.rs`, `coast.rs` and `supervisor/mod.rs`.

### AX-55 DLL: trace_line

- **Host:** windows · **Agent:** rust-gameserver-dev · **Branch:** `lab-autopilot/AX-55-trace` · **Worktree:** `lax-55` (from main once the wave 2 PR merged; merges into `lax-w5`)
- **Status:** Ready once wave 2 merged. Static pass 2 S1 and the 2026-10-31 live pass ([ue3-backport-verified.md § Live pass](ue3-backport-verified.md#live-pass-lab-client-2026-10-31), L1, L2, L3, L7) answered every question it had; it does not wait on AX-01.
- **Commit:** `feat(client-telemetry): lab-only line trace through UWorld::SingleLineCheck (AX-55)`
- **Rules:** R1-R4, R6, R8, R10. Contract: [C14](contract.md#c14-line-trace-and-ledge-guard-ax-55-ax-56) DLL half, C13. Commit attribution: Sonnet.

C14, the DLL half, as it binds this packet:

- Params `{ "start": [f32;3], "end": [f32;3], "flags"?: u32 }`, client UE3 units. `flags` defaults to `0x2086` (level geometry); the only other accepted value is `0x20bf` (also pawns). These are the only two values proven live (L2); `0x3fff` access-violated at `0x00914FD3`. Anything else: error `-32602` `trace_line: flags must be 0x2086 or 0x20bf`, before any call.
- Call `0x00875970` as `extern "thiscall" fn(world: u32, hit: *mut u8 /* FCheckResult, 0x48 bytes */, source: u32, end: *const [f32; 3], start: *const [f32; 3], flags: u32, extent: *const [f32; 3], zero: u32) -> i32` (7 stack arguments, `ret 0x1c`, live L1). `world = *(0x01EE2684 as *const u32)`; extent zero (extent traces are untested); `source` the player's pawn actor, or 0 when the reader finds none. A direct `extern "thiscall"` function pointer: the `client_call_native` 6-argument cap and its 7-argument shim do not apply here.
- `FCheckResult` initialised: `Time` (f32 at `+0x20`) 1.0, `Item` (i32 at `+0x24`) -1, `LevelIndex` (i32 at `+0x40`) -1, everything else 0.
- Refuse with `-32000` `trace_line unavailable: <reason>`, before any call, when: the fingerprint gate did not pass (`fingerprint`); `world` is 0 or a guarded read of `[world]` is not the `UWorld` vtable `0x01838E7C` (`world not ready`; live L7 and L3: `GWorld` is class `World` and changes pointer per zone); or an earlier call faulted (`faulted earlier; restart the client`, with `error_with_data` `{"trace_latched": true}`). Never read the loading flag `0x01EE2B6C` (0 in every live sample, L3).
- Result `{ "hit": bool, "time": f32, "location"?: [f32;3], "normal"?: [f32;3], "actor"?: u32, "item"?: i32 }`. `hit = (ret == 0)`. When `hit`: `location` `+0x08`, `normal` `+0x14`, `actor` `+0x04`, `item` `+0x24`.
- A fault: `bridge::seh::guard` returns `SehFault`; set the process-wide latch `static TRACE_LATCHED: AtomicBool`, then answer `-32000` with `error_with_data(..., fault.to_exception_json())`, as `call_native` does (grep it in `bridge/`). Why the latch: the call reaches `MultiLineCheck` `0x008721B0`, which pushes a `GMem` stack mark; an access violation mid-call can leave the mark unrestored. The live AV (L2) left the client running, but nothing proves a second call into a half-unwound `GMem` is safe, so the DLL never makes one. Only a client restart clears it.
- Not in the C7 batch list (`bridge/batch.rs` `ALLOWED` is unchanged).

Files and changes:

1. New `bridge/trace.rs`:
   - Pure: `pub fn parse(params: &Value) -> Result<TraceReq, String>` (flags rule, three-float arrays; a wrong shape is `-32602` `trace_line: start and end must be [x, y, z]`).
   - Pure: `pub fn init_check_result() -> [u8; 0x48]` and `pub fn decode(ret: i32, buf: &[u8; 0x48]) -> Value`.
   - Pure: `pub fn pawn_actor(mem: &impl Mem) -> Option<u32>`: `[[ENTITY_MANAGER_VA] + MANAGER_LOCAL_ENTITY_OFF] + ENTITY_PAWN_OFF`, the chain `hooks/camera/read.rs` `read_pawn_pose` walks (l.181-183). Import the constants from `hooks/camera/layout.rs` if they are visible to `bridge`; if they are private, copy them with a comment citing `layout.rs` and the line, and do not widen their visibility (that file compiles without `lab-bridge`).
   - `pub fn dispatch(id, params) -> RpcResponse`, which calls `dispatch_with(id, params, mem, native)`: `native: &dyn Fn(u32, *mut u8, u32, &[f32;3], &[f32;3], u32) -> Result<i32, SehFault>` and `mem: &dyn Mem` are seams. The real native (only under `#[cfg(all(target_arch = "x86", windows))]`) wraps the `extern "thiscall"` call in `seh::guard`; the real `mem` is `LiveMem`. Tests pass a `FakeMem` and a fake native that writes a hit into the buffer or returns `Err`.
   - `pub(crate) static TRACE_LATCHED: AtomicBool` and a `#[cfg(test)] fn reset_latch()`. Order of checks in `dispatch_with`: parse, fingerprint, latch, world and vtable, then the call.
   - The gate: if AX-24's report names a stored "gate passed" flag, read it. If it found none, the main-thread drain runs inside the Tick hook, which `boot.rs` installs only after the fingerprint gate passed: cite that line in the C13 proof (item 4) and add a `debug_assert!` comment at the check site.
2. `bridge/dispatch.rs`: route `"trace_line" => super::trace::dispatch(id, &req.params)`. `bridge/mod.rs`: `pub mod trace;`.
3. Tests in `bridge/trace_tests.rs` (host-side and i686, the fake native everywhere; no test calls the real address).

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `flags_default_and_allowlist` | no `flags`: `0x2086`; `0x20bf` accepted; `0x3fff` errors `-32602` `trace_line: flags must be 0x2086 or 0x20bf` | C14 (live L2: `0x3fff` faulted) |
| `check_result_is_initialised` | `init_check_result()`: `+0x20` is 1.0f32, `+0x24` and `+0x40` are -1i32, the rest 0 | C14 |
| `return_one_is_no_hit` | fake returns 1: `{"hit": false, "time": 1.0}` and no `location` | C14 (live L1: returns 1 on no hit) |
| `bsp_hit_reports_item` | fake returns 0 with `item` 17 and an actor: `hit: true`, `item: 17`, `location` and `normal` decoded | C14 (live L2) |
| `own_pawn_is_the_source` | a `FakeMem` chain to pawn `0x1234_0000`: the fake native receives `source == 0x1234_0000`; a broken chain gives 0 | C14 |
| `refuses_without_world` | world read 0: `-32000` `trace_line unavailable: world not ready`, native not called | C14 |
| `wrong_world_vtable_refuses` | a `FakeMem` world whose first dword is not `0x01838E7C`: `world not ready`, native not called | C14 (L7) |
| `fault_is_reported_not_raised` | the fake native returns `Err(SehFault)`: `-32000` with the exception data | C13 |
| `fault_latches_off` | after one faulting call, a second call with a fake native that would succeed answers `faulted earlier; restart the client` with `trace_latched: true`, and the native is not called again | C14 latch: **the AV revert proof** |
| `flags_other_than_proven_refused` | `0x20be`, `0x3fff` and `0` each refuse `-32602` with the native not called | C14 (L2) |
| `batch_does_not_allow_trace` | `bridge::batch::ALLOWED` does not contain `trace_line` | C14 |

Every test that touches `TRACE_LATCHED` calls `reset_latch()` first.

Checks: T1, T2. Report the C13 proof, one line per item, with **item 6's exception stated**: `trace_line` calls one engine function that reads collision and writes only the caller's stack buffer. Also report which gate signal step 1 used, and the latch line (C13 item 7: a fault never leads to a second call).

### AX-56 client_trace_line and the ledge guard

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-56-ledge` · **Worktree:** `lax-56` (at `lax-w5` after AX-54 and AX-55 merged)
- **Status:** BlockedDependency AX-55, AX-54.
- **Commit:** `feat(lab): client_trace_line and a ledge guard for move_to (AX-56)`
- **Rules:** R1-R4, R6-R9. Contract: C14 daemon half, C8 (`StopKind::Ledge`).

C14, the daemon half, as it binds this packet:

- `WorldIo::trace_line(&mut self, start: Vec3, end: Vec3, flags: u32) -> Result<Option<TraceHit>, String>`, default `Err("trace_line unsupported".into())`. `Ok(None)`: nothing hit. `TraceHit { time: f64, location: Vec3, item: i32, actor: u32 }`, client space.
- `client_trace_line { start: [f64;3], end: [f64;3], flags?: u32, instance? }`: LEASED, `NO_CLAIM`; returns the DLL result unchanged; a `-32601` gives `{"error": "trace_line needs a lab DLL with trace_line (AX-55)"}`.
- Ledge guard in `client_move_to`, `allow_drops?: bool` (default false): at most every 500 ms while walking, trace down at `LEDGE_AHEAD_UU` = 150 ahead of the pawn along its yaw, from the pawn's z to `pawn.z - LEDGE_HALF_HEIGHT_UU - LEDGE_DROP_UU` (78 + 200), flags `0x2086`. No hit: release the keys and fail with `StopKind::Ledge`, message `ledge ahead: no floor within 2 m`. Any trace error turns the guard off for that walk and the result carries `"ledge_guard": "unavailable: <error>"`; otherwise `"on"`, or `"off"` with `allow_drops`.
- Self-check at walk start: terrain and outdoor floors are untested live (only `SGC_W1` interiors answered `0x2086`). Before the first step, one trace straight down under the pawn over the same span; no hit sets `Unavailable("no floor under pawn")` for that walk, so a floor the trace cannot see never stops a walk as a false ledge.

Files and changes:

1. `supervisor/world/io.rs`: the `TraceHit` struct and the default `trace_line` method (at most 10 lines). `LiveWorld::trace_line` lives in a new `supervisor/world/trace_io.rs` as a free `pub(super) async fn live_trace(sup: &Supervisor, start, end, flags) -> Result<Option<TraceHit>, String>` that `bridge_call("trace_line", {start: [x,y,z], end, flags})` and maps `hit: false` to `None`; the `impl WorldIo for LiveWorld` method is one line calling it. An error containing `-32601` maps to `trace_line unsupported`.
2. New `supervisor/world/ledge.rs`:
   - `pub const LEDGE_AHEAD_UU: f64 = 150.0; pub const LEDGE_HALF_HEIGHT_UU: f64 = 78.0; pub const LEDGE_DROP_UU: f64 = 200.0; pub const LEDGE_EVERY_MS: u64 = 500;`.
   - Pure: `pub fn probe(pose: Pose) -> (Vec3, Vec3)`: the point `LEDGE_AHEAD_UU` along `(cos yaw, sin yaw)` from `pose.pos`, at `pose.z` and at `pose.z - 278`. (Check the yaw convention against `steer.rs` `yaw_error`; use the same.)
   - `pub enum Guard { On { last_ms: Option<u64> }, Off, Unavailable(String) }` with `pub fn label(&self) -> String` (`"on"`, `"off"`, `"unavailable: <e>"`).
   - Pure: `pub fn under(pose: Pose) -> (Vec3, Vec3)`: the same span straight under `pose.pos`. `pub async fn self_check<W: WorldIo>(g: &mut Guard, io: &mut W, pose: Pose)`: only in `On`; `Ok(None)` sets `Unavailable("no floor under pawn".into())`, `Err(e)` sets `Unavailable(e)`.
   - `pub async fn check<W: WorldIo>(g: &mut Guard, io: &mut W, now: u64, pose: Pose) -> bool` (true: a ledge ahead). It traces only in `On` and only when `last_ms` is `None` or `LEDGE_EVERY_MS` old; `Err(e)` sets `Unavailable(e)` and returns false; `Ok(None)` returns true.
3. `supervisor/world/movement.rs` (at most 15 lines): `MoveRequest` (and the tool args in `server/world.rs`) gain `allow_drops: Option<bool>`. Before the loop, `let mut guard = if allow_drops { Guard::Off } else { Guard::On { last_ms: None } };` then `ledge::self_check(&mut guard, io, start_pose).await;`. In the tick, only while `forward` is held: `if ledge::check(&mut guard, io, now, pose).await { release_stray(io).await; return Err(steps.fail_stop(now, &format!("leg_{i}"), "ledge ahead: no floor within 2 m", <StopReport kind Ledge, built the way AX-32 builds its stops>)); }`. The success JSON and every stop's evidence gain `"ledge_guard": guard.label()`.
4. `Sim` (in a new `supervisor/world/sim_trace.rs`): `pub floor_end_x: Option<f64>` on `Sim`. `trace_line` returns `Ok(Some(hit at end))` unless `floor_end_x` is `Some(x)` and the trace's `start.x > x`, then `Ok(None)`. A second field `pub trace_error: Option<String>` makes it return `Err`.
5. New `server/trace.rs` with its own router (`trace_router`), tool `client_trace_line`, the args per C14 plus `instance` and `lease_id`. Description: `Trace a line in the client world (UE3 units, client space) through the engine's own collision check. flags 0x2086 (level geometry, the default) or 0x20bf (also pawns). Returns hit, time, location, normal, actor and item (a level hit has item >= 0). A read: no evidence.` `server/mod.rs`: `mod trace;` and `+ Self::trace_router()` (2 lines).
6. `lease/policy.rs`: `client_trace_line` in `LEASED`. `proof/claims.rs`: in `NO_CLAIM`.
7. `server/world.rs` `client_move_to` description: append ` Stops before a drop of more than 2 m ahead (stop kind ledge) unless allow_drops; ledge_guard says whether the guard ran.`
8. Tests: `supervisor/world/ledge_tests.rs` (pure and `Sim`).

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `probe_points_ahead_and_down` | yaw 0 at (0,0,100): start (150, 0, 100), end (150, 0, -178) | C14 |
| `walk_stops_at_the_ledge` | `Sim` with `floor_end_x: Some(500.0)`, goal at x 2000: error with `stop.kind == "ledge"`, message `ledge ahead: no floor within 2 m`, pawn x below 700, W released | **the revert proof** |
| `allow_drops_walks_off` | same with `allow_drops: true`: arrives; `ledge_guard: "off"` | C14 |
| `trace_error_disables_the_guard` | `trace_error: Some("trace_line unsupported")`: arrives; `ledge_guard` is `"unavailable: trace_line unsupported"` | C14 |
| `no_floor_under_pawn_disables_guard` | `Sim` with `floor_end_x: Some(-1.0)` (no floor anywhere): the walk arrives, never stops as `ledge`, and `ledge_guard` is `"unavailable: no floor under pawn"` | C14 self-check: an unseen terrain floor is never a false ledge |
| `guard_traces_at_most_every_500ms` | a counting `Sim`: a 3 s walk makes at most 8 traces (one self-check plus one per 500 ms) | C14 |
| `trace_tool_is_classified` | `claim("client_trace_line") == Claim::NoClaim`; `every_routed_tool_is_classified` passes | R9 |

Checks: L1, L2. Report the `movement.rs`, `io.rs` and `server/mod.rs` line counts.
