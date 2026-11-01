# Lab evidence: how the lab tools prove what they did

> **Type:** How-to and Reference
> **Audience:** Lab coordinators and the people who write `lab-driver` briefs
> **Last updated:** 2026-11-01
> **Companions:** [Live research lab guide](live-research-lab.md) (tools, lease, rulebook), [lab ADR](../architecture/live-research-lab.md), [automated UAT](automated-uat.md) (how a UAT row is graded), [lab-autopilot contract](../analysis/lab-autopilot/contract.md) (the exact keys and codes), [lab-autopilot ledger](../analysis/lab-autopilot/README.md) (decisions D-AX1..D-AX18)

A lab tool that clicks, walks or types used to answer "ok" when it had only *sent* the input. This page tells you how to read the answer it gives now. The short version: **a tool's word is its `evidence.verdict`.** The prose around it (`ok`, `arrived`, `counts_as_native_pass`) is a claim until the verdict backs it.

Every key, code and number below comes from the [contract](../analysis/lab-autopilot/contract.md) and `crates/lab/src`; if this page and the code disagree, the code wins and this page has a bug.

## 1. Verdicts

A judged tool returns one top-level `evidence` object, mirrored as a top-level `verdict` string for terse readers:

```json
{
  "verdict": "denied",
  "evidence": {
    "verdict": "denied",
    "why": "no_hover",
    "hint": "The cursor was not over the target when it clicked. Find the entity again and retry.",
    "checks": [
      { "name": "hover_verified", "source": "client", "ok": false, "required": true }
    ]
  }
}
```

| Verdict | Means | What you report |
|---|---|---|
| `confirmed` | Every required check read back as true. | The action worked. |
| `denied` | A required check read back as false. | The action did not work. Quote `why` and `hint`. |
| `inconclusive` | Nothing could be read back, or the read cannot be trusted. | Not a pass and not a fail: say INCONCLUSIVE. |
| `unjudged` | The tool does not judge its result at all. | A claim, not a fact. Treat it like `inconclusive`. |

**How `judge` decides.** The checks are walked in order. The first *required* check with `ok: false` makes the result `denied`, and its `why` is the check's own code (or `no_effect` if it has none). Else the first required check with no `ok` (not measured) makes it `inconclusive` (`no_post_read` by default). Else, with no required checks at all, it is `inconclusive` with `no_evidence`. Otherwise it is `confirmed`. A check with `required: false` never changes the verdict; it is there for you to read. A missing `ok` means "not measured", not "false".

**The gate.** The daemon stamps every tool result after the tool returns, so a tool cannot overstate itself:

- A tool that is not confirmed cannot keep `counts_as_native_pass: true` (the UI tools spell it `native_pass`). The gate sets it `false` and records the original under `claimed`, for example `"claimed": {"counts_as_native_pass": true}`.
- A `denied` result that says `effect_ok: true` is flipped to `false`; an `inconclusive` one loses `effect_ok` entirely. Either way the old value survives under `claimed.effect_ok`.
- A tool the daemon lists as judged but that returned no evidence gets `inconclusive` / `no_evidence`. Read tools (`client_entity_find`, `client_watch`, `client_trace_line`, lease and UAT tools, and the like) claim no effect and pass through untouched.
- Errors pass through unchanged.

**Unjudged tools.** Waves 1-3 left some tools unjudged on purpose, with their `effect_ok` untouched (D-AX13, "legacy until the strict switch"). The strict switch is now on (`STRICT` in `proof/gate.rs`): an unjudged tool's `effect_ok` is removed like an `inconclusive` one, and the UAT runner reads `unjudged` as inconclusive. Rule for you: if the verdict is `unjudged`, never write "it worked" from that tool alone.

**How a UAT row is graded (D-AX1).** An action that comes back `inconclusive` makes its UAT row UNVERIFIED, unless a required clause on that row passed on evidence of its own. A row never PASSES on an action nobody saw work, and never FAILS on missing evidence. See [automated UAT](automated-uat.md) for the row shapes.

## 2. Why codes

`why` says what the verdict rests on. The `hint` text is fixed per code; the right-hand column is what a coordinator does about it.

