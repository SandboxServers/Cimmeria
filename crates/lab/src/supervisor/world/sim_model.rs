//! The inherent `Sim` methods: pawn and camera motion, projection and
//! mouse-over. Split out of `sim.rs` so that file stays under 480 lines.

use std::collections::HashMap;
use std::f64::consts::FRAC_PI_2;

use super::super::geometry::{horizontal_m, wrap_pi, Screen, Vec3};
use super::{Sim, SimEntity, PLAYER_ID, UNITS};

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
            dead_ids: Vec::new(),
            unknown_alive_ids: Vec::new(),
            server_templates: HashMap::new(),
            server_unavailable: false,
            slots,
            windows: vec!["ChatWin".into(), "SelfStatusWin".into()],
            keys: HashMap::new(),
            cursor: (0, 0),
            mouse_over_on_repeated_move: None,
            repeated_move_override_active: false,
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
            slots_dispatch_timeout_after_click: false,
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
            server_pos: None,
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

    pub(super) fn held(&self, k: &str) -> bool {
        self.keys.get(k).copied().unwrap_or(false)
    }

    pub(super) fn slot(&self, s: i32) -> u32 {
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
    pub(super) fn under_cursor(&self) -> u32 {
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
            let off = self.entities.iter().find(|e| e.id == id).map(|e| {
                wrap_pi(super::super::geometry::bearing(self.pos, e.pos) - self.cam_yaw).abs()
            });
            let low = best.is_some_and(|b| b.2 < self.avatar_cover_dz);
            if hit == id && low && off.is_some_and(|o| o < half) {
                return PLAYER_ID;
            }
        }
        hit
    }

    pub(super) fn advance(&mut self, ms: u64) {
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
