# lab-autopilot wave 3: world evidence, stops, recoveries

> **Type:** Reference
> **Audience:** Packet-coders and packet-reviewers of wave 3
> **Last updated:** 2026-10-31
> **Companions:** [work packets](work-packets.md) (rules R1-R10, checks), [contract](contract.md), [ledger](README.md), [wave 2](packets-wave2.md)

Wave 3 makes the world tools judge themselves:

- `client_world_click` and `client_target` say whether the click did what it claimed;
- `client_move_to` asks the server where the player ended up and explains each stop;
- `client_camera` checks that the view moved.

It also adds the `lab_recover` tool (C11), the quick-fail cooldown (C9), and (AX-27) the release of every key and button when a lease ends. Everything is in `crates/lab`; nothing touches the DLL.

Every packet carries R1-R10 of [work-packets.md § Rules](work-packets.md#rules-every-packet-carries). Paths are under `crates/lab/src/`.

**Dispatch:**

1. AX-30 and AX-27 first and at once, both from `lax-w3` (created by `ops` from main once the wave 2 PR merged). They share no file.
2. When AX-30 and AX-27 have merged into `lax-w3`, dispatch AX-31, AX-32, AX-33, AX-35 and AX-36 at once, each at that head.
3. AX-34 waits until AX-32 has merged (both edit the movement files).

**Merge order:** AX-30, AX-27, AX-31, AX-32, AX-33, AX-34, AX-35, AX-36. One PR from `lab-autopilot/wave3`. Eight packets: AX-27 is small and the wave reviewer reads it beside AX-35, which calls the same release helper.

**The pattern every judged world tool follows** (AX-31..AX-34):

1. Take the action frame after the action: `let action_frame = io.frame().await;`.
2. Call `fresh_wait::wait_fresh(io, action_frame)` (AX-22) before the read that proves the effect.
3. Build the `Check` list in a pure function in a sibling `*_proof.rs` file that takes plain values (no `WorldIo`), so the tests cover every branch without a `Sim` run.
4. Call `steps.judged(Evidence::judge(checks))` before `steps.finish`.

A `Stale` freshness turns the deciding check into `ok: None` with why `stale_read`. An `Unstamped` one keeps `ok` and goes through `proof::fresh::mark_unstamped` on `after` (C4).

Register each tool in `proof/claims.rs` `JUDGED` in the same packet that makes it judged (R9).

### AX-27 Release every key and button when a lease ends

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-27-lease-release` · **Worktree:** `lax-27` (at `lax-w3`)
- **Commit:** `fix(lab): let go of every key and mouse button when a lab lease ends (AX-27)`
- **Rules:** R1-R4, R6, R7, R8, R9 (no new tool: say so). Contract: [C5](contract.md#c5-key-holds-with-a-lease) "Key fail-safe", last bullet.

Why: a session that loses its lease (release, 10-minute expiry, takeover) can leave keys held and posted buttons down for the next holder. Wave 2 gave the daemon one release path (`release_all_input`) and a 1 s guard loop; this packet points the loop at the lease.

Files and changes:

1. `lease/mod.rs` (484 lines; at most 20 added):
   - `pub fn current_id(&self) -> Option<String>`: lock, `expire_if_due(now_ms())`, the current lease's id.
   - `pub fn ended_how(&self, id: &str) -> Option<&'static str>`: the `how` of the `history` entry with that `lease_id`.
   - Nothing else changes; expiry still does not notify (the guard loop's 1 s tick sees it).
2. `supervisor/hold_book.rs` (AX-23): `buttons: BTreeSet<usize>` with `button_down(b)`, `button_up(b)`, `take_buttons() -> Vec<usize>`; `clear()` leaves `buttons` alone.
3. `supervisor/input.rs` `post_button` (l.338; at most 8 lines added): after a successful `"down"` post, `self.holds.lock().await.button_down(button)`; after `"up"`, `button_up(button)`. `"click"` changes nothing.
4. New pure `pub fn lease_ended(seen: Option<&str>, now: Option<&str>) -> bool` in `supervisor/input_release.rs`: true when `seen` is `Some` and differs from `now` (a new lease after an old one counts; `None` to `Some` does not).
5. `supervisor/input_release.rs`:
   - `release_all_input` (AX-26): at the `// AX-27` line, for each `take_buttons()` entry, `self.post_button(b, "up", at, None)` with `at` from `self.cursor().await` or `(0, 0)` when that fails; errors are logged and ignored.
   - `spawn_input_guards`: keep `seen = sup.leases().current_id()` and a `sup.leases().subscribe()` receiver; each turn `tokio::select!` on `changed()` and the 1 s sleep, renew holds, then read `now = current_id()`; when `lease_ended(seen, now)`, call `sup.release_all_input("lease_end")` and log `tracing::info!(target: "lab.input", how = sup.leases().ended_how(old).unwrap_or("?"), "lab.input.released_on_lease_end")`; set `seen = now`. Grep how `Supervisor` exposes its `LeaseBook` (`main.rs` l.141 calls `supervisor.leases()`).
6. Tests: pure cases in `#[cfg(test)] mod tests` of `input_release.rs` or its `_tests.rs`; book cases in `hold_book.rs`'s tests; lease cases in `lease_tests.rs` (the lease's own sibling test file).

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `lease_ended_cases` | `(Some a, None)` true; `(Some a, Some b)` true; `(Some a, Some a)` false; `(None, Some a)` false; `(None, None)` false | step 4 |
| `current_id_sees_expiry` | `acquire_at` with ttl 30 at t=0, then a `current_id` read after the clock passes expiry is `None` (use the `_at` variants or the injected clock the lease tests already use) | step 1 |
| `ended_how_names_takeover` | acquire a, force-acquire b: `ended_how(a)` is `taken_over`; release b: `ended_how(b)` is `released` | step 1 |
| `buttons_are_tracked_until_up` | `button_down(0)`, `button_down(1)`, `button_up(0)`: `take_buttons()` is `[1]`, then empty | step 2 |
| `release_lets_go_of_posted_buttons` | on `fake_bridge::supervisor` with a stubbed window: a tracked button 1, then `release_all_input("t")`: `take_buttons()` is empty afterwards and the release did not error on a failing cursor read | step 5 |

Checks: L1, L2. Report: the `lease/mod.rs` and `input.rs` line counts, and "no tool added; policy.rs unchanged". The live check is AX-64's lease-end row.
### AX-30 Mechanical splits: click result, move probe, unstick

- **Host:** windows · **Agent:** rust-gameserver-dev · **Branch:** `lab-autopilot/AX-30-splits` · **Worktree:** `lax-30` (at `lax-w3`)
- **Commit:** `refactor(lab): move click results, the input probe and unsticks to their own files (AX-30)`
- **Rules:** R1-R4, R6. No behaviour change: the diff moves code and adds `mod` lines and calls.

Files and changes:

1. `supervisor/world/click.rs` (733 lines):
   - Move the result-building tail of `click_and_observe` into a new `supervisor/world/click_result.rs`, as `pub(super) fn click_result(...) -> Result<Value, (String, Value)>`. That tail is l.680-728: the `opened`/`closed` lists, the local `evidence` JSON, the `!ok` error message and the final `out`. It takes what the tail reads: `expect`, `entity`, `&before`, `&after`, `ok`, `settle`, `button`, `range_m`, `found`, `(x, y)`, `hover_verified`, `face_json` and `fallback`. Bundle them in a `pub(super) struct ClickTail<'a>` if there are more than 7.
   - `click_and_observe` maps `Err((msg, ev))` to `steps.fail_with(io.now_ms(), "observe", msg, ev)` and `Ok(out)` to `steps.finish`.
   - Declare `mod click_result;` in `supervisor/world/mod.rs`.
   - Target: `click.rs` under 690 lines.
2. `supervisor/world/movement.rs` (595 lines):
   - Move the input probe block (l.~403-476, W then S, ending in the "blocked ahead" / `input_probe` failures) into a new `supervisor/world/move_probe.rs`, as `pub(super) async fn input_probe<W: WorldIo>(io: &mut W, steps: &mut Steps, ...) -> Result<ProbeResult, WorldError>`. `ProbeResult` carries whatever the loop uses after the block (`prev`, the moved distance).
   - Keep every failure message and step name byte-for-byte.
3. `supervisor/world/movement.rs`:
   - Move the `Unstick` match (l.477-500) into a new `supervisor/world/unstick.rs`, as `pub async fn run_unstick<W: WorldIo>(io: &mut W, u: Unstick) -> Result<(), String>` (Jump: key down, 80 ms, key up; strafes: key down, `STRAFE_MS`, key up).
   - Move `STRAFE_MS` with it.
   - The loop calls it and maps `Err` to `steps.fail(io.now_ms(), "unstick", e)`.
   - AX-35 reuses `run_unstick`.
   - Target: `movement.rs` under 500 lines.
4. Declare `mod move_probe; pub mod unstick;` in `supervisor/world/mod.rs`.

Tests: none new. The whole suite passes unchanged, including `move_to_tests.rs` (`assert_eq!(out["legs"][0]["unsticks"][0], "jump")`, l.213) and `click_tests.rs`.

Checks: L1, L2. Report the line counts of `click.rs`, `click_result.rs`, `movement.rs`, `move_probe.rs`, `unstick.rs` and `world/mod.rs`.

### AX-31 Click and target evidence

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-31-click-proof` · **Worktree:** `lax-31` (at `lax-w3` after AX-30)
- **Commit:** `feat(lab): world_click and target judge their clicks (AX-31)`
- **Rules:** R1-R4, R6, R8, R9. Contract: C1, C2, C4. Turns the #1660 right-click range item (fixed as a report by LH-08, #1663) into a denial.

Files and changes:

1. New `supervisor/world/click_proof.rs`:

```rust
pub(super) struct ClickFacts<'a> {
    pub expect: Expect,
    pub entity: Option<u32>,
    pub before: &'a UiSnap,
    pub after: &'a UiSnap,       // the fresh post-read (step 2)
    pub hover_verified: bool,
    pub button: u8,              // 0 left, 1 right (as click.rs uses it)
    pub range_m: Option<f64>,
    pub fresh: Freshness,
    pub read_frame: Option<u64>,
}
pub(super) fn checks(f: &ClickFacts) -> Vec<Check>;
```

   Rules, in vector order:

   | Case | Check | `ok` | `required` | `why` |
   |---|---|---|---|---|
   | right button, `range_m > INTERACT_RANGE_M` (`click_range.rs`) | `in_range` | `false` | `true` | `out_of_range` |
   | right button, `range_m <= INTERACT_RANGE_M` | `in_range` | `true` | `false` | - |
   | `Expect::Target` | `target_is_entity` | `after.target == entity` | `true` | `no_effect` |
   | `Expect::Window` | `window_opened` | any window in `after` not in `before` (`after` = the opened list) | `true` | `no_effect` |
   | `Expect::Any` | `effect_seen` | either of the two above | `true` | `no_effect` |
   | `Expect::Nothing` | `post_read` | `None` | `true` | `no_post_read` |
   | always | `hover_verified` | `hover_verified` | `false` | `no_hover` |

   - Source `Client` for all. `before`/`after` carry the compared values (target ids, or window lists).
   - `fresh == Stale` sets the deciding check (Target, Window or Any) to `ok: None` with why `stale_read`.
   - `Unstamped` passes its `after` through `mark_unstamped`.
   - `read_frame` goes on the deciding check with `with_frame`.
2. `supervisor/world/click.rs` `click_and_observe`:
   1. After the button-up, `let action_frame = io.frame().await;`.
   2. After the observe loop and the fallback, when `ok && expect != Expect::Nothing`: `let (fresh, frame) = fresh_wait::wait_fresh(io, action_frame).await;`, then read `after` once more with `ui_snap` (the post-read), and use that `after` for the checks.
   3. Call `steps.judged(Evidence::judge(click_proof::checks(&facts)))` before the success return.
   4. The failure path (`!ok`) is unchanged: it is already an error.
   - At most 20 lines added to `click.rs`; the rest goes in `click_proof.rs`. Declare `mod click_proof;` in `world/mod.rs`.
3. `proof/claims.rs`: add `client_world_click` and `client_target` to `JUDGED`.
4. `server/world.rs`: append to both tool descriptions: ` Returns evidence: confirmed only when a fresh read after the click shows the expected target or window; expect nothing is inconclusive; a right-click beyond 5 m is denied (out_of_range).`
5. Tests: `supervisor/world/click_proof_tests.rs` (pure), plus two `Sim` tests in the same file through `click::run`. Model them on `click_tests.rs`, which sets up a `Sim` with an entity and a target outcome.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `target_met_is_confirmed` | Target, `after.target == entity`, Fresh: `judge` gives Confirmed | rule table |
| `expect_nothing_is_inconclusive` | Nothing: Inconclusive, why `no_post_read` | rule table |
| `far_right_click_is_denied_out_of_range` | right, Nothing, `range_m` 9.0: Denied, why `out_of_range` (the `in_range` check comes first) | #1660 range item |
| `near_right_click_has_optional_range` | right, Target met, `range_m` 3.0: Confirmed, `in_range` present with `required: false` | rule table |
| `stale_read_is_inconclusive` | Target met, Stale: Inconclusive, why `stale_read` | C4 |
| `unstamped_marks_after` | Target met, Unstamped: Confirmed, the deciding check's `after` has `unstamped: true` | C4 |
| `sim_world_click_carries_evidence` | `click::run` on a `Sim` whose target becomes the entity: output `evidence.verdict == "confirmed"`, `counts_as_native_pass` true | step 2 wiring |
| `sim_point_click_is_inconclusive` | a point click (default `expect` nothing): `evidence.verdict == "inconclusive"` | step 2 wiring |
| `claims_lists_click_tools` | `claim("client_world_click") == Claim::Judged` | step 3 |

Checks: L1, L2. Report the `click.rs` line count.

### AX-32 move_to evidence and stop reports

- **Host:** windows · **Agent:** rust-gameserver-dev · **Branch:** `lab-autopilot/AX-32-move-proof` · **Worktree:** `lax-32` (at `lax-w3` after AX-30)
- **Commit:** `feat(lab): move_to asks the server where the player ended and explains each stop (AX-32)`
- **Rules:** R1-R4, R6, R7, R8, R9. Contract: C1, C2, C8. Decision: D-AX11 (move_to is the one tool whose server check is required).

Files and changes:

1. New `supervisor/world/move_proof.rs`, pure:
   - `pub(super) fn arrival_checks(goal_client: Vec3, end_client: Vec3, arrival_m: f64, start_server: Option<Vec3>, end_server: Option<Vec3>) -> Vec<Check>`:
     - `client_arrived`: `Client`, `ok: Some(horizontal_m(end_client, goal_client) <= arrival_m)`, `required: true`, why `no_effect`.
     - `server_arrived`: `Server`, `required: true`.
       - `end_server` `None`: `ok: None`, why `server_unreachable`.
       - Otherwise `ok: Some(horizontal_m(server_to_client(end_server), goal_client) <= arrival_m + 1.0)`, why `server_disagrees`, with `before` = the goal and `after` = the server position, both in server metres.
     - `no_rubber_band`: `Server`, `required: true`. It is present only when `start_server` and `end_server` are both `Some`, and the client moved at least 1 m. It is false when the server's end is within 0.5 m of the server's start (the client walked; the server never moved it); why `server_disagrees`.
   - `pub(super) fn stop_report(kind: StopKind, step: &str, seconds: f64, tried: Vec<String>, at_client: Vec3, now_client: Vec3) -> StopReport`:
     - converts both positions with `geometry::client_to_server`;
     - builds `StopPlayer::new`;
     - suggests per kind:
       - `Stuck`: `["unstick", "release_input"]`;
       - `IgnoredInput`: `["release_input", "close_dialogs"]`;
       - `Timeout` and `Stalled`: `["unstick"]`;
       - `BlockedUi`: `["close_dialogs"]`;
       - `Ledge`: `[]`.
2. `supervisor/world/movement.rs` `walk`:
   1. At the start, after `player_actor`, read `let start_server = io.server_position(snap.player_id).await.ok().flatten();`. Read the snapshot once for the id; `player_actor` already reads it, so return the id from it too: change its return to `(u32 /*actor*/, Pose, u32 /*player id*/)` and update its callers.
   2. At the success end, read `end_server` the same way, call `arrival_checks` for the final leg's goal and arrival radius, and `steps.judged(Evidence::judge(...))`.
   3. Add `"server_end": end_server.map(|p| p.to_json())` to the output.
   - Failures change from `fail_with` to `fail_stop`, keeping their message and the evidence JSON. Put the evidence under the stop's step: `fail_stop` takes a message; keep the evidence by calling `fail_with` first and then setting `.stop` on the `WorldFailure`, or add `Steps::fail_with_stop` in `world/mod.rs` (at most 10 lines).
   - Map each failure as follows. Use `tried` from the leg's `unsticks` list (names), plus `"input_probe"` when the probe ran.

     | Failure | Kind |
     |---|---|
     | timeout (l.~353-369) | `Timeout` |
     | the teleport-jump failure | `Stalled` |
     | `MoveOutcome::Stuck` | `Stuck` |
     | `MoveOutcome::Diverged` | `Stalled` |
     | `MoveOutcome::NoTurn` | `IgnoredInput` |
     | the input probe failures (now in `move_probe.rs`) | `IgnoredInput` (AX-34 refines them) |
3. `proof/claims.rs`: add `client_move_to` to `JUDGED`.
4. `server/world.rs` `client_move_to` description: append ` Returns evidence: confirmed only when the client and the server both put the player within the arrival radius (server +1 m); a server that cannot be asked makes it inconclusive. Failures carry stop: what happened, what was tried, and lab_recover recipes to try.`
5. Tests: `supervisor/world/move_proof_tests.rs` (pure), plus `Sim` runs in a new `supervisor/world/move_to_proof_tests.rs`, wired from `move_proof.rs` as a second `#[cfg(test)]` module. `Sim.server_pos` (AX-13) scripts the server answer; give it a way to differ between start and end (a `Vec<Vec3>` queue, or set it after the walk starts through the test's own sleep hook). If `Sim` cannot do that in 10 lines, test the rubber band only in the pure tests.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `both_agree_confirms` | client and server end at the goal: Confirmed | rule |
| `server_far_is_denied` | client at the goal, server 6 m off with arrival 1.5: Denied, why `server_disagrees` | D-AX11 |
| `no_server_is_inconclusive` | `end_server` `None`: Inconclusive, why `server_unreachable` | honesty |
| `rubber_band_is_denied` | client moved 10 m; server start equals server end: Denied, why `server_disagrees` | rubber band |
| `stuck_stop_report` | `stop_report(Stuck, ..)`: kind `stuck`, `suggest[0] == "unstick"`, `player.moved_m` in server metres | C8 |
| `sim_move_confirms_with_server` | `Sim` walk with `server_pos` at the goal: `evidence.verdict == "confirmed"`, `server_end` present | step 2 |
| `sim_move_without_server_is_inconclusive` | `Sim` default (`server_pos` `None`): `evidence.verdict == "inconclusive"`, `arrived` still true | D-AX11 |
| `sim_stuck_carries_stop` | the `Sim` wall setup `move_to_tests.rs` uses for stuck: the error JSON has `stop.kind == "stuck"` and `stop.tried` lists the unsticks | step 2 |

