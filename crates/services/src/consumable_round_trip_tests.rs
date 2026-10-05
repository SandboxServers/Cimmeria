//! The native consumables end to end, on real seed rows: the base's
//! `useItem` (`handle_use_inventory_item`) → `ItemUsed` → the cell's
//! `fire_item_use` → `ConsumeItemForUse` through the base's real
//! `handle_cell_message` → `ItemUseConsumed` → the cell's
//! `apply_consumed_item`.
//!
//! The cell half is `cimmeria-cell-content` and the base half
//! `cimmeria-base-methods` / `cimmeria-base-world-entry`; neither track can
//! depend on the other, so the round trip lives in the facade, like the
//! gate, mission and bank round trips.
//!
//! The ability, effect and binding caches and the chain engine are loaded
//! from the database the way `CellService` startup loads them, so the
//! guards run against the seed's `items_event_sets`, `effects.script_name`
//! and `effect_nvps` rows, not a hand-built copy.
//!
//! Sentinels (the consumable block, `0x7000_D140`..`0x7000_D17F`): account
//! `0x7000_D140`, player `0x7000_D141`, entity `0x7000_D141`. Port 40930.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine};
use cimmeria_content_engine::triggers::Trigger;
use cimmeria_entity::stats::{COORDINATION, ENGAGEMENT, FOCUS, HEALTH};
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::vault::VaultAccess;

use crate::base::world_entry::handle_cell_message;
use crate::base::world_entry::methods::handle_use_inventory_item;
use crate::cell::content::{apply_consumed_item, build_engine, fire_item_use};
use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;
use crate::test_support::{require_db_or_skip, test_default_connected_client_state, TestTransport};

const ACCOUNT: i32 = 0x7000_D140;
const PLAYER_ID: i32 = 0x7000_D141;
const ENTITY: u32 = 0x7000_D141;
const PORT: u16 = 40930;

/// Health Slappack TC1: event 5 -> ability 648 -> effect 712
/// `HealHealth`, `HealAmount` 500.
const SLAPPACK: i32 = 2893;
/// Mark III Stimpack: Coordination: 2735 -> 3950 `StatBuff` +5.
const STIM_COORDINATION: i32 = 6677;
/// Mark III Stimpack: Engagement: 2736 -> 3951 `StatBuff` +5.
const STIM_ENGAGEMENT: i32 = 6678;
/// Opheltes's Injection: a mission item whose only event-5 binding is the
/// 597 Heal Focus filler.
const QUEST_ITEM_ON_597: i32 = 1893;
/// Subspace Tracking Device: bound to the 597 filler but a bag item
/// (`container_sets` `{1,17}`), so the 597 path's silence is tested where
/// the not-implemented refusal would otherwise apply.
const BAG_ITEM_ON_597: i32 = 2042;
/// Stealth Boost Consumable: a bag consumable whose effect 3221 has no
/// script.
const STEALTH_BOOST: i32 = 6206;

async fn teardown(pool: &PgPool) {
    for sql in [
        "DELETE FROM cell_event_outbox WHERE entity_id = $1",
        "DELETE FROM sgw_inventory WHERE character_id = $1",
    ] {
        sqlx::query(sql)
            .bind(PLAYER_ID)
            .execute(pool)
            .await
            .unwrap();
    }
    sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(ACCOUNT)
        .execute(pool)
        .await
        .unwrap();
}

async fn seed(pool: &PgPool) {
    teardown(pool).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(ACCOUNT)
        .bind(format!("consumable-{ACCOUNT}"))
        .execute(pool)
        .await
        .expect("insert account");
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, 0)",
    )
    .bind(ACCOUNT)
    .bind(PLAYER_ID)
    .bind(format!("consumable-{PLAYER_ID}"))
    .execute(pool)
    .await
    .expect("insert player");
}

