//! `lab_recover` recipes (contract C11). Each recipe does a known fix, then
//! reads back whether it worked and says so as C1 evidence: a recipe that
//! could not confirm its effect is inconclusive, never a silent success.
//! (`recovery.rs` is crash bookkeeping; this is not that.)

use std::time::Duration;

use serde_json::{json, Value};

use super::world::geometry::{horizontal_m, Vec3};
use super::world::io::{LiveWorld, WorldIo};
use super::world::steer::Unstick;
use super::world::unstick::run_unstick;
use super::world::{camera, camera::CameraRequest};
use super::Supervisor;
use crate::proof::{Check, Evidence, Source, Why};

/// The recipe names, in the order the error message lists them.
pub const RECIPES: &[&str] = &[
    "release_input",
    "close_dialogs",
    "recenter_camera",
    "unstick",
];

/// Escape taps `close_dialogs` makes before giving up.
const ESCAPE_TRIES: usize = 3;
/// Let the UI react to an Escape before reading it back.
const ESCAPE_SETTLE_MS: u64 = 300;
/// How far the camera pitch may sit from level and still count (degrees).
const PITCH_TOLERANCE_DEG: f64 = 5.0;
/// How far the pawn must move for an unstick to count (metres).
pub const MOVED_MIN_M: f64 = 0.5;

/// `input_status` shows nothing held: no keys, no buttons, no holds.
/// `None` (the call failed) or a status without a `held` object is not
/// measured.
pub fn input_released_check(status: Option<&Value>) -> Check {
    let name = "input_released";
    let Some(status) = status else {
        return Check::new(name, Source::Bridge, None, true).with_why(Why::NoPostRead);
    };
    let Some(held) = status.get("held").filter(|h| h.is_object()) else {
        return Check::new(name, Source::Bridge, None, true)
            .with_after(status.clone())
            .with_why(Why::NoPostRead);
    };
    // A missing list means nothing in it (an old DLL may omit `holds`).
    let empty = |v: Option<&Value>| match v {
        Some(Value::Array(a)) => a.is_empty(),
        _ => true,
    };
    let clear = empty(held.get("keys")) && empty(held.get("buttons")) && empty(status.get("holds"));
    Check::new(name, Source::Bridge, Some(clear), true)
        .with_after(json!({ "held": held, "holds": status.get("holds") }))
        .with_why(Why::NoEffect)
}

/// The UI state's `dialog` is null or absent. `None` is a failed read.
pub fn dialog_closed_check(ui: Option<&Value>) -> Check {
    let name = "dialog_closed";
    let Some(ui) = ui else {
        return Check::new(name, Source::Client, None, true).with_why(Why::NoPostRead);
    };
    // `dialog: null` also comes back when the Lua dialog section threw; that
    // is an unread dialog, not a closed one.
    if ui.get("errors").and_then(Value::as_array).is_some_and(|e| {
        e.iter()
            .any(|s| s.as_str().is_some_and(|s| s.starts_with("dialog:")))
    }) {
        return Check::new(name, Source::Client, None, true)
            .with_after(ui.get("errors").cloned().unwrap_or(Value::Null))
            .with_why(Why::NoPostRead);
    }
    let closed = ui.get("dialog").is_none_or(Value::is_null);
    Check::new(name, Source::Client, Some(closed), true)
        .with_after(ui.get("dialog").cloned().unwrap_or(Value::Null))
        .with_why(Why::NoEffect)
}

/// The camera readout's pitch offset is within tolerance of level.
pub fn pitch_level_check(pitch_offset_deg: Option<f64>) -> Check {
    let name = "pitch_level";
    match pitch_offset_deg {
        None => Check::new(name, Source::Client, None, true).with_why(Why::NoPostRead),
        Some(p) => Check::new(
            name,
            Source::Client,
            Some(p.abs() <= PITCH_TOLERANCE_DEG),
            true,
        )
        .with_after(json!(p))
        .with_why(Why::NoEffect),
    }
}

/// Horizontal metres moved; a jump in place does not count.
pub fn moved_m(start: Vec3, now: Vec3) -> f64 {
    horizontal_m(start, now)
}

/// The pawn moved at least [`MOVED_MIN_M`] by the client pose.
pub fn moved_check(moved_m: Option<f64>) -> Check {
    let name = "moved";
    match moved_m {
        None => Check::new(name, Source::Client, None, true).with_why(Why::NoPostRead),
        Some(m) => Check::new(name, Source::Client, Some(m >= MOVED_MIN_M), true)
            .with_after(json!(m))
            .with_why(Why::NoEffect),
    }
}

/// The optional server cross-check of an unstick: only when the server
/// answered both before and after.
pub fn server_moved_check(before: Option<Vec3>, after: Option<Vec3>) -> Option<Check> {
    let (b, a) = (before?, after?);
    // Server Y is up, so the horizontal plane is x/z.
    let (dx, dz) = (a.x - b.x, a.z - b.z);
    let m = (dx * dx + dz * dz).sqrt();
    Some(
        Check::new(
            "server_moved",
            Source::Server,
            Some(m >= MOVED_MIN_M),
            false,
        )
        .with_after(json!(m)),
    )
}

fn step(name: &str, ok: bool, detail: Option<Value>) -> Value {
    match detail {
        Some(d) => json!({ "step": name, "ok": ok, "detail": d }),
        None => json!({ "step": name, "ok": ok }),
    }
}

impl Supervisor {
    /// Run one recovery recipe on the live client.
    pub(crate) async fn recover(&self, recipe: &str) -> Result<Value, String> {
        self.recover_with(recipe, None).await
    }

