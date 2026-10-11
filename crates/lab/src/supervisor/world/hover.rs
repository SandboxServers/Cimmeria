//! `client_hover_probe`: hover an entity or a world point with the cursor,
//! never clicking, and score the pixel for a spec pose (README F12, F13,
//! D-SV6).
//!
//! 1. Resolve the aim like `client_world_click` does, but never turn the
//!    camera: an aim off screen is `verdict: "off_screen"`.
//! 2. Centre pixel: for an entity, the first body height whose hover is the
//!    entity (else the first height on screen); for a point, its projection.
//! 3. At the centre and at 8 points on a ring around it, place the cursor
//!    and read the client's `Unit.MouseOver` and the HUD window rects. An
//!    entity aim wants the entity under the cursor; a point aim wants nothing
//!    or `expect_entity` (a torso point may not hover its body, F13). Either
//!    way no HUD window may cover the pixel.
//! 4. Report how far the centre is from the avatar and the screen edge.
//!
//! The cursor goes back where it was, also on failure. No path presses a
//! mouse button.

use rmcp::schemars;
use serde_json::{json, Value};

use super::camera::Aim;
use super::click::{hover_once, Hover, HOVER_OFFSETS, HOVER_WAIT_MS};
use super::find::{find_entities, FindRequest, Found, CLICK_MARGIN_PX};
use super::geometry::Vec3;
use super::io::WorldIo;
use super::lua::UnitSlots;
use super::memory::slot_entity;
use super::{NativeLevel, PointArg, Steps, TargetArg, WorldError};

const TOOL: &str = "client_hover_probe";
/// Jitter ring radius when the request gives none, px.
pub const DEFAULT_JITTER_PX: u32 = 12;
/// Points on the jitter ring, every 45 degrees from 0.
pub const JITTER_POINTS: u32 = 8;

/// `client_hover_probe` arguments. Exactly one of `entity_id` and `point`.
#[derive(Debug, Clone, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct HoverRequest {
    /// Entity aim: the entity must be under the cursor.
    #[serde(default)]
    pub entity_id: Option<u32>,
    /// Point aim: nothing (or `expect_entity`) may be under the cursor.
    #[serde(default)]
    pub point: Option<PointArg>,
    /// With `point` only: this entity under the cursor also passes.
    #[serde(default)]
    pub expect_entity: Option<u32>,
    /// Jitter ring radius, px (default 12).
    #[serde(default)]
    pub jitter_px: Option<u32>,
}

/// What the aim accepts under the cursor.
#[derive(Debug, Clone, Copy)]
enum AimKind {
    Entity(u32),
    Point { at: Vec3, expect: Option<u32> },
}

/// `client_hover_probe`.
pub async fn run<W: WorldIo>(io: &mut W, req: HoverRequest) -> Result<Value, WorldError> {
    let mut steps = Steps::new(TOOL, io.now_ms());
    let mut subject = TargetArg {
        entity_id: req.entity_id,
        name: None,
        point: req.point,
    }
    .describe();
    subject["expect_entity"] = json!(req.expect_entity);
    steps.subject(subject);
    let kind = match (req.entity_id, req.point) {
        (Some(e), None) if req.expect_entity.is_none() => AimKind::Entity(e),
        (None, Some(p)) => AimKind::Point {
            at: p.to_client(),
            expect: req.expect_entity,
        },
        (Some(_), None) => {
            return Err(steps.fail(
                io.now_ms(),
                "arguments",
                "expect_entity goes with point only",
            ))
        }
        _ => {
            return Err(steps.fail(
                io.now_ms(),
                "arguments",
                "give exactly one of entity_id and point",
            ))
        }
    };
    io.ensure_focus()
        .await
        .map_err(|e| steps.fail(io.now_ms(), "focus", e))?;
    let units = io
        .unit_slots()
        .await
        .map_err(|e| steps.fail(io.now_ms(), "unit_slots", e))?;
    // Read before any cursor move: without it the cursor cannot go back.
    let start = io
        .cursor()
        .await
        .map_err(|e| steps.fail(io.now_ms(), "read_cursor", e))?;

    let r = probe(io, &mut steps, &req, kind, units).await;

    // `place_cursor` without a mouse move: putting the cursor back must not
    // hover anything on the way.
    let restored = io.place_cursor(start.0, start.1, false).await;
    let mut out = match r {
        Ok(out) => out,
        Err(e) => return Err(with_restore_error(e, restored, start)),
    };
    out["cursor_restored"] = json!(restored.is_ok());
    if let Err(e) = restored {
        out["cursor_restore_error"] = json!(e);
    }
    Ok(steps.finish(io.now_ms(), out))
}

