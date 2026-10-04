//! The native consumable path on the cell: the classification gates (597
//! placeholder, unwired effects, chain ownership), the refusals and their
//! wire bytes, and the apply half after the base consumed the unit.
//!
//! The seed-backed guards (real rows for items 2893, 6677, 1893, the full
//! base round trip) are live-DB tests in `cimmeria-services`
//! (`consumable_round_trip_tests`).

use std::collections::HashMap;

use tokio::sync::mpsc;
use tracing::Level;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine};
use cimmeria_content_engine::triggers::Trigger;
use cimmeria_entity::abilities::{AbilityDef, EffectDef};
use cimmeria_entity::stats::{COORDINATION, FOCUS, HEALTH};
use cimmeria_wire::cell::chat::CHAN_FEEDBACK;

use super::consumable_use::{
    classify, error_code_args, Classification, ConsumablePlan, DEAD_TEXT, FEEDBACK_NOT_LIVING,
    FEEDBACK_STAT_AT_MAX, FULL_FOCUS_TEXT, FULL_HEALTH_TEXT, NOT_IMPLEMENTED_TEXT,
    PLACEHOLDER_ITEM_USE_ABILITY,
};
use super::{apply_consumed_item, fire_item_use};
use crate::cell::messages::{CellToBaseMsg, ConsumeItemForUse, ItemUseConsumed};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::EVENT_ITEM_USE_ABILITY;
use crate::test_support::LogCapture;
use cimmeria_wire::cell::vault::VaultAccess;

#[path = "consumable_use_stunned_tests.rs"]
mod stunned;

const PLAYER: u32 = 1;
const PLAYER_ID: i32 = 42;
const INSTANCE: i32 = 0x7000_D1C1;

// The seed's shapes, rebuilt in memory: a health slappack, a focus heal, a
// Mark III and a Mark V stim, a 597-bound mission item and a scanner whose
// effect has no script.
const SLAPPACK: i32 = 2893;
const FOCUS_HEAL: i32 = 6106;
const STIM_III: i32 = 6677;
const STIM_V: i32 = 6697;
const QUEST_ITEM: i32 = 1893;
const SCANNER: i32 = 5672;
/// Stealth Boost Consumable: a bag consumable (`{1,17}`) whose effect 3221
/// has no script.
const STEALTH_BOOST: i32 = 6206;

fn ability(ability_id: i32, effect_ids: Vec<i32>) -> AbilityDef {
    AbilityDef {
        ability_id,
        name: format!("ability {ability_id}"),
        cooldown: 0.0,
        warmup: 0.0,
        flags: 0,
        is_ranged: false,
        min_range: 0.0,
        max_range: 0.0,
        target_type_id: 1,
        effect_ids,
        moniker_ids: vec![],
        required_ammo: 0,
        event_set_id: None,
        velocity: 0.0,
        type_id: Default::default(),
        passive: false,
    }
}

fn effect(
    effect_id: i32,
    ability_id: i32,
    script: Option<&str>,
    nvp: (&str, &str),
    pulse_duration: f32,
) -> EffectDef {
    EffectDef {
        effect_id,
        ability_id,
        script_name: script.map(str::to_string),
        pulse_count: 1,
        pulse_duration,
        params: HashMap::from([(nvp.0.to_string(), nvp.1.to_string())]),
        ..EffectDef::default()
    }
}

