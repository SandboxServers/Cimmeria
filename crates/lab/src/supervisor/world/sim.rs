//! A simulated client for the world-tool tests: a player pawn and camera,
//! a few entities, a pinhole projection, mouse-over by pixel distance,
//! and clicks that target or open a window. Time advances only in `sleep`.
//!
//! The mouse-look gain and sign, whether the pawn turns while standing,
//! and whether mouse-over needs a `WM_MOUSEMOVE` are knobs, so the tests
//! prove the tools cope with each without being told.

use std::collections::HashMap;
use std::f64::consts::FRAC_PI_2;

use super::geometry::{horizontal_m, wrap_pi, Pose, Screen, Vec3};
use super::io::{Projected, WorldIo};
use super::lua::{UnitInfo, UnitSlots};
use super::memory::{WorldEntity, WorldSnapshot};

pub const PLAYER_ID: u32 = 1;
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
    pub standing_turn: bool,
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
            standing_turn: true,
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

    /// The nearest rendered entity within 25 px of the cursor.
    fn under_cursor(&self) -> u32 {
        let (x, y) = (self.cursor.0 as f64, self.cursor.1 as f64);
        let mut best: Option<(f64, u32)> = None;
        for e in self.entities.iter().filter(|e| e.rendered) {
            for dz in [-40.0, 0.0, 50.0, 90.0] {
                let Some((px, py)) = self.project_point(Vec3::new(e.pos.x, e.pos.y, e.pos.z + dz))
                else {
                    continue;
                };
                if ((px - x).powi(2) + (py - y).powi(2)).sqrt() <= 25.0 {
                    let dist = horizontal_m(self.pos, e.pos);
                    if best.is_none_or(|(d, _)| dist < d) {
                        best = Some((dist, e.id));
                    }
                }
            }
        }
        best.map(|b| b.1).unwrap_or(0)
    }

    fn advance(&mut self, ms: u64) {
        let mut left = ms;
        while left > 0 {
            let dt = left.min(10);
            left -= dt;
            self.now += dt;
            if self.held("W") {
                self.pawn_yaw = self.cam_yaw;
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

    async fn look(&mut self, dx: i32, dy: i32, wheel: i32) -> Result<(), String> {
        self.looks.push((dx, dy, wheel));
        self.cam_yaw = wrap_pi(self.cam_yaw + dx as f64 / self.gain);
        // Mouse forward (negative dy) looks up.
        self.cam_pitch = (self.cam_pitch - dy as f64 / self.gain.abs()).clamp(-1.4, 1.4);
        if self.standing_turn {
            self.pawn_yaw = self.cam_yaw;
        }
        Ok(())
    }

    async fn key(&mut self, key: &str, down: bool) -> Result<(), String> {
        if key == "Space" && down && self.jump_clears_wall {
            self.wall_x = None;
        }
        self.keys.insert(key.to_string(), down);
        Ok(())
    }

    async fn place_cursor(&mut self, x: i32, y: i32, mouse_move: bool) -> Result<(), String> {
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
            }
        }
        Ok(())
    }

    async fn visible_windows(&mut self) -> Result<Vec<String>, String> {
        Ok(self.windows.clone())
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
}
