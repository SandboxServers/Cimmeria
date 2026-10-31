# lab-autopilot contract

> **Type:** Reference
> **Audience:** Packet-coders and packet-reviewers of the lab-autopilot campaign
> **Last updated:** 2026-10-31
> **Companions:** [Ledger](README.md), [work packets](work-packets.md), [resume card](RESUME.md)

Every name, JSON key, method, error code and constant a packet uses comes from this file (rule R8). A packet that needs something not here stops with `blocked - contract gap: <what>`. Line numbers are from `main` `2019922fe`; re-grep every anchor before editing.

The module for the new honesty layer is `crates/lab/src/proof/` (Rust path `crate::proof`). It is not `crate::uat::evidence`, which is the UAT bundle writer and stays as it is.

## C1 Evidence

Every judged tool result carries one `evidence` object at the top level of its JSON:

```json
{
  "evidence": {
    "verdict": "confirmed | denied | inconclusive | unjudged",
    "why": "<C2 code>",
    "hint": "<C2 hint text>",
    "checks": [
      { "name": "hover_matches", "source": "client", "ok": true, "required": true,
        "before": 0, "after": 4711, "frame": 81234, "why": "no_hover" }
    ]
  }
}
```

- `source` is `client`, `server`, `pixel` or `bridge`.
- `ok` is `true`, `false` or absent. Absent means "not measured" (null). Edge compaction drops nulls, so readers treat a missing `ok` as null.
- `before`, `after`, `frame` and `why` are optional. `why` on a check names the C2 code that check implies when it is the deciding check.

Rust, in `crates/lab/src/proof/mod.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvVerdict { Confirmed, Denied, Inconclusive, Unjudged }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source { Client, Server, Pixel, Bridge }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Check {
    pub name: String,
    pub source: Source,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<Why>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    pub verdict: EvVerdict,
    pub why: Why,
    pub hint: String,
    pub checks: Vec<Check>,
}
```

Constructors and methods (exact names):

| Item | Meaning |
|---|---|
| `Check::new(name: &str, source: Source, ok: Option<bool>, required: bool) -> Check` | The other fields `None` |
| `Check::with_before(self, v: Value)`, `with_after`, `with_frame(u64)`, `with_why(Why)` | Builders, each returns `Self` |
| `Evidence::judge(checks: Vec<Check>) -> Evidence` | The rules below |
| `Evidence::unjudged() -> Evidence` | `Unjudged`, why `NotYetJudged`, no checks |
| `Evidence::no_evidence() -> Evidence` | `Inconclusive`, why `NoEvidence`, no checks |
| `Evidence::from_verdict(verdict: EvVerdict, why: Why, checks: Vec<Check>) -> Evidence` | For callers whose rule is not `judge` (input primitives, C2 `PostedOnly`) |
| `Evidence::counts(&self) -> bool` | `verdict == Confirmed` |
| `Evidence::to_value(&self) -> Value` | `serde_json::to_value` |
| `Evidence::from_value(v: &Value) -> Option<Evidence>` | `None` on any parse failure; a missing check `ok` reads as `None` |

`judge` rules, applied in order over the checks in vector order:

1. The first required check with `ok == Some(false)`: verdict `Denied`, why = that check's `why`, else `NoEffect`.
2. Else the first required check with `ok == None`: verdict `Inconclusive`, why = that check's `why`, else `NoPostRead`.
3. Else no required checks at all: verdict `Inconclusive`, why `NoEvidence`.
4. Else verdict `Confirmed`, why `Confirmed`.

Optional checks (`required: false`) never change the verdict; they are shown to the reader. `hint` is always `why.hint()`.

## C2 Why codes

`crates/lab/src/proof/why.rs`: `pub enum Why`, serde `snake_case`, with `pub fn as_str(self) -> &'static str` and `pub fn hint(self) -> &'static str`. Exactly these variants and hints:

| Code | Hint (exact text) |
|---|---|
| `confirmed` | The result was read back after the action and matches. |
| `not_delivered` | The game never read the input (its input counters did not move). Check focus, then retry once. |
| `ignored` | The game read the input but nothing changed. Something is blocking it (a dialog, a cast, a stun, UI focus): read the UI state. |
| `no_effect` | The action ran but the expected change did not happen. |
| `out_of_range` | The target is out of range. Move closer and retry (interact range is about 5 m). |
| `no_hover` | The cursor was not over the target when it clicked. Find the entity again and retry. |
| `stale_read` | The read-back came from the same frame as the action, so it proves nothing. Read again. |
| `no_post_read` | Nothing was read back after the action, so the result cannot be judged. Read the state you expected to change. |
| `server_unreachable` | The server could not be asked, so nothing corroborates the client. Report INCONCLUSIVE. |
| `server_disagrees` | The client shows the change but the server does not. Report it as a bug, not a pass. |
| `not_yet_judged` | This tool does not judge its result yet. Treat the result as a claim, not a fact. |
| `no_evidence` | This tool should judge its result but returned no evidence. Report INCONCLUSIVE. |
| `cooling_down` | The same call failed moments ago. Wait retry_after_ms or change the arguments. |
| `delivered` | The game received the input. Whether it had an effect is not judged here. |
| `posted_only` | The input was posted to the game window. Nothing can read back whether the game used it: read the state you expected to change. |
| `timeout` | The action did not finish before its deadline. |
| `stalled` | The action stopped making progress. Read stop for what was tried. |
| `already_true` | The expected state already held before the action, so reading it back proves nothing. Start from a state where it does not hold. |