| `why` | Verdict it comes with | Do this |
|---|---|---|
| `confirmed` | `confirmed` | Nothing. Cite it. |
| `not_delivered` | `denied` | The game never read the input. Check focus (`client_input_focus`), retry once. |
| `ignored` | `denied` | The game read it and did nothing: a dialog, a cast, a stun or UI focus is in the way. Read `client_ui_state`. |
| `no_effect` | `denied` | The action ran, the expected change did not happen. Read the state yourself and report the miss. |
| `out_of_range` | `denied` | Move closer (interact range is about 5 m) and retry. |
| `no_hover` | `denied` | The cursor was not over the target. Find the entity again, retry. |
| `stale_read` | `inconclusive` | The read-back came from the same frame as the action. The tool already re-read up to 3 times; read once more yourself. |
| `no_post_read` | `inconclusive` | Nothing was read back. Read the state you expected to change. |
| `server_unreachable` | `inconclusive` | The server could not be asked. Report INCONCLUSIVE; do not fall back to the client alone. |
| `server_disagrees` | `denied` | The client shows the change and the server does not. File it as a bug, not a pass. |
| `not_yet_judged` | `unjudged` | See section 1. |
| `no_evidence` | `inconclusive` | A judged tool returned none. Report INCONCLUSIVE and file the tool. |
| `cooling_down` | (refused call) | The same call failed moments ago. Wait `retry_after_ms` or change the arguments. |
| `delivered` | `inconclusive` | The game received the input (the DirectInput counter rose). Whether it did anything is not judged: read the state. |
| `posted_only` | `inconclusive` | A key or button was posted to the window. Nothing can read back that the game used it. Read the state. |
| `timeout` | (reserved) | No tool sets this code today. A walk that runs out of time reports `stop.kind: "timeout"` (section 4). |
| `stalled` | (reserved) | No tool sets this code today. A walk that stops making progress reports `stop.kind: "stalled"` (section 4). |
| `already_true` | `inconclusive` | The expected state held before the action, so the read-back proves nothing. Start from a state where it does not hold. |

**Cooldown.** After the same call (same tool, instance and arguments, ignoring `lease_id`, `instance`, `verbose` and `fields`) fails, an identical retry inside `min(8000, 500 * 2^(n-1))` ms is refused without running, as an `is_error` result with `why: "cooling_down"` and `retry_after_ms`. A failure is an MCP error, `is_error: true`, or a `denied` verdict. A success clears it; an idle key is forgotten after 60 s. Open (unleased) read tools, the `lab_lease_*` and `lab_uat_*` tools, `lab_client_start/stop/restart/status` and `lab_recover` are exempt, and so is the UAT runner. The point is to stop a coordinator hammering a call that cannot work; change an argument or read state first.

## 3. Server corroboration

Two actions ask the server as well as the client, and a `confirmed` there means both agree (D-AX11):