/// A stack of `stack_size` of `type_id` in the main bag at `slot_id`.
async fn stack(pool: &PgPool, type_id: i32, stack_size: i32, slot_id: i32) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges) \
         VALUES ($1, $2, $3, $4, 1, false, 100, 0) RETURNING item_id",
    )
    .bind(PLAYER_ID)
    .bind(type_id)
    .bind(stack_size)
    .bind(slot_id)
    .fetch_one(pool)
    .await
    .expect("insert inventory row")
}

async fn stack_size(pool: &PgPool, instance: i32) -> Option<i32> {
    sqlx::query_scalar(
        "SELECT stack_size FROM sgw_inventory WHERE character_id = $1 AND item_id = $2",
    )
    .bind(PLAYER_ID)
    .bind(instance)
    .fetch_optional(pool)
    .await
    .unwrap()
}

/// The cell as `CellService` startup builds it for these paths: the real
/// ability, effect and binding caches and the chain engine, and the player
/// with HEALTH `health`/2000, FOCUS 100/900 and Coordination /
/// Engagement at an archetype-like 10/10.
async fn stage(pool: &PgPool, health: i32) -> (SpaceManager, ChainEngine) {
    let mut mgr = SpaceManager::new(1);
    mgr.install_effect_scripts(crate::plugins::effect_scripts().unwrap());
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" Instanced="false" MinX="-100" MaxX="100" MinY="-100" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.ability_defs = spawner::load_ability_defs(pool).await.unwrap();
    mgr.effect_defs = spawner::load_effect_defs(pool).await.unwrap();
    mgr.item_event_set_abilities = spawner::load_item_event_set_abilities(pool).await.unwrap();
    mgr.item_containers = spawner::load_item_containers(pool).await.unwrap();
    mgr.create_entity(ENTITY, "W", [0.0; 3], [0.0; 3]).unwrap();
    let e = mgr.get_entity_mut(ENTITY).unwrap();
    e.is_player = true;
    e.player_id = Some(PLAYER_ID);
    e.account_id = Some(ACCOUNT as u32);
    e.stats.get_mut(HEALTH).unwrap().update(0, health, 2000);
    e.stats.get_mut(FOCUS).unwrap().update(0, 100, 900);
    e.stats.get_mut(COORDINATION).unwrap().update(0, 10, 10);
    e.stats.get_mut(ENGAGEMENT).unwrap().update(0, 10, 10);
    e.stats.clear_dirty();
    (mgr, build_engine(Some(pool)).await)
}

/// The base's side: one in-world client for `ENTITY`.
struct Base {
    transport: Arc<dyn Transport>,
    e2a: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    conn: Arc<Mutex<HashMap<SocketAddr, crate::base::ConnectedClientState>>>,
    pool: Option<Arc<PgPool>>,
}

impl Base {
    fn new(pool: &PgPool) -> Base {
        let addr: SocketAddr = format!("127.0.0.1:{PORT}").parse().unwrap();
        let mut state = test_default_connected_client_state();
        state.player_entity_id = Some(ENTITY);
        Base {
            transport: Arc::new(TestTransport::new()),
            e2a: Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)]))),
            conn: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
            pool: Some(Arc::new(pool.clone())),
        }
    }

    /// Route one cell message through the base's real dispatcher and return
    /// what the base sent back to the cell.
    async fn dispatch(&self, msg: CellToBaseMsg) -> Vec<BaseToCellMsg> {
        let (tx, mut rx) = mpsc::channel(64);
        handle_cell_message(
            msg,
            &self.transport,
            &self.conn,
            &self.e2a,
            &Some(tx),
            &self.pool,
            &None,
            "",
            0,
        )
        .await;
        let mut out = Vec::new();
        while let Ok(m) = rx.try_recv() {
            out.push(m);
        }
        out
    }
}

/// What the whole round trip produced for the uses played.
#[derive(Debug, Default)]
struct Outcome {
    /// `ConsumeItemForUse` requests the cell sent.
    consume_requests: usize,
    /// `ItemUseConsumed` answers the base sent.
    consumed: usize,
    /// `RemoveInventoryItem` / `ByType` a chain sent.
    chain_removes: usize,
    /// Client method indices the cell sent the player.
    methods: Vec<u16>,
    /// The `CHAN_FEEDBACK`-bearing args, for the refusal text.
    method_args: Vec<Vec<u8>>,
}

