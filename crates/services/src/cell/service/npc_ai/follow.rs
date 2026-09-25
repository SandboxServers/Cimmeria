//! Follow state: maintain a distance band to a target entity.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::record_decision_outcome;

/// NPC follow behavior: maintain a distance band to a target entity.
///
/// State machine within Follow:
/// - **No follow_target_id** → drop to Idle.
/// - **Target gone (entity removed)** → clear follow_target_id,
///   drop to Idle.
/// - **Target in band** (`min <= dist <= max`) → no work; stay put.
/// - **Target above max** → pathfind to a point one `min_distance`
///   short of the target so the NPC settles inside the band rather
///   than running all the way up to the target.
/// - **Target below min** → no work (NPCs don't back away).
pub(super) async fn npc_ai_follow(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::{AiState, MobMovementType};

    let (target_id, npc_pos, min_d, max_d, nav_empty) = match space_mgr.get_entity(npc_id) {
        Some(e) => (
            e.follow_target_id,
            e.position,
            e.follow_min_distance,
            e.follow_max_distance,
            e.nav_path.is_empty(),
        ),
        None => return,
    };

    // No-target / gone-target drops fire BEFORE the Follow broadcast
    // so the wire doesn't see a Follow byte for an NPC that's about
    // to leave Follow this same tick.
    let Some(target_id) = target_id else {
        super::set_ai_state(
            space_mgr,
            npc_id,
            AiState::Idle,
            super::AiTransitionReason::FollowNoTarget,
        );
        tracing::debug!(
            target: "npc_ai",
            event = "decision",
            decision_outcome = "follow_dropped_no_target",
            npc_id,
            "NPC AI: follow state with no follow target -- dropping to Idle"
        );
        crate::cell::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
        return;
    };

    let Some(target_pos) = space_mgr.get_entity(target_id).map(|e| e.position) else {
        // Target despawned/disconnected. Clear and drop to Idle.
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.follow_target_id = None;
        }
        super::set_ai_state(
            space_mgr,
            npc_id,
            AiState::Idle,
            super::AiTransitionReason::FollowTargetGone,
        );
        tracing::warn!(
            target: "npc_ai",
            event = "decision",
            decision_outcome = "follow_target_lost",
            reason = "entity_not_found",
            npc_id,
            target_id,
            "NPC AI: follow target not found in space -- follow cleared, escort stands still until a chain re-arms it"
        );
        crate::cell::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
        return;
    };

    crate::cell::abilities::broadcast_movement_type(
        npc_id,
        Some(MobMovementType::Follow),
        tx,
        space_mgr,
    )
    .await;

    let dist = npc_pos.distance_to(&target_pos);
    // Stuck-escort detector (clears itself once the escort is back in band).
    crate::cell::playtest_friction::escort_tick(
        npc_id,
        target_id,
        dist,
        max_d,
        space_mgr.space_has_navmesh(npc_id),
    );
    if dist < min_d {
        // Too close — hold position.
        record_decision_outcome("follow_band");
        return;
    }
    if dist <= max_d {
        // In band — hold position.
        record_decision_outcome("follow_band");
        return;
    }

    if !nav_empty {
        // Movement in flight toward the target.
        record_decision_outcome("follow_band");
        return;
    }

    // Out of band — pathfind to a point one min_distance short of
    // the target along the line between the NPC and the target.
    let dx = target_pos.x - npc_pos.x;
    let dy = target_pos.y - npc_pos.y;
    let dz = target_pos.z - npc_pos.z;
    let mag = (dx * dx + dy * dy + dz * dz).sqrt();
    let stop_distance = min_d.max(0.1);
    let scale = ((mag - stop_distance) / mag).max(0.0);
    let dest = cimmeria_common::Vector3::new(
        npc_pos.x + dx * scale,
        npc_pos.y + dy * scale,
        npc_pos.z + dz * scale,
    );
    // Kept as an `Option` until after classification — see
    // `patrol.rs`: `None` (the pathfinder declined) and
    // `Some(one_waypoint)` (it answered with something unwalkable) are
    // different findings and get different `reason` tokens.
    let routing = space_mgr.find_path(npc_id, &npc_pos, &dest);
    let routed = routing.as_ref().is_some_and(|p| p.len() > 1);
    // Unrouted: keep the follower on its OWN height. The raw lerp copied the
    // leader's Y, so a jumping or upstairs leader dragged the escort into the
    // air ("levitating, then he came down" -- 2026-09-18 playtest).
    let dest = if routed {
        dest
    } else {
        cimmeria_common::Vector3::new(dest.x, npc_pos.y, dest.z)
    };
    if !routed {
        // The unrouted `dest` is a raw 3-axis lerp toward the target,
        // including the target's Y -- an airborne or upstairs target drags
        // the follower through the air and through geometry.
        // Resolved before the call: `report_path_failure` takes `&mut`
        // and this classifier takes `&`.
        let reason =
            super::path_failure::PathFailReason::classify(space_mgr, npc_id, routing.as_deref());
        super::path_failure::report_path_failure(
            space_mgr,
            super::path_failure::PathFailure {
                npc_id,
                state: "follow",
                decision_outcome: "follow_no_path",
                from: npc_pos,
                to: dest,
                reason,
                // The block below clears `nav_path` and pushes `dest`.
                fallback: super::path_failure::PathFallback::DirectWaypoint,
                target_id: Some(target_id),
            },
            std::time::Instant::now(),
        );
    }
    let path = routing.unwrap_or_default();
    let path_len = path.len();
    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
        npc.nav_path.clear();
        if path.len() > 1 {
            for wp in path.into_iter().skip(1) {
                npc.nav_path.push_back(wp);
            }
        } else {
            npc.nav_path.push_back(dest);
        }
    }
    // Out-of-band pathfind queued — the next tick observes
    // nav_empty=false (movement in flight) and records follow_band
    // until back in range.
    record_decision_outcome("follow_band");
    tracing::debug!(
        target: "npc_ai",
        event = "follow_routed",
        npc_id,
        target_id,
        dist,
        max_d,
        routed,
        path_len,
        npc_x = npc_pos.x,
        npc_y = npc_pos.y,
        npc_z = npc_pos.z,
        target_y = target_pos.y,
        dest_x = dest.x,
        dest_y = dest.y,
        dest_z = dest.z,
        "NPC AI: follow → pathfinding toward target"
    );
}

