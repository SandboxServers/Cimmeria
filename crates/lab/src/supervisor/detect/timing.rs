//! `hitch`: the client stalled. Two rules, the watch one (per frame) and the
//! heartbeat one (no watch running).

use serde_json::json;

use super::Fired;
use crate::supervisor::watch_store::WatchSample;
use crate::timeline::client_events::HeartbeatSample;

/// A gap longer than this many ms per frame is a hitch (C12).
const HITCH_MS_PER_FRAME: u64 = 250;

/// Watch mode: consecutive samples whose gap exceeds 250 ms per frame elapsed
/// (`max(1, df)` frames, so a period-1 watch allows 250 ms a sample).
pub fn hitch_watch(samples: &[WatchSample]) -> Vec<Fired> {
    samples
        .windows(2)
        .filter_map(|w| {
            let gap = w[1].ms.saturating_sub(w[0].ms);
            let frames = w[1].f.saturating_sub(w[0].f).max(1);
            (gap > HITCH_MS_PER_FRAME * frames).then(|| Fired {
                name: "hitch",
                at_ms: w[1].wall_ms,
                frame: Some(w[1].f),
                detail: json!({ "gap_ms": gap, "frames": frames }),
            })
        })
        .collect()
}

/// Heartbeat mode: two observations at least 250 ms apart that show the same
/// Tick-drain counter, so the client's main thread did not advance.
pub fn hitch_heartbeat(beats: &[HeartbeatSample]) -> Vec<Fired> {
    beats
        .windows(2)
        .filter_map(|w| {
            let gap = w[1].observed_ms - w[0].observed_ms;
            (w[1].tick_count == w[0].tick_count && gap >= HITCH_MS_PER_FRAME as i64).then(|| {
                Fired {
                    name: "hitch",
                    at_ms: w[1].observed_ms,
                    frame: Some(w[1].tick_count),
                    detail: json!({ "gap_ms": gap, "tick": w[1].tick_count }),
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(i: u64, f: u64, ms: u64) -> WatchSample {
        WatchSample {
            i,
            f,
            ms,
            wall_ms: 1000 + ms as i64,
            t: None,
            pos: None,
            rot: None,
            hover: None,
            mode: None,
            buttons: None,
            actors: None,
        }
    }

    fn hb(tick_count: u64, observed_ms: i64) -> HeartbeatSample {
        HeartbeatSample {
            tick_count,
            observed_ms,
        }
    }

    #[test]
    fn hitch_watch_period_one() {
        let v = [s(0, 0, 0), s(1, 1, 16), s(2, 2, 400)];
        let fired = hitch_watch(&v);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].at_ms, 1400);
        assert_eq!(fired[0].frame, Some(2));
        assert_eq!(fired[0].detail["gap_ms"], 384);
    }

    #[test]
    fn hitch_watch_scales_with_frames() {
        assert!(hitch_watch(&[s(0, 0, 0), s(1, 2, 400)]).is_empty());
        assert_eq!(hitch_watch(&[s(0, 0, 0), s(1, 1, 400)]).len(), 1);
    }

    #[test]
    fn hitch_heartbeat_same_tick() {
        assert_eq!(hitch_heartbeat(&[hb(5, 0), hb(5, 300)]).len(), 1);
        assert!(hitch_heartbeat(&[hb(5, 0), hb(6, 300)]).is_empty());
    }
}
