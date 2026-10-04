//! `gmSetMobAbilitySet(INT32 aAbilitySetId)` (SGWGmPlayer 158,
//! `/gmsetmobabilityset`): swap the selected mob onto an NPC ability set.
//!
//! The def has no target argument; the subject is the GM's selection
//! (`current_target_id`), which must be an NPC in the GM's space. A player
//! is refused: a player's abilities are the persisted character's, and this
//! command writes only the cell's in-memory set. The set comes from
//! `resources.ability_set_abilities` (`SpaceManager::ability_sets`, loaded
//! at startup). The mob's known abilities become exactly the set's, and the
//! NPC AI picks from them on its next decision (`npc_ai::ability_select`).
//! A warming cast of an ability the swap removed is interrupted, as a
//! respec interrupts one (`interrupt_unlearned_cast`).
//! Nothing is persisted: the mob's next spawn uses its template's set
//! again.
//!
//! Refusals, each with a feedback line and one `gm_command` row:
//! `bad_args`, `no_target`, `target_gone`, `target_other_space`,
//! `target_is_player`, `no_ability_set_data` (the startup load failed or
//! the table is empty, so no id can be checked) and `unknown_set`.

use tokio::sync::mpsc;

use super::command_log::{log_gm_command, Outcome};
use super::feedback::send_gm_feedback;
use super::{read_i32, GM_SET_MOB_ABILITY_SET};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

const CMD: &str = "gmSetMobAbilitySet";

/// `gmSetMobAbilitySet(INT32 aAbilitySetId)` on the GM's selected mob.
pub(super) async fn handle_set_mob_ability_set(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(set_id) = read_i32(args, 0) else {
        let refused = Outcome::Refused("bad_args");
        log_gm_command(
            space_mgr,
            entity_id,
            CMD,
            GM_SET_MOB_ABILITY_SET,
            "",
            refused,
        );
        send_gm_feedback(
            entity_id,
            "gmSetMobAbilitySet: missing INT32 aAbilitySetId",
            tx,
        )
        .await;
        return true;
    };
    let target = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.current_target_id)
        .and_then(|t| u32::try_from(t).ok())
        .filter(|&t| t != 0);
    let args_text = match target {
        Some(t) => format!("aAbilitySetId={set_id} target={t}"),
        None => format!("aAbilitySetId={set_id}"),
    };
    match apply(entity_id, target, set_id, space_mgr) {
        Ok(Swap {
            mob,
            abilities,
            removed,
        }) => {
            // A warmup of an ability the swap took away would otherwise
            // fire it when the warmup ends (AT-10's respec rule).
            crate::cell::abilities::interrupt_unlearned_cast(mob, &removed, tx, space_mgr).await;
            log_gm_command(
                space_mgr,
                entity_id,
                CMD,
                GM_SET_MOB_ABILITY_SET,
                &args_text,
                Outcome::Applied,
            );
            let text = format!(
                "gmSetMobAbilitySet: mob {mob} now uses ability set {set_id}: {abilities:?} \
                 (until it respawns)"
            );
            send_gm_feedback(entity_id, &text, tx).await;
        }
        Err((reason, text)) => {
            let refused = Outcome::Refused(reason);
            log_gm_command(
                space_mgr,
                entity_id,
                CMD,
                GM_SET_MOB_ABILITY_SET,
                &args_text,
                refused,
            );
            send_gm_feedback(entity_id, &text, tx).await;
        }
    }
    true
}

/// Check the selection and the set, then replace the mob's known
/// abilities.
fn apply(
    caller_id: u32,
    target: Option<u32>,
    set_id: i32,
    space_mgr: &mut SpaceManager,
) -> Result<Swap, (&'static str, String)> {
    let Some(mob) = target else {
        return Err(("no_target", format!("{CMD}: select a mob first")));
    };
    let caller_space = space_mgr.get_entity(caller_id).map(|e| e.space_id);
    let Some(entity) = space_mgr.get_entity(mob) else {
        return Err(("target_gone", format!("{CMD}: entity {mob} is gone")));
    };
    if Some(entity.space_id) != caller_space {
        let text = format!("{CMD}: entity {mob} is not in your space");
        return Err(("target_other_space", text));
    }
    if entity.is_player {
        let text = format!("{CMD}: entity {mob} is a player; select a mob");
        return Err(("target_is_player", text));
    }
    if space_mgr.ability_sets.is_empty() {
        let text = format!("{CMD}: no ability sets are loaded on this server");
        return Err(("no_ability_set_data", text));
    }
    let Some(abilities) = space_mgr.ability_sets.get(&set_id).cloned() else {
        let text = format!("{CMD}: no ability set {set_id} in resources.ability_set_abilities");
        return Err(("unknown_set", text));
    };
    let Some(entity) = space_mgr.get_entity_mut(mob) else {
        return Err(("target_gone", format!("{CMD}: entity {mob} is gone")));
    };
    let removed: Vec<i32> = entity
        .abilities
        .known_ability_ids()
        .into_iter()
        .filter(|id| !abilities.contains(id))
        .collect();
    for &id in &removed {
        entity.abilities.remove_ability(id);
    }
    for &id in &abilities {
        entity.abilities.add_ability(id);
    }
    Ok(Swap {
        mob,
        abilities,
        removed,
    })
}

/// A set swap [`apply`] made.
#[derive(Debug)]
struct Swap {
    mob: u32,
    /// The mob's known abilities now.
    abilities: Vec<i32>,
    /// What it knew before and no longer does.
    removed: Vec<i32>,
}
