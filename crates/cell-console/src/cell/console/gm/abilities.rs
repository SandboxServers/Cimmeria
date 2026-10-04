//! The GM's own ability set (ability-mechanics AB-N2):
//!
//! - `gmGiveAbility(INT32)` (136, `/gmgiveability`): one ability, no
//!   training point. The `.giveability` grant ([`plan_grant`] and
//!   [`send_grant`]), with the subject fixed to the caller.
//! - `gmResetAbilities()` (153, `/gmresetabilities`): back to the
//!   archetype's character-creation starters, spend refunded, no trainer
//!   and no charge.
//! - `gmGiveAllAbilities()` (154, `/gmgiveallabilities`): every ability in
//!   the caller's archetype tree, all three branches, in one write and one
//!   hotbar burst.
//!
//! None of the three defs carries a target argument, so each acts on the
//! calling GM only. The base persists (`sgw_player.abilities`) and answers;
//! the cell mirror sends `onKnownAbilitiesUpdate` and the GM's result line
//! (`service/base_messages/gm_abilities.rs` for 153 and 154,
//! `ability_granted.rs` for 136).

use tokio::sync::mpsc;

use super::super::give_ability::{plan_grant, send_grant};
use super::command_log::{log_gm_command, Outcome};
use super::feedback::send_gm_feedback;
use super::{read_i32, GM_GIVE_ABILITY, GM_GIVE_ALL_ABILITIES, GM_RESET_ABILITIES};
use crate::cell::messages::{CellToBaseMsg, GmAbilityBulk, GmAbilityChange};
use crate::cell::space_manager::{SpaceManager, TreeGrantRefusal};

/// `gmGiveAbility(INT32 aAbilityID)`: grant one ability to the caller.
pub(super) async fn handle_give_ability(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    const CMD: &str = "gmGiveAbility";
    let Some(ability_id) = read_i32(args, 0) else {
        let refused = Outcome::Refused("bad_args");
        log_gm_command(space_mgr, entity_id, CMD, GM_GIVE_ABILITY, "", refused);
        send_gm_feedback(entity_id, "gmGiveAbility: missing INT32 aAbilityID", tx).await;
        return true;
    };
    let args_text = format!("aAbilityID={ability_id}");
    let grant = match plan_grant(CMD, entity_id, None, ability_id, space_mgr) {
        Ok(grant) => grant,
        Err(refusal) => {
            let refused = Outcome::Refused(refusal.reason);
            log_gm_command(
                space_mgr,
                entity_id,
                CMD,
                GM_GIVE_ABILITY,
                &args_text,
                refused,
            );
            send_gm_feedback(entity_id, &refusal.text, tx).await;
            return true;
        }
    };
    if send_grant(&grant, entity_id, tx).await.is_err() {
        // Server shutting down: nobody is left to read feedback either.
        let refused = Outcome::Refused("cell_to_base_closed");
        log_gm_command(
            space_mgr,
            entity_id,
            CMD,
            GM_GIVE_ABILITY,
            &args_text,
            refused,
        );
        return true;
    }
    log_gm_command(
        space_mgr,
        entity_id,
        CMD,
        GM_GIVE_ABILITY,
        &args_text,
        Outcome::Forwarded,
    );
    true
}

/// `gmGiveAllAbilities()`: every tree ability the caller does not know.
pub(super) async fn handle_give_all_abilities(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let change = GmAbilityChange::GrantAll;
    let planned = plan_give_all(entity_id, space_mgr);
    let (player_id, ability_ids) = match planned {
        Ok(v) => v,
        Err((reason, text)) => {
            let refused = Outcome::Refused(reason);
            log_gm_command(
                space_mgr,
                entity_id,
                change.command(),
                GM_GIVE_ALL_ABILITIES,
                "",
                refused,
            );
            send_gm_feedback(entity_id, &text, tx).await;
            return true;
        }
    };
    let args_text = format!("missing={}", ability_ids.len());
    forward_bulk(
        entity_id,
        player_id,
        change,
        ability_ids,
        &args_text,
        tx,
        space_mgr,
    )
    .await;
    true
}

/// The caller's character and the archetype-tree ids it does not know yet,
/// in tree order ([`SpaceManager::plan_tree_grant`], shared with the Debug
/// Area ability granter).
pub(super) fn plan_give_all(
    entity_id: u32,
    space_mgr: &SpaceManager,
) -> Result<(i32, Vec<i32>), (&'static str, String)> {
    let cmd = GmAbilityChange::GrantAll.command();
    match space_mgr.plan_tree_grant(entity_id) {
        Ok(plan) => Ok((plan.player_id, plan.missing)),
        Err(refusal) => {
            let text = match refusal {
                TreeGrantRefusal::EntityGone => format!("{cmd}: caller entity not found"),
                TreeGrantRefusal::NotPlayer => format!("{cmd}: you have no player id"),
                TreeGrantRefusal::NoArchetype => {
                    format!("{cmd}: your character has no archetype")
                }
                TreeGrantRefusal::NoTree { archetype } => {
                    format!("{cmd}: archetype {archetype} has no ability tree loaded")
                }
                TreeGrantRefusal::NothingToGrant { tree_len } => {
                    format!("{cmd}: you already know all {tree_len} abilities in your tree")
                }
            };
            Err((refusal.reason(), text))
        }
    }
}

/// `gmResetAbilities()`: back to the archetype's starters.
pub(super) async fn handle_reset_abilities(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let change = GmAbilityChange::Reset;
    let Some(player_id) = space_mgr.get_entity(entity_id).and_then(|e| e.player_id) else {
        let refused = Outcome::Refused("caller_not_player");
        log_gm_command(
            space_mgr,
            entity_id,
            change.command(),
            GM_RESET_ABILITIES,
            "",
            refused,
        );
        send_gm_feedback(entity_id, "gmResetAbilities: you have no player id", tx).await;
        return true;
    };
    forward_bulk(entity_id, player_id, change, Vec::new(), "", tx, space_mgr).await;
    true
}

/// Send one `GmAbilityBulk` to the base and log the command's row.
async fn forward_bulk(
    entity_id: u32,
    player_id: i32,
    change: GmAbilityChange,
    ability_ids: Vec<i32>,
    args_text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let index = match change {
        GmAbilityChange::GrantAll => GM_GIVE_ALL_ABILITIES,
        GmAbilityChange::Reset => GM_RESET_ABILITIES,
    };
    let msg = CellToBaseMsg::GmAbilityBulk(GmAbilityBulk {
        entity_id,
        player_id,
        account_id: space_mgr.player_identity(entity_id).account_id,
        change,
        ability_ids,
    });
    let outcome = if tx.send(msg).await.is_ok() {
        Outcome::Forwarded
    } else {
        Outcome::Refused("cell_to_base_closed")
    };
    log_gm_command(
        space_mgr,
        entity_id,
        change.command(),
        index,
        args_text,
        outcome,
    );
}