## C3 The evidence gate

`crates/lab/src/proof/claims.rs`:

```rust
pub enum Claim { Judged, Unjudged, NoClaim }
pub const JUDGED: &[&str] = &[ /* grows packet by packet; empty in AX-11 */ ];
pub const NO_CLAIM: &[&str] = &[ /* reads, lease, uat, lifecycle tools: listed in AX-11 */ ];
pub fn claim(tool: &str) -> Claim; // JUDGED -> Judged, NO_CLAIM -> NoClaim, anything else -> Unjudged
```

`crates/lab/src/proof/gate.rs`: `pub fn stamp(tool: &str, v: &mut Value)`. Rules:

- (a) Only a JSON object without an `error` key is touched. Errors pass unchanged.
- (b) `Claim::NoClaim`: unchanged.
- (c) `Claim::Judged` and no parseable `evidence`: insert `Evidence::no_evidence()`.
- (d) `Claim::Unjudged` and no `evidence`: insert `Evidence::unjudged()`. One that already carries evidence keeps it.
- (e) Then, with `ev` the (now present) evidence:
  - `ev.verdict != Confirmed` and `counts_as_native_pass` is `true`: set it `false` and record `"claimed": {"counts_as_native_pass": true}`. The UI tools spell the same flag `native_pass` (`supervisor/ui/mod.rs` `NativeTrail::stamp`): apply the same rule to it and record `claimed.native_pass`. `native_level` and `native_tier` say how the input was driven, not whether it worked, and stay untouched.
  - `ev.verdict == Denied` and `effect_ok` is `true`: set `effect_ok` `false`, record `claimed.effect_ok = true`.
  - `ev.verdict == Inconclusive` and `effect_ok` is present: remove `effect_ok`, record `claimed.effect_ok = <old value>`.
  - `Unjudged` leaves `effect_ok` as it is (legacy until the AX-44 strict switch, D-AX13).
- (f) Insert `"verdict": "<ev.verdict>"` at the top level as a mirror for terse readers.
- (g) The strict switch (AX-44): `pub const STRICT: bool` in `proof/gate.rs`, `false` until AX-44 sets it `true`. With it on, rule (e) treats `Unjudged` like `Inconclusive` (removes `effect_ok`, records `claimed.effect_ok`), and the runner's `verdict::of_outcome` maps `unjudged` to `Inconclusive("{tool}: unjudged")`. Rule (d) still inserts `Evidence::unjudged()`, so a tool someone adds later without evidence is visible as such.

The gate runs in two places and nowhere else:

1. `server/handler.rs` `call_tool`, after the tool returns and before `log_call` and `shape_response`, on every `ContentBlock::Text` that parses as a JSON object and on `structured_content` when present. A text block that is not JSON passes unchanged.
2. `server/uat.rs` `RouterInvoker::call`, on `out.json` after `normalize_result`.

`log_call` gains a `verdict: Option<&str>` field in the `lab.call` event (the mirrored `verdict`, or `None`).

## C4 Bridge meta (DLL to daemon)

Every bridge response the DLL sends from its main-thread drain carries `meta`:

```json
{ "jsonrpc": "2.0", "id": 7, "result": { "...": "..." },
  "meta": { "frame": 81234, "mono_ms": 523311, "game_t": 1042.25 } }
```

- `frame`: the bridge tick counter (the same counter `heartbeat` reports) at the moment the response is built, after the method ran.
- `mono_ms`: milliseconds since the bridge started (`Instant`).
- `game_t`: `hooks::ability_trace::clock::game_time` when the fingerprint gate passed and it returns `Some`; otherwise absent.
- Error responses carry `meta` too. Responses built off the main thread (QUEUE_FULL, parse errors) carry none.