/// Click `instance` `clicks` times (each `useItem` reaches the cell before
/// any consume is answered, as with a fast double-click), then let the base
/// answer the cell's requests and the cell apply the answers.
async fn use_item(
    mgr: &mut SpaceManager,
    engine: &ChainEngine,
    base: &Base,
    instance: i32,
    clicks: usize,
) -> Outcome {
    let mut out = Outcome::default();
    let (to_cell_tx, mut to_cell_rx) = mpsc::channel(64);
    for _ in 0..clicks {
        handle_use_inventory_item(
            ENTITY,
            PLAYER_ID,
            instance,
            0,
            VaultAccess::NO_SESSION,
            &base.pool,
            &Some(to_cell_tx.clone()),
            &base.transport,
            &base.conn,
            &base.e2a,
        )
        .await;
    }
    let (tx, mut rx) = mpsc::channel(256);
    while let Ok(msg) = to_cell_rx.try_recv() {
        let BaseToCellMsg::ItemUsed {
            entity_id,
            instance_id,
            type_id,
            ..
        } = msg
        else {
            panic!("useItem sent the cell something other than ItemUsed");
        };
        fire_item_use(entity_id, PLAYER_ID, instance_id, type_id, engine, &tx, mgr).await;
    }
    let mut to_base = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        to_base.push(msg);
    }
    for msg in to_base {
        match msg {
            CellToBaseMsg::ConsumeItemForUse(_) => {
                out.consume_requests += 1;
                for answer in base.dispatch(msg).await {
                    if let BaseToCellMsg::ItemUseConsumed(c) = answer {
                        out.consumed += 1;
                        apply_consumed_item(c, &tx, mgr).await;
                    }
                }
            }
            CellToBaseMsg::RemoveInventoryItem { .. }
            | CellToBaseMsg::RemoveInventoryItemByType { .. } => {
                out.chain_removes += 1;
                let _ = base.dispatch(msg).await;
            }
            CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } => {
                out.methods.push(method_index);
                out.method_args.push(args);
            }
            _ => {}
        }
    }
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } = msg
        {
            out.methods.push(method_index);
            out.method_args.push(args);
        }
    }
    out
}

fn stat(mgr: &SpaceManager, id: i32) -> i32 {
    mgr.get_entity(ENTITY).unwrap().stats.get(id).unwrap().cur
}

fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

fn saw_text(out: &Outcome, text: &str) -> bool {
    let want = utf16(text);
    out.method_args
        .iter()
        .any(|a| a.windows(want.len()).any(|w| w == want))
}

/// The Health Slappack, the whole way: +500 HEALTH and one unit per use,
/// clamped at max; at full health the use is refused with the chat line
/// and the stack is untouched.
#[tokio::test]
async fn live_db_a_health_slappack_heals_500_per_unit_and_is_refused_at_full_health() {
    let pool = require_db_or_skip!();
    seed(&pool).await;
    let instance = stack(&pool, SLAPPACK, 3, 0).await;
    let (mut mgr, engine) = stage(&pool, 1200).await;
    let base = Base::new(&pool);

    let out = use_item(&mut mgr, &engine, &base, instance, 1).await;
    assert_eq!((out.consume_requests, out.consumed), (1, 1), "{out:?}");
    assert_eq!(stat(&mgr, HEALTH), 1700, "exactly +500");
    assert_eq!(
        stack_size(&pool, instance).await,
        Some(2),
        "exactly one unit"
    );

    let out = use_item(&mut mgr, &engine, &base, instance, 1).await;
    assert_eq!(out.consumed, 1);
    assert_eq!(stat(&mgr, HEALTH), 2000, "clamped at max");
    assert_eq!(stack_size(&pool, instance).await, Some(1));

    let out = use_item(&mut mgr, &engine, &base, instance, 1).await;
    assert_eq!(
        (out.consume_requests, out.consumed),
        (0, 0),
        "a use at full health must consume nothing: {out:?}"
    );
    assert_eq!(
        stack_size(&pool, instance).await,
        Some(1),
        "stack untouched"
    );
    assert!(
        out.methods
            .contains(&crate::mercury::method_idx::ON_ERROR_CODE),
        "onErrorCode for parity"
    );
    assert!(
        saw_text(&out, "You are already at full health."),
        "the refusal line the player sees"
    );
    teardown(&pool).await;
}

