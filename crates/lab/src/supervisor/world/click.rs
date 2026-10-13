//! `client_world_click` and `client_target`: click an entity (or a world
//! point) in the 3D view like a player.
//!
//! 1. Resolve the entity and project it with the game's own view.
//! 2. Off screen: turn the camera with mouse-look until it is on screen
//!    (`rotate_camera`, default on), or fail naming the entity.
//! 3. Put the UI cursor on it and read the client's own `Unit.MouseOver`.
//!    Another entity under the cursor is an occluder: try higher and lower
//!    points on the body, then fail. Explicit entity clicks may use `force`.
//! 4. Click with real button messages and watch what the client did:
//!    `Unit.Target` and the visible windows, before and after.
//!
//! `client_target` expects the target to become the entity. When that fails
//! and `allow_fallback` is set, it calls the stock `targetUnit` on a pinned
//! slot and reports `native_level: ui_lua` — never a native pass.

use rmcp::schemars;
use serde_json::{json, Value};

use super::camera::{face, Aim, DEFAULT_FACE_STEPS};
use super::click_hover::{
    hover_once, read_mouse_over, wants_view_retry, Hover, HOVER_WAIT_MS, NEAR_AVATAR_M,
};
pub use super::click_hover::{restore_pitch_counts, VIEW_RETRIES};
use super::click_range::{range_hint, range_json};
// The hover_tests import path; the non-test build reads it from click_hover.
#[cfg(test)]
pub use super::click_hover::HOVER_OFFSETS;
use super::find::{find_entities, FindRequest, Found, CLICK_MARGIN_PX};
use super::geometry::distance_m;
use super::io::WorldIo;
use super::memory::{slot_entity, LAB_SLOT_BASE};
use super::steer::TurnModel;
use super::{NativeLevel, PointArg, Steps, TargetArg, WorldError};

/// Default wait for the click's effect.
pub const DEFAULT_SETTLE_MS: u64 = 1200;
const POLL_MS: u64 = 100;
/// Total time available for mouse-over verification, including view retries.
const TARGET_HOVER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
/// Private slot for the `targetUnit` fallback.
const FALLBACK_SLOT: i32 = LAB_SLOT_BASE - 1;

/// What the click must achieve to pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Expect {
    /// `Unit.Target` becomes the entity.
    Target,
    /// A top-level window opens.
    Window,
    /// Either of the above.
    Any,
    /// Nothing is required (report only; for "nothing should happen" steps).
    Nothing,
}

/// `client_world_click` / `client_target` arguments.
#[derive(Debug, Clone, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct ClickRequest {
    #[serde(default)]
    pub entity_id: Option<u32>,
    /// Nearest entity whose name contains this (case-insensitive).
    #[serde(default)]
    pub name: Option<String>,
    /// Click a world point. A point at a rendered entity requires its hover.
    #[serde(default)]
    pub point: Option<PointArg>,
    /// `left` or `right` (client_world_click default right = interact;
    /// client_target always left).
    #[serde(default)]
    pub button: Option<String>,
    /// Turn the camera when the entity is off screen (default true).
    #[serde(default)]
    pub rotate_camera: Option<bool>,
    /// Click an explicitly named entity even when another entity is under
    /// the cursor. Point clicks never force through occlusion.
    #[serde(default)]
    pub force: bool,
    /// client_target only: fall back to the stock `targetUnit` (N3).
    #[serde(default)]
    pub allow_fallback: bool,
    /// What must change (client_world_click default `any`; client_target
    /// is always `target`).
    #[serde(default)]
    pub expect: Option<Expect>,
    /// How long to wait for the effect, ms (default 1200).
    #[serde(default)]
    pub settle_ms: Option<u64>,
}

fn button_index(name: Option<&str>, default: usize) -> Result<usize, String> {
    match name.map(str::to_ascii_lowercase).as_deref() {
        None => Ok(default),
        Some("left") | Some("0") => Ok(0),
        Some("right") | Some("1") => Ok(1),
        Some(other) => Err(format!("button must be left or right, not {other:?}")),
    }
}

/// Evidence read before and after the click.
#[derive(Debug, Clone, PartialEq)]
struct UiSnap {
    target: u32,
    mouse_over: u32,
    windows: Vec<String>,
}

async fn ui_snap<W: WorldIo>(io: &mut W, target_slot: i32, mo_slot: i32) -> Result<UiSnap, String> {
    let slots = io.slots().await?;
    Ok(UiSnap {
        target: slot_entity(&slots, target_slot),
        mouse_over: slot_entity(&slots, mo_slot),
        windows: io.visible_windows().await?,
    })
}

