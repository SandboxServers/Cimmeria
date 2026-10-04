//! Explosive-round splash through `apply_damage_to_target` (ammo campaign
//! AM-10): a shot whose on-hit effect is a `TCM_AERadius` splash deals a
//! share of the shot to the other hostiles within the radius of its target,
//! never to the shooter or a friendly, never through a wall, and never
//! chains.
//!
//! Like `ammo_tests`, these turn `ammo.finite_special` on for the process
//! and never off (see that module's docs for why that is safe).

use super::tests::make_ability;
use super::*;
use crate::cell::spawner::{AmmoCatalog, AmmoModifier};
use crate::test_support::occluder_fixtures::synthetic;
use crate::test_support::LogCapture;
use cimmeria_cell_world::cell::effects::ammo_explosive::SPLASH_FRACTION_NVP;
use cimmeria_entity::abilities::{EffectDef, TCM_AE_RADIUS, TCM_SINGLE};
use cimmeria_entity::ammo_type::{BULLET_DEFAULT, BULLET_EXPLOSIVE};
use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_entity::stats::AMMO_SLOT_1;

const WORLD: &str = "Castle";
const SHOOTER: u32 = 1;
const PRIMARY: u32 = 2;
const ABILITY: i32 = 7;
const EFFECT: i32 = 100;
/// The seeded splash effect id (`ammo_modifiers_explosive.sql`).
const SPLASH_EFFECT: i32 = 9130;
const HP: i32 = 100_000;
/// The primary target stands here; every splash distance is from it.
const AT: [f32; 3] = [10.0, 0.0, 10.0];

/// The seeded shape: a Short (5 m) radius, half the shot.
fn splash_effect(tcm: &str) -> EffectDef {
    let mut params = std::collections::HashMap::new();
    params.insert(SPLASH_FRACTION_NVP.to_string(), "0.5".to_string());
    EffectDef {
        effect_id: SPLASH_EFFECT,
        target_collection_method: tcm.to_string(),
        tcm_param1: "Short".to_string(),
        params,
        ..Default::default()
    }
}

fn explosive_row() -> AmmoModifier {
    AmmoModifier {
        ammo_type: BULLET_EXPLOSIVE,
        damage_mult: 1.1,
        penetration_mult: 0.5,
        damage_type: Some(i32::from(DT_PHYSICAL)),
        on_hit_effect_id: Some(SPLASH_EFFECT),
        toggle_ability_id: 1446,
        beneficial: false,
    }
}

fn npc(mgr: &mut SpaceManager, eid: u32, pos: [f32; 3], hostile: bool) {
    mgr.spawn_npc(eid, WORLD, pos, [0.0; 3]).unwrap();
    let e = mgr.get_entity_mut(eid).unwrap();
    e.faction = if hostile {
        crate::cell::combat::HOSTILE_FACTION
    } else {
        // Any non-hostile faction: a friendly or neutral NPC.
        crate::cell::combat::HOSTILE_FACTION + 1
    };
    e.stats.get_mut(HEALTH).unwrap().update(0, HP, HP);
}

/// Player `SHOOTER` at the origin with a pistol loaded with `ammo_type`,
/// ability 7 (a one-round weapon shot of 1000 HealthDamage) and a hostile
/// primary target at [`AT`]. `setup` adds the bystanders.
fn scene(ammo_type: i32, splash_tcm: &str, setup: impl FnOnce(&mut SpaceManager)) -> SpaceManager {
    cimmeria_entity::ammo_feature::set_finite_special(true);
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(SHOOTER, WORLD, [0.0; 3], [0.0; 3])
        .unwrap();
    npc(&mut mgr, PRIMARY, AT, true);

    let mut ability = make_ability(ABILITY, vec![EFFECT]);
    ability.required_ammo = 1;
    ability.is_ranged = true;
    ability.cooldown = 0.0;
    mgr.ability_defs.insert(ABILITY, ability);
    let mut params = std::collections::HashMap::new();
    params.insert("HealthDamage".to_string(), "1000".to_string());
    mgr.effect_defs.insert(
        EFFECT,
        EffectDef {
            effect_id: EFFECT,
            params,
            ..Default::default()
        },
    );
    mgr.effect_defs
        .insert(SPLASH_EFFECT, splash_effect(splash_tcm));
    mgr.ammo_catalog = AmmoCatalog::from_rows(vec![explosive_row()], [(BULLET_EXPLOSIVE, 9004)]);

    let p = mgr.get_entity_mut(SHOOTER).unwrap();
    p.is_player = true;
    p.player_id = Some(100);
    p.weapon_holstered = false;
    p.abilities.add_ability(ABILITY);
    p.active_bandolier_slot = 0;
    p.bandolier_items.insert(
        0,
        BandolierItem {
            instance_id: 1,
            item_id: 3241,
            clip_size: 15,
            default_ammo_type: BULLET_DEFAULT,
            current_ammo: 15,
            cur_ammo_type: ammo_type,
        },
    );
    if let Some(s) = p.stats.get_mut(AMMO_SLOT_1) {
        s.update(0, 15, 15);
        s.clear_dirty();
    }
    setup(&mut mgr);
    mgr.connect_entity(SHOOTER);
    let _ = mgr.compute_aoi_changes();
    mgr
}

