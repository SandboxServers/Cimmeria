//! AI-tick detectors (2 s cadence).
//!
//! - [`before_tick`] runs over **every** NPC, admitted or not: the
//!   `npc_ai_idle_unticked{world}` gauge (audit A1/T4: an Idle NPC with no
//!   aggression, patrol or wander is never handed to a handler and leaves no
//!   trace).
//! - [`after_handler`] runs for each ticked NPC after its handler:
//!   `npc_ai event=npc_off_mesh` and `npc_ai event=stuck`.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use cimmeria_entity::cell_entity::AiState;

use super::NpcIdent;
use crate::cell::space_manager::SpaceManager;

const IDLE_SUMMARY_INTERVAL: Duration = Duration::from_secs(30);
const OFF_MESH_WARN_INTERVAL: Duration = Duration::from_secs(30);
const STUCK_WARN_INTERVAL: Duration = Duration::from_secs(15);
/// Chasing ticks without progress before `stuck` fires.
pub(in crate::cell) const STUCK_TICKS: usize = 3;
/// `npc_to_target` must shrink by at least this much over the window.
const STUCK_MIN_PROGRESS: f32 = 0.5;
/// The fight outcomes that mean "target out of range or occluded, trying to
/// close": the out-of-range branch of `npc_ai_fight`.
const CHASING_OUTCOMES: [&str; 4] = ["chase", "hold_no_repath", "repath_degenerate", "no_path"];

/// Per-AI-tick pass over every NPC. `admitted` is the set the dispatcher is
/// about to hand to a handler.
pub(in crate::cell) fn before_tick(
    space_mgr: &mut SpaceManager,
    admitted: &HashSet<u32>,
    now: Instant,
) {
    let mut unticked: HashMap<String, i64> = HashMap::new();
    for npc_id in space_mgr.all_npc_entity_ids() {
        let Some(e) = space_mgr.get_entity(npc_id) else {
            continue;
        };
        let state = e.ai_state();
        if state == AiState::Dead || state == AiState::Spawning {
            continue;
        }
        if state == AiState::Idle && !admitted.contains(&npc_id) {
            let world = super::super::world_label(space_mgr, npc_id);
            *unticked.entry(world).or_insert(0) += 1;
        }
    }
    report_idle_unticked(space_mgr, unticked, now);
}

fn report_idle_unticked(
    space_mgr: &mut SpaceManager,
    unticked: HashMap<String, i64>,
    now: Instant,
) {
    // Worlds that had unticked NPCs last time and have none now must be
    // driven back to zero too.
    let mut worlds: Vec<String> = space_mgr
        .npc_detectors
        .idle_unticked_reported
        .keys()
        .cloned()
        .collect();
    for w in unticked.keys() {
        if !worlds.contains(w) {
            worlds.push(w.clone());
        }
    }
    for world in worlds {
        let count = unticked.get(&world).copied().unwrap_or(0);
        let prev = space_mgr
            .npc_detectors
            .idle_unticked_reported
            .insert(world.clone(), count)
            .unwrap_or(0);
        if count != prev {
            cimmeria_observability::gauge_add!(
                "npc_ai_idle_unticked",
                count - prev,
                "world" => world.clone(),
            );
        }
        if count == 0 {
            continue;
        }
        // Per world (the gauge's label), in its own map: an instanced
        // world can have several spaces, and a space id is not an NPC id.
        if let Some(suppressed) =
            space_mgr
                .npc_detectors
                .admit_world_summary(&world, now, IDLE_SUMMARY_INTERVAL)
        {
            tracing::debug!(
                target: "npc_ai.idle",
                event = "unticked",
                world = %world,
                idle_unticked = count,
                suppressed,
                "npc_ai: Idle NPCs the AI tick never visits (no aggression, patrol \
                 or wander) -- they will not notice a player on their own"
            );
        }
    }
}

/// Per ticked NPC, after its handler ran.
pub(in crate::cell) fn after_handler(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    state_before: AiState,
    outcome: &str,
    now: Instant,
) {
    check_off_mesh(space_mgr, npc_id, now);
    check_stuck(space_mgr, npc_id, state_before, outcome, now);
}

