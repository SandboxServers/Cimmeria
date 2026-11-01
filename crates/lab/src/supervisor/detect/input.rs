//! `stuck_input`: a key held down for 3 s while the pawn does not move, so the
//! hold is probably wedged (C12, the HoldBook start from C5).

use serde_json::json;

use super::Fired;
use crate::supervisor::watch_store::WatchSample;
use crate::supervisor::world::geometry::UU_PER_METRE;

/// A hold must be this old, and the samples must span this long, to count.
const HOLD_MS: i64 = 3000;
/// The pawn must end within this many UE3 units of where it started.
const STILL_UU: f64 = 20.0;

/// For each held key older than 3 s: the samples since the hold began, whose
/// first and last positions are less than 20 UU apart (horizontally) over at
/// least 3 s, fire once. `holds` are key name and hold start, daemon wall ms.
pub fn stuck_input(samples: &[WatchSample], holds: &[(String, i64)], now_ms: i64) -> Vec<Fired> {
    holds
        .iter()
        .filter_map(|(key, since)| {
            if now_ms - *since < HOLD_MS {
                return None;
            }
            let during: Vec<(&WatchSample, [f32; 3])> = samples
                .iter()
                .filter(|s| s.wall_ms >= *since)
                .filter_map(|s| s.pos.map(|p| (s, p)))
                .collect();
            let (first, last) = (during.first()?, during.last()?);
            let span = last.0.wall_ms - first.0.wall_ms;
            let dx = f64::from(last.1[0] - first.1[0]);
            let dy = f64::from(last.1[1] - first.1[1]);
            let moved_uu = dx.hypot(dy);
            (span >= HOLD_MS && moved_uu < STILL_UU).then(|| Fired {
                name: "stuck_input",
                at_ms: last.0.wall_ms,
                frame: Some(last.0.f),
                detail: json!({
                    "key": key,
                    "held_ms": now_ms - *since,
                    "moved_m": moved_uu / UU_PER_METRE,
                }),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sample at `wall_ms` on a straight walk along x that covers `total`
    /// UU over the 4 s from 0 to 4000 ms.
    fn walk(wall_ms: i64, total: f32) -> WatchSample {
        WatchSample {
            i: 0,
            f: wall_ms as u64 / 16,
            ms: wall_ms as u64,
            wall_ms,
            t: None,
            pos: Some([total * wall_ms as f32 / 4000.0, 0.0, 0.0]),
            rot: None,
            hover: None,
            mode: None,
            buttons: None,
            actors: None,
        }
    }

    fn held(since: i64) -> Vec<(String, i64)> {
        vec![("W".to_string(), since)]
    }

    #[test]
    fn stuck_input_needs_three_seconds_of_hold() {
        let still: Vec<_> = (0..=4).map(|k| walk(k * 1000, 10.0)).collect();
        assert_eq!(stuck_input(&still, &held(0), 4000).len(), 1);
        let moving: Vec<_> = (0..=4).map(|k| walk(k * 1000, 500.0)).collect();
        assert!(stuck_input(&moving, &held(0), 4000).is_empty());
        assert!(stuck_input(&still, &held(2000), 4000).is_empty());
    }
}
