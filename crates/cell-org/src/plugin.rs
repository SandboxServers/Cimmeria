//! [`OrgPlugin`]: what squads and organization creation register with the
//! cell at startup.
//!
//! - Cell methods 8-19, the OrganizationMember interface, handled by the
//!   organization router ([`crate::cell::organization::dispatch`]): a squad
//!   id goes to the squad handlers, a Team or Command id is forwarded to the
//!   base, and the rest is answered.
//! - Cell method 94 `onOrganizationCreation`, handled by
//!   [`crate::cell::organization::creation::on_organization_creation`].
//! - [`EntityHookPoint::AfterDisconnectTradeCancel`]: a disconnecting squad
//!   member leaves with `Logout` (the squad is promoted or disbanded and
//!   their invites dropped), then their open registrar offer ends. Before
//!   the entity is torn down, so the remaining members' notice still names
//!   the departing entity.
//! - [`PlayerHookPoint::AfterInitPlayerState`]: a player entering a world
//!   (a first login or a gate arrival) gets their squad again, and an
//!   `onOrganizationLeft` owed while they were in transit is delivered.
//!
//! Each hook fires at the line the inline call occupied before the move
//! (`docs/architecture/plugin-architecture.md` §4.3), so the wire order is
//! unchanged.

use cimmeria_cell_world::cell::plugin::{
    BoxFuture, CellMethodCall, CellPlugin, CellPluginBuilder, EntityHookPoint, PlayerHookPoint,
};
use cimmeria_wire::cell::cell_methods::organization::{INVITE_RESPONSE, TRANSFER_CASH};
use cimmeria_wire::cell::cell_methods::player::constants::ORG_CREATION;
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::organization::{self, creation, squad};
use crate::cell::space_manager::SpaceManager;

/// Squads and organization creation, as a cell plugin.
#[derive(Debug, Clone, Copy, Default)]
pub struct OrgPlugin;

impl CellPlugin for OrgPlugin {
    fn name(&self) -> &'static str {
        "org"
    }

    fn build(&self, plugin: &mut CellPluginBuilder<'_>) {
        for index in INVITE_RESPONSE..=TRANSFER_CASH {
            plugin.cell_method(index, organization_member);
        }
        plugin
            .cell_method(ORG_CREATION, organization_creation)
            .entity_hook(EntityHookPoint::AfterDisconnectTradeCancel, on_disconnect)
            .player_hook(PlayerHookPoint::AfterInitPlayerState, on_world_entry);
    }
}

/// Cell methods 8-19: the organization router decodes the call and routes
/// it on the id it carries.
fn organization_member(call: CellMethodCall<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        // Every index this handler is registered for is in the router's
        // range, so it always handles the call.
        organization::dispatch(
            call.entity_id,
            call.method_index,
            call.args,
            call.tx,
            call.space_mgr,
        )
        .await;
    })
}

/// Cell method 94: the name for the player's open registrar offer.
fn organization_creation(call: CellMethodCall<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        creation::on_organization_creation(call.entity_id, call.args, call.tx, call.space_mgr)
            .await;
    })
}

/// The base's `DisconnectEntity`: the squad leave, then the offer, in the
/// order the inline calls had.
fn on_disconnect<'a>(
    entity_id: u32,
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    space_mgr: &'a mut SpaceManager,
) -> BoxFuture<'a, ()> {
    Box::pin(async move {
        squad::on_disconnect(entity_id, tx, space_mgr).await;
        creation::on_disconnect(entity_id, space_mgr);
    })
}

/// `InitPlayerState`, with the character id the base sent.
fn on_world_entry<'a>(
    entity_id: u32,
    player_id: i32,
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    space_mgr: &'a mut SpaceManager,
) -> BoxFuture<'a, ()> {
    Box::pin(async move {
        squad::on_world_entry(entity_id, player_id, tx, space_mgr).await;
    })
}
