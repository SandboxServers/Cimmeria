//! The hover half of `client_world_click`: which body height the client says
//! is the entity, and when to change the view and look again.

use serde_json::{json, Value};

use super::camera::Aim;
use super::find::CLICK_MARGIN_PX;
use super::geometry::Vec3;
use super::io::WorldIo;
use super::memory::slot_entity;
use super::{Steps, WorldError};

/// Heights tried on an entity's body, UE3 units (100 per metre) above its
/// actor `Location`, which is its FEET (probe B-P2: hovering the feet gives
/// nothing). In order: chest, shoulders, head, knee. The own avatar covers the
/// lower band when the camera is behind it, so the higher heights come first
/// after the chest and the knee comes last.
pub const HOVER_OFFSETS: [f64; 4] = [100.0, 140.0, 170.0, 60.0];
/// How long the client gets to update mouse-over after the cursor moves.
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
            let covered = tried
                .iter()
                .any(|t| t["hover"].get("other_entity").is_some() || t["hover"] == "own_avatar");
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
        let own = [
            json!({ "hover": "own_avatar" }),
            json!({ "hover": "nothing" }),
        ];
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

    /// The aim never touches the feet (probe B-P2): every height is a body
    /// point, the chest comes first, and the head is within a 1.8 m humanoid.
    #[test]
    fn the_aim_heights_are_above_the_feet() {
        assert_eq!(HOVER_OFFSETS[0], 100.0);
        assert!(HOVER_OFFSETS.iter().all(|dz| (60.0..=180.0).contains(dz)));
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
