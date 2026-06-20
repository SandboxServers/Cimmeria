//! Investigate state: walk to a content-set POI, dwell, return to Idle.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::record_decision_outcome;

/// Dwell at the POI after pathfinding reaches it, in seconds. Hardcoded
/// because no template field carries an investigate-specific dwell yet;
/// future work can lift this to `entity_templates.investigate_dwell_secs`
/// if encounters need varying durations.
const INVESTIGATE_DWELL_SECS: f32 = 5.0;

/// NPC investigate behavior: walk to a content-set POI, dwell, return
/// to Idle.
///
/// State machine within Investigating:
/// - **No POI** → drop back to Idle (defensive — a content action
///   could have cleared the POI mid-tick).
/// - **POI + nav_path non-empty** → walking, no-op.
/// - **POI + nav_path empty + no dwell** → first entry; pathfind to
///   POI and queue.
/// - **POI + nav_path empty + future dwell** → at the POI, pausing.
/// - **POI + nav_path empty + elapsed dwell** → done, clear POI +
///   investigate_until, drop to Idle.
pub(super) async fn npc_ai_investigate(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::{AiState, MobMovementType};

    let (poi, npc_pos, nav_empty, dwell) = match space_mgr.get_entity(npc_id) {
        Some(e) => (
            e.poi,
            e.position,
            e.nav_path.is_empty(),
            e.investigate_until,
        ),
        None => return,
    };

    // No-POI drop fires BEFORE the CombatAdvance broadcast so the
    // wire doesn't see a movement-type for an NPC that's about to
    // leave Investigating this tick.
    let Some(poi_pos) = poi else {
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.ai_state = AiState::Idle;
            npc.investigate_until = None;
        }
        crate::cell::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
        return;
    };

    // Use CombatAdvance as the closest movement-type — no dedicated
    // "investigating" byte exists in EMobMovementType, and the
    // animation it implies (alert advance) is the right hint.
    crate::cell::abilities::broadcast_movement_type(
        npc_id,
        Some(MobMovementType::CombatAdvance),
        tx,
        space_mgr,
    )
    .await;

    if !nav_empty {
        // Movement in flight toward the POI — equivalent to
        // investigate_routed; the next tick observes arrival.
        record_decision_outcome("investigate_routed");
        return; // Movement in flight.
    }

    // nav_path empty. Either we've arrived at the POI or we haven't
    // started yet. Use position-vs-POI distance to distinguish.
    let close = npc_pos.distance_to(&poi_pos) < 1.0;
    let now = std::time::Instant::now();

    if close {
        // At the POI. Dwell logic mirrors patrol.
        // investigate_arrived covers all dwell sub-states (just-
        // arrived, still-dwelling, dwell-elapsed-return-to-idle) —
        // they're all "at the POI" outcomes from a SigNoz aggregation
        // perspective. The per-transition debug events discriminate
        // the sub-step.
        record_decision_outcome("investigate_arrived");
        match dwell {
            None => {
                // Just arrived — stamp dwell.
                if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                    npc.investigate_until =
                        Some(now + std::time::Duration::from_secs_f32(INVESTIGATE_DWELL_SECS));
                }
                tracing::debug!(
                    target: "npc_ai",
                    event = "investigate_arrived",
                    npc_id,
                    "NPC AI: investigate → arrived at POI, dwelling"
                );
            }
            Some(deadline) if now < deadline => {
                // Still dwelling.
            }
            Some(_) => {
                // Dwell elapsed → return to Idle.
                if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                    npc.ai_state = AiState::Idle;
                    npc.poi = None;
                    npc.investigate_until = None;
                }
                crate::cell::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
            }
        }
    } else {
        // Not at POI — pathfind and queue movement. Clearing
        // `investigate_until` here handles the knockback case: if
        // the NPC was dwelling at the POI and got pushed off, the
        // re-arrival should re-stamp from scratch rather than
        // observe `Some(past)` and immediately return to Idle.
        let path = space_mgr
            .find_path(npc_id, &npc_pos, &poi_pos)
            .unwrap_or_default();
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.investigate_until = None;
            npc.nav_path.clear();
            if path.len() > 1 {
                for wp in path.into_iter().skip(1) {
                    npc.nav_path.push_back(wp);
                }
            } else {
                npc.nav_path.push_back(poi_pos);
            }
        }
        // Pathfind queued — the next tick will observe nav_empty=false
        // and record investigate_routed again until arrival.
        record_decision_outcome("investigate_routed");
        tracing::debug!(
            target: "npc_ai",
            event = "investigate_routed",
            npc_id,
            poi_x = poi_pos.x,
            poi_y = poi_pos.y,
            poi_z = poi_pos.z,
            "NPC AI: investigate → pathfinding to POI"
        );
    }
}
