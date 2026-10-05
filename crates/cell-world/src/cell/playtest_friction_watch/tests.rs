use super::*;
use crate::cell::space_manager::RegionData;

fn secs(t0: Instant, s: u64) -> Instant {
    t0 + Duration::from_secs(s)
}

/// Throne Room shape: server-side containment, no client hint.
#[test]
fn region_dwell_fires_once_and_a_hint_suppresses_it() {
    let t0 = Instant::now();
    let inside = [(7u32, "Castle.ThroneRoom".to_string())];
    let mut w = PlayerWatch::default();
    assert!(w.evaluate(t0, [0.0; 3], &[], &inside).is_empty());
    let fired = w.evaluate(secs(t0, 6), [0.0; 3], &[], &inside);
    assert_eq!(
        fired,
        vec![Friction::RegionDwellNoHint {
            region_id: 7,
            region_tag: "Castle.ThroneRoom".into(),
            dwell_secs: 6
        }]
    );
    assert!(w.evaluate(secs(t0, 60), [0.0; 3], &[], &inside).is_empty());

    // A hint shortly BEFORE the server notices the entry still counts.
    let mut hinted = PlayerWatch::default();
    hinted.note_region_hint(7, t0);
    assert!(hinted
        .evaluate(secs(t0, 1), [0.0; 3], &[], &inside)
        .is_empty());
    assert!(hinted
        .evaluate(secs(t0, 30), [0.0; 3], &[], &inside)
        .is_empty());

    // Leaving and re-entering starts a fresh episode.
    assert!(w.evaluate(secs(t0, 70), [0.0; 3], &[], &[]).is_empty());
    assert!(w.evaluate(secs(t0, 72), [0.0; 3], &[], &inside).is_empty());
    assert_eq!(w.evaluate(secs(t0, 80), [0.0; 3], &[], &inside).len(), 1);
}

#[test]
fn step_stalled_fires_once_per_step_and_resets_on_advance() {
    let t0 = Instant::now();
    let mut w = PlayerWatch::default();
    assert!(w.evaluate(t0, [0.0; 3], &[(706, 2411)], &[]).is_empty());
    assert!(w
        .evaluate(secs(t0, 299), [0.0; 3], &[(706, 2411)], &[])
        .is_empty());
    let fired = w.evaluate(secs(t0, 300), [0.0; 3], &[(706, 2411)], &[]);
    assert_eq!(
        fired,
        vec![Friction::StepStalled {
            mission_id: 706,
            step_id: 2411,
            age_secs: 300
        }]
    );
    assert!(w
        .evaluate(secs(t0, 900), [0.0; 3], &[(706, 2411)], &[])
        .is_empty());
    // Advancing restarts the clock for the new step.
    assert!(w
        .evaluate(secs(t0, 901), [0.0; 3], &[(706, 2412)], &[])
        .is_empty());
    assert!(w
        .evaluate(secs(t0, 1100), [0.0; 3], &[(706, 2412)], &[])
        .is_empty());
    assert_eq!(
        w.evaluate(secs(t0, 1201), [0.0; 3], &[(706, 2412)], &[])
            .len(),
        1
    );
}

/// The 2026-09-18 respawn bug: hints before death, none after, player
/// still travelling.
#[test]
fn death_then_silence_needs_prior_hints_time_and_travel() {
    let t0 = Instant::now();
    let mut w = PlayerWatch::default();
    for r in 0..3 {
        w.note_region_hint(r, t0);
    }
    w.note_respawn(secs(t0, 10));
    // Time alone is not enough -- the player has not moved.
    assert!(w.evaluate(secs(t0, 200), [0.0; 3], &[], &[]).is_empty());
    // Walk 120 units in sub-teleport hops.
    let mut x = 0.0;
    let mut t = 200;
    let mut fired = Vec::new();
    while x < 120.0 {
        x += 20.0;
        t += 2;
        fired.extend(w.evaluate(secs(t0, t), [x, 0.0, 0.0], &[], &[]));
    }
    assert_eq!(fired.len(), 1, "fires exactly once: {fired:?}");
    assert!(matches!(
        fired[0],
        Friction::DeathThenSilence {
            hints_before_respawn: 3,
            ..
        }
    ));

    // A hint after respawn means the client is fine.
    let mut ok = PlayerWatch::default();
    for r in 0..3 {
        ok.note_region_hint(r, t0);
    }
    ok.note_respawn(secs(t0, 10));
    ok.note_region_hint(9, secs(t0, 20));
    ok.evaluate(secs(t0, 21), [0.0; 3], &[], &[]);
    assert!(ok
        .evaluate(secs(t0, 400), [40.0, 0.0, 0.0], &[], &[])
        .is_empty());
}