fn check_off_mesh(space_mgr: &mut SpaceManager, npc_id: u32, now: Instant) {
    let Some(pos) = space_mgr.get_entity(npc_id).map(|e| e.position) else {
        return;
    };
    // `None` = no navmesh in this space: nothing to be off.
    let Some(verdict) = space_mgr.diagnose_point(npc_id, &pos) else {
        return;
    };
    if verdict.valid {
        return;
    }
    let Some(ident) = NpcIdent::of(space_mgr, npc_id) else {
        return;
    };
    let gate = verdict.gate.map_or("unknown", |g| g.label());
    cimmeria_observability::counter!(
        "npc_off_mesh_total",
        "world" => ident.world.clone(),
        "gate" => gate,
    );
    // An NPC that has sat where it spawned since it spawned is one fact, not
    // a recurring one: nothing about it changes until it moves (NA24, UAT-1
    // D -- Castle_BravoOfficer3 wrote 402 WARNs into an empty Castle). One
    // WARN per NPC, then the same row at DEBUG on the 30 s window. As soon as
    // anything moves it the source is no longer `Spawn` and the WARN cadence
    // is back.
    let parked_since_spawn =
        space_mgr.npc_detectors.move_source(npc_id) == Some(super::MoveSource::Spawn);
    let admitted = if parked_since_spawn {
        space_mgr
            .npc_detectors
            .admit_warn(npc_id, "npc_off_mesh_parked", now, Duration::MAX)
            .map(|s| (true, s))
            .or_else(|| {
                space_mgr
                    .npc_detectors
                    .admit_sample(npc_id, "npc_off_mesh", now, OFF_MESH_WARN_INTERVAL)
                    .map(|s| (false, s))
            })
    } else {
        space_mgr
            .npc_detectors
            .admit_warn(npc_id, "npc_off_mesh", now, OFF_MESH_WARN_INTERVAL)
            .map(|s| (true, s))
    };
    let Some((warn, suppressed)) = admitted else {
        return;
    };
    let last_move_source = space_mgr
        .npc_detectors
        .move_source(npc_id)
        .map_or("unknown", |s| s.label());
    let ai_state = space_mgr
        .get_entity(npc_id)
        .map_or("unknown", |e| e.ai_state().label());
    macro_rules! off_mesh_row {
        ($level:ident) => {
            tracing::$level!(
                target: "npc_ai",
                event = "npc_off_mesh",
                npc_id,
                tag = %ident.tag,
                template_id = ident.template_id,
                world = %ident.world,
                space_id = ident.space_id,
                gate,
                horizontal_dist = verdict.horizontal_dist,
                dy = verdict.dy,
                x = pos.x,
                y = pos.y,
                z = pos.z,
                last_move_source,
                ai_state,
                suppressed,
                "npc_ai: ticked NPC is outside navmesh coverage -- its paths start from                  nowhere and its line of sight reads unknown"
            )
        };
    }
    if warn {
        off_mesh_row!(warn);
    } else {
        off_mesh_row!(debug);
    }
}

fn check_stuck(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    state_before: AiState,
    outcome: &str,
    now: Instant,
) {
    let Some(e) = space_mgr.get_entity(npc_id) else {
        return;
    };
    // `no_path` counts with or without a (stale) path: an NPC that never
    // got a route is the most stuck of all. The other chasing outcomes need
    // a path, or the NPC is not trying to close.
    let chasing = state_before == AiState::Fighting
        && e.ai_state() == AiState::Fighting
        && CHASING_OUTCOMES.contains(&outcome)
        && (outcome == "no_path" || !e.nav_path.is_empty());
    let target = e
        .threat_list
        .iter()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(id, _)| *id);
    let target_pos = target
        .and_then(|t| space_mgr.get_entity(t))
        .map(|t| t.position);
    let (npc_pos, nav_path_len, next_wp) =
        (e.position, e.nav_path.len(), e.nav_path.front().copied());
    let track = space_mgr.npc_detectors.ai.entry(npc_id).or_default();
    let (Some(target_id), Some(target_pos), true) = (target, target_pos, chasing) else {
        track.chase_history.clear();
        return;
    };
    track
        .chase_history
        .push_back(npc_pos.distance_to(&target_pos));
    while track.chase_history.len() > STUCK_TICKS {
        track.chase_history.pop_front();
    }
    if track.chase_history.len() < STUCK_TICKS {
        return;
    }
    let first = track.chase_history[0];
    let best_later = track
        .chase_history
        .iter()
        .skip(1)
        .copied()
        .fold(f32::INFINITY, f32::min);
    if best_later < first - STUCK_MIN_PROGRESS {
        return;
    }
    let history: Vec<f32> = track.chase_history.iter().copied().collect();
    let Some(ident) = NpcIdent::of(space_mgr, npc_id) else {
        return;
    };
    cimmeria_observability::counter!("npc_stuck_total", "world" => ident.world.clone());
    let Some(suppressed) =
        space_mgr
            .npc_detectors
            .admit_warn(npc_id, "stuck", now, STUCK_WARN_INTERVAL)
    else {
        return;
    };
    let los = super::super::aggro_acquired::los_label(space_mgr.line_of_sight(npc_id, target_id));
    tracing::warn!(
        target: "npc_ai",
        event = "stuck",
        npc_id,
        tag = %ident.tag,
        template_id = ident.template_id,
        world = %ident.world,
        space_id = ident.space_id,
        target_id,
        npc_to_target_history = ?history,
        decision_outcome = outcome,
        nav_path_len,
        los,
        next_wp = ?next_wp.map(|p| [p.x, p.y, p.z]),
        x = npc_pos.x,
        y = npc_pos.y,
        z = npc_pos.z,
        suppressed,
        "npc_ai: NPC is chasing but has not closed on its target for {} AI ticks",
        STUCK_TICKS
    );
}
