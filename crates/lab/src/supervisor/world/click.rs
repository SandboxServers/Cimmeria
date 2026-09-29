//! `client_world_click` and `client_target`: click an entity (or a world
//! point) in the 3D view like a player.
//!
//! 1. Resolve the entity and project it with the game's own view.
//! 2. Off screen: turn the camera with mouse-look until it is on screen
//!    (`rotate_camera`, default on), or fail naming the entity.
//! 3. Put the UI cursor on it and read the client's own `Unit.MouseOver`.
//!    Another entity under the cursor is an occluder: try higher and lower
//!    points on the body, then fail (unless `force`).
//! 4. Click with real button messages and watch what the client did:
//!    `Unit.Target` and the visible windows, before and after.
//!
//! `client_target` expects the target to become the entity. When that fails
//! and `allow_fallback` is set, it calls the stock `targetUnit` on a pinned
//! slot and reports `native_level: ui_lua` — never a native pass.

use rmcp::schemars;
use serde_json::{json, Value};

use super::camera::{face, Aim, DEFAULT_FACE_STEPS};
use super::find::{find_entities, FindRequest, Found, CLICK_MARGIN_PX};
use super::geometry::Vec3;
use super::io::WorldIo;
use super::memory::{slot_entity, LAB_SLOT_BASE};
use super::steer::TurnModel;
use super::{NativeLevel, PointArg, Steps, TargetArg, WorldError};

/// Heights tried on an entity's body, UE3 units above its actor location
/// (the collision centre): centre, chest, legs, head.
pub const HOVER_OFFSETS: [f64; 4] = [0.0, 50.0, -40.0, 90.0];
/// Default wait for the click's effect.
pub const DEFAULT_SETTLE_MS: u64 = 1200;
const POLL_MS: u64 = 100;
const HOVER_WAIT_MS: u64 = 120;
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
    /// Click a world point instead of an entity (no mouse-over check).
    #[serde(default)]
    pub point: Option<PointArg>,
    /// `left` or `right` (client_world_click default right = interact;
    /// client_target always left).
    #[serde(default)]
    pub button: Option<String>,
    /// Turn the camera when the entity is off screen (default true).
    #[serde(default)]
    pub rotate_camera: Option<bool>,
    /// Click even when another entity is under the cursor.
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

/// How the hover went at one height.
#[derive(Debug, Clone, PartialEq)]
enum Hover {
    Matched,
    Nothing,
    Other(u32),
    OffScreen,
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
        Some(p) => (Aim::Point(p.to_client()), None, None),
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

    let before = ui_snap(io, units.target, units.mouse_over)
        .await
        .map_err(|e| steps.fail(io.now_ms(), "read_before", e))?;

    // 2. On screen?
    let player = io
        .snapshot()
        .await
        .ok()
        .and_then(|s| s.player().and_then(|p| p.pose));
    let pos = aim.position(io).await;
    let pr = io
        .project(&[pos])
        .await
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
        let rep = face(
            io,
            &mut steps,
            &mut aim,
            player,
            TurnModel::default(),
            DEFAULT_FACE_STEPS,
        )
        .await?;
        face_json = rep.to_json();
        if !rep
            .pixel
            .is_some_and(|p| rep.screen.is_some_and(|s| s.contains(p, CLICK_MARGIN_PX)))
        {
            return Err(steps.fail_with(
                io.now_ms(),
                "on_screen",
                "could not turn the camera onto the target",
                face_json,
            ));
        }
    }

    // 3. Hover: find a point on the body the client says is the entity.
    let t_hover = io.now_ms();
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
        for mouse_move in [false, true] {
            io.place_cursor(x, y, mouse_move)
                .await
                .map_err(|err| steps.fail(io.now_ms(), "cursor", err))?;
            io.sleep(HOVER_WAIT_MS).await;
            let slots = io
                .slots()
                .await
                .map_err(|err| steps.fail(io.now_ms(), "read_mouse_over", err))?;
            let mo = slot_entity(&slots, units.mouse_over);
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
            Hover::Other(o) if req.force && chosen.is_none() => chosen = Some((p, Hover::Other(o))),
            _ => {}
        }
    }
    steps.record("hover", t_hover, io.now_ms(), json!({ "tried": tried }));
    let Some((pixel, hover)) = chosen else {
        return Err(steps.fail_with(
            io.now_ms(),
            "hover",
            "every point on the target is off screen or covered by another entity (force=true clicks anyway)",
            json!({ "tried": tried }),
        ));
    };
    let (x, y) = (pixel.0.round() as i32, pixel.1.round() as i32);
    if !matches!(hover, Hover::Matched) || tried.len() > 1 {
        io.place_cursor(x, y, true)
            .await
            .map_err(|e| steps.fail(io.now_ms(), "cursor", e))?;
        io.sleep(HOVER_WAIT_MS).await;
    }

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
    });
    if !ok {
        return Err(steps.fail_with(
            io.now_ms(),
            "observe",
            format!("the click did not produce the expected {expect:?} within {settle} ms"),
            evidence,
        ));
    }
    let out = json!({
        "entity": found.as_ref().map(Found::to_json),
        "button": if button == 0 { "left" } else { "right" },
        "clicked_at": [x, y],
        "hover_verified": matches!(hover, Hover::Matched),
        "expect": format!("{expect:?}").to_lowercase(),
        "result": evidence,
        "camera": face_json,
        "fallback": fallback,
    });
    Ok(steps.finish(io.now_ms(), out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(target: u32, windows: &[&str]) -> UiSnap {
        UiSnap {
            target,
            mouse_over: 0,
            windows: windows.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn expectations_read_target_and_windows() {
        let before = snap(0, &["ChatWin"]);
        let targeted = snap(42, &["ChatWin"]);
        let opened = snap(0, &["ChatWin", "DialogWin"]);
        assert!(met(Expect::Target, Some(42), &before, &targeted));
        assert!(!met(Expect::Target, Some(7), &before, &targeted));
        assert!(met(Expect::Window, Some(42), &before, &opened));
        assert!(!met(Expect::Window, Some(42), &before, &targeted));
        assert!(met(Expect::Any, Some(42), &before, &opened));
        assert!(met(Expect::Nothing, Some(42), &before, &before));
    }

    #[test]
    fn buttons_parse_by_name() {
        assert_eq!(button_index(None, 1), Ok(1));
        assert_eq!(button_index(Some("Left"), 1), Ok(0));
        assert!(button_index(Some("middle"), 1).is_err());
    }
}
