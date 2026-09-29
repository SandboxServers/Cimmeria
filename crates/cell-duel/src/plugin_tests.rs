//! `DuelPlugin`'s cell-method registrations, driven through the registry the
//! way the cell router drives them.
//!
//! The router itself (`dispatch_cell_method`) is `cimmeria-cell`'s; its
//! routing tests with this plugin installed live there. The tick and the
//! lifecycle hooks are `cell::duel::tests::hooks`.

use cimmeria_cell_world::cell::duel::DuelResources;
use cimmeria_cell_world::cell::plugin::{CellMethodCall, CellPlugins, PluginError};
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_wire::cell::cell_methods::player::constants::{DUEL_FORFEIT, SEND_DUEL_RESPONSE};
use cimmeria_wire::cell::client_methods::duel::TEXT_FORFEIT_NOT_ENGAGED;
use tokio::sync::mpsc;

use crate::cell::messages::{CellToBaseMsg, DuelBaseToCell};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::make_space_manager_with_player;
use crate::DuelPlugin;

fn duel() -> CellPlugins {
    CellPlugins::build(&[&DuelPlugin]).expect("DuelPlugin registers only valid indices")
}

/// Run the handler the plugin registered for `index`.
async fn call(
    plugins: &CellPlugins,
    index: u16,
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &mut SpaceManager,
) {
    let engine = ChainEngine::new();
    let handler = plugins.cell_method(index).expect("registered");
    handler(CellMethodCall {
        entity_id,
        method_index: index,
        args,
        tx,
        space_mgr: mgr,
        engine: &engine,
    })
    .await;
}

/// The plugin owns exactly the two duel cell methods: with it alone, the
/// startup check names only the other plugins' methods as missing.
#[test]
fn duel_plugin_registers_exactly_the_duel_methods() {
    let plugins = duel();
    assert_eq!(plugins.plugin_names(), &["duel"]);
    assert_eq!(
        plugins.cell_method_indices().collect::<Vec<_>>(),
        vec![SEND_DUEL_RESPONSE, DUEL_FORFEIT]
    );
    match plugins.check_complete() {
        Err(PluginError::MissingCellMethods { missing }) => assert_eq!(
            missing.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
            cimmeria_cell_world::cell::plugin::PLUGIN_OWNED_CELL_METHODS
                .iter()
                .copied()
                .filter(|i| ![SEND_DUEL_RESPONSE, DUEL_FORFEIT].contains(i))
                .collect::<Vec<_>>(),
            "only the other plugins' methods are missing"
        ),
        other => panic!("expected the other plugins' methods missing: {other:?}"),
    }
}

/// CM 102 reaches the duel answer through the registered handler: an accept
/// from the target of a pending challenge starts the duel. (Moved from
/// `cimmeria-cell-methods`' `send_duel_response_routes_to_the_duel_handler`,
/// which proved the static arm; the old stub logged
/// `UNIMPLEMENTED: sendDuelResponse` and changed nothing.)
#[tokio::test]
async fn send_duel_response_reaches_the_duel_handler() {
    let mut mgr = make_space_manager_with_player(1);
    mgr.create_entity(2, "Agnos", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    for (eid, pid) in [(1u32, 100i32), (2, 200)] {
        mgr.connect_entity(eid);
        mgr.get_entity_mut(eid).unwrap().player_id = Some(pid);
    }
    let (tx, mut rx) = mpsc::channel(8);
    crate::cell::duel::challenge::handle(
        DuelBaseToCell::Challenge {
            player_id: 100,
            entity_id: 1,
            account_id: 0,
            target_player_id: 200,
            target_entity_id: 2,
        },
        &tx,
        &mut mgr,
    )
    .await;
    while rx.try_recv().is_ok() {}
    call(&duel(), SEND_DUEL_RESPONSE, 2, &[1], &tx, &mut mgr).await;
    assert!(
        mgr.resources.duels().duel_of(100).is_some(),
        "the accept started the duel"
    );
}

/// CM 103 reaches the duel's forfeit through the registered handler. With no
/// engaged duel the caller hears 880. (Moved from `cimmeria-cell-methods`'
/// `duel_forfeit_routes_to_the_duel_handler`; the old stub logged
/// `UNIMPLEMENTED: duelForfeit` and sent nothing, a silent press.)
#[tokio::test]
async fn duel_forfeit_reaches_the_duel_handler() {
    let mut mgr = make_space_manager_with_player(1);
    mgr.connect_entity(1);
    mgr.get_entity_mut(1).unwrap().player_id = Some(100);
    let (tx, mut rx) = mpsc::channel(8);
    call(&duel(), DUEL_FORFEIT, 1, &[], &tx, &mut mgr).await;
    let Ok(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index,
        args,
    }) = rx.try_recv()
    else {
        panic!("the forfeit was not answered");
    };
    assert_eq!((entity_id, method_index), (1, 28));
    let text: Vec<u8> = TEXT_FORFEIT_NOT_ENGAGED
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    assert!(
        args.windows(text.len()).any(|w| w == text.as_slice()),
        "the line is 880"
    );
}
