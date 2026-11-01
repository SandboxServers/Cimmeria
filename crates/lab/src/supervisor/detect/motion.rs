//! Pawn motion: `teleport`, `long_fall`, `out_of_world` and `nan`. Each reads
//! the `pos` of the samples and skips the ones without one. Distances are
//! client UE3 units; `UU_PER_METRE` converts them.

use serde_json::json;

use super::Fired;
use crate::supervisor::watch_store::WatchSample;
use crate::supervisor::world::geometry::UU_PER_METRE;

/// Teleport: a jump of more than this many metres...
const TELEPORT_METRES: f64 = 30.0;
/// ...within this many ms.
const TELEPORT_MS: i64 = 500;
/// A fall (z strictly decreasing) longer than this many ms.
const LONG_FALL_MS: i64 = 3000;
/// Below this z the pawn is out of the world. D-AX6, provisional.
pub const OUT_OF_WORLD_Z: f32 = -100_000.0;

/// Distance in metres between two positions, horizontal and vertical together.
fn metres(a: [f32; 3], b: [f32; 3]) -> f64 {
    let d: f64 = (0..3)
        .map(|k| (f64::from(a[k]) - f64::from(b[k])).powi(2))
        .sum();
    d.sqrt() / UU_PER_METRE
}

/// The samples that carry a position, oldest first.
fn with_pos(samples: &[WatchSample]) -> Vec<(&WatchSample, [f32; 3])> {
    samples
        .iter()
        .filter_map(|s| s.pos.map(|p| (s, p)))
        .collect()
}

/// `teleport`: the pawn moved more than 30 m within 500 ms. Each sample is
/// compared with the earliest sample at most 500 ms before it; a run of
/// consecutive over-limit samples is one jump, reported once.
pub fn teleport(samples: &[WatchSample]) -> Vec<Fired> {
    let pts = with_pos(samples);
    let mut out = Vec::new();
    let mut lo = 0;
    let mut last_over: Option<usize> = None;
    for j in 0..pts.len() {
        while lo < j && pts[j].0.wall_ms - pts[lo].0.wall_ms > TELEPORT_MS {
            lo += 1;
        }
        if lo == j {
            continue;
        }
        let m = metres(pts[lo].1, pts[j].1);
        if m > TELEPORT_METRES {
            if !matches!(last_over, Some(l) if l + 1 == j) {
                out.push(Fired {
                    name: "teleport",
                    at_ms: pts[j].0.wall_ms,
                    frame: Some(pts[j].0.f),
                    detail: json!({
                        "from": pts[lo].1,
                        "to": pts[j].1,
                        "metres": m,
                    }),
                });
            }
            last_over = Some(j);
        }
    }
    out
}

/// `long_fall`: z strictly decreasing for more than 3 s. Fired once per run,
/// at the run's last sample.
pub fn long_fall(samples: &[WatchSample]) -> Vec<Fired> {
    let pts = with_pos(samples);
    let mut out = Vec::new();
    let mut start = 0;
    for k in 1..=pts.len() {
        let run_ends = k == pts.len() || pts[k].1[2] >= pts[k - 1].1[2];
        if !run_ends {
            continue;
        }
        let e = k - 1;
        let secs_ms = pts[e].0.wall_ms - pts[start].0.wall_ms;
        if e > start && secs_ms > LONG_FALL_MS {
            out.push(Fired {
                name: "long_fall",
                at_ms: pts[e].0.wall_ms,
                frame: Some(pts[e].0.f),
                detail: json!({
                    "seconds": secs_ms as f64 / 1000.0,
                    "drop_uu": pts[start].1[2] - pts[e].1[2],
                }),
            });
        }
        start = k;
    }
    out
}

/// `out_of_world`: the first sample below [`OUT_OF_WORLD_Z`] in each run of
/// such samples.
pub fn out_of_world(samples: &[WatchSample]) -> Vec<Fired> {
    let mut out = Vec::new();
    let mut was_below = false;
    for (s, p) in with_pos(samples) {
        let below = p[2] < OUT_OF_WORLD_Z;
        if below && !was_below {
            out.push(Fired {
                name: "out_of_world",
                at_ms: s.wall_ms,
                frame: Some(s.f),
                detail: json!({ "z": p[2] }),
            });
        }
        was_below = below;
    }
    out
}

/// `nan`: the first sample whose position has a NaN or infinite component.
/// Rotations are integers in the DLL, so `rot` cannot be NaN and is not read.
pub fn nan(samples: &[WatchSample]) -> Vec<Fired> {
    with_pos(samples)
        .into_iter()
        .find(|(_, p)| p.iter().any(|c| !c.is_finite()))
        .map(|(s, p)| Fired {
            name: "nan",
            at_ms: s.wall_ms,
            frame: Some(s.f),
            detail: json!({ "pos": p.map(|c| c.to_string()) }),
        })
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(wall_ms: i64, f: u64, pos: [f32; 3]) -> WatchSample {
        WatchSample {
            i: f,
            f,
            ms: f * 16,
            wall_ms,
            t: None,
            pos: Some(pos),
            rot: None,
            hover: None,
            mode: None,
            buttons: None,
            actors: None,
        }
    }

    #[test]
    fn teleport_over_30m_in_500ms() {
        let fast = [at(0, 0, [0.0, 0.0, 0.0]), at(200, 1, [3100.0, 0.0, 0.0])];
        assert_eq!(teleport(&fast).len(), 1);
        let short = [at(0, 0, [0.0, 0.0, 0.0]), at(200, 1, [2900.0, 0.0, 0.0])];
        assert!(teleport(&short).is_empty());
        let slow = [at(0, 0, [0.0, 0.0, 0.0]), at(900, 1, [3100.0, 0.0, 0.0])];
        assert!(teleport(&slow).is_empty());
    }

    #[test]
    fn teleport_fires_once_per_jump() {
        let jump = [
            at(0, 0, [0.0, 0.0, 0.0]),
            at(100, 1, [3100.0, 0.0, 0.0]),
            at(200, 2, [3100.0, 0.0, 0.0]),
        ];
        assert_eq!(teleport(&jump).len(), 1);
    }

    #[test]
    fn long_fall_after_three_seconds() {
        let falling: Vec<_> = (0..=7)
            .map(|k| at(k * 500, k as u64, [0.0, 0.0, 1000.0 - 100.0 * k as f32]))
            .collect();
        assert_eq!(long_fall(&falling).len(), 1);
        let short: Vec<_> = (0..=5)
            .map(|k| at(k * 500, k as u64, [0.0, 0.0, 1000.0 - 100.0 * k as f32]))
            .collect();
        assert!(long_fall(&short).is_empty());
    }

    #[test]
    fn out_of_world_threshold() {
        let below = [at(0, 0, [0.0, 0.0, -100_001.0])];
        assert_eq!(out_of_world(&below).len(), 1);
        let above = [at(0, 0, [0.0, 0.0, -99_999.0])];
        assert!(out_of_world(&above).is_empty());
    }

    #[test]
    fn nan_position() {
        let bad = [
            at(0, 0, [f32::NAN, 0.0, 0.0]),
            at(16, 1, [f32::NAN, 0.0, 0.0]),
        ];
        let fired = nan(&bad);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].at_ms, 0);
    }
}