- `client_move_to`: `client_arrived` (client pose within the arrival radius) and `server_arrived` (the server's position of the player within the radius plus 1 m) are both required. A third required check, `no_rubber_band`, fails when the client walked at least 1 m and the server moved the player 0.5 m or less. A server that cannot be asked gives `server_unreachable`; a server that disagrees gives `server_disagrees`.
- `client_minigame_play` for minigames that are not placeholders: the `server_outcome` check requires the outcome the server recorded to be a victory.

`lab_recover` `unstick` may also carry an optional `server_moved` check.

**What this does not cover.** Every other `confirmed` rests on client reads: window visibility, inventory diffs, hover ids, pose, the stat panel. The server was not asked. Most of the time that is enough; when the claim is about the world (a vendor purchase, a mission step), confirm it on the server side with the server lab tools as well. D-AX11 limited corroboration to the two actions that have a cheap server-side record today.

## 4. Stop reports and recovery

A walk that cannot finish fails with a `stop` object next to the error, so you do not have to guess why:

```json
{ "stop": { "kind": "stuck", "step": "leg_0", "seconds": 12.4,
            "tried": ["jump", "input_probe"],
            "player": { "at": [10.0, 5.0, 3.0], "now": [10.2, 5.0, 3.0], "moved_m": 0.2 },
            "suggest": ["unstick", "release_input"] } }
```

`player` positions are server metres. `tried` is what the tool already did, in order. `suggest` names [`lab_recover`](#recovery-recipes) recipes, best first.

| `kind` | Means | `suggest` |
|---|---|---|
| `stuck` | The pawn does not move. | `unstick`, `release_input` |
| `timeout` | The deadline passed. | `unstick` |
| `ignored_input` | The game never read the keys (input counters flat). | `release_input`, `close_dialogs` |
| `stalled` | Progress stopped. | `unstick` |
| `blocked_ui` | A dialog or window covers the view. | `close_dialogs` |
| `ledge` | The ledge guard saw no floor ahead (section 8). | none: look at the map and walk round |

### Recovery recipes

`lab_recover { recipe, instance? }` (leased) runs one recipe and reads back whether it worked. It is judged: a recipe that could not confirm its effect is `inconclusive`.

| `recipe` | Does | Confirmed when |
|---|---|---|
| `release_input` | Lets go of every key, mouse button and DLL hold. | `input_status` shows nothing held. |
| `close_dialogs` | Escape, up to 3 times, reading the UI state after each. | No dialog is open. |
| `recenter_camera` | The `client_camera` path with pitch 0. | Pitch within 5 degrees of 0. |
| `unstick` | The walker's unstick sequence. | The player moved at least 0.5 m. |

### Resuming a UAT run

`lab_uat_run` takes `resume_from` with a `run_dir`: `"<section>/<row>"` skips the rows before that one and runs it and the rest; `"auto"` resumes at the first row with no entry in the run's manifest. Skipped rows keep their entries and are not graded again. It is exclusive with `rows`, and an unknown row is an error. Use it after a stop, once `lab_recover` got the client back to a state the next row expects.

## 5. Clicks and items

A click alone proves nothing, and the tools say so. `client_ui_click` and `client_window_click` come back `inconclusive` / `posted_only` unless you hand them a post-condition:

- `expect_visible: "<window>"`: the click is judged on that window being visible afterwards.
- `expect_hidden: "<window>"`: judged on that window being hidden.
- `expect_timeout_ms`: how long to wait for it.

`client_ui_sequence` takes no `expect_*`. It is judged by its `wait_window` steps (the window appeared or went), its drags (the item moved) and its typed edit boxes (the text is in the box). A click or key step proves nothing, and a sequence made only of those is `inconclusive` / `posted_only`. To prove a click, use `client_ui_click` or `client_window_click` with `expect_visible` / `expect_hidden`, or end the sequence with a `wait_window` step.

**Finding the name to click.** `client_ui_clickables { root?, include_hidden?, limit?, cursor? }` (a read) lists the clickable windows under `root` (default `Root`) with their full CEGUI `name`, `type`, `text`, pixel `rect`, `visible` and `enabled`. A name containing `__auto_` is a stock, unnamed child (`VendorWin/__auto_closebutton__`); pass the full name to a click. The client has about 21 000 windows, so a call inspects at most 2000, lists visible ones only unless you ask, and sets `truncated: true` with `next` when there is more: pass `next` back as `cursor`. A page is a snapshot of one frame; treat `next` as a hint.

**What a direct widget click is not.** There is no tool that fires a widget's click event directly. It would skip the input path a player uses, which is what a UAT is testing, so it could count as N3 at best and was not built (D-AX7). If you call `getWindow(name):fireEvent(...)` yourself through `client_lua_eval`, the result is a probe, not a native pass.

Item tools check the world after the action: `client_item_action` and `client_drag_drop` compare inventory snapshots (`inventory_changed`, `moved`, `items_arrived`, `quantity_matched`, `loot_taken`), and `client_vendor_buy` reads the purchase back. A use that left no change is `denied`. A `double_use_risk` check flags an item whose use could have fired twice.

## 6. Watching a run

### `client_watch`

`client_watch { action: "start" | "read" | "stop", period_frames?, cap?, since?, max? }` (leased, a read) runs a per-frame sampler inside the lab DLL: pose, rotation, hover id, controller mode, buttons, game time. One watch runs at a time.

- `start`: `period_frames` 1..60 (default 1), `cap` 16..4096 (default 1024). Returns `started_frame` and `replaced`.
- `read`: samples since index `since`, up to `max` (1..1024, default 256). Returns `samples`, `next`, `dropped`, `running`, and `pulled` (samples the daemon kept). It moves no cursor of its own: pass `next` back as `since`.
- `stop`: returns `samples_taken` and `dropped`.

A sample is `{ i, f, ms, t?, pos?, rot?, hover?, mode?, buttons?, actors? }` in client space (UE3 units). A field that could not be read is left out; on a client build the DLL does not recognise, a sample is just `{i, f, ms}`. The ring overwrites its oldest sample and counts `dropped`. The watch stops when the daemon disconnects. The daemon keeps up to 4096 samples per instance for the detectors.

A DLL without the method gives `watch needs a lab DLL with watch_start (AX-24)`.

### `lab_detectors_read`

`lab_detectors_read { instance?, since_ms? }` (open, no client call) runs pure detectors over what the daemon already holds and returns `{ watch, window_ms, fired: [{ name, at_ms, frame?, detail }] }`. `since_ms` defaults to 120 s before the call.

| Detector | Fires when |
|---|---|
| `hitch` | Watch: a frame gap over 250 ms per frame (with `period_frames` above 1 this is the mean over the gap, so a single long frame can hide; use 1 to catch it). No watch: two heartbeats at least 250 ms apart with the same `tick_count`. |
| `teleport` | The pawn moved more than 30 m within 500 ms. |
| `long_fall` | z falling for more than 3 s. |
| `out_of_world` | z below -100000 UE3 units. Provisional (D-AX6): no world floor has been measured yet. |
| `nan` | A NaN or infinite position or rotation. |
| `stuck_input` | A key held 3 s while the pawn moved under 0.2 m. |
| `actor_growth` | Actor count up more than 50% within 60 s. Only when samples carry `actors`. |
| `log_error` | The same tool errored three times in 60 s. |

**What runs without a watch.** Only `hitch` (from heartbeat gaps) and `log_error`, and `watch` is `false` (no samples pulled). Start a watch before a walk or a fight you want checked. The detectors only see samples you pulled, so call `client_watch` `read` (passing `next` back as `since`) during the run and once more at the end, often enough that the ring (`cap`, default 1024) does not drop samples. Then read the detectors, then `stop` the watch. `client_watch` `start` clears the daemon's copy. A detector that fired is a lead, not a verdict: look at `detail` and the screenshot before you call it a bug. There is no pitch-clamp detector; `client_camera`'s own evidence covers a stuck pitch.

## 7. Frames

The DLL stamps every bridge reply with `meta { frame, mono_ms, game_t? }`, and the daemon compares the frame of a read with the frame of the action it follows. A read-back taken fewer than 2 frames after the action is **stale**: its check goes unmeasured with `why: stale_read`, and the tool re-reads (3 times, 50 ms apart) before keeping it. A reply with no frame (an older DLL) is *unstamped*: the measured result stands and the check's `after` carries `"unstamped": true`.

- **Screenshots name their frames.** `lab_screenshot` and `lab_screenshot_region` say `frames <from>..<to>` in their text, and `lab_pixel_probe` carries `frames: [from, to]`. They come from a heartbeat before and after the capture, so the picture is somewhere in that span, not at a point. A wide span means the client was busy; capture again before you trust a fast-moving scene. No `frames` means a heartbeat failed, not that the capture did.
- **One frame, several reads.** `client_batch` runs literal `mem_read` and `lua` steps inside a single client frame when every step is one of those, and its result carries `frame`. Use it when two values must come from the same instant (a position and a hover id). A batch with a `wait`, `call_native` or other step runs step by step and carries no shared frame.
- **Capture stays PrintWindow** (D-AX2). Capturing the back buffer inside the client needs a deeper render hook; that is an open owner decision, not a missing feature.

## 8. Movement

Walking is the part of the lab most likely to fail slowly, so it has its own safety.

**Key holds with a lease.** A held key is a hold in the lab DLL with a lease (10 s by default, clamped 200 ms to 10 s, D-AX8). The daemon renews every hold while it lives, once a second, and the DLL lets go when the lease runs out. So a raw `client_input_key` `down` cannot outlive both the daemon and its lease. At most 16 holds exist at once. `client_input_key` `down`, `up` and `tap` all go through this path; `tap` is hold, sleep, release.

**The key fail-safe.** What lets go of keys and buttons:

- A daemon drop: the connection drop makes the DLL release every hold and the DirectInput state. No daemon action is needed.
- A lease ending (release, expiry or takeover): the daemon releases all input on that instance, including the mouse buttons it posted down.
- `client_input_release` and the `release_input` recipe: the one release path.

What it does *not* cover. A DLL without the hold method runs in fallback mode and posts keys the old way: a down stays down until `client_input_release` or a lease end. And a daemon *crash* sends no button-up, so a posted mouse button can stay down. After a crash, run `lab_recover release_input`.

**Coast and arrival.** The pawn keeps moving after `W` goes up. `client_move_to` learns that coast (a 0.12 s prior, updated from each measured release, capped at 0.6 s), releases `W` early by `speed * coast_s`, waits out the coast, and calls the walk arrived only where the pawn **settled**. Each final leg reports a `coast` object (`coast_s`, `samples`, `lead_m`, `settled_m`). If a walk "arrives" short or long, read `coast` first.

**The ledge guard.** While walking, at most every 500 ms, `client_move_to` traces straight down at a point 1.5 m ahead along the pawn's heading, from 1 m above the pawn's height down to 2.78 m below it. No floor means a drop of more than 2 m: it lets go of the keys and fails with `stop.kind: "ledge"` and the message `ledge ahead: no floor within 2 m`. Pass `allow_drops: true` to walk off a ledge on purpose. The result always says what the guard did in `ledge_guard`:

| `ledge_guard` | Means |
|---|---|
| `on` | The guard ran. |
| `off` | You passed `allow_drops`. |
| `unavailable: <reason>` | The guard could not run for this walk, and the walk was not stopped by it. |

Before the first step the walk traces down under the pawn once. No hit means the trace cannot see this map's floor (outdoor terrain did answer a straight-down `0x2086` trace in the one AX-64 run, so on flat ground a miss means the pawn is not over floor, not that the map is blind), so the guard turns itself off with `unavailable: no floor under pawn` rather than stop on a floor it cannot see.

**Known issue (AX-64, 2026-11-01):** on open start-area ground the guard stopped one honest walk with `ledge` ('no floor within 2 m') and the walker stopped another `stuck`; only 1 of 3 honest 10 m walks confirmed. If a walk stops `ledge` on level ground, retry once, then report it with the `stop` object. See finding F3 in [uat-ax64.md](../analysis/lab-autopilot/uat-ax64.md).

**`client_trace_line`.** `client_trace_line { start, end, flags? }` (leased, a read, client space in UE3 units) asks the engine's own collision check. `flags` is `0x2086` (level geometry, the default) or `0x20bf` (also pawns); anything else is refused, because other values fault. It returns `hit`, `time`, and `location`, `normal`, `actor`, `item` when it hit. A level hit reports the BSP node as `item >= 0`, so test `item`, not `actor == 0`. The tool refuses with `trace_line unavailable: ...` when the fingerprint gate failed, the world is not ready (a zone change swaps the pointer, so wait and retry), or an earlier trace faulted. **A fault latches `trace_line` off until the client restarts**: the engine may leave an allocator mark unrestored mid-call, so the DLL will not risk a second call. That refusal carries `{"trace_latched": true}`; restart the client with `lab_client_restart`. Because the ledge guard shares the trace, a latched trace turns the guard `unavailable` for later walks.

## 9. The blind describer

When a decision hangs on what a screenshot shows (is a dialog open, which window has focus, did the vendor list render), do not read the picture while holding your hypothesis: you will see what you hoped for. Send the image paths to the `frame-describer` agent with neutral questions and nothing about the expected outcome.

- **Neutral:** "Is there a window titled \"Vendor\"? Quote any button text." The answer is `yes`, `no` or `cannot tell`, plus what it saw, plus the frames line you gave it.
- **Leading:** "Did the vendor window open after the click?" It names the outcome you want and the describer starts agreeing. Do not ask that.
- Give it the `frames a..b` text from the screenshot result, so its answer names the span.
- `cannot tell` is a real answer. A black, cut-off or blurred frame is a reason to capture again, not to push for a yes.
- Use a describer verdict as evidence for one *clause* of a finding. It is not a verdict in the sense of section 1; the tool's own `evidence` still decides whether the action worked.

---

Concepts adapted from ramenvr/autopilot (Apache 2.0): the evidence gate, stop reports, the watch, the detectors and the blind describer. No code or text was copied.