fn mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    crate::test_support::install_effect_scripts(&mut mgr);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" Instanced="false" MinX="-100" MaxX="100" MinY="-100" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(PLAYER, "W", [0.0; 3], [0.0; 3]).unwrap();
    let e = mgr.get_entity_mut(PLAYER).unwrap();
    e.is_player = true;
    e.player_id = Some(PLAYER_ID);
    e.stats.get_mut(HEALTH).unwrap().update(0, 200, 1000);
    e.stats.get_mut(FOCUS).unwrap().update(0, 100, 900);
    e.stats.get_mut(COORDINATION).unwrap().update(0, 10, 10);
    e.stats.clear_dirty();

    let defs = [
        (
            SLAPPACK,
            ability(648, vec![712]),
            vec![effect(
                712,
                648,
                Some("HealHealth"),
                ("HealAmount", "500"),
                0.0,
            )],
        ),
        (
            FOCUS_HEAL,
            ability(2206, vec![3062]),
            vec![effect(
                3062,
                2206,
                Some("HealFocus"),
                ("HealAmount", "384"),
                0.0,
            )],
        ),
        (
            STIM_III,
            ability(2735, vec![3950]),
            vec![effect(
                3950,
                2735,
                Some("StatBuff"),
                ("Coordination", "5"),
                3600.0,
            )],
        ),
        (
            STIM_V,
            ability(2740, vec![3956, 3955]),
            vec![
                effect(3955, 2740, Some("StatBuff"), ("Engagement", "3"), 3600.0),
                effect(3956, 2740, Some("StatBuff"), ("Coordination", "7"), 3600.0),
            ],
        ),
        (
            QUEST_ITEM,
            ability(PLACEHOLDER_ITEM_USE_ABILITY, vec![659]),
            vec![effect(
                659,
                PLACEHOLDER_ITEM_USE_ABILITY,
                Some("HealFocus"),
                ("HealPercentage", "35.00"),
                0.0,
            )],
        ),
        (
            SCANNER,
            ability(2091, vec![2815]),
            vec![effect(2815, 2091, None, ("x", "1"), 0.0)],
        ),
        (
            STEALTH_BOOST,
            ability(2269, vec![3221]),
            vec![effect(3221, 2269, None, ("x", "1"), 0.0)],
        ),
    ];
    // Preferred containers, as `load_item_containers` derives them from
    // `container_sets`: bag consumables `{1,17}` -> 1, mission items `{2}`
    // -> 2. The 597 quest item is put in the bag on purpose, like the 14
    // real 597-bound `{1,17}` items (2042, 2592, ...): the filler must stay
    // silent there too.
    for (item, container) in [
        (SLAPPACK, 1),
        (FOCUS_HEAL, 1),
        (STIM_III, 1),
        (STIM_V, 1),
        (QUEST_ITEM, 1),
        (SCANNER, 2),
        (STEALTH_BOOST, 1),
    ] {
        mgr.item_containers.insert(item, container);
    }
    for (item, def, effects) in defs {
        mgr.item_event_set_abilities
            .insert((item, EVENT_ITEM_USE_ABILITY), def.ability_id);
        mgr.ability_defs.insert(def.ability_id, def);
        for e in effects {
            mgr.effect_defs.insert(e.effect_id, e);
        }
    }
    mgr
}

fn set_pool(mgr: &mut SpaceManager, stat: i32, cur: i32) {
    let s = mgr
        .get_entity_mut(PLAYER)
        .unwrap()
        .stats
        .get_mut(stat)
        .unwrap();
    let max = s.max;
    s.update(0, cur, max);
}

fn pool(mgr: &SpaceManager, stat: i32) -> i32 {
    mgr.get_entity(PLAYER).unwrap().stats.get(stat).unwrap().cur
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}

fn consumes(sent: &[CellToBaseMsg]) -> Vec<&ConsumeItemForUse> {
    sent.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::ConsumeItemForUse(c) => Some(c),
            _ => None,
        })
        .collect()
}

fn method_calls(sent: &[CellToBaseMsg]) -> Vec<(u16, &Vec<u8>)> {
    sent.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: PLAYER,
                method_index,
                args,
            } => Some((*method_index, args)),
            _ => None,
        })
        .collect()
}

async fn use_item(
    mgr: &mut SpaceManager,
    engine: &ChainEngine,
    type_id: i32,
) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(64);
    fire_item_use(PLAYER, PLAYER_ID, INSTANCE, type_id, engine, &tx, mgr).await;
    drain(&mut rx)
}

