//! `.giveammo <ammo type> <rounds>`: grant rounds of a special ammo type as
//! bag stacks (ammo campaign AM-06, issue #1026).
//!
//! The client's native `/gmgiveammo` has no server receiver: the
//! `Event_NetOut_GiveAmmo` event is client-side only, no `.def` declares a
//! `GiveAmmo` cell or base method, and its byte layout is unrecovered
//! (`docs/reverse-engineering/findings/ammo-system.md` § Q1). So this ships
//! as a `.`-console command (`.gmgiveammo` is an alias, for testers who type
//! the native name with a dot).
//!
//! The type is an `EAmmoType` ordinal (`3`), its label (`Bullet_Hollow_Point`,
//! any case), the label without its family when that is unique
//! (`hollow_point`, `hollowpoint`, `hp`, `ap`), or the reserve item id
//! (`9001`). Default ammo is refused: it is free and has no reserve item.
//! The reserve item comes from `ammo_item_types` through
//! `SpaceManager::ammo_catalog`, never a hardcoded id.
//!
//! The recipient is the selected player, else the caller. The cell forwards
//! `CellToBaseMsg::GmGiveAmmo`; the base grants through
//! `AmmoReserve::return_rounds` (capped stacks, remainder reported) and
//! sends the definitive line, so there is no optimistic line here.
//!
//! # Telemetry
//!
//! Target `ammo`, event `gm_give_ammo`: every refusal here is a WARN with a
//! `reason` from `ammo_telemetry::reasons` and the caller's `account_id` /
//! `player_id`; the forward is DEBUG; the base logs the INFO grant.

use cimmeria_entity::ammo_telemetry::{events, reasons};
use cimmeria_entity::ammo_type::{self, LABELS};
use tokio::sync::mpsc;

use super::feedback::send_gm_feedback;
use crate::cell::messages::{CellToBaseMsg, GmGiveAmmo};
use crate::cell::space_manager::SpaceManager;

/// Most rounds one `.giveammo` grants; a larger count is clamped, with a
/// line saying so. Ten full stacks at the seeded 500-round cap.
pub(crate) const MAX_ROUNDS: i32 = 5000;

/// Short names testers type for the first two families (D-AM04).
const ALIASES: &[(&str, i32)] = &[
    ("hp", ammo_type::BULLET_HOLLOW_POINT),
    ("ap", ammo_type::BULLET_ARMOR_PIERCING),
];

/// Lowercase, with `_`, `-` and spaces removed.
fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '_' | '-' | ' '))
        .flat_map(char::to_lowercase)
        .collect()
}

