//! `.cooldowns [reset [abilityId]]` — the caller's cooldowns.
//!
//! - `.cooldowns` lists the running ability cooldowns.
//! - `.cooldowns reset` clears every ability and moniker cooldown.
//! - `.cooldowns reset <abilityId>` clears that ability's cooldown and its
//!   moniker groups'.
//!
//! **The client is told.** The hotbar sweep is the client's own timer, set by
//! the launch's `onTimerUpdate(TIMER_ABILITY_COOLDOWN)`; clearing only the
//! server's map would leave the button greyed out for the rest of the
//! sweep, and a press during it never leaves the client. So each cleared
//! ability gets the same zero-length timer the warmup interrupt sends when
//! it refunds a cooldown (`id = ability_id, type = 2` (`TIMER_ABILITY_COOLDOWN`),
//! `source = caster, secondary = 0, total = 0, complete = 0`). The one-ability form always
//! sends it, even with nothing running server-side: that is the form to use
//! when the two sides disagree.
//!
//! Caller only: a GM resets their own lab character, never another player.

use std::time::Instant;

use cimmeria_entity::abilities::{serialize_timer_update, TIMER_ABILITY_COOLDOWN};
use cimmeria_entity::name_intern::intern_opt;
use tokio::sync::mpsc;

use crate::cell::abilities::send_timer_update;
use crate::cell::console::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

const USAGE: &str = ".cooldowns: usage .cooldowns | .cooldowns reset [abilityId]";

/// The `onTimerUpdate` arguments that clear `ability_id`'s cooldown sweep on
/// `caster_id`'s client.
pub(crate) fn clear_cooldown_timer(ability_id: i32, caster_id: u32) -> Vec<u8> {
    serialize_timer_update(
        ability_id,
        TIMER_ABILITY_COOLDOWN,
        caster_id as i32,
        0,
        0.0,
        0.0,
    )
}

pub(super) async fn run(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    match args {
        [] => list(caller_id, tx, space_mgr).await,
        ["reset"] => reset_all(caller_id, tx, space_mgr).await,
        ["reset", id] => match id.parse::<i32>() {
            Ok(ability_id) if ability_id > 0 => {
                reset_one(caller_id, ability_id, tx, space_mgr).await
            }
            _ => {
                send_gm_feedback(
                    caller_id,
                    &format!(".cooldowns reset: abilityId must be a positive integer (got {id})"),
                    tx,
                )
                .await
            }
        },
        _ => send_gm_feedback(caller_id, USAGE, tx).await,
    }
}

async fn list(caller_id: u32, tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &SpaceManager) {
    let Some(s) = space_mgr.ability_state(caller_id) else {
        return;
    };
    let line = if s.cooldowns.is_empty() {
        "cooldowns: none running".to_string()
    } else {
        let items: Vec<String> = s
            .cooldowns
            .iter()
            .map(|c| {
                format!(
                    "{} {:.1}s of {:.1}s",
                    c.ability_id, c.remaining_secs, c.total_secs
                )
            })
            .collect();
        format!("cooldowns: {}", items.join("; "))
    };
    send_gm_feedback(caller_id, &line, tx).await;
}

async fn reset_all(caller_id: u32, tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    let now = Instant::now();
    let Some(caller) = space_mgr.get_entity_mut(caller_id) else {
        return;
    };
    let mut running: Vec<i32> = caller
        .abilities
        .ability_cooldowns()
        .filter(|(_, c)| c.expires_at > now)
        .map(|(id, _)| id)
        .collect();
    running.sort_unstable();
    let monikers = caller
        .abilities
        .moniker_cooldowns()
        .filter(|(_, c)| c.expires_at > now)
        .count();
    caller.abilities.clear_all_cooldowns();
    for &ability_id in &running {
        send_timer_update(
            caller_id,
            clear_cooldown_timer(ability_id, caller_id),
            tx,
            space_mgr,
        )
        .await;
    }
    log_reset(space_mgr, caller_id, None, &running, monikers);
    let line = if running.is_empty() && monikers == 0 {
        "cooldowns reset: none were running".to_string()
    } else {
        format!(
            "cooldowns reset: {} ability cooldown(s) cleared ({}), {} moniker group(s); your client was sent the clears",
            running.len(),
            running
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            monikers
        )
    };
    send_gm_feedback(caller_id, &line, tx).await;
}

async fn reset_one(
    caller_id: u32,
    ability_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let moniker_ids = space_mgr.ability_moniker_ids(ability_id);
    let Some(caller) = space_mgr.get_entity_mut(caller_id) else {
        return;
    };
    let was_running = caller.abilities.is_on_cooldown(ability_id);
    caller.abilities.clear_ability_cooldown(ability_id);
    let monikers = moniker_ids
        .iter()
        .filter(|&&m| {
            let running = caller.abilities.is_moniker_on_cooldown(m);
            caller.abilities.clear_moniker_cooldown(m);
            running
        })
        .count();
    send_timer_update(
        caller_id,
        clear_cooldown_timer(ability_id, caller_id),
        tx,
        space_mgr,
    )
    .await;
    let cleared: &[i32] = if was_running { &[ability_id] } else { &[] };
    log_reset(space_mgr, caller_id, Some(ability_id), cleared, monikers);
    let line = format!(
        "cooldowns reset {ability_id}: {}, {monikers} moniker group(s) cleared; your client was sent the clear",
        if was_running { "was running" } else { "was not running server-side" }
    );
    send_gm_feedback(caller_id, &line, tx).await;
}

fn log_reset(
    space_mgr: &SpaceManager,
    caller_id: u32,
    requested: Option<i32>,
    cleared: &[i32],
    monikers: usize,
) {
    let who = space_mgr.player_identity(caller_id);
    tracing::info!(
        target: "abilities.gm",
        event = "cooldowns_reset",
        entity_id = caller_id,
        entity_name = who.player_name,
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        requested_ability_id = requested,
        requested_ability_name = requested.and_then(|a| intern_opt(cimmeria_names::book().ability(a))),
        cleared = ?cleared,
        cleared_count = cleared.len(),
        moniker_groups = monikers,
        "GM cleared cooldowns and sent the client the clear timers",
    );
}
