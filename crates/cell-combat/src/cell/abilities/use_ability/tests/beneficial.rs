//! AB-01: Self casts land on the caster, heals on allies or fall back to the
//! caster (D-AB01, D-AB02 proposed default), and never on a hostile.
//!
//! `duel_gate`'s fixture: player A (1) casts, player B (2) is an ally at
//! (3,0,0), player C (3) a bystander, NPC 4 a hostile mob at (2,0,0); this
//! file adds a neutral NPC 5. Every entity starts at 500/1000 Health and
//! 0/1000 Focus with nothing dirty. The abilities copy the seed's shapes:
//! 597 Heal Focus (Self, `HealFocus` 35%), 1646 Health Heal (Target,
//! `HealHealth` 10%), 1218 Recuperation (Target, `HealHealth` 3% x 25 pulses).
//!
//! Before AB-01 the server took the client's target at its word: 597 with
//! nothing selected did nothing, with the caster or an ally selected was
//! refused by #444, and with a mob selected healed the mob (audit B-12 to
//! B-14).

use std::collections::HashMap;

use cimmeria_entity::abilities::{AbilityType, EffectDef, TARGET_SELF, TARGET_TARGET};
use cimmeria_entity::stats::{FOCUS, HEALTH};
use cimmeria_wire::state_field::BSF_IN_COMBAT;

use super::super::beneficial::{resolve_cast_target, CastTarget};
use super::duel_gate::{duel_mgr, A, B, MOB};
use super::warmup::{after_warmup, calls, effect_results, EVENT_SET, INSTANT_ABILITY, SEQ_END};
use super::*;
use crate::cell::abilities::resolve_warmups;
use crate::test_support::{LogCapture, NoContentEvents};

pub(super) const HEAL_FOCUS: i32 = 597;
const HEALTH_HEAL: i32 = 1646;
pub(super) const RECUPERATION: i32 = 1218;
/// 2228 `MS020_080818_CallTarget`'s shape: Heal-typed, but it deals damage.
const HEAL_TYPED_ATTACK: i32 = 2228;
const NEUTRAL: u32 = 5;
const MAX: i32 = 1000;
const START_HEALTH: i32 = 500;

fn effect(id: i32, ability: i32, script: &str, percent: &str, pulses: i32) -> EffectDef {
    EffectDef {
        effect_id: id,
        ability_id: ability,
        script_name: Some(script.to_string()),
        params: HashMap::from([("HealPercentage".to_string(), percent.to_string())]),
        pulse_count: pulses,
        pulse_duration: if pulses > 1 { 1.0 } else { 0.0 },
        ..Default::default()
    }
}

fn heal(id: i32, target_type_id: i32, effect_id: i32, warmup: f32) -> AbilityDef {
    AbilityDef {
        ability_id: id,
        name: "heal".to_string(),
        cooldown: 30.0,
        warmup,
        target_type_id,
        effect_ids: vec![effect_id],
        type_id: AbilityType::Heal,
        ..make_ability(id, 0, 30)
    }
}

/// The fixture (module docs). `warmup` applies to 597 only.
pub(super) fn heal_mgr(warmup: f32) -> SpaceManager {
    let mut mgr = duel_mgr();
    crate::test_support::install_effect_scripts(&mut mgr);
    mgr.spawn_npc(NEUTRAL, "Castle", [1.0, 0.0, 1.0], [0.0; 3])
        .unwrap();
    for (id, target_type, eid, w) in [
        (HEAL_FOCUS, TARGET_SELF, 659, warmup),
        (HEALTH_HEAL, TARGET_TARGET, 2008, 0.0),
        (RECUPERATION, TARGET_TARGET, 1383, 0.0),
        (HEAL_TYPED_ATTACK, TARGET_TARGET, 3091, 0.0),
    ] {
        mgr.ability_defs.insert(id, heal(id, target_type, eid, w));
    }
    mgr.effect_defs
        .insert(659, effect(659, HEAL_FOCUS, "HealFocus", "35", 1));
    mgr.effect_defs
        .insert(2008, effect(2008, HEALTH_HEAL, "HealHealth", "10", 1));
    mgr.effect_defs
        .insert(1383, effect(1383, RECUPERATION, "HealHealth", "3", 25));
    let mut attack = effect(3091, HEAL_TYPED_ATTACK, "unused", "0", 1);
    attack.script_name = None;
    attack.params = HashMap::from([("HealthDamage".to_string(), "30".to_string())]);
    mgr.effect_defs.insert(3091, attack);
    for eid in [A, B, 3, MOB, NEUTRAL] {
        let e = mgr.get_entity_mut(eid).unwrap();
        e.stats
            .get_mut(HEALTH)
            .unwrap()
            .update(0, START_HEALTH, MAX);
        e.stats.get_mut(FOCUS).unwrap().update(0, 0, MAX);
        e.stats.clear_dirty();
    }
    let a = mgr.get_entity_mut(A).unwrap();
    for id in [HEAL_FOCUS, HEALTH_HEAL, RECUPERATION, HEAL_TYPED_ATTACK] {
        a.abilities.add_ability(id);
    }
    mgr
}

