//! [`DuelPlugin`]: what duels register with the cell at startup.
//!
//! - Cell method 102 `sendDuelResponse`, handled by
//!   [`crate::cell::duel::response`], and 103 `duelForfeit`, handled by
//!   [`crate::cell::duel::forfeit`].
//! - [`TickStage::AfterGateCrossing`]: the duel tick, which expires
//!   unanswered challenges, engages duels whose countdown has run out and
//!   runs the safety ends.
//! - [`EntityHookPoint::BeforeDisconnectTeardown`]: a disconnecting duelist
//!   loses (`EDUEL_DEFEAT_Connection`) while the entity still exists.
//! - [`EntityHookPoint::BeforeTravelSend`]: a teleported or gate-travelling
//!   duelist loses (`EDUEL_DEFEAT_Teleport`).
//! - [`DeathHookPoint::AfterPlayerThreatPurge`]: a duelist killed by anyone
//!   but the partner loses (`EDUEL_DEFEAT_Health`).
//!
//! On each leave path a challenge or countdown is withdrawn instead. Each
//! hook fires at the line the inline call occupied before the move
//! (`docs/architecture/plugin-architecture.md` §4.2), so the wire order is
//! unchanged.

use cimmeria_cell_world::cell::duel;
use cimmeria_cell_world::cell::plugin::{
    BoxFuture, CellMethodCall, CellPlugin, CellPluginBuilder, DeathHookPoint, EntityHookPoint,
    TickStage,
};
use cimmeria_wire::cell::cell_methods::player::constants::{DUEL_FORFEIT, SEND_DUEL_RESPONSE};
use tokio::sync::mpsc;

use crate::cell::duel::{forfeit, response, tick};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Duels, as a cell plugin.
#[derive(Debug, Clone, Copy, Default)]
pub struct DuelPlugin;

impl CellPlugin for DuelPlugin {
    fn name(&self) -> &'static str {
        "duel"
    }

    fn build(&self, plugin: &mut CellPluginBuilder<'_>) {
        plugin
            .cell_method(SEND_DUEL_RESPONSE, send_duel_response)
            .cell_method(DUEL_FORFEIT, duel_forfeit)
            .tick(TickStage::AfterGateCrossing, duel_tick)
            .entity_hook(EntityHookPoint::BeforeDisconnectTeardown, on_disconnect)
            .entity_hook(EntityHookPoint::BeforeTravelSend, on_travel)
            .death_hook(DeathHookPoint::AfterPlayerThreatPurge, on_death);
    }
}

/// Cell method 102: the one-byte answer to the challenge addressed to the
/// caller.
fn send_duel_response(call: CellMethodCall<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        response::handle(call.entity_id, call.args, call.tx, call.space_mgr).await;
    })
}

/// Cell method 103. No arguments: the caller's own engaged duel, or 880.
fn duel_forfeit(call: CellMethodCall<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        forfeit::handle(call.entity_id, call.tx, call.space_mgr).await;
    })
}

fn duel_tick<'a>(
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    space_mgr: &'a mut SpaceManager,
) -> BoxFuture<'a, ()> {
    Box::pin(async move {
        tick::run(tx, space_mgr).await;
    })
}

fn on_disconnect<'a>(
    entity_id: u32,
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    space_mgr: &'a mut SpaceManager,
) -> BoxFuture<'a, ()> {
    Box::pin(async move {
        duel::on_disconnect(tx, space_mgr, entity_id).await;
    })
}

fn on_travel<'a>(
    entity_id: u32,
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    space_mgr: &'a mut SpaceManager,
) -> BoxFuture<'a, ()> {
    Box::pin(async move {
        duel::on_travel(tx, space_mgr, entity_id).await;
    })
}

fn on_death<'a>(
    victim: u32,
    killer: u32,
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    space_mgr: &'a mut SpaceManager,
) -> BoxFuture<'a, ()> {
    Box::pin(async move {
        duel::on_death(tx, space_mgr, victim, killer).await;
    })
}