/// The `CHAN_FEEDBACK` chat args for `text`: speaker `SYSTEM`, flags 0.
fn feedback_line(text: &str) -> Vec<u8> {
    let mut want = vec![6, 0, 0, 0];
    for c in "SYSTEM".encode_utf16() {
        want.extend_from_slice(&c.to_le_bytes());
    }
    want.extend_from_slice(&[0, CHAN_FEEDBACK]);
    want.extend_from_slice(&(text.encode_utf16().count() as u32).to_le_bytes());
    for c in text.encode_utf16() {
        want.extend_from_slice(&c.to_le_bytes());
    }
    want
}

// ── classification ─────────────────────────────────────────────────────

#[test]
fn classify_reads_the_event_5_binding_and_the_effect_scripts() {
    let mgr = mgr();
    assert_eq!(
        classify(SLAPPACK, &mgr),
        Classification::Native(ConsumablePlan {
            ability_id: 648,
            heals: vec![HEALTH],
            buffs: false
        })
    );
    assert_eq!(
        classify(STIM_V, &mgr),
        Classification::Native(ConsumablePlan {
            ability_id: 2740,
            heals: vec![],
            buffs: true
        })
    );
    assert_eq!(classify(QUEST_ITEM, &mgr), Classification::Placeholder);
    assert_eq!(
        classify(SCANNER, &mgr),
        Classification::NotNative {
            ability_id: 2091,
            reason: "effect_not_native"
        }
    );
    assert_eq!(classify(12_345, &mgr), Classification::NotBound);
}

#[test]
fn an_event_6_or_7_binding_is_not_an_item_use() {
    let mut mgr = mgr();
    // A weapon's auto-attack binding (event 7) to a heal ability must not
    // make the weapon a consumable.
    mgr.item_event_set_abilities.insert((55, 7), 648);
    assert_eq!(classify(55, &mgr), Classification::NotBound);
}

#[test]
fn an_ability_without_a_def_is_not_native() {
    let mut mgr = mgr();
    mgr.item_event_set_abilities
        .insert((777, EVENT_ITEM_USE_ABILITY), 99_999);
    assert_eq!(
        classify(777, &mgr),
        Classification::NotNative {
            ability_id: 99_999,
            reason: "no_ability_def"
        }
    );
}

// ── phase 1: the use ───────────────────────────────────────────────────

#[tokio::test]
async fn a_slappack_below_max_asks_the_base_to_consume_and_heals_nothing_yet() {
    let mut mgr = mgr();
    let sent = use_item(&mut mgr, &ChainEngine::new(), SLAPPACK).await;
    assert_eq!(
        consumes(&sent),
        vec![&ConsumeItemForUse {
            entity_id: PLAYER,
            player_id: PLAYER_ID,
            instance_id: INSTANCE,
            type_id: SLAPPACK,
            vault: VaultAccess::NO_SESSION,
        }]
    );
    assert_eq!(pool(&mgr, HEALTH), 200, "the heal waits for the consume");
    assert_eq!(sent.len(), 1, "nothing else is sent: {sent:?}");
}

#[tokio::test]
async fn a_slappack_at_full_health_is_refused_with_feedback_and_consumes_nothing() {
    let mut mgr = mgr();
    set_pool(&mut mgr, HEALTH, 1000);
    let sent = use_item(&mut mgr, &ChainEngine::new(), SLAPPACK).await;
    assert!(consumes(&sent).is_empty(), "a refused use consumes nothing");
    let calls = method_calls(&sent);
    assert_eq!(
        calls,
        vec![
            (
                crate::mercury::method_idx::ON_ERROR_CODE,
                // ERRORCODE_SYSTEM_Ability, InstanceID = ability 648,
                // CONDITION_FEEDBACK_StatValueGreaterThanOrEqual (32).
                &vec![0x00, 0x88, 0x02, 0x00, 0x00, 0x20, 0x00]
            ),
            (
                crate::mercury::method_idx::ON_PLAYER_COMMUNICATION,
                &feedback_line(FULL_HEALTH_TEXT)
            ),
        ]
    );
}