/// Test-only, crate-visible entry point that drives a single follow tick
/// directly. Production reaches `npc_ai_follow` only through
/// `npc_ai_tick`, which is `pub(in crate::cell::service)` and snapshots
/// every NPC in the space — too coarse for a test that wants to assert
/// `nav_path` after each individual step. `npc_ai_follow` itself stays
/// `pub(super)` (npc_ai-internal); this thin wrapper is the one item
/// whose visibility widens, and only under `cfg(test)`, so production
/// callers keep the same narrow surface. Re-exported up through
/// `npc_ai::mod` and `cell::service::mod` for
/// `cell::content::chain_replay_tests::gc1_escort`, which lives outside
/// `cell::service` entirely.
#[cfg(test)]
pub(crate) async fn npc_ai_follow_for_test(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    npc_ai_follow(npc_id, tx, space_mgr).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::cell_entity::AiState;

    /// Non-instanced "Agnos" fixture with no navmesh loaded — matches
    /// `SpaceManager::find_path`'s documented "no navmesh loaded ->
    /// pathfinding returns `None`" branch. This is the same failure
    /// shape as two disconnected navmesh components (the Castle
    /// Cellblock Preparation-room / topside split the GC1 feasibility
    /// pass found): `find_path` returning `None` either way.
    fn make_space_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr
    }

    /// Pins the CURRENT silent-failure shape in the out-of-band
    /// pathfind branch, flagged by the GC1b-0 feasibility pass as a
    /// trap worth documenting (fixing it is out of scope here).
    ///
    /// `space_mgr.find_path(...).unwrap_or_default()` turns a `None`
    /// (no navmesh loaded, or the navmesh has no route between two
    /// disconnected components) into an empty `Vec` — not an error.
    /// `path.len() > 1` is then false, so the code falls into the
    /// `else` arm and pushes exactly one waypoint: `dest`, the raw
    /// straight-line point short of the target. The NPC then walks
    /// directly toward that point on the next movement tick with zero
    /// awareness of walls or navmesh containment — a "cuts straight
    /// through geometry" bug that produces no error, no log at
    /// warn-or-above, and no visible signal beyond the NPC clipping
    /// through a wall.
    #[tokio::test]
    async fn out_of_band_follow_with_no_navmesh_falls_back_to_straight_line_waypoint() {
        let mut mgr = make_space_mgr();
        mgr.spawn_npc(101, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        mgr.spawn_npc(102, "Agnos", [50.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(101) {
            crate::cell::service::npc_ai::force_ai_state(npc, AiState::Follow);
            npc.follow_target_id = Some(102);
            // follow_min/max_distance default to 2.0/5.0 (construction.rs);
            // the target is 50 units away, well outside the band, so the
            // out-of-band pathfind branch runs.
        }

        let (tx, _rx) = mpsc::channel(8);
        npc_ai_follow(101, &tx, &mut mgr).await;

        let npc = mgr.get_entity(101).unwrap();
        assert_eq!(
            npc.nav_path.len(),
            1,
            "no navmesh loaded -> find_path returns None -> the fallback \
             pushes exactly one waypoint (the raw destination), not a \
             navmesh-routed multi-waypoint path"
        );
        // stop_distance = follow_min_distance.max(0.1) = 2.0; dest sits
        // 2.0 short of the target along the straight line from npc to
        // target: 50.0 - 2.0 = 48.0 on the x axis.
        let dest = npc.nav_path.front().copied().expect("waypoint pushed");
        assert!(
            (dest.x - 48.0).abs() < 0.01 && dest.y.abs() < 0.01 && dest.z.abs() < 0.01,
            "fallback waypoint must be the unrouted straight-line point, \
             got {dest:?}"
        );
    }

    /// Regression guard for the 2026-09-18 colo playtest: the straight-line
    /// fallback above fired on 54 of 54 Castle follow legs with no log at
    /// all. It must now say so, and say WHY (no mesh vs no route).
    #[tokio::test]
    async fn follow_straight_line_fallback_warns_with_reason() {
        let mut mgr = make_space_mgr();
        mgr.spawn_npc(101, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        mgr.spawn_npc(102, "Agnos", [50.0, 3.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(101) {
            crate::cell::service::npc_ai::force_ai_state(npc, AiState::Follow);
            npc.follow_target_id = Some(102);
        }
        let logs = crate::test_support::LogCapture::install();
        let (tx, _rx) = mpsc::channel(8);
        npc_ai_follow(101, &tx, &mut mgr).await;

        let ev = logs
            .find_event(
                tracing::Level::WARN,
                "follow got no usable navmesh route",
                "no_mesh",
            )
            .expect("unrouted follow leg must emit a follow_no_path warn");
        assert!(ev.has_field("decision_outcome", "follow_no_path"));
        assert!(ev.has_field("npc_id", "101"));
        assert!(
            ev.has_field("fallback", "direct_waypoint"),
            "follow clears nav_path and pushes the raw dest, so the row \
             must promise the straight-line fallback it actually takes"
        );
        assert!(
            ev.fields.contains_key("dy"),
            "dy is the air-climb signature and must be on the event"
        );
    }

    /// A follow target that no longer resolves used to clear the escort's
    /// target and park it in Idle without a trace (the Marsh symptom).
    #[tokio::test]
    async fn follow_target_lost_warns_and_clears() {
        let mut mgr = make_space_mgr();
        mgr.spawn_npc(101, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(101) {
            crate::cell::service::npc_ai::force_ai_state(npc, AiState::Follow);
            npc.follow_target_id = Some(999);
        }
        let logs = crate::test_support::LogCapture::install();
        let (tx, _rx) = mpsc::channel(8);
        npc_ai_follow(101, &tx, &mut mgr).await;

        assert!(logs
            .find_event(
                tracing::Level::WARN,
                "follow target not found",
                "entity_not_found"
            )
            .is_some());
        let npc = mgr.get_entity(101).unwrap();
        assert_eq!(npc.follow_target_id, None);
        assert_eq!(npc.ai_state(), AiState::Idle);
    }

    /// The unrouted fallback must keep the follower on its OWN height. It used
    /// to lerp toward the leader's Y, so a jumping or upstairs leader pulled
    /// the escort into the air.
    #[tokio::test]
    async fn unrouted_follow_keeps_the_followers_own_height() {
        let mut mgr = make_space_mgr();
        mgr.spawn_npc(101, "Agnos", [0.0, 7.0, 0.0], [0.0; 3])
            .unwrap();
        mgr.spawn_npc(102, "Agnos", [50.0, 11.5, 0.0], [0.0; 3])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(101) {
            crate::cell::service::npc_ai::force_ai_state(npc, AiState::Follow);
            npc.follow_target_id = Some(102);
        }
        let (tx, _rx) = mpsc::channel(8);
        npc_ai_follow(101, &tx, &mut mgr).await;

        let dest = mgr
            .get_entity(101)
            .unwrap()
            .nav_path
            .front()
            .copied()
            .expect("fallback waypoint");
        assert!(
            (dest.y - 7.0).abs() < 1e-4,
            "fallback waypoint must stay at the follower's Y (7.0), not drift \
             toward the leader's 11.5; got {}",
            dest.y
        );
        assert!(dest.x > 40.0, "still heads toward the leader in XZ");
    }
}
