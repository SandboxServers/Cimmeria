//! Follow state: maintain a distance band to a target entity.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use cimmeria_common::Vector3;

use super::leash::policy::horizontal_distance;
use super::record_decision_outcome;

/// A route in flight is abandoned for a fresh one once the leader stands
/// further than this (horizontally) from where the route ends, or further
/// than its follow band's outer edge when that is wider.
const FOLLOW_REPATH_HORIZONTAL: f32 = 5.0;

/// ...or further than this up or down from it: a storey, not a jump.
const FOLLOW_REPATH_VERTICAL: f32 = 4.0;

/// Whether the route the follower is walking, ending at `route_end`, still
/// leads to the leader now at `target_pos`. A follower used to walk every
/// route to its end: after the player died and respawned 125 u away, Col
/// Marsh walked the whole way to the respawn point while the player ran back
/// past him, then turned round (colo 2026-09-26 02:34:48-02:35:18).
fn route_is_stale(route_end: &Vector3, target_pos: &Vector3, max_d: f32) -> bool {
    horizontal_distance(route_end, target_pos) > FOLLOW_REPATH_HORIZONTAL.max(max_d)
        || (route_end.y - target_pos.y).abs() > FOLLOW_REPATH_VERTICAL
}

/// NPC follow behavior: maintain a distance band to a target entity.
///
/// State machine within Follow:
/// - **No follow_target_id** → drop to Idle.
/// - **Target gone (entity removed)** → clear follow_target_id,
///   drop to Idle.
/// - **Target in band** (`min <= dist <= max`) → no work; stay put.
/// - **Target above max** → pathfind to a point one `min_distance`
///   short of the target so the NPC settles inside the band rather
///   than running all the way up to the target. A route already in
///   flight is kept while it still ends near the target, and replaced
///   once the target has moved away from its end ([`route_is_stale`]).
///   With no usable route, [`super::path_failure::UnroutedMove`] decides:
///   on a meshed world the follower slides across the mesh toward that
///   point or holds, keeping its target and the Follow state; only a
///   meshless world walks the straight line (NA41).
/// - **Target below min** → no work (NPCs don't back away).
pub(super) async fn npc_ai_follow(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::{AiState, MobMovementType};

    let (target_id, npc_pos, min_d, max_d, route_end) = match space_mgr.get_entity(npc_id) {
        Some(e) => (
            e.follow_target_id,
            e.position,
            e.follow_min_distance,
            e.follow_max_distance,
            e.nav_path.back().copied(),
        ),
        None => return,
    };

    // No-target / gone-target drops fire BEFORE the Follow movement type is
    // recorded, so `last_movement_type` never reads Follow for an NPC that
    // leaves Follow this tick. Nothing goes on the wire either way: the
    // client animates from velocity (NA10).
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
            npc_name = space_mgr.entity_label(npc_id),
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
            npc_name = space_mgr.entity_label(npc_id),
            target_id,
            target_name = space_mgr.entity_label(target_id),
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
        space_mgr,
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

    if let Some(end) = route_end {
        if !route_is_stale(&end, &target_pos, max_d) {
            // Movement in flight toward the target.
            record_decision_outcome("follow_band");
            return;
        }
        // The leader has left the route's end behind (respawned, ran the
        // other way): plan again from here rather than walk the old route
        // out. The new route below replaces it.
        tracing::debug!(
            target: "npc_ai",
            event = "follow_repath_stale",
            npc_id,
            npc_name = space_mgr.entity_label(npc_id),
            target_id,
            target_name = space_mgr.entity_label(target_id),
            dist,
            end_to_target = horizontal_distance(&end, &target_pos),
            end_dy = target_pos.y - end.y,
            "NPC AI: follow route no longer leads to the target -- replanning"
        );
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
    let request = super::path_request::request_path(
        space_mgr,
        super::path_request::PathRequest {
            npc_id,
            state: "follow",
            from: npc_pos,
            to: dest,
            target_id: Some(target_id),
            partial_outcome: "follow_partial",
        },
        std::time::Instant::now(),
    );
    let (routing, status) = (request.waypoints, request.status);
    let routed = routing.as_ref().is_some_and(|p| p.len() > 1);
    let (dest, path_len, fallback) = if routed {
        let path = routing.unwrap_or_default();
        let path_len = path.len();
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            super::replace_nav_path_on(npc, path.into_iter().skip(1));
        }
        (dest, path_len, None)
    } else {
        // Unrouted: keep the follower on its OWN height. The raw lerp copied
        // the leader's Y, so a jumping or upstairs leader dragged the escort
        // into the air ("levitating, then he came down" -- 2026-09-18
        // playtest).
        let dest = cimmeria_common::Vector3::new(dest.x, npc_pos.y, dest.z);
        // Resolved before the report: `report_path_failure` takes `&mut`
        // and these take `&`.
        let reason = super::path_failure::PathFailReason::classify(
            space_mgr,
            npc_id,
            status,
            routing.as_deref(),
        );
        // A meshed world slides across the mesh or holds; only a meshless
        // one walks the straight line (NA41). The target and the Follow
        // state are kept either way, and the next tick tries again.
        let unrouted = super::path_failure::UnroutedMove::plan(space_mgr, npc_id, npc_pos, dest);
        let fallback = unrouted.fallback();
        super::path_failure::report_path_failure(
            space_mgr,
            super::path_failure::PathFailure {
                npc_id,
                state: "follow",
                decision_outcome: "follow_no_path",
                from: npc_pos,
                to: dest,
                reason,
                fallback,
                target_id: Some(target_id),
            },
            std::time::Instant::now(),
        );
        unrouted.apply(space_mgr, npc_id);
        (dest, 0, Some(fallback))
    };
    // Out-of-band pathfind queued — the next tick observes
    // nav_empty=false (movement in flight) and records follow_band
    // until back in range.
    record_decision_outcome("follow_band");
    tracing::debug!(
        target: "npc_ai",
        event = "follow_routed",
        npc_id,
        npc_name = space_mgr.entity_label(npc_id),
        target_id,
        target_name = space_mgr.entity_label(target_id),
        dist,
        max_d,
        routed,
        path_len,
        fallback = ?fallback,
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

/// Test-only entry point that drives a single follow tick directly.
/// Production reaches `npc_ai_follow` only through `npc_ai_tick`, which
/// snapshots every NPC in the space — too coarse for a test that wants to
/// assert `nav_path` after each individual step. `npc_ai_follow` itself
/// stays `pub(super)` (npc_ai-internal); this thin wrapper is the one item
/// whose visibility widens, and only under `cfg(test)` or the
/// `test-support` feature, so production callers keep the same narrow
/// surface. Re-exported through `npc_ai` for
/// `cell::content::chain_replay_tests::gc1_escort` in `cimmeria-cell-content`.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub async fn npc_ai_follow_for_test(
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
    /// pathfinding returns `None`" branch. Only a meshless space still
    /// walks the straight line; the meshed case (two disconnected
    /// Cellblock components) is `tests/npc_ai/no_route.rs` (NA41).
    fn make_space_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr
    }

    /// With no navmesh at all there is nothing to slide along, so the
    /// out-of-band branch still pushes exactly one waypoint: `dest`, the
    /// raw straight-line point short of the target.
    ///
    /// The GC1b-0 feasibility pass flagged the same push on a meshed
    /// world (two disconnected components), where it walked the escort
    /// through walls. NA41 confined it to meshless spaces; the meshed
    /// case slides or holds and is guarded in
    /// `tests/npc_ai/no_route.rs`.
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

    /// Colo 2026-09-26 02:34:48: the player died and respawned 125 u away,
    /// and Col Marsh walked his whole 25-waypoint route to the respawn point
    /// while the player ran back toward him. A route whose end the leader
    /// has left behind must be replaced on the next tick, not walked out.
    #[tokio::test]
    async fn a_route_the_leader_has_left_behind_is_replanned() {
        let mut mgr = make_space_mgr();
        mgr.spawn_npc(101, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        mgr.spawn_npc(102, "Agnos", [50.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(101) {
            crate::cell::service::npc_ai::force_ai_state(npc, AiState::Follow);
            npc.follow_target_id = Some(102);
        }
        let (tx, _rx) = mpsc::channel(8);
        npc_ai_follow(101, &tx, &mut mgr).await;
        let end = |m: &SpaceManager| m.get_entity(101).unwrap().nav_path.back().copied();
        assert!((end(&mgr).unwrap().x - 48.0).abs() < 0.01);

        // The leader turns up on the other side of the follower.
        mgr.get_entity_mut(102).unwrap().position = cimmeria_common::Vector3::new(0.0, 0.0, 30.0);
        npc_ai_follow(101, &tx, &mut mgr).await;
        let end = end(&mgr).expect("a new route");
        assert!(
            end.x.abs() < 0.01 && (end.z - 28.0).abs() < 0.01,
            "the route must now end 2 u short of the leader at (0, 0, 30), \
             not at the old (48, 0, 0); got {end:?}"
        );
    }

    /// A leader still near the end of the route in flight keeps that route:
    /// the follower does not replan every tick while it closes the gap.
    #[tokio::test]
    async fn a_route_still_ending_near_the_leader_is_kept() {
        let mut mgr = make_space_mgr();
        mgr.spawn_npc(101, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        mgr.spawn_npc(102, "Agnos", [50.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(101) {
            crate::cell::service::npc_ai::force_ai_state(npc, AiState::Follow);
            npc.follow_target_id = Some(102);
        }
        let (tx, _rx) = mpsc::channel(8);
        npc_ai_follow(101, &tx, &mut mgr).await;
        mgr.get_entity_mut(102).unwrap().position = cimmeria_common::Vector3::new(51.0, 0.0, 3.0);
        npc_ai_follow(101, &tx, &mut mgr).await;
        let end = mgr
            .get_entity(101)
            .unwrap()
            .nav_path
            .back()
            .copied()
            .unwrap();
        assert!((end.x - 48.0).abs() < 0.01 && end.z.abs() < 0.01, "{end:?}");
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
