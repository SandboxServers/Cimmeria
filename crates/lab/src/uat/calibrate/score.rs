//! D-SV6 scoring: a candidate is judged by hover probes, never a click.

use serde_json::Value;

/// One hover probe of a candidate pose, from `client_hover_probe`.
#[derive(Debug, Clone, PartialEq)]
pub struct Probe {
    /// The aim pixel gave the wanted verdict (`verdict == "ok"`).
    pub ok: bool,
    /// How many of the eight jitter points around the aim pixel agreed.
    pub jitter_hits: u32,
    /// Pixels from the aim pixel to the avatar's projected body.
    pub avatar_px: f64,
    /// Pixels from the aim pixel to the nearest screen edge.
    pub edge_px: f64,
}

/// From a `client_hover_probe` result; None when a field is missing.
pub fn parse_probe(v: &Value) -> Option<Probe> {
    Some(Probe {
        ok: v.get("verdict")?.as_str()? == "ok",
        jitter_hits: u32::try_from(v.get("jitter_hits")?.as_u64()?).ok()?,
        avatar_px: v.get("avatar_px")?.as_f64()?,
        edge_px: v.get("edge_px")?.as_f64()?,
    })
}

/// D-SV6: None unless ok; else jitter_hits*1000 + min(avatar_px,400) + min(edge_px,200)/2, rounded.
pub fn score(p: &Probe) -> Option<u32> {
    if !p.ok {
        return None;
    }
    let s = f64::from(p.jitter_hits) * 1000.0 + p.avatar_px.min(400.0) + p.edge_px.min(200.0) / 2.0;
    Some(s.round() as u32)
}

/// A confirmation re-probe passes when ok and jitter_hits >= CONFIRM_MIN_HITS.
pub const CONFIRM_MIN_HITS: u32 = 6;
/// D-SV6: the winner is re-probed this many times, each from a fresh
/// teleport, before it is proposed.
pub const CONFIRM_RUNS: u32 = 3;

/// Index of the best score; ties go to the lower index.
pub fn best(scores: &[Option<u32>]) -> Option<usize> {
    let mut best: Option<(usize, u32)> = None;
    for (i, s) in scores.iter().enumerate() {
        match (best, *s) {
            (None, Some(s)) => best = Some((i, s)),
            (Some((_, b)), Some(s)) if s > b => best = Some((i, s)),
            _ => {}
        }
    }
    best.map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn probe(ok: bool, jitter_hits: u32, avatar_px: f64, edge_px: f64) -> Probe {
        Probe {
            ok,
            jitter_hits,
            avatar_px,
            edge_px,
        }
    }

    #[test]
    fn a_miss_scores_none() {
        assert_eq!(score(&probe(false, 8, 400.0, 200.0)), None);
    }

    #[test]
    fn jitter_dominates() {
        // 8 hits with the avatar at 0 px beat 7 hits with both bonuses maxed.
        let eight = score(&probe(true, 8, 0.0, 0.0)).unwrap();
        let seven = score(&probe(true, 7, 400.0, 200.0)).unwrap();
        assert!(eight > seven, "{eight} vs {seven}");
    }

    #[test]
    fn ties_go_to_the_first() {
        assert_eq!(best(&[Some(5), None, Some(9), Some(9), Some(2)]), Some(2));
        assert_eq!(best(&[None, None]), None);
        assert_eq!(best(&[]), None);
    }

    #[test]
    fn the_formula_matches_hand_values() {
        // 8000 + 120.4 + min(50, 200)/2 = 8145.4
        assert_eq!(score(&probe(true, 8, 120.4, 50.0)), Some(8145));
        // 6000 + min(512, 400) + min(301, 200)/2 = 6500
        assert_eq!(score(&probe(true, 6, 512.0, 301.0)), Some(6500));
    }

    #[test]
    fn parse_probe_reads_the_result_and_needs_every_field() {
        let v = json!({
            "verdict": "ok", "pixel": [640, 360], "hover": "own", "jitter_hits": 8,
            "jitter": 8, "avatar_px": 120.4, "edge_px": 50.0
        });
        assert_eq!(parse_probe(&v), Some(probe(true, 8, 120.4, 50.0)));
        assert_eq!(parse_probe(&json!({ "verdict": "miss" })), None);
        let no_edge = json!({ "verdict": "ok", "jitter_hits": 8, "avatar_px": 1.0 });
        assert_eq!(parse_probe(&no_edge), None);
    }
}