/// Is `after` what `expect` asked for?
fn met(expect: Expect, entity: Option<u32>, before: &UiSnap, after: &UiSnap) -> bool {
    let targeted = entity.is_some_and(|e| after.target == e)
        || (entity.is_none() && after.target != before.target);
    let opened = after.windows.iter().any(|w| !before.windows.contains(w));
    match expect {
        Expect::Target => targeted,
        Expect::Window => opened,
        Expect::Any => targeted || opened,
        Expect::Nothing => true,
    }
}

/// Shared by both tools. `tool` names the result; `forced_button` pins the
/// button for `client_target`.
pub async fn run<W: WorldIo>(
    io: &mut W,
    tool: &'static str,
    req: ClickRequest,
    forced: Option<(usize, Expect)>,
) -> Result<Value, WorldError> {
    let mut steps = Steps::new(tool, io.now_ms());
    let target = TargetArg {
        entity_id: req.entity_id,
        name: req.name.clone(),
        point: req.point,
    };
    steps.subject(target.describe());
    if target.is_empty() {
        return Err(steps.fail(io.now_ms(), "arguments", "give entity_id, name or point"));
    }
    let (button, expect) = match forced {
        Some(f) => f,
        None => (
            button_index(req.button.as_deref(), 1)
                .map_err(|e| steps.fail(io.now_ms(), "arguments", e))?,
            req.expect.unwrap_or(if req.point.is_some() {
                Expect::Nothing
            } else {
                Expect::Any
            }),
        ),
    };
    io.ensure_focus()
        .await
        .map_err(|e| steps.fail(io.now_ms(), "focus", e))?;
    let units = io
        .unit_slots()
        .await
        .map_err(|e| steps.fail(io.now_ms(), "unit_slots", e))?;

    // 1. Resolve.
    let (mut aim, entity, found): (Aim, Option<u32>, Option<Found>) = match target.point {
        Some(p) => {
            let point = p.to_client();
            let snap = io
                .snapshot()
                .await
                .map_err(|e| steps.fail(io.now_ms(), "read_entities", e))?;
            // Server points are commonly rounded to centimetres. Offset
            // points intentionally aimed at a body part stay distinct.
            let candidates: Vec<_> = snap
                .entities
                .iter()
                .filter(|e| e.id != snap.player_id && e.rendered())
                .filter(|e| {
                    e.pose
                        .is_some_and(|pose| distance_m(point, pose.pos) <= 0.05)
                })
                .map(|e| e.id)
                .collect();
            if candidates.len() > 1 {
                return Err(steps.fail_with(
                    io.now_ms(),
                    "resolve_target",
                    "world point matches multiple rendered entities; supply entity_id",
                    json!({ "candidate_ids": candidates }),
                ));
            }
            if let Some(id) = candidates.first().copied() {
                let fr = FindRequest {
                    entity_id: Some(id),
                    rendered_only: true,
                    limit: Some(1),
                    ..FindRequest::default()
                };
                let r = find_entities(io, &fr, &mut steps).await?;
                let m = r.matches.into_iter().next().ok_or_else(|| {
                    steps.fail(
                        io.now_ms(),
                        "resolve_target",
                        format!("matched entity {id} disappeared"),
                    )
                })?;
                let pos = m.pose.ok_or_else(|| {
                    steps.fail(
                        io.now_ms(),
                        "resolve_target",
                        format!("matched entity {id} has no pose"),
                    )
                })?;
                (
                    Aim::Actor {
                        actor: m.actor,
                        last: pos.pos,
                    },
                    Some(id),
                    Some(m),
                )
            } else {
                (Aim::Point(point), None, None)
            }
        }
        None => {
            let fr = FindRequest {
                entity_id: target.entity_id,
                name: target.name.clone(),
                rendered_only: true,
                limit: Some(1),
                ..FindRequest::default()
            };
            let r = find_entities(io, &fr, &mut steps).await?;
            let Some(m) = r.matches.into_iter().next() else {
                return Err(steps.fail(
                    io.now_ms(),
                    "resolve_target",
                    "no rendered entity matches (see client_entity_find)",
                ));
            };
            let pos = m.pose.map(|p| p.pos).ok_or_else(|| {
                steps.fail(
                    io.now_ms(),
                    "resolve_target",
                    format!("entity {} has no pose", m.id),
                )
            })?;
            (
                Aim::Actor {
                    actor: m.actor,
                    last: pos,
                },
                Some(m.id),
                Some(m),
            )
        }
    };
    if let Some(f) = &found {
        steps.subject(json!({ "entity_id": f.id, "name": f.name() }));
    }

    // The budget starts before the first bridge read. A stalled client must
    // not hold a target attempt indefinitely before mouse-over begins.
    let hover_deadline = tokio::time::Instant::now() + TARGET_HOVER_TIMEOUT;
    let mut tried = Vec::new();
    let mut attempted_pixel = None;
    macro_rules! hover_wait {
        ($phase:expr, $future:expr) => {
            match tokio::time::timeout_at(hover_deadline, $future).await {
                Ok(value) => value,
                Err(_) => {
                    return Err(hover_deadline_error(
                        io,
                        &steps,
                        $phase,
                        attempted_pixel,
                        &tried,
                        hover_deadline,
                    ))
                }
            }
        };
    }
    let before = hover_wait!("read_before", ui_snap(io, units.target, units.mouse_over))
        .map_err(|e| steps.fail(io.now_ms(), "read_before", e))?;

    // 2. On screen?
    let player = hover_wait!("snapshot", io.snapshot())
        .ok()
        .and_then(|s| s.player().and_then(|p| p.pose));
    let pos = hover_wait!("position", aim.position(io));
    let pr = hover_wait!("project", io.project(&[pos]))
        .map_err(|e| steps.fail(io.now_ms(), "project", e))?;
    let on_screen = pr
        .points
        .first()
        .copied()
        .flatten()
        .is_some_and(|p| pr.screen.contains(p, CLICK_MARGIN_PX));
    let mut face_json = Value::Null;
    if !on_screen {
        if !req.rotate_camera.unwrap_or(true) {
            return Err(steps.fail_with(
                io.now_ms(),
                "on_screen",
                "the target is off screen and rotate_camera is false",
                json!({ "pixel": pr.points.first(), "screen": [pr.screen.w, pr.screen.h] }),
            ));
        }
        let rep = hover_wait!(
            "face",
            face(
                io,
                &mut steps,
                &mut aim,
                player,
                TurnModel::default(),
                DEFAULT_FACE_STEPS,
            )
        )?;
        face_json = rep.to_json();
        if !rep
            .pixel
            .is_some_and(|p| rep.screen.is_some_and(|s| s.contains(p, CLICK_MARGIN_PX)))
        {
            return Err(steps.fail_with(
                io.now_ms(),
                "on_screen",
                format!(
                    "could not turn the camera onto the target{}",
                    rep.clamp_note()
                ),
                face_json,
            ));
        }
    }

    // How far the target is from the avatar, straight line (the server's
    // interact range is 3D). Near targets sit behind or under the avatar.
    let range_m = match (player, entity) {
        (Some(p), Some(_)) => Some(distance_m(p.pos, hover_wait!("position", aim.position(io)))),
        _ => None,
    };
    let near = range_m.is_some_and(|d| d <= NEAR_AVATAR_M);
    // 3. Hover: find a point on the body the client says is the entity.
    // When every point is covered or off screen, change the view (pitch
    // up over the occluder, yaw each way, zoom in) and try again; the bank
    // UAT needed a hand-made pitch for exactly this (2026-10-10).
    let t_hover = io.now_ms();
    let mut adjusted = Vec::new();
    let mut chosen = None;
    let mut start_view: Option<Value> = None;
    let mut zoomed = 0;
    let mut saw_own = false;
    for (i, adj) in std::iter::once(None)
        .chain(VIEW_RETRIES.iter().map(Some))
        .enumerate()
    {
        if tokio::time::Instant::now() >= hover_deadline {
            return Err(hover_deadline_error(
                io,
                &steps,
                "hover",
                attempted_pixel,
                &tried,
                hover_deadline,
            ));
        }
        if let Some(&(dx, dy, notches)) = adj {
            if entity.is_none() || !req.rotate_camera.unwrap_or(true) {
                break;
            }
            if start_view.is_none() {
                start_view = hover_wait!("camera_readout", io.camera_readout()).ok();
            }
            zoomed += notches;
            let level = hover_wait!(
                "view_retry",
                io.look(dx, dy, notches * super::camera::WHEEL_NOTCH)
            )
            .map_err(|e| steps.fail(io.now_ms(), "view_retry", e))?;
            steps.used(level);
            hover_wait!("view_retry", io.sleep(super::camera::LOOK_SETTLE_MS));
            adjusted.push(json!({ "retry": i, "yaw_counts": dx, "pitch_counts": dy, "zoom_notches": notches }));
        }
        let (c, t) = hover_wait!(
            "hover",
            hover_once(
                io,
                &mut steps,
                &mut aim,
                entity,
                units.mouse_over,
                units.player,
                req.force && req.point.is_none(),
                Some(hover_deadline),
                &mut attempted_pixel,
            )
        )?;
        saw_own |= t.iter().any(|h| h["hover"] == "own_avatar");
        tried = t;
        chosen = c;
        if !wants_view_retry(&chosen, &tried, near, req.force && req.point.is_none()) {
            break;
        }
    }
    if tokio::time::Instant::now() >= hover_deadline {
        return Err(hover_deadline_error(
            io,
            &steps,
            "hover",
            attempted_pixel,
            &tried,
            hover_deadline,
        ));
    }
    let mut hover_detail = json!({ "tried": tried });
    hover_detail["candidate_ids"] = json!(tried
        .iter()
        .filter_map(|t| {
            t["hover"]["other_entity"]
                .as_u64()
                .and_then(|id| u32::try_from(id).ok())
        })
        .collect::<Vec<_>>());
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
        let now = hover_wait!("restore_camera_readout", io.camera_readout()).ok();
        let dy = match (&start_view, &now) {
            (Some(a), Some(b)) => restore_pitch_counts(a, b),
            _ => 0,
        };
        let restored = hover_wait!(
            "restore_view",
            io.look(0, dy, -zoomed * super::camera::WHEEL_NOTCH)
        )
        .is_ok();
        hover_detail["view_restored"] = json!(restored);
        if restored {
            hover_wait!("restore_view", io.sleep(super::camera::LOOK_SETTLE_MS));
            let (c, t) = hover_wait!(
                "hover_after_restore",
                hover_once(
                    io,
                    &mut steps,
                    &mut aim,
                    entity,
                    units.mouse_over,
                    units.player,
                    req.force && req.point.is_none(),
                    Some(hover_deadline),
                    &mut attempted_pixel,
                )
            )?;
            hover_detail["tried_after_restore"] = json!(t);
            chosen = c;
        }
    }
    steps.record("hover", t_hover, io.now_ms(), hover_detail.clone());
    if req.point.is_some() && !matches!(chosen, Some((_, Hover::Matched))) {
        if let Some(id) = entity {
            return Err(steps.fail_with(io.now_ms(), "hover",
                "point matched an entity but its ID was not verified under the cursor; target is occluded",
                json!({ "candidate_ids": [id], "hover": hover_detail })));
        }
    }
    let Some((pixel, hover)) = chosen else {
        return Err(steps.fail_with(
            io.now_ms(),
            "hover",
            if saw_own {
                if req.point.is_some() { "your own avatar covers the point: stand 3.5-4.5 m to the side and retry; force cannot bypass point-click hover verification" } else { "your own avatar covers the target at every height tried, even after the view retries: stand 3.5-4.5 m to the side of it (or turn the camera with client_camera yaw counts) and click with rotate_camera=false; force=true clicks anyway" }
            } else {
                if req.point.is_some() { "the point is off screen or occluded: use a clear view or explicit entity_id; force cannot bypass point-click hover verification" } else { "every point on the target is off screen or covered by another entity, after the view retries (force=true clicks anyway)" }
            },
            hover_detail,
        ));
    };
    let (x, y) = (pixel.0.round() as i32, pixel.1.round() as i32);
    if req.point.is_some() && entity.is_some()
        || !matches!(hover, Hover::Matched)
        || tried.len() > 1
    {
        attempted_pixel = Some((x, y));
        hover_wait!("cursor", io.place_cursor(x, y, true))
            .map_err(|e| steps.fail(io.now_ms(), "cursor", e))?;
        hover_wait!("cursor", io.sleep(HOVER_WAIT_MS));
    }
    if let (Some(id), Some(_)) = (entity, req.point) {
        let slots = hover_wait!(
            "final_hover",
            read_mouse_over(io, &steps, (x, y), Some(hover_deadline))
        )?;
        let observed = slot_entity(&slots, units.mouse_over);
        if observed != id {
            return Err(steps.fail_with(io.now_ms(), "hover",
                "point matched an entity, but its ID was not under the cursor immediately before input; force cannot bypass point-click hover verification",
                json!({ "candidate_ids": [id], "mouse_over": observed, "at": [x, y], "hover": hover_detail })));
        }
    }
    let plan = ClickPlan {
        button,
        expect,
        entity,
        units,
        before,
        at: (x, y),
        hover_verified: matches!(hover, Hover::Matched),
        face_json,
        found,
        range_m,
    };
    click_and_observe(io, steps, &req, plan).await
}