fn pool(mgr: &SpaceManager, eid: u32, stat: i32) -> i32 {
    mgr.get_entity(eid).unwrap().stats.get(stat).unwrap().cur
}

// ── resolve_cast_target, arm by arm ─────────────────────────────────────

#[test]
fn a_self_ability_resolves_on_the_caster_whatever_the_wire_says() {
    let mgr = heal_mgr(0.0);
    let def = mgr.ability_defs.get(&HEAL_FOCUS);
    for wire in [0, A as i32, B as i32, MOB as i32, NEUTRAL as i32, 9999] {
        assert_eq!(
            resolve_cast_target(&mgr, A, def, wire),
            CastTarget::Caster,
            "597 with wire target {wire}"
        );
    }
}

#[test]
fn a_target_heal_resolves_on_the_caster_or_an_ally_it_names() {
    let mgr = heal_mgr(0.0);
    let def = mgr.ability_defs.get(&HEALTH_HEAL);
    assert_eq!(
        resolve_cast_target(&mgr, A, def, B as i32),
        CastTarget::Ally(B)
    );
    assert_eq!(
        resolve_cast_target(&mgr, A, def, A as i32),
        CastTarget::Ally(A)
    );
}

/// D-AB02's proposed default: a hostile, a neutral NPC, a dead ally, a
/// missing entity or no target at all falls back to the caster.
#[test]
fn a_target_heal_with_no_ally_falls_back_to_the_caster() {
    let mut mgr = heal_mgr(0.0);
    let dead_ally = 3;
    mgr.get_entity_mut(dead_ally).unwrap().state_field |= cimmeria_wire::state_field::BSF_DEAD;
    let def = mgr.ability_defs.get(&HEALTH_HEAL);
    for wire in [MOB as i32, NEUTRAL as i32, dead_ally as i32, 9999, 0] {
        assert_eq!(
            resolve_cast_target(&mgr, A, def, wire),
            CastTarget::Caster,
            "1646 with wire target {wire}"
        );
    }
}

#[test]
fn a_non_beneficial_or_npc_cast_keeps_the_wire_target() {
    let mgr = heal_mgr(0.0);
    let attack = mgr.ability_defs.get(&INSTANT_ABILITY);
    assert_eq!(
        resolve_cast_target(&mgr, A, attack, MOB as i32),
        CastTarget::Hostile(MOB)
    );
    assert_eq!(
        resolve_cast_target(&mgr, A, attack, B as i32),
        CastTarget::Hostile(B)
    );
    assert_eq!(resolve_cast_target(&mgr, A, attack, 0), CastTarget::None);
    // A Heal-typed ability that deals damage is not beneficial.
    let heal_typed_attack = mgr.ability_defs.get(&HEAL_TYPED_ATTACK);
    assert_eq!(
        resolve_cast_target(&mgr, A, heal_typed_attack, B as i32),
        CastTarget::Hostile(B)
    );
    // An NPC caster is never redirected (the NPC AI picks its own targets).
    let heal_focus = mgr.ability_defs.get(&HEAL_FOCUS);
    assert_eq!(
        resolve_cast_target(&mgr, MOB, heal_focus, A as i32),
        CastTarget::Hostile(A)
    );
}