    /// [`Supervisor::recover`] with the game window handle given (the test
    /// seam: the fake bridge has no window). `None` resolves it on demand.
    pub(crate) async fn recover_with(
        &self,
        recipe: &str,
        hwnd: Option<isize>,
    ) -> Result<Value, String> {
        let mut steps = Vec::new();
        let checks = match recipe {
            "release_input" => self.recover_release_input(&mut steps).await,
            "close_dialogs" => self.recover_close_dialogs(&mut steps, hwnd).await,
            "recenter_camera" => self.recover_recenter_camera(&mut steps).await,
            "unstick" => self.recover_unstick(&mut steps).await,
            other => {
                return Err(format!(
                    "unknown recipe {other}; one of {}",
                    RECIPES.join(", ")
                ))
            }
        };
        let evidence = Evidence::judge(checks);
        tracing::info!(
            target: "lab.input",
            recipe,
            verdict = ?evidence.verdict,
            why = ?evidence.why,
            "lab.recover"
        );
        Ok(json!({
            "recipe": recipe,
            "evidence": evidence.to_value(),
            "steps": steps,
        }))
    }

    async fn recover_release_input(&self, steps: &mut Vec<Value>) -> Vec<Check> {
        match self.release_all_input("recover").await {
            Ok(_) => steps.push(step("input_release", true, None)),
            Err(e) => steps.push(step("input_release", false, Some(json!(e)))),
        }
        let status = self.bridge_call("input_status", json!({})).await;
        match &status {
            Ok(_) => steps.push(step("input_status", true, None)),
            Err(e) => steps.push(step("input_status", false, Some(json!(e)))),
        }
        vec![input_released_check(status.as_ref().ok())]
    }

    async fn recover_close_dialogs(
        &self,
        steps: &mut Vec<Value>,
        hwnd: Option<isize>,
    ) -> Vec<Check> {
        let hwnd = match hwnd {
            Some(h) => Ok(h),
            None => self.game_hwnd().await,
        };
        let hwnd = match hwnd {
            Ok(h) => h,
            Err(e) => {
                steps.push(step("escape", false, Some(json!(e))));
                return vec![dialog_closed_check(None)];
            }
        };
        let mut last = None;
        for attempt in 1..=ESCAPE_TRIES {
            if let Err(e) = self.input_key_at(hwnd, "Escape", "tap", None).await {
                steps.push(step(
                    "escape",
                    false,
                    Some(json!({ "attempt": attempt, "error": e })),
                ));
                last = None;
                break;
            }
            tokio::time::sleep(Duration::from_millis(ESCAPE_SETTLE_MS)).await;
            match self.ui_state(Some(0)).await {
                Ok(ui) => {
                    let closed = dialog_closed_check(Some(&ui)).ok == Some(true);
                    steps.push(step(
                        "escape",
                        true,
                        Some(json!({ "attempt": attempt, "dialog_closed": closed })),
                    ));
                    last = Some(ui);
                    if closed {
                        break;
                    }
                }
                Err(e) => {
                    steps.push(step(
                        "ui_state",
                        false,
                        Some(json!({ "attempt": attempt, "error": e })),
                    ));
                    last = None;
                    break;
                }
            }
        }
        vec![dialog_closed_check(last.as_ref())]
    }

    async fn recover_recenter_camera(&self, steps: &mut Vec<Value>) -> Vec<Check> {
        let mut io = LiveWorld::new(self);
        let req = CameraRequest {
            pitch_deg: Some(0.0),
            ..Default::default()
        };
        let pitch = match camera::run(&mut io, req).await {
            Ok(v) => {
                steps.push(step("camera", true, None));
                v["view_after"]["pitch_offset_deg"].as_f64()
            }
            Err(e) => {
                steps.push(step("camera", false, Some(json!(e.summary()))));
                // The pitch may still be readable: judge on it, not on the error.
                io.camera_readout()
                    .await
                    .ok()
                    .and_then(|r| r["pitch_offset_deg"].as_f64())
            }
        };
        vec![pitch_level_check(pitch)]
    }

    async fn recover_unstick(&self, steps: &mut Vec<Value>) -> Vec<Check> {
        let mut io = LiveWorld::new(self);
        let start = match io.snapshot().await {
            Ok(s) => s
                .player()
                .and_then(|p| p.pose.map(|pose| (s.player_id, p.actor, pose))),
            Err(e) => {
                steps.push(step("pose", false, Some(json!(e))));
                None
            }
        };
        let Some((player_id, actor, start_pose)) = start else {
            return vec![moved_check(None)];
        };
        let server_before = io.server_position(player_id).await.ok().flatten();
        let mut moved = None;
        for u in [Unstick::Jump, Unstick::StrafeRight, Unstick::StrafeLeft] {
            if let Err(e) = run_unstick(&mut io, u).await {
                // Never leave the key down on a failed press.
                let _ = io.key(u.key(), false).await;
                steps.push(step(u.name(), false, Some(json!(e))));
                moved = None;
                break;
            }
            match io.actor_pose(actor).await {
                Ok(p) => {
                    let m = moved_m(start_pose.pos, p.pos);
                    steps.push(step(u.name(), true, Some(json!({ "moved_m": m }))));
                    moved = Some(m);
                    if m >= MOVED_MIN_M {
                        break;
                    }
                }
                Err(e) => {
                    steps.push(step(u.name(), false, Some(json!(e))));
                    moved = None;
                    break;
                }
            }
        }
        let mut checks = vec![moved_check(moved)];
        let server_after = io.server_position(player_id).await.ok().flatten();
        checks.extend(server_moved_check(server_before, server_after));
        checks
    }
}

#[cfg(test)]
#[path = "recover_recipes_tests.rs"]
mod tests;