Checks: L1, L2. Report the `movement.rs` line count (at most 560).

### AX-33 Camera evidence

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-33-camera-proof` · **Worktree:** `lax-33` (at `lax-w3` after AX-30)
- **Commit:** `feat(lab): client_camera judges whether the view moved (AX-33)`
- **Rules:** R1-R4, R6, R8, R9. Contract: C1, C2, C4.

The camera actor pose (`camera_before`/`camera_after`) can read all zeros on the live client. So judge on the controller readout (`view_before`/`view_after`: keys `zoom`, `yaw_offset_deg` and `pitch_offset_deg`, `supervisor/world/memory.rs` l.372-381), never on the actor pose.

Files and changes:

1. New `supervisor/world/camera_proof.rs`, pure:
   - `pub(super) fn checks(req: &CameraRequest, view_before: &Value, view_after: &Value, face: &Value, absolute: Option<&Value>, fresh: Freshness) -> Vec<Check>`.
   - Rules:

     | Asked for | Check | `ok` | `required` | `why` when false |
     |---|---|---|---|---|
     | `yaw_counts` or `pitch_counts` non-zero | `look_moved` | yaw or pitch offset changed by at least 0.1 degree | yes | `ignored` (when `view_after` is pinned at the pitch limit, the check's `after` adds `"pitch_clamped": true`) |
     | `zoom`, `zoom_to` or `zoom_notches` non-zero | `zoom_changed` | `zoom` changed by at least 1 unit, or `zoom_to` met within 30 | yes | `ignored` |
     | a face target | `face_centred` | `face.centred == true` | yes | `no_effect` |
     | `pitch_deg` | `pitch_at` | `|pitch_offset_deg - want| <= ABS_TOLERANCE_DEG` | yes | `no_effect` |
     | `yaw_offset_deg` | `yaw_at` | same for yaw | yes | `no_effect` |

   - A `view_after` that is `{"error": ..}` makes every check `ok: None`, why `no_post_read`.
   - `Stale` freshness makes `look_moved` and `zoom_changed` `ok: None`, why `stale_read`.
   - No argument asked for gives no checks, so `judge` gives `no_evidence`.
   - Reuse `camera.rs`'s own wrap rule (`pinned_pitch_of`, the yaw difference helper; grep `fn wrap` or `angle_diff` in `world/`) for the yaw delta. Do not write a second one.
2. `supervisor/world/camera.rs` `run`:
   1. After the look block, `let action_frame = io.frame().await;`.
   2. Before `camera_after`, `let (fresh, _) = fresh_wait::wait_fresh(io, action_frame).await;`.
   3. Before `finish`, `steps.judged(Evidence::judge(camera_proof::checks(&req, &out["view_before"], &out["view_after"], &out["face"], out.get("absolute"), fresh)))`.
   - Clone `req` fields before they move, if needed. At most 12 lines added (`camera.rs` is 580).
3. `proof/claims.rs`: add `client_camera` to `JUDGED`.
4. `server/world.rs` `client_camera` description: append ` Returns evidence from the controller's camera readout: confirmed when every asked-for change is read back; a look that changed nothing is denied (ignored).`
5. Tests: `supervisor/world/camera_proof_tests.rs` (pure), plus one `Sim` run through `camera::run`.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `look_that_moved_confirms` | yaw 10 to 25: Confirmed | rule |
| `look_that_did_nothing_is_ignored` | yaw counts 500, offsets unchanged: Denied, why `ignored` | rule |
| `pinned_pitch_notes_the_clamp` | pitch counts only, pitch pinned at 78.75 in both: Denied, `after.pitch_clamped == true` | #1660 pitch clamp item |
| `no_args_is_no_evidence` | empty request: Inconclusive, why `no_evidence` | rule |
| `readout_error_is_no_post_read` | `view_after = {"error":"x"}` with yaw counts: Inconclusive, why `no_post_read` | rule |
| `abs_pitch_within_tolerance` | `pitch_deg` 10, readout 10.3: Confirmed; readout 12: Denied | rule |
| `sim_camera_look_confirms` | `camera::run` on a `Sim` with `yaw_counts` 200: `evidence.verdict == "confirmed"` | step 2 |