/// A probe failure with the cursor restore's own failure attached, so a
/// cursor left mid-screen is never a silent side effect of the first error.
fn with_restore_error(
    mut e: WorldError,
    restored: Result<(), String>,
    start: (i32, i32),
) -> WorldError {
    let Err(re) = restored else {
        return e;
    };
    tracing::warn!(
        tool = TOOL,
        step = %e.step,
        restore_error = %re,
        "client_hover_probe failed and could not put the cursor back"
    );
    let state = e.0.state.get_or_insert_with(|| json!({}));
    if !state.is_object() {
        *state = json!({ "state": state.take() });
    }
    state["cursor_restored"] = json!(false);
    state["cursor_restore_error"] = json!(re);
    state["cursor_start"] = json!([start.0, start.1]);
    e
}

/// Everything between reading the cursor and putting it back.
async fn probe<W: WorldIo>(
    io: &mut W,
    steps: &mut Steps,
    req: &HoverRequest,
    kind: AimKind,
    units: UnitSlots,
) -> Result<Value, WorldError> {
    // 1. Resolve.
    let (mut aim, found): (Aim, Option<Found>) = match kind {
        AimKind::Point { at, .. } => (Aim::Point(at), None),
        AimKind::Entity(id) => {
            let fr = FindRequest {
                entity_id: Some(id),
                rendered_only: true,
                limit: Some(1),
                ..FindRequest::default()
            };
            let r = find_entities(io, &fr, steps).await?;
            let Some(m) = r.matches.into_iter().next() else {
                return Err(steps.fail(
                    io.now_ms(),
                    "resolve_target",
                    "no rendered entity matches (see client_entity_find)",
                ));
            };
            let Some(pos) = m.pose.map(|p| p.pos) else {
                return Err(steps.fail(
                    io.now_ms(),
                    "resolve_target",
                    format!("entity {} has no pose", m.id),
                ));
            };
            (
                Aim::Actor {
                    actor: m.actor,
                    last: pos,
                },
                Some(m),
            )
        }
    };
    let entity_json = found.as_ref().map(Found::to_json);
    // Read once, before any cursor move: a tooltip the hover itself spawns
    // must not count as a window covering the pixel (worknote deviation).
    let rects = io
        .window_rects()
        .await
        .map_err(|e| steps.fail(io.now_ms(), "window_rects", e))?;
    // An unreadable avatar is reported as such, never as "off screen".
    let player_pos: Result<Vec3, String> = match io.snapshot().await {
        Ok(s) => s
            .player()
            .and_then(|p| p.pose)
            .map(|p| p.pos)
            .ok_or_else(|| "snapshot: the player has no pose".to_string()),
        Err(e) => Err(format!("snapshot: {e}")),
    };

    // 2. On screen? Never turn the camera.
    let t_proj = io.now_ms();
    let base = aim.position(io).await;
    let heights: Vec<Vec3> = match kind {
        AimKind::Entity(_) => HOVER_OFFSETS
            .iter()
            .map(|dz| Vec3::new(base.x, base.y, base.z + dz))
            .collect(),
        AimKind::Point { .. } => vec![base],
    };
    let pr = io
        .project(&heights)
        .await
        .map_err(|e| steps.fail(io.now_ms(), "project", e))?;
    let screen = pr.screen;
    let first_on = pr
        .points
        .iter()
        .flatten()
        .copied()
        .find(|p| screen.contains(*p, CLICK_MARGIN_PX));
    steps.record(
        "on_screen",
        t_proj,
        io.now_ms(),
        json!({ "on_screen": first_on.is_some() }),
    );
    let Some(first_on) = first_on else {
        return Ok(json!({
            "verdict": "off_screen",
            "pixel": pr.points.first().copied().flatten().map(|p| [p.0.round(), p.1.round()]),
            "screen": [screen.w, screen.h],
            "entity": entity_json,
        }));
    };

    // 3. Centre pixel.
    let centre = match kind {
        AimKind::Entity(e) => {
            let t = io.now_ms();
            let (chosen, tried) =
                hover_once(io, steps, &mut aim, Some(e), units.mouse_over, false).await?;
            steps.used(NativeLevel::RealInput);
            steps.record("centre", t, io.now_ms(), json!({ "tried": tried }));
            match chosen {
                Some((p, Hover::Matched)) => p,
                _ => first_on,
            }
        }
        AimKind::Point { .. } => first_on,
    };
    let (x, y) = (centre.0.round() as i32, centre.1.round() as i32);

    // 4. Score the centre and the ring.
    let t_score = io.now_ms();
    let (ok, hover) = score_at(io, steps, kind, units, &rects, (x, y)).await?;
    let radius = f64::from(req.jitter_px.unwrap_or(DEFAULT_JITTER_PX));
    let mut ring = Vec::new();
    let mut hits = 0u32;
    for k in 0..JITTER_POINTS {
        let a = f64::from(k) * std::f64::consts::TAU / f64::from(JITTER_POINTS);
        let (jx, jy) = (
            (f64::from(x) + radius * a.cos()).round() as i32,
            (f64::from(y) + radius * a.sin()).round() as i32,
        );
        // Pixels run 0..w and 0..h: x == w is already off the screen.
        if jx < 0 || jy < 0 || f64::from(jx) >= screen.w || f64::from(jy) >= screen.h {
            ring.push(json!({ "at": [jx, jy], "hover": "off_screen" }));
            continue;
        }
        let (jok, jhover) = score_at(io, steps, kind, units, &rects, (jx, jy)).await?;
        hits += u32::from(jok);
        ring.push(json!({ "at": [jx, jy], "hover": jhover, "ok": jok }));
    }
    steps.record(
        "score",
        t_score,
        io.now_ms(),
        json!({ "centre": hover, "ring": ring }),
    );

    // 5. Distances from the avatar and the screen edge. `avatar_px` null
    // with no `avatar_error` means the avatar is off screen; with one, the
    // distance is unknown.
    let (avatar_px, avatar_error) = match player_pos {
        Ok(p) => {
            let body: Vec<Vec3> = HOVER_OFFSETS
                .iter()
                .map(|dz| Vec3::new(p.x, p.y, p.z + dz))
                .collect();
            match io.project(&body).await {
                Ok(pr) => (
                    pr.points
                        .iter()
                        .flatten()
                        .map(|(ax, ay)| (ax - f64::from(x)).hypot(ay - f64::from(y)))
                        .min_by(f64::total_cmp)
                        .map(f64::round),
                    None,
                ),
                Err(e) => (None, Some(format!("project: {e}"))),
            }
        }
        Err(e) => (None, Some(e)),
    };
    let (fx, fy) = (f64::from(x), f64::from(y));
    let edge_px = fx.min(fy).min(screen.w - fx).min(screen.h - fy).round();

    let mut out = json!({
        "verdict": if ok { json!("ok") } else { hover.clone() },
        "pixel": [x, y],
        "hover": hover,
        "jitter_hits": hits,
        "jitter": JITTER_POINTS,
        "avatar_px": avatar_px,
        "edge_px": edge_px,
        "entity": entity_json,
    });
    if let Some(e) = avatar_error {
        out["avatar_error"] = json!(e);
    }
    Ok(out)
}