// ── The pipeline ────────────────────────────────────────────────────────

/// **Regression guard (B-12, B-13, B-14).** 597 restores 35% of the
/// caster's Focus with nothing selected, the caster, an ally or a hostile
/// selected, and never touches anyone else's. On revert the hostile case
/// heals the mob (`MOB's Focus must not change` fails) and the other three
/// leave the caster at 0 (`A's Focus` fails).
#[tokio::test]
async fn heal_focus_restores_the_casters_focus_and_never_the_hostiles() {
    // The hostile first: on revert its assertion is the one that trips.
    for wire in [MOB as i32, 0, A as i32, B as i32] {
        let mut mgr = heal_mgr(0.0);
        let (tx, mut rx) = mpsc::channel(256);

        assert!(
            handle_use_ability(A, HEAL_FOCUS, wire, &tx, &mut mgr).await,
            "597 at wire target {wire} must commit"
        );
        let msgs = drain(&mut rx);

        assert_eq!(
            pool(&mgr, MOB, FOCUS),
            0,
            "MOB's Focus must not change (wire {wire})"
        );
        assert_eq!(
            pool(&mgr, B, FOCUS),
            0,
            "B's Focus must not change (wire {wire})"
        );
        assert_eq!(
            pool(&mgr, A, FOCUS),
            350,
            "A's Focus: 35% of 1000 (wire {wire})"
        );
        // No damage resolution: no QR result, no threat, no combat state.
        assert_eq!(
            effect_results(&msgs, A),
            0,
            "no onEffectResults (wire {wire})"
        );
        let a = mgr.get_entity(A).unwrap();
        assert!(a.threatened_mobs.is_empty(), "no threat (wire {wire})");
        assert_eq!(
            a.state_field & BSF_IN_COMBAT,
            0,
            "not in combat (wire {wire})"
        );
        assert!(mgr.get_entity(MOB).unwrap().threat_list.is_empty());
    }
}

/// **Regression guard (B-13).** 1646 at an ally heals the ally. On revert
/// #444 refuses the cast (`committed` fails).
#[tokio::test]
async fn health_heal_on_an_ally_heals_the_ally() {
    let mut mgr = heal_mgr(0.0);
    let (tx, mut rx) = mpsc::channel(256);

    let committed = handle_use_ability(A, HEALTH_HEAL, B as i32, &tx, &mut mgr).await;
    let msgs = drain(&mut rx);

    assert!(committed, "1646 at an ally must commit");
    assert_eq!(
        pool(&mgr, B, HEALTH),
        START_HEALTH + 100,
        "B healed 10% of 1000"
    );
    assert_eq!(
        pool(&mgr, A, HEALTH),
        START_HEALTH,
        "the caster is not healed"
    );
    assert!(
        calls(&msgs)
            .iter()
            .any(|(e, m, _)| *e == B && *m == method_idx::ON_STAT_UPDATE),
        "B's own client hears the heal"
    );
}

/// **Regression guard (B-14, D-AB02 default).** 1646 at a hostile heals the
/// caster, not the mob. On revert the mob is healed (`MOB's Health` fails).
#[tokio::test]
async fn health_heal_at_a_hostile_falls_back_to_the_caster() {
    let mut mgr = heal_mgr(0.0);
    let (tx, _rx) = mpsc::channel(256);

    assert!(handle_use_ability(A, HEALTH_HEAL, MOB as i32, &tx, &mut mgr).await);

    assert_eq!(
        pool(&mgr, MOB, HEALTH),
        START_HEALTH,
        "MOB's Health must not change"
    );
    assert_eq!(
        pool(&mgr, A, HEALTH),
        START_HEALTH + 100,
        "the caster is healed instead"
    );
}

