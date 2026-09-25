//! Idle auto-aggro: the scan that promotes an aggressive Idle NPC into
//! Fighting (`cause=proximity`). Split out of `fight.rs` when NA00 pushed
//! that file past the size cap; this is the target-acquisition seam, and
//! NA02 / NA13 extend it with the scan-reject telemetry and gates.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Auto-aggro tick for Idle NPCs with `aggression > 0`.
///
/// Scans witnesses for opposing-faction players and seeds a small threat on
/// the closest. `generate_threat` moves the NPC into Fighting on the spot
/// and emits `npc_ai.aggro event=acquired cause=proximity`.
///
/// Seed magnitude (`1.0`) is intentionally tiny so an explicit
/// `generate_threat` from a content chain (e.g., chain 1032's `1000`)
/// dominates and focuses the NPC on the triggering player rather than
/// whichever player happens to be closest. Caller (`npc_ai_tick`)
/// guarantees `aggression > 0`.
pub(super) async fn npc_ai_idle_auto_aggro(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use crate::cell::combat;

    let (npc_pos, npc_faction) = match space_mgr.get_entity(npc_id) {
        Some(e) => (e.position, e.faction),
        None => return,
    };

    // Witnesses-of-NPC = players currently rendering this NPC, i.e. players
    // in the NPC's AoI. That's exactly the candidate set the Python `Atrea`
    // engine scans — restricted to players because NPCs don't aggro on
    // other NPCs from idle.
    let witnesses = space_mgr.get_witnesses_of(npc_id);
    let target = witnesses
        .into_iter()
        .filter_map(|pid| {
            let p = space_mgr.get_entity(pid)?;
            if !p.is_player || p.faction == npc_faction {
                return None;
            }
            // Skip dead players (BSF_DEAD in state_field — bit 0).
            if combat::is_dead_state(p.state_field) {
                return None;
            }
            let dist = npc_pos.distance_to(&p.position);
            Some((pid, dist))
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(pid, _)| pid);

    if let Some(player_id) = target {
        tracing::info!(
            npc_id,
            player_id,
            "NPC AI: aggression-driven auto-aggro on opposing-faction player"
        );
        // Invariant: when `enter_player_combat` flips `weapon_holstered`
        // to false on first-add, the client's cached `ComponentList`
        // must be refreshed — otherwise the fire path passes the
        // `needs_unholster_queue` gate (server thinks drawn) while
        // the client still renders the holstered mesh. The
        // `onStateFieldUpdate` half is intentionally suppressed here
        // (auto-aggro can fire before the player has any visible
        // reason to know — lighting up `BSF_IN_COMBAT` is the "ghost
        // combat HUD" carve-out); the damage path broadcasts it on
        // the next explicit hit.
        if combat::generate_threat(
            space_mgr,
            player_id,
            npc_id,
            1.0,
            combat::AggroCause::Proximity,
        )
        .is_some()
        {
            crate::cell::abilities::request_appearance_refresh(player_id, tx, space_mgr).await;
        }
    }
}