/// Hover one pixel: place the cursor (no button), wait, and classify what
/// the client reports under it.
async fn score_at<W: WorldIo>(
    io: &mut W,
    steps: &mut Steps,
    kind: AimKind,
    units: UnitSlots,
    rects: &[(String, [f64; 4])],
    (x, y): (i32, i32),
) -> Result<(bool, Value), WorldError> {
    io.place_cursor(x, y, true)
        .await
        .map_err(|e| steps.fail(io.now_ms(), "cursor", e))?;
    steps.used(NativeLevel::RealInput);
    io.sleep(HOVER_WAIT_MS).await;
    let slots = io
        .slots()
        .await
        .map_err(|e| steps.fail(io.now_ms(), "read_mouse_over", e))?;
    let mo = slot_entity(&slots, units.mouse_over);
    let player = slot_entity(&slots, units.player);
    Ok(classify(kind, mo, player, window_at(rects, x, y)))
}

/// The first window whose rect contains the pixel.
fn window_at(rects: &[(String, [f64; 4])], x: i32, y: i32) -> Option<&str> {
    let (x, y) = (f64::from(x), f64::from(y));
    rects
        .iter()
        .find(|(_, r)| x >= r[0] && x <= r[2] && y >= r[1] && y <= r[3])
        .map(|(n, _)| n.as_str())
}

/// `(ok, hover)` for one pixel. A HUD window wins: the cursor is on the UI,
/// so a click there never reaches the world.
fn classify(kind: AimKind, mo: u32, player: u32, ui: Option<&str>) -> (bool, Value) {
    if let Some(w) = ui {
        return (false, json!({ "ui": w }));
    }
    let want = match kind {
        AimKind::Entity(e) => Some(e),
        AimKind::Point { expect, .. } => expect,
    };
    if mo != 0 && Some(mo) == want {
        return (true, json!("entity"));
    }
    if mo == 0 {
        return (matches!(kind, AimKind::Point { .. }), json!("nothing"));
    }
    if mo == player {
        return (false, json!("own"));
    }
    (false, json!({ "other_entity": mo }))
}

#[cfg(test)]
#[path = "hover_tests.rs"]
mod tests;
