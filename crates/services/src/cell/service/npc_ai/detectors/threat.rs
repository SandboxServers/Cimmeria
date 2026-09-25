//! `threat event=cleared_without_exit`: an NPC dropped its whole threat list
//! while a player still lists it in `threatened_mobs` (audit S7).
//!
//! Before NA12 neither the leash nor a lost target called
//! `exit_player_combat`, so the player kept `BSF_InCombat`, could not
//! regenerate and could not holster. NA12 drains every player before each
//! clear, so this should read zero; a row now means a new clear path that
//! skipped the drain.

use std::time::{Duration, Instant};

use super::NpcIdent;
use crate::cell::space_manager::SpaceManager;

const CLEARED_WITHOUT_EXIT_WARN_INTERVAL: Duration = Duration::from_secs(30);

/// Why the NPC's threat list was cleared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::cell) enum ThreatClear {
    /// Fighting with nobody left on the list.
    ThreatEmpty,
    /// The NPC went past its leash radius (NA12: its own distance).
    LeashOut,
    /// The last target died, vanished or was lost (NA12).
    TargetLost,
    /// Leash recovery finished.
    LeashComplete,
}

impl ThreatClear {
    fn label(self) -> &'static str {
        match self {
            Self::ThreatEmpty => "threat_empty",
            Self::LeashOut => "leash_out",
            Self::TargetLost => "target_lost",
            Self::LeashComplete => "leash_complete",
        }
    }
}

/// Call right after an NPC's threat list was cleared.
pub(in crate::cell) fn check_cleared(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    reason: ThreatClear,
    now: Instant,
) {
    let Some(space_id) = space_mgr.get_entity_space_id(npc_id) else {
        return;
    };
    let players: Vec<u32> = space_mgr
        .spaces
        .get(&space_id)
        .map(|s| s.players.iter().copied().collect())
        .unwrap_or_default();
    let still_listed: Vec<u32> = players
        .into_iter()
        .filter(|pid| {
            space_mgr
                .get_entity(*pid)
                .is_some_and(|p| p.threatened_mobs.contains(&npc_id))
        })
        .collect();
    if still_listed.is_empty() {
        return;
    }
    let Some(ident) = NpcIdent::of(space_mgr, npc_id) else {
        return;
    };
    for player_id in still_listed {
        cimmeria_observability::counter!(
            "npc_threat_cleared_without_exit_total",
            "world" => ident.world.clone(),
            "reason" => reason.label(),
        );
        let Some(suppressed) = space_mgr.npc_detectors.pair_log.admit(
            player_id,
            npc_id,
            "cleared_without_exit",
            now,
            CLEARED_WITHOUT_EXIT_WARN_INTERVAL,
        ) else {
            continue;
        };
        let in_combat_with = space_mgr
            .get_entity(player_id)
            .map_or(0, |p| p.threatened_mobs.len());
        tracing::warn!(
            target: "threat",
            event = "cleared_without_exit",
            npc_id,
            mob_id = npc_id,
            tag = %ident.tag,
            template_id = ident.template_id,
            world = %ident.world,
            space_id = ident.space_id,
            player_id,
            reason = reason.label(),
            player_threatened_mobs = in_combat_with,
            suppressed,
            "threat: NPC cleared its threat but the player still lists it in \
             threatened_mobs -- the player stays in combat (no regen, no holster)"
        );
    }
}