#[tokio::test]
async fn a_focus_heal_at_full_focus_says_focus() {
    let mut mgr = mgr();
    set_pool(&mut mgr, FOCUS, 900);
    let sent = use_item(&mut mgr, &ChainEngine::new(), FOCUS_HEAL).await;
    let calls = method_calls(&sent);
    assert_eq!(calls[1].1, &feedback_line(FULL_FOCUS_TEXT));
    assert!(consumes(&sent).is_empty());
}

#[tokio::test]
async fn a_dead_user_is_refused() {
    let mut mgr = mgr();
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .set_state_flag(crate::cell::combat::BSF_DEAD);
    let sent = use_item(&mut mgr, &ChainEngine::new(), STIM_III).await;
    assert!(consumes(&sent).is_empty());
    let calls = method_calls(&sent);
    assert_eq!(calls[0].1, &error_code_args(2735, FEEDBACK_NOT_LIVING));
    assert_eq!(calls[1].1, &feedback_line(DEAD_TEXT));
}

#[tokio::test]
async fn a_stim_is_never_refused_for_headroom() {
    let mut mgr = mgr();
    set_pool(&mut mgr, HEALTH, 1000);
    set_pool(&mut mgr, FOCUS, 900);
    let sent = use_item(&mut mgr, &ChainEngine::new(), STIM_III).await;
    assert_eq!(consumes(&sent).len(), 1);
}

/// The 597 gate: a mission item bound to the Heal Focus filler must not
/// heal focus, and must not be consumed by the native path.
#[tokio::test]
async fn an_item_bound_to_the_597_placeholder_does_nothing_natively() {
    let mut mgr = mgr();
    let sent = use_item(&mut mgr, &ChainEngine::new(), QUEST_ITEM).await;
    assert!(consumes(&sent).is_empty(), "{sent:?}");
    assert!(method_calls(&sent).is_empty());
    assert_eq!(pool(&mgr, FOCUS), 100);
}

#[tokio::test]
async fn an_unwired_binding_falls_through_to_chains() {
    let mut mgr = mgr();
    let sent = use_item(&mut mgr, &ChainEngine::new(), SCANNER).await;
    assert!(sent.is_empty(), "{sent:?}");
}

#[tokio::test]
async fn an_instanceless_item_used_consumes_nothing() {
    let mut mgr = mgr();
    let (tx, mut rx) = mpsc::channel(8);
    fire_item_use(
        PLAYER,
        PLAYER_ID,
        0,
        SLAPPACK,
        &ChainEngine::new(),
        &tx,
        &mut mgr,
    )
    .await;
    assert!(consumes(&drain(&mut rx)).is_empty());
}

/// The refusal row is the negative log a SigNoz query finds: INFO,
/// `reason = already_at_max`, with the pool's values.
#[tokio::test]
async fn the_refusal_logs_reason_already_at_max() {
    let capture = LogCapture::install();
    let mut mgr = mgr();
    set_pool(&mut mgr, HEALTH, 1000);
    let _ = use_item(&mut mgr, &ChainEngine::new(), SLAPPACK).await;
    let row = capture
        .find_event(Level::INFO, "item use refused", "already_at_max")
        .expect("the refusal must log INFO reason=already_at_max");
    assert_eq!(row.fields.get("player_id").map(String::as_str), Some("42"));
    assert_eq!(row.fields.get("stat_cur").map(String::as_str), Some("1000"));
}

/// A slappack with its old chain restored (`change_stat +500` then
/// `remove_item`, what chain 4001 was) must still heal once and consume
/// once: the chain owns the item and the native path stands aside.
#[tokio::test]
async fn a_restored_item_use_chain_owns_the_item_so_nothing_doubles() {
    let mut mgr = mgr();
    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 4001,
        name: "Health Slappack TC1 (restored)".to_string(),
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
    let sent = use_item(&mut mgr, &engine, SLAPPACK).await;
    assert!(
        consumes(&sent).is_empty(),
        "the native path must not also consume: {sent:?}"
    );
    let removes = sent
        .iter()
        .filter(|m| matches!(m, CellToBaseMsg::RemoveInventoryItem { .. }))
        .count();
    assert_eq!(removes, 1, "exactly one unit consumed");
    assert_eq!(pool(&mgr, HEALTH), 700, "exactly one +500 heal");
}

