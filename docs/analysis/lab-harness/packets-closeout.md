# LH expansion E: round-2 findings (packets LH-19 and LH-20)

Worktree: `<wt>` (branch lab-harness/integration). Crate package `cimmeria-lab` (`crates/lab`). Paths are relative to the worktree. docs/**/*.md are CRLF.

Lane prefix (`<wt>` = the packet's own worktree):

```
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo fmt -p cimmeria-lab
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo clippy -p cimmeria-lab --all-targets -- -D warnings
pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-lab <filter>
```

## Summary of findings (all probed live on `default`, Bankone, Castle_CellBlock, 2026-10-12 ~13:30Z)

| Item | Root cause | Evidence | Sure? |
|---|---|---|---|
| 8b `client_move_to` never turns, walks away | The pawn's heading answers only to the turn keys. `io.look` is the camera's own turn handler (`native_look`, `CAMERA_TURN_YAW_VA`); it moves the camera yaw offset and never the pawn. `W` walks along the pawn heading, not the camera's. | `client_camera yaw_counts 600`: `yaw_offset_deg` 0 -> 65.92, `heading_deg` unchanged (49.71 -> 49.71). Then `W` held 600 ms: displacement bearing atan2(276, 234) = 49.7 deg = the unchanged pawn heading, not camera+heading. `client_entity_find` `yaw_deg` (the memory pose the controller reads) tracks `heading_deg` exactly (49.7). | 98% |
| 8b rates | Turn keys rotate the pawn at about 110 deg/s (1.92 rad/s), E raises the yaw (clockwise seen from above is not claimed; the yaw convention is the memory one, +X toward +Y). Q lowers it. Standing and walking turns both work. | E 500 ms: 0 -> 55.48. E 1000 ms: +104.35. Q 1000 ms: -110.12. Q 2000 ms: -220 (110/s). E 250 ms: +31.8 (127/s, includes about 30 ms of tail). `W` down + E 500 ms: the walk curved (displacement bearing 31 deg between start and the final 57.66). | 99% |
| 8b bindings | `getBindingKey('TurnLeft', 1)` = `{key=81, vkeyText='Q'}`, slot 2 = Delete (46). `('TurnRight', 1)` = E (69), slot 2 = PageDown (34). `MoveForward` = W (87), slot 2 = Up (38). Slots are 1 and 2 (slot 0 returns an empty table; an unknown action also returns an empty table, no error). | `client_lua_eval` dump of the table fields. | 99% |
| 20 vault drag never starts | **Window overlap, not the vault Lua.** `.bank` shows `VaultWin` rect [445,210,834,498]; opening the bag (what `reveal_slot` does for the other end) shows `InventoryWin` at [445,210,834,510], the same place, raised above it. `Vault_Slot1` [460,340,494,374] and `Inventory_Slot1` [460,352,494,386] overlap. The press at the Vault_Slot1 centre (477,357) lands on `Inventory_Slot1` (the topmost window there), so the vault slot's `DragContainer` never sees a press. The vault Lua (`Vault.lua:481-536`, `:966-972`) is the same shape as the inventory's and is fine. | After `VaultWin:moveToFront()` (one Lua call) the same `client_drag_drop` Vault_Slot1 -> Inventory_Slot1 gave `drag_started:true, drag_started_at_step:6`, and `vault 1 -> crafting 1` gave `moved:true effect_ok:true` (a swap; swapped back the same way, state restored: Crafting1 Steel Core 10320 qty 6, Vault1 10305 qty 1). The inventory-first direction worked in the close-out because the bag is the window on top. Step 6 of 8 for a 12 px path is just the CEGUI drag threshold. | 95% |
| `lab_logout` os error 10053 | Not caused by this branch. `logout_flow` is byte-identical to main (`git diff main..HEAD` on `flows/login.rs` touches only `select_server_chunk` / `pick_server`). The error text is the bridge TCP socket (`bridge: ... os error 10053`) being aborted locally while the world is torn down. It is intermittent: the same flow succeeded right now in 7.5 s (`send_logout` 1.0 s, `character_select` 5.4 s) on the same build. CS-08's worknote already records `lab_logout` "often times out on its first try" (`docs/analysis/class-start-v6/worknotes/CS-08.md:79`), before D-LH15. | live re-run; git diff | Pre-existing 85%. Possible branch contributor, unproven: the event keeper (on by default since LH-11, this branch) keeps ticking while `login == InWorld`, and `logout_flow` does not leave `InWorld` before sending `/logout`, so a tick (an `events_read` plus a pump `lua_eval`) can be on the wire as the UI reloads. D-LH15 does not cover it. See the small mitigation at the end; it is not the cause until proven. |
| `lab_ensure_in_world` "name did not read for 15 s" | Not caused by this branch. `observe_chunk` / `plan` / `MAX_SETTLES = 15` are untouched (the branch adds one `set_login_state(InWorld)` line in the `Next::Done` arm). `unitName(Unit.Player)` reads empty until the player entity is bound after zone-in, so a slow load (CPU or disk starved: see the SV-15 pause, project memory) keeps it empty past 15 settles. It did not reproduce: `lab_ensure_in_world` after `lab_logout` just now returned in 12.4 s. D-LH15 only makes the keeper quieter during loads (it ticks from `login == InWorld`, which `flows/world.rs:178` sets right after Play, so it may still tick during the settle; a tick holds the bridge up to 3 s, which can delay `observe`'s reads but cannot blank a name). LH-15 (lane wait) acts only in `launch_client` and cannot touch an in-world call. | read the code, live re-run | Pre-existing 80%; cause (load) is a hypothesis, not shown. |

Expansion: `client_camera` / face still turn only the camera (that works and is right for aiming a click). Only `client_move_to` needs the pawn's heading, so LH-19 changes only the walk. The note in `.claude/agent-memory/main-session/reference_lab_client_controls.md` ("Lab mouse-look does not move the camera") is out of date for this build: `native_look` moves the camera yaw offset (65.92 deg for 600 counts, gain 20 rotator units per count). Update it in LH-19.

Dispatch: LH-19 (L) and LH-20 (S) touch disjoint files and can run in parallel. LH-19 edits `steer.rs`, which LH-10 (merged) and LH-06 also edit: rebase on the integration branch first. Neither depends on a live probe; the probes above are done.

---

## Packet LH-19: `client_move_to` turns the pawn with the bound turn keys

**ID / title / size:** LH-19, `client_move_to` steers with the turn keys (Q/E as bound), closed loop on the pawn's memory yaw, L. Depends on: none (rebase on lab-harness/integration).

**Design:** One controller change and one IO method.

- `MoveController` stops producing mouse counts. Each step it returns a `TurnCmd`: `Off`, `Hold(key)` for errors of 0.35 rad (20 deg) or more, `Pulse(key, ms)` for smaller ones (`ms = error / rate * 0.9`, clamped to 20..300). Holding only needs one key edge per tick; the pulse gives a precise finish inside one tick.
- The turn rate is learned from the yaw change between ticks while a key was held (`TurnRate`, starts at 1.92 rad/s, the measured 110 deg/s) so a different frame rate or a slowed pawn costs one sample. The key-to-sign mapping is learned too: two samples in a row that turned the wrong way flip it (`right_sign`), so a client where E lowers the yaw still works.
- 10 consecutive ticks of `Hold` with no yaw change (about 1.2 s) end the walk with `MoveOutcome::NoTurn` and a failure that says so (a dialog, cast or stun roots the pawn, or the bindings are not Q/E and not readable). It never walks on blind.
- The key names come from the client once per walk: `getBindingKey('TurnLeft'|'TurnRight', 1|2)`, first bound slot without modifiers, mapped to a lab key by `combat::binding::lab_key_for_vk`; any miss falls back to Q/E.
- Forward policy is unchanged: walk while the heading error is at most `turn_in_place_rad` (1.0 rad), otherwise turn standing first (both work on the live client).
- Mouse-look is removed from `client_move_to`, not kept as a fallback: it is shown not to turn the pawn. `TurnModel` and `FaceController` stay for the camera tools.
- Sim: the live pawn model is a new constructor `Sim::live()` so hover/camera tests keep their old pawn behaviour.

**Files**
1. `crates/lab/src/supervisor/world/turn_keys.rs` (new).
2. `crates/lab/src/supervisor/world/mod.rs`: add `mod turn_keys;` beside `mod steer;`.
3. `crates/lab/src/supervisor/world/steer.rs`: replace the `MoveConfig` .. `MoveController` block (lines 91-293) and its tests (lines 456-559 except the unstick test body, kept).
4. `crates/lab/src/supervisor/world/movement.rs`.
5. `crates/lab/src/supervisor/world/io.rs`: `WorldIo::turn_keys` + `LiveWorld` impl.
6. `crates/lab/src/supervisor/world/sim.rs`: live pawn model + `turn_keys`.
7. `crates/lab/src/supervisor/world/move_to_tests.rs`: use `Sim::live()`, new tests.
8. `docs/guides/live-research-lab.md` (CRLF) and `.claude/agent-memory/main-session/reference_lab_client_controls.md`.

**Steps**

1. Create `crates/lab/src/supervisor/world/turn_keys.rs`:
   ```rust
   //! The pawn's turn keys, as the client binds them.
   //!
   //! The pawn's heading answers only to these keys (live 2026-10-12: the
   //! camera's mouse-look handler moved the camera yaw offset 65.9 degrees
   //! and the pawn heading not at all; Q/E turn it at about 110 degrees a
   //! second, standing or walking). `getBindingKey(action, 1|2)` returns a
   //! table `{key = <virtual-key code>, ...}`; an unknown or unbound action
   //! returns an empty table, not an error.

   use crate::supervisor::combat::binding::lab_key_for_vk;

   pub const DEFAULT_LEFT: &str = "Q";
   pub const DEFAULT_RIGHT: &str = "E";

   /// Lua: the first and second binding of `TurnLeft` and `TurnRight` as
   /// virtual-key codes (`'0'` = unbound, or needs a modifier). Returns four
   /// values: left 1, right 1, left 2, right 2.
   pub const TURN_KEYS_CHUNK: &str = "local function k(a, n) \
        local ok, t = pcall(getBindingKey, a, n) \
        if ok and type(t) == 'table' then \
          local v = tonumber(t.key) \
          if v ~= nil and v > 0 and not t.shift and not t.control and not t.alt then \
            return tostring(math.floor(v)) end end \
        return '0' end \
        return k('TurnLeft', 1), k('TurnRight', 1), k('TurnLeft', 2), k('TurnRight', 2)";

   /// `(left, right)` lab key names from [`TURN_KEYS_CHUNK`]'s four results:
   /// the first bound slot the lab can press, else the stock Q / E.
   pub fn parse_turn_keys(r: &[String]) -> (String, String) {
       let pick = |a: usize, b: usize, default: &str| {
           [a, b]
               .iter()
               .filter_map(|i| r.get(*i)?.parse::<u32>().ok())
               .filter(|v| *v > 0)
               .find_map(lab_key_for_vk)
               .unwrap_or_else(|| default.to_string())
       };
       (pick(0, 2, DEFAULT_LEFT), pick(1, 3, DEFAULT_RIGHT))
   }

   #[cfg(test)]
   mod tests {
       use super::*;

       fn s(v: &[&str]) -> Vec<String> {
           v.iter().map(|x| x.to_string()).collect()
       }

       #[test]
       fn the_stock_bindings_are_q_and_e() {
           assert_eq!(parse_turn_keys(&s(&["81", "69", "46", "34"])), ("Q".into(), "E".into()));
       }

       #[test]
       fn an_unbound_first_slot_falls_to_the_second() {
           assert_eq!(parse_turn_keys(&s(&["0", "69", "46", "0"])), ("Delete".into(), "E".into()));
       }

       #[test]
       fn nothing_readable_means_q_and_e() {
           assert_eq!(parse_turn_keys(&[]), ("Q".into(), "E".into()));
           // A code the lab cannot press (999) is ignored.
           assert_eq!(parse_turn_keys(&s(&["999", "0", "0", "0"])), ("Q".into(), "E".into()));
       }

       #[test]
       fn the_chunk_asks_for_both_actions_and_both_slots() {
           for needle in ["'TurnLeft', 1", "'TurnRight', 1", "'TurnLeft', 2", "'TurnRight', 2"] {
               assert!(TURN_KEYS_CHUNK.contains(needle), "{needle}");
           }
       }
   }
   ```
   `lab_key_for_vk(0x2E)` is "Delete" and `(0x22)` "PageDown" per `combat/binding.rs`; only names `keys::named` knows are handed out.

2. `world/mod.rs`: add `mod turn_keys;` next to `mod steer;`. (`combat::binding` is already `pub mod binding;` at `combat/mod.rs:17`.)

3. `steer.rs`: replace lines 91-293 (from the `/// Tuning for [`MoveController`].` doc through the end of `impl MoveController`) with the block below. `Unstick` and `MoveOutcome` stay where they are except for the new variant; reproduce them as shown.
   ```rust
   /// Turn-rate model for the pawn's turn keys.
   ///
   /// Live: about 1.92 rad/s (110 deg/s), the same standing and walking.
   pub const DEFAULT_TURN_RATE: f64 = 1.92;
   const RATE_BOUNDS: (f64, f64) = (0.2, 10.0);
   /// A hold shorter than this, or a turn smaller than [`MIN_RATE_RAD`], teaches nothing.
   const MIN_RATE_DT_MS: u64 = 50;
   const MIN_RATE_RAD: f64 = 0.02;
   /// Under this much yaw change over a held tick the pawn "did not turn".
   const NO_TURN_RAD: f64 = 0.005;
   const MIN_PULSE_MS: u64 = 20;
   const MAX_PULSE_MS: u64 = 300;

   /// A turn key, by what it does to the yaw once the sign is known.
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum TurnKey {
       Left,
       Right,
   }

   /// What to do with the turn keys this step.
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum TurnCmd {
       /// Neither key down.
       Off,
       /// Keep this key down until the next step.
       Hold(TurnKey),
       /// Press this key for this many ms inside the step, then let go.
       Pulse(TurnKey, u64),
   }

   /// Learned turn rate and key sign.
   #[derive(Debug, Clone, Copy, PartialEq)]
   pub struct TurnRate {
       /// Radians per second while a turn key is held.
       pub rad_s: f64,
       /// +1.0: the Right key raises the yaw (stock: E turns toward +Y).
       pub right_sign: f64,
       pub samples: u32,
       pub sign_flips: u32,
       wrong_run: u32,
   }

   impl Default for TurnRate {
       fn default() -> Self {
           Self::new(DEFAULT_TURN_RATE)
       }
   }

   impl TurnRate {
       pub fn new(rad_s: f64) -> Self {
           Self {
               rad_s,
               right_sign: 1.0,
               samples: 0,
               sign_flips: 0,
               wrong_run: 0,
           }
       }

       fn sign_of(&self, key: TurnKey) -> f64 {
           match key {
               TurnKey::Right => self.right_sign,
               TurnKey::Left => -self.right_sign,
           }
       }

       /// The key that turns toward a heading error `err` (positive = the yaw
       /// must rise).
       pub fn key_for(&self, err: f64) -> TurnKey {
           if err * self.right_sign > 0.0 {
               TurnKey::Right
           } else {
               TurnKey::Left
           }
       }

       /// Learn from `key` held for `dt_ms` while the yaw changed by `dyaw`.
       /// Two samples in a row that turned the wrong way flip the key sign.
       /// Returns whether the sample taught the rate.
       pub fn observe(&mut self, key: TurnKey, dyaw: f64, dt_ms: u64) -> bool {
           if dt_ms < MIN_RATE_DT_MS || dyaw.abs() < MIN_RATE_RAD {
               return false;
           }
           let along = dyaw * self.sign_of(key);
           if along < 0.0 {
               self.wrong_run += 1;
               if self.wrong_run >= 2 {
                   self.right_sign = -self.right_sign;
                   self.sign_flips += 1;
                   self.wrong_run = 0;
               }
               return false;
           }
           self.wrong_run = 0;
           let r = (along / (dt_ms as f64 / 1000.0)).clamp(RATE_BOUNDS.0, RATE_BOUNDS.1);
           self.rad_s = if self.samples == 0 {
               r
           } else {
               0.5 * self.rad_s + 0.5 * r
           };
           self.samples += 1;
           true
       }

       pub fn to_json(self) -> Value {
           json!({
               "rad_s": (self.rad_s * 100.0).round() / 100.0,
               "deg_s": (self.rad_s.to_degrees() * 10.0).round() / 10.0,
               "right_sign": self.right_sign,
               "samples": self.samples,
               "sign_flips": self.sign_flips,
           })
       }
   }

   /// Tuning for [`MoveController`].
   #[derive(Debug, Clone, Copy, PartialEq)]
   pub struct MoveConfig {
       /// Done when the horizontal distance is at most this, metres.
       pub arrival_m: f64,
       /// Stuck when walking this long without getting `stuck_progress_m` closer.
       pub stuck_window_ms: u64,
       pub stuck_progress_m: f64,
       /// Unstick attempts (jump, strafe right, strafe left) before giving up.
       pub max_unsticks: u32,
       /// Heading errors above this turn standing still first.
       pub turn_in_place_rad: f64,
       /// Errors under this are corrected with a timed key pulse, above it a hold.
       pub pulse_below_rad: f64,
       /// Heading errors below this need no correction.
       pub deadband_rad: f64,
       /// Give up when walking has taken the avatar this far (metres) beyond the
       /// closest it has been to the goal: it is walking away, not snagged.
       pub diverge_m: f64,
       /// Give up after this many held-key ticks in a row with no yaw change.
       pub no_turn_ticks: u32,
   }

   impl Default for MoveConfig {
       fn default() -> Self {
           Self {
               arrival_m: 1.5,
               stuck_window_ms: 2500,
               stuck_progress_m: 0.5,
               max_unsticks: 3,
               turn_in_place_rad: 1.0,
               pulse_below_rad: 0.35,
               deadband_rad: 0.05,
               diverge_m: 6.0,
               no_turn_ticks: 10,
           }
       }
   }

   /// A way out of a snag, tried in this order.
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum Unstick {
       Jump,
       StrafeRight,
       StrafeLeft,
   }

   impl Unstick {
       pub fn nth(i: u32) -> Self {
           match i % 3 {
               0 => Unstick::Jump,
               1 => Unstick::StrafeRight,
               _ => Unstick::StrafeLeft,
           }
       }

       /// The key it presses (movement keys per the stock `SGWInput.ini`).
       pub fn key(self) -> &'static str {
           match self {
               Unstick::Jump => "Space",
               Unstick::StrafeRight => "D",
               Unstick::StrafeLeft => "A",
           }
       }

       pub fn name(self) -> &'static str {
           match self {
               Unstick::Jump => "jump",
               Unstick::StrafeRight => "strafe_right",
               Unstick::StrafeLeft => "strafe_left",
           }
       }
   }

   /// How a walk ended.
   #[derive(Debug, Clone, Copy, PartialEq)]
   pub enum MoveOutcome {
       Arrived {
           distance_m: f64,
       },
       Stuck {
           distance_m: f64,
           attempts: u32,
       },
       /// Farther from the goal than ever after walking: a wall deflecting the
       /// pawn, or a goal that is running away.
       Diverged {
           distance_m: f64,
           best_m: f64,
       },
       /// The turn keys were held for `ticks` ticks and the pawn's yaw never
       /// moved: rooted (dialog, cast, stun), or the bindings are not these keys.
       NoTurn {
           ticks: u32,
       },
   }

   /// What to do this step.
   #[derive(Debug, Clone, Copy, PartialEq)]
   pub struct MoveStep {
       pub turn: TurnCmd,
       /// Hold the forward key after this step.
       pub forward: bool,
       pub unstick: Option<Unstick>,
       pub outcome: Option<MoveOutcome>,
       pub distance_m: f64,
       pub heading_error: f64,
   }

   /// Walk toward a point: turn with the turn keys, hold forward, detect snags.
   #[derive(Debug, Clone)]
   pub struct MoveController {
       pub cfg: MoveConfig,
       pub rate: TurnRate,
       pub unsticks: u32,
       last: Option<(Pose, u64, TurnCmd, bool)>,
       no_turn: u32,
       best_m: f64,
       best_at: u64,
       min_m: f64,
   }

   impl MoveController {
       pub fn new(cfg: MoveConfig, rate: TurnRate) -> Self {
           Self {
               cfg,
               rate,
               unsticks: 0,
               last: None,
               no_turn: 0,
               best_m: f64::INFINITY,
               best_at: 0,
               min_m: f64::INFINITY,
           }
       }

       pub fn step(&mut self, now_ms: u64, pose: Pose, target: Vec3) -> MoveStep {
           let dist = horizontal_m(pose.pos, target);
           let err = yaw_error(pose.pos, pose.yaw, target);
           let mut out = MoveStep {
               turn: TurnCmd::Off,
               forward: false,
               unstick: None,
               outcome: None,
               distance_m: dist,
               heading_error: err,
           };

           // Learn from what the previous step's held key did.
           let was_forward = match self.last {
               Some((prev, t, cmd, fwd)) => {
                   if let TurnCmd::Hold(k) = cmd {
                       let dyaw = wrap_pi(pose.yaw - prev.yaw);
                       self.rate.observe(k, dyaw, now_ms.saturating_sub(t));
                       self.no_turn = if dyaw.abs() < NO_TURN_RAD {
                           self.no_turn + 1
                       } else {
                           0
                       };
                   }
                   fwd
               }
               None => false,
           };

           if dist <= self.cfg.arrival_m {
               out.outcome = Some(MoveOutcome::Arrived { distance_m: dist });
               self.last = None;
               return out;
           }
           if self.no_turn >= self.cfg.no_turn_ticks {
               out.outcome = Some(MoveOutcome::NoTurn { ticks: self.no_turn });
               return out;
           }

           // Never closer than this: remembered across turns and unsticks.
           self.min_m = self.min_m.min(dist);
           if was_forward && dist > self.min_m + self.cfg.diverge_m {
               out.outcome = Some(MoveOutcome::Diverged {
                   distance_m: dist,
                   best_m: self.min_m,
               });
               return out;
           }

           // Progress is only expected while walking: turning in place or a
           // fresh start restarts the window.
           if !was_forward || dist < self.best_m - self.cfg.stuck_progress_m {
               self.best_m = dist;
               self.best_at = now_ms;
           } else if now_ms.saturating_sub(self.best_at) >= self.cfg.stuck_window_ms {
               if self.unsticks >= self.cfg.max_unsticks {
                   out.outcome = Some(MoveOutcome::Stuck {
                       distance_m: dist,
                       attempts: self.unsticks,
                   });
                   return out;
               }
               out.unstick = Some(Unstick::nth(self.unsticks));
               self.unsticks += 1;
               self.best_m = dist;
               self.best_at = now_ms;
           }

           if err.abs() > self.cfg.deadband_rad {
               let key = self.rate.key_for(err);
               out.turn = if err.abs() < self.cfg.pulse_below_rad {
                   // 0.9: undershoot a little (the key has a short tail).
                   let ms = (err.abs() / self.rate.rad_s * 1000.0 * 0.9).round() as u64;
                   TurnCmd::Pulse(key, ms.clamp(MIN_PULSE_MS, MAX_PULSE_MS))
               } else {
                   TurnCmd::Hold(key)
               };
           }
           out.forward = err.abs() <= self.cfg.turn_in_place_rad;
           self.last = Some((pose, now_ms, out.turn, out.forward));
           out
       }
   }
   ```
   Keep `MIN_LEARN_COUNTS`, `MIN_LEARN_RAD`, `DEFAULT_COUNTS_PER_RAD` and `TurnModel` (the camera tools use them). Remove `FRAC_PI_2`/`PI` imports only if clippy reports them unused (they are used by `FaceController`; leave them).

4. `steer.rs` tests (lines 456-559). Keep `pose()`, `move_controller_arrives_inside_the_radius`, and `move_controller_unsticks_then_gives_up` as they are except `TurnModel::new(400.0)` -> `TurnRate::default()`. Do the same in `move_controller_gives_up_when_it_walks_away` and `move_controller_does_not_diverge_while_closing`. Delete `move_controller_turns_toward_and_walks` and `move_controller_walks_while_turning_when_standing_turns_do_nothing`; add:
   ```rust
   #[test]
   fn move_controller_holds_the_key_that_turns_toward_the_goal() {
       let mut c = MoveController::new(MoveConfig::default(), TurnRate::default());
       // Straight ahead: walk, no turn.
       let s = c.step(0, pose(0.0, 0.0, 0.0), Vec3::new(2000.0, 0.0, 0.0));
       assert!(s.forward);
       assert_eq!(s.turn, TurnCmd::Off);
       // 90 degrees toward +Y (yaw must rise): hold Right, stand while turning.
       let mut c = MoveController::new(MoveConfig::default(), TurnRate::default());
       let s = c.step(0, pose(0.0, 0.0, 0.0), Vec3::new(0.0, 2000.0, 0.0));
       assert!(!s.forward);
       assert_eq!(s.turn, TurnCmd::Hold(TurnKey::Right));
       // 90 degrees the other way: hold Left.
       let mut c = MoveController::new(MoveConfig::default(), TurnRate::default());
       let s = c.step(0, pose(0.0, 0.0, 0.0), Vec3::new(0.0, -2000.0, 0.0));
       assert_eq!(s.turn, TurnCmd::Hold(TurnKey::Left));
   }

   #[test]
   fn a_small_error_is_a_timed_pulse() {
       let mut c = MoveController::new(MoveConfig::default(), TurnRate::default());
       // 0.2 rad off: 0.2 / 1.92 * 1000 * 0.9 = 94 ms.
       let t = Vec3::new(2000.0 * 0.2f64.cos(), 2000.0 * 0.2f64.sin(), 0.0);
       let s = c.step(0, pose(0.0, 0.0, 0.0), t);
       assert!(s.forward);
       assert_eq!(s.turn, TurnCmd::Pulse(TurnKey::Right, 94));
       // Inside the deadband: nothing.
       let mut c = MoveController::new(MoveConfig::default(), TurnRate::default());
       let t = Vec3::new(2000.0 * 0.03f64.cos(), 2000.0 * 0.03f64.sin(), 0.0);
       assert_eq!(c.step(0, pose(0.0, 0.0, 0.0), t).turn, TurnCmd::Off);
   }

   #[test]
   fn turn_rate_learns_the_rate_and_flips_a_wrong_key() {
       let mut r = TurnRate::new(1.0);
       // Right held 100 ms, yaw rose 0.2 rad: 2 rad/s.
       assert!(r.observe(TurnKey::Right, 0.2, 100));
       assert!((r.rad_s - 2.0).abs() < 1e-9);
       // Too short or too small teaches nothing.
       assert!(!r.observe(TurnKey::Right, 0.2, 10));
       assert!(!r.observe(TurnKey::Right, 0.001, 100));
       // Two wrong-way samples flip the sign; one does not.
       assert!(!r.observe(TurnKey::Right, -0.2, 100));
       assert_eq!(r.right_sign, 1.0);
       assert!(!r.observe(TurnKey::Right, -0.2, 100));
       assert_eq!(r.right_sign, -1.0);
       assert_eq!(r.sign_flips, 1);
       assert_eq!(r.key_for(1.0), TurnKey::Left);
   }

   #[test]
   fn ten_held_ticks_without_a_turn_end_the_walk() {
       let mut c = MoveController::new(MoveConfig::default(), TurnRate::default());
       let target = Vec3::new(0.0, 2000.0, 0.0);
       let mut outcome = None;
       for k in 0..30u64 {
           let s = c.step(k * 100, pose(0.0, 0.0, 0.0), target);
           if s.outcome.is_some() {
               outcome = s.outcome;
               break;
           }
       }
       assert!(matches!(outcome, Some(MoveOutcome::NoTurn { ticks: 10 })), "{outcome:?}");
   }
   ```
   Update the `use super::*;` set if needed (`TurnCmd`, `TurnKey`, `TurnRate` come with it).

5. `io.rs`: in the `WorldIo` trait, after `ensure_focus`:
   ```rust
       /// The `(left, right)` lab key names the client binds to turn the pawn
       /// (default Q and E).
       async fn turn_keys(&mut self) -> Result<(String, String), String> {
           Ok((
               super::turn_keys::DEFAULT_LEFT.to_string(),
               super::turn_keys::DEFAULT_RIGHT.to_string(),
           ))
       }
   ```
   In `impl WorldIo for LiveWorld<'_>` after `ensure_focus`:
   ```rust
       async fn turn_keys(&mut self) -> Result<(String, String), String> {
           let r = self.lua(super::turn_keys::TURN_KEYS_CHUNK).await?;
           Ok(super::turn_keys::parse_turn_keys(&r))
       }
   ```

6. `movement.rs`:
   a. Header doc lines 3-4 become: `//! Forward is the `W` key held down; turning is the client's bound turn keys (TurnLeft / TurnRight, Q / E by default), because the pawn's heading ignores mouse-look. The loop reads the player's actor pose every tick ...` (keep the rest).
   b. Line 15 import: `use super::steer::{MoveConfig, MoveController, MoveOutcome, TurnCmd, TurnKey, TurnRate, Unstick};`
   c. After `FORWARD_KEY` add:
   ```rust
   /// The names of the turn keys this walk may press, for pressing and for
   /// the release on every exit.
   #[derive(Debug, Clone)]
   struct TurnKeys {
       left: String,
       right: String,
   }

   impl Default for TurnKeys {
       fn default() -> Self {
           Self {
               left: "Q".into(),
               right: "E".into(),
           }
       }
   }

   impl TurnKeys {
       fn name(&self, k: TurnKey) -> &str {
           match k {
               TurnKey::Left => &self.left,
               TurnKey::Right => &self.right,
           }
       }
   }

   /// Put the turn keys in the state `cmd` wants. `held` is the key currently
   /// down. Returns whether a turn key was used.
   async fn apply_turn<W: WorldIo>(
       io: &mut W,
       keys: &TurnKeys,
       held: &mut Option<TurnKey>,
       cmd: TurnCmd,
   ) -> Result<bool, String> {
       let want = match cmd {
           TurnCmd::Off => None,
           TurnCmd::Hold(k) | TurnCmd::Pulse(k, _) => Some(k),
       };
       if *held != want {
           if let Some(old) = held.take() {
               io.key(keys.name(old), false).await?;
           }
           if let Some(k) = want {
               io.key(keys.name(k), true).await?;
               *held = Some(k);
           }
       }
       if let TurnCmd::Pulse(_, ms) = cmd {
           io.sleep(ms).await;
           if let Some(k) = held.take() {
               io.key(keys.name(k), false).await?;
           }
       }
       Ok(want.is_some())
   }
   ```
   d. `release` (lines 106-112) and `run` (114-119):
   ```rust
   async fn release<W: WorldIo>(io: &mut W, turn: &TurnKeys) {
       for k in [FORWARD_KEY, "A", "D", turn.left.as_str(), turn.right.as_str()] {
           let _ = io.key(k, false).await;
       }
   }

   /// `client_move_to`.
   pub async fn run<W: WorldIo>(io: &mut W, req: MoveRequest) -> Result<Value, WorldError> {
       let mut turn_keys = TurnKeys::default();
       let r = walk(io, &req, &mut turn_keys).await;
       release(io, &turn_keys).await;
       r
   }
   ```
   e. `walk` signature gets `turn_keys: &mut TurnKeys`. After the `io.ensure_focus()` block (line 138) add:
   ```rust
       match io.turn_keys().await {
           Ok((left, right)) => {
               turn_keys.left = left;
               turn_keys.right = right;
           }
           Err(why) => tracing::debug!(%why, "turn key bindings unreadable; using Q and E"),
       }
   ```
   f. Line 188 `let mut turn = TurnModel::default();` -> `let mut rate = TurnRate::default(); let mut held_turn: Option<TurnKey> = None;`. Line 199-205: `MoveController::new(MoveConfig { arrival_m: arrival, ..MoveConfig::default() }, rate)`. Line 340 `turn = ctl.turn;` -> `rate = ctl.rate;`.
   g. Replace every `"turn_gain": ctl.turn.to_json()` / `turn.to_json()` (lines 221, 363, 405) with `"turn_rate": ctl.rate.to_json()` (line 221) and `"turn_rate": rate.to_json()` (363, 405). Remove `"standing_turn": ctl.standing_turn,` (line 347).
   h. Replace lines 322-334 (the `io.look` call, the forward edge and the `s.dx` used-level) with:
   ```rust
               let turned = apply_turn(io, turn_keys, &mut held_turn, s.turn)
                   .await
                   .map_err(|e| steps.fail(io.now_ms(), "turn_key", e))?;
               if s.forward != forward {
                   io.key(FORWARD_KEY, s.forward)
                       .await
                       .map_err(|e| steps.fail(io.now_ms(), "forward_key", e))?;
                   forward = s.forward;
               }
               if turned || s.forward {
                   steps.used(NativeLevel::RealInput);
               }
   ```
   (delete the old `if s.dx != 0 { steps.used(look_level); }` and `if s.forward { ... }` pair).
   i. The leg report `outcome` match (lines 348-352) gets a fourth arm:
   `MoveOutcome::NoTurn { ticks } => json!({ "arrived": false, "no_turn": true, "ticks": ticks }),`
   j. The failure match (lines 368-391) gets:
   ```rust
               MoveOutcome::NoTurn { ticks } => {
                   return Err(steps.fail_with(
                       io.now_ms(),
                       &format!("leg_{i}"),
                       format!(
                           "the pawn did not turn in {ticks} ticks of holding the turn keys ({} / {}): something roots it (a dialog, a cast, a stun) or the client binds other keys; compare client_player_state heading_deg before and after a client_input_key hold",
                           turn_keys.left, turn_keys.right
                       ),
                       evidence(),
                   ));
               }
   ```
   k. Remove the `NativeLevel` import only if unused (it is still used by `steps.used(NativeLevel::RealInput)`).

7. `sim.rs`:
   a. Struct fields (after `standing_turn`):
   ```rust
       /// The live pawn: its heading answers only to the turn keys (the
       /// camera's mouse-look never moves it), and `W` walks along it.
       pub live_pawn: bool,
       /// Radians per second while a turn key is held (live: 1.92).
       pub turn_rate_rad_s: f64,
       /// The right turn key raises the yaw (live: yes, E).
       pub right_raises_yaw: bool,
       /// `(left, right)` key names the client binds.
       pub turn_key_names: (String, String),
   ```
   b. In `Sim::new()` struct literal: `live_pawn: false, turn_rate_rad_s: 1.92, right_raises_yaw: true, turn_key_names: ("Q".into(), "E".into()),`.
   c. After `with_entity`:
   ```rust
       /// A sim whose pawn behaves like the live one (see `live_pawn`).
       pub fn live() -> Self {
           Self {
               live_pawn: true,
               standing_turn: false,
               ..Self::new()
           }
       }
   ```
   d. `advance`, inside the `while left > 0` loop after `self.now += dt;`:
   ```rust
               if self.live_pawn {
                   let (l, r) = self.turn_key_names.clone();
                   let dir = f64::from(u8::from(self.held(&r))) - f64::from(u8::from(self.held(&l)));
                   let sign = if self.right_raises_yaw { 1.0 } else { -1.0 };
                   self.pawn_yaw =
                       wrap_pi(self.pawn_yaw + sign * dir * self.turn_rate_rad_s * dt as f64 / 1000.0);
               }
   ```
   and change `self.pawn_yaw = self.cam_yaw;` inside `if self.held("W")` to `if !self.live_pawn { self.pawn_yaw = self.cam_yaw; }`.
   e. In `look` change `if self.standing_turn {` to `if self.standing_turn && !self.live_pawn {`.
   f. In `impl WorldIo for Sim` add:
   ```rust
       async fn turn_keys(&mut self) -> Result<(String, String), String> {
           Ok(self.turn_key_names.clone())
       }
   ```

8. `move_to_tests.rs`: every `Sim::new()` in this file becomes `Sim::live()` (and `room()` users: check `tool_tests::room()` builds with `Sim::new()`; add `room_live()` or set `sim.live_pawn = true; sim.standing_turn = false;` right after `room()` in the move_to tests that call it). Delete `move_to_learns_an_inverted_gain_and_walking_turns` (lines 26-46) and add:
   ```rust
   /// The 2026-10-12 live defect (#1660 item 8): the pawn ignores mouse-look,
   /// so a controller that steers with it never turns and walks away. The
   /// target is 90 degrees to the left; only the turn keys can get there.
   #[tokio::test]
   async fn move_to_turns_a_pawn_that_ignores_mouse_look() {
       let mut sim = Sim::live();
       let out = movement::run(
           &mut sim,
           MoveRequest {
               point: Some(client_point(0.0, 3000.0, 0.0)),
               ..MoveRequest::default()
           },
       )
       .await
       .unwrap();
       assert_eq!(out["arrived"], true);
       assert!(sim.looks.is_empty(), "steered with mouse-look: {:?}", sim.looks);
       assert!((sim.pos.y - 3000.0).abs() <= 150.0 + 60.0);
       assert!(out["turn_rate"]["samples"].as_u64().unwrap() >= 1);
       assert!(!sim.keys["E"] && !sim.keys["W"], "keys left held");
   }

   /// A target behind, a slower pawn and a flipped key: the controller learns
   /// the rate and the sign from the yaw it reads and still arrives.
   #[tokio::test]
   async fn move_to_learns_the_turn_rate_and_a_flipped_key() {
       let mut sim = Sim::live();
       sim.turn_rate_rad_s = 1.0;
       sim.right_raises_yaw = false;
       let out = movement::run(
           &mut sim,
           MoveRequest {
               point: Some(client_point(-2500.0, 800.0, 0.0)),
               timeout_ms: Some(60_000),
               ..MoveRequest::default()
           },
       )
       .await
       .unwrap();
       assert_eq!(out["arrived"], true);
       assert!(out["turn_rate"]["sign_flips"].as_u64().unwrap() >= 1);
       let rate = out["turn_rate"]["rad_s"].as_f64().unwrap();
       assert!((rate - 1.0).abs() < 0.35, "learned {rate} rad/s");
   }

   /// The keys are the client's bindings, not a hard-coded Q/E.
   #[tokio::test]
   async fn move_to_presses_the_bound_turn_keys() {
       let mut sim = Sim::live();
       sim.turn_key_names = ("J".into(), "L".into());
       let out = movement::run(
           &mut sim,
           MoveRequest {
               point: Some(client_point(0.0, 3000.0, 0.0)),
               ..MoveRequest::default()
           },
       )
       .await
       .unwrap();
       assert_eq!(out["arrived"], true);
       assert!(sim.keys.contains_key("L"), "the bound right key was never pressed");
       assert!(!sim.keys.contains_key("E") && !sim.keys.contains_key("Q"));
       assert!(!sim.keys["L"], "turn key left held");
   }

   /// A rooted pawn (turn rate 0) ends the walk honestly, with the keys up.
   #[tokio::test]
   async fn move_to_reports_a_pawn_that_does_not_turn() {
       let mut sim = Sim::live();
       sim.turn_rate_rad_s = 0.0;
       let e = movement::run(
           &mut sim,
           MoveRequest {
               point: Some(client_point(0.0, 3000.0, 0.0)),
               ..MoveRequest::default()
           },
       )
       .await
       .unwrap_err();
       assert_eq!(e.step, "leg_0");
       assert!(e.message.contains("did not turn"), "{}", e.message);
       assert!(!sim.keys["E"] && !sim.keys["W"]);
   }
   ```
   The waypoint test (`move_to_follows_waypoints_in_order`) needs `Sim::live()` because its first leg is 45 degrees off.

9. `world/camera.rs` and `world/click.rs` use `TurnModel` for the camera: no change. Check `grep -rn "standing_turn\|turn_gain" crates/lab` is empty afterwards (docs and the `client_move_to` tool description in `crates/lab/src/server/` included: update any description that says "mouse-look" for `client_move_to`).

**Test: why it fails when reverted**

- `move_to_turns_a_pawn_that_ignores_mouse_look`: if `walk` goes back to `io.look(s.dx, ...)` the live-model pawn never turns (look does not touch `pawn_yaw` when `live_pawn`), `W` walks along +X, the distance to the +Y goal grows past `min + 6 m`, `MoveOutcome::Diverged` fires, `.unwrap()` panics. The `sim.looks.is_empty()` assertion also catches a half-revert that adds mouse counts on top.
- `move_to_presses_the_bound_turn_keys`: a hard-coded Q/E (not reading `io.turn_keys()`) never presses `L`; the pawn does not turn; the assert on `sim.keys.contains_key("L")` and the arrival fail.
- `move_to_learns_the_turn_rate_and_a_flipped_key`: without `right_sign` learning, Right keeps lowering the yaw and the walk diverges or times out.
- steer tests assert `TurnCmd` values; a controller that returns mouse counts does not compile against them.

**Lane checks (from the worktree)**
```
...lane.ps1 cargo fmt -p cimmeria-lab
...lane.ps1 cargo clippy -p cimmeria-lab --all-targets -- -D warnings
...lane.ps1 cargo nextest run -p cimmeria-lab move_to
...lane.ps1 cargo nextest run -p cimmeria-lab steer
...lane.ps1 cargo nextest run -p cimmeria-lab turn_keys
...lane.ps1 cargo nextest run -p cimmeria-lab world
```
The last one runs the hover/camera/click tests against the untouched legacy `Sim::new()` pawn.

**Docs (`docs/guides/live-research-lab.md`, CRLF)**
- Line ~303 (`client_move_to` row): replace "with `W` and mouse-look" by "with `W` and the client's bound turn keys (TurnLeft / TurnRight, Q / E by default)".
- Line ~720 (Walking paragraph): replace the first two sentences by: "**Walking** holds `W` and turns with the client's bound turn keys (read from `getBindingKey('TurnLeft'|'TurnRight')`, default Q / E): the pawn's heading ignores mouse-look, which only moves the camera. Big errors hold a key, small ones pulse it for a timed fraction of a tick; the turn rate (about 110 deg/s) and the key sign are learned from the pawn's yaw (`turn_rate` in the result). Ten ticks of a held key with no yaw change fail the walk as `did not turn`." Keep the snag sentences.
- Line ~451 stays (it is about mouse-look for the camera).
- `.claude/agent-memory/main-session/reference_lab_client_controls.md`: add: "2026-10-12: the pawn heading answers only to the turn keys (about 110 deg/s, E raises the yaw, standing or walking); `client_camera` mouse-look moves the camera yaw offset only (600 counts = 65.9 deg), and `W` walks along the pawn heading. `getBindingKey('TurnLeft'|'TurnRight', 1|2)`; an unknown action returns an empty table." and edit the older bullet that says mouse-look does not move the camera.

**Live smoke (close-out lab UAT, ask the user first)**
From Castle_CellBlock stand 12 m from a vendor (the item-8 setup). `client_move_to {name, verbose:true}`: `arrived:true`, `native_level:"real_input"`, `turn_rate.samples >= 1` and `deg_s` between 80 and 140, `path` closes monotonically. Then a target behind the avatar (stand with your back to the vendor): arrives, no `diverged`. Before/after `client_player_state heading_deg` differ. Compare `server_entity_get` position.

**Commit subject:** `fix(lab): client_move_to turns the pawn with the bound turn keys, not mouse-look (LH-19)`

---

## Packet LH-20: `client_drag_drop` raises the source's window before the press

**ID / title / size:** LH-20, drag from a slot hidden under another window, S. Depends on: none (LH-01/02 merged; edits `ui/drag_drop.rs`).

**Design:** Opening the second window of a drag puts it on top of the first and, for the vault and the bag, in the same place (`VaultWin` [445,210,834,498], `InventoryWin` [445,210,834,510]). CEGUI delivers the press to the topmost window under the cursor, so the lower window's slot gets nothing and the drag "never starts". The tool already knows both window names. Before it presses it asks the client to `moveToFront()` the source slot's top-level window (the ancestor whose parent is `Root`). The target may then sit under the source's window; the drop already falls back to `notifyDragDropItemDropped` (N3) because `d_dropTarget` is null in the live client, and that fallback was proven live in this very layout (`vault 1 -> crafting 1` `moved:true`, swap). The result says which window was raised.

**Files**
1. `crates/lab/src/supervisor/ui/drag_drop.rs`.
2. `crates/lab/src/supervisor/cegui_fake.rs`.
3. `crates/lab/src/supervisor/ui/drag_drop_tests.rs`.
4. `docs/guides/live-research-lab.md` (CRLF).

**Steps**

1. `drag_drop.rs` imports: change `use super::window_click::{click_point, locate_chunk, ClickTarget};` to also bring `WALK_FN`:
   ```rust
   use super::window_read::WALK_FN;
   ```
   Add after `DRAG_STATE`:
   ```rust
   /// Lua body: raise the top-level window that holds `name` (the ancestor
   /// whose parent is the root) above its siblings, so a press at the slot's
   /// centre reaches the slot and not a window drawn over it. Live
   /// 2026-10-12: the bag opens exactly over the vault, so the press on
   /// `Vault_Slot1` landed on `Inventory_Slot1` and no vault drag started.
   /// Returns `{found, raised, ok}`.
   pub fn raise_chunk(name: &str) -> String {
       format!(
           "{WALK_FN}\nlocal w = __lab_find({n})\n\
            if w == nil then return __jenc({{ found = false }}) end\n\
            local top, cur = w, w\n\
            while cur ~= nil do\n\
              local p = __jcall(function() return cur:getParent() end)\n\
              if p == nil then break end\n\
              top = cur\n\
              cur = p\n\
            end\n\
            local ok = __jcall(function() top:moveToFront() return true end)\n\
            return __jenc({{ found = true, ok = ok == true,\n\
              raised = __jcall(function() return top:getName() end) }})",
           n = crate::supervisor::flows::widgets::lua_quote(name)
       )
   }
   ```
   (`lua_quote` is the same helper `window_click.rs` imports; import it at the top instead if you prefer: `use crate::supervisor::flows::widgets::lua_quote;`.) For `Vault_Slot1` the walk is Vault_Slot1 < Vault_AllSlotFrame < VaultWin < Root, so it raises `VaultWin` (probed).

2. In `drag_native`, after `let stand_in = match ... ;` (line ~341) and before `self.inject_cursor(&cegui, src)`:
   ```rust
        // A window drawn over the source would take the press: bring the
        // source's window to the front first. A miss is not fatal (the
        // source may already be on top); it is reported either way.
        let raised = self.lua_json(&raise_chunk(&src_name)).await?;
```
   and add `"raised_window": raised["raised"],` to the result `json!` (next to `"stand_in"`). Add no trail entry: the raise is a z-order change a player makes by clicking a window title, and a trail entry could downgrade a drag that is otherwise N1.

3. `cegui_fake.rs`:
   a. Field after `stand_in`:
   ```rust
       /// Another window is drawn over `SrcSlot`: a press there lands on it, so
       /// the source container never drags until `raise_chunk` ran.
       pub src_covered: bool,
       /// The press of the current gesture reached `SrcSlot`.
       press_on_src: bool,
       /// Windows `moveToFront` was called for, in order.
       pub raises: Vec<String>,
   ```
   `Default`: `src_covered: false, press_on_src: true, raises: Vec::new(),`.
   b. In `call_native`, the button-down arm: add `self.press_on_src = !self.src_covered;` after `self.button_down = self.down_handled;`. In the move arm replace `if self.button_down && self.container_drags {` by `if self.button_down && self.container_drags && self.press_on_src {`.
   c. In `lua`, make the raise the FIRST arm of the `let doc = if ... else if` chain (the WALK_FN text must not hit the later `contains` tests):
   ```rust
        let doc = if chunk.contains("moveToFront") {
            self.raises.push(window(chunk).map(|w| w.0).unwrap_or("?").to_string());
            self.src_covered = false;
            json!({ "found": true, "ok": true, "raised": "TopWin" })
        } else if chunk.contains("getCash") {
   ```
   (no other chunk in `crates/lab/src` contains `moveToFront`: grep first.) Since `window(chunk)` matches by quoted name, a raise for `SrcSlot` records `"SrcSlot"`.

4. `drag_drop_tests.rs`: all existing tests keep passing (`src_covered` defaults false; the extra Lua call is answered by the new arm). Add (near the other `drag(...)` tests):
   ```rust
   /// Live 2026-10-12 (#1660): the bag opened over the vault, the press at the
   /// vault slot landed on the window drawn over it, and no vault drag
   /// started. The tool raises the source's window before it presses.
   #[tokio::test]
   async fn a_source_under_another_window_is_raised_before_the_press() {
       let (out, s) = drag(
           FakeCegui {
               src_covered: true,
               ..FakeCegui::default()
           },
           4,
           true,
       )
       .await;
       let s = s.lock().unwrap();
       assert_eq!(s.raises, vec!["SrcSlot".to_string()]);
       assert_eq!(out["raised_window"], "TopWin");
       assert_eq!(out["drag_started"], true);
       assert_eq!(out["moved"], true);
       assert_eq!(out["effect_ok"], true);
   }

   /// Nothing is raised for an end that is only a target: the target stays
   /// where it is (the drop is the explicit notify).
   #[tokio::test]
   async fn only_the_source_window_is_raised() {
       let (_, s) = drag(FakeCegui::default(), 4, true).await;
       assert_eq!(s.lock().unwrap().raises, vec!["SrcSlot".to_string()]);
   }
   ```

**Test: why it fails when reverted:** with the raise call removed, `src_covered` stays true, the press lands on the covering window (`press_on_src` false) and the move never sets `dragging`: `drag_started` is false, no drop is notified, `moved` is false, `raises` is empty. Every assertion of the first test fails; the second fails on the empty `raises`. This is the exact live shape (press on the wrong window, `drag_started:false`).

**Lane checks**
```
...lane.ps1 cargo fmt -p cimmeria-lab
...lane.ps1 cargo clippy -p cimmeria-lab --all-targets -- -D warnings
...lane.ps1 cargo nextest run -p cimmeria-lab drag
```
(also `cegui` and `item_action`: other tests share the fake.)

**Docs (`docs/guides/live-research-lab.md`, CRLF):** in the `client_drag_drop` row/section add: "Before the press the tool raises the source slot's top-level window above the others (`raised_window` in the result): the bag opens exactly over the vault, and CEGUI gives the press to the topmost window. The drop target may stay underneath; the drop is the explicit `notifyDragDropItemDropped` (N3) as before." Add to the known-limits list: "`client_ui_click` / `client_item_action` on a slot hidden under another window hit the window on top; raise it first with a `client_lua_eval` `Window:moveToFront()` (follow-up, same cause)."

**Live smoke (ask the user first):** `.bank`, then `client_drag_drop {from:{container:"vault",slot:1}, to:{container:"crafting",slot:1}}` with the bag opened second: `drag_started:true`, `raised_window:"VaultWin"`, `moved:true`; swap back with the same call; compare `client_inventory` before and after (state is restored by the second drag).

**Commit subject:** `fix(lab): client_drag_drop raises the source window before the press (LH-20)`

---

## Not a packet: the two small supervisor notes

1. `lab_logout` 10053. Not caused by this branch (logout flow unchanged; intermittent; CS-08 noted timeouts before). One cheap, safe mitigation worth a line inside LH-19's PR or a follow-up: in `logout_flow`, move the login state off `InWorld` before typing `/logout` so the keeper stops ticking while the UI is torn down:
   - `supervisor/flows/login.rs`, `logout_flow`, right after `self.input_focus(true)...?;` add `self.set_login_state(LoginState::LoggingIn).await;` (or a dedicated `LoginState::LoggingOut` if `as_str` is extended), and restore with `self.set_login_state(LoginState::InWorld)` on the failure paths (`run.fail`) so a failed logout does not leave the keeper dark. Test: the existing keeper test `the_keeper_ticks_only_while_in_world` (`events/keeper.rs:191`) plus a new one that `logout_flow` against a fake bridge leaves `login != InWorld` for the duration of the wait. Do this only if a second 10053 shows up with the keeper armed; the evidence today does not prove the keeper is involved. If it recurs, capture `lab_client_status` and the lab MCP log line for the aborted call.
2. `ensure_in_world` "name did not read for 15 s". Pre-existing. The cheapest honest improvement: the error should say what it saw. At `ensure.rs:323` include the last observation: `format!("in the world but the player's name did not read for {MAX_SETTLES} s (world HUD up: {}, dialog: {})", ...)` and raise `MAX_SETTLES` to 30 when `lab_client_status` shows the client was started less than 120 s ago. Not specced: no repro, one data point of a slow load.