fn lost(mgr: &SpaceManager, eid: u32) -> i32 {
    HP - mgr.get_entity(eid).unwrap().stats.get(HEALTH).unwrap().cur
}

/// The first `effect_seq` whose primary roll is a plain hit (a miss runs no
/// on-hit effect, so it would splash nothing).
fn hit_seq(mgr: &SpaceManager) -> u32 {
    let qr = combat::calculate_qr(
        &mgr.get_entity(SHOOTER).unwrap().stats,
        &mgr.get_entity(PRIMARY).unwrap().stats,
        true,
    );
    (1..500)
        .find(|&s| {
            combat::calculate_result(qr, pseudo_random_seed(SHOOTER, ABILITY, s)).result_code
                == cimmeria_entity::abilities::RC_HIT
        })
        .expect("a plain hit in 500 rolls")
}

async fn shoot(mgr: &mut SpaceManager) {
    let seq = hit_seq(mgr);
    let ability = mgr.ability_defs.get(&ABILITY).cloned();
    let (tx, _rx) = mpsc::channel(4096);
    apply_damage_to_target(SHOOTER, PRIMARY, ABILITY, &ability, seq, true, &tx, mgr).await;
}

/// A hostile 3 m from the target is splashed for a reduced share; one 7 m
/// away (outside the 5 m radius) is not. Fails if the splash is not wired,
/// or if the radius is ignored.
#[tokio::test]
async fn splash_hits_a_hostile_in_radius_and_not_one_outside() {
    let mut mgr = scene(BULLET_EXPLOSIVE, TCM_AE_RADIUS, |m| {
        npc(m, 3, [13.0, 0.0, 10.0], true);
        npc(m, 4, [10.0, 0.0, 17.0], true);
    });
    shoot(&mut mgr).await;
    let primary = lost(&mgr, PRIMARY);
    let inside = lost(&mgr, 3);
    assert!(primary > 0, "the shot hits its target");
    assert!(inside > 0, "the hostile 3 m away is splashed");
    assert!(
        inside < primary,
        "splash {inside} is reduced from the shot's {primary}"
    );
    assert_eq!(lost(&mgr, 4), 0, "7 m away is outside the 5 m radius");
}

/// The splash never reaches the shooter (standing 2 m from the target), a
/// friendly NPC, or a player bystander who is not a duel partner, all within
/// the radius. Fails if the splash stops filtering through the area rule.
#[tokio::test]
async fn splash_never_hits_the_shooter_or_a_friendly() {
    let mut mgr = scene(BULLET_EXPLOSIVE, TCM_AE_RADIUS, |m| {
        m.get_entity_mut(SHOOTER).unwrap().position = cimmeria_common::Vector3::new(8.0, 0.0, 10.0);
        npc(m, 3, [12.0, 0.0, 10.0], false);
        m.create_entity(5, WORLD, [10.0, 0.0, 12.0], [0.0; 3])
            .unwrap();
        let other = m.get_entity_mut(5).unwrap();
        other.is_player = true;
        other.player_id = Some(105);
        other.stats.get_mut(HEALTH).unwrap().update(0, HP, HP);
        // A hostile in range too, so the splash demonstrably ran.
        npc(m, 4, [10.0, 0.0, 8.0], true);
    });
    shoot(&mut mgr).await;
    assert!(lost(&mgr, 4) > 0, "the splash ran");
    assert_eq!(lost(&mgr, 3), 0, "a friendly NPC is never splashed");
    assert_eq!(lost(&mgr, 5), 0, "a non-duel player is never splashed");
    let shooter = mgr.get_entity(SHOOTER).unwrap().stats.get(HEALTH).unwrap();
    assert_eq!(shooter.cur, shooter.max, "the shooter is never splashed");
}