/// A bag consumable whose effect this path cannot apply is refused with a
/// line and kept: no `ConsumeItemForUse`, no chain, no `onErrorCode`.
#[tokio::test]
async fn an_unimplemented_bag_consumable_is_refused_and_kept() {
    let mut mgr = mgr();
    let sent = use_item(&mut mgr, &ChainEngine::new(), STEALTH_BOOST).await;
    assert!(consumes(&sent).is_empty(), "nothing consumed: {sent:?}");
    assert_eq!(
        method_calls(&sent),
        vec![(
            crate::mercury::method_idx::ON_PLAYER_COMMUNICATION,
            &feedback_line(NOT_IMPLEMENTED_TEXT)
        )],
        "exactly the chat line"
    );
}

#[tokio::test]
async fn the_not_implemented_refusal_logs_its_reason() {
    let capture = LogCapture::install();
    let mut mgr = mgr();
    let _ = use_item(&mut mgr, &ChainEngine::new(), STEALTH_BOOST).await;
    let row = capture
        .find_event(
            Level::WARN,
            "item use refused",
            "consumable_not_implemented",
        )
        .expect("WARN reason=consumable_not_implemented");
    assert_eq!(
        row.fields.get("item_type_id").map(String::as_str),
        Some("6206")
    );
    assert_eq!(
        row.fields.get("item_id").map(String::as_str),
        Some(INSTANCE.to_string().as_str())
    );
}

/// A mission item with an unwired binding stays silent: its chains decide.
#[tokio::test]
async fn an_unwired_mission_item_gets_no_refusal() {
    let mut mgr = mgr();
    let sent = use_item(&mut mgr, &ChainEngine::new(), SCANNER).await;
    assert!(method_calls(&sent).is_empty(), "{sent:?}");
}

/// The 597 filler stays silent even for a bag item: no new refusal.
#[tokio::test]
async fn a_597_bound_bag_item_gets_no_refusal() {
    let mut mgr = mgr();
    assert_eq!(mgr.item_containers.get(&QUEST_ITEM), Some(&1));
    let sent = use_item(&mut mgr, &ChainEngine::new(), QUEST_ITEM).await;
    assert!(sent.is_empty(), "the 597 path must send nothing: {sent:?}");
}

/// A chain for an unimplemented bag consumable owns it: no refusal.
#[tokio::test]
async fn a_chain_owned_unimplemented_consumable_gets_no_refusal() {
    let mut mgr = mgr();
    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 9001,
        name: "stealth boost chain".to_string(),
        enabled: true,
        trigger: Trigger::OnItemUse {
            item_id: STEALTH_BOOST,
        },
        conditions: vec![],
        actions: vec![],
        action_delays: vec![],
        priority: 0,
        once: false,
    });
    let sent = use_item(&mut mgr, &engine, STEALTH_BOOST).await;
    assert!(method_calls(&sent).is_empty(), "{sent:?}");
}

// ── phase 2: after the base consumed ───────────────────────────────────

fn consumed(type_id: i32) -> ItemUseConsumed {
    ItemUseConsumed {
        entity_id: PLAYER,
        player_id: PLAYER_ID,
        instance_id: INSTANCE,
        type_id,
    }
}

#[tokio::test]
async fn a_consumed_slappack_heals_500_clamped_at_max() {
    let mut mgr = mgr();
    let (tx, mut rx) = mpsc::channel(64);
    apply_consumed_item(consumed(SLAPPACK), &tx, &mut mgr).await;
    assert_eq!(pool(&mgr, HEALTH), 700);
    apply_consumed_item(consumed(SLAPPACK), &tx, &mut mgr).await;
    apply_consumed_item(consumed(SLAPPACK), &tx, &mut mgr).await;
    assert_eq!(pool(&mgr, HEALTH), 1000, "clamped at max");
    assert!(
        method_calls(&drain(&mut rx))
            .iter()
            .any(|(m, _)| *m == crate::mercury::method_idx::ON_STAT_UPDATE),
        "the heal reaches the client"
    );
}

