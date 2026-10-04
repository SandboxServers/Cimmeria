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
//!
//! The native `gmGiveAbility` (SGWGmPlayer 136, `gm::abilities`) shares
//! [`plan_grant`] and [`send_grant`] with the subject fixed to the caller
//! (its def has no target argument) and writes the GM `gm_command` row
//! instead of these.

use cimmeria_entity::name_intern::intern_opt;
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
        entity_name = id.player_name,
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        ability_id,
        ability_name = ability_id.and_then(|a| intern_opt(cimmeria_names::book().ability(a))),
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
    let grant = match plan_grant(".giveability", caller_id, target_id, ability_id, space_mgr) {
        Ok(grant) => grant,
        Err(refusal) => {
            let reason = refusal.reason;
            refuse(caller_id, reason, ability, &refusal.text, tx, space_mgr).await;
            return;
        }
    };
    let id = space_mgr.player_identity(caller_id);
    let subject_name = space_mgr.entity_names(grant.subject).entity_name;
    tracing::info!(
        decision_outcome = "forwarded",
        entity_id = caller_id,
        entity_name = id.player_name,
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        subject_entity_id = grant.subject,
        subject_player_id = grant.player_id,
        subject_entity_name = subject_name,
        subject_player_name = subject_name,
        ability_id,
        ability_name = cimmeria_names::book().ability(ability_id),
        fell_back_to_caller = grant.fallback_note.is_some(),
        persisted = false,
        "GM .giveability forwarded to the base"
    );
    if let Some(note) = &grant.fallback_note {
        send_gm_feedback(caller_id, &format!(".giveability:{note}"), tx).await;
    }
    if let Err(e) = send_grant(&grant, caller_id, tx).await {
        tracing::warn!(
            decision_outcome = "send_failed",
            reason = "cell_to_base_closed",
            entity_id = caller_id,
            entity_name = id.player_name,
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            subject_entity_id = grant.subject,
            subject_player_id = grant.player_id,
            subject_entity_name = subject_name,
        subject_player_name = subject_name,
            ability_id,
            ability_name = cimmeria_names::book().ability(ability_id),
            error = %e,
            "GM .giveability: grant not sent to the base; nothing persisted"
        );
    }
}

/// A grant the cell checked and will forward to the base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PlannedGrant {
    pub(super) ability_id: i32,
    /// The subject's entity and character.
    pub(super) subject: u32,
    pub(super) player_id: i32,
    /// The GM's character.
    pub(super) gm_player_id: i32,
    /// Set when a selected non-player made the grant fall back to the
    /// caller; the text after the command's `:`.
    pub(super) fallback_note: Option<String>,
}

/// Why [`plan_grant`] refused, and the GM's feedback line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GrantRefusal {
    pub(super) reason: &'static str,
    pub(super) text: String,
}

/// Check one GM ability grant. `cmd` prefixes the feedback (`.giveability`
/// or `gmGiveAbility`). The subject is `target_id` when that is a player
/// in the caller's space, otherwise the caller.
pub(super) fn plan_grant(
    cmd: &str,
    caller_id: u32,
    target_id: Option<u32>,
    ability_id: i32,
    space_mgr: &SpaceManager,
) -> Result<PlannedGrant, GrantRefusal> {
    let refuse = |reason, text: String| Err(GrantRefusal { reason, text });
    if !space_mgr.ability_defs.contains_key(&ability_id) {
        let text = format!("{cmd}: no ability {ability_id} in resources.abilities");
        return refuse("unknown_ability", text);
    }
    // `resolve_target` already dropped a selection in another space.
    let selected_player =
        target_id.filter(|&t| space_mgr.get_entity(t).is_some_and(|e| e.is_player));
    let subject = selected_player.unwrap_or(caller_id);
    let fallback_note = match (target_id, selected_player) {
        (Some(t), None) => Some(format!(
            " target {t} is not a player, so the grant is yours"
        )),
        _ => None,
    };
    let Some(gm_player_id) = space_mgr.get_entity(caller_id).and_then(|e| e.player_id) else {
        return refuse("caller_not_player", format!("{cmd}: you have no player id"));
    };
    let Some(subject_entity) = space_mgr.get_entity(subject) else {
        return refuse("subject_gone", format!("{cmd}: entity {subject} is gone"));
    };
    let Some(player_id) = subject_entity.player_id else {
        let text = format!("{cmd}: entity {subject} has no player id");
        return refuse("subject_not_player", text);
    };
    if subject_entity.abilities.has_ability(ability_id) {
        let text = format!("{cmd}: entity {subject} already knows {ability_id}");
        return refuse("already_known", text);
    }
    Ok(PlannedGrant {
        ability_id,
        subject,
        player_id,
        gm_player_id,
        fallback_note,
    })
}

/// Forward a planned grant to the base (`GmGrantAbility`). The base
/// persists it, answers with `GmAbilityGranted` (the hotbar burst) and
/// sends the GM the definitive line. `Err` only when the base channel is
/// closed (shutdown).
pub(super) async fn send_grant(
    grant: &PlannedGrant,
    caller_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> Result<(), String> {
    tx.send(CellToBaseMsg::GmGrantAbility {
        entity_id: grant.subject,
        player_id: grant.player_id,
        ability_id: grant.ability_id,
        gm_entity_id: caller_id,
        gm_player_id: grant.gm_player_id,
    })
    .await
    // The message itself is dropped: only the reason is worth logging.
    .map_err(|e| e.to_string())
}