/// A fast double-click on the last slappack: both `useItem`s reach the
/// cell before either consume is answered. One unit, one heal.
#[tokio::test]
async fn live_db_a_double_click_on_the_last_slappack_heals_once() {
    let pool = require_db_or_skip!();
    seed(&pool).await;
    let instance = stack(&pool, SLAPPACK, 1, 0).await;
    let (mut mgr, engine) = stage(&pool, 200).await;
    let base = Base::new(&pool);

    let out = use_item(&mut mgr, &engine, &base, instance, 2).await;
    assert_eq!(out.consume_requests, 2, "both clicks pass the cell's gate");
    assert_eq!(out.consumed, 1, "the base answers only the unit it took");
    assert_eq!(stat(&mgr, HEALTH), 700, "one +500 heal, not two");
    assert_eq!(
        stack_size(&pool, instance).await,
        None,
        "the last unit is gone"
    );
    teardown(&pool).await;
}

/// The retired chain 4001, restored beside the native path: the chain owns
/// the item, so one use heals +500 once and consumes one unit once.
#[tokio::test]
async fn live_db_a_restored_slappack_chain_neither_double_heals_nor_double_consumes() {
    let pool = require_db_or_skip!();
    seed(&pool).await;
    let instance = stack(&pool, SLAPPACK, 3, 0).await;
    let (mut mgr, mut engine) = stage(&pool, 200).await;
    engine.register_chain(Chain {
        id: 4001,
        name: "Health Slappack TC1: +500 HP + consume (restored)".to_string(),
        enabled: true,
        trigger: Trigger::OnItemUse { item_id: SLAPPACK },
        conditions: vec![],
        actions: vec![
            Action::ChangeStat {
                stat_id: HEALTH,
                min: None,
                max: None,
                use_ammo_stat: None,
                set_to_max: None,
                amount: Some(500),
            },
            Action::RemoveItem {
                item_id: SLAPPACK,
                count: 1,
            },
        ],
        action_delays: vec![],
        priority: 0,
        once: false,
    });
    let base = Base::new(&pool);

    let out = use_item(&mut mgr, &engine, &base, instance, 1).await;
    assert_eq!(out.consume_requests, 0, "the native path stands aside");
    assert_eq!(out.chain_removes, 1);
    assert_eq!(stat(&mgr, HEALTH), 700, "one +500 heal");
    assert_eq!(stack_size(&pool, instance).await, Some(2), "one unit");
    teardown(&pool).await;
}

/// A Mark III stim: Coordination +5 for its hour, HEALTH untouched, one
/// unit; a second, different-stat stim lands beside it.
#[tokio::test]
async fn live_db_stimpacks_buff_their_stat_and_two_different_stats_both_hold() {
    let pool = require_db_or_skip!();
    seed(&pool).await;
    let coordination = stack(&pool, STIM_COORDINATION, 1, 0).await;
    let engagement = stack(&pool, STIM_ENGAGEMENT, 1, 1).await;
    let (mut mgr, engine) = stage(&pool, 1200).await;
    let base = Base::new(&pool);

    let out = use_item(&mut mgr, &engine, &base, coordination, 1).await;
    assert_eq!(out.consumed, 1);
    assert_eq!(stat(&mgr, COORDINATION), 15, "+5 Coordination");
    assert_eq!(stat(&mgr, HEALTH), 1200, "a stim heals nothing");
    assert_eq!(stack_size(&pool, coordination).await, None);
    assert!(
        out.methods
            .contains(&crate::cell::client_methods::being::ON_TIMER_UPDATE),
        "the buff's duration icon"
    );

    let out = use_item(&mut mgr, &engine, &base, engagement, 1).await;
    assert_eq!(out.consumed, 1);
    assert_eq!(stat(&mgr, ENGAGEMENT), 15, "+5 Engagement");
    assert_eq!(stat(&mgr, COORDINATION), 15, "the first buff still holds");
    let buffs = &mgr.get_entity(ENTITY).unwrap().stat_buffs.entries;
    assert_eq!(buffs.len(), 2);
    assert!(buffs.iter().all(|b| b.duration_secs == 3600.0));
    teardown(&pool).await;
}

