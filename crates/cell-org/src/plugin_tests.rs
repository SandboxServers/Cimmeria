//! `OrgPlugin`'s registrations, driven through the registry the way the cell
//! router and core's hook points drive them.
//!
//! The router itself (`dispatch_cell_method`) is `cimmeria-cell`'s; its
//! routing tests with this plugin installed live there, beside the
//! base-message tests that fire the disconnect and world-entry hooks through
//! the real arms.

use cimmeria_cell_world::cell::org_creation::OrgCreationResources;
use cimmeria_cell_world::cell::plugin::{
    CellMethodCall, CellPlugins, EntityHookPoint, PlayerHookPoint, PluginError,
    PLUGIN_OWNED_CELL_METHODS,
};
use cimmeria_cell_world::cell::squad::{entity_squad_id, set_entity_squad_id, SquadResources};
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::organization::OrgType;
use cimmeria_wire::cell::cell_methods::organization::{
    INVITE_RESPONSE, SQUAD_SET_LOOT_MODE, TRANSFER_CASH,
};
use cimmeria_wire::cell::cell_methods::player::constants::ORG_CREATION;
use tokio::sync::mpsc;
use tracing::Level;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::organization::tests::{channel, drain, seed_squad, world};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{make_space_manager_with_player, LogCapture};
use crate::OrgPlugin;

fn org() -> CellPlugins {
    CellPlugins::build(&[&OrgPlugin]).expect("OrgPlugin registers only valid indices")
}

