//! `CellPlugins`: the startup checks and the hook firing order.
//!
//! Hooks here report by sending an `EntityMethodCall` whose `method_index`
//! is a marker, so the test reads the firing order off the channel.

use tokio::sync::mpsc;

use super::*;
use crate::cell::messages::CellToBaseMsg;
use cimmeria_wire::cell::cell_methods::player::constants::{
    DUEL_FORFEIT, ORG_CREATION, PET_ABILITY_TOGGLE, PET_CHANGE_STANCE, PET_INVOKE_ABILITY,
    SEND_DUEL_RESPONSE, WHO,
};

fn mark<'a>(marker: u16, entity_id: u32, tx: &'a mpsc::Sender<CellToBaseMsg>) -> BoxFuture<'a, ()> {
    Box::pin(async move {
        tx.send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: marker,
            args: Vec::new(),
        })
        .await
        .unwrap();
    })
}

fn noop_method(_call: CellMethodCall<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async {})
}
fn tick_a<'a>(tx: &'a mpsc::Sender<CellToBaseMsg>, _: &'a mut SpaceManager) -> BoxFuture<'a, ()> {
    mark(1, 0, tx)
}
fn tick_b<'a>(tx: &'a mpsc::Sender<CellToBaseMsg>, _: &'a mut SpaceManager) -> BoxFuture<'a, ()> {
    mark(2, 0, tx)
}
fn tick_c<'a>(tx: &'a mpsc::Sender<CellToBaseMsg>, _: &'a mut SpaceManager) -> BoxFuture<'a, ()> {
    mark(3, 0, tx)
}
fn destroy_hook<'a>(
    entity_id: u32,
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    _: &'a mut SpaceManager,
) -> BoxFuture<'a, ()> {
    mark(4, entity_id, tx)
}
fn travel_hook<'a>(
    entity_id: u32,
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    _: &'a mut SpaceManager,
) -> BoxFuture<'a, ()> {
    mark(5, entity_id, tx)
}
/// Reports the killer as the marker and the victim as the entity.
fn death_hook<'a>(
    victim: u32,
    killer: u32,
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    _: &'a mut SpaceManager,
) -> BoxFuture<'a, ()> {
    mark(killer as u16, victim, tx)
}

/// A plugin that registers the given method indices and nothing else.
struct Methods(&'static str, &'static [u16]);
impl CellPlugin for Methods {
    fn name(&self) -> &'static str {
        self.0
    }
    fn build(&self, plugin: &mut CellPluginBuilder<'_>) {
        for &i in self.1 {
            plugin.cell_method(i, noop_method);
        }
    }
}

const PET_OWNED: &[u16] = &[PET_INVOKE_ABILITY, PET_ABILITY_TOGGLE, PET_CHANGE_STANCE];
const DUEL_OWNED: &[u16] = &[SEND_DUEL_RESPONSE, DUEL_FORFEIT];
const ORG_OWNED: &[u16] = &[8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, ORG_CREATION];

#[test]
fn the_plugin_owned_list_is_the_pet_duel_and_org_methods() {
    assert_eq!(
        PLUGIN_OWNED_CELL_METHODS,
        &[8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 88, 89, 90, 94, 102, 103]
    );
    assert!(
        PLUGIN_OWNED_CELL_METHODS.windows(2).all(|w| w[0] < w[1]),
        "ascending, as `cell_method_indices` returns them"
    );
}

#[test]
fn a_complete_registration_builds_and_resolves_each_index() {
    let plugins = CellPlugins::build(&[
        &Methods("pets", PET_OWNED),
        &Methods("duel", DUEL_OWNED),
        &Methods("org", ORG_OWNED),
    ])
    .unwrap();
    plugins.check_complete().unwrap();
    assert_eq!(plugins.plugin_names(), &["pets", "duel", "org"]);
    assert_eq!(
        plugins.cell_method_indices().collect::<Vec<_>>(),
        PLUGIN_OWNED_CELL_METHODS.to_vec()
    );
    assert!(plugins.cell_method(88).is_some());
    assert!(
        plugins.cell_method(WHO).is_none(),
        "73 stays on the static router"
    );
}

