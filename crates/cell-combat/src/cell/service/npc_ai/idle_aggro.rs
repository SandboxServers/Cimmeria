//! Idle auto-aggro: the scan that promotes a hostile Idle NPC into Fighting
//! (`cause=proximity`). Split out of `fight.rs` when NA00 pushed that file
//! past the size cap; this is the target-acquisition seam. NA13 added the
//! faction-derived hostility and the radius / vertical band / LoS / GM gates
//! ([`super::aggro_gates`]). #1009 added the NPC-vs-NPC half: an NPC also
//! scans the NPCs around it and engages one its faction is HOSTILE to.

use std::time::Instant;

use tokio::sync::mpsc;

use super::aggro_gates::{evaluate_candidate, evaluate_npc_candidate};
use super::detectors::aggro_scan::{report_npc_rejects, report_scan, ScanReject};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// A scan's pick: the candidate and its horizontal distance.
type Pick = Option<(u32, f32)>;

/// Auto-aggro tick for an Idle NPC that is hostile to players (NA13: the
/// override, else the faction reaction) or that fights NPCs (#1009:
/// `combat::seeks_npc_targets`).
///
/// Two scans, and the closest survivor of either wins:
///
/// - **Players**, for an NPC hostile to players: the NPC's witnesses through
///   the NA13 candidate gates.
/// - **NPCs**, for an NPC whose faction has an NPC enemy: the SGWMob NPCs
///   within its aggro radius, from the space grid
///   ([`SpaceManager::npc_ids_near`], a bounded query, never a sweep of the
///   space), through [`evaluate_npc_candidate`]. It runs only while some
///   player witnesses the NPC: an NPC fight nobody can see is not started,
///   so a standoff in an empty zone costs one grid-free check per tick. A
///   fight already under way carries on without a witness.
///
/// The winner gets a small threat seed. `generate_threat` moves the NPC into
/// Fighting on the spot and emits `npc_ai.aggro event=acquired
/// cause=proximity` with the target's kind and both factions.
///
/// Seed magnitude (`1.0`) is intentionally tiny so an explicit
/// `generate_threat` from a content chain (e.g., chain 1032's `1000`)
/// dominates and focuses the NPC on the triggering player rather than
/// whichever player happens to be closest; and so that whoever then damages
/// the NPC, player or NPC, takes over its top threat.
///
/// Returns whether the NPC is now Fighting, so the Idle dispatcher falls
/// through to patrol / wander only when nobody qualified. (Before #1009 this
/// returned whether the *player* had just entered combat, so a guard that
/// engaged a player already fighting something else reported `false` and a
/// patroller's dispatcher then overwrote its fresh Fighting with Patrol.)
pub(super) async fn npc_ai_idle_auto_aggro(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    use crate::cell::combat;
    use cimmeria_entity::cell_entity::AiState;

    let now = Instant::now();
    let Some((suppressed, hostile_to_players, seeks_npcs)) =
        space_mgr.get_entity(npc_id).map(|e| {
            (
                e.leash.reaggro_suppressed(now),
                combat::is_hostile_to_players(e),
                combat::seeks_npc_targets(e),
            )
        })
    else {
        return false;
    };

    // Witnesses-of-NPC = players currently rendering this NPC, i.e. players
    // whose view (`PLAYER_AOI_RADIUS`, 150 m) holds it. The gates below
    // narrow that set to the NPC's aggro radius and its room.
    let witnesses = space_mgr.get_witnesses_of(npc_id);
    let witness_count = witnesses.len();
    let mut rejects: Vec<(u32, ScanReject)> = Vec::new();

    // Post-reset window (NA12): an NPC that just walked home and reset
    // ignores everyone for a few seconds. Without it an aggressive NPC
    // re-aggroed on the same player the tick after its reset, which is half
    // of the aggro/leash loop (audit S5). NPC targets wait out the same
    // window.
    if suppressed {
        super::record_decision_outcome("reaggro_suppressed");
        if hostile_to_players {
            rejects.extend(
                witnesses
                    .iter()
                    .map(|&pid| (pid, ScanReject::PostResetSuppressed)),
            );
            report_scan(space_mgr, npc_id, witness_count, &rejects, false, now);
        }
        return false;
    }

    let player_pick: Pick = if hostile_to_players {
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
            .min_by(|a, b| a.1.total_cmp(&b.1))
    } else {
        None
    };

    let mut npc_rejects: Vec<(u32, ScanReject)> = Vec::new();
    let npc_pick: Pick = if seeks_npcs && witness_count > 0 {
        scan_npc_targets(space_mgr, npc_id, &mut npc_rejects)
    } else {
        None
    };

    if hostile_to_players {
        report_scan(
            space_mgr,
            npc_id,
            witness_count,
            &rejects,
            player_pick.is_some() || npc_pick.is_some(),
            now,
        );
    }
    report_npc_rejects(space_mgr, npc_id, &npc_rejects, now);

    // The closer of the two; a tie goes to the player.
    let target = match (player_pick, npc_pick) {
        (Some(p), Some(n)) if n.1 < p.1 => Some((n, false)),
        (Some(p), _) => Some((p, true)),
        (None, Some(n)) => Some((n, false)),
        (None, None) => None,
    };
    let Some(((target_id, dist), is_player)) = target else {
        return false;
    };
    super::record_decision_outcome("aggro_acquired");

    if is_player {
        tracing::info!(
            npc_id,
            npc_name = space_mgr.entity_label(npc_id),
            player_id = target_id,
            player_name = space_mgr.entity_label(target_id),
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
            target_id,
            npc_id,
            1.0,
            combat::AggroCause::Proximity,
        )
        .is_some()
        {
            // `Some` means the player just entered combat, but only the
            // appearance is refreshed here: the in-combat state field is not
            // sent, so the client gets no combat flag for this entry.
            let who = space_mgr.player_identity(target_id);
            tracing::info!(
                target: "threat",
                event = "enter_combat_unannounced",
                reason = "proximity_carveout",
                entity_id = target_id,
                entity_name = space_mgr.entity_label(target_id),
                account_id = who.account_id,
                account_name = who.account_name,
                player_id = who.player_id,
                player_name = who.player_name,
                mob_id = npc_id,
                mob_name = space_mgr.entity_label(npc_id),
                "proximity aggro entered combat without announcing the combat flag"
            );
            crate::cell::abilities::request_appearance_refresh(target_id, tx, space_mgr).await;
        }
    } else {
        // No player is involved, so there is no combat state or appearance
        // to send: `generate_threat` returns `None` for an NPC attacker.
        let _ = combat::generate_threat(
            space_mgr,
            target_id,
            npc_id,
            1.0,
            combat::AggroCause::Proximity,
        );
    }
    space_mgr
        .get_entity(npc_id)
        .is_some_and(|e| e.ai_state() == AiState::Fighting)
}

