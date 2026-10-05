//! The owner-pet tick (pets PT-08): expire timed pet buffs and carry out To
//! The Death.
//!
//! Runs every AoI tick from the cell message loop, after the pet sweep. It
//! returns at once when no pet carries a buff or a doom.
//!
//! **To The Death (2839).** When the pet's `doomed_at` passes, its buffs
//! that ran out come off first (the +400 Accuracy lapses at the same
//! moment), then the pet dies through the one death resolver
//! (`kill_npc_out_of_band`) with itself as the killer and `grant_xp =
//! false`. So the kill pays nobody: no owner XP (a pet's kill would
//! otherwise credit its owner, PT-06), no mission `EntityDeath` (only the
//! kill-credit wrappers raise it), and the pet has no loot table. The
//! corpse then follows the ordinary pet-death path: the sweep stamps its
//! 10 s timer and despawns it (D-PT08). A pet that died earlier, or was
//! dismissed, is skipped.

use cimmeria_entity::cell_entity::PetState;
use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_cell_world::cell::pets::BuffRemoval;
use cimmeria_entity::stats::HEALTH;
use cimmeria_wire::state_field::BSF_DEAD;

use super::super::super::super::messages::CellToBaseMsg;
use super::super::super::super::space_manager::SpaceManager;
use super::fire::flush_pet_stats;

/// [`owner_pet_tick_at`] on the wall clock.
pub async fn owner_pet_tick(tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    let _ = owner_pet_tick_at(Instant::now(), tx, space_mgr).await;
}

/// Expire every pet buff and carry out every doom due by `now`. Returns how
/// many pets To The Death killed. `now` is a parameter so tests can step
/// past a 60 s timer without sleeping.
pub async fn owner_pet_tick_at(
    now: Instant,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    if space_mgr.pets.is_empty() || !space_mgr.any_pet_buff_or_doom() {
        return 0;
    }
    let mut touched = Vec::new();
    for (pet, effect_id) in space_mgr.expired_pet_buffs(now) {
        if space_mgr
            .remove_pet_buff(pet, effect_id, BuffRemoval::Expired)
            .is_some()
        {
            touched.push(pet);
        }
    }
    touched.dedup();
    for pet in touched {
        flush_pet_stats(pet, tx, space_mgr).await;
    }

    let mut killed = 0;
    for pet in space_mgr.doomed_pets_due(now) {
        if carry_out_doom(pet, tx, space_mgr).await {
            killed += 1;
        }
    }
    killed
}

/// Kill a doomed pet. Returns true when it died here.
async fn carry_out_doom(
    pet: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(entity) = space_mgr.get_entity_mut(pet) else {
        return false;
    };
    let already_dead = entity.state_field & BSF_DEAD != 0;
    let template_id = entity.template_id;
    let Some(state) = entity.extensions.get_mut::<PetState>() else {
        return false;
    };
    state.doomed_at = None;
    let owner_id = state.owner_id;
    let id = space_mgr.pet_summoner_identity(pet);
    if already_dead {
        tracing::debug!(
            target: "pets.buff",
            event = "doom_skipped",
            decision_outcome = "doom_skipped",
            reason = "pet_dead",
            entity_id = pet,
            entity_name = space_mgr.entity_label(pet),
            pet_id = pet,
            pet_name = space_mgr.entity_label(pet),
            owner_id,
            owner_name = id.player_name,
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            template_id,
            template_name = cimmeria_cell_world::cell::effects::content_names::template_name(template_id),
            "To The Death ran out on a pet that was already dead"
        );
        return false;
    }
    // The health bar reads empty on every client before the corpse burst.
    let health_before = space_mgr
        .get_entity_mut(pet)
        .and_then(|e| e.stats.get_mut(HEALTH))
        .map(|h| {
            let before = h.cur;
            h.set_current(h.min);
            before
        });
    flush_pet_stats(pet, tx, space_mgr).await;
    let killed =
        crate::cell::abilities::kill_npc_out_of_band(pet, pet, false, false, tx, space_mgr).await;
    tracing::info!(
        target: "pets.buff",
        event = "doom_fired",
        decision_outcome = if killed { "pet_killed" } else { "kill_not_applied" },
        entity_id = pet,
        entity_name = space_mgr.entity_label(pet),
        pet_id = pet,
        pet_name = space_mgr.entity_label(pet),
        owner_id,
        owner_name = id.player_name,
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        template_id,
        template_name = cimmeria_cell_world::cell::effects::content_names::template_name(template_id),
        health_before,
        killed,
        xp_granted = 0,
        "To The Death ran out: the pet dies (no XP, no kill credit)"
    );
    killed
}
