//! Idle auto-aggro: the scan that promotes a hostile Idle NPC into Fighting
//! (`cause=proximity`). Split out of `fight.rs` when NA00 pushed that file
//! past the size cap; this is the target-acquisition seam. NA13 added the
//! faction-derived hostility and the radius / vertical band / LoS / GM gates
//! ([`super::aggro_gates`]).

use std::time::Instant;

use tokio::sync::mpsc;

use super::aggro_gates::evaluate_candidate;
use super::detectors::aggro_scan::{report_scan, ScanReject};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Auto-aggro tick for Idle NPCs that are hostile to players (NA13: the
/// override, else the faction reaction).
///
/// Scans the NPC's witnesses through the candidate gates and seeds a small
/// threat on the closest survivor. `generate_threat` moves the NPC into
/// Fighting on the spot and emits `npc_ai.aggro event=acquired
/// cause=proximity`.
///
/// Seed magnitude (`1.0`) is intentionally tiny so an explicit
/// `generate_threat` from a content chain (e.g., chain 1032's `1000`)
/// dominates and focuses the NPC on the triggering player rather than
/// whichever player happens to be closest.
///
/// Returns whether the NPC engaged, so the Idle dispatcher can fall through
/// to patrol / wander when nobody qualified.
pub(super) async fn npc_ai_idle_auto_aggro(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    use crate::cell::combat;

    let now = Instant::now();
    let Some(suppressed) = space_mgr
        .get_entity(npc_id)
        .map(|e| e.leash.reaggro_suppressed(now))
    else {
        return false;
    };

    // Witnesses-of-NPC = players currently rendering this NPC, i.e. players
    // in the NPC's AoI. The gates below narrow that 100 u set to the room.
    let witnesses = space_mgr.get_witnesses_of(npc_id);
    let witness_count = witnesses.len();
    let mut rejects: Vec<(u32, ScanReject)> = Vec::new();

    // Post-reset window (NA12): an NPC that just walked home and reset
    // ignores players for a few seconds. Without it an aggressive NPC
    // re-aggroed on the same player the tick after its reset, which is half
    // of the aggro/leash loop (audit S5).
    if suppressed {
        super::record_decision_outcome("reaggro_suppressed");
        rejects.extend(
            witnesses
                .iter()
                .map(|&pid| (pid, ScanReject::PostResetSuppressed)),
        );
        report_scan(space_mgr, npc_id, witness_count, &rejects, false, now);
        return false;
    }

    let target = {
        let Some(npc) = space_mgr.get_entity(npc_id) else {
            return false;
        };
        witnesses
            .iter()
            .filter_map(|&pid| match evaluate_candidate(space_mgr, npc, pid) {
                Ok(dist) => Some((pid, dist)),
                Err(reason) => {
                    rejects.push((pid, reason));
                    None
                }
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    };
    report_scan(
        space_mgr,
        npc_id,
        witness_count,
        &rejects,
        target.is_some(),
        now,
    );

    let Some((player_id, dist)) = target else {
        return false;
    };
    tracing::info!(
        npc_id,
        player_id,
        dist,
        "NPC AI: proximity auto-aggro on a hostile player"
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
        return true;
    }
    false
}
