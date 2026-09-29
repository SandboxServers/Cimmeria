//! `.infiniteammo [on|off]`: the GM infinite-ammo switch, `bInfiniteAmmo`
//! (ammo campaign AM-06, issue #1026; D-AM09).
//!
//! Like `/gmgiveammo`, the native `/gmsetinfiniteammo` never reaches a
//! server handler (no `.def` method), so this is a `.`-console command;
//! `.gmsetinfiniteammo` is an alias.
//!
//! D-AM09: the switch frees only the reserve. A reload of special ammo draws
//! nothing from the bags, but the clip still empties and needs a reload.
//! AM-02's reload path reads it (`cimmeria_entity::ammo_infinite::is_on`).
//! It is keyed by character and holds across zones and relogs until the
//! server restarts. The subject is the selected player, else the caller; a
//! bare `.infiniteammo` reports the switch without changing it. Every use
//! answers on the feedback channel, a bad argument included.

use cimmeria_entity::ammo_telemetry::{events, reasons};
use cimmeria_entity::{ammo_feature, ammo_infinite};
use tokio::sync::mpsc;

use super::feedback::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `.infiniteammo`, `.infiniteammo on`, `.infiniteammo off`.
pub(crate) async fn set_infinite_ammo(
    caller_id: u32,
    target_id: Option<u32>,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let caller = space_mgr.player_identity(caller_id);
    let want = match args.first() {
        None => None,
        Some(a) => match ammo_feature::parse_flag(a) {
            Some(on) => Some(on),
            None => {
                tracing::warn!(
                    target: "ammo",
                    event = events::GM_INFINITE_AMMO_TOGGLED,
                    decision_outcome = "refused",
                    reason = reasons::BAD_ARGS,
                    entity_id = caller_id,
                    account_id = caller.account_id,
                    player_id = caller.player_id,
                    "infinite ammo: bad argument; nothing changed"
                );
                send_gm_feedback(
                    caller_id,
                    &format!("infiniteammo: expected 'on' or 'off', got '{a}' -- nothing changed"),
                    tx,
                )
                .await;
                return;
            }
        },
    };
    let selected = target_id.filter(|&t| space_mgr.get_entity(t).is_some_and(|e| e.is_player));
    let subject = selected.unwrap_or(caller_id);
    let Some(player_id) = space_mgr.get_entity(subject).and_then(|e| e.player_id) else {
        tracing::warn!(
            target: "ammo",
            event = events::GM_INFINITE_AMMO_TOGGLED,
            decision_outcome = "refused",
            reason = reasons::NOT_A_PLAYER,
            entity_id = caller_id,
            account_id = caller.account_id,
            player_id = caller.player_id,
            subject_entity_id = subject,
            "infinite ammo: no character id; nothing changed"
        );
        send_gm_feedback(
            caller_id,
            &format!("infiniteammo: entity {subject} has no character id -- nothing changed"),
            tx,
        )
        .await;
        return;
    };

    let changed = want.is_some_and(|on| ammo_infinite::set(player_id, on));
    let on = ammo_infinite::is_on(player_id);
    if want.is_some() {
        tracing::info!(
            target: "ammo",
            event = events::GM_INFINITE_AMMO_TOGGLED,
            decision_outcome = "set",
            entity_id = caller_id,
            account_id = caller.account_id,
            player_id = caller.player_id,
            subject_entity_id = subject,
            subject_player_id = player_id,
            on,
            changed,
            "infinite ammo switch set"
        );
    }
    let prefix = match (want.is_some(), changed) {
        (false, _) => "infiniteammo",
        (true, true) => "infiniteammo set",
        (true, false) => "infiniteammo unchanged",
    };
    let who = if subject == caller_id {
        "you".to_string()
    } else {
        format!("entity {subject}")
    };
    let state = if on {
        "ON -- special-ammo reloads take nothing from the bags; the clip still empties and needs a reload"
    } else {
        "OFF -- special-ammo reloads draw from the bags"
    };
    let mut line = format!("{prefix}: {state} (for {who})");
    if !ammo_feature::finite_special() {
        line.push_str(
            "; finite special ammo is off on this server, so every reload is free anyway",
        );
    }
    if let (Some(t), None) = (target_id, selected) {
        line.push_str(&format!("; target {t} is not a player, so this is yours"));
    }
    send_gm_feedback(caller_id, &line, tx).await;
}
