//! AB-08: a stat passive (`EF_AlwaysPersist` + `TimedStat`) is a held entry
//! on the timed effect ledger from the moment the ability is known: world
//! entry, a purchase, and off again on a respec, with the stat update the
//! client needs each time.
//!
//! The fixture is 1731 Warrior's Resilience as the `stat` family writes it:
//! effect 2645 "Kinetic Resists Increased: +15%", flags 524305, NVP
//! `KineticResistance` 150 (D-AB09: 10 points per 1 %).

use super::*;
use crate::ability_tree::RespecOutcome;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::spawner::{load_ability_defs, load_effect_defs};
use crate::test_support::require_db_or_skip;
use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::cell_entity::TreeProgress;
use cimmeria_entity::stats::{COVER_ACCURACY, DEFENSE, KINETIC_RES};

const PLAYER: u32 = 1;
const PLAYER_ID: i32 = 100;
const WARRIORS_RESILIENCE: i32 = 1731;

fn fixture() -> SpaceManager {
    let mut mgr = crate::test_support::make_space_manager();
    crate::test_support::install_effect_scripts(&mut mgr);
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.is_player = true;
        p.player_id = Some(PLAYER_ID);
        p.archetype_id = Some(1);
        p.level = 10;
    }
    crate::test_support::seed_ability_defs(&mut mgr, &[WARRIORS_RESILIENCE]);
    mgr.ability_defs
        .get_mut(&WARRIORS_RESILIENCE)
        .unwrap()
        .effect_ids = vec![2645];
    mgr.effect_defs.insert(
        2645,
        EffectDef {
            effect_id: 2645,
            ability_id: WARRIORS_RESILIENCE,
            flags: 524_305,
            pulse_count: 1,
            pulse_duration: 0.0,
            script_name: Some("TimedStat".to_string()),
            params: [("KineticResistance".to_string(), "150".to_string())].into(),
            ..Default::default()
        },
    );
    mgr
}

fn init_msg(abilities: Vec<i32>) -> BaseToCellMsg {
    BaseToCellMsg::InitPlayerState {
        entity_id: PLAYER,
        player_id: PLAYER_ID,
        account_id: 6,
        world_name: "Agnos".into(),
        archetype_id: 1,
        saved_missions: vec![],
        abilities,
        active_bandolier_slot: 0,
        bandolier_items: vec![],
        system_options: cimmeria_entity::cell_entity::SystemOptions::default(),
        access_level: 0,
        known_stargates: vec![],
        tree_progress: TreeProgress::default(),
        level: 10,
        character_name: None,
        body_set: None,
        looted_containers: Vec::new(),
    }
}

/// Deliver `msg`; return the stat ids of every `onStatUpdate` the player's
/// own client got.
async fn deliver(mgr: &mut SpaceManager, msg: BaseToCellMsg) -> Vec<i32> {
    let (tx, mut rx) = mpsc::channel(512);
    let engine = ChainEngine::new();
    handle_base_message(msg, &tx, mgr, &engine, &[]).await;
    let mut stats = Vec::new();
    while let Ok(m) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id: PLAYER,
            method_index,
            args,
        } = m
        {
            if method_index == crate::mercury::method_idx::ON_STAT_UPDATE {
                // count:u32, then (id, min, cur, max) as i32 each.
                for chunk in args[4..].as_chunks::<16>().0 {
                    stats.push(i32::from_le_bytes(chunk[..4].try_into().unwrap()));
                }
            }
        }
    }
    stats
}

fn stat(mgr: &SpaceManager, id: i32) -> i32 {
    mgr.get_entity(PLAYER).unwrap().stats.get(id).unwrap().cur
}

/// **Regression guard (B-36).** World entry holds the passive's +150 and
/// tells the client: on revert of the `TimedStat` passive the resist stays
/// at the archetype's 40, and without the sync no `onStatUpdate` names it.
#[tokio::test]
async fn world_entry_holds_a_stat_passive_and_sends_it() {
    let mut mgr = fixture();
    mgr.connect_entity(PLAYER);
    let sent = deliver(&mut mgr, init_msg(vec![WARRIORS_RESILIENCE])).await;
    assert_eq!(
        stat(&mgr, KINETIC_RES),
        40 + 150,
        "archetype 40 + the passive"
    );
    let entries = &mgr.get_entity(PLAYER).unwrap().stat_buffs.entries;
    assert_eq!(entries.len(), 1);
    assert!(entries[0].expires_at.is_none(), "held, no expiry");
    assert!(sent.contains(&KINETIC_RES), "the client hears it: {sent:?}");
}