/// Splash does not chain: a hostile 4 m from a splashed hostile but 8 m from
/// the target is untouched, and the splashed one is hit exactly once (its
/// onEffectResults count). Fails if a splash target runs the on-hit effect
/// again.
#[tokio::test]
async fn splash_does_not_chain() {
    let mut mgr = scene(BULLET_EXPLOSIVE, TCM_AE_RADIUS, |m| {
        npc(m, 3, [14.0, 0.0, 10.0], true);
        npc(m, 4, [18.0, 0.0, 10.0], true);
    });
    let logs = LogCapture::install();
    shoot(&mut mgr).await;
    assert!(lost(&mgr, 3) > 0, "the first ring is splashed");
    assert_eq!(lost(&mgr, 4), 0, "a splash target does not splash");
    let splashes: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|c| c.target == "ammo" && c.has_field("event", "ammo_splash"))
        .collect();
    assert_eq!(splashes.len(), 1, "one splash per shot: {splashes:?}");
    let s = &splashes[0];
    for (k, v) in [
        ("player_id", "100"),
        ("entity_id", "1"),
        ("target_entity_id", "2"),
        ("effect_id", "9130"),
        ("splash_count", "1"),
        ("los_blocked", "0"),
    ] {
        assert!(s.has_field(k, v), "field {k}={v} missing: {s:?}");
    }
}

/// A hostile inside the radius but behind a wall from the target is not
/// splashed; one in the open is. Fails if the blast ignores the occluder.
#[tokio::test]
async fn splash_does_not_pass_through_a_wall() {
    let mut mgr = scene(BULLET_EXPLOSIVE, TCM_AE_RADIUS, |m| {
        // A 4 m wall at x 12, z 0-20: 3 is 4 m from the target behind it,
        // 4 is 3 m from the target on the same side.
        let sid = m.get_entity_space_id(SHOOTER).unwrap();
        m.spaces.get_mut(&sid).unwrap().occluder =
            Some(synthetic(&[([11.85, 0.0, 0.0], [12.15, 4.0, 20.0])]));
        npc(m, 3, [14.0, 0.0, 10.0], true);
        npc(m, 4, [10.0, 0.0, 13.0], true);
    });
    shoot(&mut mgr).await;
    assert!(lost(&mgr, 4) > 0, "in the open: splashed");
    assert_eq!(lost(&mgr, 3), 0, "behind the wall: not splashed");
}

/// Only a radius on-hit effect splashes, and only Explosive rounds carry
/// one: with default ammo, or a single-target on-hit effect, the bystander
/// is untouched. Fails if the pipeline splashes on any on-hit effect.
#[tokio::test]
async fn only_a_radius_on_hit_effect_splashes() {
    let near = |m: &mut SpaceManager| npc(m, 3, [13.0, 0.0, 10.0], true);
    let mut default_ammo = scene(BULLET_DEFAULT, TCM_AE_RADIUS, near);
    shoot(&mut default_ammo).await;
    assert!(lost(&default_ammo, PRIMARY) > 0);
    assert_eq!(lost(&default_ammo, 3), 0, "default ammo does not splash");

    let mut single = scene(BULLET_EXPLOSIVE, TCM_SINGLE, near);
    shoot(&mut single).await;
    assert!(lost(&single, PRIMARY) > 0);
    assert_eq!(
        lost(&single, 3),
        0,
        "a single-target on-hit effect does not splash"
    );
}

/// Smoke: a real `useAbility` shot (launch, ammo consume, fire) with
/// Explosive rounds loaded splashes the hostile next to its target.
#[tokio::test]
async fn an_explosive_shot_through_use_ability_splashes() {
    let mut mgr = scene(BULLET_EXPLOSIVE, TCM_AE_RADIUS, |m| {
        npc(m, 3, [13.0, 0.0, 10.0], true);
    });
    // `useAbility` rolls with the ability manager's next effect id; a miss
    // is possible, so fire until one hits (the ability has no cooldown).
    let (tx, _rx) = mpsc::channel(4096);
    for _ in 0..10 {
        crate::cell::abilities::handle_use_ability(SHOOTER, ABILITY, PRIMARY as i32, &tx, &mut mgr)
            .await;
        if lost(&mgr, PRIMARY) > 0 {
            break;
        }
    }
    assert!(lost(&mgr, PRIMARY) > 0, "the shot hit");
    assert!(
        lost(&mgr, 3) > 0,
        "the hostile next to the target is splashed"
    );
}

