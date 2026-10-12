# LH expander B: issue #1660 items 5, 6, 7, 9 (packets LH-05..LH-08)

Worktree for all reading: `<wt>` (origin/main). Crate package name: `cimmeria-lab` (`crates/lab/Cargo.toml`). All paths below are relative to the worktree root. `<wt>` in commands is the packet's own worktree.

## Summary of root causes

| Item | Root cause | Sure? |
|---|---|---|
| 5 `client_input_mouse` ignores x/y | There are no x/y arguments at all. `InputMouseArgs` (`crates/lab/src/server/mod.rs:195`) has dx, dy, wheel, button, action, hold_ms, and `Supervisor::input_mouse` (`crates/lab/src/supervisor/input.rs:268-288`) presses the button at `self.cursor()` ("the current UI cursor", see its doc). Serde drops unknown `x`/`y` silently. CEGUI applies a button at its own cursor, never at the message coordinates (module doc, input.rs:9-10). | Certain |
| 6 pitch pinned at the clamp | `LiveWorld::native_look` (`world/io.rs:160-168`) clamps pitch to +-78.75 deg (`memory::clamped_pitch_counts`) and makes a push into the limit **no call at all** (`counts == 0.0`). `FaceController` (`world/steer.rs:310`) starts from a hard-coded guess (pitch counts/rad = -400, yaw +400) and only learns a gain from a pixel *change*. So at the limit, with a wrong-sign or wrong-size pitch guess (inverted axis flag 0x8, or the 90 deg FOV guess), the push is swallowed, the pixel never moves, no sample is learned and the loop repeats until `max_steps`: "fixed pixel, 0 yaw samples". `face()` (`world/camera.rs:132`) never looks at where the pitch is, never reports it, and nothing returns the camera to a usable pitch. `VIEW_RETRIES` in `click.rs:361` also drives pitch -300 twice, i.e. to about 66 deg up, before it tries any yaw. | High for the mechanism; Medium for which input first pins it |
| 6b p3 `camera_before/after` zero pose | `LiveWorld::camera_pose` (`io.rs:367`) returns whatever `memory::actor_pose` decodes; `decode_pose` (`memory.rs:59`) accepts an all-zero location. A camera actor whose `Location` the client never wrote reads exactly 0,0,0 and is passed on as a real pose, so `yaw_hint` (`camera.rs:122`) computes the bearing from the world origin. Static reading cannot say why this is p3-only (per-instance routing in `server/mod.rs:251-286` looks right: every tool uses the routed `self.supervisor`). | Mechanism certain; p3-only cause UNKNOWN: **probe P1** |
| 7 own avatar occludes | The third-person camera orbits the avatar, so `face` (centre the target on screen) puts a near target straight behind the avatar's body: the hover answers the player's own entity id (`Hover::Other(self)`). The guide already says so (`live-research-lab.md:722`). The click flow does retry a changed view, but (a) its first two retries are pitch -300 twice, which can drive the camera to the clamp and the target off screen before any yaw is tried (`click.rs:361-367`), (b) a `Hover::Nothing` candidate ends the retries at once (`click.rs:306`, "chosen.is_some() => break"), so the `mouse_over 0` variant never retries, and (c) the result never says "your avatar". | Medium: own-avatar vs mouse_over 0 cannot be told apart statically: **probe P2** |
| 9 right-click at 6.4 m opens nothing | Not a lab bug. The server refuses an interact beyond `MAX_INTERACT_DISTANCE = 5.0` m, straight-line 3D (`crates/cell-world/src/cell/space_manager/interact_range.rs:13,65`, rejection logged at info). The lab never reports the distance, so a driver sees "hovered, clicked, nothing opened". Note `Found.distance_m` is horizontal (`find.rs:177`), the server uses 3D. | Certain |

## Live probes (the coordinator runs these before the coder starts, lab pre-approval needed)

### P1 (item 6b, p3 zero pose). Does NOT block LH-06; it decides whether a follow-up packet is needed.
Two instances in the world, both idle, same lease owner per instance. Scripted Haiku lab-driver, raw results only:
1. `client_camera {"instance":"p3","lease_id":"<L>"}` (no other args). Record `camera_before.client`, `camera_before.yaw_deg`, `view_before` (zoom, pitch_offset_deg, yaw_offset_deg, gain, pitch_gain, yaw_gain).
2. The same call with `"instance":"default"`.
3. `client_camera {"instance":"p3","yaw_counts":200}`. Record `view_after.yaw_offset_deg` and `camera_after`.
4. The same on `default`.
Reading the outcome:
- `camera_before.client` is `{0,0,0}` on **both**: the camera actor's Location is never written by this client build. LH-06's zero-pose guard is the whole fix.
- Zero on p3 only, and `view_after.yaw_offset_deg` moved by about `200 * gain * 360/65536` on p3: the camera is fine and only the pose read is dead. LH-06 is the whole fix.
- Zero on p3 only AND `yaw_offset_deg` did not change: the native camera call hits a stale or wrong camera on p3. That is a real per-instance bug (stale cached pair, wrong level scan). Open a follow-up (needs `client_mem_read` of the controller pair on p3); not part of these packets.
- Also note the sign of `view_before.pitch_gain`: negative means pitch inversion (flag 0x8). If p3 is negative and default is positive, that is the p3-only trigger for the pinned-pitch trap, and LH-06's gain seeding is the specific fix.

### P2 (item 7, own avatar vs mouse_over 0). Do this before LH-07 is coded; the answer tunes two constants.
Debug Area plaza, trainer (look it up by name each time, ids change): put the avatar 3.4 m from the trainer on open floor (teleport), then:
1. `client_entity_find {"name":"<trainer name>"}` -> id T and distance.
2. `client_camera {"face_entity_id":T}`.
3. `client_hover_probe {"entity_id":T}`. Record `verdict`, `hover`, `avatar_px`.
4. `client_camera {"yaw_offset_deg":27}` then `client_hover_probe {"entity_id":T}`. Record `verdict`.
5. `client_camera {"yaw_offset_deg":-54}` (net -27 from the face) then `client_hover_probe {"entity_id":T}`.
6. Repeat 2-3 at 4.2 m.
Reading the outcome:
- Step 3 `verdict: "own"`: the avatar covers the target. The LH-07 design (yaw retries first) applies as written. If step 4 or 5 is `ok`, keep `VIEW_RETRIES` yaw at 250 counts (27.5 deg at gain 20). If both are still `own`, change the first two entries of `VIEW_RETRIES` to `(400, 0, 0)` and `(-800, 0, 0)` and the third to `(400, -300, 0)` (the sums stay zero).
- Step 3 `verdict: "nothing"` and `avatar_px` small: the pixel is under the HUD ring or the avatar is not reported as mouse-over. The near-target `Nothing` retry (LH-07 step 5) is what fixes it; keep it.
- Step 3 `ok` at both distances: not reproducible; ship LH-07 anyway (the retry order is still a correctness fix) and say so in the PR.

## Dependency order

LH-05 is independent (input.rs, server/mod.rs, io.rs `button`). LH-06 -> LH-07 -> LH-08 are a chain because they all touch `world/sim.rs`, `world/mod.rs` and `world/click.rs`. LH-05 and LH-06 can run in parallel (LH-05 touches one line of `io.rs::button`, LH-06 touches `io.rs::camera_pose`/`camera_readout`: different functions, trivial merge).

Doc edits are in `docs/guides/live-research-lab.md` and `.claude/agents/lab-driver.md`. Another expander edits `lab-driver.md` (item 16, the tool list in the front matter); my edits there are body lines 36, 37, 39 and 47 only. Docs are CRLF: edit with a tool that preserves it.

---

# LH-05: `client_input_mouse` takes x and y (item 5)

**Size S. Depends on: none.**

## Files
- `crates/lab/src/supervisor/input.rs` (add `MouseInput`, `cursor_target`, rewrite `input_mouse`, add tests)
- `crates/lab/src/server/mod.rs` (`InputMouseArgs`, the `client_input_mouse` tool, one import)
- `crates/lab/src/supervisor/world/io.rs` (`LiveWorld::button` call site, one import)
- `docs/guides/live-research-lab.md`, `.claude/agents/lab-driver.md`

Why a struct: `input_mouse` has 6 parameters plus `self`; adding a seventh trips `clippy::too_many_arguments` under `-D warnings`.

## Steps

1. `crates/lab/src/supervisor/input.rs`. Add after the `DEFAULT_HOLD_MS` constant (anchor: `pub const DEFAULT_HOLD_MS: u64 = 80;`):

```rust
/// One `client_input_mouse` call.
#[derive(Debug, Clone, Copy, Default)]
pub struct MouseInput<'a> {
    /// DirectInput relative motion (mouse-look) and wheel.
    pub dx: i32,
    pub dy: i32,
    pub wheel: i32,
    /// 0 left, 1 right, 2 middle.
    pub button: Option<usize>,
    /// `"click"` (default), `"down"` or `"up"`.
    pub action: Option<&'a str>,
    pub hold_ms: Option<u64>,
    /// UI pixel to put the cursor on first. CEGUI applies a button at its own
    /// cursor, so without this the press lands wherever the cursor happens
    /// to be.
    pub at: Option<(i32, i32)>,
}

/// `x` and `y` together, or neither: half a point is a mistake, and the old
/// behaviour (ignore it, click at the last cursor position) hid it.
pub fn cursor_target(x: Option<i32>, y: Option<i32>) -> Result<Option<(i32, i32)>, String> {
    match (x, y) {
        (Some(x), Some(y)) => Ok(Some((x, y))),
        (None, None) => Ok(None),
        _ => Err("give both x and y (UI pixels), or neither".to_string()),
    }
}
```

2. Same file. Replace the whole `input_mouse` method (from the doc comment `/// \`client_input_mouse\` — relative motion / wheel (DirectInput, for` through its closing `}` just before `pub(super) async fn post_button`) with:

```rust
    /// `client_input_mouse` — relative motion / wheel (DirectInput, for
    /// mouse-look), and a button press, release or click (window messages)
    /// at the UI cursor. With `at`, the cursor is put there first (a real
    /// CEGUI `MouseMove`, like `ui_click`); a point with no button just
    /// moves the cursor.
    pub async fn input_mouse(&self, m: MouseInput<'_>) -> Result<Value, String> {
        let placed = match m.at {
            Some((x, y)) => Some(self.place_cursor(x, y).await?),
            None => None,
        };
        if m.dx != 0 || m.dy != 0 || m.wheel != 0 {
            self.bridge_call(
                "input_mouse",
                json!({ "dx": m.dx, "dy": m.dy, "wheel": m.wheel }),
            )
            .await?;
        }
        let Some(button) = m.button else {
            return match placed {
                Some(p) => Ok(p.to_json()),
                None => self.bridge_call("input_status", json!({})).await,
            };
        };
        let at = match &placed {
            Some(p) => p.at,
            None => self.cursor().await?,
        };
        if placed.is_some() {
            // A frame for the UI to register the hover before the press.
            tokio::time::sleep(Duration::from_millis(40)).await;
        }
        self.post_button(button, m.action.unwrap_or("click"), at, m.hold_ms)
            .await?;
        let mut out = placed.map(|p| p.to_json()).unwrap_or_else(|| json!({}));
        out["button"] = json!(button);
        out["at"] = json!([at.0, at.1]);
        Ok(out)
    }
```

3. `crates/lab/src/server/mod.rs`. Add to the imports (anchor `use crate::supervisor::Supervisor;`):

```rust
use crate::supervisor::input::{cursor_target, MouseInput};
```
Replace the `InputMouseArgs` struct (anchor `/// Args for \`client_input_mouse\`.`) body so it ends with the two new fields (keep the existing fields and their docs, add after `hold_ms`):

```rust
    /// With `y`: put the UI cursor on this UI pixel first (a real CEGUI
    /// MouseMove), then press. Both or neither. With no button, just moves
    /// the cursor.
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
```
Replace the `client_input_mouse` tool (anchor `async fn client_input_mouse(`) with:

```rust
    #[tool(
        description = "Relative mouse motion / wheel through the game's DirectInput mouse (mouse-look while the viewport has the mouse captured), and/or a mouse-button press, release or click (window messages). Give x and y (UI pixels, both or neither) to put the UI cursor there first, with a real CEGUI MouseMove, and have the press land on it; without them the press lands wherever the UI cursor already is (CEGUI applies a button at its own cursor, never at a message's coordinates). x and y with no button just move the cursor."
    )]
    async fn client_input_mouse(
        &self,
        Parameters(a): Parameters<InputMouseArgs>,
    ) -> Result<CallToolResult, McpError> {
        let at = match cursor_target(a.x, a.y) {
            Ok(at) => at,
            Err(e) => return self.wrap(Err(e)),
        };
        self.wrap(
            self.supervisor
                .input_mouse(MouseInput {
                    dx: a.dx,
                    dy: a.dy,
                    wheel: a.wheel,
                    button: a.button,
                    action: a.action.as_deref(),
                    hold_ms: a.hold_ms,
                    at,
                })
                .await,
        )
    }
```
(The old `#[tool(description = ...)]` attribute above the function is replaced by the one in this block; do not leave two.)

4. `crates/lab/src/supervisor/world/io.rs`. Add `use crate::supervisor::input::MouseInput;` next to the other `use crate::supervisor::...` lines (anchor `use crate::supervisor::{process, Supervisor};`). Replace `LiveWorld::button` (anchor `async fn button(&mut self, button: usize, down: bool) -> Result<(), String> {` inside `impl WorldIo for LiveWorld`) with:

```rust
    async fn button(&mut self, button: usize, down: bool) -> Result<(), String> {
        self.sup
            .input_mouse(MouseInput {
                button: Some(button),
                action: Some(if down { "down" } else { "up" }),
                ..MouseInput::default()
            })
            .await
            .map(|_| ())
    }
```

## Test
Add to the existing `#[cfg(test)] mod tests` at the bottom of `crates/lab/src/supervisor/input.rs` (it already has `use super::*;`). Add these imports inside that module first:

```rust
    use std::sync::{Arc, Mutex};

    use crate::supervisor::cegui_fake::{self, FakeCegui};
    use crate::supervisor::events::fake_bridge;
```
and these tests (the fake bridge cannot post window messages: there is no game HWND, so a press fails with "no client running" *after* the cursor move, which is exactly the order being guarded):

```rust
    #[test]
    fn half_a_point_is_refused() {
        assert_eq!(cursor_target(Some(3), Some(4)), Ok(Some((3, 4))));
        assert_eq!(cursor_target(None, None), Ok(None));
        assert!(cursor_target(Some(3), None).unwrap_err().contains("both x and y"));
        assert!(cursor_target(None, Some(4)).is_err());
    }

    /// Item 5 of #1660: the press used to land at the last cursor position.
    /// With a point, the cursor goes there (a native `injectMousePosition`)
    /// before the button message is posted.
    #[tokio::test]
    async fn a_press_with_a_point_moves_the_cursor_first() {
        let state = Arc::new(Mutex::new(FakeCegui::default()));
        let sup = fake_bridge::supervisor(cegui_fake::responder(state.clone())).await;
        let err = sup
            .input_mouse(MouseInput {
                button: Some(0),
                at: Some((300, 200)),
                ..MouseInput::default()
            })
            .await
            .unwrap_err();
        assert!(err.contains("no client running"), "{err}");
        assert_eq!(state.lock().unwrap().calls, vec!["move 300 200"]);
    }

    #[tokio::test]
    async fn a_point_with_no_button_just_moves_the_cursor() {
        let state = Arc::new(Mutex::new(FakeCegui::default()));
        let sup = fake_bridge::supervisor(cegui_fake::responder(state.clone())).await;
        let out = sup
            .input_mouse(MouseInput {
                at: Some((300, 200)),
                ..MouseInput::default()
            })
            .await
            .unwrap();
        assert_eq!(out["cursor"], json!([300, 200]));
        assert_eq!(out["cursor_via"], "native_inject");
        assert_eq!(state.lock().unwrap().calls, vec!["move 300 200"]);
    }
```
Why it fails when reverted: remove the `place_cursor` call (the old behaviour) and `calls` stays empty, so both async tests fail; the old code also had no way to pass a point.

## Lane checks (in order, from `<wt>`)
```
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo fmt -p cimmeria-lab
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo clippy -p cimmeria-lab --all-targets -- -D warnings
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-lab supervisor::input
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-lab supervisor::world
```

## Docs
`docs/guides/live-research-lab.md`, the tools table row (anchor, line 293):
`| \`client_input_mouse\` | DirectInput relative motion (mouse-look) and button clicks at the UI cursor. |`
replace with:
`| \`client_input_mouse\` | DirectInput relative motion (mouse-look) and button clicks. \`x\` and \`y\` (UI pixels, both or neither) put the UI cursor there first, with a real CEGUI \`MouseMove\`, and the press lands on it; without them the press lands wherever the cursor already is (CEGUI applies a button at its own cursor, never at the message's coordinates). \`x\`/\`y\` with no button just move the cursor. |`

`.claude/agents/lab-driver.md` line 47 (anchor `\`client_cursor_move\` to the close button's centre, \`client_input_mouse\` button 0, then read again to confirm it closed.`) replace that clause with:
`\`client_input_mouse {button: 0, x, y}\` at the close button's centre (it moves the cursor first), then read again to confirm it closed.`

## Live smoke (close-out lab UAT)
Park the cursor away first: `client_cursor_move {x:5,y:5}`. Open the bag (`client_input_key {key:"B"}`), read a visible button's rectangle with `client_window_read`, then `client_input_mouse {button:0, x:<cx>, y:<cy>}` for the bag's close button. Expect the result `cursor == [cx,cy]`, `cursor_via: "native_inject"`, `at == [cx,cy]` and the bag closed (`client_ui_state`). Before the fix the same call pressed at (5,5) and the bag stayed open. No server evidence (client only).

## Commit subject
`fix(lab): client_input_mouse takes x and y and moves the cursor before the press (LH-05)`

---

# LH-06: pitch clamp is detected, reported and recovered; the face loop uses the camera's own gains; zero camera pose is not a pose (items 6, 6b)

**Size M. Depends on: none (but LH-07 and LH-08 depend on this packet).**

## Design
1. `memory::pitch_pinned(deg)`: true within 0.5 deg of +-78.75.
2. `FaceController::seed_gains`: start the yaw and pitch models from the readout's signed gains (`yaw_gain`, `pitch_gain`, rotator units per count) instead of the hard-coded +-400. Derivation (verified against the sim, which models both signs): counts per radian = `65536/TAU / yaw_gain` for yaw, and `-65536/TAU / pitch_gain` for pitch. An inverted axis (flag 0x4 / 0x8) flips the readout's sign, so the first look already goes the right way; the existing sign-flip learning stays as the backstop.
3. `face()` reads the readout once: seeds the gains, and when the pitch is pinned it records `pitch_clamped` / `pitch_recovered_from` and returns the camera to `PITCH_RECOVER_DEG` (10 deg) with the existing closed loop `abs_pitch` before it looks. A failed face names the clamp in its message. `client_camera` adds `pitch_clamped: true` and a `warning` when it ends pinned.
4. `LiveWorld::camera_pose` / `camera_readout` treat an all-zero camera Location as "no pose" (`memory::real_camera_pose`). `camera_before/after` are then `null` instead of a fake origin pose, `yaw_hint` falls back to the player's facing, and `abs_yaw` runs open loop (already supported).