DLL: `crates/client-telemetry/src/bridge/meta.rs`, `pub struct Meta { pub frame: u64, pub mono_ms: u64, #[serde(skip_serializing_if = "Option::is_none")] pub game_t: Option<f64> }`; `RpcResponse` gains `#[serde(skip_serializing_if = "Option::is_none")] pub meta: Option<Meta>`.

Daemon: `crates/lab/src/client.rs` gains `pub async fn call_meta(&self, method: &str, params: Value) -> anyhow::Result<(Value, Option<BridgeMeta>)>` (match the error type `call` already returns), with `BridgeMeta { frame: u64, mono_ms: u64, game_t: Option<f64> }` (`Deserialize`). `call` keeps its signature and ignores `meta`. An old DLL gives `None`.

Freshness, `crates/lab/src/proof/fresh.rs`:

```rust
pub const FRESH_FRAMES: u64 = 2;
pub enum Freshness { Fresh, Stale, Unstamped }
pub fn freshness(action_frame: Option<u64>, read_frame: Option<u64>) -> Freshness;
// Fresh when read >= action + FRESH_FRAMES; Stale when both present and not Fresh; Unstamped when either is None.
```

A `Stale` post-read makes its check `ok: None` with why `stale_read`; the caller re-reads (at most 3 times, one 50 ms sleep apart) before keeping it. `Unstamped` keeps the measured `ok` and adds `"unstamped": true` to the check's `after` object, or wraps a scalar `after` as `{"value": <v>, "unstamped": true}`.

World tools get frames through `WorldIo::frame(&mut self) -> Option<u64>` (default `None`); `LiveWorld` returns the `frame` of a `heartbeat` `call_meta`; the test `Sim` returns its tick counter.

## C5 Key holds with a lease

DLL method `input_hold`, file `crates/client-telemetry/src/bridge/input/hold.rs`:

- Params: `{ "hwnd": u32, "vk": u8, "lparam_down": u32, "lparam_up": u32, "down": bool, "lease_ms"?: u32 }`. The daemon computes both lParams with `keys::Key::lparam_down()` / `lparam_up()` exactly as `supervisor/input.rs::post_key` does today.
- `lease_ms` is clamped to `200..=10000`; absent means `10000` (D-AX8).
- `down: true`, key not held: `PostMessageW(hwnd, WM_KEYDOWN, vk, lparam_down)`, record the hold with `until = now + lease`. Result `{"posted": true, ...}`.
- `down: true`, key held: extend `until`, post nothing. `{"posted": false, ...}`.
- `down: false`: post `WM_KEYUP` with `lparam_up` if held (or not; an up is always safe), remove the hold.
- At most 16 holds; a 17th distinct key is error `-32602` with message `too many holds (max 16)`.
- Every result is `{ "posted": bool, "held": [ { "vk": u8, "left_ms": u32 } ] }`.
- On every Tick (C13 idle rule applies): each expired hold posts `WM_KEYUP` and is removed; `HOLDS_EXPIRED` counts them.
- On client disconnect (`serve_client` loop exit): every hold posts `WM_KEYUP`, the list clears, `InputState::release_all()` runs, and `HOLDS_RELEASED_ON_DISCONNECT` counts the holds. Virtual focus is left as it is.
- `input_status` gains `"holds": [ {"vk", "left_ms"} ]`, `"holds_expired": u64`, `"holds_released_on_disconnect": u64`.

Daemon: `crates/lab/src/supervisor/hold_book.rs`, a pure `HoldBook` (no IO) that tracks what the daemon believes it holds and when to renew (`lease_ms / 2`). The bridge drops the TCP connection on any error, which releases every hold; so a renew whose result shows `"posted": true` means the key had been released and is now pressed again, and the book logs `lab.hold.reasserted`. A DLL without the method (`-32601`) puts the book in fallback mode for the connection: posted messages as today.

Key fail-safe (added 2026-10-31; today only `client_move_to` releases on cancel, and a raw `client_input_key` down stays held after a daemon drop until someone calls `client_input_release`):

