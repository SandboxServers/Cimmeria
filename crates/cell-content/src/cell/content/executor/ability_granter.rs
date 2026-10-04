//! `Action::GmAbilityBulk`: the Debug Area ability granter and ability reset
//! NPCs (DA-02).
//!
//! A right-click on the granter fires its `interact_tag` chain, which runs
//! this action for the clicking player. It does, from an NPC, exactly what
//! the native GM commands do for the GM themself:
//!
//! - `grant_all`: `/gmgiveallabilities` (154). Every ability of the
//!   player's archetype tree it does not know, all three branches and the
//!   capstones ([`SpaceManager::plan_tree_grant`], the plan the command
//!   uses).
//! - `reset`: `/gmresetabilities` (153). Back to the archetype's
//!   character-creation starters, tree points refunded.
//!
//! Both first clear every running cooldown and send the client the clear
//! timers ([`reset_all_cooldowns`], the `.cooldowns reset` path), so a
//! tester can press everything again at once.
//!
//! **Authority.** GM-gated on the clicking player's account access level
//! (`CellEntity::access_level`, the same field the native GM commands are
//! gated on). A non-GM gets a refusal line and nothing changes, not even a
//! cooldown. The base persists (`sgw_player.abilities`) and answers with
//! `GmAbilitiesChanged`; the cell mirror sends `onKnownAbilitiesUpdate`, so
//! the Abilities window refreshes, and the result line.
//!
//! **Feedback.** Every click gets a line from here on the first press: the
//! refusal, "you already know every ability", or what is on its way and
//! how many cooldowns were cleared. The base's result line follows a grant.

use tokio::sync::mpsc;

use cimmeria_cell_world::cell::dispatch::is_gm;
use cimmeria_content_engine::actions::AbilityBulkChange;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use crate::cell::abilities::{reset_all_cooldowns, CooldownReset};
use crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use crate::cell::messages::{CellToBaseMsg, GmAbilityBulk, GmAbilityChange};
use crate::cell::space_manager::{SpaceManager, TreeGrantRefusal};

/// The line a non-GM gets.
pub(crate) const NOT_GM_LINE: &str =
    "Only a GM can use this. Your abilities and cooldowns are unchanged.";

/// `event` of every granter row (target `content`).
const EVENT: &str = "ability_granter";

