# lab-autopilot wave 2: bridge foundations

> **Type:** Reference
> **Audience:** Packet-coders and packet-reviewers of wave 2
> **Last updated:** 2026-10-31
> **Companions:** [work packets](work-packets.md) (rules R1-R10, checks), [contract](contract.md), [ledger](README.md)

Wave 2 gives the lab-only DLL four things: a frame stamp on every response (C4), key holds that release themselves (C5), a per-frame watch sampler (C6) and same-frame batch reads (C7). Its daemon packets read the stamp, check post-reads for freshness, drive holds from the world tools, and (AX-26) route raw `client_input_key` presses through the same holds, so no key outlives the daemon. Every DLL packet carries R10 and writes the C13 proof in its report.

Every packet carries R1-R10 of [work-packets.md § Rules](work-packets.md#rules-every-packet-carries). DLL paths are under `crates/client-telemetry/src/`; daemon paths under `crates/lab/src/`.

**Dispatch:**

- AX-20 starts from `main` as soon as wave 1 is dispatched; it touches only `crates/client-telemetry`.
- AX-21, AX-24 and AX-25 branch at AX-20's reviewed head (`branch-at.ps1`) and run in parallel.
- After wave 1's PR merges, `ops` creates `lax-w2` from main and merges AX-20, AX-21, AX-24 and AX-25 in that order.
- AX-22 then branches at the `lax-w2` head. AX-23 branches after AX-22 merged in (both edit `world/io.rs`). AX-26 branches after AX-23 merged in (it builds on `held_keys.rs`).

**Merge order:** AX-20, AX-21, AX-24, AX-25, AX-22, AX-23, AX-26. One PR from `lab-autopilot/wave2`.

**DLL test rule (all DLL packets).** Game memory and `PostMessageW` are not available in host tests. Put the logic in pure types that take a clock and a reader or poster as arguments, and test those. The thin live wrapper (the `LiveMem` reads, the real post) is `#[cfg(all(windows, target_arch = "x86"))]`, like `bridge/input/mod.rs` l.203-207. The fake memory is `hooks::entity_trace::map::fake::FakeMem`.

### AX-20 Frame stamps and the Tick hub

- **Host:** windows · **Agent:** rust-gameserver-dev · **Branch:** `lab-autopilot/AX-20-meta` · **Worktree:** `lax-20` (from `main`)
- **Commit:** `feat(client-telemetry): frame stamp on lab bridge responses and a tick hub (AX-20)`
- **Rules:** R1-R4, R6, R8, R10. Contract: [C4](contract.md#c4-bridge-meta-dll-to-daemon) DLL half, [C13](contract.md#c13-client-side-safety-proof-every-dll-packet).

Files and changes:

1. New `bridge/meta.rs`:
   - `Meta` as C4 gives it (`Serialize`, `Debug`, `Clone`, `PartialEq`).
   - `static START: OnceLock<Instant>` and `pub(crate) fn mark_start()`, called first thing in `bridge::start` (`bridge/mod.rs` l.143).
   - `pub(crate) fn now(game_t: Option<f64>) -> Meta` with `frame = super::heartbeat_count()` and `mono_ms` = elapsed since `START` (0 when unset).
   - `pub(crate) fn live_game_t() -> Option<f64>`, which calls `crate::hooks::ability_trace::clock::game_time(&LiveMem)` on i686 Windows and returns `None` elsewhere. `game_time` is `pub(crate)` and takes `&dyn Mem`.
2. `bridge/dispatch.rs`:
   - `RpcResponse` (l.~46-66) gains `#[serde(skip_serializing_if = "Option::is_none")] pub meta: Option<super::meta::Meta>`.
   - Set it to `None` in `ok`, `error` and `error_with_data`.
3. `bridge/mod.rs` `BridgeHandle::drain` (l.116-130): after `dispatch::dispatch(...)`, set `resp.meta = Some(meta::now(meta::live_game_t()))` (make `resp` `mut`). Responses built in `handle_request_body` and `enqueue` stay without `meta`, as C4 says.
4. New `bridge/tick.rs`, the hub that later packets fill:

```rust
//! Per-frame work for lab features: called once per Tick from
//! `drain_main_thread`. Idle cost: two relaxed atomic loads (C13 item 3).
pub(crate) static HOLDS_ACTIVE: AtomicBool = AtomicBool::new(false);
pub(crate) static WATCH_ACTIVE: AtomicBool = AtomicBool::new(false);
pub(crate) fn on_tick() {
    if HOLDS_ACTIVE.load(Ordering::Relaxed) { holds_tick(); }
    if WATCH_ACTIVE.load(Ordering::Relaxed) { watch_tick(); }
}
/// Called when the authenticated daemon connection ends.
pub(crate) fn on_disconnect() { holds_disconnect(); watch_disconnect(); }
fn holds_tick() {}        // AX-21
fn holds_disconnect() {}  // AX-21
fn watch_tick() {}        // AX-24
fn watch_disconnect() {}  // AX-24
```

   - Under `#[cfg(test)]`, add `static TICK_WORK: AtomicU64` that each of the four stubs increments; later packets keep that increment at the top of their bodies.
5. `bridge/mod.rs`:
   - Add `pub mod meta; pub mod tick;` in the module list (l.37-50, alphabetical).
   - In `drain_main_thread` (l.195-200), call `tick::on_tick()` after the drain block.
   - In `serve_client`, call `tick::on_disconnect()` once after the request loop ends (l.~262), and only there: an unauthenticated connection never reaches it.
   - At most 12 lines added to `mod.rs` (445).
6. Tests in new `bridge/meta_tests.rs` (wired from `meta.rs`) and in a `#[cfg(test)] mod tests` in `tick.rs`.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `drained_responses_carry_meta` | start a bridge as `end_to_end_request_response` does (`bridge/mod.rs` l.461), send `heartbeat`, drain on the test thread: the response JSON has `meta.frame` and `meta.mono_ms`, and `meta.frame` equals the reply's `tick_count` | step 3 |
| `queue_errors_carry_no_meta` | the response from `enqueue` on a full queue (as `queue_overflow_returns_error_variant` builds it) serialises without `meta` | C4: off-thread responses are unstamped |
| `meta_omits_absent_game_t` | `serde_json::to_value(Meta{frame:1,mono_ms:2,game_t:None})` has no `game_t` key | C4 |
| `idle_tick_does_no_work` | with both flags false, `on_tick()` 100 times leaves `TICK_WORK` unchanged | C13 item 3 |
| `disconnect_runs_once_per_session` | the bad-token path (`bad_token_closes_connection` pattern) does not bump `TICK_WORK`; an authenticated connect then close bumps it by exactly 2 (both disconnect stubs) | step 5 |

Tests that share `TICK_WORK` or the global `HEARTBEAT` must not run in parallel with each other. Read the counter before and after, and assert on the difference, never on an absolute value.

Checks: T1, T2. Report the C13 proof (7 lines) and the `bridge/mod.rs` line count.

### AX-21 Key holds with a lease

- **Host:** windows · **Agent:** rust-gameserver-dev · **Branch:** `lab-autopilot/AX-21-holds` · **Worktree:** `lax-21` (at AX-20's reviewed head)
- **Commit:** `feat(client-telemetry): lab key holds that release themselves (AX-21)`
- **Rules:** R1-R4, R6, R8, R10. Contract: [C5](contract.md#c5-key-holds-with-a-lease) DLL half, C13.

Files and changes:

1. New `bridge/input/hold.rs`, a pure table:
   - `pub(crate) struct Hold { vk: u8, lparam_up: u32, hwnd: u32, until_ms: u64 }`.
   - `pub(crate) struct HoldTable { holds: Vec<Hold> }`.
   - `pub(crate) enum Post { Down { hwnd: u32, vk: u8, lparam: u32 }, Up { hwnd: u32, vk: u8, lparam: u32 } }`.
   - Methods, each taking `now_ms` and returning the posts to make:
     - `press(params, now_ms) -> Result<(bool /*posted*/, Vec<Post>), String>`;
     - `release(params) -> Vec<Post>`;
     - `expire(now_ms) -> Vec<Post>`;
     - `release_all() -> Vec<Post>`;
     - `snapshot(now_ms) -> Vec<(u8, u32 /*left_ms*/)>`;
     - `is_empty()`.
   - Clamp `lease_ms` per C5, with `pub const MAX_HOLDS: usize = 16`, and the exact error text `too many holds (max 16)`.
   - Counters `pub(crate) static HOLDS_EXPIRED` and `HOLDS_RELEASED_ON_DISCONNECT` (`AtomicU64`).
2. New `bridge/input/post.rs`:
   - `pub(crate) fn post(p: &Post)`. On i686 Windows it calls `PostMessageW` through a local declaration: `#[link(name = "user32")] extern "system" { fn PostMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> i32; }`, with `WM_KEYDOWN = 0x0100` and `WM_KEYUP = 0x0101`. Elsewhere it is a no-op.
   - Do **not** add a `windows-sys` feature to `Cargo.toml`. A Cargo.toml change would touch the default build (C13 item 1).
3. `bridge/input/mod.rs` (248 lines; at most 45 added):
   - `pub mod hold; pub mod post;`.
   - `pub(crate) fn hold_table() -> &'static Mutex<HoldTable>` (the `input_state()` pattern, l.36-41).
   - `pub fn dispatch_hold(id, params)`:
     1. Parse `{hwnd, vk, lparam_down, lparam_up, down, lease_ms?}`; a parse error is `INVALID_PARAMS` with `input_hold: <e>`.
     2. Call `press` or `release` with `now_ms` from `crate::bridge::meta`'s clock. Add a `pub(crate) fn mono_ms() -> u64` there if AX-20 did not.
     3. `post::post` each returned post.
     4. Set `tick::HOLDS_ACTIVE` to `!table.is_empty()`.
     5. Return `{posted, held: [{vk, left_ms}]}`.
   - `dispatch_status` adds `"holds"`, `"holds_expired"` and `"holds_released_on_disconnect"` (C5).
   - `dispatch_release` (l.186) first posts `hold_table().lock()` then `release_all()`, clears `tick::HOLDS_ACTIVE`, then runs today's `InputState::release_all()`. So `input_release` lets go of every hold too (C5 key fail-safe).
4. `bridge/dispatch.rs`: route `"input_hold" => super::input::dispatch_hold(id, &req.params)`, next to `input_key`.
5. `bridge/tick.rs`:
   - `holds_tick` locks the table, posts `expire(now)`, adds the count to `HOLDS_EXPIRED` and clears `HOLDS_ACTIVE` when the table is empty.
   - `holds_disconnect` posts `release_all()`, adds the count to `HOLDS_RELEASED_ON_DISCONNECT`, runs `input_state().lock()` then `release_all()`, and clears `HOLDS_ACTIVE`.
   - Keep the `TICK_WORK` test increment.
   - Use `try_lock` in `holds_tick`. If it fails, skip this frame: never block the game thread.
6. Tests in new `bridge/input/hold_tests.rs` (wired from `hold.rs`) and in `tick.rs`.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `press_posts_once_and_extends` | first `press` returns `posted: true` and one `Down`; a second `press` of the same vk returns `posted: false`, no post, and a later `until` | C5 |
| `lease_is_clamped` | `lease_ms` 50 gives 200; 60000 gives 10000; absent gives 10000 | D-AX8 |
| `seventeenth_key_errors` | 16 distinct vks succeed; the 17th errors `too many holds (max 16)` and posts nothing | C5 |
| `expire_posts_key_up_with_lparam_up` | at `until` the table posts `Up` with the hold's `lparam_up` and drops it | C5 |
| `release_all_posts_every_up` | three holds give three `Up` posts and an empty table | C13 item 7 |
| `dispatch_hold_bad_params_is_invalid_params` | `dispatch_hold` with `{}` is error `-32602` | step 3 |
| `status_reports_holds` | after a press through `dispatch_hold` (the post is a no-op on the host), `dispatch_status` has `holds` with that vk | step 3 |
| `disconnect_releases_holds_and_input` | a held key plus a held DI key in `InputState`: `tick::on_disconnect()` empties both and bumps `HOLDS_RELEASED_ON_DISCONNECT` by 1 | C13 item 7 |
| `input_release_releases_holds` | a press through `dispatch_hold`, then `dispatch_release`: `dispatch_status` shows no holds and `HOLDS_ACTIVE` is false | C5 key fail-safe |

The table and `InputState` are process globals, so every test that touches them must reset them first and must not run in parallel with the others.

Checks: T1, T2. Report the C13 proof and the `bridge/input/mod.rs` line count.

### AX-24 Watch sampler

- **Host:** windows · **Agent:** rust-gameserver-dev · **Branch:** `lab-autopilot/AX-24-watch` · **Worktree:** `lax-24` (at AX-20's reviewed head)
- **Commit:** `feat(client-telemetry): per-frame lab watch sampler (AX-24)`
- **Rules:** R1-R4, R6, R8, R10. Contract: [C6](contract.md#c6-watch-sampler), C13.

Files and changes:

1. New `bridge/watch_ring.rs`, pure:
   - `Sample` (`Serialize`, every optional field `skip_serializing_if = "Option::is_none"`, names exactly as C6).
   - `Ring { buf: VecDeque<Sample>, cap, next_i, dropped, period, last_frame }`.
   - `fn due(&self, frame) -> bool`, true when `frame - last_frame >= period`.
   - `fn push(&mut self, s)`, which overwrites the oldest sample when full and counts it in `dropped`.
   - `fn read(&self, since, max) -> (Vec<Sample>, next)`.
   - Clamp `period_frames` to 1..=60 and `cap` to 16..=4096 (C6).
2. New `bridge/watch.rs`:
   - `static WATCH: Mutex<Option<Ring>>`.
   - `dispatch_start`, `dispatch_read` and `dispatch_stop` per C6. `start` replaces a running watch and reports `replaced: true`. `read` never removes samples.
   - `fn sample(frame, ms, mem: &dyn Mem) -> Sample`. It fills `t` from `game_time`, `pos` and `rot` from `hooks::camera::read::read_pawn_pose`, and `hover`, `mode` and `buttons` from `hooks::nav::controller::read_controller` with the PC from `find_player_controller`. A reader that returns `None` leaves its field absent.
   - `actors`: fill it only when `docs/reverse-engineering/findings/lab-autopilot-anchors.md` (AX-01, static) marks `actor_count` `confirmed`; the 2026-10-31 live pass did not read it. Otherwise leave the field `None` and say so in the report. Never guess an offset.
   - These readers take `&impl Mem` or `&dyn Mem`. Pass `LiveMem` on i686 Windows and a `FakeMem` in tests.
   - Fingerprint mismatch: the Tick hook is one of the gated hooks, so `watch_tick` only runs on a client whose gate passed. If you find a stored "gate passed" flag (grep the callers of `gate_and_install` in `boot.rs`), read it and emit `{i, f, ms}` when it is false.
3. `bridge/tick.rs`:
   - `watch_tick` uses `try_lock`; when `due`, it pushes `sample(heartbeat_count(), mono_ms, &LiveMem)` (the host build passes a no-op reader).
   - `watch_disconnect` sets the watch to `None` and clears `WATCH_ACTIVE`.
   - `dispatch_start` sets `WATCH_ACTIVE`; `dispatch_stop` clears it.
4. `bridge/dispatch.rs`: route `watch_start`, `watch_read` and `watch_stop`. `bridge/mod.rs`: `pub mod watch; pub mod watch_ring;`.
5. Tests in new `bridge/watch_tests.rs` and a `#[cfg(test)] mod tests` in `watch_ring.rs`.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `ring_overwrites_and_counts_dropped` | cap 16, push 20: 16 kept, `dropped` 4, the first kept `i` is 4 | C6 |
| `read_since_moves_no_cursor` | `read(0, 5)` twice gives the same 5 samples; `next` is 5 | C6: two readers |
| `period_skips_frames` | period 3: frames 0..9 give samples at 0, 3, 6, 9 | C6 |
| `params_are_clamped` | `period_frames` 0 gives 1 and 99 gives 60; `cap` 1 gives 16 and 99999 gives 4096 | C6 |
| `sample_omits_failed_reads` | a `FakeMem` with no controller gives a sample without `hover`, `mode` and `buttons` | C6 |
| `start_replaces_and_stop_reports` | start, start again (`replaced: true`), stop gives `samples_taken` and `dropped` | C6 |
| `disconnect_stops_the_watch` | after `tick::on_disconnect()`, `watch_read` reports `running: false` and `WATCH_ACTIVE` is false | C13 item 7 |

Checks: T1, T2. Report the C13 proof, and whether `actors` is in (with the AX-01 line that allowed it) or out.

### AX-25 Same-frame batch

- **Host:** windows · **Agent:** rust-gameserver-dev · **Branch:** `lab-autopilot/AX-25-batch` · **Worktree:** `lax-25` (at AX-20's reviewed head)
- **Commit:** `feat(client-telemetry): same-frame batch of lab reads (AX-25)`
- **Rules:** R1-R4, R6, R8, R10. Contract: [C7](contract.md#c7-batch-reads-in-one-frame), C13.

Files and changes:

1. New `bridge/batch.rs`:
   - `pub const ALLOWED: [&str; 6] = ["heartbeat", "mem_read", "lua_eval", "input_status", "watch_read", "module_info"];`.
   - `pub fn dispatch(id, params) -> RpcResponse`:
     1. Parse `{calls: [{method, params}]}`.
     2. Check the length is 1..=16. Otherwise return `INVALID_PARAMS` with `batch: 1..=16 calls`.
     3. Check every method against `ALLOWED` before anything runs. A method not in it returns `INVALID_PARAMS` with `batch: method <m> not allowed`.
     4. Run each call through `super::dispatch::dispatch` with a synthetic `RpcRequest` (id 0).
     5. Map each result to `{"ok": result}` or `{"error": {code, message}}`.
     6. Return `{results, frame: heartbeat_count()}`.
   - `watch_read` is allowed even before AX-24 merges. Without AX-24 it answers `unknown method`, which is a per-call error and does not fail the batch.
2. `bridge/dispatch.rs`: route `"batch" => super::batch::dispatch(id, &req.params)`. `bridge/mod.rs`: `pub mod batch;`.
3. Tests in a `#[cfg(test)] mod tests` in `batch.rs`.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `batch_runs_reads_in_order` | `[heartbeat, module_info]` gives 2 results in order with one `frame` | C7 |
| `a_write_refuses_the_whole_batch` | `[heartbeat, mem_write]` errors `-32602` `batch: method mem_write not allowed`; the heartbeat did not run (a `mem_read` of a test buffer placed before it shows no side effect; or assert the error came before dispatch by using `call_native`, whose dispatch would fail differently) | C7: nothing runs |
| `nested_batch_refused` | `[batch]` is refused | no recursion |
| `empty_and_oversized_refused` | 0 calls and 17 calls error | C7 |
| `per_call_errors_do_not_fail_the_batch` | `[mem_read with a bad addr, heartbeat]` gives `[{error}, {ok}]` | C7 |

Checks: T1, T2. Report the C13 proof (item 6: batch adds no capability).

### AX-22 The daemon reads frames and checks freshness

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-22-fresh` · **Worktree:** `lax-22` (at `lax-w2` after AX-20, AX-21, AX-24 and AX-25 merged)
- **Commit:** `feat(lab): bridge frame stamps and fresh post-reads (AX-22)`
- **Rules:** R1-R4, R6, R7, R8. Contract: C4 daemon half.

Files and changes:

1. `client.rs` (347 lines; at most 40 added):
   - Add `#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)] pub struct BridgeMeta { pub frame: u64, pub mono_ms: u64, #[serde(default)] pub game_t: Option<f64> }`.
   - Factor `call_inner` so it returns the whole response object. `call` keeps its signature and returns `result`.
   - Add the new `pub async fn call_meta(&self, method, params) -> Result<(Value, Option<BridgeMeta>)>`, which reads `resp.get("meta")`. A missing or unparseable `meta` is `None`. It refreshes `last_ok_ms` like `call`.
2. `supervisor/events/fake_bridge.rs` (114 lines): add `pub type MetaResponder = Arc<dyn Fn(&str, &Value) -> Result<(Value, Option<Value>), String> + Send + Sync>` and `pub async fn supervisor_with_meta(r: MetaResponder) -> Supervisor`. It shares `serve` through a small enum or a second `serve_meta` fn, and adds `"meta"` to the response when the responder returns `Some`. The existing `supervisor` and `Responder` stay unchanged.
3. `proof/fresh.rs`: replace the stub with C4's `FRESH_FRAMES`, `Freshness` and `freshness()`, plus `pub fn mark_unstamped(after: Value) -> Value`, which applies the C4 wrap rule.
4. `supervisor/world/io.rs` `WorldIo`: add `async fn frame(&mut self) -> Option<u64> { None }` (default) with a doc line. `LiveWorld` implements it as `self.sup.bridge().heartbeat().await.ok()`. Use the existing heartbeat path; grep how `LiveWorld` reaches the `BridgeClient` (`self.sup`). The heartbeat `tick_count` is the same counter as `meta.frame` (C4), so this also works against an older DLL. At most 12 lines added.
5. `Sim` (`sim.rs` / `sim_model.rs`, from AX-13): add `pub frames: Option<u64>` (default `None`). `sleep(ms)` advances it by `max(1, ms / 16)` when it is `Some`, and `frame()` returns it.
6. New `supervisor/world/fresh_wait.rs`:
   - `pub async fn wait_fresh<W: WorldIo>(io: &mut W, action_frame: Option<u64>) -> (Freshness, Option<u64>)`.
   - It reads `io.frame()`. While the result is `Stale`, it sleeps 50 ms and reads again, at most 3 times.
   - It returns the last freshness and frame.
   - Declare it in `supervisor/world/mod.rs` as `pub mod fresh_wait;` (one line).
   - World tools call it after their action and before their post-read (wave 3).
7. Tests: `proof/fresh_tests.rs`, `client_meta_tests.rs` (wired from `client.rs`), and `supervisor/world/fresh_wait_tests.rs`.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `freshness_rules` | (10, 12) Fresh; (10, 11) Stale; (None, 5) and (5, None) Unstamped | C4 |
| `mark_unstamped_wraps_scalars` | `4711` gives `{"value":4711,"unstamped":true}`; an object gains `unstamped: true` | C4 |
| `call_meta_reads_meta_and_tolerates_absence` | `fake_bridge::supervisor_with_meta` answering with and without `meta`: `Some(BridgeMeta{frame:81234,..})`, then `None` | step 1 |
| `call_still_returns_result_only` | `call` against the meta fake returns just `result` | step 1 |
| `wait_fresh_waits_out_a_stale_frame` | `Sim` with `frames: Some(10)`, action frame 10: returns `Fresh` after one or more sleeps | step 6 |
| `wait_fresh_gives_up_after_three` | a `Sim` whose frame never advances (`frames` frozen by a test hook, or a `WorldIo` stub) returns `Stale` after exactly 3 sleeps | C4: at most 3 re-reads |
| `unstamped_without_frames` | `Sim` default: `(Unstamped, None)` and no sleep | C4 |

Checks: L1, L2. Report the `client.rs` and `io.rs` line counts.

### AX-23 World tools hold keys through the DLL

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-23-holdbook` · **Worktree:** `lax-23` (at `lax-w2` after AX-22 merged)
- **Commit:** `feat(lab): world-tool key holds renew through the lab DLL (AX-23)`
- **Rules:** R1-R4, R6, R7, R8. Contract: C5 daemon half. Decision: D-AX8.

Files and changes:

1. New `supervisor/hold_book.rs`, pure:
   - `HoldBook { held: BTreeMap<u8, Held { name: String, renew_at_ms: u64 }>, lease_ms: u32, fallback: bool }`.
   - `pub const LEASE_MS: u32 = 10_000;`.
   - Methods:
     - `pressed(vk, name, now)`, which sets `renew_at = now + lease/2`;
     - `released(vk)`;
     - `due(now) -> Vec<u8>`;
     - `renewed(vk, now, posted: bool) -> bool`, which returns `true` when `posted` (the key had been released and was pressed again: the caller logs it);
     - `clear()`;
     - `set_fallback()`, `is_fallback()`.
2. New `supervisor/held_keys.rs`, an `impl Supervisor` block:
   - `pub(crate) async fn hold_key(&self, key: &str, down: bool) -> Result<(), String>`:
     1. Resolve `keys::named(key)` and the window the way `input_key` does (`supervisor/input.rs` l.278 onward; reuse its hwnd lookup).
     2. In fallback mode, call `self.post_key(hwnd, k, down)` as today.
     3. Otherwise call `bridge_call("input_hold", {hwnd, vk, lparam_down: k.lparam_down(), lparam_up: k.lparam_up(), down, lease_ms: LEASE_MS})`.
     4. An error whose text contains `-32601` sets fallback and retries once with `post_key`.
     5. On success, update the book.
   - `pub(crate) async fn renew_holds(&self)`: for each `due` vk, send `input_hold down:true` again. When `renewed` returns `true`, log `tracing::warn!(target: "lab.hold", vk, "lab.hold.reasserted")`. Errors are logged at debug level and ignored: the DLL lease releases the key anyway.
   - `pub(crate) async fn release_all_holds(&self)`: send `input_hold down:false` for every held vk, then `clear()`.
3. `supervisor/mod.rs` (645 lines; at most 6 added): `mod hold_book; mod held_keys;`, a field `holds: tokio::sync::Mutex<hold_book::HoldBook>` on `Supervisor`, and its default in the constructor.
4. `supervisor/world/io.rs` `LiveWorld`:
   - `key()` calls `self.sup.hold_key(key, down)` instead of `input_key`.
   - `sleep(ms)` sleeps in slices of at most 1000 ms and calls `self.sup.renew_holds().await` between slices.
   - At most 15 lines changed.
5. `server/world.rs` `held_keys_guard_releases_on_a_dropped_future_only` (l.296) and the guard it tests: when the guard releases stray keys, it also calls `release_all_holds`. Grep `release_stray` callers. If the guard has no `Supervisor` handle, stop and report `blocked - contract gap: hold release on a dropped future`.
6. Tests: a `#[cfg(test)] mod tests` in `hold_book.rs`, and `supervisor/held_keys_tests.rs` on `fake_bridge::supervisor`. The fake answers `input_hold` with scripted `{posted, held}`, or with an error text containing `-32601`. Any test that needs an hwnd must stub it the way the existing input tests do; grep `fake_bridge` users that call `input_key`. If none can, test `hold_key` through a seam that takes the hwnd as an argument.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `renew_is_due_at_half_lease` | pressed at 0: `due(4999)` empty, `due(5000)` has the vk | C5 |
| `posted_renew_is_a_reassert` | `renewed(vk, t, true)` returns true and moves `renew_at` | C5 |
| `hold_key_uses_input_hold` | the fake records one `input_hold` with `down: true`, `lease_ms: 10000` and both lParams | step 2 |
| `old_dll_falls_back_to_posts` | the fake errors `-32601`: the book is in fallback and a second `hold_key` sends no `input_hold` | C5 |
| `renew_logs_reassert` | the fake answers a renew with `posted: true`: a `tracing` capture (or a counter the code exposes under `cfg(test)`) shows `lab.hold.reasserted` | C5 |
| `release_all_sends_ups` | two holds: two `input_hold down:false` calls, the book empty | step 2 |

Checks: L1, L2. Report the `supervisor/mod.rs` and `io.rs` line counts.

### AX-26 Raw keys through DLL holds, one release path

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-26-key-failsafe` · **Worktree:** `lax-26` (at `lax-w2` after AX-23 merged)
- **Commit:** `fix(lab): raw key presses die with the daemon; one input release path (AX-26)`
- **Rules:** R1-R4, R6, R7, R8, R9 (no new tool: say so). Contract: [C5](contract.md#c5-key-holds-with-a-lease) "Key fail-safe".

Why: today a `client_input_key` down is a posted message, so it stays held after a daemon crash until someone calls `client_input_release`; only `client_move_to` releases on cancel. Routed through AX-21's holds, a down dies with the bridge connection (the DLL `on_disconnect`) or the 10 s DLL lease.

Files and changes:

1. `supervisor/input.rs` `input_key` (l.278-297; at most 4 lines net added): `down` calls `self.hold_key(key, true)`, `up` calls `self.hold_key(key, false)`, `tap` calls `hold_key(key, true)`, sleeps `hold_ms` as today, then `hold_key(key, false)`. `hold_key` (AX-23) already falls back to `post_key` for an old DLL. `type_text` keeps `post_key` (its presses last milliseconds).
2. New `supervisor/input_release.rs`, an `impl Supervisor` block plus one free function:
   - `pub(crate) async fn release_all_input(&self, reason: &str) -> Result<Value, String>`: `self.bridge_call("input_release", json!({}))`, then `self.holds.lock().await.clear()` whatever the call returned, then `tracing::info!(target: "lab.input", reason, "lab.input.released")`; returns the call's result or its error. Leave a `// AX-27: button-ups` line where the mouse buttons will go.
   - `pub fn spawn_input_guards(sup: Arc<Supervisor>)`: a `tokio::spawn(... .in_current_span())` loop like `lease::spawn_sweeper` (`lease/mod.rs` l.120): every 1 s, `sup.renew_holds().await`. Nothing else yet; AX-27 adds the lease watch to this loop.
3. `supervisor/held_keys.rs` (AX-23): `release_all_holds` becomes `self.release_all_input("holds").await.map(|_| ())` when not in fallback (one release path); in fallback it posts the ups as today.
4. `supervisor/mod.rs`: `mod input_release;` and `pub use input_release::spawn_input_guards;` (2 lines).
5. `main.rs` (306 lines): `supervisor::spawn_input_guards(supervisor.clone());` next to each `lease::spawn_sweeper(...)` call (l.141 and l.190; 2 lines).
6. `server/input_tools.rs` (AX-15) `client_input_release`: `self.wrap(self.supervisor.release_all_input("tool").await)` in place of `self.proxy("input_release", json!({}))`.
7. `server/uat.rs` (641 lines; no net growth): the two `.bridge_call("input_release", json!({}))` calls (l.547 and l.556) become `.release_all_input("uat_lease_lost")`.
8. Tests in new `supervisor/input_release_tests.rs` (wired from `input_release.rs`) on `fake_bridge::supervisor`, which records each method it was sent. Stub the window handle as AX-23's `held_keys_tests.rs` does.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `input_key_down_is_a_dll_hold` | `input_key("W", "down", None)`: the fake recorded `input_hold` with `down: true` and no posted message path | step 1 |
| `input_key_tap_holds_then_lets_go` | `tap` records `input_hold` `down: true` then `down: false` | step 1 |
| `old_dll_key_still_posts` | the fake errors `-32601` on `input_hold`: `input_key down` succeeds through the fallback | step 1, C5 fallback |
| `release_all_input_clears_the_book` | one hold pressed, then `release_all_input("t")`: the fake recorded `input_release` and the `HoldBook` is empty | step 2 |
| `release_all_input_clears_even_on_error` | the fake errors on `input_release`: the call returns `Err` and the book is still empty | step 2 |
| `tool_release_goes_through_the_helper` | `client_input_release` through the server router on a fake bridge records `input_release` and empties the book | step 6 |

Checks: L1, L2. Report: the `input.rs`, `uat.rs` and `main.rs` line counts, and "no tool added; policy.rs unchanged". The live check (kill the daemon with W held; the avatar stops) is AX-64's key fail-safe row.
