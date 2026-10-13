//! A simulated client for the world-tool tests: a player pawn and camera,
//! a few entities, a pinhole projection, mouse-over by pixel distance,
//! and clicks that target or open a window. Time advances only in `sleep`.
//!
//! The mouse-look gain and sign, whether the pawn turns while standing,
//! and whether mouse-over needs a `WM_MOUSEMOVE` are knobs, so the tests
//! prove the tools cope with each without being told.

use std::collections::HashMap;
use std::f64::consts::FRAC_PI_2;

use serde_json::{json, Value};

use super::camera::WHEEL_NOTCH;
use super::geometry::{distance_m, horizontal_m, wrap_pi, Pose, Screen, Vec3};
use super::io::{Projected, WorldIo};
use super::lua::{UnitInfo, UnitSlots};
use super::memory::{
    clamped_pitch_counts, WorldEntity, WorldSnapshot, CAMERA_DIST_MAX, CAMERA_DIST_MIN,
    CAMERA_ZOOM_STEP,
};
use super::NativeLevel;

pub const PLAYER_ID: u32 = 1;
/// A standing humanoid's body heights (units above Location): knee, chest, shoulders and head, not the feet.
pub const REAL_BODY_DZ: [f64; 4] = [60.0, 100.0, 140.0, 180.0];
pub const PLAYER_ACTOR: u32 = 0x9000;
pub const UNITS: UnitSlots = UnitSlots {
    player: 0,
    target: 1,
    mouse_over: 2,
};

#[derive(Debug, Clone)]
pub struct SimEntity {
    pub id: u32,
    pub name: String,
    pub pos: Vec3,
    pub hostility: String,
    /// Right-click opens this window.
    pub window: Option<String>,
    pub rendered: bool,
}

impl SimEntity {
    pub fn actor(&self) -> u32 {
        if self.rendered {
            0x1000 + self.id
        } else {
            0
        }
    }
}

#[derive(Debug)]
pub struct Sim {
    pub now: u64,
    pub pos: Vec3,
    pub pawn_yaw: f64,
    pub cam_yaw: f64,
    pub cam_pitch: f64,
    /// Counts per radian of camera yaw (sign included).
    pub gain: f64,
    /// Camera distance, moved by the wheel (30 per notch, 100..775).
    pub zoom: f64,
    pub standing_turn: bool,
    /// The live pawn: its heading answers only to the turn keys (the
    /// camera's mouse-look never moves it), and `W` walks along it.
    pub live_pawn: bool,
    /// Radians per second while a turn key is held (live: 1.92).
    pub turn_rate_rad_s: f64,
    /// The right turn key raises the yaw (live: yes, E).
    pub right_raises_yaw: bool,
    /// `(left, right)` key names the client binds.
    pub turn_key_names: (String, String),
    pub hover_needs_mouse_move: bool,
    pub speed_uu_s: f64,
    /// Walls: forward motion stops at x >= wall until a jump clears it.
    pub wall_x: Option<f64>,
    pub jump_clears_wall: bool,
    pub screen: Screen,
    pub hfov: f64,
    pub entities: Vec<SimEntity>,
    pub slots: HashMap<i32, u32>,
    pub windows: Vec<String>,
    pub keys: HashMap<String, bool>,
    pub cursor: (i32, i32),
    pub looks: Vec<(i32, i32, i32)>,
    pub pins: u32,
    pub target_unit_calls: u32,
    pub buttons: Vec<(usize, bool)>,
    /// Clicks reach the game but do nothing (to exercise the fallback).
    pub clicks_do_nothing: bool,
    /// Scales the gains the camera readout reports (1.0 is exact), so a
    /// closed loop's first look misses and its correction has to run.
    pub readout_gain_scale: f64,
    /// What `window_rects` reports (empty: nothing over the 3D view).
    pub window_rects: Vec<(String, [f64; 4])>,
    /// Mouse-over is the player within this many pixels of the player's
    /// projected body. The camera sits at the head, so the avatar projects
    /// only when the camera pitches down (negative `cam_pitch`).
    pub avatar_radius_px: Option<f64>,
    /// `slots` (the mouse-over read) fails once it has answered this many
    /// times, to prove a failing tool still puts the cursor back.
    pub fail_mouse_over_after: Option<u32>,
    /// Return the bridge's own dispatch timeout after this many slot reads.
    pub slots_dispatch_timeout_after: Option<u32>,
    /// Make `slots` hang, as it does when the bridge stops answering.
    pub slots_never_returns: bool,
    /// Make `slots` hang after this many successful reads.
    pub slots_never_after: Option<u32>,
    pub slots_reads: u32,
    /// Model an unresponsive projection or camera bridge operation.
    pub project_never_after: Option<u32>,
    pub project_reads: u32,
    pub camera_readout_never_returns: bool,
    pub look_never_returns: bool,
    /// `place_cursor` without a mouse move (a cursor restore) fails.
    pub fail_cursor_restore: bool,
    /// Entities mouse-over never reports (rendered, but the client's trace
    /// does not hit them).
    pub unhoverable: Vec<u32>,
    /// Pitch turns behave like the live camera: clamped to +-78.75 deg
    /// (`memory::clamped_pitch_counts`), and a push into the limit is no call.
    pub live_pitch_clamp: bool,
    /// The pitch axis is inverted (flag 0x8): `pitch_gain` reads negative and
    /// +dy looks up.
    pub pitch_inverted: bool,
    /// While the camera looks within this many radians of the bearing to the
    /// entity, the avatar stands in front of it: mouse-over reports the
    /// player. (The usual third-person cover.)
    pub avatar_hides: Option<(u32, f64)>,
    /// The avatar only covers the target at pointer heights below this many
    /// units above the target's `Location` (the target's upper body shows
    /// over the avatar's head). Default infinity: covers every height.
    pub avatar_cover_dz: f64,
    /// Heights (units above an entity's `Location`) the sim treats as body
    /// when hit-testing the cursor. Default is the old permissive set that
    /// includes the feet; `REAL_BODY_DZ` is a standing humanoid, whose feet
    /// hover nothing (probe B-P2).
    pub body_dz: Vec<f64>,
    /// A right-click opens an entity's window only within this many metres, straight line, like the server.
    pub interact_range_m: Option<f64>,
}

