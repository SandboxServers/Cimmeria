//! Evidence for `client_camera` (AX-33): did the view move the way the call
//! asked? Pure over plain values so the tests need no `Sim`.
//!
//! Judged on the controller readout (`zoom`, `yaw_offset_deg`,
//! `pitch_offset_deg`), never on the camera actor pose: the actor pose can read
//! all zeros on the live client.

use serde_json::{json, Value};

use super::camera::{pinned_pitch_of, CameraRequest, ABS_TOLERANCE_DEG};
use super::geometry::wrap_pi;
use crate::proof::fresh::{mark_unstamped, Freshness};
use crate::proof::{Check, Source, Why};

/// A look must move an offset by this many degrees to count.
const LOOK_MIN_DEG: f64 = 0.1;
/// A zoom must move the distance by this much to count.
const ZOOM_MIN_UNITS: f64 = 1.0;
/// How close a `zoom_to` must land.
const ZOOM_TO_TOLERANCE: f64 = 30.0;

fn num(v: &Value, key: &str) -> Option<f64> {
    v[key].as_f64().filter(|n| n.is_finite())
}

/// The checks for one `client_camera` call.
pub(super) fn checks(
    req: &CameraRequest,
    view_before: &Value,
    view_after: &Value,
    face: &Value,
    absolute: Option<&Value>,
    fresh: Freshness,
) -> Vec<Check> {
    let mut out = Vec::new();
    // No post-read: nothing below can be measured.
    let no_read = view_after.get("error").is_some();
    let stale = fresh == Freshness::Stale;
    let unstamped = fresh == Freshness::Unstamped;
    let blind = |name: &str| Check::new(name, Source::Client, None, true).with_why(Why::NoPostRead);
    let stamp = |after: Value| {
        if unstamped {
            mark_unstamped(after)
        } else {
            after
        }
    };

    if req.yaw_counts.unwrap_or(0) != 0 || req.pitch_counts.unwrap_or(0) != 0 {
        let name = "look_moved";
        let (yb, ya) = (
            num(view_before, "yaw_offset_deg"),
            num(view_after, "yaw_offset_deg"),
        );
        let (pb, pa) = (
            num(view_before, "pitch_offset_deg"),
            num(view_after, "pitch_offset_deg"),
        );
        if no_read || (yb.is_none() && pb.is_none()) || (ya.is_none() && pa.is_none()) {
            out.push(blind(name));
        } else {
            // Same wrap rule as the face loop: offsets are rotator degrees.
            let dyaw = match (yb, ya) {
                (Some(b), Some(a)) => wrap_pi((a - b).to_radians()).to_degrees().abs(),
                _ => 0.0,
            };
            let dpitch = match (pb, pa) {
                (Some(b), Some(a)) => (a - b).abs(),
                _ => 0.0,
            };
            let moved = dyaw >= LOOK_MIN_DEG || dpitch >= LOOK_MIN_DEG;
            let mut after = json!({ "yaw_offset_deg": ya, "pitch_offset_deg": pa });
            if pinned_pitch_of(view_after).is_some() {
                after["pitch_clamped"] = json!(true);
            }
            out.push(
                Check::new(
                    name,
                    Source::Client,
                    if stale { None } else { Some(moved) },
                    true,
                )
                .with_before(json!({ "yaw_offset_deg": yb, "pitch_offset_deg": pb }))
                .with_after(stamp(after))
                .with_why(if stale { Why::StaleRead } else { Why::Ignored }),
            );
        }
    }

    let zoom_to = req.zoom_to.or(req.zoom).map(f64::from);
    if zoom_to.is_some() || req.zoom_notches.unwrap_or(0) != 0 {
        let name = "zoom_changed";
        match (num(view_before, "zoom"), num(view_after, "zoom")) {
            (Some(b), Some(a)) if !no_read => {
                let changed = (a - b).abs() >= ZOOM_MIN_UNITS
                    || zoom_to.is_some_and(|to| (a - to).abs() <= ZOOM_TO_TOLERANCE);
                out.push(
                    Check::new(
                        name,
                        Source::Client,
                        if stale { None } else { Some(changed) },
                        true,
                    )
                    .with_before(json!(b))
                    .with_after(stamp(json!(a)))
                    .with_why(if stale {
                        Why::StaleRead
                    } else {
                        Why::Ignored
                    }),
                );
            }
            _ => out.push(blind(name)),
        }
    }

    if req.face_entity_id.is_some() || req.face_name.is_some() || req.face_point.is_some() {
        out.push(if no_read {
            blind("face_centred")
        } else {
            Check::new(
                "face_centred",
                Source::Client,
                Some(face["centred"] == true),
                true,
            )
            .with_after(face.clone())
            .with_why(Why::NoEffect)
        });
    }

    if let Some(want) = req.pitch_deg {
        out.push(match num(view_after, "pitch_offset_deg") {
            Some(got) if !no_read => Check::new(
                "pitch_at",
                Source::Client,
                Some((got - want).abs() <= ABS_TOLERANCE_DEG),
                true,
            )
            .with_before(json!(want))
            .with_after(stamp(json!(got)))
            .with_why(Why::NoEffect),
            _ => blind("pitch_at"),
        });
    }
    if let Some(want) = req.yaw_offset_deg {
        // The yaw loop measures the camera actor's turn, so its own report is
        // the reading; an open-loop run has none.
        let got = absolute.and_then(|a| num(&a["yaw"], "got"));
        out.push(match got {
            Some(got) if !no_read => Check::new(
                "yaw_at",
                Source::Client,
                Some((got - want).abs() <= ABS_TOLERANCE_DEG),
                true,
            )
            .with_before(json!(want))
            .with_after(stamp(json!(got)))
            .with_why(Why::NoEffect),
            _ => blind("yaw_at"),
        });
    }
    out
}