/// #962 test rule: a feature that never registers must fail the startup
/// assertion, not silently no-op. An empty table leaves 8-19, 88-90, 94
/// and 102-103 unhandled.
#[test]
fn a_missing_plugin_fails_check_complete_with_every_unhandled_index() {
    let err = CellPlugins::build(&[])
        .unwrap()
        .check_complete()
        .unwrap_err();
    assert_eq!(
        err,
        PluginError::MissingCellMethods {
            missing: vec![
                (8, "organizationInviteResponse"),
                (9, "organizationLeave"),
                (10, "BroadcastMinimapPing"),
                (11, "strikeTeamResponse"),
                (12, "pvpOrganizationLeaveResponse"),
                (13, "organizationMOTD"),
                (14, "organizationNote"),
                (15, "organizationOfficerNote"),
                (16, "organizationSetRankPermissions"),
                (17, "organizationSetRankName"),
                (18, "squadSetLootMode"),
                (19, "organizationTransferCash"),
                (88, "petInvokeAbility"),
                (89, "petAbilityToggle"),
                (90, "petChangeStance"),
                (94, "onOrganizationCreation"),
                (102, "sendDuelResponse"),
                (103, "duelForfeit"),
            ]
        }
    );
    // A partial registration names only the gap.
    let err = CellPlugins::build(&[
        &Methods("pets", &[88, 90]),
        &Methods("duel", DUEL_OWNED),
        &Methods("org", ORG_OWNED),
    ])
    .unwrap()
    .check_complete()
    .unwrap_err();
    assert_eq!(
        err,
        PluginError::MissingCellMethods {
            missing: vec![(89, "petAbilityToggle")]
        }
    );
}

#[test]
fn two_plugins_claiming_one_index_fail_the_build() {
    let err = CellPlugins::build(&[&Methods("pets", PET_OWNED), &Methods("other", &[89])])
        .err()
        .unwrap();
    assert_eq!(
        err,
        PluginError::DuplicateCellMethod {
            index: 89,
            name: "petAbilityToggle",
            first: "pets",
            second: "other",
        }
    );
}

#[test]
fn an_index_that_is_not_a_client_cell_method_fails_the_build() {
    let err = CellPlugins::build(&[&Methods("bad", &[0xFFFF])])
        .err()
        .unwrap();
    assert_eq!(
        err,
        PluginError::UnknownCellMethod {
            index: 0xFFFF,
            plugin: "bad"
        }
    );
}

/// An index the static router still owns must not be claimed by a plugin:
/// the router would never reach one of the two handlers.
#[test]
fn an_index_the_static_router_owns_fails_the_build() {
    let err = CellPlugins::build(&[&Methods("bad", &[WHO])])
        .err()
        .unwrap();
    assert_eq!(
        err,
        PluginError::NotPluginOwned {
            index: WHO,
            name: "who",
            plugin: "bad"
        }
    );
}

struct Ticks(&'static str, &'static [(TickStage, TickHook)]);
impl CellPlugin for Ticks {
    fn name(&self) -> &'static str {
        self.0
    }
    fn build(&self, plugin: &mut CellPluginBuilder<'_>) {
        for &(stage, hook) in self.1 {
            plugin.tick(stage, hook);
        }
    }
}

fn drain_markers(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<(u16, u32)> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            ..
        } = msg
        {
            out.push((method_index, entity_id));
        }
    }
    out
}

/// Hooks fire only at their stage, in plugin-table order (§3.2): the order
/// reaches the wire, so it must not depend on anything but the table.
#[tokio::test]
async fn tick_hooks_fire_per_stage_in_table_order() {
    let plugins = CellPlugins::build(&[
        &Ticks(
            "first",
            &[
                (TickStage::AfterRingTransport, tick_a),
                (TickStage::AfterStatBuffs, tick_c),
            ],
        ),
        &Ticks("second", &[(TickStage::AfterRingTransport, tick_b)]),
    ])
    .unwrap();
    let mut mgr = SpaceManager::new(1);
    let (tx, mut rx) = mpsc::channel(8);

    plugins
        .run_tick(TickStage::AfterRingTransport, &tx, &mut mgr)
        .await;
    assert_eq!(drain_markers(&mut rx), vec![(1, 0), (2, 0)]);
    plugins
        .run_tick(TickStage::AfterStatBuffs, &tx, &mut mgr)
        .await;
    assert_eq!(drain_markers(&mut rx), vec![(3, 0)]);
}

struct Destroy;
impl CellPlugin for Destroy {
    fn name(&self) -> &'static str {
        "destroy"
    }
    fn build(&self, plugin: &mut CellPluginBuilder<'_>) {
        plugin.entity_hook(EntityHookPoint::BeforeBaseDestroy, destroy_hook);
    }
}

