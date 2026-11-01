//! `actor_growth`: the actor count climbs past half again within a minute.
//! Only samples that carry `actors` count, so the rule stays silent until the
//! DLL reports the count (AX-01 confirms the source).

use serde_json::json;

use super::Fired;
use crate::supervisor::watch_store::WatchSample;

/// The window the count is compared over, and the gap between firings (ms).
const WINDOW_MS: i64 = 60_000;

/// Fire when a count is more than 1.5 times the smallest count seen within
/// the last 60 s, at most once per 60 s.
pub fn actor_growth(samples: &[WatchSample]) -> Vec<Fired> {
    let pts: Vec<(&WatchSample, u32)> = samples
        .iter()
        .filter_map(|s| s.actors.map(|a| (s, a)))
        .collect();
    let mut out = Vec::new();
    let mut lo = 0;
    let mut last_fire: Option<i64> = None;
    for j in 0..pts.len() {
        while lo < j && pts[j].0.wall_ms - pts[lo].0.wall_ms > WINDOW_MS {
            lo += 1;
        }
        let Some(base) = pts[lo..j].iter().map(|p| p.1).min().filter(|&b| b > 0) else {
            continue;
        };
        let now = pts[j].0.wall_ms;
        let grew = f64::from(pts[j].1) > 1.5 * f64::from(base);
        if grew && !matches!(last_fire, Some(t) if now - t <= WINDOW_MS) {
            out.push(Fired {
                name: "actor_growth",
                at_ms: now,
                frame: Some(pts[j].0.f),
                detail: json!({ "from": base, "to": pts[j].1 }),
            });
            last_fire = Some(now);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counted(wall_ms: i64, actors: u32) -> WatchSample {
        WatchSample {
            i: 0,
            f: 0,
            ms: 0,
            wall_ms,
            t: None,
            pos: None,
            rot: None,
            hover: None,
            mode: None,
            buttons: None,
            actors: Some(actors),
        }
    }

    #[test]
    fn actor_growth_over_half_within_a_minute() {
        let grew = [counted(0, 100), counted(30_000, 160)];
        assert_eq!(actor_growth(&grew).len(), 1);
        let small = [counted(0, 100), counted(30_000, 140)];
        assert!(actor_growth(&small).is_empty());
        let slow = [counted(0, 100), counted(90_000, 200)];
        assert!(actor_growth(&slow).is_empty());
    }

    #[test]
    fn samples_without_actors_are_ignored() {
        let mut bare = counted(0, 1);
        bare.actors = None;
        assert!(actor_growth(&[bare, counted(1000, 500)]).is_empty());
    }
}