/// A second world entry (a relog reaching a live entity) does not stack it:
/// `InitPlayerState` empties the ledger before the passive pass.
#[tokio::test]
async fn a_second_world_entry_does_not_stack_the_passive() {
    let mut mgr = fixture();
    mgr.connect_entity(PLAYER);
    let _ = deliver(&mut mgr, init_msg(vec![WARRIORS_RESILIENCE])).await;
    let _ = deliver(&mut mgr, init_msg(vec![WARRIORS_RESILIENCE])).await;
    assert_eq!(stat(&mgr, KINETIC_RES), 40 + 150);
    assert_eq!(mgr.get_entity(PLAYER).unwrap().stat_buffs.entries.len(), 1);
}

/// A purchase holds it at once and sends it; a respec takes it off, sends
/// the restored value, and leaves no entry behind.
#[tokio::test]
async fn a_purchase_holds_the_passive_and_a_respec_removes_it() {
    let mut mgr = fixture();
    let base = stat(&mgr, KINETIC_RES);
    let sent = deliver(
        &mut mgr,
        BaseToCellMsg::AbilityGranted {
            entity_id: PLAYER,
            ability_id: WARRIORS_RESILIENCE,
            training_points: 0,
            tree_points_spent: 1,
        },
    )
    .await;
    assert_eq!(stat(&mgr, KINETIC_RES), base + 150);
    assert!(sent.contains(&KINETIC_RES), "purchase: {sent:?}");

    let sent = deliver(
        &mut mgr,
        BaseToCellMsg::AbilitiesReset {
            entity_id: PLAYER,
            player_id: PLAYER_ID,
            outcome: RespecOutcome::Reset {
                refunded: vec![WARRIORS_RESILIENCE],
                training_points: 1,
                naquadah: 0,
            },
        },
    )
    .await;
    assert_eq!(stat(&mgr, KINETIC_RES), base, "refunded: exactly restored");
    assert!(sent.contains(&KINETIC_RES), "respec: {sent:?}");
    assert!(mgr
        .get_entity(PLAYER)
        .unwrap()
        .stat_buffs
        .entries
        .is_empty());
}

/// **Live-DB (the packet's acceptance).** World entry on the real seed
/// holds the three stat passives the generator binds: 1731 Warrior's
/// Resilience (+150 Kinetic Resistance), 1450 Cover Penetration (+100 Cover
/// Accuracy), 1574 Create Density: Basic (+100 Defense).
#[tokio::test]
async fn world_entry_holds_the_seeded_stat_passives_live_db() {
    let pool = require_db_or_skip!();
    let mut mgr = fixture();
    mgr.ability_defs = load_ability_defs(&pool).await.expect("ability defs");
    mgr.effect_defs = load_effect_defs(&pool).await.expect("effect defs");
    mgr.connect_entity(PLAYER);
    let fresh = {
        let mut m = fixture();
        m.connect_entity(PLAYER);
        let _ = deliver(&mut m, init_msg(vec![])).await;
        [
            stat(&m, KINETIC_RES),
            stat(&m, COVER_ACCURACY),
            stat(&m, DEFENSE),
        ]
    };
    let _ = deliver(&mut mgr, init_msg(vec![1731, 1450, 1574])).await;
    assert_eq!(
        [
            stat(&mgr, KINETIC_RES),
            stat(&mgr, COVER_ACCURACY),
            stat(&mgr, DEFENSE),
        ],
        [fresh[0] + 150, fresh[1] + 100, fresh[2] + 100]
    );
    let mut effects: Vec<i32> = mgr
        .get_entity(PLAYER)
        .unwrap()
        .stat_buffs
        .entries
        .iter()
        .map(|b| b.effect_id)
        .collect();
    effects.sort_unstable();
    assert_eq!(effects, vec![1741, 2645, 4782]);
}
