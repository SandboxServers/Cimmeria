//! AT-10: a warmup ability fires after its warmup, not at launch.
//!
//! The fixture's ability 50 has a 1.5 s warmup and event set 900 with
//! `Ability_Begin` / `Ability_End` / `Ability_Interrupt` sequences, so every
//! phase leaves a distinct `onSequence` on the wire. Time is stepped by
//! handing `resolve_warmups` an explicit `now`.

use std::time::{Duration, Instant};

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::abilities::{serialize_timer_update, EffectDef};

use super::*;
use crate::cell::abilities::resolve_warmups;

pub(super) const WARMUP_ABILITY: i32 = 50;
pub(super) const INSTANT_ABILITY: i32 = 51;
pub(super) const EVENT_SET: i32 = 900;
pub(super) const SEQ_BEGIN: i32 = 9100;
pub(super) const SEQ_END: i32 = 9101;
pub(super) const SEQ_INTERRUPT: i32 = 9102;
pub(super) const WARMUP_SECS: f32 = 1.5;
/// `onTimerUpdate`; `method_idx` has no constant for it (callers use 12).
pub(super) const ON_TIMER_UPDATE: u16 = 12;
pub(super) const COOLDOWN_SECS: f32 = 2.0;

pub(super) fn cast_ability(id: i32, warmup: f32) -> AbilityDef {
    AbilityDef {
        ability_id: id,
        name: "charged".to_string(),
        cooldown: COOLDOWN_SECS,
        warmup,
        flags: 0,
        is_ranged: true,
        min_range: 0,
        max_range: 30,
        target_type_id: 0,
        effect_ids: vec![500],
        moniker_ids: vec![],
        required_ammo: 0,
        event_set_id: Some(EVENT_SET),
        velocity: 0.0,
    }
}

/// Player 1 at the origin and hostile NPC 2 three units away, in one
/// non-instanced space so AoI, witness routing and the fire-time space
/// check all see them together. The NPC has enough health that no test
/// kills it by accident.
pub(super) fn warmup_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    mgr.create_entity(2, "Castle", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(101);
        p.abilities.add_ability(WARMUP_ABILITY);
        p.abilities.add_ability(INSTANT_ABILITY);
    }
    if let Some(npc) = mgr.get_entity_mut(2) {
        npc.faction = crate::cell::combat::HOSTILE_FACTION;
        if let Some(stat) = npc.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            stat.update(0, 100_000, 100_000);
            stat.clear_dirty();
        }
    }
    mgr.connect_entity(1);
    let _ = mgr.compute_aoi_changes();

    mgr.ability_defs
        .insert(WARMUP_ABILITY, cast_ability(WARMUP_ABILITY, WARMUP_SECS));
    mgr.ability_defs
        .insert(INSTANT_ABILITY, cast_ability(INSTANT_ABILITY, 0.0));
    let mut params = std::collections::HashMap::new();
    params.insert("HealthDamage".to_string(), "5".to_string());
    mgr.effect_defs.insert(
        500,
        EffectDef {
            effect_id: 500,
            ability_id: 0,
            delay: 0,
            effect_sequence: 0,
            event_set_id: None,
            script_name: None,
            params,
            ..Default::default()
        },
    );
    mgr.sequence_map.insert((EVENT_SET, 1000), SEQ_BEGIN);
    mgr.sequence_map.insert((EVENT_SET, 1001), SEQ_END);
    mgr.sequence_map.insert((EVENT_SET, 1002), SEQ_INTERRUPT);
    mgr
}

/// `(entity_id, method_index, args)` of every entity-method message, in
/// send order, whether addressed to the entity or routed to a witness.
pub(super) fn calls(msgs: &[CellToBaseMsg]) -> Vec<(u32, u16, Vec<u8>)> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            }
            | CellToBaseMsg::WitnessEntityMethod {
                entity_id,
                method_index,
                args,
                ..
            } => Some((*entity_id, *method_index, args.clone())),
            _ => None,
        })
        .collect()
}

/// The `onSequence` ids (first 4 bytes of the args) sent by `entity`.
pub(super) fn sequences(msgs: &[CellToBaseMsg], entity: u32) -> Vec<i32> {
    calls(msgs)
        .into_iter()
        .filter(|(e, m, _)| *e == entity && *m == method_idx::ON_SEQUENCE)
        .map(|(_, _, a)| i32::from_le_bytes([a[0], a[1], a[2], a[3]]))
        .collect()
}

/// How many `onEffectResults` `entity` sent: one per resolved hit.
pub(super) fn effect_results(msgs: &[CellToBaseMsg], entity: u32) -> usize {
    calls(msgs)
        .into_iter()
        .filter(|(e, m, _)| *e == entity && *m == method_idx::ON_EFFECT_RESULTS)
        .count()
}

