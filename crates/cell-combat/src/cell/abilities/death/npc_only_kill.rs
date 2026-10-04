//! NPC-only kills (NPC-vs-NPC, #1009): recognising a kill whose killer is a
//! plain NPC, and the one row that says what such a kill did not pay.

use super::super::super::space_manager::SpaceManager;

/// Whether a kill by `attacker_id` is an NPC-only kill (NPC-vs-NPC, #1009):
/// the killer is a live NPC that is not a pet. Such a kill pays nobody: no
/// loot roll here, no XP (`side_effects::grant_kill_xp` finds no player to
/// credit), and no mission `EntityDeath` (every credit path resolves the
/// killer through `credited_player`, which is `None` for a plain NPC). A
/// pet's kill is its owner's (pets PT-06) and a missing killer (a DoT whose
/// invoker left) keeps the old behaviour, so neither counts.
///
/// Credit is the killing blow's, as it has always been: a player who wounded
/// a guard that a friendly NPC then finished gets no kill credit.
pub(super) fn npc_only_kill(space_mgr: &SpaceManager, attacker_id: u32) -> bool {
    space_mgr.get_entity(attacker_id).is_some_and(|a| {
        !a.is_player
            && !a
                .extensions
                .contains::<cimmeria_entity::cell_entity::PetState>()
    })
}

/// The one row an NPC-only kill writes about what it did not pay
/// (`loot.drop event=skipped reason=npc_only_kill`), with both entities.
pub(super) fn log_npc_only_kill(space_mgr: &SpaceManager, target_eid: u32, attacker_id: u32) {
    let target = space_mgr.get_entity(target_eid);
    let killer = space_mgr.get_entity(attacker_id);
    tracing::info!(
        target: "loot.drop",
        event = "skipped",
        reason = "npc_only_kill",
        target_eid,
        target_tag = target.and_then(|t| t.tag.as_deref()).unwrap_or(""),
        target_faction = target.map(|t| t.faction),
        loot_table_id = target.and_then(|t| t.loot_table_id), // nt:id-only NameBook has no loot_tables lookup (description only)
        attacker_id,
        attacker_name = space_mgr.entity_label(attacker_id),
        attacker_tag = killer.and_then(|k| k.tag.as_deref()).unwrap_or(""),
        attacker_faction = killer.map(|k| k.faction),
        "death: NPC-only kill -- no loot rolled, no XP, no mission credit"
    );
}