/// The NPC-vs-NPC scan (#1009): the closest HOSTILE NPC within `npc_id`'s
/// aggro radius that passes [`evaluate_npc_candidate`]. Refusals of hostile
/// NPCs go into `rejects` for [`report_npc_rejects`].
///
/// The candidates come from the space grid, so the work is the population of
/// the few 50 u cells around the NPC. The navmesh / occluder ray runs only for
/// a hostile NPC already inside the radius and the vertical band.
pub(super) fn scan_npc_targets(
    space_mgr: &SpaceManager,
    npc_id: u32,
    rejects: &mut Vec<(u32, ScanReject)>,
) -> Pick {
    let npc = space_mgr.get_entity(npc_id)?;
    npc_scan_candidates(space_mgr, npc_id)
        .into_iter()
        .filter_map(|cid| match evaluate_npc_candidate(space_mgr, npc, cid)? {
            Ok(dist) => Some((cid, dist)),
            Err(reason) => {
                rejects.push((cid, reason));
                None
            }
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

/// NPCs within this multiple of the scanner's aggro radius are *considered*:
/// a hostile one just outside the radius logs `out_of_radius`, so "why did the
/// friendly not engage the guard 20 u away" has an answer. The same factor as
/// the NA14 assist fan-out.
const NPC_CONSIDER_RADIUS_FACTOR: f32 = 2.0;

/// The NPCs [`scan_npc_targets`] evaluates for `npc_id`: a grid query of
/// [`NPC_CONSIDER_RADIUS_FACTOR`] times its aggro radius, in ascending id
/// order (deterministic logs and ties). Bounded by the population of the few
/// grid cells around the NPC, whatever the size of the space.
pub(super) fn npc_scan_candidates(space_mgr: &SpaceManager, npc_id: u32) -> Vec<u32> {
    let Some(npc) = space_mgr.get_entity(npc_id) else {
        return Vec::new();
    };
    let radius = NPC_CONSIDER_RADIUS_FACTOR * crate::cell::combat::aggro_radius(npc);
    let mut candidates = space_mgr.npc_ids_near(npc_id, radius);
    candidates.sort_unstable();
    candidates
}

#[cfg(test)]
mod tests {
    use cimmeria_entity::cell_entity::{AiState, MobAggression};
    use tracing::Level;

    use super::npc_ai_idle_auto_aggro;
    use crate::test_fixtures::npc_detectors::{
        add_npc, add_threat_player, castle_mgr, NPC, PLAYER,
    };
    use crate::test_support::LogCapture;

    /// Proximity aggro on a hostile player in range gives one INFO
    /// `enter_combat_unannounced` row with the reason, the player's identity
    /// and the mob pair. Fails if the row is removed from the proximity
    /// branch of `npc_ai_idle_auto_aggro`.
    #[tokio::test]
    async fn proximity_aggro_on_a_player_logs_unannounced_combat_once() {
        let mut mgr = castle_mgr();
        add_npc(&mut mgr, "Castle", [10.0, 0.0, 10.0], None, AiState::Idle);
        let npc = mgr.get_entity_mut(NPC).unwrap();
        npc.aggro.override_level = Some(MobAggression::Hostile);
        // The player fixture is faction 0: a different faction keeps the
        // NPC from ignoring it as an ally (`SameFaction`).
        npc.faction = 1;
        add_threat_player(&mut mgr, "Castle", [13.0, 0.0, 14.0]);
        mgr.get_entity_mut(PLAYER).unwrap().account_id = Some(77);
        let logs = LogCapture::install();
        tracing::callsite::rebuild_interest_cache();
        let (tx, _rx) = tokio::sync::mpsc::channel(16);

        let engaged = npc_ai_idle_auto_aggro(NPC, &tx, &mut mgr).await;

        assert!(engaged, "the hostile player in range must engage the NPC");
        let rows: Vec<_> = logs
            .all()
            .into_iter()
            .filter(|c| c.target == "threat" && c.has_field("event", "enter_combat_unannounced"))
            .collect();
        assert_eq!(rows.len(), 1, "{rows:?}");
        let row = &rows[0];
        assert_eq!(row.level, Level::INFO);
        for (k, v) in [
            ("reason", "proximity_carveout"),
            ("entity_id", "101"),
            ("account_id", "77"),
            ("player_id", "101"),
            ("mob_id", "200"),
        ] {
            assert!(row.has_field(k, v), "field {k}={v} missing: {row:?}");
        }
    }
}