- `input_release` (DLL, `bridge/input/mod.rs` `dispatch_release`) also releases every hold in the `HoldTable` (posts their `WM_KEYUP`), on top of `InputState::release_all()` (AX-21).
- `client_input_key` routes `down`, `up` and `tap` through the hold path (AX-26): `down` is a hold with the default lease (10 s, renewed by the `HoldBook` while the daemon lives), `up` releases it, `tap` is a hold down, the sleep, then the hold up. So a raw down dies with the daemon connection or the DLL lease, whichever comes first. Fallback mode (old DLL) posts as today.
- A daemon drop needs no daemon action: the TCP drop fires the DLL `on_disconnect`, which releases every hold and the DI state (AX-21). While the daemon lives, `supervisor/input_release.rs` `spawn_input_guards` renews every hold in the `HoldBook` once a second (AX-26). `release_all_input(instance, reason)` is the one release path: `input_release`, then clear the `HoldBook`, then the button-ups of C5's last bullet; `client_input_release` and the `release_input` recovery call it (AX-26, AX-35). The residual gap is fallback mode (a DLL without `input_hold`): a down posted there stays down until `client_input_release` or a lease end.
- When a lease ends (release, expiry or takeover), the daemon runs `release_all_input` on that instance, which also sends a button-up for every mouse button it posted down and has not posted up (AX-27). The guard loop sees the end by comparing `LeaseBook::current_id()` with the id it saw last: `subscribe()` wakes it at once on a release or takeover, and its 1 s tick sees an expiry (expiry does not notify). It logs `lab.input.released_on_lease_end` with `how` from `LeaseBook::ended_how(id)` (`released`, `expired`, `taken_over`). A daemon crash sends no button-up: posted mouse buttons are the second residual gap.

## C6 Watch sampler

DLL methods, file `crates/client-telemetry/src/bridge/watch.rs`. One watch at a time, main-thread only.

| Method | Params | Result |
|---|---|---|
| `watch_start` | `{ "period_frames"?: 1..=60 (default 1), "cap"?: 16..=4096 (default 1024) }` | `{ "started_frame": u64, "replaced": bool }` |
| `watch_read` | `{ "since"?: u64 (sample index, default 0), "max"?: 1..=1024 (default 256) }` | `{ "samples": [Sample], "next": u64, "dropped": u64, "running": bool }` |
| `watch_stop` | `{}` | `{ "samples_taken": u64, "dropped": u64 }` |