/// The 597 gate on a real seed row: Opheltes's Injection is bound to 597
/// Heal Focus (whose effect 659 does heal 35% Focus), and using it must
/// neither heal focus nor be consumed by the native path.
#[tokio::test]
async fn live_db_an_item_bound_to_the_597_placeholder_does_not_heal_focus() {
    let pool = require_db_or_skip!();
    seed(&pool).await;
    let instance = stack(&pool, QUEST_ITEM_ON_597, 1, 0).await;
    let (mut mgr, engine) = stage(&pool, 1200).await;
    assert_eq!(
        mgr.item_event_set_abilities.get(&(QUEST_ITEM_ON_597, 5)),
        Some(&597),
        "seed drift: 1893's event-5 binding is no longer 597; pick another \
         597-bound item from items_event_sets"
    );
    assert!(
        mgr.effect_defs
            .get(&659)
            .and_then(|e| e.script_name.as_deref())
            == Some("HealFocus"),
        "597's effect must still be a real HealFocus, or this guard is vacuous"
    );
    let base = Base::new(&pool);

    let out = use_item(&mut mgr, &engine, &base, instance, 1).await;
    assert_eq!(out.consume_requests, 0, "{out:?}");
    assert_eq!(
        stat(&mgr, FOCUS),
        100,
        "no free focus heal from a quest item"
    );
    assert_eq!(stack_size(&pool, instance).await, Some(1));
    teardown(&pool).await;
}

/// A real bag consumable whose effect is not implemented (the Stealth
/// Boost) is refused with a line and kept; a 597-bound bag item stays
/// silent and is kept too.
#[tokio::test]
async fn live_db_an_unimplemented_consumable_is_refused_and_a_597_bag_item_stays_silent() {
    let pool = require_db_or_skip!();
    seed(&pool).await;
    let boost = stack(&pool, STEALTH_BOOST, 1, 0).await;
    let filler = stack(&pool, BAG_ITEM_ON_597, 1, 1).await;
    let (mut mgr, engine) = stage(&pool, 1200).await;
    for (item, container) in [(STEALTH_BOOST, 1), (BAG_ITEM_ON_597, 1)] {
        assert_eq!(
            mgr.item_containers.get(&item),
            Some(&container),
            "seed drift: {item} is no longer a bag item"
        );
    }
    assert_eq!(
        mgr.item_event_set_abilities.get(&(BAG_ITEM_ON_597, 5)),
        Some(&597),
        "seed drift: {BAG_ITEM_ON_597} is no longer bound to 597"
    );
    let base = Base::new(&pool);

    let out = use_item(&mut mgr, &engine, &base, boost, 1).await;
    assert_eq!(out.consume_requests, 0, "{out:?}");
    assert_eq!(stack_size(&pool, boost).await, Some(1), "the boost is kept");
    assert!(saw_text(&out, "This item has no effect yet."), "{out:?}");

    let out = use_item(&mut mgr, &engine, &base, filler, 1).await;
    assert_eq!(out.consume_requests, 0);
    assert!(
        out.methods.is_empty(),
        "the 597 path sends nothing: {out:?}"
    );
    assert_eq!(stack_size(&pool, filler).await, Some(1));
    assert_eq!(stat(&mgr, FOCUS), 100, "no focus heal");
    teardown(&pool).await;
}