/// A splash kill earns mission credit on the shot that made it, even when
/// the primary target survives: the kill-credit wrapper fires
/// `EntityDeath` for the splashed NPC's tag, and nothing is left on the
/// scratchpad for a later cast. Fails if the wrapper only drains the
/// scratchpad when the primary died.
#[tokio::test]
async fn a_splash_kill_is_credited_when_the_primary_survives() {
    use crate::test_support::{RecordedContentEvent, RecordingContentEvents};
    let mut mgr = scene(BULLET_EXPLOSIVE, TCM_AE_RADIUS, |m| {
        npc(m, 3, [13.0, 0.0, 10.0], true);
        let e = m.get_entity_mut(3).unwrap();
        e.tag = Some("Splash_Victim".to_string());
        // One hit point: any splash that lands kills it.
        e.stats.get_mut(HEALTH).unwrap().update(0, 1, HP);
    });
    let events = RecordingContentEvents::new();
    let (tx, _rx) = mpsc::channel(4096);
    for _ in 0..10 {
        crate::cell::abilities::handle_use_ability_with_kill_credit(
            SHOOTER,
            ABILITY,
            PRIMARY as i32,
            &events,
            &tx,
            &mut mgr,
        )
        .await;
        if lost(&mgr, PRIMARY) > 0 {
            break;
        }
    }
    assert!(lost(&mgr, PRIMARY) > 0, "the shot hit");
    assert!(
        mgr.get_entity(PRIMARY)
            .unwrap()
            .stats
            .get(HEALTH)
            .unwrap()
            .cur
            > 0,
        "the primary survives"
    );
    let deaths: Vec<_> = events
        .events()
        .into_iter()
        .filter(|e| matches!(e, RecordedContentEvent::EntityDeath { .. }))
        .collect();
    assert_eq!(
        deaths,
        vec![RecordedContentEvent::EntityDeath {
            killer_entity_id: SHOOTER,
            player_id: 100,
            entity_tag: "Splash_Victim".to_string(),
        }]
    );
    assert!(mgr.get_entity(SHOOTER).unwrap().last_aoe_deaths.is_empty());
}

/// The same for a ground cast fired with Explosive rounds: the splash kill
/// around the cast's target is not in the list the ground path returns, so
/// `credit_ground_deaths` drains the scratchpad too. Fails if it credits
/// only the ground path's own deaths.
#[tokio::test]
async fn a_splash_kill_from_a_ground_cast_is_credited() {
    use crate::test_support::{RecordedContentEvent, RecordingContentEvents};
    let mut mgr = scene(BULLET_EXPLOSIVE, TCM_AE_RADIUS, |m| {
        // A 1 m ground radius: only the primary is in the cast's own area.
        m.effect_defs
            .get_mut(&EFFECT)
            .unwrap()
            .params
            .insert("Radius".to_string(), "1.0".to_string());
        npc(m, 3, [13.0, 0.0, 10.0], true);
        let e = m.get_entity_mut(3).unwrap();
        e.tag = Some("Splash_Victim".to_string());
        e.stats.get_mut(HEALTH).unwrap().update(0, 1, HP);
    });
    let events = RecordingContentEvents::new();
    let (tx, _rx) = mpsc::channel(4096);
    for _ in 0..10 {
        let deaths = crate::cell::abilities::handle_use_ability_on_ground(
            SHOOTER, ABILITY, AT, &tx, &mut mgr,
        )
        .await;
        assert!(
            deaths.is_empty(),
            "the ground path saw no death: {deaths:?}"
        );
        crate::cell::abilities::credit_ground_deaths(SHOOTER, deaths, &events, &tx, &mut mgr).await;
        if lost(&mgr, PRIMARY) > 0 {
            break;
        }
    }
    assert!(lost(&mgr, PRIMARY) > 0, "the cast hit");
    let deaths: Vec<_> = events
        .events()
        .into_iter()
        .filter(|e| matches!(e, RecordedContentEvent::EntityDeath { .. }))
        .collect();
    assert_eq!(
        deaths,
        vec![RecordedContentEvent::EntityDeath {
            killer_entity_id: SHOOTER,
            player_id: 100,
            entity_tag: "Splash_Victim".to_string(),
        }]
    );
    assert!(mgr.get_entity(SHOOTER).unwrap().last_aoe_deaths.is_empty());
}

/// **Regression guard (AB-07, the AB-03 review carry-over).** A shot with a
/// DoT splashes its direct hit only: the splash target loses the same with
/// the DoT on the ability as without it. Before AB-07 the splash target
/// took the DoT's first tick (at the splash fraction) with no DoT
/// registered behind it, so the two losses differed.
#[tokio::test]
async fn splash_takes_the_direct_hit_not_the_dot() {
    async fn splashed(with_dot: bool) -> i32 {
        const DOT: i32 = 101;
        let mut mgr = scene(BULLET_EXPLOSIVE, TCM_AE_RADIUS, |m| {
            npc(m, 3, [13.0, 0.0, 10.0], true);
            if with_dot {
                let mut params = std::collections::HashMap::new();
                params.insert("HealthDamage".to_string(), "5000".to_string());
                m.effect_defs.insert(
                    DOT,
                    EffectDef {
                        effect_id: DOT,
                        pulse_count: 8,
                        pulse_duration: 1.0,
                        params,
                        ..Default::default()
                    },
                );
                m.ability_defs
                    .get_mut(&ABILITY)
                    .unwrap()
                    .effect_ids
                    .push(DOT);
            }
        });
        shoot(&mut mgr).await;
        lost(&mgr, 3)
    }
    let (plain, with_dot) = (splashed(false).await, splashed(true).await);
    assert!(plain > 0, "the splash ran");
    assert_eq!(
        with_dot, plain,
        "the DoT's tick never reaches a splash target"
    );
}