/// Run one `gm_ability_bulk` action for `entity_id`.
pub(super) async fn run(
    change: AbilityBulkChange,
    entity_id: u32,
    chain_id: i64,
    params: &std::collections::HashMap<String, serde_json::Value>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let who = space_mgr.player_identity(entity_id);
    let npc_entity_id = params
        .get("target_entity_id")
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok());
    let npc_name = npc_entity_id
        .and_then(|id| space_mgr.entity_label(id))
        .map(str::to_string);
    let Some(player) = space_mgr.get_entity(entity_id).filter(|e| e.is_player) else {
        tracing::warn!(
            target: "content",
            event = EVENT,
            decision_outcome = "refused",
            reason = "not_a_player",
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            chain_id, // nt:id-only the executor holds no chain description
            change = change.as_str(),
            "gm_ability_bulk fired for an entity that is not a player; nothing changed"
        );
        return;
    };
    let access_level = player.access_level;
    if !is_gm(access_level) {
        // A player clicking an NPC is ordinary play, not a fault.
        tracing::info!(
            target: "content",
            event = EVENT,
            decision_outcome = "refused",
            reason = "not_gm",
            entity_id,
            entity_name = who.player_name,
            account_id = who.account_id,
            account_name = who.account_name,
            player_id = who.player_id,
            player_name = who.player_name,
            access_level,
            npc_entity_id,
            npc_entity_name = npc_name.as_deref(),
            chain_id, // nt:id-only the executor holds no chain description
            change = change.as_str(),
            "ability granter refused: the player is not a GM"
        );
        send_line(entity_id, NOT_GM_LINE, tx).await;
        return;
    }

    let cleared = reset_all_cooldowns(entity_id, tx, space_mgr)
        .await
        .unwrap_or_default();
    let (wire_change, ability_ids, outcome) = match change {
        AbilityBulkChange::GrantAll => match space_mgr.plan_tree_grant(entity_id) {
            Ok(plan) => {
                let line = format!(
                    "Ability granter: granting {} abilities of your {} tree (all branches and \
                     capstones); {} cooldown(s) cleared.",
                    plan.missing.len(),
                    archetype_label(plan.archetype),
                    cleared.abilities.len()
                );
                (GmAbilityChange::GrantAll, plan.missing, Ok(line))
            }
            Err(refusal) => {
                let line = match refusal {
                    TreeGrantRefusal::NothingToGrant { tree_len } => format!(
                        "Ability granter: you already know all {tree_len} abilities of your \
                         tree; {} cooldown(s) cleared.",
                        cleared.abilities.len()
                    ),
                    TreeGrantRefusal::NoTree { archetype } => format!(
                        "Ability granter: no ability tree is loaded for archetype {archetype}; \
                         nothing granted, {} cooldown(s) cleared.",
                        cleared.abilities.len()
                    ),
                    TreeGrantRefusal::EntityGone
                    | TreeGrantRefusal::NotPlayer
                    | TreeGrantRefusal::NoArchetype => format!(
                        "Ability granter: your character has no archetype or record here; \
                         nothing granted, {} cooldown(s) cleared.",
                        cleared.abilities.len()
                    ),
                };
                (
                    GmAbilityChange::GrantAll,
                    Vec::new(),
                    Err((refusal.reason(), line)),
                )
            }
        },
        AbilityBulkChange::Reset => {
            let line = format!(
                "Ability reset: back to your starter abilities; {} cooldown(s) cleared.",
                cleared.abilities.len()
            );
            (GmAbilityChange::Reset, Vec::new(), Ok(line))
        }
    };

    let line = match outcome {
        Err((reason, line)) => {
            log_row(
                "nothing_sent",
                Some(reason),
                entity_id,
                &who,
                npc_entity_id,
                npc_name.as_deref(),
                chain_id,
                change,
                &[],
                &cleared,
            );
            send_line(entity_id, &line, tx).await;
            return;
        }
        Ok(line) => line,
    };
    let Some(player_id) = who.player_id else {
        // The plan resolved a player id for a grant; a reset needs it too.
        log_row(
            "nothing_sent",
            Some("caller_not_player"),
            entity_id,
            &who,
            npc_entity_id,
            npc_name.as_deref(),
            chain_id,
            change,
            &[],
            &cleared,
        );
        send_line(
            entity_id,
            "Ability granter: your character has no record here; nothing changed.",
            tx,
        )
        .await;
        return;
    };
    let msg = CellToBaseMsg::GmAbilityBulk(GmAbilityBulk {
        entity_id,
        player_id,
        account_id: who.account_id,
        change: wire_change,
        ability_ids: ability_ids.clone(),
    });
    if tx.send(msg).await.is_err() {
        log_row(
            "nothing_sent",
            Some("cell_to_base_closed"),
            entity_id,
            &who,
            npc_entity_id,
            npc_name.as_deref(),
            chain_id,
            change,
            &ability_ids,
            &cleared,
        );
        return;
    }
    log_row(
        "forwarded",
        None,
        entity_id,
        &who,
        npc_entity_id,
        npc_name.as_deref(),
        chain_id,
        change,
        &ability_ids,
        &cleared,
    );
    send_line(entity_id, &line, tx).await;
}

/// "Soldier", or "archetype 3" when the id is outside the enum.
fn archetype_label(archetype: i32) -> String {
    cimmeria_names::archetype_name(archetype)
        .map(str::to_string)
        .unwrap_or_else(|| format!("archetype {archetype}"))
}

/// The one row per click: who, at which NPC, and every ability id sent with
/// its name (Rule 6). `ability_names` lists `id:name` pairs; an id with no
/// name in the NameBook is listed bare.
#[allow(clippy::too_many_arguments)]
fn log_row(
    decision_outcome: &'static str,
    reason: Option<&'static str>,
    entity_id: u32,
    who: &cimmeria_entity::cell_entity::PlayerIdentity,
    npc_entity_id: Option<u32>,
    npc_name: Option<&str>,
    chain_id: i64,
    change: AbilityBulkChange,
    ability_ids: &[i32],
    cleared: &CooldownReset,
) {
    let book = cimmeria_names::book();
    let ability_names = ability_ids
        .iter()
        .map(|&id| match book.ability(id) {
            Some(name) => format!("{id}:{name}"),
            None => id.to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    tracing::info!(
        target: "content",
        event = EVENT,
        decision_outcome,
        reason,
        entity_id,
        entity_name = who.player_name,
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        npc_entity_id,
        npc_entity_name = npc_name,
        chain_id, // nt:id-only the executor holds no chain description
        change = change.as_str(),
        ability_count = ability_ids.len(),
        ability_ids = ?ability_ids,
        ability_names = %ability_names,
        cooldowns_cleared = cleared.abilities.len(),
        moniker_groups_cleared = cleared.moniker_groups,
        "ability granter used"
    );
}

/// One `SYSTEM` feedback line to the player.
async fn send_line(entity_id: u32, text: &str, tx: &mpsc::Sender<CellToBaseMsg>) {
    let args = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
    if tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_PLAYER_COMMUNICATION,
            args,
        })
        .await
        .is_err()
    {
        tracing::warn!(
            target: "content",
            event = "ability_granter_feedback_send_failed",
            reason = "base_channel_closed",
            entity_id, // nt:id-only shutdown path; the click's own row names the player
            "ability granter feedback line not queued"
        );
    }
}