Checks: L1, L2. Report the `camera.rs` line count.

### AX-34 Ignored input and blocked UI

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-34-ignored-input` · **Worktree:** `lax-34` (at `lax-w3` after AX-32 merged)
- **Commit:** `feat(lab): tell ignored input from a blocking window (AX-34)`
- **Rules:** R1-R4, R6, R8. Contract: C2 (`not_delivered`, `ignored`), C8 (`IgnoredInput`, `BlockedUi`).

Files and changes:

1. `supervisor/world/io.rs` `WorldIo`:
   - Add a default method `async fn input_counters(&mut self) -> Option<u64> { None }`.
   - `LiveWorld` implements it from `self.sup.bridge_call("input_status", json!({}))` as `hooks.events_delivered + hooks.data_reads_mouse`, `None` when `hooks` is null.
   - At most 15 lines added.
2. New `supervisor/world/ignored_input.rs`, pure:
   - `pub(super) fn classify(visible_windows: &[String], hud: &[&str]) -> StopKind`: `BlockedUi` when a visible window is not in `hud`, else `IgnoredInput`.
   - `pub(super) const HUD_WINDOWS: &[&str]`: the set the lab already treats as always-on HUD. Grep `ui_state.rs` and `hover.rs` for the list used to say "no HUD window"; reuse that constant if it exists and is visible, and make it `pub(crate)` if needed.
   - `pub(super) fn look_delivered(before: Option<u64>, after: Option<u64>) -> Check`: `bridge` source, `required: true`, `ok: Some(after > before)` when both are `Some`, why `not_delivered` when false; both `None` gives `ok: None`, why `no_post_read`.
3. `supervisor/world/move_probe.rs` (from AX-30): when the probe fails, read `io.visible_windows()` and pick the stop kind with `classify`. Its `suggest` comes from `move_proof::stop_report`.
4. `supervisor/world/camera.rs`:
   - Around the `io.look` call, read `input_counters()` before and after, only when `look` returned `NativeLevel::RealInput` (the DirectInput path, not the native camera).
   - Push `look_delivered` into the checks, in front of `look_moved`: `camera_proof::checks` takes an extra `Option<Check>`.
   - Key presses never use the DI counters: posted `WM_KEYDOWN` does not go through DirectInput.
   - At most 10 lines added to `camera.rs`.
5. Tests in `supervisor/world/ignored_input_tests.rs`. A `Sim` probe failure with a scripted visible window: `Sim` has `visible_windows`; grep `fn visible_windows` in `sim.rs`/`sim_model.rs`.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `a_dialog_makes_it_blocked_ui` | `classify(["DialogWin"], HUD)` is `BlockedUi` | rule |
| `only_hud_is_ignored_input` | HUD windows only: `IgnoredInput` | rule |
| `counters_that_did_not_move_deny_delivery` | `look_delivered(Some(5), Some(5))`: `ok` false, why `not_delivered` | C2 |
| `no_counters_is_unmeasured` | `(None, None)`: `ok` None | honesty: an old DLL never denies |
| `sim_probe_with_dialog_stops_blocked_ui` | a `Sim` whose pawn does not move under W with `DialogWin` visible: the error's `stop.kind == "blocked_ui"`, `suggest == ["close_dialogs"]` | step 3 |

Checks: L1, L2. Report the `io.rs` and `camera.rs` line counts.

### AX-35 lab_recover

- **Host:** windows · **Agent:** rust-gameserver-dev · **Branch:** `lab-autopilot/AX-35-recover` · **Worktree:** `lax-35` (at `lax-w3` after AX-30)
- **Commit:** `feat(lab): lab_recover recipes that say whether they worked (AX-35)`
- **Rules:** R1-R4, R6, R7, R8, R9. Contract: [C11](contract.md#c11-recovery-recipes), C1, C5 (holds).

Files and changes:

1. New `supervisor/recover_recipes.rs` (not `recovery.rs`): `impl Supervisor { pub(crate) async fn recover(&self, recipe: &str) -> Result<Value, String> }`. An unknown recipe errors `unknown recipe <r>; one of release_input, close_dialogs, recenter_camera, unstick`. Each recipe records its steps in a `Vec<Value>` and builds the `evidence` with `Evidence::judge`:

   | Recipe | Does | Required check |
   |---|---|---|
   | `release_input` | `self.release_all_input("recover")` (AX-26, with AX-27's button-ups), then `bridge_call("input_status", {})` | `input_released` (`Bridge`): `input_status.held.keys` and `held.buttons` empty and `holds` empty or absent |
   | `close_dialogs` | up to 3 times: `input_key("Escape", "tap", None)` (grep the key name `keys::named` accepts for Escape), sleep 300 ms, `self.ui_state(Some(0))` | `dialog_closed` (`Client`): its `dialog` field null or absent; why `no_effect` |
   | `recenter_camera` | `camera::run(&mut LiveWorld::new(self), CameraRequest { pitch_deg: Some(0.0), ..Default::default() })` | `pitch_level` (`Client`): `view_after.pitch_offset_deg` within 5 degrees of 0 |
   | `unstick` | client pose, then `unstick::run_unstick` with `Jump`, `StrafeRight`, `StrafeLeft` until the pawn moved 0.5 m | `moved` (`Client`): moved at least 0.5 m; plus optional `server_moved` (`Server`) from `server_position` |

   - The result is `{ "recipe", "evidence", "steps" }`.
   - A failing bridge call ends the recipe with that check `ok: None`, why `no_post_read`. It is never an error: the tool reports what it could not confirm.
2. New `server/recover_tools.rs`:
   - `#[tool_router(router = recover_router, vis = "pub(super)")] impl LabServer` with `lab_recover(RecoverArgs { recipe: String, instance: Option<String> })`.
   - Use the instance-resolution pattern the other leased tools use (grep `instance` in `server/world.rs` or `server/ui/mod.rs`).
   - Description: `Run one recovery recipe (release_input, close_dialogs, recenter_camera, unstick) and read back whether it worked. Returns evidence; a recipe that could not confirm its effect is inconclusive, never a silent success.`
   - In `server/mod.rs`, add `mod recover_tools;` and `+ Self::recover_router()` next to the other routers.