impl Sim {
    pub fn new() -> Self {
        let mut slots = HashMap::new();
        slots.insert(UNITS.player, PLAYER_ID);
        Self {
            now: 0,
            pos: Vec3::new(0.0, 0.0, 0.0),
            pawn_yaw: 0.0,
            cam_yaw: 0.0,
            cam_pitch: 0.0,
            gain: 400.0,
            zoom: 250.0,
            standing_turn: true,
            live_pawn: false,
            turn_rate_rad_s: 1.92,
            right_raises_yaw: true,
            turn_key_names: ("Q".into(), "E".into()),
            hover_needs_mouse_move: false,
            speed_uu_s: 500.0,
            wall_x: None,
            jump_clears_wall: true,
            screen: Screen {
                w: 1000.0,
                h: 800.0,
            },
            hfov: FRAC_PI_2,
            entities: Vec::new(),
            slots,
            windows: vec!["ChatWin".into(), "SelfStatusWin".into()],
            keys: HashMap::new(),
            cursor: (0, 0),
            looks: Vec::new(),
            pins: 0,
            target_unit_calls: 0,
            buttons: Vec::new(),
            clicks_do_nothing: false,
            readout_gain_scale: 1.0,
            window_rects: Vec::new(),
            avatar_radius_px: None,
            fail_mouse_over_after: None,
            slots_dispatch_timeout_after: None,
            slots_never_returns: false,
            slots_never_after: None,
            slots_reads: 0,
            project_never_after: None,
            project_reads: 0,
            camera_readout_never_returns: false,
            look_never_returns: false,
            fail_cursor_restore: false,
            unhoverable: Vec::new(),
            live_pitch_clamp: false,
            pitch_inverted: false,
            avatar_hides: None,
            avatar_cover_dz: f64::INFINITY,
            body_dz: vec![-40.0, 0.0, 50.0, 90.0],
            interact_range_m: None,
        }
    }

    pub fn with_entity(mut self, id: u32, name: &str, pos: Vec3) -> Self {
        self.entities.push(SimEntity {
            id,
            name: name.into(),
            pos,
            hostility: "Friendly".into(),
            window: None,
            rendered: true,
        });
        self
    }

    /// A sim whose pawn behaves like the live one (see `live_pawn`).
    pub fn live() -> Self {
        Self {
            live_pawn: true,
            standing_turn: false,
            ..Self::new()
        }
    }