pub(super) fn after_warmup() -> Instant {
    Instant::now() + Duration::from_secs_f32(WARMUP_SECS) + Duration::from_millis(50)
}

/// **Regression guard (AT-10).** Before the fix `handle_use_ability` sent
/// `Ability_Begin`, `Ability_End` and the damage in one pass, so a charged
/// ability hit when the charge started. Now nothing resolves at launch,
/// nothing resolves while the warmup runs, and the hit lands exactly once
/// when it expires.
#[tokio::test]
async fn warmup_damage_waits_for_the_warmup_and_lands_once() {
    let mut mgr = warmup_mgr();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(256);

    let committed = handle_use_ability(1, WARMUP_ABILITY, 2, &tx, &mut mgr).await;
    assert!(committed, "the launch commits: the cooldown is charged");
    let launch = drain(&mut rx);
    assert_eq!(
        effect_results(&launch, 1),
        0,
        "no damage at launch; got {launch:?}"
    );
    assert!(mgr.get_entity(1).unwrap().pending_cast.is_some());

    // Mid-warmup tick: still nothing.
    let fired = resolve_warmups(Instant::now(), &tx, &mut mgr, &engine).await;
    assert_eq!(fired, 0);
    assert_eq!(effect_results(&drain(&mut rx), 1), 0);

    // Expired: exactly one hit.
    let fired = resolve_warmups(after_warmup(), &tx, &mut mgr, &engine).await;
    assert_eq!(fired, 1);
    let fire = drain(&mut rx);
    assert_eq!(
        effect_results(&fire, 1),
        1,
        "one hit at the fire; got {fire:?}"
    );
    assert!(mgr.get_entity(1).unwrap().pending_cast.is_none());
    assert!(mgr.pending_casts.is_empty());

    // A later tick (a replayed timer) fires nothing more.
    let fired = resolve_warmups(
        after_warmup() + Duration::from_secs(5),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;
    assert_eq!(fired, 0);
    assert_eq!(
        effect_results(&drain(&mut rx), 1),
        0,
        "the cast fired twice"
    );
}

/// Wire order across the delay, byte-exact for the launch burst.
///
/// Launch (python `launch()` order): the cooldown timer covering cooldown +
/// warmup, `Ability_Begin`, then the `AbilityWarmup` (type 1) timer. No
/// `Ability_End`. Fire: `Ability_End`, then the damage.
#[tokio::test]
async fn warmup_wire_is_begin_at_launch_then_end_at_fire() {
    let mut mgr = warmup_mgr();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(256);

    assert!(handle_use_ability(1, WARMUP_ABILITY, 2, &tx, &mut mgr).await);
    let launch = calls(&drain(&mut rx));
    let effect_seq = mgr
        .get_entity(1)
        .unwrap()
        .pending_cast
        .as_ref()
        .unwrap()
        .effect_seq;

    let mut begin = Vec::new();
    begin.extend_from_slice(&SEQ_BEGIN.to_le_bytes());
    begin.extend_from_slice(&1i32.to_le_bytes());
    begin.extend_from_slice(&2i32.to_le_bytes());
    begin.push(1);
    begin.extend_from_slice(&0.0f32.to_le_bytes());
    begin.extend_from_slice(&0u32.to_le_bytes());
    begin.push(0);
    begin.extend_from_slice(&effect_seq.to_le_bytes());

    let expected = vec![
        (
            1,
            ON_TIMER_UPDATE,
            serialize_timer_update(WARMUP_ABILITY, 2, 1, 0, COOLDOWN_SECS + WARMUP_SECS, 0.0),
        ),
        (1, method_idx::ON_SEQUENCE, begin),
        (
            1,
            ON_TIMER_UPDATE,
            serialize_timer_update(WARMUP_ABILITY, 1, 1, 0, WARMUP_SECS, 0.0),
        ),
    ];
    assert_eq!(launch, expected, "launch burst");

    resolve_warmups(after_warmup(), &tx, &mut mgr, &engine).await;
    let fire = drain(&mut rx);
    let fire_calls = calls(&fire);
    let end_at = fire_calls
        .iter()
        .position(|(e, m, a)| {
            *e == 1 && *m == method_idx::ON_SEQUENCE && a[0..4] == SEQ_END.to_le_bytes()
        })
        .expect("Ability_End at the fire");
    let hit_at = fire_calls
        .iter()
        .position(|(e, m, _)| *e == 1 && *m == method_idx::ON_EFFECT_RESULTS)
        .expect("the hit at the fire");
    assert!(end_at < hit_at, "Ability_End precedes the damage");
    assert_eq!(
        &fire_calls[end_at].2[22..26],
        &effect_seq.to_le_bytes(),
        "Ability_End carries the launch's InstanceId"
    );
    assert!(
        !sequences(&fire, 1).contains(&SEQ_BEGIN),
        "Ability_Begin is not re-sent at the fire"
    );
}

/// **No-change guard.** A zero-warmup ability's wire is what it was before
/// AT-10: cooldown timer, `Ability_End`, then the damage, all in the launch
/// pass, with no `Ability_Begin` and no warmup timer. The timer and
/// `Ability_End` are compared byte for byte. This test passes on the pre-fix
/// code too; that is the point.
#[tokio::test]
async fn zero_warmup_wire_is_unchanged() {
    let mut mgr = warmup_mgr();
    let (tx, mut rx) = mpsc::channel(256);

    assert!(handle_use_ability(1, INSTANT_ABILITY, 2, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);
    let c = calls(&msgs);

    let effect_seq = 1i32; // first `next_effect_id()` on a fresh manager
    let mut end = Vec::new();
    end.extend_from_slice(&SEQ_END.to_le_bytes());
    end.extend_from_slice(&1i32.to_le_bytes());
    end.extend_from_slice(&2i32.to_le_bytes());
    end.push(1);
    end.extend_from_slice(&0.0f32.to_le_bytes());
    end.extend_from_slice(&0u32.to_le_bytes());
    end.push(0);
    end.extend_from_slice(&effect_seq.to_le_bytes());

    assert_eq!(
        c[0],
        (
            1,
            ON_TIMER_UPDATE,
            serialize_timer_update(INSTANT_ABILITY, 2, 1, 0, COOLDOWN_SECS, 0.0)
        ),
        "first: the plain cooldown timer"
    );
    assert_eq!(
        c[1],
        (1, method_idx::ON_SEQUENCE, end),
        "second: Ability_End"
    );
    assert_eq!(
        effect_results(&msgs, 1),
        1,
        "the hit lands in the same pass"
    );
    assert_eq!(sequences(&msgs, 1), vec![SEQ_END], "no Ability_Begin");
    assert_eq!(
        c.iter()
            .filter(|(e, m, _)| *e == 1 && *m == ON_TIMER_UPDATE)
            .count(),
        1,
        "no warmup timer"
    );
}

/// One cast at a time (python `canUseAbility`: "already using an
/// ability"). A second press during the warmup, of the same or another
/// ability, is refused without charging a cooldown, and the warming cast
/// still fires once.
#[tokio::test]
async fn second_press_during_warmup_is_refused() {
    let mut mgr = warmup_mgr();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(256);

    assert!(handle_use_ability(1, WARMUP_ABILITY, 2, &tx, &mut mgr).await);
    drain(&mut rx);

    assert!(
        !handle_use_ability(1, INSTANT_ABILITY, 2, &tx, &mut mgr).await,
        "another ability is refused while one warms up"
    );
    assert!(!mgr
        .get_entity(1)
        .unwrap()
        .abilities
        .is_on_cooldown(INSTANT_ABILITY));
    assert!(drain(&mut rx).is_empty(), "the refusal sends nothing");

    resolve_warmups(after_warmup(), &tx, &mut mgr, &engine).await;
    assert_eq!(effect_results(&drain(&mut rx), 1), 1);
}

/// An NPC caster uses the same path: its warmup ability does not hit the
/// player at launch and does hit when the warmup expires.
#[tokio::test]
async fn npc_caster_warms_up_through_the_same_path() {
    let mut mgr = warmup_mgr();
    let engine = ChainEngine::new();
    if let Some(npc) = mgr.get_entity_mut(2) {
        npc.abilities.add_ability(WARMUP_ABILITY);
    }
    let (tx, mut rx) = mpsc::channel(256);

    assert!(handle_use_ability(2, WARMUP_ABILITY, 1, &tx, &mut mgr).await);
    let launch = drain(&mut rx);
    assert_eq!(
        effect_results(&launch, 2),
        0,
        "no hit at launch; got {launch:?}"
    );
    assert_eq!(sequences(&launch, 2), vec![SEQ_BEGIN]);
    assert!(crate::cell::abilities::is_casting(&mgr, 2));

    resolve_warmups(after_warmup(), &tx, &mut mgr, &engine).await;
    let fire = drain(&mut rx);
    assert_eq!(
        effect_results(&fire, 2),
        1,
        "one hit at the fire; got {fire:?}"
    );
    assert_eq!(sequences(&fire, 2), vec![SEQ_END]);
}

/// Ground target (AT-10): the AoE secondaries wait for the primary's
/// warmup instead of being hit at launch.
#[tokio::test]
async fn ground_cast_secondaries_wait_for_the_warmup() {
    let mut mgr = warmup_mgr();
    let engine = ChainEngine::new();
    mgr.create_entity(3, "Castle", [3.5, 0.0, 0.0], [0.0; 3])
        .unwrap();
    // Ground collection only sees SGWMob-class NPCs.
    mgr.get_entity_mut(2).unwrap().class_id = 0x04;
    if let Some(npc) = mgr.get_entity_mut(3) {
        npc.class_id = 0x04;
        npc.faction = crate::cell::combat::HOSTILE_FACTION;
        if let Some(stat) = npc.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            stat.update(0, 100_000, 100_000);
            stat.clear_dirty();
        }
    }
    let (tx, mut rx) = mpsc::channel(256);

    let deaths = crate::cell::abilities::handle_use_ability_on_ground(
        1,
        WARMUP_ABILITY,
        [3.0, 0.0, 0.0],
        &tx,
        &mut mgr,
    )
    .await;
    assert!(deaths.is_empty());
    let launch = drain(&mut rx);
    assert_eq!(
        effect_results(&launch, 1),
        0,
        "no hit at launch; got {launch:?}"
    );
    assert_eq!(
        mgr.get_entity(1)
            .unwrap()
            .pending_cast
            .as_ref()
            .unwrap()
            .ground,
        Some([3.0, 0.0, 0.0])
    );

    resolve_warmups(after_warmup(), &tx, &mut mgr, &engine).await;
    let fire = drain(&mut rx);
    assert_eq!(
        effect_results(&fire, 1),
        2,
        "primary and secondary are hit at the fire; got {fire:?}"
    );
}

/// **Regression guard (review of AT-10).** The launch redirects Pistol Shot
/// (592) to the active weapon's ranged ability. A ground cast of 592 whose
/// redirected ability warms up must still defer its splash damage: matching
/// the parked cast by the client's ability id missed it and hit every
/// secondary at launch, for free once the cast was interrupted.
#[tokio::test]
async fn redirected_ground_cast_defers_its_splash_damage() {
    use cimmeria_entity::cell_entity::BandolierItem;

    let mut mgr = warmup_mgr();
    let engine = ChainEngine::new();
    mgr.create_entity(3, "Castle", [3.5, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(2).unwrap().class_id = 0x04;
    if let Some(npc) = mgr.get_entity_mut(3) {
        npc.class_id = 0x04;
        npc.faction = crate::cell::combat::HOSTILE_FACTION;
        if let Some(stat) = npc.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            stat.update(0, 100_000, 100_000);
            stat.clear_dirty();
        }
    }
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(592);
        p.weapon_holstered = false;
        p.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: 21,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 30,
                cur_ammo_type: 2,
            },
        );
    }
    // P90 (21) binds the warmup ability to RANGED (event 7).
    mgr.item_event_set_abilities.insert((21, 7), WARMUP_ABILITY);
    mgr.ability_defs.insert(592, cast_ability(592, 0.0));
    let (tx, mut rx) = mpsc::channel(256);

    crate::cell::abilities::handle_use_ability_on_ground(1, 592, [3.0, 0.0, 0.0], &tx, &mut mgr)
        .await;
    let launch = drain(&mut rx);
    assert_eq!(
        effect_results(&launch, 1),
        0,
        "no splash damage while the redirected primary warms up; got {launch:?}"
    );
    let pc = mgr.get_entity(1).unwrap().pending_cast.clone().unwrap();
    assert_eq!(pc.ability_id, WARMUP_ABILITY);
    assert_eq!(pc.ground, Some([3.0, 0.0, 0.0]));

    resolve_warmups(after_warmup(), &tx, &mut mgr, &engine).await;
    assert_eq!(effect_results(&drain(&mut rx), 1), 2);
}

/// Python `launch()`: `speedAttack` shortens an `SpeedAttack`-flagged
/// ability's warmup by 1% per point; without the flag the stat is ignored.
#[test]
fn speed_attack_stat_shortens_flagged_warmup() {
    use cimmeria_entity::abilities::AF_SPEED_ATTACK;
    use cimmeria_entity::stats::SPEED_ATTACK;

    let mut mgr = warmup_mgr();
    if let Some(p) = mgr.get_entity_mut(1) {
        if let Some(stat) = p.stats.get_mut(SPEED_ATTACK) {
            stat.update(0, 20, 100);
        }
    }
    let caster = mgr.get_entity(1).unwrap();
    let mut flagged = cast_ability(60, 2.0);
    flagged.flags = AF_SPEED_ATTACK;
    let unflagged = cast_ability(61, 2.0);
    let w = super::super::warmup::effective_warmup(Some(&flagged), caster);
    assert!((w - 1.6).abs() < 1e-5, "20% off 2.0 s, got {w}");
    let w = super::super::warmup::effective_warmup(Some(&unflagged), caster);
    assert!(
        (w - 2.0).abs() < 1e-5,
        "unflagged ability ignores the stat, got {w}"
    );
}