3. `lease/policy.rs`: `lab_recover` in `LEASED`. `proof/claims.rs`: `lab_recover` in `JUDGED`.
4. Tests in `supervisor/recover_recipes_tests.rs` on `fake_bridge::supervisor`:
   - script `input_status`, `input_release`, `input_hold`, `lua_eval` (for `ui_state`) and the key path;
   - grep `fake_bridge::supervisor` users for how `ui_state` and `input_key` are faked;
   - `recenter_camera` and `unstick` need `WorldIo` reads through the bridge: if faking them takes more than 40 lines, test those two through a pure `recipe_checks(recipe, before, after) -> Vec<Check>` helper instead, and say so.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `release_input_confirms_on_empty_status` | `input_status` answers empty held and holds: Confirmed; the fake saw `input_release` | C11 |
| `release_input_with_a_stuck_key_is_denied` | `held.keys == [87]` after: Denied | C11 |
| `close_dialogs_stops_when_closed` | `ui_state` dialog set on the first read, null on the second: Confirmed after 2 Escapes | C11 |
| `close_dialogs_gives_up_after_three` | dialog never closes: 3 Escapes, Denied, why `no_effect` | C11 |
| `unknown_recipe_errors` | `recipe = "dance"`: error text | step 1 |
| `bridge_failure_is_inconclusive` | `input_status` errors: Inconclusive, why `no_post_read` | honesty |
| `lab_recover_is_classified` | `lab_recover` routed and in `LEASED` and `JUDGED` (the existing classification tests pass) | R9 |