#[tokio::test]
async fn entity_hooks_get_the_entity_id() {
    let plugins = CellPlugins::build(&[&Destroy]).unwrap();
    let mut mgr = SpaceManager::new(1);
    let (tx, mut rx) = mpsc::channel(8);
    plugins
        .run_entity_hook(EntityHookPoint::BeforeBaseDestroy, 42, &tx, &mut mgr)
        .await;
    assert_eq!(drain_markers(&mut rx), vec![(4, 42)]);
}

/// A fresh `SpaceManager` holds the empty registry until the cell service
/// installs the table, and installing replaces it.
#[test]
fn space_manager_starts_empty_and_takes_the_installed_table() {
    let mut mgr = SpaceManager::new(1);
    assert!(mgr.plugins().plugin_names().is_empty());
    mgr.install_plugins(CellPlugins::build(&[&Methods("pets", PET_OWNED)]).unwrap());
    assert_eq!(mgr.plugins().plugin_names(), &["pets"]);
}

struct Leave;
impl CellPlugin for Leave {
    fn name(&self) -> &'static str {
        "leave"
    }
    fn build(&self, plugin: &mut CellPluginBuilder<'_>) {
        plugin
            .entity_hook(EntityHookPoint::BeforeTravelSend, travel_hook)
            .death_hook(DeathHookPoint::AfterPlayerThreatPurge, death_hook);
    }
}

/// `SpaceManager::fire_entity_hook`, the helper the travel sites below
/// `cimmeria-cell` call, fires only the hooks at its point (the travel hook,
/// not the base-destroy one), with the entity id.
#[tokio::test]
async fn fire_entity_hook_runs_the_installed_hooks_at_that_point() {
    let mut mgr = SpaceManager::new(1);
    mgr.install_plugins(CellPlugins::build(&[&Destroy, &Leave]).unwrap());
    let (tx, mut rx) = mpsc::channel(8);
    mgr.fire_entity_hook(EntityHookPoint::BeforeTravelSend, 7, &tx)
        .await;
    assert_eq!(drain_markers(&mut rx), vec![(5, 7)]);
    mgr.fire_entity_hook(EntityHookPoint::BeforeDisconnectTeardown, 7, &tx)
        .await;
    assert!(
        drain_markers(&mut rx).is_empty(),
        "nothing subscribes there"
    );
}

/// A death hook gets the victim and the killer, in that order.
#[tokio::test]
async fn death_hooks_get_the_victim_and_the_killer() {
    let mut mgr = SpaceManager::new(1);
    mgr.install_plugins(CellPlugins::build(&[&Leave]).unwrap());
    let (tx, mut rx) = mpsc::channel(8);
    mgr.fire_death_hook(DeathHookPoint::AfterPlayerThreatPurge, 42, 9, &tx)
        .await;
    assert_eq!(drain_markers(&mut rx), vec![(9, 42)]);
}

struct Entry;
impl CellPlugin for Entry {
    fn name(&self) -> &'static str {
        "entry"
    }
    fn build(&self, plugin: &mut CellPluginBuilder<'_>) {
        plugin.player_hook(PlayerHookPoint::AfterInitPlayerState, entry_hook);
    }
}

fn entry_hook<'a>(
    entity_id: u32,
    player_id: i32,
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    _: &'a mut SpaceManager,
) -> BoxFuture<'a, ()> {
    mark(player_id as u16, entity_id, tx)
}

/// A player hook gets the entity and the character id the base sent, and
/// fires only at its point.
#[tokio::test]
async fn player_hooks_get_the_entity_and_the_character() {
    let plugins = CellPlugins::build(&[&Entry, &Destroy]).unwrap();
    let mut mgr = SpaceManager::new(1);
    let (tx, mut rx) = mpsc::channel(8);
    plugins
        .run_player_hook(PlayerHookPoint::AfterInitPlayerState, 42, 77, &tx, &mut mgr)
        .await;
    assert_eq!(drain_markers(&mut rx), vec![(77, 42)]);
    mgr.install_plugins(plugins);
    mgr.fire_entity_hook(EntityHookPoint::AfterDisconnectTradeCancel, 42, &tx)
        .await;
    assert!(
        drain_markers(&mut rx).is_empty(),
        "nothing subscribes to the disconnect point"
    );
}