Sample: `{ "i": u64, "f": u64, "ms": u64, "t"?: f64, "pos"?: [f32;3], "rot"?: [i32;3], "hover"?: u32, "mode"?: u32, "buttons"?: u32, "actors"?: u32 }` (client space, UE3 units; `rot` is pitch, yaw, roll in UE3 rotator units). Reads use only the existing guarded readers: `hooks/camera/read.rs` `find_player_controller` and `read_pawn_pose`, `hooks/nav/controller.rs` `read_controller` (`mode`, `buttons`, `hover_id`), `game_time`, and the actor count only when AX-01 marks it confirmed (the level's Actors array `level + 0x3C` data / `+0x40` count, with `level = [world + 0x50]`, the chain `supervisor/world/memory.rs` already walks; AX-01 confirms it statically). A failed read omits its field. On a fingerprint mismatch a sample is `{i, f, ms}`. The ring overwrites the oldest sample and counts `dropped`. The watch stops on client disconnect and on `watch_stop`. `watch_read` moves no cursor (the caller passes `since`), so it is safe for two readers.

## C7 Batch reads in one frame

DLL method `batch`, file `crates/client-telemetry/src/bridge/batch.rs`. Params `{ "calls": [ { "method": str, "params": object } ] }`, 1..=16 calls. Each call runs in order inside the same drain, through the same handler its single call uses. Allowed methods: `heartbeat`, `mem_read`, `lua_eval`, `input_status`, `watch_read`, `module_info`. Any other method (writes, `call_native`, `hook_*`, `input_*` other than `input_status`, `console`, `batch`) makes the whole batch error `-32602` `batch: method <m> not allowed`, before anything runs. Result `{ "results": [ {"ok": value} | {"error": {"code", "message"}} ], "frame": u64 }`. Batch adds no capability: every allowed method is one a single call already offers.

## C8 Stop report

`crates/lab/src/proof/stop.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StopKind { Stuck, Timeout, IgnoredInput, Stalled, BlockedUi, Ledge }

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StopReport {
    pub kind: StopKind,
    pub step: String,
    pub seconds: f64,
    pub tried: Vec<String>,          // what the tool already tried, in order
    pub player: Option<StopPlayer>,  // where the player was and is
    pub suggest: Vec<String>,        // C11 recipe names, best first
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StopPlayer { pub at: [f64; 3], pub now: [f64; 3], pub moved_m: f64 }
```

`WorldFailure` gains `pub stop: Option<StopReport>` (default `None`); `to_json` adds `"stop"` when `Some`. `Steps::fail_stop(&self, now_ms, step, message, stop: StopReport) -> WorldError` builds it. Positions are server metres (`geometry::client_to_server`).

## C9 Quick-fail cooldown

`crates/lab/src/server/cooldown.rs`, MCP path only (the UAT runner never sees it):

- Key: tool name plus a hash of the args with `lease_id`, `instance`, `verbose` and `fields` removed (canonical JSON, sorted keys).
- A call fails when the tool returns an MCP error, `is_error: true`, or a JSON body whose `evidence.verdict` is `denied`.
- After the n-th consecutive failure of a key, the next identical call within `min(8000, 500 * 2^(n-1))` ms is refused without running: an `is_error` result with `{"error": "cooling down", "why": "cooling_down", "hint": <C2 hint>, "retry_after_ms": <left>, "failures": n}`.
- A success clears the key. A key idle for 60 s is forgotten. At most 256 keys (oldest evicted).
- Exempt: every tool in `lease::policy::OPEN`, the `lab_lease_*` tools, `lab_uat_*`, `lab_client_start/stop/restart/status`, `lab_recover`.

## C10 resume_from

`UatRunArgs` (`server/uat.rs`) and `RunRequest` (`uat/runner/mod.rs`) gain `resume_from: Option<String>`:

- `"<section>/<row>"`: rows before it in spec order are skipped; that row and the rest run (re-running a row overwrites its manifest entry).
- `"auto"`: resume at the first row in spec order that has no entry in the run's manifest.
- Requires `run_dir`; without it the call errors `resume_from needs run_dir`. With `rows` also set it errors `resume_from and rows are exclusive`. An unknown row errors `resume_from: no row <id> in the selected sections`. `auto` with every row present returns the summary with zero rows run.
- Skipped rows keep their manifest entries and are not re-graded.

## C11 Recovery recipes

Tool `lab_recover { recipe: "release_input" | "close_dialogs" | "recenter_camera" | "unstick", instance? }`, LEASED. Result `{ "recipe": str, "evidence": C1, "steps": [...] }` (a judged tool). Recipes, `crates/lab/src/supervisor/recover_recipes.rs` (not `recovery.rs`, which is crash bookkeeping):

| Recipe | Does | Confirmed when |
|---|---|---|
| `release_input` | `release_all_input("recover")`: `input_release` (which lets go of every DLL hold), the `HoldBook` cleared, the posted mouse buttons let go (C5) | `input_status` shows no held keys, buttons or holds |
| `close_dialogs` | Escape tap, up to 3 times, reading `Supervisor::ui_state(Some(0))` after each | its `dialog` field is null or absent (the probe `lab_play_character` reports as `dialog`, `supervisor/flows/world.rs:282-288`) |
| `recenter_camera` | the `client_camera` path with pitch 0 | `view_after` pitch within 5 degrees of 0 |
| `unstick` | the existing `movement.rs` unstick sequence | the player moved at least 0.5 m (client pose; server position as an optional check) |

## C12 Detectors

Tool `lab_detectors_read { instance?, since_ms? }`, OPEN. Pure detectors in `crates/lab/src/supervisor/detect/` run over watch samples (C6) the daemon already pulled plus the heartbeat history and the `lab.call` log. Result `{ "watch": bool, "window_ms": u64, "fired": [ { "name", "at_ms", "frame"?, "detail" } ] }`.

| Name | Fires when |
|---|---|
| `hitch` | watch: `(ms[i] - ms[i-1]) > 250 * max(1, f[i] - f[i-1])`, so with `period_frames` above 1 it is the mean frame over the gap and a single long frame can hide (use period 1 to catch it); heartbeat (no watch): two consecutive heartbeat observations at least 250 ms apart with the same `tick_count` (the client did not tick across the gap) |
| `teleport` | the pawn moved more than 30 m within 500 ms |
| `long_fall` | z decreasing for more than 3 s |
| `out_of_world` | z below -100000 UE3 units (provisional; D-AX6) |
| `nan` | any NaN or infinite position or rotation |
| `stuck_input` | a hold (C5) present for 3 s with the pawn moving less than 0.2 m |
| `actor_growth` | actor count up more than 50% within 60 s (only when samples carry `actors`) |
| `log_error` | a `lab.call` with outcome `error` for the same tool three times in 60 s |

Without a running watch only `hitch` (from heartbeat gaps) and `log_error` run, and `watch` is `false`. `stuck_input` reads when each hold started from the daemon's `HoldBook` (C5), not from the DLL. `since_ms` defaults to 120 s before the call; `window_ms` is the span the result covers.

There is no pitch-clamp detector: the sample's `rot` is the pawn's, and the camera pitch (`+0x368` on the camera object) is not sampled. `client_camera`'s evidence (AX-33) covers a stuck pitch instead.

## C13 Client-side safety proof (every DLL packet)

Each DLL packet's report proves each line for its own code:

1. **Absent from release.** All new code is under `crates/client-telemetry/src/bridge/`, which compiles only with feature `lab-bridge`. `build-desktop.sh` builds default features and its verify step refuses `LAB_BRIDGE_MARKER`; the packet does not touch either.
2. **Double gate.** Nothing runs unless the feature is compiled in and the session file carries the `lab` block (`maybe_start`).
3. **Idle no-op.** With no holds, no watch and no client connected, the per-Tick cost is at most two atomic loads; a unit test asserts that an idle tick posts nothing and reads no game memory.
4. **Fingerprint gate.** No new hook is installed. Game-memory reads go through the existing readers, which return `None` when the fingerprint gate failed.
5. **Kill switch.** `CIMMERIA_CLIENT_HOOKS_DISABLE=all` disables the Tick hook, so nothing in this campaign runs.
6. **No new writes.** The only side effects are the `WM_KEYDOWN`/`WM_KEYUP` posts the supervisor already sends today (C5). Watch and batch are read-only.
7. **Release on disconnect.** Holds release and the watch stops when the daemon goes away; a test covers it.

AX-55 adds one exception to item 6, and its report must say so: `trace_line` (C14) calls one engine function that reads collision and writes only the caller's stack buffer.

## C14 Line trace and ledge guard (AX-55, AX-56)

DLL method `trace_line`, file `crates/client-telemetry/src/bridge/trace.rs`. Both of its gates are met: Static pass 2 S1 gives the signature and the `FCheckResult` layout, and the live pass of 2026-10-31 ([ue3-backport-verified.md § Live pass](ue3-backport-verified.md#live-pass-lab-client-2026-10-31), L1 and L2) called it cleanly (`ret 0x1c`, level geometry answers `0x2086`). Not gated on AX-01. Still untested live: terrain and outdoor maps (only `SGC_W1` interior BSP and static meshes were traced), extent traces and `MultiLineCheck`; so the extent stays zero and the ledge guard self-checks (below).

- Params: `{ "start": [f32;3], "end": [f32;3], "flags"?: u32 }`, client space, UE3 units. `flags` defaults to `0x2086` (level geometry; BigWorld pawns are not hit). The only other accepted value is `0x20bf` (also pawns, live L2: `0x20be` misses them). Anything else is error `-32602` `trace_line: flags must be 0x2086 or 0x20bf` (live L2: `0x3fff` raised a guarded access violation).
- It runs in the main-thread drain and calls `UWorld::SingleLineCheck` `0x00875970` as `extern "thiscall" fn(world, *mut FCheckResult, source, *const [f32;3] end, *const [f32;3] start, flags, *const [f32;3] extent, 0) -> i32` (7 stack arguments, `ret 0x1c`). `world = *(0x01EE2684 as *const u32)`, the extent is zero, and `source` is the player's pawn actor when the reader finds it (so the trace never hits the player's own body), else 0. The `FCheckResult` is a `0x48`-byte stack buffer initialised as Static pass 2 S1.5 gives (`Time` 1.0 at `+0x20`, `Item` -1 at `+0x24`, `LevelIndex` -1 at `+0x40`, the rest 0). The call is wrapped in `bridge::seh::guard`. The DLL calls it directly as an `extern "thiscall"` function pointer; the 7-argument shim is only for `client_call_native` (6-argument cap) and is not used here.
- Refused with error `-32000` `trace_line unavailable: <reason>` (reasons below), before any call:
  - `fingerprint` when the fingerprint gate did not pass;
  - `world not ready` when `world` is 0, or the guarded read of `[world]` is not the `UWorld` vtable `0x01838E7C` (live L7: `GWorld` is class `World` with that vtable; live L3: the pointer changes on every zone change, so a stale or mid-swap pointer fails this check). The loading flag `0x01EE2B6C` is **not** read: it was 0 in all 2894 live samples, including across zone loads;
  - `faulted earlier; restart the client` once any call in this client session raised a `SehFault`. `SingleLineCheck` reaches `MultiLineCheck` `0x008721B0`, which pushes a `GMem` stack mark (`0x01EA5750`, mark at `0x01EA575C`); an access violation mid-call can leave the mark unrestored, so the first fault latches `trace_line` off for the rest of the client session (a process-wide `AtomicBool`; the client restart clears it). This refusal carries `error_with_data` `{"trace_latched": true}`.
- Result `{ "hit": bool, "time": f32, "location"?: [f32;3], "normal"?: [f32;3], "actor"?: u32, "item"?: i32 }`. `hit` is the inverse of the return value (it returns true when nothing was hit). `location`, `normal`, `actor` and `item` (`+0x08`, `+0x14`, `+0x04`, `+0x24`) only when `hit`. A level (BSP) hit reports WorldInfo as `actor` and the BSP node as `item >= 0` (live L2), so callers test `item`, not `actor == 0`.
- A fault (`SehFault`) is error `-32000` with `error_with_data` carrying `to_exception_json()`, as `call_native` does, and sets the latch above. Live L2 showed a guarded fault (flags `0x3fff`, AV at `0x00914FD3`) leaves the client running; the latch covers what the live pass could not see (the `GMem` mark).
- Not in the C7 batch list.

Daemon half (AX-56):

- `WorldIo::trace_line(&mut self, start: Vec3, end: Vec3, flags: u32) -> Result<Option<TraceHit>, String>`, default `Err("trace_line unsupported".into())`. `Ok(None)` means nothing was hit. `TraceHit { time: f64, location: Vec3, item: i32, actor: u32 }`, client space. `LiveWorld` calls the bridge method; an error containing `-32601` maps to the default error text.
- Tool `client_trace_line { start: [f64;3], end: [f64;3], flags?: u32, instance? }`, client space, UE3 units. **LEASED**, in `NO_CLAIM` (a read). It returns the DLL result unchanged. A DLL without the method gives `{"error": "trace_line needs a lab DLL with trace_line (AX-55)"}`.
- Ledge guard in `client_move_to`, new argument `allow_drops?: bool` (default `false`). While walking, at most every 500 ms, it traces straight down at a point `LEDGE_AHEAD_UU` = 150 ahead of the pawn along its heading, from the pawn's z to `pawn.z - LEDGE_HALF_HEIGHT_UU - LEDGE_DROP_UU` (78 + 200), flags `0x2086`. No hit means a drop of more than 2 m: release the keys and fail with `StopKind::Ledge`, message `ledge ahead: no floor within 2 m`. Any trace error turns the guard off for that walk and the result carries `"ledge_guard": "unavailable: <error>"`; otherwise `"ledge_guard": "on"` (or `"off"` with `allow_drops`).
- Self-check at walk start (terrain is untested live): before the first step, one trace straight down under the pawn over the same span. No hit means the trace cannot see this map's floor, so the guard is off for that walk with `"ledge_guard": "unavailable: no floor under pawn"`, and an unseen floor can never stop a walk as a false ledge. AX-64 U4 runs the guard on an outdoor map to say whether terrain answers `0x2086`.

## C15 client_watch

Tool `client_watch { action: "start" | "read" | "stop", period_frames?, cap?, since?, max?, instance? }`, LEASED, in `NO_CLAIM`. It forwards to the C6 methods and returns their results unchanged, plus `"pulled": <samples kept>` on `read`. The daemon keeps the samples it read in `crates/lab/src/supervisor/watch_store.rs`, `WatchStore` (a ring of 4096 samples per instance, with each sample's `ms` mapped to daemon wall ms through the bridge `meta.mono_ms` of the read). C12 detectors read the store; nothing else does. A DLL without the methods (`-32601`) gives `{"error": "watch needs a lab DLL with watch_start (AX-24)"}`.

## C16 client_ui_clickables

Tool `client_ui_clickables { root?: str (default "Root"), include_hidden?: bool (default false), limit?: 1..=500 (default 200), cursor?: u32 (default 0), instance? }`, OPEN (so `claim()` gives `NoClaim`; never also list it in `NO_CLAIM`, which AX-11's test forbids). One Lua walk per call from `root` through `getChildCount`/`getChildAtIdx`, depth-first in child-index order, starting from `getWindow(root)` (live L8: the `CEGUI` Lua global is not exposed to the bridge, the global `getWindow` is, so the walk never touches `CEGUI.*`). The live client has 21 416 windows under `Root` (L8, hidden ones included), so one call never walks them all: paging and a visit cap below. Result:

```json
{ "windows": [ { "name": "VendorWin/__auto_closebutton__", "type": "TaharezLook/Button",
                 "text": "X", "rect": [x, y, w, h], "visible": true, "enabled": true,
                 "auto": true, "parent": "VendorWin" } ],
  "truncated": false, "visited": 412, "next": null }
```

- `name` is the full CEGUI name, which `getWindow(name)` accepts; `auto` is `name.contains("__auto_")`.
- `type` from `getType()`, omitted when the binding lacks it (the walk checks with `type(w.getType) == "function"`).
- `rect` from `getUnclippedPixelRect` in client pixels; omitted when absent.
- Hidden windows and their children are skipped unless `include_hidden` (the visible-only prune is what keeps a normal call small).
- Paging: `cursor` is a walk position, the number of windows the walk order puts before this page (pruned hidden subtrees are not in the order unless `include_hidden`). The walk steps past the first `cursor` positions reading only `getChildCount`/`getChildAtIdx`, then inspects windows from there. It stops at `limit` returned windows or `VISIT_CAP` = 2000 inspected windows, whichever comes first, and sets `truncated: true` with `next` = the walk position it stopped at; a finished walk gives `next: null`. `visited` is the windows this call inspected. The UI can change between pages; a page is a snapshot of one frame, and the caller treats `next` as a hint, not a stable key.

## C17 Login-screen server combo (AX-03)

Today `lab_login` never touches the login screen's server combo. The client's `Login.lua` `LoginMod.startup()` preselects the first `pairs()` entry of `LoginMod.servers` (hash order, not file order, so it can be a dead server) and hides the combo when it has one row or none; `onLoginClicked` logs into `LoginMod.servers[Login_ServerTypeCombo:getSelectedItem():getText()]`. The lab's `pick_server` acts later, on the post-login shard list (`ServerSelect_List`), and `login_servers::login_url` mints the telemetry token for the account file's `server` row; so the client can log into one server while the lab minted for another.

File `crates/lab/src/supervisor/flows/login_combo.rs` (new; `flows/login.rs` is 657 lines and gets at most 10 new lines):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComboPick { Skip, Use(String), Error(String) }

/// Pure. `rows` are the combo's item texts in index order; `want` is the
/// resolved login's server name (request over `lab-account.json`).
pub fn decide_combo(rows: &[String], want: Option<&str>) -> ComboPick;

/// Lua returning the combo's item texts, one result string per row
/// (`run.lua` results), or the single string `"no_combo"`.
pub fn combo_rows_chunk() -> String;

/// Lua that selects the row whose text is `name` and returns the read-back
/// `getSelectedItem():getText()` (or `"none"`).
pub fn select_combo_chunk(name: &str) -> String;
```

`decide_combo` mirrors `login_servers::pick`:

| Rows | `want` | Result |
|---|---|---|
| 0 (or `"no_combo"`) | any | `Skip`: nothing to select; the login proceeds as today and the result carries `login_server: null` |
| 1 | any | `Use(<that row>)`, whatever the name (one server is the only choice) |
| 2+ | a name matching a row (case-insensitive) | `Use(<the row's own text>)` |
| 2+ | a name matching no row | `Error("login server '<want>' is not on the login screen; rows: <a>, <b>")` |
| 2+ | `None` | `Error("several login servers and no server in the request or lab-account.json; rows: <a>, <b>")` |

Never the silent preselect when there are several rows. Both chunks run only when `login_steps` found the login screen (`StartScreen::Login`), before credentials are typed; every widget call is inside `pcall`. A client that skipped the login screen (already at server or character select) gives `login_server: null`. `select_combo_chunk` finds the item with `Login_ServerTypeCombo:findItemWithText(name, nil)`, falling back to a loop over `getItemCount()` / `getListboxItemFromIndex(i)` comparing `getText()`, then runs `clearAllSelections()`, `setItemSelectState(item, true)` and `setText(name)`, and returns the read-back `getSelectedItem():getText()`. The login fails with `login server combo: selected '<got>', wanted '<name>'` when the read-back differs. The result of `lab_login` (and of every flow that logs in) gains `"login_server": "<name>" | null`. No tool is added, so `lease/policy.rs` does not change.

## C18 Screensaver before launch (AX-04)

`supervisor/display.rs` `ensure_display_for_launch` dismisses a screensaver only when `SPI_GETSCREENSAVERRUNNING` says one runs; when SPI reads false while a `*.scr` process runs (the 2026-10 Ribbons.scr case), the D3D probe fails and the refusal does not name the cause. AX-04 adds, in `display.rs`:

```rust
pub enum SaverAction { None, Kill(Vec<u32>), Refuse(String) }

/// Pure. `scr` is every running process whose image name ends in `.scr`
/// (case-insensitive), as (pid, image name); `secure` is SPI_GETSCREENSAVESECURE.
pub fn screensaver_action(scr: &[(u32, String)], secure: bool) -> SaverAction;
```

- No `.scr` process: `None`.
- Some, not secure: `Kill(pids)`; the existing `dismiss_screensaver` path kills them (the existing policy: a non-secure screensaver is killed today), then the D3D probe runs.
- Some, secure: `Refuse("screensaver <name> (pid <pid>) is running and locks the session; unlock the lab machine")`.

`ensure_display_for_launch` enumerates `*.scr` processes **before** the D3D probe, whatever SPI says, and acts on `screensaver_action`. The enumeration is the Toolhelp process snapshot `display.rs` already walks in `sgw_process_pids` (factor it into `win::process_entries() -> Vec<(u32, String)>` plus a pure `scr_entries(entries) -> Vec<(u32, String)>`), **not** `process::running_pids_where`: that one lists only processes with a top-level window on the daemon's desktop, and a running screensaver draws on its own desktop, which is a likely reason today's path misses it. `dismiss_screensaver` kills the pids `screensaver_action` returns. When the probe still fails, `describe_d3d_failure`'s message names any `.scr` still running. `lab_client_start` and `lab_client_restart` carry the refusal text in their error unchanged. No tool is added.