Checks: L1, L2.

### AX-36 Quick-fail cooldown

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-36-cooldown` · **Worktree:** `lax-36` (at `lax-w3` after AX-30)
- **Commit:** `feat(lab): refuse an identical call that just failed, for a short backoff (AX-36)`
- **Rules:** R1-R4, R6, R8. Contract: [C9](contract.md#c9-quick-fail-cooldown).

Files and changes:

1. New `server/cooldown.rs`, pure apart from the clock:
   - `pub(super) struct Cooldown { keys: HashMap<u64, Entry>, order: VecDeque<u64> }`.
   - `Entry { failures: u32, last_ms: u64 }`.
   - `pub(super) fn key(tool: &str, args: &Value) -> u64`: strip `lease_id`, `instance`, `verbose` and `fields` from the top-level object, serialise the rest canonically (sorted keys at every level: build it with a `BTreeMap` recursion), and hash it with the tool name using `std::hash::DefaultHasher`.
   - `pub(super) fn check(&mut self, key, now_ms) -> Option<Value>` returns the C9 refusal JSON when a backoff is running: `min(8000, 500 * 2^(n-1))` after the n-th failure, measured from `last_ms`.
   - `pub(super) fn record(&mut self, key, failed: bool, now_ms)`: a failure increments; a success removes.
   - Forget keys idle for 60 s on every call. Evict the oldest past 256.
   - `pub(super) fn exempt(tool: &str) -> bool` per C9: `lease::policy::OPEN`, the `lab_lease_` prefix, the `lab_uat_` prefix, `lab_client_start`, `lab_client_stop`, `lab_client_restart`, `lab_client_status` and `lab_recover`.
   - `pub(super) fn failed(out: &Result<CallToolResponse, McpError>) -> bool`: an `Err`, `is_error == Some(true)`, or a first JSON text block whose `evidence.verdict == "denied"`. Reuse the text-block parse from `server/evidence_gate.rs`; make its helper `pub(super)` if needed.
2. `server/handler.rs` `call_tool`:
   1. Before the tool runs and when not `exempt`, lock the server's `Cooldown` (a `std::sync::Mutex<Cooldown>` field on `LabServer`, or a `OnceLock` static in `cooldown.rs` keyed by nothing else; prefer the static, so `LabServer` stays untouched) and `check`.
   2. A refusal returns `Ok(CallToolResult::error(vec![Content::text(refusal.to_string())]))`, built the way handler errors are built (grep `CallToolResult::error`), and is logged with `log_call` like any call.
   3. After the gate stamps `out`, `record(key, failed(&out), now)`.
   - At most 20 lines added to `handler.rs`.
3. Tests in `server/cooldown_tests.rs`, pure with an injected `now_ms`.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `backoff_doubles_and_caps` | after failures 1..6 the windows are 500, 1000, 2000, 4000, 8000, 8000 ms | C9 |
| `refusal_shape` | `check` during a backoff returns `error`, `why: "cooling_down"`, the C2 hint, `retry_after_ms` and `failures` | C9 |
| `success_clears` | fail, succeed, then the same call: no refusal | C9 |
| `lease_and_verbose_do_not_change_the_key` | args differing only in `lease_id`, `instance`, `verbose` or `fields` hash equal; differing in `entity_id`, they do not | C9 |
| `key_order_does_not_change_the_key` | `{"a":1,"b":2}` and `{"b":2,"a":1}` hash equal | canonical JSON |
| `idle_keys_are_forgotten` | a failure, then 61 s later: no refusal | C9 |
| `exempt_tools` | `lab_lease_acquire`, `client_ui_state` (OPEN), `lab_uat_run` and `lab_recover` are exempt; `client_world_click` is not | C9 |
| `denied_evidence_counts_as_failure` | a `CallToolResult` whose JSON has `evidence.verdict: "denied"` gives `failed == true` | C9 |

Checks: L1, L2. Report the `handler.rs` line count and the wiring lines.