#[tokio::test]
async fn a_consumed_mark_v_stim_buffs_both_stats_and_sends_both_icons() {
    let mut mgr = mgr();
    let (tx, mut rx) = mpsc::channel(64);
    apply_consumed_item(consumed(STIM_V), &tx, &mut mgr).await;
    let e = mgr.get_entity(PLAYER).unwrap();
    assert_eq!(e.stats.get(COORDINATION).unwrap().cur, 17);
    assert_eq!(e.stat_buffs.entries.len(), 2);
    assert!(e.stat_buffs.entries.iter().all(|b| b.timer_sent));
    let icons: Vec<i32> = method_calls(&drain(&mut rx))
        .into_iter()
        .filter(|(m, _)| *m == crate::cell::client_methods::being::ON_TIMER_UPDATE)
        .map(|(_, a)| i32::from_le_bytes(a[..4].try_into().unwrap()))
        .collect();
    assert_eq!(icons.len(), 2, "one duration icon per effect: {icons:?}");
    assert!(icons.contains(&3955) && icons.contains(&3956));
}

#[tokio::test]
async fn a_second_different_stim_keeps_the_first_buff() {
    let mut mgr = mgr();
    let (tx, _rx) = mpsc::channel(64);
    apply_consumed_item(consumed(STIM_III), &tx, &mut mgr).await;
    // Mark V Engagement/Coordination: Coordination is replaced (+7 wins),
    // Engagement is new.
    apply_consumed_item(consumed(STIM_V), &tx, &mut mgr).await;
    let e = mgr.get_entity(PLAYER).unwrap();
    assert_eq!(e.stats.get(COORDINATION).unwrap().cur, 17);
    assert_eq!(e.stat_buffs.entries.len(), 2);
}

#[tokio::test]
async fn a_consume_for_another_player_applies_nothing() {
    let mut mgr = mgr();
    let (tx, _rx) = mpsc::channel(64);
    let mut msg = consumed(SLAPPACK);
    msg.player_id = PLAYER_ID + 1;
    apply_consumed_item(msg, &tx, &mut mgr).await;
    assert_eq!(pool(&mgr, HEALTH), 200);
}

#[tokio::test]
async fn a_user_who_died_before_the_consume_landed_is_not_healed() {
    let capture = LogCapture::install();
    let mut mgr = mgr();
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .set_state_flag(crate::cell::combat::BSF_DEAD);
    let (tx, _rx) = mpsc::channel(64);
    apply_consumed_item(consumed(SLAPPACK), &tx, &mut mgr).await;
    assert_eq!(pool(&mgr, HEALTH), 200);
    assert!(capture
        .find_event(Level::WARN, "effect was not applied", "dead_at_apply")
        .is_some());
}

/// The refusal codes, read from the enum file the client parses
/// (`entities/defs/enumerations.xml`), not restated as literals.
#[test]
fn the_refusal_codes_are_the_client_enum_values() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../entities/defs/enumerations.xml");
    let xml = std::fs::read_to_string(&path).expect("enumerations.xml");
    let value = |name: &str| -> u16 {
        let line = xml
            .lines()
            .find(|l| l.contains(&format!("<Name>{name}</Name>")))
            .unwrap_or_else(|| panic!("{name} missing from enumerations.xml"));
        let v = line
            .split("<Value>")
            .nth(1)
            .unwrap()
            .split("</Value>")
            .next()
            .unwrap();
        v.trim().parse().unwrap()
    };
    assert_eq!(
        FEEDBACK_STAT_AT_MAX,
        value("CONDITION_FEEDBACK_StatValueGreaterThanOrEqual")
    );
    assert_eq!(FEEDBACK_NOT_LIVING, value("CONDITION_FEEDBACK_NotLiving"));
}
