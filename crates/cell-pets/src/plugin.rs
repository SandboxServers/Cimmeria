//! [`PetsPlugin`]: what pets register with the cell at startup.
//!
//! - Cell methods 88 `petInvokeAbility`, 89 `petAbilityToggle` and 90
//!   `petChangeStance`, handled by [`crate::cell::cell_methods::player::pet`].
//! - [`TickStage::AfterRingTransport`]: `pet_owner_sweep`, which despawns
//!   pets whose owner is gone, dead or in another space.
//! - [`TickStage::AfterStatBuffs`]: `pet_arrival_tick`, the summon's arrival
//!   VFX once the owner witnesses the pet (after the AoI tick, so it follows
//!   the pet's `CREATE_ENTITY`).
//! - [`EntityHookPoint::BeforeBaseDestroy`]: `on_owner_left`, so the base's
//!   `DestroyEntity` for an owner despawns the pet visibly before the owner
//!   goes.
//!
//! Each hook fires at the line the inline call occupied before PT-01..PT-08
//! moved here (`docs/architecture/plugin-architecture.md` §4.1), so the wire
//! order is unchanged.

use cimmeria_cell_world::cell::pets;
use cimmeria_cell_world::cell::plugin::{
    BoxFuture, CellMethodCall, CellPlugin, CellPluginBuilder, EntityHookPoint, TickStage,
};
use tokio::sync::mpsc;

use crate::cell::cell_methods::player::constants::{
    PET_ABILITY_TOGGLE, PET_CHANGE_STANCE, PET_INVOKE_ABILITY,
};
use crate::cell::cell_methods::player::pet;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// The pets feature, as a cell plugin.
#[derive(Debug, Clone, Copy, Default)]
pub struct PetsPlugin;

impl CellPlugin for PetsPlugin {
    fn name(&self) -> &'static str {
        "pets"
    }

    fn build(&self, plugin: &mut CellPluginBuilder<'_>) {
        plugin
            .cell_method(PET_INVOKE_ABILITY, pet_command)
            .cell_method(PET_ABILITY_TOGGLE, pet_command)
            .cell_method(PET_CHANGE_STANCE, pet_command)
            .tick(TickStage::AfterRingTransport, owner_sweep)
            .tick(TickStage::AfterStatBuffs, arrival_tick)
            .entity_hook(EntityHookPoint::BeforeBaseDestroy, owner_destroyed);
    }
}

/// Cell methods 88-90. `pet::dispatch` returns `true` for all three.
fn pet_command(call: CellMethodCall<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        pet::dispatch(
            call.entity_id,
            call.method_index,
            call.args,
            call.tx,
            call.space_mgr,
            call.engine,
        )
        .await;
    })
}

fn owner_sweep<'a>(
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    space_mgr: &'a mut SpaceManager,
) -> BoxFuture<'a, ()> {
    Box::pin(async move {
        pets::pet_owner_sweep(tx, space_mgr).await;
    })
}

fn arrival_tick<'a>(
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    space_mgr: &'a mut SpaceManager,
) -> BoxFuture<'a, ()> {
    Box::pin(async move {
        pets::pet_arrival_tick(tx, space_mgr).await;
    })
}

/// Pets leave with their owner, visibly, before the owner goes: the destroy
/// may take an instanced space (and the pet in it) down with it, and
/// `destroy_entity` itself has no `tx` for the pet's `LeftAoI`.
fn owner_destroyed<'a>(
    entity_id: u32,
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    space_mgr: &'a mut SpaceManager,
) -> BoxFuture<'a, ()> {
    Box::pin(async move {
        pets::on_owner_left(
            entity_id,
            pets::PetDespawnReason::OwnerGone,
            pets::OwnerPath::BaseDestroy,
            tx,
            space_mgr,
        )
        .await;
    })
}
