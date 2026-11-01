//! A simulated client for the world-tool tests: a player pawn and camera,
//! a few entities, a pinhole projection, mouse-over by pixel distance,
//! and clicks that target or open a window. Time advances only in `sleep`.
//!
//! The mouse-look gain and sign, whether the pawn turns while standing,
//! and whether mouse-over needs a `WM_MOUSEMOVE` are knobs, so the tests
//! prove the tools cope with each without being told.

use std::collections::HashMap;

use serde_json::{json, Value};

use super::camera::WHEEL_NOTCH;
use super::coast::CoastModel;
use super::find::ServerEntity;
use super::geometry::{distance_m, wrap_pi, Pose, Screen, Vec3};
use super::io::{Projected, WorldIo};
use super::lua::{UnitInfo, UnitSlots};
use super::memory::{
    clamped_pitch_counts, WorldEntity, WorldSnapshot, CAMERA_DIST_MAX, CAMERA_DIST_MIN,
    CAMERA_ZOOM_STEP,
};
use super::sim_lag::Lag;
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
    /// Input latency and coasting (both 0 by default: no lag).
    pub lag: Lag,
    /// The walker's coast model as the last walk left it (`None`: the prior).
    pub coast_model: Option<CoastModel>,
    /// Every key change the walker asked for, in order (before any input lag).
    pub key_log: Vec<(String, bool)>,
    pub screen: Screen,
    pub hfov: f64,
    pub entities: Vec<SimEntity>,
    pub dead_ids: Vec<u32>,
    pub unknown_alive_ids: Vec<u32>,
    pub server_templates: HashMap<u32, i32>,
    pub server_unavailable: bool,
    pub slots: HashMap<i32, u32>,
    pub windows: Vec<String>,
    pub keys: HashMap<String, bool>,
    pub cursor: (i32, i32),
    /// Simulate mouse-over changing after a repeated final cursor move.
    pub mouse_over_on_repeated_move: Option<u32>,
    repeated_move_override_active: bool,
    pub looks: Vec<(i32, i32, i32)>,
    /// `input_status` delivery counter; `None` plays an old DLL.
    pub counters: Option<u64>,
    /// Whether a look raises `counters` (false: the game took nothing).
    pub counters_follow_looks: bool,
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
    /// Simulate the bridge becoming unavailable as soon as a click is sent.
    pub slots_dispatch_timeout_after_click: bool,
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
    /// The position the server reports for the player (see `server_position`).
    pub server_pos: Option<Vec3>,
    /// What every read after the first reports instead, so a walk can see the
    /// server at one place before it and another after.
    pub server_pos_end: Option<Vec3>,
    pub server_pos_reads: u32,
    /// The bridge frame counter; `None` models a client without frames.
    /// `sleep` advances it by `max(1, ms / 16)`.
    pub frames: Option<u64>,
}

#[path = "sim_model.rs"]
mod model;

impl WorldIo for Sim {
    fn now_ms(&self) -> u64 {
        self.now
    }

    async fn sleep(&mut self, ms: u64) {
        self.advance(ms);
        if let Some(f) = self.frames.as_mut() {
            *f += (ms / 16).max(1);
        }
    }

    async fn frame(&mut self) -> Option<u64> {
        self.frames
    }

    async fn server_position(&mut self, _entity_id: u32) -> Result<Option<Vec3>, String> {
        self.server_pos_reads += 1;
        if self.server_pos_reads > 1 {
            return Ok(self.server_pos_end.or(self.server_pos));
        }
        Ok(self.server_pos)
    }

    async fn server_entities(
        &mut self,
        _player_id: u32,
        template_id: Option<i32>,
    ) -> Result<Vec<ServerEntity>, String> {
        if self.server_unavailable {
            return Err("server lab MCP unavailable".into());
        }
        Ok(self
            .entities
            .iter()
            .filter_map(|e| {
                let id = self
                    .server_templates
                    .get(&e.id)
                    .copied()
                    .or_else(|| i32::try_from(e.id).ok()?.checked_add(1000))?;
                (template_id.is_none_or(|t| t == id)).then(|| ServerEntity {
                    id: e.id,
                    template_id: Some(id),
                    alive: (!self.unknown_alive_ids.contains(&e.id))
                        .then_some(!self.dead_ids.contains(&e.id)),
                })
            })
            .collect())
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
        if self.slots_dispatch_timeout_after_click
            && self.buttons.last().is_some_and(|(_, down)| !down)
        {
            return Err("dispatch timeout during world load".into());
        }
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
        let mo = if self.repeated_move_override_active {
            self.mouse_over_on_repeated_move
                .unwrap_or_else(|| self.under_cursor())
        } else {
            self.under_cursor()
        };
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
                        // Client mob IDs differ from the server's template IDs.
                        mob_id: Some(9000 + e.id as i64),
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
        if self.counters_follow_looks {
            if let Some(c) = self.counters.as_mut() {
                *c += 1;
            }
        }
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
            "yaw_offset_deg": self.cam_yaw.to_degrees(),
            "pitch_offset_deg": -self.cam_pitch.to_degrees(),
            "gain": gain,
            "pitch_gain": if self.pitch_inverted { -gain } else { gain },
            "yaw_gain": units / self.gain * self.readout_gain_scale,
        }))
    }

    fn coast_model(&self) -> CoastModel {
        self.coast_model.unwrap_or_default()
    }

    fn store_coast_model(&mut self, m: CoastModel) {
        self.coast_model = Some(m);
    }

    async fn key(&mut self, key: &str, down: bool) -> Result<(), String> {
        self.key_log.push((key.to_string(), down));
        if let Some((key, down)) = self.lag.key(self.now, key, down) {
            self.apply_key(&key, down);
        }
        Ok(())
    }

    async fn place_cursor(&mut self, x: i32, y: i32, mouse_move: bool) -> Result<(), String> {
        if !mouse_move && self.fail_cursor_restore {
            return Err("injected cursor restore failure".into());
        }
        if mouse_move && self.cursor == (x, y) && self.mouse_over_on_repeated_move.is_some() {
            self.repeated_move_override_active = true;
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

    async fn input_counters(&mut self) -> Option<u64> {
        self.counters
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