    fn held(&self, k: &str) -> bool {
        self.keys.get(k).copied().unwrap_or(false)
    }

    fn slot(&self, s: i32) -> u32 {
        self.slots.get(&s).copied().unwrap_or(0)
    }

    /// Camera-space projection of a client point (camera at the pawn's
    /// head, looking along the camera yaw and pitch).
    pub fn project_point(&self, p: Vec3) -> Option<(f64, f64)> {
        let eye = Vec3::new(self.pos.x, self.pos.y, self.pos.z + 150.0);
        let d = p.sub(eye);
        let (cy, sy) = (self.cam_yaw.cos(), self.cam_yaw.sin());
        let fwd_h = d.x * cy + d.y * sy;
        let right = -d.x * sy + d.y * cy;
        let (cp, sp) = (self.cam_pitch.cos(), self.cam_pitch.sin());
        let fwd = fwd_h * cp + d.z * sp;
        let up = -fwd_h * sp + d.z * cp;
        if fwd <= 1.0 {
            return None;
        }
        let (cx, cyy) = self.screen.centre();
        let focal = cx / (self.hfov / 2.0).tan();
        Some((cx + right / fwd * focal, cyy - up / fwd * focal))
    }

    /// The nearest rendered entity within 25 px of the cursor; the player
    /// first (nearest of all) when `avatar_radius_px` is set and covers it.
    fn under_cursor(&self) -> u32 {
        let (x, y) = (self.cursor.0 as f64, self.cursor.1 as f64);
        if let Some(r) = self.avatar_radius_px {
            let covered = [-40.0, 0.0, 50.0, 90.0].iter().any(|dz| {
                self.project_point(Vec3::new(self.pos.x, self.pos.y, self.pos.z + dz))
                    .is_some_and(|(px, py)| ((px - x).powi(2) + (py - y).powi(2)).sqrt() <= r)
            });
            if covered {
                return PLAYER_ID;
            }
        }
        let mut best: Option<(f64, u32, f64)> = None;
        for e in self
            .entities
            .iter()
            .filter(|e| e.rendered && !self.unhoverable.contains(&e.id))
        {
            // The body height whose projection is nearest the cursor, if any
            // is within 25 px.
            let mut near: Option<(f64, f64)> = None;
            for dz in self.body_dz.iter().copied() {
                let Some((px, py)) = self.project_point(Vec3::new(e.pos.x, e.pos.y, e.pos.z + dz))
                else {
                    continue;
                };
                let d_px = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
                if d_px <= 25.0 && near.is_none_or(|(d, _)| d_px < d) {
                    near = Some((d_px, dz));
                }
            }
            if let Some((_, dz)) = near {
                let dist = horizontal_m(self.pos, e.pos);
                if best.is_none_or(|(d, _, _)| dist < d) {
                    best = Some((dist, e.id, dz));
                }
            }
        }
        let hit = best.map(|b| b.1).unwrap_or(0);
        if let Some((id, half)) = self.avatar_hides {
            let off =
                self.entities.iter().find(|e| e.id == id).map(|e| {
                    wrap_pi(super::geometry::bearing(self.pos, e.pos) - self.cam_yaw).abs()
                });
            let low = best.is_some_and(|b| b.2 < self.avatar_cover_dz);
            if hit == id && low && off.is_some_and(|o| o < half) {
                return PLAYER_ID;
            }
        }
        hit
    }

    fn advance(&mut self, ms: u64) {
        let mut left = ms;
        while left > 0 {
            let dt = left.min(10);
            left -= dt;
            self.now += dt;
            if self.live_pawn {
                let (l, r) = self.turn_key_names.clone();
                let dir = f64::from(u8::from(self.held(&r))) - f64::from(u8::from(self.held(&l)));
                let sign = if self.right_raises_yaw { 1.0 } else { -1.0 };
                self.pawn_yaw =
                    wrap_pi(self.pawn_yaw + sign * dir * self.turn_rate_rad_s * dt as f64 / 1000.0);
            }
            if self.held("S") {
                // Backward is never blocked by `wall_x` (the wall is ahead).
                let step = self.speed_uu_s * dt as f64 / 1000.0;
                self.pos.x -= self.pawn_yaw.cos() * step;
                self.pos.y -= self.pawn_yaw.sin() * step;
            }
            if self.held("W") {
                if !self.live_pawn {
                    self.pawn_yaw = self.cam_yaw;
                }
                let step = self.speed_uu_s * dt as f64 / 1000.0;
                let nx = self.pos.x + self.pawn_yaw.cos() * step;
                let ny = self.pos.y + self.pawn_yaw.sin() * step;
                if self.wall_x.is_some_and(|w| nx >= w) {
                    continue;
                }
                self.pos.x = nx;
                self.pos.y = ny;
            }
        }
    }
}

