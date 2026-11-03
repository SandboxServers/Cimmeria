//! The bounded 3D scan for small props (`prop_scan`): when none of the body
//! heights hover a vial or a switch, look for a pixel the client's own pick
//! (`Unit.MouseOver`) resolves to the entity, in a small box around its actor
//! origin. The heights in `ENTITY_HOVER_OFFSETS` float 0.6 to 1.7 m above a
//! prop that is a few centimetres tall, so they land on the wall or desk
//! behind it (click-pick research, 2026-11-03).

use serde_json::{json, Value};

use super::camera::Aim;
use super::click_hover::{read_mouse_over, HOVER_WAIT_MS};
use super::find::CLICK_MARGIN_PX;
use super::geometry::Vec3;
use super::io::WorldIo;
use super::memory::slot_entity;
use super::{Steps, WorldError};

/// Half-width of the scan box in x and y, client units (cm): about 15 cm.
pub const SCAN_HALF_XY: f64 = 15.0;
/// Grid step in x and y.
pub const SCAN_STEP_XY: f64 = 5.0;
/// Heights above the actor origin: 0 to 40 cm.
pub const SCAN_DZ: [f64; 5] = [0.0, 10.0, 20.0, 30.0, 40.0];
/// Candidates closer than this to a kept one add nothing: the pick has no
/// finer resolution than a few pixels, and every kept pixel costs a read.
pub const SCAN_MIN_SEPARATION_PX: f64 = 3.0;
/// Hard cap on cursor placements plus mouse-over reads (about 7 s at 120 ms).
pub const SCAN_MAX_READS: usize = 60;

/// One projected scan point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ScanPoint {
    pub dz: f64,
    pub px: (f64, f64),
}

/// The scan box around `base`, in world space.
pub(super) fn scan_box(base: Vec3) -> Vec<(Vec3, f64)> {
    let n = (SCAN_HALF_XY / SCAN_STEP_XY) as i32;
    let mut out = Vec::new();
    for dz in SCAN_DZ {
        for ix in -n..=n {
            for iy in -n..=n {
                out.push((
                    Vec3::new(
                        base.x + f64::from(ix) * SCAN_STEP_XY,
                        base.y + f64::from(iy) * SCAN_STEP_XY,
                        base.z + dz,
                    ),
                    dz,
                ));
            }
        }
    }
    out
}

/// Keep the on-screen points, thin out near-duplicates and order by distance
/// from the origin pixel `centre` (the most likely hit first), at most `cap`.
pub(super) fn pick_candidates(
    pts: &[Option<(f64, f64)>],
    dzs: &[f64],
    on_screen: impl Fn((f64, f64)) -> bool,
    centre: (f64, f64),
    cap: usize,
) -> Vec<ScanPoint> {
    let dist = |p: (f64, f64)| (p.0 - centre.0).hypot(p.1 - centre.1);
    let mut all: Vec<ScanPoint> = pts
        .iter()
        .zip(dzs)
        .filter_map(|(p, dz)| {
            let px = (*p)?;
            on_screen(px).then_some(ScanPoint { dz: *dz, px })
        })
        .collect();
    all.sort_by(|a, b| dist(a.px).total_cmp(&dist(b.px)));
    let mut kept: Vec<ScanPoint> = Vec::new();
    for c in all {
        if kept.len() >= cap {
            break;
        }
        let near = kept
            .iter()
            .any(|k| (k.px.0 - c.px.0).hypot(k.px.1 - c.px.1) < SCAN_MIN_SEPARATION_PX);
        if !near {
            kept.push(c);
        }
    }
    kept
}

/// What a scan found, for the `tried` list.
#[derive(Debug)]
pub(super) struct ScanOutcome {
    pub hit: Option<((f64, f64), f64)>,
    pub detail: Value,
}

