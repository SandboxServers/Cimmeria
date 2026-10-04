//! Clearing every cooldown a player has running, server and client side.
//!
//! Shared by the GM `.cooldowns reset` command and the Debug Area ability
//! granter (`gm_ability_bulk` content action), so both clear the same state
//! and send the client the same frames.
//!
//! **The client is told.** The hotbar sweep is the client's own timer, set by
//! the launch's `onTimerUpdate(TIMER_ABILITY_COOLDOWN)`; clearing only the
//! server's map would leave the button greyed out for the rest of the
//! sweep, and a press during it never leaves the client. So each cleared
//! ability gets the same zero-length timer the warmup interrupt sends when
//! it refunds a cooldown (`id = ability_id, type = 2`
//! (`TIMER_ABILITY_COOLDOWN`), `source = caster, secondary = 0, total = 0,
//! complete = 0`).

use std::time::Instant;

use cimmeria_entity::abilities::{serialize_timer_update, TIMER_ABILITY_COOLDOWN};
use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::timer_update::send_timer_update;

/// The `onTimerUpdate` arguments that clear `ability_id`'s cooldown sweep on
/// `caster_id`'s client.
pub fn clear_cooldown_timer(ability_id: i32, caster_id: u32) -> Vec<u8> {
    serialize_timer_update(
        ability_id,
        TIMER_ABILITY_COOLDOWN,
        caster_id as i32,
        0,
        0.0,
        0.0,
    )
}

/// What [`reset_all_cooldowns`] cleared.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CooldownReset {
    /// The abilities whose cooldown was still running, ascending. Each was
    /// sent its clear timer.
    pub abilities: Vec<i32>,
    /// How many moniker-group cooldowns were still running.
    pub moniker_groups: usize,
}

/// Clear every ability and moniker cooldown `entity_id` has, and send its
/// client the clear timer for each ability that was still running. `None`
/// when the entity is gone.
pub async fn reset_all_cooldowns(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Option<CooldownReset> {
    let now = Instant::now();
    let entity = space_mgr.get_entity_mut(entity_id)?;
    let mut abilities: Vec<i32> = entity
        .abilities
        .ability_cooldowns()
        .filter(|(_, c)| c.expires_at > now)
        .map(|(id, _)| id)
        .collect();
    abilities.sort_unstable();
    let moniker_groups = entity
        .abilities
        .moniker_cooldowns()
        .filter(|(_, c)| c.expires_at > now)
        .count();
    entity.abilities.clear_all_cooldowns();
    for &ability_id in &abilities {
        send_timer_update(
            entity_id,
            clear_cooldown_timer(ability_id, entity_id),
            tx,
            space_mgr,
        )
        .await;
    }
    Some(CooldownReset {
        abilities,
        moniker_groups,
    })
}
