use super::*;
use crate::cell::service::npc_ai::{set_ai_state, AiTransitionReason};
use crate::test_support::LogCapture;
use cimmeria_entity::cell_entity::AiState;
use std::collections::VecDeque;

const NPC: u32 = 100;

fn mgr_with_moving_npc() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.spawn_npc(NPC, "Agnos", [15.0, 0.0, 15.0], [0.0; 3])
        .unwrap();
    let npc = mgr.get_entity_mut(NPC).unwrap();
    npc.nav_path = VecDeque::from([Vector3::new(20.0, 0.0, 15.0)]);
    npc.velocity = [6.0, 0.0, 0.0];
    mgr
}

fn stop_rows(logs: &crate::test_support::LogCaptureGuard) -> usize {
    logs.all()
        .into_iter()
        .filter(|c| c.target == "movement.npc" && c.has_field("event", "stop"))
        .count()
}

#[test]
fn stop_clears_the_path_and_zeroes_velocity_and_logs_once() {
    let mut mgr = mgr_with_moving_npc();
    let logs = LogCapture::install();

    stop_npc_movement(&mut mgr, NPC, StopReason::AttackInPlace);

    let npc = mgr.get_entity(NPC).unwrap();
    assert!(npc.nav_path.is_empty());
    assert_eq!(npc.velocity, [0.0; 3]);
    let rows: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|c| c.target == "movement.npc")
        .collect();
    assert_eq!(rows.len(), 1, "{rows:?}");
    for (k, v) in [
        ("event", "stop"),
        ("reason", "attack_in_place"),
        ("npc_id", "100"),
        ("nav_path_len", "1"),
    ] {
        assert!(
            rows[0].has_field(k, v),
            "field {k}={v} missing: {:?}",
            rows[0]
        );
    }

    // A second stop on a standing NPC changes nothing and stays silent, so
    // an attack-in-place hold does not log on every AI tick.
    stop_npc_movement(&mut mgr, NPC, StopReason::AttackInPlace);
    assert_eq!(stop_rows(&logs), 1);
}

#[test]
fn velocity_alone_counts_as_moving() {
    let mut mgr = mgr_with_moving_npc();
    let npc = mgr.get_entity_mut(NPC).unwrap();
    npc.nav_path.clear();
    assert!(
        stop_movement_on(npc),
        "stale velocity with no path is the S1 shape"
    );
    assert!(!stop_movement_on(npc), "second call: already still");
}

#[test]
fn reroute_keeps_velocity_but_an_empty_reroute_is_a_stop() {
    let mut mgr = mgr_with_moving_npc();
    let npc = mgr.get_entity_mut(NPC).unwrap();

    replace_nav_path_on(
        npc,
        [Vector3::new(1.0, 0.0, 1.0), Vector3::new(2.0, 0.0, 2.0)],
    );
    assert_eq!(npc.nav_path.len(), 2);
    assert_eq!(
        npc.velocity,
        [6.0, 0.0, 0.0],
        "a live reroute leaves velocity for the movement tick to steer"
    );

    replace_nav_path_on(npc, std::iter::empty());
    assert!(npc.nav_path.is_empty());
    assert_eq!(npc.velocity, [0.0; 3], "an empty route is a stop");
}

/// Every real state change stops the NPC. The transition row is written
/// first, so its `nav_path_len` records the route that was dropped.
#[test]
fn a_state_change_stops_the_npc_after_logging_the_dropped_route() {
    let mut mgr = mgr_with_moving_npc();
    let logs = LogCapture::install();

    set_ai_state(
        &mut mgr,
        NPC,
        AiState::Fighting,
        AiTransitionReason::ThreatPreempt,
    );

    let npc = mgr.get_entity(NPC).unwrap();
    assert!(npc.nav_path.is_empty());
    assert_eq!(npc.velocity, [0.0; 3]);
    let row = logs
        .all()
        .into_iter()
        .find(|c| c.target == "npc_ai.transition")
        .expect("transition row");
    assert!(row.has_field("nav_path_len", "1"), "{row:?}");
}