/// **#444 guard.** A damaging ability at an ally is still refused, and so is
/// a Heal-typed ability that deals damage: neither commits, starts its
/// cooldown or touches the ally. Reverting the damage veto in
/// `ability_is_beneficial` lets 2228's shape commit at the ally
/// (`a Heal-typed attack` fails).
#[tokio::test]
async fn a_damaging_ability_at_an_ally_is_still_refused() {
    let mut mgr = heal_mgr(0.0);
    let (tx, _rx) = mpsc::channel(256);

    for id in [INSTANT_ABILITY, HEAL_TYPED_ATTACK] {
        let committed = handle_use_ability(A, id, B as i32, &tx, &mut mgr).await;
        assert!(
            !committed,
            "ability {id} at an ally must be refused (#444); a Heal-typed attack too"
        );
        assert!(!mgr.get_entity(A).unwrap().abilities.is_on_cooldown(id));
        assert_eq!(pool(&mgr, B, HEALTH), START_HEALTH);
    }
    // The Heal-typed attack still hits a hostile through the damage pipeline.
    assert!(handle_use_ability(A, HEAL_TYPED_ATTACK, MOB as i32, &tx, &mut mgr).await);
    assert!(
        pool(&mgr, MOB, HEALTH) < START_HEALTH,
        "the mob takes the hit"
    );
}

/// **Regression guard (warmup re-check).** 597's seed warmup is 2 s. Aimed
/// at a mob, it launches resolved to the caster and lands on the caster when
/// the warmup expires. Reverting the tick's resolver use makes the fire-time
/// #444 re-check refuse the caster as a target (`fired` fails).
#[tokio::test]
async fn a_warmed_up_heal_lands_on_the_caster_at_fire() {
    let mut mgr = heal_mgr(1.5);
    let (tx, mut rx) = mpsc::channel(256);
    let logs = LogCapture::install();

    assert!(handle_use_ability(A, HEAL_FOCUS, MOB as i32, &tx, &mut mgr).await);
    assert_eq!(pool(&mgr, A, FOCUS), 0, "nothing lands at launch");

    let fired = resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await;
    let _ = drain(&mut rx);

    assert_eq!(fired, 1, "the warmed-up heal fires");
    assert_eq!(pool(&mgr, A, FOCUS), 350);
    assert_eq!(pool(&mgr, MOB, FOCUS), 0, "MOB's Focus must not change");
    // The fire re-resolves from what the client sent (the mob), not from the
    // launch's resolved caster, so the row keeps the mob and the reason.
    let fire = logs
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "beneficial_cast") && c.has_field("stage", "fire"))
        .expect("a fire-stage beneficial_cast row");
    assert!(fire.has_field("wire_target_id", "4"), "{fire:?}");
    assert!(fire.has_field("resolution", "self_ability"), "{fire:?}");
}

/// **Regression guard (Copilot 4175813065).** A warmed-up Target heal at a
/// mob logs `fallback_to_caster` at fire with the mob as the wire target. If
/// the fire re-resolves from the launch's resolved target (the caster), the
/// reason turns into `ally` and the wire target into the caster.
#[tokio::test]
async fn a_warmed_up_fallback_keeps_its_reason_and_wire_target() {
    let mut mgr = heal_mgr(0.0);
    mgr.ability_defs.get_mut(&HEALTH_HEAL).unwrap().warmup = 1.5;
    let (tx, _rx) = mpsc::channel(256);
    let logs = LogCapture::install();

    assert!(handle_use_ability(A, HEALTH_HEAL, MOB as i32, &tx, &mut mgr).await);
    assert_eq!(
        resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await,
        1
    );

    let fire = logs
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "beneficial_cast") && c.has_field("stage", "fire"))
        .expect("a fire-stage beneficial_cast row");
    assert!(
        fire.has_field("resolution", "fallback_to_caster"),
        "{fire:?}"
    );
    assert!(fire.has_field("wire_target_id", "4"), "{fire:?}");
    assert!(fire.has_field("resolved_target_id", "1"), "{fire:?}");
    assert_eq!(pool(&mgr, A, HEALTH), START_HEALTH + 100);
    assert_eq!(
        pool(&mgr, MOB, HEALTH),
        START_HEALTH,
        "MOB's Health must not change"
    );
}