impl WorldIo for Sim {
    fn now_ms(&self) -> u64 {
        self.now
    }

    async fn sleep(&mut self, ms: u64) {
        self.advance(ms);
    }

    async fn snapshot(&mut self) -> Result<WorldSnapshot, String> {
        let mut entities = vec![WorldEntity {
            id: PLAYER_ID,
            ptr: 0x8000,
            actor: PLAYER_ACTOR,
            rendered_flag: true,
            pose: Some(Pose {
                pos: self.pos,
                yaw: self.pawn_yaw,
                pitch: 0.0,
            }),
            pose_error: None,
        }];
        for e in &self.entities {
            entities.push(WorldEntity {
                id: e.id,
                ptr: 0x2000 + e.id,
                actor: e.actor(),
                rendered_flag: e.rendered,
                pose: e.rendered.then_some(Pose {
                    pos: e.pos,
                    yaw: 0.0,
                    pitch: 0.0,
                }),
                pose_error: None,
            });
        }
        Ok(WorldSnapshot {
            manager: 0x7000,
            player_id: PLAYER_ID,
            entities,
            truncated: false,
        })
    }

    async fn actor_pose(&mut self, actor: u32) -> Result<Pose, String> {
        if actor == PLAYER_ACTOR {
            return Ok(Pose {
                pos: self.pos,
                yaw: self.pawn_yaw,
                pitch: 0.0,
            });
        }
        self.entities
            .iter()
            .find(|e| e.rendered && e.actor() == actor)
            .map(|e| Pose {
                pos: e.pos,
                yaw: 0.0,
                pitch: 0.0,
            })
            .ok_or_else(|| format!("no actor {actor:#x}"))
    }

    async fn slots(&mut self) -> Result<Vec<(i32, u32)>, String> {
        if self
            .slots_dispatch_timeout_after
            .is_some_and(|after| self.slots_reads >= after)
        {
            return Err("dispatch timeout".into());
        }
        if self.slots_never_returns
            || self
                .slots_never_after
                .is_some_and(|after| self.slots_reads >= after)
        {
            std::future::pending().await
        }
        if self
            .fail_mouse_over_after
            .is_some_and(|n| self.slots_reads >= n)
        {
            return Err("injected mouse-over read failure".into());
        }
        self.slots_reads += 1;
        let mo = self.under_cursor();
        let hover_ok = !self.hover_needs_mouse_move || self.held("__mousemove");
        self.slots
            .insert(UNITS.mouse_over, if hover_ok { mo } else { 0 });
        Ok(self.slots.iter().map(|(k, v)| (*k, *v)).collect())
    }

    async fn unit_slots(&mut self) -> Result<UnitSlots, String> {
        Ok(UNITS)
    }

    async fn pin(&mut self, slot: i32, entity: u32) -> Result<(), String> {
        self.pins += 1;
        self.slots.insert(slot, entity);
        Ok(())
    }

    async fn unit_info(&mut self, slots: &[i32]) -> Result<Vec<UnitInfo>, String> {
        Ok(slots
            .iter()
            .map(|s| {
                let id = self.slot(*s);
                match self.entities.iter().find(|e| e.id == id) {
                    Some(e) => UnitInfo {
                        slot: *s,
                        exists: true,
                        name: e.name.clone(),
                        level: Some(5),
                        hostility: e.hostility.clone(),
                        is_friend: Some(e.hostility == "Friendly"),
                        mob_id: Some(1000 + e.id as i64),
                    },
                    None => UnitInfo {
                        slot: *s,
                        ..UnitInfo::default()
                    },
                }
            })
            .collect())
    }

    async fn project(&mut self, points: &[Vec3]) -> Result<Projected, String> {
        if self
            .project_never_after
            .is_some_and(|after| self.project_reads >= after)
        {
            std::future::pending().await
        }
        self.project_reads += 1;
        self.advance(20);
        Ok(Projected {
            screen: self.screen,
            points: points.iter().map(|p| self.project_point(*p)).collect(),
            host: "SCTWin".into(),
        })
    }

