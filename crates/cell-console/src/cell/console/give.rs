//! Player-grant console commands: `.givecash`, `.givexp`, and `.giveability`
//! (pets campaign PT-07, documented on [`give_ability`]).
//!
//! Both route through the same base-side grant sinks the native `gmGiveCash`/
//! `gmGiveXp` methods use (`crates/cell-console/src/cell/console/gm/give.rs`
//! -> `CellToBaseMsg::GrantCash`/`GrantXP`) — but unlike those caller-grants-
//! to-self native paths, `.givecash`/`.givexp` grant to a *selected* target
//! while the calling GM receives the feedback line. This caller/subject split
//! is exactly what `gm_feedback_to: Option<u32>` exists for (see
//! `CellToBaseMsg::GrantCash`/`GrantXP`'s doc comments): passing
//! `Some(caller_id)` here tells the base to send the definitive post-commit
//! feedback line to the caller, never to the target.
//!
//! No optimistic "requested" feedback is sent from here — the base sends the
//! real outcome once the DB write actually commits.
//!
//! Legacy reference: `deprecated/python/cell/commands/Player.py::giveCash`/
//! `giveExperience`.

use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `.givecash <amount>` — grant naquadah to the selected target.
///
/// Legacy `giveCash` has no lower bound on `amount`; the current native
/// `gmGiveCash` rejects `<= 0` (a no-op grant at best, a footgun for an
/// accidental balance decrease at worst) — D02 keeps that bound rather than
/// reproducing the legacy gap.
pub(super) async fn give_cash(
    caller_id: u32,
    target: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(amount) = super::parse_i32(caller_id, args, 0, "amount", tx).await else {
        return;
    };
    if amount <= 0 {
        send_gm_feedback(caller_id, "givecash: amount must be positive", tx).await;
        return;
    }
    let Some(player_id) = space_mgr.get_entity(target).and_then(|e| e.player_id) else {
        send_gm_feedback(caller_id, "givecash: target has no player id", tx).await;
        return;
    };
    tracing::info!(caller_id, target, player_id, amount, "GM .givecash");
    let _ = tx
        .send(CellToBaseMsg::GrantCash {
            entity_id: target,
            player_id,
            amount,
            gm_feedback_to: Some(caller_id),
        })
        .await;
}

/// `.givexp <amount>` — grant experience to the selected target.
///
/// `xp_amount` is `u64` on the wire message; `amount > 0` is confirmed before
/// the cast so a negative `i32` can't wrap into an absurd unsigned grant
/// (mirrors the native `gmGiveXp` ordering).
pub(super) async fn give_xp(
    caller_id: u32,
    target: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(amount) = super::parse_i32(caller_id, args, 0, "amount", tx).await else {
        return;
    };
    if amount <= 0 {
        send_gm_feedback(caller_id, "givexp: amount must be positive", tx).await;
        return;
    }
    if space_mgr
        .get_entity(target)
        .and_then(|e| e.player_id)
        .is_none()
    {
        send_gm_feedback(caller_id, "givexp: target has no player id", tx).await;
        return;
    }
    tracing::info!(caller_id, target, amount, "GM .givexp");
    let _ = tx
        .send(CellToBaseMsg::GrantXP {
            entity_id: target,
            xp_amount: amount as u64,
            gm_feedback_to: Some(caller_id),
        })
        .await;
}

/// `.giveability <abilityId>`: grant an ability and save it to the character
/// (pets campaign PT-07; UAT for summon abilities a tester's archetype, level
/// or tree progress cannot reach).
///
/// The subject is the selected target when it is a player in the caller's
/// space, otherwise the caller; the feedback names the fallback, so a GM who
/// meant to grant a tester never silently grants themselves. The grant goes
/// through the base (`GmGrantAbility`), which appends it to
/// `sgw_player.abilities` and answers with `GmAbilityGranted`, the mirror
/// that refreshes the hotbar. It is never a trainer purchase: no points and
/// no `trained_abilities`, so a respec keeps it. The base sends the
/// definitive line ("saved to the character") once the `UPDATE` commits.
pub(super) async fn give_ability(
    caller_id: u32,
    target_id: Option<u32>,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(ability_id) = super::parse_i32(caller_id, args, 0, "abilityId", tx).await else {
        return;
    };
    if !space_mgr.ability_defs.contains_key(&ability_id) {
        send_gm_feedback(
            caller_id,
            &format!(".giveability: no ability {ability_id} in resources.abilities"),
            tx,
        )
        .await;
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
        send_gm_feedback(caller_id, ".giveability: you have no player id", tx).await;
        return;
    };
    let Some(subject_entity) = space_mgr.get_entity(subject) else {
        send_gm_feedback(
            caller_id,
            &format!(".giveability: entity {subject} is gone"),
            tx,
        )
        .await;
        return;
    };
    let Some(player_id) = subject_entity.player_id else {
        send_gm_feedback(
            caller_id,
            &format!(".giveability: entity {subject} has no player id"),
            tx,
        )
        .await;
        return;
    };
    if subject_entity.abilities.has_ability(ability_id) {
        send_gm_feedback(
            caller_id,
            &format!(".giveability: entity {subject} already knows {ability_id}"),
            tx,
        )
        .await;
        return;
    }
    tracing::info!(
        caller_id,
        gm_player_id,
        subject,
        player_id,
        ability_id,
        "GM .giveability"
    );
    if !fallback_note.is_empty() {
        send_gm_feedback(caller_id, &format!(".giveability:{fallback_note}"), tx).await;
    }
    let _ = tx
        .send(CellToBaseMsg::GmGrantAbility {
            entity_id: subject,
            player_id,
            ability_id,
            gm_entity_id: caller_id,
            gm_player_id,
        })
        .await;
}