/// **Regression guard (Copilot 4175813098).** An ally who dies during the
/// warmup takes the cast to the caster, and `Ability_End` names the caster,
/// the entity the heal actually lands on. If the sequence goes out with the
/// launch-time target, it names the dead ally (`Ability_End target` fails).
#[tokio::test]
async fn ability_end_names_the_caster_when_the_warmed_up_ally_dies() {
    let mut mgr = heal_mgr(0.0);
    {
        let def = mgr.ability_defs.get_mut(&HEALTH_HEAL).unwrap();
        def.warmup = 1.5;
        def.event_set_id = Some(EVENT_SET);
    }
    let (tx, mut rx) = mpsc::channel(256);

    assert!(handle_use_ability(A, HEALTH_HEAL, B as i32, &tx, &mut mgr).await);
    mgr.get_entity_mut(B).unwrap().state_field |= cimmeria_wire::state_field::BSF_DEAD;
    let _ = drain(&mut rx);

    assert_eq!(
        resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await,
        1
    );
    let msgs = drain(&mut rx);

    let end_targets: Vec<i32> = calls(&msgs)
        .into_iter()
        .filter(|(e, m, a)| {
            *e == A
                && *m == method_idx::ON_SEQUENCE
                && i32::from_le_bytes(a[0..4].try_into().unwrap()) == SEQ_END
        })
        .map(|(_, _, a)| i32::from_le_bytes(a[8..12].try_into().unwrap()))
        .collect();
    assert!(!end_targets.is_empty(), "Ability_End was sent");
    assert!(
        end_targets.iter().all(|&t| t == A as i32),
        "Ability_End target must be the caster: {end_targets:?}"
    );
    assert_eq!(
        pool(&mgr, A, HEALTH),
        START_HEALTH + 100,
        "the caster is healed"
    );
    assert_eq!(pool(&mgr, B, HEALTH), START_HEALTH, "the dead ally is not");
}

/// 1218 on an ally: the first pulse lands at once and the other 24 are
/// registered on the ally with the caster as invoker.
#[tokio::test]
async fn recuperation_pulses_on_the_ally_with_the_caster_as_invoker() {
    let mut mgr = heal_mgr(0.0);
    let (tx, _rx) = mpsc::channel(256);

    assert!(handle_use_ability(A, RECUPERATION, B as i32, &tx, &mut mgr).await);

    assert_eq!(
        pool(&mgr, B, HEALTH),
        START_HEALTH + 30,
        "first pulse: 3% of 1000"
    );
    let active = &mgr.get_entity(B).unwrap().active_effects;
    assert_eq!(active.len(), 1, "the HoT is registered on B");
    assert_eq!(active[0].effect_id, 1383);
    assert_eq!(active[0].invoker_id, A);
    assert_eq!(active[0].remaining_pulses, 24);
    assert!(mgr.get_entity(A).unwrap().active_effects.is_empty());
}

/// **Wire format.** The caster's own client gets exactly one stat in its
/// `onStatUpdate`: Focus, min 0, cur 350, max 1000.
#[tokio::test]
async fn the_caster_receives_its_focus_stat_update_byte_exact() {
    let mut mgr = heal_mgr(0.0);
    let (tx, mut rx) = mpsc::channel(256);

    assert!(handle_use_ability(A, HEAL_FOCUS, 0, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);

    let mut want = 1u32.to_le_bytes().to_vec();
    for v in [FOCUS, 0, 350, MAX] {
        want.extend_from_slice(&v.to_le_bytes());
    }
    let got: Vec<_> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: A,
                method_index,
                args,
            } if *method_index == method_idx::ON_STAT_UPDATE => Some(args.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(got, vec![want], "one onStatUpdate to A: Focus 0/350/1000");
}

/// **Negative log.** A self heal with the caster selected logs no #444 WARN,
/// and logs `beneficial_cast` at launch and fire. On revert the #444 WARN
/// fires (`no #444 WARN` fails).
#[tokio::test]
async fn a_self_heal_logs_beneficial_cast_and_no_444_warn() {
    let mut mgr = heal_mgr(0.0);
    let (tx, _rx) = mpsc::channel(256);
    let logs = LogCapture::install();

    let committed = handle_use_ability(A, HEAL_FOCUS, A as i32, &tx, &mut mgr).await;

    assert!(
        logs.find_message(tracing::Level::WARN, "#444").is_none(),
        "no #444 WARN for a beneficial self cast: {:#?}",
        logs.all()
    );
    assert!(committed);
    let rows: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|c| c.target == "abilities" && c.has_field("event", "beneficial_cast"))
        .collect();
    assert_eq!(rows.len(), 2, "one row at launch, one at fire: {rows:#?}");
    // Launch is DEBUG (the fire's INFO row is the one per committed cast);
    // fire is INFO. Both carry the cast's id (`beneficial_cast_rows`).
    for (row, (stage, level)) in rows.iter().zip([
        ("launch", tracing::Level::DEBUG),
        ("fire", tracing::Level::INFO),
    ]) {
        assert_eq!(row.level, level, "{row:?}");
        assert!(row.has_field("stage", stage), "{row:?}");
        assert!(row.has_field("target_player_id", "101"), "{row:?}");
        assert!(row.has_field("resolution", "self_ability"), "{row:?}");
        assert!(row.has_field("ability_id", "597"), "{row:?}");
        assert!(row.has_field("wire_target_id", "1"), "{row:?}");
        assert!(row.has_field("resolved_target_id", "1"), "{row:?}");
    }
}