/// Re-asserting the current state is not a transition and must not stop
/// the NPC. Otherwise content re-asserting `idle` would halt a walk.
#[test]
fn a_same_state_write_does_not_stop_the_npc() {
    let mut mgr = mgr_with_moving_npc();
    set_ai_state(&mut mgr, NPC, AiState::Idle, AiTransitionReason::Content);
    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.nav_path.len(), 1);
    assert_eq!(npc.velocity, [6.0, 0.0, 0.0]);
}

#[test]
fn snap_moves_through_the_grid_stops_and_sets_facing() {
    let mut mgr = mgr_with_moving_npc();
    let facing = Vector3::new(0.0, 0.5, 0.0);
    // (15, 15) is grid cell (0, 0); (80, 80) is (1, 1).
    snap_npc_to(&mut mgr, NPC, Vector3::new(80.0, 0.0, 80.0), Some(facing));

    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.position, Vector3::new(80.0, 0.0, 80.0));
    assert_eq!(npc.direction, facing);
    assert!(npc.nav_path.is_empty());
    assert_eq!(npc.velocity, [0.0; 3]);
    let space = &mgr.spaces[&mgr.entity_space[&NPC]].space;
    assert!(space
        .get_entities_in_range(&Vector3::new(80.0, 0.0, 80.0), 1.0)
        .iter()
        .any(|e| e.0 == NPC as i32));
    assert!(!space
        .get_entities_in_range(&Vector3::new(15.0, 0.0, 15.0), 1.0)
        .iter()
        .any(|e| e.0 == NPC as i32));
}

/// Guard for S1: the only production writers of `nav_path` are this module
/// and the movement tick (which only pops waypoints). A new raw
/// `nav_path.clear()` would reintroduce the stale-velocity bug, because a
/// cleared path stops the movement tick from ever zeroing velocity again.
///
/// Scans every `.rs` file under `crates/services/src`, up to its first
/// `#[cfg(test)]` line, and skips test-only files. Reverting any converted
/// site fails this test.
#[test]
fn nav_path_is_written_only_through_the_movement_stop_helpers() {
    const PATTERNS: [&str; 4] = [
        // Field accesses only (leading `.`), so a local named `nav_path`
        // (the navmesh file path) does not match.
        ".nav_path.clear()",
        ".nav_path = ",
        ".nav_path.push_back(",
        ".nav_path.extend(",
    ];
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let allowed = ["cell/service/npc_ai/movement_stop/mod.rs"];
    let mut offenders = Vec::new();
    let mut scanned = 0usize;
    let mut stack = vec![src.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let rel = path
                .strip_prefix(&src)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let test_only = rel.contains("/tests/")
                || rel.ends_with("tests.rs")
                || rel.contains("_tests/")
                || rel.starts_with("test_support");
            if test_only || allowed.contains(&rel.as_str()) {
                continue;
            }
            scanned += 1;
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let lines: Vec<&str> = text.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                let code = line.trim_start();
                // An inline `#[cfg(test)] mod tests { ... }` block ends the
                // production code. A `#[cfg(test)]` on a single item (a
                // test-only re-export) does not.
                if code.starts_with("#[cfg(test)]")
                    && lines.get(i + 1).is_some_and(|n| {
                        n.trim_start().starts_with("mod ") && n.trim_end().ends_with('{')
                    })
                {
                    break;
                }
                if code.starts_with("//") {
                    continue;
                }
                if PATTERNS.iter().any(|p| code.contains(p)) {
                    offenders.push(format!("{rel}:{}: {}", i + 1, code));
                }
            }
        }
    }
    assert!(
        scanned > 100,
        "scan found only {scanned} files; wrong root?"
    );
    assert!(
        offenders.is_empty(),
        "raw nav_path writes outside `npc_ai::movement_stop`. Use `stop_movement_on` / \
         `stop_npc_movement` to stop an NPC (it zeroes velocity too) or \
         `replace_nav_path_on` to reroute it:\n{}",
        offenders.join("\n")
    );
}