/// Dialog 2516 replaced by 5859 after 0.6 s.
#[test]
fn dialog_displaced_only_for_a_different_dialog_inside_the_window() {
    let t0 = Instant::now();
    let mut w = PlayerWatch::default();
    assert_eq!(w.note_dialog(2516, t0), None);
    assert_eq!(
        w.note_dialog(5859, t0 + Duration::from_millis(600)),
        Some(Friction::DialogDisplaced {
            dialog_id: 5859,
            replaced_dialog_id: 2516,
            ms_since_previous: 600
        })
    );
    assert_eq!(w.note_dialog(5859, t0 + Duration::from_millis(700)), None);
    assert_eq!(w.note_dialog(1, secs(t0, 10)), None);
}

/// Seeded Castle_Cellblock regions (point_set_points 2034/2037/2043,
/// height 0, flags 1) as the loader hands them over.
fn cellblock_region(runtime_id: u32, tag: &str, points: [[f32; 3]; 4]) -> RegionData {
    RegionData {
        runtime_id,
        db_set_id: 0,
        tag: tag.to_string(),
        world_name: "Castle_CellBlock".to_string(),
        height: 0.0,
        radius: 0.0,
        flags: crate::cell::space_manager::REGION_FLAG_CLIENT_HINTED,
        points: points.to_vec(),
    }
}

/// 2026-09-26 colo: entity 2 at (-129.338, 39.552, -96.1) -- the room up
/// the ramp west of the Mess Hall -- drew `client_region_hint_missing` for
/// Region6 and Region12, whose ceilings are 29.96 and 31.90. The client
/// stays silent there by design, so the watcher must not expect a hint.
/// Down on their floor (y 24.67) it still must.
#[test]
fn dwell_candidates_follow_the_clients_ceiling_not_just_xz() {
    let r6 = cellblock_region(
        19,
        "Castle_Cellblock.Region6",
        [
            [-115.08, 24.64, -146.42],
            [-115.08, 24.64, -75.2],
            [-148.85, 24.64, -75.2],
            [-148.85, 29.96, -146.42],
        ],
    );
    let r12 = cellblock_region(
        25,
        "Castle_Cellblock.Region12",
        [
            [-148.81, 24.64, -93.8],
            [-148.81, 24.64, -146.72],
            [-115.18, 24.64, -146.72],
            [-115.18, 31.9, -93.8],
        ],
    );
    let mut unhinted = r12.clone();
    unhinted.runtime_id = 99;
    unhinted.flags = 0;
    let regions = [&r6, &r12, &unhinted];

    assert!(
        regions_client_should_hint(&regions, [-129.338, 39.552, -96.1]).is_empty(),
        "upper floor: above both ceilings, the client sends nothing"
    );
    let below: Vec<u32> = regions_client_should_hint(&regions, [-129.338, 24.67, -96.1])
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    assert_eq!(
        below,
        vec![19, 25],
        "on their floor, both are expected; the unregistered one never is"
    );
}

/// Colo 2026-09-26: the player closed 2299, whose `dialog_choice` chain
/// (1019) displayed 2298 in the same millisecond. That is the player
/// moving on, not a displacement. Same shape: 4001 -> 4000.
#[test]
fn dialog_answered_by_the_player_is_not_displaced_by_its_follow_up() {
    let t0 = Instant::now();
    let mut w = PlayerWatch::default();
    assert_eq!(w.note_dialog(2299, t0), None);
    w.note_dialog_answered(2299);
    assert_eq!(w.note_dialog(2298, t0 + Duration::from_millis(1421)), None);

    // The answered flag covers exactly one follow-up: 2298 itself was
    // not answered, so a third dialog on its heels is still flagged.
    assert!(w
        .note_dialog(4000, t0 + Duration::from_millis(1600))
        .is_some());
}

/// 2516 -> 5859: the server displays 5859 first, and 2516's close only
/// arrives afterwards (the client's eviction). A late answer for the
/// displaced dialog must not suppress anything.
#[test]
fn late_answer_for_an_evicted_dialog_does_not_mask_the_displacement() {
    let t0 = Instant::now();
    let mut w = PlayerWatch::default();
    assert_eq!(w.note_dialog(2516, t0), None);
    assert!(w
        .note_dialog(5859, t0 + Duration::from_millis(500))
        .is_some());
    w.note_dialog_answered(2516);
    assert!(!w.last_dialog_answered);
    assert!(w
        .note_dialog(2518, t0 + Duration::from_millis(900))
        .is_some());
}

#[test]
fn point_in_polygon_handles_rotated_boxes() {
    // A diamond: its AABB contains (0.9, 0.9) but the polygon does not.
    let diamond = [
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, -1.0],
        [-1.0, 0.0, 0.0],
    ];
    assert!(region_contains_xz(&diamond, 0.0, 0.0));
    assert!(region_contains_xz(&diamond, 0.4, 0.4));
    assert!(!region_contains_xz(&diamond, 0.9, 0.9));
    assert!(!region_contains_xz(&diamond[..2], 0.0, 0.0));
}
