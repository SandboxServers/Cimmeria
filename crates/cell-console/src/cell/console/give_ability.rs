//! `.giveability <abilityId>`: grant an ability and save it to the character
//! (pets campaign PT-07; UAT for summon abilities a tester's archetype, level
//! or tree progress cannot reach, such as 2826 Summon Straegis).
//!
//! The subject is the selected target when it is a player in the caller's
//! space, otherwise the caller; the feedback names the fallback, so a GM who
//! meant to grant a tester never silently grants themselves. The grant goes
//! through the base (`GmGrantAbility`), which appends it to
//! `sgw_player.abilities` and answers with `GmAbilityGranted`, the mirror
//! that refreshes the hotbar. It is never a trainer purchase: no points and
//! no `trained_abilities`, so a respec keeps it. The base sends the
//! definitive line ("saved to the character") once the `UPDATE` commits,
//! and logs whether it persisted.
//!
//! # Telemetry
//!
//! The info span `console.giveability` (Rule 1). Events use the console's
//! module-path target, like the rest of the `.`-console (exported by the
//! `cimmeria_cell_console=debug` row), and carry the caller's `account_id` /
//! `player_id` plus `subject_player_id` (Rule 5):
//!
//! - DEBUG `decision_outcome = refused` with `reason` = `bad_args` |
//!   `unknown_ability` | `caller_not_player` | `subject_gone` |
//!   `subject_not_player` | `already_known` (a GM triggers each at will);
//! - INFO `decision_outcome = forwarded` once the grant is sent to the base
//!   (`persisted = false`: the base's own line says whether it persisted);
//! - WARN `decision_outcome = send_failed` if the base channel is closed.

use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Log a refusal and answer the GM with `text`.
async fn refuse(
    caller_id: u32,
    reason: &'static str,
    ability_id: Option<i32>,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(caller_id);
    tracing::debug!(
        decision_outcome = "refused",
        reason,
        entity_id = caller_id,
        account_id = id.account_id,
        player_id = id.player_id,
        ability_id,
        "GM .giveability refused"
    );
    send_gm_feedback(caller_id, text, tx).await;
}

#[tracing::instrument(
    name = "console.giveability",
    level = "info",
    skip_all,
    fields(
        entity_id = caller_id,
        account_id = space_mgr.player_identity(caller_id).account_id,
        player_id = space_mgr.player_identity(caller_id).player_id,
    )
)]
pub(super) async fn give_ability(
    caller_id: u32,
    target_id: Option<u32>,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(ability_id) = args.first().and_then(|s| s.parse::<i32>().ok()) else {
        let text = ".giveability: abilityId must be an integer";
        refuse(caller_id, "bad_args", None, text, tx, space_mgr).await;
        return;
    };
    let ability = Some(ability_id);
    if !space_mgr.ability_defs.contains_key(&ability_id) {
        let text = format!(".giveability: no ability {ability_id} in resources.abilities");
        refuse(caller_id, "unknown_ability", ability, &text, tx, space_mgr).await;
        return;
    }
    // `resolve_target` already dropped a selection in another space.
    let selected_player =
        target_id.filter(|&t| space_mgr.get_entity(t).is_some_and(|e| e.is_player));
    let subject = selected_player.unwrap_or(caller_id);
    let fallback_note = match (target_id, selected_player) {
        (Some(t), None) => format!(" target {t} is not a player, so the grant is yours"),
        _ => String::new(),
    };
    let Some(gm_player_id) = space_mgr.get_entity(caller_id).and_then(|e| e.player_id) else {
        let text = ".giveability: you have no player id";
        refuse(caller_id, "caller_not_player", ability, text, tx, space_mgr).await;
        return;
    };
    let Some(subject_entity) = space_mgr.get_entity(subject) else {
        let text = format!(".giveability: entity {subject} is gone");
        refuse(caller_id, "subject_gone", ability, &text, tx, space_mgr).await;
        return;
    };
    let Some(player_id) = subject_entity.player_id else {
        let text = format!(".giveability: entity {subject} has no player id");
        refuse(
            caller_id,
            "subject_not_player",
            ability,
            &text,
            tx,
            space_mgr,
        )
        .await;
        return;
    };
    if subject_entity.abilities.has_ability(ability_id) {
        let text = format!(".giveability: entity {subject} already knows {ability_id}");
        refuse(caller_id, "already_known", ability, &text, tx, space_mgr).await;
        return;
    }
    let id = space_mgr.player_identity(caller_id);
    tracing::info!(
        decision_outcome = "forwarded",
        entity_id = caller_id,
        account_id = id.account_id,
        player_id = id.player_id,
        subject_entity_id = subject,
        subject_player_id = player_id,
        ability_id,
        fell_back_to_caller = !fallback_note.is_empty(),
        persisted = false,
        "GM .giveability forwarded to the base"
    );
    if !fallback_note.is_empty() {
        send_gm_feedback(caller_id, &format!(".giveability:{fallback_note}"), tx).await;
    }
    if let Err(e) = tx
        .send(CellToBaseMsg::GmGrantAbility {
            entity_id: subject,
            player_id,
            ability_id,
            gm_entity_id: caller_id,
            gm_player_id,
        })
        .await
    {
        tracing::warn!(
            decision_outcome = "send_failed",
            reason = "cell_to_base_closed",
            entity_id = caller_id,
            account_id = id.account_id,
            player_id = id.player_id,
            subject_entity_id = subject,
            subject_player_id = player_id,
            ability_id,
            error = %e,
            "GM .giveability: grant not sent to the base; nothing persisted"
        );
    }
}
