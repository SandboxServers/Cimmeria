//! `.aggro [on|off]` — the GM's own proximity-aggro switch (NA13, D-NA02).
//!
//! Mobs aggro onto GMs by default: the owner tests on a GM account and needs
//! to see what a player sees. `.aggro off` makes the Idle auto-aggro scan pass
//! the caller over (`reason=gm_ignored`), for walking a zone without pulling
//! it. It is server-side on purpose: the client's `ghost` / noclip never
//! reaches the server (audit A8), so it cannot be the switch.
//!
//! Scope: only proximity aggro. Shooting a mob, or a content chain's
//! `generate_threat`, still engages the GM, and a mob already fighting keeps
//! fighting. The switch is keyed by character (`SpaceManager::gm_aggro_off`),
//! so it survives zone changes and relogs until the server restarts, and it
//! is ignored if the character loses GM access.
//!
//! New in the Rust server; the legacy python console had no equivalent.

use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `.aggro` reports the switch; `.aggro on` / `.aggro off` set it. Every use
/// answers on the feedback channel, including a bad argument.
pub(super) async fn toggle(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(character_id) = space_mgr.get_entity(caller_id).and_then(|e| e.player_id) else {
        send_gm_feedback(
            caller_id,
            "aggro: no character id on this entity -- nothing changed",
            tx,
        )
        .await;
        return;
    };
    let want_off = match args.first().map(|a| a.to_ascii_lowercase()) {
        None => None,
        Some(a) if a == "off" => Some(true),
        Some(a) if a == "on" => Some(false),
        Some(other) => {
            send_gm_feedback(
                caller_id,
                &format!("aggro: expected 'on' or 'off', got '{other}' -- nothing changed"),
                tx,
            )
            .await;
            return;
        }
    };
    let changed = match want_off {
        Some(true) => space_mgr.gm_aggro_off.insert(character_id),
        Some(false) => space_mgr.gm_aggro_off.remove(&character_id),
        None => false,
    };
    let off = space_mgr.gm_aggro_off.contains(&character_id);
    if want_off.is_some() {
        tracing::info!(
            target: "npc_ai.aggro",
            event = "gm_toggle",
            entity_id = caller_id,
            player_id = character_id,
            aggro_off = off,
            changed,
            "GM proximity-aggro switch set"
        );
    }
    let state = if off {
        "OFF -- idle mobs will not notice you (damage and scripted threat still engage)"
    } else {
        "ON -- idle hostile mobs aggro on you like any player"
    };
    let prefix = match (want_off.is_some(), changed) {
        (false, _) => "aggro",
        (true, true) => "aggro set",
        (true, false) => "aggro unchanged",
    };
    send_gm_feedback(caller_id, &format!("{prefix}: proximity aggro {state}"), tx).await;
}