    async fn camera_pose(&mut self) -> Result<Pose, String> {
        Ok(Pose {
            pos: self.pos,
            yaw: self.cam_yaw,
            pitch: self.cam_pitch,
        })
    }

    async fn look(&mut self, dx: i32, dy: i32, wheel: i32) -> Result<NativeLevel, String> {
        if self.look_never_returns {
            std::future::pending().await
        }
        self.looks.push((dx, dy, wheel));
        self.cam_yaw = wrap_pi(self.cam_yaw + dx as f64 / self.gain);
        let sign = if self.pitch_inverted { -1.0 } else { 1.0 };
        let mut counts = f64::from(dy);
        if self.live_pitch_clamp && dy != 0 {
            // What LiveWorld::native_look does before it calls the turn handler.
            let offset_units = (-self.cam_pitch).to_degrees() * 65536.0 / 360.0;
            let gain = sign * 65536.0 / std::f64::consts::TAU / self.gain.abs();
            counts = f64::from(clamped_pitch_counts(
                offset_units as i32,
                gain as f32,
                dy as f32,
            ));
        }
        // Mouse forward (negative dy) looks up (inverted: down).
        self.cam_pitch = (self.cam_pitch - sign * counts / self.gain.abs()).clamp(-1.4, 1.4);
        if self.standing_turn && !self.live_pawn {
            self.pawn_yaw = self.cam_yaw;
        }
        let notches = f64::from(wheel) / f64::from(WHEEL_NOTCH);
        self.zoom = (self.zoom - notches * f64::from(CAMERA_ZOOM_STEP))
            .clamp(f64::from(CAMERA_DIST_MIN), f64::from(CAMERA_DIST_MAX));
        Ok(NativeLevel::RealInput)
    }

    async fn camera_readout(&mut self) -> Result<Value, String> {
        if self.camera_readout_never_returns {
            std::future::pending().await
        }
        // Rotator units per mouse count: 65536 per turn over the sim's gain.
        // The readout's pitch rises with +dy, as the live camera's does.
        let units = 65536.0 / std::f64::consts::TAU;
        let gain = units / self.gain.abs() * self.readout_gain_scale;
        Ok(json!({
            "zoom": self.zoom,
            "yaw_offset_deg": 0.0,
            "pitch_offset_deg": -self.cam_pitch.to_degrees(),
            "gain": gain,
            "pitch_gain": if self.pitch_inverted { -gain } else { gain },
            "yaw_gain": units / self.gain * self.readout_gain_scale,
        }))
    }

    async fn key(&mut self, key: &str, down: bool) -> Result<(), String> {
        if key == "Space" && down && self.jump_clears_wall {
            self.wall_x = None;
        }
        self.keys.insert(key.to_string(), down);
        Ok(())
    }

    async fn place_cursor(&mut self, x: i32, y: i32, mouse_move: bool) -> Result<(), String> {
        if !mouse_move && self.fail_cursor_restore {
            return Err("injected cursor restore failure".into());
        }
        self.cursor = (x, y);
        self.keys.insert("__mousemove".into(), mouse_move);
        Ok(())
    }

    async fn button(&mut self, button: usize, down: bool) -> Result<(), String> {
        self.buttons.push((button, down));
        if !down && !self.clicks_do_nothing {
            let hover_ok = !self.hover_needs_mouse_move || self.held("__mousemove");
            let under = if hover_ok { self.under_cursor() } else { 0 };
            if under != 0 {
                self.slots.insert(UNITS.target, under);
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
            }
        }
        Ok(())
    }

    async fn visible_windows(&mut self) -> Result<Vec<String>, String> {
        Ok(self.windows.clone())
    }

    async fn window_rects(&mut self) -> Result<Vec<(String, [f64; 4])>, String> {
        Ok(self.window_rects.clone())
    }

    async fn cursor(&mut self) -> Result<(i32, i32), String> {
        Ok(self.cursor)
    }

    async fn target_unit(&mut self, slot: i32) -> Result<(), String> {
        self.target_unit_calls += 1;
        let id = self.slot(slot);
        self.slots.insert(UNITS.target, id);
        Ok(())
    }

    async fn ensure_focus(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn turn_keys(&mut self) -> Result<(String, String), String> {
        Ok(self.turn_key_names.clone())
    }
}