/// Scan the box around the entity's origin until `Unit.MouseOver == entity`.
/// Candidates come from one `project` call; each is hovered with the same
/// cursor placement as `hover_once`. A pixel under the own avatar or another
/// entity is skipped, never taken. The deadline ends the scan quietly (the
/// caller reports a deadline); a read failure is an error as in `hover_once`.
pub(super) async fn scan_for_prop<W: WorldIo>(
    io: &mut W,
    steps: &mut Steps,
    aim: &mut Aim,
    entity: u32,
    mo_slot: i32,
    player_slot: i32,
    deadline: Option<tokio::time::Instant>,
    attempted_pixel: &mut Option<(i32, i32)>,
) -> Result<ScanOutcome, WorldError> {
    let base = aim.position(io).await;
    let cells = scan_box(base);
    let world: Vec<Vec3> = cells.iter().map(|c| c.0).collect();
    let dzs: Vec<f64> = cells.iter().map(|c| c.1).collect();
    let pr = io
        .project(&world)
        .await
        .map_err(|e| steps.fail(io.now_ms(), "project", e))?;
    let centre = io
        .project(&[base])
        .await
        .map_err(|e| steps.fail(io.now_ms(), "project", e))?
        .points
        .first()
        .copied()
        .flatten()
        .unwrap_or((pr.screen.w / 2.0, pr.screen.h / 2.0));
    let screen = pr.screen;
    let cands = pick_candidates(
        &pr.points,
        &dzs,
        |p| screen.contains(p, CLICK_MARGIN_PX),
        centre,
        SCAN_MAX_READS,
    );
    let mut reads = 0u32;
    let (mut own, mut other, mut empty) = (0u32, 0u32, 0u32);
    let mut hit = None;
    for c in &cands {
        if deadline.is_some_and(|d| tokio::time::Instant::now() >= d) {
            break;
        }
        let (x, y) = (c.px.0.round() as i32, c.px.1.round() as i32);
        *attempted_pixel = Some((x, y));
        io.place_cursor(x, y, true)
            .await
            .map_err(|e| steps.fail(io.now_ms(), "cursor", e))?;
        io.sleep(HOVER_WAIT_MS).await;
        let slots = read_mouse_over(io, steps, (x, y), deadline).await?;
        reads += 1;
        let mo = slot_entity(&slots, mo_slot);
        if mo == entity {
            hit = Some((c.px, c.dz));
            break;
        }
        match mo {
            0 => empty += 1,
            m if m == slot_entity(&slots, player_slot) => own += 1,
            _ => other += 1,
        }
    }
    let mut detail = json!({
        "candidates": cands.len(),
        "reads": reads,
        "nothing": empty,
        "own_avatar": own,
        "other_entity": other,
    });
    if let Some((px, dz)) = hit {
        detail["hit"] = json!({ "at": [px.0.round(), px.1.round()], "dz": dz });
    }
    Ok(ScanOutcome { hit, detail })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_box_is_bounded_to_15_cm_sideways_and_40_cm_up() {
        let base = Vec3::new(100.0, 200.0, 50.0);
        let b = scan_box(base);
        assert_eq!(b.len(), 7 * 7 * SCAN_DZ.len());
        for (p, _) in &b {
            assert!((p.x - base.x).abs() <= 15.0 && (p.y - base.y).abs() <= 15.0);
            assert!((0.0..=40.0).contains(&(p.z - base.z)));
        }
    }

    #[test]
    fn candidates_are_thinned_ordered_and_capped() {
        let pts = [
            Some((100.0, 100.0)),
            Some((101.0, 100.0)), // within 3 px of the first
            Some((110.0, 100.0)),
            None,
            Some((5000.0, 5.0)), // off screen
            Some((104.0, 100.0)),
        ];
        let dzs = [0.0; 6];
        let got = pick_candidates(&pts, &dzs, |p| p.0 < 1000.0, (100.0, 100.0), 10);
        let px: Vec<_> = got.iter().map(|c| c.px.0).collect();
        assert_eq!(px, [100.0, 104.0, 110.0]);
        let capped = pick_candidates(&pts, &dzs, |p| p.0 < 1000.0, (100.0, 100.0), 2);
        assert_eq!(capped.len(), 2);
    }
}