/// Resolve `.giveammo`'s type argument to an `EAmmoType` ordinal. `Err` is
/// the GM-facing reason.
pub(crate) fn parse_ammo_type(
    arg: &str,
    item_to_type: impl Fn(i32) -> Option<i32>,
) -> Result<i32, String> {
    if let Ok(n) = arg.parse::<i32>() {
        if ammo_type::label(n).is_some() {
            return Ok(n);
        }
        return item_to_type(n).ok_or_else(|| {
            format!("{n} is neither an ammo type (0-23) nor an ammo item id (see .help giveammo)")
        });
    }
    let want = normalize(arg);
    if let Some(&(_, t)) = ALIASES.iter().find(|(a, _)| *a == want) {
        return Ok(t);
    }
    if let Some(i) = LABELS.iter().position(|l| normalize(l) == want) {
        return Ok(i as i32);
    }
    // Without the family prefix: `hollowpoint` -> Bullet_Hollow_Point, but
    // `emp` is both Bullet_EMP and Dart_EMP.
    let hits: Vec<usize> = LABELS
        .iter()
        .enumerate()
        .filter(|(_, l)| {
            let n = normalize(l);
            ["bullet", "dagger", "dart"]
                .iter()
                .any(|f| n.strip_prefix(f) == Some(want.as_str()))
        })
        .map(|(i, _)| i)
        .collect();
    match hits.as_slice() {
        [one] => Ok(*one as i32),
        [] => Err(format!("no ammo type named '{arg}'")),
        many => Err(format!(
            "'{arg}' is ambiguous: {}",
            many.iter()
                .map(|&i| LABELS[i])
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// One refusal: a WARN on `ammo` and the GM's line.
async fn refuse(
    caller_id: u32,
    reason: &'static str,
    ammo_type: Option<i32>,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(caller_id);
    tracing::warn!(
        target: "ammo",
        event = events::GM_GIVE_AMMO,
        decision_outcome = "refused",
        reason,
        entity_id = caller_id,
        entity_name = id.player_name,
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        ammo_type,
        "gm_give_ammo refused at the console; nothing sent to the base"
    );
    send_gm_feedback(caller_id, &format!(".giveammo: {text}"), tx).await;
}

/// `.giveammo <type> <rounds>`.
#[tracing::instrument(
    name = "ammo.gm_give",
    level = "info",
    skip_all,
    fields(
        entity_id = caller_id,
        account_id = space_mgr.player_identity(caller_id).account_id,
        player_id = space_mgr.player_identity(caller_id).player_id,
    )
)]
pub(crate) async fn give_ammo(
    caller_id: u32,
    target_id: Option<u32>,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    const USAGE: &str =
        "usage .giveammo <ammo type name|id> <rounds>, e.g. .giveammo hollowpoint 500";
    let (Some(type_arg), Some(qty_arg)) = (args.first(), args.get(1)) else {
        refuse(caller_id, reasons::BAD_ARGS, None, USAGE, tx, space_mgr).await;
        return;
    };
    let ammo_type =
        match parse_ammo_type(type_arg, |i| space_mgr.ammo_catalog.ammo_type_for_item(i)) {
            Ok(t) => t,
            Err(why) => {
                let text = format!("{why}; {USAGE}");
                refuse(
                    caller_id,
                    reasons::UNKNOWN_AMMO_TYPE,
                    None,
                    &text,
                    tx,
                    space_mgr,
                )
                .await;
                return;
            }
        };
    let name = ammo_type::label(ammo_type).unwrap_or("?");
    let at = Some(ammo_type);
    if !ammo_type::is_special(ammo_type) {
        let text = format!("{name} is free default ammo; there is nothing to grant");
        refuse(
            caller_id,
            reasons::NOT_SPECIAL_AMMO,
            at,
            &text,
            tx,
            space_mgr,
        )
        .await;
        return;
    }
    let Some(item_id) = space_mgr.ammo_catalog.item_id_for(ammo_type) else {
        let text = format!("{name} has no reserve item in ammo_item_types");
        refuse(
            caller_id,
            reasons::NO_RESERVE_ITEM,
            at,
            &text,
            tx,
            space_mgr,
        )
        .await;
        return;
    };
    let rounds = match qty_arg.parse::<i32>() {
        Ok(n) if n > 0 => n,
        _ => {
            let text = format!("rounds must be a positive whole number, got '{qty_arg}'");
            refuse(caller_id, reasons::BAD_QUANTITY, at, &text, tx, space_mgr).await;
            return;
        }
    };

    // `resolve_target` already dropped a selection outside the caller's
    // space or view.
    let selected = target_id.filter(|&t| space_mgr.get_entity(t).is_some_and(|e| e.is_player));
    let subject = selected.unwrap_or(caller_id);
    let caller = space_mgr.player_identity(caller_id);
    let Some(gm_player_id) = caller.player_id else {
        let text = "you have no character id";
        refuse(caller_id, reasons::NOT_A_PLAYER, at, text, tx, space_mgr).await;
        return;
    };
    let Some(player_id) = space_mgr.get_entity(subject).and_then(|e| e.player_id) else {
        let text = format!("entity {subject} has no character id");
        refuse(caller_id, reasons::NOT_A_PLAYER, at, &text, tx, space_mgr).await;
        return;
    };

    let mut notes = Vec::new();
    if let (Some(t), None) = (target_id, selected) {
        notes.push(format!(
            "target {t} is not a player, so the rounds are yours"
        ));
    }
    let granted = rounds.min(MAX_ROUNDS);
    if granted < rounds {
        notes.push(format!("{rounds} rounds clamped to {MAX_ROUNDS}"));
    }
    if !notes.is_empty() {
        send_gm_feedback(caller_id, &format!(".giveammo: {}", notes.join("; ")), tx).await;
    }

    tracing::debug!(
        target: "ammo",
        event = events::GM_GIVE_AMMO,
        decision_outcome = "forwarded",
        entity_id = caller_id,
        entity_name = caller.player_name,
        account_id = caller.account_id,
        account_name = caller.account_name,
        player_id = gm_player_id,
        player_name = caller.player_name,
        subject_entity_id = subject,
        subject_entity_name = space_mgr.entity_label(subject),
        subject_player_id = player_id,
        subject_player_name = space_mgr.player_identity(subject).player_name,
        item_id,
        item_name = cimmeria_names::book().item(item_id),
        ammo_type,
        quantity = granted,
        requested = rounds,
        "gm_give_ammo forwarded to the base",
    );
    let msg = GmGiveAmmo {
        entity_id: subject,
        player_id,
        gm_entity_id: caller_id,
        gm_player_id,
        gm_account_id: caller.account_id,
        ammo_type,
        item_id,
        rounds: granted,
    };
    if let Err(e) = tx.send(CellToBaseMsg::GmGiveAmmo(msg)).await {
        tracing::warn!(
            target: "ammo",
            event = events::GM_GIVE_AMMO,
            decision_outcome = "send_failed",
            reason = "cell_to_base_closed",
            entity_id = caller_id,
            entity_name = caller.player_name,
            account_id = caller.account_id,
            account_name = caller.account_name,
            player_id = gm_player_id,
            player_name = caller.player_name,
            subject_player_id = player_id,
            subject_player_name = space_mgr.player_identity(subject).player_name,
            item_id,
            item_name = cimmeria_names::book().item(item_id),
            ammo_type,
            quantity = granted,
            error = %e,
            "gm_give_ammo: not sent to the base; nothing granted",
        );
    }
}