/// **Regression guard.** An `EF_ResolveOnAbilityUser` effect of a beneficial
/// cast lands on the caster while the rest lands on the ally. If the flag is
/// ignored, B's Focus rises instead (`B's Focus` fails).
#[tokio::test]
async fn a_user_flagged_effect_lands_on_the_caster_while_the_heal_lands_on_the_ally() {
    const SPLIT: i32 = 9001;
    let mut mgr = heal_mgr(0.0);
    let mut user_half = effect(9002, SPLIT, "HealFocus", "35", 1);
    user_half.flags = cimmeria_entity::abilities::EF_RESOLVE_ON_ABILITY_USER;
    mgr.effect_defs.insert(9002, user_half);
    let mut def = heal(SPLIT, TARGET_TARGET, 2008, 0.0);
    def.effect_ids.push(9002);
    mgr.ability_defs.insert(SPLIT, def);
    mgr.get_entity_mut(A).unwrap().abilities.add_ability(SPLIT);
    let (tx, mut rx) = mpsc::channel(256);

    assert!(handle_use_ability(A, SPLIT, B as i32, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);

    assert_eq!(pool(&mgr, B, FOCUS), 0, "B's Focus must not change");
    assert_eq!(
        pool(&mgr, A, FOCUS),
        350,
        "the user half restores A's Focus"
    );
    assert_eq!(
        pool(&mgr, B, HEALTH),
        START_HEALTH + 100,
        "the heal lands on B"
    );
    assert_eq!(pool(&mgr, A, HEALTH), START_HEALTH);
    for who in [A, B] {
        assert!(
            calls(&msgs)
                .iter()
                .any(|(e, m, _)| *e == who && *m == method_idx::ON_STAT_UPDATE),
            "{who} hears its own stat change"
        );
    }
}

/// The flag constants and the `type_id` labels AB-01 reads are pinned to the
/// client's `enumerations.xml`, not to a copy of themselves.
#[test]
fn beneficial_constants_match_the_client_enums() {
    use crate::cell::abilities::enumerations_xml::{token, tokens};
    assert_eq!(
        i64::from(cimmeria_entity::abilities::EF_BENEFICIAL_EFFECT),
        token("EEffectFlag", "EF_Beneficial_Effect")
    );
    assert_eq!(
        i64::from(cimmeria_entity::abilities::EF_RESOLVE_ON_ABILITY_USER),
        token("EEffectFlag", "EF_ResolveOnAbilityUser")
    );
    let labels = tokens("EAbilityTypes");
    assert_eq!(labels.len(), 6, "{labels:?}");
    for (_, name) in &labels {
        assert!(
            AbilityType::from_db_label(name).is_some(),
            "{name} has no AbilityType"
        );
    }
    assert_eq!(
        AbilityType::from_db_label("ABILITY_TYPE_Heal"),
        Some(AbilityType::Heal)
    );
}