fn hover_deadline_error<W: WorldIo>(
    io: &W,
    steps: &Steps,
    phase: &str,
    attempted_pixel: Option<(i32, i32)>,
    tried: &[Value],
    deadline: tokio::time::Instant,
) -> WorldError {
    steps.fail_with(
        io.now_ms(),
        "hover_timeout",
        format!("target hover attempt exceeded its 10 s deadline during {phase}"),
        json!({ "attempted_pixel": attempted_pixel, "remaining_deadline_ms": deadline.saturating_duration_since(tokio::time::Instant::now()).as_millis() as u64, "tried": tried }),
    )
}

/// What the click step needs from the resolve and hover steps.
struct ClickPlan {
    button: usize,
    expect: Expect,
    entity: Option<u32>,
    units: super::lua::UnitSlots,
    before: UiSnap,
    at: (i32, i32),
    hover_verified: bool,
    face_json: Value,
    found: Option<Found>,
    range_m: Option<f64>,
}

/// Steps 4 and 5: click, then watch what the client did.
async fn click_and_observe<W: WorldIo>(
    io: &mut W,
    mut steps: Steps,
    req: &ClickRequest,
    plan: ClickPlan,
) -> Result<Value, WorldError> {
    let ClickPlan {
        button,
        expect,
        entity,
        units,
        before,
        at: (x, y),
        hover_verified,
        face_json,
        found,
        range_m,
    } = plan;
    // 4. Click and watch.
    let t_click = io.now_ms();
    io.button(button, true)
        .await
        .map_err(|e| steps.fail(io.now_ms(), "click", e))?;
    io.sleep(80).await;
    io.button(button, false)
        .await
        .map_err(|e| steps.fail(io.now_ms(), "click", e))?;
    steps.used(NativeLevel::RealInput);
    steps.record(
        "click",
        t_click,
        io.now_ms(),
        json!({ "button": button, "at": [x, y] }),
    );

    let settle = req.settle_ms.unwrap_or(DEFAULT_SETTLE_MS);
    let t_wait = io.now_ms();
    let mut after = before.clone();
    let mut ok = false;
    while io.now_ms().saturating_sub(t_wait) <= settle {
        io.sleep(POLL_MS).await;
        after = ui_snap(io, units.target, units.mouse_over)
            .await
            .map_err(|e| steps.fail(io.now_ms(), "read_after", e))?;
        if expect != Expect::Nothing && met(expect, entity, &before, &after) {
            ok = true;
            break;
        }
    }
    if expect == Expect::Nothing {
        ok = true;
    }
    steps.record("observe", t_wait, io.now_ms(), json!({ "met": ok }));

    let mut fallback = Value::Null;
    if !ok && expect == Expect::Target && req.allow_fallback {
        if let Some(e) = entity {
            let t_fb = io.now_ms();
            io.pin(FALLBACK_SLOT, e)
                .await
                .map_err(|err| steps.fail(io.now_ms(), "fallback_pin", err))?;
            io.target_unit(FALLBACK_SLOT)
                .await
                .map_err(|err| steps.fail(io.now_ms(), "fallback_target_unit", err))?;
            steps.used(NativeLevel::UiLua);
            io.sleep(POLL_MS * 2).await;
            after = ui_snap(io, units.target, units.mouse_over)
                .await
                .map_err(|err| steps.fail(io.now_ms(), "read_after", err))?;
            ok = met(expect, entity, &before, &after);
            fallback = json!({ "used": "targetUnit", "met": ok });
            steps.record("fallback", t_fb, io.now_ms(), fallback.clone());
        }
    }

    let opened: Vec<&String> = after
        .windows
        .iter()
        .filter(|w| !before.windows.contains(w))
        .collect();
    let closed: Vec<&String> = before
        .windows
        .iter()
        .filter(|w| !after.windows.contains(w))
        .collect();
    let evidence = json!({
        "target_before": before.target,
        "target_after": after.target,
        "target_is_entity": entity.is_some_and(|e| after.target == e),
        "windows_opened": opened,
        "windows_closed": closed,
        "mouse_over_after": after.mouse_over,
        "range": range_json(range_m),
    });
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
    let out = json!({
        "entity": found.as_ref().map(Found::to_json),
        "button": if button == 0 { "left" } else { "right" },
        "clicked_at": [x, y],
        "hover_verified": hover_verified,
        "expect": format!("{expect:?}").to_lowercase(),
        "result": evidence,
        "camera": face_json,
        "fallback": fallback,
    });
    Ok(steps.finish(io.now_ms(), out))
}

#[cfg(test)]
#[path = "click_tests.rs"]
mod tests;