Deliberately not done: an in-loop recovery (cannot be reproduced in the sim without contrivance) and an end-of-face recovery (the next call's start-of-face recovery does the same).

## Files
- `crates/lab/src/supervisor/world/memory.rs`
- `crates/lab/src/supervisor/world/steer.rs`
- `crates/lab/src/supervisor/world/camera.rs`
- `crates/lab/src/supervisor/world/click.rs` (one error message)
- `crates/lab/src/supervisor/world/io.rs` (`camera_pose`, `camera_readout`)
- `crates/lab/src/supervisor/world/sim.rs` (two knobs; test support)
- `crates/lab/src/supervisor/world/mod.rs` (register a test module)
- `crates/lab/src/supervisor/world/camera_pitch_tests.rs` (new)
- `crates/lab/src/server/world.rs` (tool description)
- `docs/guides/live-research-lab.md`, `.claude/agents/lab-driver.md`

## Steps

1. `memory.rs`. After `pub const CAMERA_PITCH_LIMIT: i32 = 0x3800;` (anchor) add:

```rust
/// [`CAMERA_PITCH_LIMIT`] in readout degrees (`pitch_offset_deg`).
pub const CAMERA_PITCH_LIMIT_DEG: f64 = 78.75;
/// A readout this close to the limit, or past it, is "pinned".
pub const PITCH_PINNED_MARGIN_DEG: f64 = 0.5;

/// Is a readout's `pitch_offset_deg` at the pitch limit? There the lab's
/// clamp ([`clamped_pitch_counts`]) turns every further push into that
/// direction into no call at all, so a closed loop that pushes that way
/// never moves the pixel and never learns anything.
pub fn pitch_pinned(pitch_offset_deg: f64) -> bool {
    pitch_offset_deg.is_finite()
        && pitch_offset_deg.abs() >= CAMERA_PITCH_LIMIT_DEG - PITCH_PINNED_MARGIN_DEG
}

/// The camera actor's pose, or `None` when its location is exactly zero: the
/// client never wrote one (seen on a second lab instance, 2026-10-12), and a
/// zero pose would make every bearing from the camera wrong.
pub fn real_camera_pose(p: Pose) -> Option<Pose> {
    (p.pos != Vec3::new(0.0, 0.0, 0.0)).then_some(p)
}
```
Add tests inside `memory.rs`'s `mod tests` (it has `use super::*;`):

```rust
    #[test]
    fn a_pitch_at_the_limit_is_pinned() {
        assert_eq!(
            CAMERA_PITCH_LIMIT_DEG,
            f64::from(CAMERA_PITCH_LIMIT) * 360.0 / 65536.0
        );
        assert!(pitch_pinned(78.75));
        assert!(pitch_pinned(-78.4));
        assert!(pitch_pinned(1563.0));
        assert!(!pitch_pinned(78.0));
        assert!(!pitch_pinned(0.0));
        assert!(!pitch_pinned(f64::NAN));
    }

    #[test]
    fn an_all_zero_camera_location_is_not_a_pose() {
        let at = |x, y, z| Pose {
            pos: Vec3::new(x, y, z),
            yaw: 1.0,
            pitch: 0.0,
        };
        assert_eq!(real_camera_pose(at(0.0, 0.0, 0.0)), None);
        assert!(real_camera_pose(at(0.0, 0.0, 5.0)).is_some());
    }
```

2. `steer.rs`. Change the import line `use std::f64::consts::{FRAC_PI_2, PI};` to `use std::f64::consts::{FRAC_PI_2, PI, TAU};`. After the `GAIN_BOUNDS` constant add:

```rust
/// Rotator units in one radian (65536 per turn): the camera's `gain` is
/// rotator units per mouse count.
const UNITS_PER_RAD: f64 = 65536.0 / TAU;
```
In `impl FaceController`, after `pixel_angles` (anchor: the end of `fn pixel_angles`), add:

```rust
    /// Start from the camera's own signed gains (`yaw_gain` / `pitch_gain` of
    /// `client_camera`'s readout, rotator units per count) instead of guesses.
    /// A target is fixed, so turning the camera one way moves it the other:
    /// yaw counts per radian is `U / yaw_gain` and pitch counts per radian is
    /// `-U / pitch_gain` (looking down raises the pitch offset and moves the
    /// target up the screen). An inverted axis has a negative gain, so the
    /// first look already goes the right way. A gain that is zero (settings
    /// not loaded yet) or not finite changes nothing, and a model that has
    /// already learned from a pixel keeps what it learned.
    pub fn seed_gains(&mut self, yaw_gain: Option<f64>, pitch_gain: Option<f64>) {
        let seed = |v: f64| v.signum() * v.abs().clamp(GAIN_BOUNDS.0, GAIN_BOUNDS.1);
        let usable = |g: &f64| g.is_finite() && g.abs() >= 1e-3;
        if let Some(g) = yaw_gain.filter(usable) {
            if self.yaw.samples == 0 {
                self.yaw = TurnModel::new(seed(UNITS_PER_RAD / g));
            }
        }
        if let Some(g) = pitch_gain.filter(usable) {
            if self.pitch.samples == 0 {
                self.pitch = TurnModel::new(seed(-UNITS_PER_RAD / g));
            }
        }
    }

    /// Forget the last look, so the next pixel is not read as its result
    /// (the camera was moved by something else in between).
    pub fn forget_last_look(&mut self) {
        self.last = None;
    }
```
Test in `steer.rs`'s `mod tests`:

```rust
    #[test]
    fn face_controller_seeds_its_gains_from_the_camera_readout() {
        let mut f = FaceController::new(FaceConfig::default(), TurnModel::default());
        // Gain 20 rotator units per count: 65536 / TAU / 20 = 521.5 counts per radian.
        f.seed_gains(Some(20.0), Some(20.0));
        assert!((f.yaw.counts_per_rad - 521.5).abs() < 0.1);
        assert!((f.pitch.counts_per_rad + 521.5).abs() < 0.1);
        // An inverted pitch axis (flag 0x8) reads a negative gain and flips the sign.
        let mut g = FaceController::new(FaceConfig::default(), TurnModel::default());
        g.seed_gains(Some(-20.0), Some(-20.0));
        assert!((g.yaw.counts_per_rad + 521.5).abs() < 0.1);
        assert!((g.pitch.counts_per_rad - 521.5).abs() < 0.1);
        // A gain read before the game loads its settings is 0: keep the guess.
        let mut h = FaceController::new(FaceConfig::default(), TurnModel::new(400.0));
        h.seed_gains(Some(0.0), None);
        assert_eq!(h.yaw.counts_per_rad, 400.0);
        assert_eq!(h.pitch.counts_per_rad, -400.0);
    }
```

3. `camera.rs`.
   a. Constants: after `pub const ABS_MAX_LOOKS: u32 = 4;` add
```rust
/// Where a face brings a pitch that is pinned at the limit back to, readout
/// degrees: the avatar and the horizon are both in view.
pub const PITCH_RECOVER_DEG: f64 = 10.0;
```
   b. `FaceReport`: add two fields after `yaw_model`:
```rust
    /// The pitch was pinned at the +-78.75 deg limit when the face started.
    pub pitch_clamped: bool,
    /// The pitch (readout degrees) the camera was brought back from.
    pub pitch_recovered_from: Option<f64>,
```
   Replace `FaceReport::to_json` with:
```rust
    pub fn to_json(&self) -> Value {
        let mut out = json!({
            "centred": self.centred,
            "screen": self.screen.map(|s| json!([s.w, s.h])),
            "pixel": self.pixel.map(|(x, y)| json!([x.round(), y.round()])),
            "looks": self.looks,
            "searches_behind_camera": self.searches,
            "yaw_gain": self.yaw_model.to_json(),
        });
        if self.pitch_clamped {
            out["pitch_clamped"] = json!(true);
            out["pitch_recovered_from"] =
                json!(self.pitch_recovered_from.map(|p| (p * 100.0).round() / 100.0));
        }
        out
    }

    /// A failure-message suffix naming the clamp, or empty.
    pub fn clamp_note(&self) -> String {
        match (self.pitch_clamped, self.pitch_recovered_from) {
            (false, _) => String::new(),
            (true, Some(p)) => format!(
                " (pitch clamped at {p:.1} deg; the camera was returned to {PITCH_RECOVER_DEG} deg)"
            ),
            (true, None) => " (pitch clamped at the 78.75 deg limit)".to_string(),
        }
    }
```
   c. Add above `face` (anchor: the doc comment `/// Turn the camera until \`aim\` is near the screen centre.`):
```rust
/// The readout's pitch when it is pinned at the limit.
pub(super) fn pinned_pitch_of(readout: &Value) -> Option<f64> {
    readout["pitch_offset_deg"]
        .as_f64()
        .filter(|p| memory::pitch_pinned(*p))
}
```
   d. In `face`, replace the lines from `let mut ctl = FaceController::new(FaceConfig::default(), model);` through the end of the `let mut report = FaceReport { ... };` statement with:
```rust
    let mut ctl = FaceController::new(FaceConfig::default(), model);
    let mut report = FaceReport {
        centred: false,
        screen: None,
        pixel: None,
        looks: 0,
        searches: 0,
        yaw_model: model,
        pitch_clamped: false,
        pitch_recovered_from: None,
    };
    // One readout: the camera's own signed gains replace the guesses (an
    // inverted axis is right from the first look), and a pitch left pinned at
    // its limit by an earlier call is undone first. At the limit the lab's
    // clamp makes a push into it no call at all, so the pixel never moves and
    // nothing is learned.
    let readout = io.camera_readout().await.ok();
    if let Some(r) = &readout {
        ctl.seed_gains(r["yaw_gain"].as_f64(), r["pitch_gain"].as_f64());
    }
    if let Some(from) = readout.as_ref().and_then(pinned_pitch_of) {
        report.pitch_clamped = true;
        report.pitch_recovered_from = Some(from);
        // A failed recovery leaves the camera where it was; the report says so.
        let _ = abs_pitch(io, steps, PITCH_RECOVER_DEG).await;
    }
```
   e. In `run`, replace the face failure (anchor `"could not bring the target to the screen centre",`) with:
```rust
                format!(
                    "could not bring the target to the screen centre{}",
                    rep.clamp_note()
                ),
```
   f. In `run`, replace the block from `let camera_after = io.camera_pose().await.ok();` through the final `Ok(steps.finish(io.now_ms(), out))` with:
```rust
    let camera_after = io.camera_pose().await.ok();
    let readout = |r: Result<Value, String>| r.unwrap_or_else(|e| json!({ "error": e }));
    let view_after = readout(io.camera_readout().await);
    let pinned = pinned_pitch_of(&view_after).is_some();
    let mut out = json!({
        "camera_before": camera_before.map(|p| p.to_json()),
        "camera_after": camera_after.map(|p| p.to_json()),
        "view_before": readout(readout_before),
        "view_after": view_after,
        "target": resolved,
        "face": face_json,
    });
    if !absolute.is_empty() {
        out["absolute"] = Value::Object(absolute);
    }
    if pinned {
        out["pitch_clamped"] = json!(true);
        out["warning"] = json!(format!(
            "the pitch is pinned at the 78.75 deg limit: every further push that way does nothing; client_camera {{pitch_deg: {PITCH_RECOVER_DEG}}} brings it back"
        ));
    }
    Ok(steps.finish(io.now_ms(), out))
```

4. `click.rs`. Anchor `"could not turn the camera onto the target",` becomes
```rust
                format!("could not turn the camera onto the target{}", rep.clamp_note()),
```

5. `io.rs`. Replace `camera_pose` and `camera_readout` in `impl WorldIo for LiveWorld` (anchors `async fn camera_pose(&mut self) -> Result<Pose, String> {` and `async fn camera_readout(&mut self) -> Result<Value, String> {`):
```rust
    async fn camera_pose(&mut self) -> Result<Pose, String> {
        let c = self.player_camera().await?;
        let p = memory::actor_pose(&mut Reader(self.sup), c.camera).await?;
        memory::real_camera_pose(p).ok_or_else(|| {
            format!("camera actor {:#x} has no location (it reads all zero)", c.camera)
        })
    }

    async fn camera_readout(&mut self) -> Result<Value, String> {
        let c = self.player_camera().await?;
        let mut r = Reader(self.sup);
        let mut out = memory::camera_state(&mut r, c.camera).await?.to_json();
        if let Some(p) = memory::actor_pose(&mut r, c.camera)
            .await
            .ok()
            .and_then(memory::real_camera_pose)
        {
            out["pose"] = p.to_json();
        }
        Ok(out)
    }
```

6. `sim.rs`. Add to the `Sim` struct (after `pub unhoverable: Vec<u32>,`):
```rust
    /// Pitch turns behave like the live camera: clamped to +-78.75 deg
    /// (`memory::clamped_pitch_counts`), and a push into the limit is no call.
    pub live_pitch_clamp: bool,
    /// The pitch axis is inverted (flag 0x8): `pitch_gain` reads negative and
    /// +dy looks up.
    pub pitch_inverted: bool,
```
and in `Sim::new` (after `unhoverable: Vec::new(),`): `live_pitch_clamp: false, pitch_inverted: false,`. Add `clamped_pitch_counts` to the `use super::memory::{...}` list. Replace the body of `Sim::look` from `self.looks.push(...)` through the `self.cam_pitch = ...;` line with:
```rust
        self.looks.push((dx, dy, wheel));
        self.cam_yaw = wrap_pi(self.cam_yaw + dx as f64 / self.gain);
        let sign = if self.pitch_inverted { -1.0 } else { 1.0 };
        let mut counts = f64::from(dy);
        if self.live_pitch_clamp && dy != 0 {
            // What LiveWorld::native_look does before it calls the turn handler.
            let offset_units = (-self.cam_pitch).to_degrees() * 65536.0 / 360.0;
            let gain = sign * 65536.0 / std::f64::consts::TAU / self.gain.abs();
            counts = f64::from(clamped_pitch_counts(offset_units as i32, gain as f32, dy as f32));
        }
        // Mouse forward (negative dy) looks up (inverted: down).
        self.cam_pitch = (self.cam_pitch - sign * counts / self.gain.abs()).clamp(-1.4, 1.4);
```
In `Sim::camera_readout` change `"pitch_gain": gain,` to `"pitch_gain": if self.pitch_inverted { -gain } else { gain },`.

7. `mod.rs`. After `#[cfg(test)]\nmod tool_tests;` add:
```rust
#[cfg(test)]
mod camera_pitch_tests;
```

8. `server/world.rs`: in the `client_camera` tool description append: ` Reports pitch_clamped (with a warning) when the camera ends at the +-78.75 deg pitch limit; a face that starts there brings it back to 10 deg first and says so under face.pitch_clamped.`

## Test
New file `crates/lab/src/supervisor/world/camera_pitch_tests.rs`:

```rust
//! A pitch pinned at the camera's limit: reported, recovered, and the face
//! loop reads the camera's own gains (#1660 item 6).

use super::camera::{self, CameraRequest};
use super::geometry::Vec3;
use super::memory::CAMERA_PITCH_LIMIT_DEG;
use super::sim::Sim;

/// A guard 10 m ahead and a medic 20 m to the left, the camera pinned at the
/// pitch limit (readout +78.75 = looking straight down), with the live clamp.
fn pinned_room() -> Sim {
    let mut s = Sim::new()
        .with_entity(10, "Cellblock Guard", Vec3::new(1000.0, 0.0, 0.0))
        .with_entity(11, "Medic Ogilvie", Vec3::new(0.0, 2000.0, 0.0));
    s.live_pitch_clamp = true;
    s.cam_pitch = -CAMERA_PITCH_LIMIT_DEG.to_radians();
    s
}

#[tokio::test]
async fn a_face_from_a_pinned_pitch_says_so_and_comes_back() {
    let mut sim = pinned_room();
    let out = camera::run(
        &mut sim,
        CameraRequest {
            face_entity_id: Some(10),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["face"]["centred"], true, "{out}");
    assert_eq!(out["face"]["pitch_clamped"], true, "{out}");
    let from = out["face"]["pitch_recovered_from"].as_f64().unwrap();
    assert!((from - CAMERA_PITCH_LIMIT_DEG).abs() < 0.1, "{out}");
    let after = out["view_after"]["pitch_offset_deg"].as_f64().unwrap();
    assert!(after.abs() < CAMERA_PITCH_LIMIT_DEG - 5.0, "still pinned: {out}");
    assert!(out["pitch_clamped"].is_null(), "{out}");
}

/// A face that fails names the clamp (max_steps 0 stops it before it can turn).
#[tokio::test]
async fn a_failed_face_from_a_pinned_pitch_names_the_clamp() {
    let mut sim = pinned_room();
    let e = camera::run(
        &mut sim,
        CameraRequest {
            face_entity_id: Some(11),
            max_steps: Some(0),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap_err();
    assert_eq!(e.step, "face");
    assert!(e.message.contains("pitch clamped"), "{}", e.message);
    assert_eq!(e.to_json()["state"]["pitch_clamped"], true);
}

/// Raw pitch past the limit ends pinned: the result says so instead of
/// leaving the next call to find out.
#[tokio::test]
async fn a_camera_call_that_ends_pinned_warns() {
    let mut sim = Sim::new();
    sim.live_pitch_clamp = true;
    let out = camera::run(
        &mut sim,
        CameraRequest {
            pitch_counts: Some(900),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["pitch_clamped"], true, "{out}");
    assert!(out["warning"].as_str().unwrap().contains("78.75"), "{out}");
}

/// An inverted pitch axis (flag 0x8) with a target far above the view: the
/// gains come from the readout, so the first look goes the right way and one
/// look centres it. With the old hard-coded guess (-400 counts/rad) the first
/// look went the wrong way and the sign had to be learned first.
#[tokio::test]
async fn an_inverted_pitch_axis_is_right_from_the_first_look() {
    let mut sim = Sim::new().with_entity(10, "High Sign", Vec3::new(1000.0, 0.0, 1500.0));
    sim.live_pitch_clamp = true;
    sim.pitch_inverted = true;
    let out = camera::run(
        &mut sim,
        CameraRequest {
            face_entity_id: Some(10),
            ..CameraRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(out["face"]["centred"], true, "{out}");
    assert!(out["face"]["looks"].as_u64().unwrap() <= 2, "{out}");
}
```
Why they fail when reverted: without `pinned_pitch_of` recovery the first test has no `pitch_clamped` and the camera is not moved off the limit by the face; without `clamp_note` the second test's message lacks the clamp; without the warning the third has no `pitch_clamped`; without `seed_gains` the fourth needs three or more looks (the first goes the wrong way). The sim knobs make the clamp behave as the live one does (a push into the limit moves nothing).

## Lane checks
```
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo fmt -p cimmeria-lab
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo clippy -p cimmeria-lab --all-targets -- -D warnings
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-lab camera_pitch_tests
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-lab supervisor::world
```
The last line must be all green: the gain seeding changes how many looks existing face tests take, but not their results (`tool_tests.rs` assertions use `>= 1` looks and final positions). If an existing test fails, STOP and report which one; do not edit it.

## Docs
`docs/guides/live-research-lab.md`:
- Tools table row for `client_camera` (line 305, anchor `| \`client_camera\` | Mouse-look yaw/pitch, wheel zoom, face an entity or point, then an absolute pitch, yaw and zoom (\`pitch_deg\`, \`yaw_offset_deg\`, \`zoom\`). |`): append before the final ` |`: ` Reports \`pitch_clamped\` when the pitch sits at the +-78.75 deg limit; a face that starts there brings it back to 10 deg first.`
- World-tools table row `client_camera` (line 700): at the end of the Result cell append `; \`face.pitch_clamped\` / \`face.pitch_recovered_from\` when the face started at the pitch limit; top-level \`pitch_clamped\` and \`warning\` when the call ends there; \`camera_before\` / \`camera_after\` are \`null\` when the camera actor's location reads all zero (use \`view_after\`)`.
- "Facing" bullet (line 714, anchor `- **Facing** turns until the target projects within a quarter of the half-width of the centre.`): append at the end of the bullet: ` It starts from the camera's own signed yaw and pitch gains (the readout's \`yaw_gain\` and \`pitch_gain\`), so an inverted axis is right from the first look, and if the pitch is pinned at the +-78.75 deg limit it first returns the camera to 10 deg (\`face.pitch_clamped\`): at the limit the lab's clamp makes a push into it no call at all, so the pixel never moved and nothing was learned (#1660). A failed face says "pitch clamped" in its message when that was the case.`
`.claude/agents/lab-driver.md` line 37 (anchor `Read \`view_after.pitch_offset_deg\` after the last move; trust \`view_after\`, not \`camera_*.pose\`.`): append: ` A result with \`pitch_clamped: true\` (or \`face.pitch_clamped\`) means the camera sat at the +-78.75 deg pitch limit; \`client_camera {pitch_deg: 10}\` brings it back, and a face now does that itself.`

## Live smoke
On a fresh character in the world: `client_camera {"pitch_counts":900}` -> expect `pitch_clamped: true` and a `warning`, `view_after.pitch_offset_deg` about 78.75. Then `client_camera {"face_entity_id":<NPC about 4 m away>}` -> expect `face.pitch_clamped: true`, `face.pitch_recovered_from` about 78.75, `face.centred: true`, `view_after.pitch_offset_deg` under 70. On p3 repeat once with `instance:"p3"` and report `camera_before` (now `null` if the location is zero) and the sign of `view_before.pitch_gain`. Client only: no server evidence.

## Commit subject
`fix(lab): detect and recover a pitch pinned at the clamp; seed face gains from the camera readout (LH-06)`

---

# LH-07: the click flow clicks a target the avatar stands in front of (item 7)

**Size L (a file split plus a loop rewrite). Depends on: LH-06** (same files: `sim.rs`, `mod.rs`, `click.rs`).

`click.rs` is 639 lines; this packet and LH-08 would push it past the 700-line hard cap, so step 1 moves the hover code into `click_hover.rs` first. Do step 1 and run its lane check before the rest.

## Design
- Retry order: yaw first (about 27.5 deg each way, 250 counts at gain 20), then pitch up twice, then zoom in. The old order (pitch -300 twice first) drove the camera to the clamp with the target off screen before any yaw was tried. The sums stay zero so the existing "ends where it began" test holds.
- A hover that lands on the player's own entity is labelled `own_avatar` in `tried`, `saw_own_avatar` goes into the hover detail, and the failure message says to stand to the side.
- A `Hover::Nothing` candidate no longer ends the retries when the target is within 8 m of the avatar (the avatar or the HUD ring is the likely reason) or when some other height hovered another entity. Far targets keep the old behaviour (click the `Nothing` candidate without retrying), so bodies that never hover cost no extra time. `rotate_camera: false` still disables retries.
- When no retry verifies a hover, the view is restored (as before), and now the first view's candidate is taken again, so a `Nothing` candidate is still clicked.

## Files
- `crates/lab/src/supervisor/world/click_hover.rs` (new; moved code plus the changes below)
- `crates/lab/src/supervisor/world/click.rs`
- `crates/lab/src/supervisor/world/hover.rs` (import path and one call)
- `crates/lab/src/supervisor/world/mod.rs` (register `click_hover` and a test module)
- `crates/lab/src/supervisor/world/geometry.rs` (`distance_m`, reused by LH-08)
- `crates/lab/src/supervisor/world/sim.rs` (`avatar_hides`)
- `crates/lab/src/supervisor/world/occlusion_tests.rs` (new)
- `docs/guides/live-research-lab.md`, `.claude/agents/lab-driver.md`

## Steps

### Step 1: split (no behaviour change except the signature and constants noted)
1. Create `crates/lab/src/supervisor/world/click_hover.rs` with exactly this content:

```rust
//! The hover half of `client_world_click`: which body height the client says
//! is the entity, and when to change the view and look again.

use serde_json::{json, Value};

use super::camera::Aim;
use super::find::CLICK_MARGIN_PX;
use super::geometry::Vec3;
use super::io::WorldIo;
use super::memory::slot_entity;
use super::{Steps, WorldError};

/// Heights tried on an entity's body, UE3 units above its actor location
/// (the collision centre): centre, chest, legs, head.
pub const HOVER_OFFSETS: [f64; 4] = [0.0, 50.0, -40.0, 90.0];
pub(super) const HOVER_WAIT_MS: u64 = 120;
/// A target this close to the avatar sits behind or under it on a
/// third-person camera, so an unverified hover is retried from another view.
pub(super) const NEAR_AVATAR_M: f64 = 8.0;

/// How the hover went at one height.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Hover {
    Matched,
    Nothing,
    Other(u32),
    OffScreen,
}

/// View changes tried, in order and cumulatively, when the target is
/// covered: (yaw counts, pitch counts, zoom notches). At the live gain (20
/// units per count) 250 counts is about 27.5 degrees and 300 about 33. Yaw
/// goes first, to either side: the usual cover is the avatar itself, which a
/// turn to the side clears, and pitching first drove the camera to its
/// clamp with the target off screen. Each column sums to zero (the zoom is
/// taken back by name).
pub const VIEW_RETRIES: [(i32, i32, i32); 5] = [
    (250, 0, 0),
    (-500, 0, 0),
    (250, -300, 0),
    (0, -300, 0),
    (0, 600, 4),
];

/// Should the view change and the hover run again? Never when it is verified,
/// and never when `force` already has a candidate to click. Another entity
/// under the cursor at some height (an occluder, or your own avatar) always
/// asks for a retry; so does finding nothing at all; and so does a bare
/// `Nothing` candidate when the target is `near` the avatar. A far target
/// that hovers nothing is clicked as it is.
pub(super) fn wants_view_retry(
    chosen: &Option<((f64, f64), Hover)>,
    tried: &[Value],
    near: bool,
    force: bool,
) -> bool {
    match chosen {
        Some((_, Hover::Matched)) => false,
        Some(_) if force => false,
        _ => {
            let covered = tried.iter().any(|t| {
                t["hover"].get("other_entity").is_some() || t["hover"] == "own_avatar"
            });
            covered || near || chosen.is_none()
        }
    }
}

/// Pitch counts that take the camera from readout `now` back to `start`
/// (`pitch_offset_deg` and `pitch_gain` from `camera_readout`); 0 when
/// either readout lacks them.
pub fn restore_pitch_counts(start: &Value, now: &Value) -> i32 {
    let (Some(a), Some(b), Some(g)) = (
        start["pitch_offset_deg"].as_f64(),
        now["pitch_offset_deg"].as_f64(),
        now["pitch_gain"].as_f64(),
    ) else {
        return 0;
    };
    if g.abs() < 1e-3 {
        return 0;
    }
    ((a - b) * 65536.0 / 360.0 / g).round() as i32
}

/// One hover pass over the target's body heights: the chosen point and how
/// each height went. `player_slot` is the avatar's unit slot: a hover that
/// lands on it is reported as `own_avatar`, not as another entity.
pub(super) async fn hover_once<W: WorldIo>(
    io: &mut W,
    steps: &mut Steps,
    aim: &mut Aim,
    entity: Option<u32>,
    mo_slot: i32,
    player_slot: i32,
    force: bool,
) -> Result<(Option<((f64, f64), Hover)>, Vec<Value>), WorldError> {
    let base = aim.position(io).await;
    let points: Vec<Vec3> = match entity {
        Some(_) => HOVER_OFFSETS
            .iter()
            .map(|dz| Vec3::new(base.x, base.y, base.z + dz))
            .collect(),
        None => vec![base],
    };
    let pr = io
        .project(&points)
        .await
        .map_err(|e| steps.fail(io.now_ms(), "project", e))?;
    let mut tried = Vec::new();
    let mut chosen: Option<((f64, f64), Hover)> = None;
    for (i, px) in pr.points.iter().enumerate() {
        let Some(p) = px.filter(|p| pr.screen.contains(*p, CLICK_MARGIN_PX)) else {
            tried.push(json!({ "dz": HOVER_OFFSETS.get(i), "hover": "off_screen" }));
            continue;
        };
        let (x, y) = (p.0.round() as i32, p.1.round() as i32);
        let Some(e) = entity else {
            io.place_cursor(x, y, true)
                .await
                .map_err(|e| steps.fail(io.now_ms(), "cursor", e))?;
            chosen = Some((p, Hover::Nothing));
            break;
        };
        let mut hover = Hover::OffScreen;
        let mut me = 0;
        for mouse_move in [false, true] {
            io.place_cursor(x, y, mouse_move)
                .await
                .map_err(|err| steps.fail(io.now_ms(), "cursor", err))?;
            io.sleep(HOVER_WAIT_MS).await;
            let slots = io
                .slots()
                .await
                .map_err(|err| steps.fail(io.now_ms(), "read_mouse_over", err))?;
            let mo = slot_entity(&slots, mo_slot);
            me = slot_entity(&slots, player_slot);
            hover = if mo == e {
                Hover::Matched
            } else if mo == 0 {
                Hover::Nothing
            } else {
                Hover::Other(mo)
            };
            if hover != Hover::Nothing {
                break;
            }
        }
        tried.push(
            json!({ "dz": HOVER_OFFSETS[i], "at": [x, y], "hover": match &hover {
                Hover::Matched => json!("entity"),
                Hover::Nothing => json!("nothing"),
                Hover::Other(o) if *o == me => json!("own_avatar"),
                Hover::Other(o) => json!({ "other_entity": o }),
                Hover::OffScreen => json!("off_screen"),
            }}),
        );
        match hover {
            Hover::Matched => {
                chosen = Some((p, Hover::Matched));
                break;
            }
            // Nothing under the cursor is not proof of a miss (the client
            // may only update mouse-over on its own schedule): keep it as
            // a candidate but prefer a verified height.
            Hover::Nothing if chosen.is_none() => chosen = Some((p, Hover::Nothing)),
            Hover::Other(o) if force && chosen.is_none() => chosen = Some((p, Hover::Other(o))),
            _ => {}
        }
    }
    Ok((chosen, tried))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn nothing() -> Option<((f64, f64), Hover)> {
        Some(((1.0, 2.0), Hover::Nothing))
    }

    #[test]
    fn a_verified_hover_never_retries() {
        let matched = Some(((1.0, 2.0), Hover::Matched));
        assert!(!wants_view_retry(&matched, &[], true, false));
    }

    #[test]
    fn a_bare_nothing_retries_only_near_the_avatar() {
        let tried = [json!({ "dz": 0.0, "hover": "nothing" })];
        assert!(wants_view_retry(&nothing(), &tried, true, false));
        assert!(!wants_view_retry(&nothing(), &tried, false, false));
    }

    #[test]
    fn a_cover_at_any_height_retries_even_with_a_nothing_candidate() {
        let own = [json!({ "hover": "own_avatar" }), json!({ "hover": "nothing" })];
        assert!(wants_view_retry(&nothing(), &own, false, false));
        let other = [json!({ "hover": { "other_entity": 20 } })];
        assert!(wants_view_retry(&nothing(), &other, false, false));
    }

    #[test]
    fn finding_nothing_at_all_retries_and_force_stops_it() {
        assert!(wants_view_retry(&None, &[], false, false));
        let forced = Some(((1.0, 2.0), Hover::Other(20)));
        assert!(!wants_view_retry(&forced, &[], true, true));
    }

    #[test]
    fn the_retries_net_to_zero_so_the_view_comes_back() {
        let (dx, dy): (i32, i32) = VIEW_RETRIES
            .iter()
            .fold((0, 0), |a, r| (a.0 + r.0, a.1 + r.1));
        assert_eq!((dx, dy), (0, 0));
        // Yaw is tried before any pitch.
        assert!(VIEW_RETRIES[0].0 != 0 && VIEW_RETRIES[0].1 == 0);
        assert!(VIEW_RETRIES[1].0 != 0 && VIEW_RETRIES[1].1 == 0);
    }
}
```

2. `crates/lab/src/supervisor/world/click.rs`: delete these pieces (all now live in `click_hover.rs`):
   - the doc comment and constant `pub const HOVER_OFFSETS: [f64; 4] = ...;` (3 lines starting `/// Heights tried on an entity's body`);
   - the line `pub(super) const HOVER_WAIT_MS: u64 = 120;`;
   - the `Hover` enum with its doc comment (starts `/// How the hover went at one height.`, ends with the closing `}` after `OffScreen,`);
   - everything from the doc comment `/// View changes tried, in order and cumulatively, when the target is` through the closing `}` of `hover_once` (that range contains `VIEW_RETRIES`, `restore_pitch_counts` and `hover_once`; it ends just before the doc comment `/// What the click step needs from the resolve and hover steps.`).
   Then replace the import block at the top (from `use super::camera::{face, Aim, DEFAULT_FACE_STEPS};` to `use super::{NativeLevel, PointArg, Steps, TargetArg, WorldError};`) with:

```rust
use super::camera::{face, Aim, DEFAULT_FACE_STEPS};
pub use super::click_hover::{restore_pitch_counts, HOVER_OFFSETS, VIEW_RETRIES};
use super::click_hover::{
    hover_once, wants_view_retry, Hover, HOVER_WAIT_MS, NEAR_AVATAR_M,
};
use super::find::{find_entities, FindRequest, Found, CLICK_MARGIN_PX};
use super::geometry::distance_m;
use super::io::WorldIo;
use super::memory::{slot_entity, LAB_SLOT_BASE};
use super::steer::TurnModel;
use super::{NativeLevel, PointArg, Steps, TargetArg, WorldError};
```
   (`Vec3` is no longer used in click.rs. If the compiler reports any other unused import, remove it.) The existing test `pitch_restore_counts_undo_the_offset_change` in `click.rs` keeps working through the `pub use`.

3. `hover.rs`: change `use super::click::{hover_once, Hover, HOVER_OFFSETS, HOVER_WAIT_MS};` to `use super::click_hover::{hover_once, Hover, HOVER_OFFSETS, HOVER_WAIT_MS};` and the call `hover_once(io, steps, &mut aim, Some(e), units.mouse_over, false).await?;` to `hover_once(io, steps, &mut aim, Some(e), units.mouse_over, units.player, false).await?;`. `hover_tests.rs` imports `super::super::click::HOVER_OFFSETS`, which the `pub use` keeps valid.

4. `mod.rs`: after `pub mod click;` add `mod click_hover;`.

5. `geometry.rs`: after `horizontal_m` add
```rust
/// Straight-line distance in metres between two client points. The server's
/// interaction range is this, not the horizontal distance [`horizontal_m`].
pub fn distance_m(a: Vec3, b: Vec3) -> f64 {
    let d = b.sub(a);
    (d.x * d.x + d.y * d.y + d.z * d.z).sqrt() / UU_PER_METRE
}
```
and in its `mod tests`:
```rust
    #[test]
    fn straight_line_distance_counts_height() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(300.0, 400.0, 1200.0);
        assert!((distance_m(a, b) - 13.0).abs() < 1e-9);
    }
```

Lane check for step 1 (must pass before step 2): fmt, clippy and `cargo nextest run -p cimmeria-lab supervisor::world` (all existing tests green, the new `click_hover` tests too; `the_retries_net_to_zero_so_the_view_comes_back` and `an_occluder_stops_the_click` pass with the new retry order).

### Step 2: the loop in `click.rs::run`
a. After the whole "2. On screen?" block, i.e. immediately before the comment line `// 3. Hover: find a point on the body the client says is the entity.`, insert:
```rust
    // How far the target is from the avatar, straight line (the server's
    // interact range is 3D). Near targets sit behind or under the avatar.
    let range_m = match (player, entity) {
        (Some(p), Some(_)) => Some(distance_m(p.pos, aim.position(io).await)),
        _ => None,
    };
    let near = range_m.is_some_and(|d| d <= NEAR_AVATAR_M);
```
b. Replace the block from `let t_hover = io.now_ms();` through the closing `};` of `let Some((pixel, hover)) = chosen else { ... };` with:
```rust
    let t_hover = io.now_ms();
    let mut adjusted = Vec::new();
    let mut tried = Vec::new();
    let mut chosen = None;
    let mut start_view: Option<Value> = None;
    let mut zoomed = 0;
    let mut saw_own = false;
    for (i, adj) in std::iter::once(None)
        .chain(VIEW_RETRIES.iter().map(Some))
        .enumerate()
    {
        if let Some(&(dx, dy, notches)) = adj {
            if entity.is_none() || !req.rotate_camera.unwrap_or(true) {
                break;
            }
            if start_view.is_none() {
                start_view = io.camera_readout().await.ok();
            }
            zoomed += notches;
            let level = io
                .look(dx, dy, notches * super::camera::WHEEL_NOTCH)
                .await
                .map_err(|e| steps.fail(io.now_ms(), "view_retry", e))?;
            steps.used(level);
            io.sleep(super::camera::LOOK_SETTLE_MS).await;
            adjusted.push(json!({ "retry": i, "yaw_counts": dx, "pitch_counts": dy, "zoom_notches": notches }));
        }
        let (c, t) = hover_once(
            io,
            &mut steps,
            &mut aim,
            entity,
            units.mouse_over,
            units.player,
            req.force,
        )
        .await?;
        saw_own |= t.iter().any(|h| h["hover"] == "own_avatar");
        tried = t;
        chosen = c;
        if !wants_view_retry(&chosen, &tried, near, req.force) {
            break;
        }
    }
    let mut hover_detail = json!({ "tried": tried });
    if !adjusted.is_empty() {
        hover_detail["view_retries"] = json!(adjusted);
    }
    if saw_own {
        hover_detail["saw_own_avatar"] = json!(true);
    }
    // No retry verified a hover: put the view back as it was (review of
    // #1309; yaw already nets to zero, zoom and a clamped pitch do not), and
    // look again from the first view, whose unverified candidate (a `Nothing`
    // pass, or a forced one) is the one to click.
    if !matches!(chosen, Some((_, Hover::Matched))) && !adjusted.is_empty() {
        let now = io.camera_readout().await.ok();
        let dy = match (&start_view, &now) {
            (Some(a), Some(b)) => restore_pitch_counts(a, b),
            _ => 0,
        };
        let restored = io
            .look(0, dy, -zoomed * super::camera::WHEEL_NOTCH)
            .await
            .is_ok();
        hover_detail["view_restored"] = json!(restored);
        if restored {
            io.sleep(super::camera::LOOK_SETTLE_MS).await;
            let (c, t) = hover_once(
                io,
                &mut steps,
                &mut aim,
                entity,
                units.mouse_over,
                units.player,
                req.force,
            )
            .await?;
            hover_detail["tried_after_restore"] = json!(t);
            chosen = c;
        }
    }
    steps.record("hover", t_hover, io.now_ms(), hover_detail.clone());
    let Some((pixel, hover)) = chosen else {
        return Err(steps.fail_with(
            io.now_ms(),
            "hover",
            if saw_own {
                "your own avatar covers the target at every height tried, even after the view retries: stand 3.5-4.5 m to the side of it (or turn the camera with client_camera yaw_offset_deg) and click with rotate_camera=false; force=true clicks anyway"
            } else {
                "every point on the target is off screen or covered by another entity, after the view retries (force=true clicks anyway)"
            },
            hover_detail,
        ));
    };
```
   The `ClickPlan` struct and everything after is unchanged in this packet.

c. Do NOT change `tool_tests.rs` (its `an_occluder_stops_the_click` must pass unmodified: the crate stays aligned with the guard under any yaw, so every retry still fails, the restore look is still the last look, and the state still contains `other_entity`:20, `view_retries` and `"view_restored":true`).

### Step 3: sim and tests
`sim.rs`: add to `Sim` (after `live_pitch_clamp`/`pitch_inverted` from LH-06):
```rust
    /// While the camera looks within this many radians of the bearing to the
    /// entity, the avatar stands in front of it: mouse-over reports the
    /// player. (The usual third-person cover.)
    pub avatar_hides: Option<(u32, f64)>,
```
init `avatar_hides: None,` in `Sim::new`. In `under_cursor`, replace the last line `best.map(|b| b.1).unwrap_or(0)` with:
```rust
        let hit = best.map(|b| b.1).unwrap_or(0);
        if let Some((id, half)) = self.avatar_hides {
            let off = self
                .entities
                .iter()
                .find(|e| e.id == id)
                .map(|e| wrap_pi(super::geometry::bearing(self.pos, e.pos) - self.cam_yaw).abs());
            if hit == id && off.is_some_and(|o| o < half) {
                return PLAYER_ID;
            }
        }
        hit
```
`mod.rs`: after `mod camera_pitch_tests;` add `#[cfg(test)]\nmod occlusion_tests;`.

New file `crates/lab/src/supervisor/world/occlusion_tests.rs`:
```rust
//! The avatar standing between the camera and the target, and targets that
//! hover nothing near the avatar (#1660 item 7).

use serde_json::Value;

use super::click::{self, ClickRequest, Expect};
use super::geometry::Vec3;
use super::sim::Sim;

/// The `hover` step's detail from a result or an error's steps.
fn hover_detail(steps: &[Value]) -> &Value {
    &steps
        .iter()
        .find(|s| s["step"] == "hover")
        .expect("a hover step")["detail"]
}

/// A medic 10 m ahead with a window, hidden behind the avatar while the
/// camera looks within about 17 degrees of it.
fn medic_behind_the_avatar() -> Sim {
    let mut s = Sim::new().with_entity(11, "Medic Ogilvie", Vec3::new(1000.0, 0.0, 0.0));
    s.entities[0].window = Some("DialogWin".into());
    s.avatar_hides = Some((11, 0.3));
    s
}

#[tokio::test]
async fn a_target_behind_the_avatar_is_clicked_after_a_yaw_retry() {
    let mut sim = medic_behind_the_avatar();
    let out = click::run(
        &mut sim,
        "client_world_click",
        ClickRequest {
            entity_id: Some(11),
            ..ClickRequest::default()
        },
        None,
    )
    .await
    .unwrap();
    assert_eq!(out["hover_verified"], true, "{out}");
    assert_eq!(out["result"]["windows_opened"][0], "DialogWin");
    let steps = out["steps"].as_array().unwrap();
    let detail = hover_detail(steps);
    assert_eq!(detail["saw_own_avatar"], true, "{detail}");
    // The first change of view is a pure yaw turn, not a pitch.
    assert_eq!(detail["view_retries"][0]["yaw_counts"], 250, "{detail}");
    assert_eq!(detail["view_retries"][0]["pitch_counts"], 0, "{detail}");
    assert_eq!(detail["view_retries"].as_array().unwrap().len(), 1);
}

/// With nowhere to go (force off, rotation off) the failure names the avatar.
#[tokio::test]
async fn an_avatar_that_cannot_be_dodged_is_named() {
    let mut sim = medic_behind_the_avatar();
    let e = click::run(
        &mut sim,
        "client_world_click",
        ClickRequest {
            entity_id: Some(11),
            rotate_camera: Some(false),
            ..ClickRequest::default()
        },
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(e.step, "hover");
    assert!(e.message.contains("own avatar"), "{}", e.message);
    assert!(sim.buttons.is_empty());
}

/// An entity that never answers mouse-over, close to the avatar: the view is
/// changed five times and put back, then the candidate is clicked anyway.
#[tokio::test]
async fn a_near_target_that_hovers_nothing_retries_then_clicks_the_candidate() {
    let mut sim = Sim::new().with_entity(11, "Corpse", Vec3::new(400.0, 0.0, 0.0));
    sim.unhoverable.push(11);
    let e = click::run(
        &mut sim,
        "client_world_click",
        ClickRequest {
            entity_id: Some(11),
            expect: Some(Expect::Target),
            settle_ms: Some(300),
            ..ClickRequest::default()
        },
        None,
    )
    .await
    .unwrap_err();
    // The click happened (the failure is the observe step, not the hover).
    assert_eq!(e.step, "observe");
    assert_eq!(sim.buttons, vec![(1, true), (1, false)]);
    let detail = hover_detail(&e.steps);
    assert_eq!(
        detail["view_retries"].as_array().unwrap().len(),
        click::VIEW_RETRIES.len(),
        "{detail}"
    );
    assert_eq!(detail["view_restored"], true, "{detail}");
    assert!(detail["tried_after_restore"].is_array(), "{detail}");
}

/// The same entity 20 m away keeps the old behaviour: no retries, click.
#[tokio::test]
async fn a_far_target_that_hovers_nothing_is_clicked_without_retries() {
    let mut sim = Sim::new().with_entity(11, "Corpse", Vec3::new(2000.0, 0.0, 0.0));
    sim.unhoverable.push(11);
    let e = click::run(
        &mut sim,
        "client_world_click",
        ClickRequest {
            entity_id: Some(11),
            expect: Some(Expect::Target),
            settle_ms: Some(300),
            ..ClickRequest::default()
        },
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(e.step, "observe");
    assert_eq!(sim.buttons, vec![(1, true), (1, false)]);
    assert!(hover_detail(&e.steps)["view_retries"].is_null());
}
```
Why they fail when reverted: with the old order (pitch first) the first test never gets a verified hover (two pitch retries put the target off screen, the later yaw retries run with the camera at the clamp, the final retry returns to the covering view) and ends in a `hover` error; with the old `chosen.is_some() => break` the third test has no `view_retries`; with the new rule applied to every target the fourth test would have retries; without the `own_avatar` label the first and second tests lose `saw_own_avatar` and the message.

## Lane checks
```
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo fmt -p cimmeria-lab
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo clippy -p cimmeria-lab --all-targets -- -D warnings
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-lab occlusion_tests click_hover
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-lab supervisor::world
```
If a retry-order assumption in the new sim tests is off (for instance the avatar cover test fails), report the failing assertion and the `sim.looks` list; do not loosen the assertions.

## Docs
`docs/guides/live-research-lab.md`:
- "Clicking" bullet (line 712, anchor `- **Clicking** places the CEGUI cursor on the projected point`): append: ` When no height hovers the entity (another entity under the cursor, your own avatar, or nothing while the target is within 8 m of you) it changes the view and tries again: yaw about 27 degrees each way first, then pitch up twice, then zoom in, and puts the view back if none helps (\`view_retries\`, \`view_restored\`, \`tried_after_restore\` in the hover step). A hover that lands on your own avatar shows as \`own_avatar\` in \`tried\`, with \`saw_own_avatar: true\` in the step, and the failure says to stand to the side. \`rotate_camera: false\` turns the retries off.`
- "The avatar eats clicks" bullet (line 722, anchor `- **The avatar eats clicks.**`): append: ` A click now dodges it by itself (yaw retries first); standing off to the side is still the reliable way for a fixed pose.`
`.claude/agents/lab-driver.md` line 39 (anchor `A hover answering \`other_entity: 2\` is your own avatar: reposition (step 2), don't \`force\`.`): replace that sentence with: `A hover reported as \`own_avatar\` (older builds: \`other_entity\` with your own id) means your avatar covers the target: the click already retried from the sides; if it still fails, reposition (step 2) and don't \`force\`.`

## Live smoke
Avatar 3.4 m from the Debug Area trainer (open floor, teleport, then `client_camera {"face_entity_id":T}` so the trainer sits straight behind the avatar): `client_world_click {"entity_id":T,"expect":"window","settle_ms":5000}`. Expect success; the `hover` step detail has `saw_own_avatar: true` (if P2 step 3 said `own`) and `view_retries[0].yaw_counts == 250`. Server evidence (ClickHouse, not `server_log_tail`): `scope_name = 'abilities' AND event = 'trainer_open'` for the player, one row. A tighter check: the same call at 3.4 m on the old build failed at step `hover`.

## Commit subject
`fix(lab): world_click dodges the avatar with yaw first and retries near targets that hover nothing (LH-07)`

---

# LH-08: world_click reports the interaction range (item 9)

**Size S. Depends on: LH-07** (same files: `click.rs`, `sim.rs`, `mod.rs`, `server/world.rs`; and it uses `distance_m` and `range_m` from LH-07).

The server refuses an interact beyond 5.0 m straight-line (`MAX_INTERACT_DISTANCE`, `interact_range.rs:13`). This is a report-only fix: no auto-approach, because `client_move_to` is itself unreliable in the hub (item 8, other packets). See the owner decision at the end.

## Files
- `crates/lab/src/supervisor/world/click.rs`
- `crates/lab/src/supervisor/world/sim.rs` (a range knob)
- `crates/lab/src/supervisor/world/mod.rs` (register a test module)
- `crates/lab/src/supervisor/world/range_tests.rs` (new)
- `crates/lab/src/server/world.rs` (tool description)
- `docs/guides/live-research-lab.md`, `.claude/agents/lab-driver.md`

## Steps
1. `click.rs`. After the `FALLBACK_SLOT` constant (anchor `const FALLBACK_SLOT: i32 = LAB_SLOT_BASE - 1;`) add:
```rust
/// The server's interaction range, metres, straight line
/// (`MAX_INTERACT_DISTANCE` in `cimmeria-cell-world`, `interact_range.rs`): a
/// right-click on an NPC farther away than this opens nothing, and the server
/// logs only an info line. Mirrored here because the lab crate does not
/// depend on the cell crates.
pub const INTERACT_RANGE_M: f64 = 5.0;

/// `{distance_m, interact_range_m, in_interact_range}` for a result, or null
/// for a point click.
fn range_json(range_m: Option<f64>) -> Value {
    match range_m {
        Some(d) => json!({
            "distance_m": (d * 100.0).round() / 100.0,
            "interact_range_m": INTERACT_RANGE_M,
            "in_interact_range": d <= INTERACT_RANGE_M,
        }),
        None => Value::Null,
    }
}

/// Why a right-click may have opened nothing: the target was out of range.
fn range_hint(button: usize, range_m: Option<f64>) -> String {
    match range_m {
        Some(d) if button == 1 && d > INTERACT_RANGE_M => format!(
            "; the target is {d:.1} m away, beyond the server's {INTERACT_RANGE_M} m interact range: stand 3.5-4.5 m from it"
        ),
        _ => String::new(),
    }
}
```
2. `ClickPlan` (anchor `struct ClickPlan {`): add a field `range_m: Option<f64>,` after `found: Option<Found>,`. In `run`, where `ClickPlan { ... }` is built (anchor `let plan = ClickPlan {`), add `range_m,` after `found,`. (`range_m` was computed by LH-07 step 2a.)
3. `click_and_observe`: add `range_m,` to the destructuring (after `found,`). In the `let evidence = json!({ ... });` add the entry `"range": range_json(range_m),` after `"mouse_over_after": after.mouse_over,`. Replace the failure at the end of the `if !ok { ... }` block with:
```rust
    if !ok {
        return Err(steps.fail_with(
            io.now_ms(),
            "observe",
            format!(
                "the click did not produce the expected {expect:?} within {settle} ms{}",
                range_hint(button, range_m)
            ),
            evidence,
        ));
    }
```
4. Tests in `click.rs`'s `mod tests` (unit):
```rust
    #[test]
    fn a_far_right_click_gets_a_range_hint() {
        assert!(range_hint(1, Some(6.4)).contains("6.4 m away"));
        assert!(range_hint(1, Some(6.4)).contains("5 m interact range"));
        assert_eq!(range_hint(0, Some(6.4)), "");
        assert_eq!(range_hint(1, Some(4.2)), "");
        assert_eq!(range_hint(1, None), "");
        assert_eq!(range_json(Some(4.2))["in_interact_range"], true);
        assert_eq!(range_json(Some(6.4))["in_interact_range"], false);
        assert!(range_json(None).is_null());
    }
```
5. `sim.rs`: add to `Sim` `pub interact_range_m: Option<f64>,` (doc: "A right-click opens an entity's window only within this many metres, straight line, like the server"), `interact_range_m: None,` in `new`, add `distance_m` to the `use super::geometry::{...}` list, and in `Sim::button` replace the right-click window block
```rust
                if button == 1 {
                    if let Some(w) = self
                        .entities
                        .iter()
                        .find(|e| e.id == under)
                        .and_then(|e| e.window.clone())
                    {
                        if !self.windows.contains(&w) {
                            self.windows.push(w);
                        }
                    }
                }
```
with
```rust
                if button == 1 {
                    let in_range = self.interact_range_m.is_none_or(|r| {
                        self.entities
                            .iter()
                            .find(|e| e.id == under)
                            .is_some_and(|e| distance_m(self.pos, e.pos) <= r)
                    });
                    if let Some(w) = self
                        .entities
                        .iter()
                        .find(|e| e.id == under)
                        .and_then(|e| e.window.clone())
                        .filter(|_| in_range)
                    {
                        if !self.windows.contains(&w) {
                            self.windows.push(w);
                        }
                    }
                }
```
6. `mod.rs`: after `mod occlusion_tests;` add `#[cfg(test)]\nmod range_tests;`.
7. `server/world.rs`: in the `client_world_click` description (anchor `Reports native_level."` at the end of that tool's description) change to: `Reports the straight-line distance to the entity and whether it is inside the server's 5 m interact range (result.range); a right-click that opens nothing from farther away says so. Reports native_level."`

## Test
New file `crates/lab/src/supervisor/world/range_tests.rs`:
```rust
//! `client_world_click` reports the server's interaction range (#1660 item 9).

use super::click::{self, ClickRequest, Expect};
use super::geometry::Vec3;
use super::sim::Sim;

/// A medic `x` UE3 units straight ahead; the sim only opens its window
/// within 5 m, like the server.
fn medic_at(x: f64) -> Sim {
    let mut s = Sim::new().with_entity(11, "Medic Ogilvie", Vec3::new(x, 0.0, 0.0));
    s.entities[0].window = Some("DialogWin".into());
    s.interact_range_m = Some(5.0);
    s
}

fn window_click() -> ClickRequest {
    ClickRequest {
        entity_id: Some(11),
        expect: Some(Expect::Window),
        settle_ms: Some(300),
        ..ClickRequest::default()
    }
}

#[tokio::test]
async fn a_right_click_from_beyond_the_range_says_why_it_opened_nothing() {
    let mut sim = medic_at(2000.0);
    let e = click::run(&mut sim, "client_world_click", window_click(), None)
        .await
        .unwrap_err();
    assert_eq!(e.step, "observe");
    assert!(e.message.contains("20.0 m away"), "{}", e.message);
    assert!(e.message.contains("5 m interact range"), "{}", e.message);
    let state = e.to_json()["state"].clone();
    assert_eq!(state["range"]["in_interact_range"], false, "{state}");
    assert_eq!(state["range"]["distance_m"], 20.0, "{state}");
}

#[tokio::test]
async fn a_right_click_inside_the_range_reports_the_distance() {
    let mut sim = medic_at(400.0);
    let out = click::run(&mut sim, "client_world_click", window_click(), None)
        .await
        .unwrap();
    assert_eq!(out["result"]["windows_opened"][0], "DialogWin", "{out}");
    assert_eq!(out["result"]["range"]["distance_m"], 4.0, "{out}");
    assert_eq!(out["result"]["range"]["in_interact_range"], true, "{out}");
}
```
Why they fail when reverted: without `range_hint`/`range_json` the first test's message and `state.range` are missing; the second needs `result.range`. (If someone removes the sim range, the first test would open the window and not fail at observe, so the sim knob is part of the guard.)

## Lane checks
```
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo fmt -p cimmeria-lab
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo clippy -p cimmeria-lab --all-targets -- -D warnings
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-lab range_tests click::tests
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-lab supervisor::world
```
`click.rs` ends near 600 lines after LH-07's split; confirm it is still under 700 (`wc -l`-equivalent: it must be).

## Docs
`docs/guides/live-research-lab.md`:
- World-tools row `client_world_click` (line 698, anchor `| \`client_world_click\` | as above plus \`point\`, \`button\` (\`right\`), \`expect\``): append to the Result cell: `; \`result.range\` = \`{distance_m, interact_range_m: 5, in_interact_range}\` (straight line, entity clicks only), and a right-click that opened nothing from beyond 5 m says so in its error`.
- After the "Native level." paragraph (line 705) add a paragraph:
  `**Interaction range.** The server refuses an interact, a vendor or a trainer open from more than 5.0 m away, measured in a straight line in 3D (\`MAX_INTERACT_DISTANCE\`), and only logs an info line. In the Haiku UAT round of 2026-10-12 a right-click at 6.4 m hovered the trainer and opened nothing; at 4.2 m it worked. \`client_world_click\` now reports \`result.range\`, and a failed right-click from beyond 5 m names the distance. Stand 3.5 to 4.5 m away: far enough that the camera is not inside your avatar, near enough to be well inside the range. \`client_entity_find\`'s \`distance_m\` is horizontal; the range check is not.`
`.claude/agents/lab-driver.md` line 36 (anchor `2. Get within 3-3.5 m: the server's interact range is 5.0 m, and closer than that the camera ends up inside your own body.`): replace with `2. Get within 3.5-4.5 m: the server's interact range is 5.0 m in a straight line (a right-click at 6.4 m opened nothing, at 4.2 m it worked), and closer than about 3.5 m the camera ends up inside your own body.` Leave the rest of that line (the \`arrival_m\` advice) as is, but change \`arrival_m: 3.5\` to \`arrival_m: 4\` in the same sentence.

## Live smoke
Debug Area trainer (look up its id by name): stand 6.4 m away, `client_world_click {"entity_id":T,"expect":"window","settle_ms":5000}`. Expect an error at step `observe` whose message says `6.4 m away ... 5 m interact range` and whose state has `range.in_interact_range == false`. Move to 4.2 m and repeat: expect success with `result.range.distance_m` about 4.2. Server evidence (ClickHouse): no `scope_name = 'abilities' AND event = 'trainer_open'` row for the first call and one for the second; the first call's rejection is an info line from `interact_target_in_range` (distance in world units).

## Commit subject
`fix(lab): world_click reports the server's 5 m interaction range and names it when a right-click opens nothing (LH-08)`

---

## Notes for the coordinator

- Owner decision (recommended): should `client_world_click` also **approach** a target that is out of range? I recommend not now: LH-08 only reports. An `approach` option would walk with `client_move_to`, which item 8 shows is unreliable in the Castle_CellBlock hub and drifts 15-35 m. Revisit after the move_to packets land; briefs stand 3.5-4.5 m meanwhile (teleport, then click).
- Owner decision (recommended yes): LH-07 makes a near (<= 8 m) target that hovers nothing cost five view retries (about 5-8 s) before its candidate is clicked. `rotate_camera: false` is the escape. If that is too slow for floor bodies, shrink `NEAR_AVATAR_M` in `click_hover.rs` to 5.0 (the interaction range) instead of removing the rule.
- `docs/guides/live-research-lab.md` line 738 (open question 7: "is that camera actor the controller or the pawn?") and the zero-pose finding should be updated by the close-out once P1 answers it.
- Nothing here touches the product server or the client.
