//! `PetsPlugin`'s registrations, driven through the registry the way the
//! cell router and the cell loop drive them.
//!
//! The router itself (`dispatch_cell_method`) is `cimmeria-cell`'s; its
//! routing tests with this plugin installed live there.

use cimmeria_cell_world::cell::plugin::{
    CellMethodCall, CellPlugins, EntityHookPoint, TickStage, PLUGIN_OWNED_CELL_METHODS,
};
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;
use tracing::Level;

use crate::test_support::{
    assert_pet_fully_gone, drain_left_aoi_for, make_space_manager_with_player, watched_pet_world,
    LogCapture, PET_FIXTURE_OTHER as OTHER, PET_FIXTURE_OWNER as OWNER,
};
use crate::PetsPlugin;

fn pets() -> CellPlugins {
    CellPlugins::build(&[&PetsPlugin]).expect("PetsPlugin registers only valid indices")
}

/// The plugin owns exactly the plugin-owned cell methods, so the startup
/// check passes with it and fails without it.
#[test]
fn pets_plugin_covers_every_plugin_owned_cell_method() {
    let plugins = pets();
    plugins.check_complete().unwrap();
    assert_eq!(
        plugins.cell_method_indices().collect::<Vec<_>>(),
        PLUGIN_OWNED_CELL_METHODS.to_vec()
    );
    assert!(CellPlugins::build(&[]).unwrap().check_complete().is_err());
}

/// Each of 88, 89 and 90 reaches the pet command parser through the
/// registered handler: empty args are refused as `malformed_args` on
/// `pets.command`, which only the pet module logs. (This is the proof the
/// old `pet_methods_route_to_pet_not_world` gave for the static arm.)
#[tokio::test]
async fn each_pet_cell_method_reaches_the_pet_command_parser() {
    let capture = LogCapture::install();
    let plugins = pets();
    let mut mgr = make_space_manager_with_player(1);
    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    for &index in PLUGIN_OWNED_CELL_METHODS {
        let handler = plugins.cell_method(index).expect("registered");
        handler(CellMethodCall {
            entity_id: 1,
            method_index: index,
            args: &[],
            tx: &tx,
            space_mgr: &mut mgr,
            engine: &engine,
        })
        .await;
    }
    let malformed = capture
        .all()
        .into_iter()
        .filter(|c| {
            c.level == Level::WARN
                && c.target == "pets.command"
                && c.has_field("reason", "malformed_args")
        })
        .count();
    assert_eq!(malformed, 3, "captured: {:#?}", capture.all());
}

/// The owner-sweep stage despawns a pet whose owner is gone (PT-01), as the
/// inline `pet_owner_sweep` call in the cell loop did.
#[tokio::test]
async fn the_owner_sweep_stage_despawns_a_pet_whose_owner_is_gone() {
    let plugins = pets();
    let (mut mgr, pet) = watched_pet_world();
    mgr.destroy_entity(OWNER);
    let (tx, mut rx) = mpsc::channel(64);

    plugins
        .run_tick(TickStage::AfterRingTransport, &tx, &mut mgr)
        .await;

    assert_eq!(drain_left_aoi_for(&mut rx, pet), vec![OTHER]);
    assert_pet_fully_gone(&mgr, OWNER, pet);
}

/// The sweep is not registered at the other stage: firing it there does
/// nothing, so the hook cannot run at a different point in the tick.
#[tokio::test]
async fn the_arrival_stage_does_not_sweep() {
    let plugins = pets();
    let (mut mgr, pet) = watched_pet_world();
    mgr.destroy_entity(OWNER);
    let (tx, _rx) = mpsc::channel(64);

    plugins
        .run_tick(TickStage::AfterStatBuffs, &tx, &mut mgr)
        .await;

    assert!(
        mgr.get_entity(pet).is_some(),
        "only the owner-sweep stage despawns"
    );
}

/// The base-destroy hook despawns the owner's pet before the owner goes,
/// telling only the other witness (the owner's session is closing).
#[tokio::test]
async fn the_base_destroy_hook_despawns_the_owners_pet() {
    let plugins = pets();
    let (mut mgr, pet) = watched_pet_world();
    let (tx, mut rx) = mpsc::channel(64);

    plugins
        .run_entity_hook(EntityHookPoint::BeforeBaseDestroy, OWNER, &tx, &mut mgr)
        .await;

    assert!(
        mgr.get_entity(OWNER).is_some(),
        "the hook runs before the destroy"
    );
    assert_eq!(drain_left_aoi_for(&mut rx, pet), vec![OTHER]);
    assert!(mgr.get_entity(pet).is_none());
}