/// The indices this plugin owns: the OrganizationMember interface and the
/// creation name.
fn org_methods() -> Vec<u16> {
    (INVITE_RESPONSE..=TRANSFER_CASH)
        .chain(std::iter::once(ORG_CREATION))
        .collect()
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

/// The plugin owns exactly 8-19 and 94: with it alone, the startup check
/// names only the other plugins' methods as missing.
#[test]
fn org_plugin_registers_exactly_the_organization_methods() {
    let plugins = org();
    assert_eq!(plugins.plugin_names(), &["org"]);
    assert_eq!(
        plugins.cell_method_indices().collect::<Vec<_>>(),
        org_methods()
    );
    let others: Vec<u16> = PLUGIN_OWNED_CELL_METHODS
        .iter()
        .copied()
        .filter(|i| !org_methods().contains(i))
        .collect();
    assert_eq!(others, vec![88, 89, 90, 102, 103]);
    match plugins.check_complete() {
        Err(PluginError::MissingCellMethods { missing }) => assert_eq!(
            missing.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
            others,
            "only the pets and duel plugins' methods are missing"
        ),
        other => panic!("expected the pet and duel methods missing: {other:?}"),
    }
}

/// Every organization index reaches the organization decoder through the
/// registered handler: an empty payload is refused as
/// `org.cell_method_malformed` on the `org` target, which only the
/// organization handlers log, and gets no answer.
#[tokio::test]
async fn each_organization_method_reaches_the_organization_decoder() {
    let capture = LogCapture::install();
    let plugins = org();
    let mut mgr = world(&["Alice"]);
    let (tx, mut rx) = channel();
    for index in org_methods() {
        call(&plugins, index, 11, &[], &tx, &mut mgr).await;
        assert!(
            capture.all().iter().any(|c| c.target == "org"
                && c.has_field("event", "org.cell_method_malformed")
                && c.has_field("method_index", &index.to_string())),
            "method {index} must reach the organization decoder: {:#?}",
            capture.all()
        );
    }
    assert!(
        drain(&mut rx).is_empty(),
        "a malformed call is not answered"
    );
}

/// CM 18 reaches the squad loot handler: a player in no squad is refused
/// with the squad rejection pair (moved from `cimmeria-cell-methods`' router
/// tests, which proved the static arm; here through the registered handler).
#[tokio::test]
async fn squad_loot_mode_reaches_the_squad_handler() {
    let plugins = org();
    let mut mgr = world(&["Alice"]);
    let (tx, mut rx) = channel();
    call(
        &plugins,
        SQUAD_SET_LOOT_MODE,
        11,
        &1i32.to_le_bytes(),
        &tx,
        &mut mgr,
    )
    .await;
    let sent = drain(&mut rx);
    assert_eq!(
        sent.iter().map(|s| (s.0, s.1)).collect::<Vec<_>>(),
        vec![(11, 121), (11, 28)],
        "onErrorCode and a feedback line to the caller"
    );
}

/// The disconnect hook removes the member (the squad of two disbands) and
/// ends the open registrar offer, in that order, at
/// `AfterDisconnectTradeCancel` only.
#[tokio::test]
async fn the_disconnect_hook_leaves_the_squad_and_ends_the_offer() {
    let mut mgr = world(&["Alice", "Bob"]);
    mgr.install_plugins(org());
    let sid = seed_squad(&mut mgr, 11, &[12]);
    mgr.resources
        .org_creations_mut()
        .open(1, OrgType::Team, 99, 1, std::time::Instant::now())
        .unwrap();
    let (tx, mut rx) = channel();

    // Another point fires nothing of this plugin's.
    mgr.fire_entity_hook(EntityHookPoint::BeforeDisconnectTeardown, 11, &tx)
        .await;
    assert_eq!(mgr.resources.squads().squad_of(1), Some(sid));
    assert!(mgr.resources.org_creations().get(1).is_some());

    mgr.fire_entity_hook(EntityHookPoint::AfterDisconnectTradeCancel, 11, &tx)
        .await;
    assert_eq!(mgr.resources.squads().squad_of(1), None);
    assert_eq!(
        mgr.resources.squads().squad_of(2),
        None,
        "a squad of one disbands"
    );
    assert!(mgr.resources.org_creations().get(1).is_none());
    assert!(
        drain(&mut rx).iter().any(|s| s.0 == 12),
        "the remaining member is told"
    );
}

/// The world-entry hook re-sends the squad to a member whose entity was
/// re-created and re-stamps the entity's squad, with the character id the
/// hook is given.
#[tokio::test]
async fn the_world_entry_hook_replays_the_squad() {
    let mut mgr = world(&["Alice", "Bob"]);
    mgr.install_plugins(org());
    let sid = seed_squad(&mut mgr, 11, &[12]);
    set_entity_squad_id(mgr.get_entity_mut(12).unwrap(), None);
    let (tx, mut rx) = channel();

    let plugins = mgr.plugins().clone();
    plugins
        .run_player_hook(PlayerHookPoint::AfterInitPlayerState, 12, 2, &tx, &mut mgr)
        .await;

    let to_bob: Vec<u16> = drain(&mut rx)
        .into_iter()
        .filter(|s| s.0 == 12)
        .map(|s| s.1)
        .collect();
    assert_eq!(to_bob, vec![35, 38, 37, 51]);
    assert_eq!(entity_squad_id(mgr.get_entity(12).unwrap()), Some(sid));
}

/// Without the plugin, neither hook changes anything.
#[tokio::test]
async fn without_the_plugin_the_hooks_change_nothing() {
    let mut mgr = world(&["Alice", "Bob"]);
    let sid = seed_squad(&mut mgr, 11, &[12]);
    let (tx, mut rx) = channel();
    mgr.fire_entity_hook(EntityHookPoint::AfterDisconnectTradeCancel, 11, &tx)
        .await;
    let plugins = mgr.plugins().clone();
    plugins
        .run_player_hook(PlayerHookPoint::AfterInitPlayerState, 12, 2, &tx, &mut mgr)
        .await;
    assert_eq!(mgr.resources.squads().squad_of(1), Some(sid));
    assert!(drain(&mut rx).is_empty());
}

/// CM 94 carries only the name (audit A-09). The registered handler hands it
/// to the ORG-05 creation check, which decodes it ("SG-1" is four units)
/// and, with no registrar offer open, refuses with 134 `(0,
/// NO_PENDING_CREATION)` and a line. Nothing reaches the base. (Moved from
/// `cimmeria-cell-methods`' `social.rs`, which proved the static arm.)
#[tokio::test]
async fn org_creation_routes_to_the_creation_handler() {
    let capture = LogCapture::install();
    let plugins = org();
    let mut mgr = make_space_manager_with_player(1);
    mgr.get_entity_mut(1).unwrap().player_id = Some(100);
    let (tx, mut rx) = mpsc::channel(8);
    let args = [4, 0, 0, 0, 0x53, 0, 0x47, 0, 0x2D, 0, 0x31, 0];
    call(&plugins, ORG_CREATION, 1, &args, &tx, &mut mgr).await;
    let ev = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "org.create"))
        .expect("creation outcome row");
    assert_eq!(ev.target, "org");
    assert_eq!(ev.level, Level::INFO);
    assert!(
        ev.has_field("reason", "no_pending_creation"),
        "{:?}",
        ev.fields
    );
    assert!(ev.has_field("name_units", "4"), "{:?}", ev.fields);
    let mut sent = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } => sent.push((method_index, args)),
            other => panic!("nothing may reach the base: {other:?}"),
        }
    }
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[0], (134, vec![0, 5]));
    assert_eq!(sent[1].0, 28);
}

/// A forged CM 94 length is logged and not answered. (Moved from
/// `cimmeria-cell-methods`' `social.rs`.)
#[tokio::test]
async fn org_creation_rejects_a_forged_length() {
    let capture = LogCapture::install();
    let plugins = org();
    let mut mgr = make_space_manager_with_player(1);
    let (tx, mut rx) = mpsc::channel(8);
    let args = [0x10, 0, 0, 0, 0x53, 0];
    call(&plugins, ORG_CREATION, 1, &args, &tx, &mut mgr).await;
    assert!(capture
        .find_event(Level::WARN, "did not decode", "truncated")
        .is_some());
    assert!(rx.try_recv().is_err(), "a malformed call is not answered");
}
