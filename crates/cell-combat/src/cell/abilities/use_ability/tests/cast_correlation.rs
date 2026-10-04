//! AB-T1: one cast, one `cast_id`, on every row it causes.
//!
//! The launch mints `cast_id` (the cast's `effect_seq`) and logs it on the
//! `ability_launched` row and the `combat.use_ability` span. These guards
//! pin that the rows emitted later, outside the launch (the warmup fire, the
//! timed-effect ledger's apply and expiry, a DoT's pulses), carry the same
//! id. Reverting the cast scope or the stamp on `TimedEffect` /
//! `ActiveEffectInstance` leaves those rows without `cast_id` and fails the
//! equality asserts.
//!
//! Each test burns a few effect ids first, so the cast's id is not the
//! counter's initial 1 and a row that logged some other sequence would not
//! match by accident.

use std::time::{Duration, Instant};

use cimmeria_entity::abilities::{EffectDef, EF_DONT_USE_QR};

use super::duel_gate::{duel_mgr, A, A_PID, B, B_PID, MOB};
use super::timed_buffs::{buff_mgr, AIM, AIM_EFFECT};
use super::*;
use crate::cell::abilities::resolve_warmups;
use crate::cell::effects::{effect_pulse_tick, stat_buff_tick_at};
use crate::test_support::{Captured, LogCapture, NoContentEvents};

/// Advance `eid`'s effect-id counter so its next cast is `skip + 1`.
fn burn_effect_ids(mgr: &mut SpaceManager, eid: u32, skip: usize) {
    let e = mgr.get_entity_mut(eid).unwrap();
    for _ in 0..skip {
        e.abilities.next_effect_id();
    }
}

/// The first captured row with `event = <event>`.
fn row(all: &[Captured], event: &str) -> Captured {
    all.iter()
        .find(|c| c.has_field("event", event))
        .cloned()
        .unwrap_or_else(|| panic!("no `{event}` row in {all:#?}"))
}

/// The `cast_id` the launch row logged.
fn launch_cast_id(all: &[Captured]) -> String {
    let launch = row(all, "ability_launched");
    assert_eq!(launch.target, "abilities");
    assert!(
        launch.has_field("player_id", &A_PID.to_string()),
        "rule 5: the launch row names the player: {launch:?}"
    );
    launch
        .fields
        .get("cast_id")
        .cloned()
        .expect("the launch row carries cast_id")
}

fn assert_cast(c: &Captured, cast_id: &str, what: &str) {
    assert_eq!(
        c.fields.get("cast_id").map(String::as_str),
        Some(cast_id),
        "{what} must carry the launch row's cast_id: {c:?}"
    );
}

/// **Regression guard (AB-T1).** A zero-warmup buff: the launch row, the
/// `combat.use_ability` span and the ledger's `stat_buff_applied` row share
/// one `cast_id`, and the expiry row logged later by the stat-buff tick,
/// outside any cast, still carries it. Without the cast scope the applied
/// row has no `cast_id`; without the stamp on the entry the expiry row has
/// none.
#[tokio::test]
async fn a_zero_warmup_casts_rows_share_one_cast_id() {
    let mut mgr = buff_mgr();
    burn_effect_ids(&mut mgr, A, 4);
    let (tx, _rx) = mpsc::channel(256);
    let logs = LogCapture::install();

    assert!(handle_use_ability(A, AIM, MOB as i32, &tx, &mut mgr).await);
    stat_buff_tick_at(Instant::now() + Duration::from_secs(60), &tx, &mut mgr).await;

    let all = logs.all();
    let cast_id = launch_cast_id(&all);
    assert_eq!(cast_id, "5", "the cast id is the minted effect_seq");
    assert!(
        logs.span_recorded("cast_id", &cast_id),
        "the combat.use_ability span records cast_id"
    );
    let applied = row(&all, "stat_buff_applied");
    assert!(applied.has_field("effect_id", &AIM_EFFECT.to_string()));
    assert_cast(&applied, &cast_id, "the ledger apply row");
    let expired = all
        .iter()
        .find(|c| c.has_field("event", "stat_buff_removed") && c.has_field("reason", "expired"))
        .expect("the buff expires");
    assert_cast(expired, &cast_id, "the ledger expiry row");
    assert_eq!(
        mgr.current_cast_id(),
        None,
        "the scope closes when the cast does"
    );
}

/// **Regression guard (AB-T1).** A warmup buff fires from the warmup tick,
/// long after the launch returned: the `warmup_started`, `warmup_complete`
/// and ledger rows and the `combat.cast_fire` span carry the launch row's
/// `cast_id`.
#[tokio::test]
async fn a_warmup_casts_fire_and_ledger_rows_carry_the_launch_cast_id() {
    let mut mgr = buff_mgr();
    mgr.ability_defs.get_mut(&AIM).unwrap().warmup = 1.5;
    burn_effect_ids(&mut mgr, A, 6);
    let (tx, _rx) = mpsc::channel(256);
    let logs = LogCapture::install();

    assert!(handle_use_ability(A, AIM, 0, &tx, &mut mgr).await);
    assert!(
        logs.all()
            .iter()
            .all(|c| !c.has_field("event", "stat_buff_applied")),
        "nothing lands before the warmup ends"
    );
    let fired = resolve_warmups(
        Instant::now() + Duration::from_secs(3),
        &tx,
        &mut mgr,
        &NoContentEvents,
    )
    .await;
    assert_eq!(fired, 1);

    let all = logs.all();
    let cast_id = launch_cast_id(&all);
    assert_eq!(cast_id, "7");
    assert_cast(&row(&all, "warmup_started"), &cast_id, "warmup_started");
    let complete = row(&all, "warmup_complete");
    assert_cast(&complete, &cast_id, "warmup_complete");
    assert!(complete.has_field("player_id", &A_PID.to_string()));
    let span = all
        .iter()
        .find(|c| c.target == "span:combat.cast_fire")
        .expect("the warmup fire opens combat.cast_fire");
    assert_cast(span, &cast_id, "the combat.cast_fire span");
    assert_cast(
        &row(&all, "stat_buff_applied"),
        &cast_id,
        "the ledger row of a warmed-up cast",
    );
}

const DOT: i32 = 70;
const DOT_EFFECT: i32 = 7070;

/// `duel_mgr` plus a no-roll DoT ability `DOT` of `pulses` 1 s pulses that A
/// knows.
fn dot_mgr(pulses: i32) -> SpaceManager {
    let mut mgr = duel_mgr();
    mgr.ability_defs.insert(
        DOT,
        AbilityDef {
            effect_ids: vec![DOT_EFFECT],
            ..super::warmup::cast_ability(DOT, 0.0)
        },
    );
    mgr.effect_defs.insert(
        DOT_EFFECT,
        EffectDef {
            effect_id: DOT_EFFECT,
            ability_id: DOT,
            // No roll: the cast never misses, so the DoT always registers.
            flags: EF_DONT_USE_QR,
            pulse_count: pulses,
            pulse_duration: 1.0,
            params: [("HealthDamage".to_string(), "5".to_string())].into(),
            ..Default::default()
        },
    );
    mgr.get_entity_mut(A).unwrap().abilities.add_ability(DOT);
    mgr
}

/// **Regression guard (AB-T1 review, rule 5).** A pulse outlives its
/// invoker's session and entity ids are recycled, so the pulse and end rows
/// must name the player who cast the DoT, from the snapshot taken at
/// registration, not whoever holds the invoker's entity id when the pulse
/// fires. Here A's entity id is taken over by another player between the
/// cast and the tick; resolving identity at log time would attribute the
/// pulse to player 555.
#[tokio::test]
async fn a_dots_pulse_rows_name_the_caster_after_its_entity_id_is_reused() {
    let mut mgr = dot_mgr(2);
    mgr.get_entity_mut(A).unwrap().account_id = Some(901);
    let (tx, _rx) = mpsc::channel(256);
    let logs = LogCapture::install();

    assert!(handle_use_ability(A, DOT, MOB as i32, &tx, &mut mgr).await);
    // Entity id A now belongs to another player's session.
    let reused = mgr.get_entity_mut(A).unwrap();
    reused.player_id = Some(555);
    reused.account_id = Some(9_555);
    for i in &mut mgr.get_entity_mut(MOB).unwrap().active_effects {
        i.next_pulse_at = Instant::now() - Duration::from_secs(1);
    }
    effect_pulse_tick(&NoContentEvents, &tx, &mut mgr).await;

    let all = logs.all();
    for event in ["pulse_ticked", "pulse_ended"] {
        let r = row(&all, event);
        assert!(
            r.has_field("player_id", &A_PID.to_string()) && r.has_field("account_id", "901"),
            "{event} must name the caster (player {A_PID}, account 901), not the entity id's \
             new owner: {r:?}"
        );
    }
}

/// **Regression guard (AB-T1).** A DoT's registration and every pulse the
/// pulse tick fires later carry the launch row's `cast_id`, on the row and
/// on the per-pulse `combat.effect_tick` span.
#[tokio::test]
async fn a_dots_tick_rows_carry_the_launch_cast_id() {
    let mut mgr = dot_mgr(3);
    burn_effect_ids(&mut mgr, A, 2);
    let (tx, _rx) = mpsc::channel(256);
    let logs = LogCapture::install();

    assert!(handle_use_ability(A, DOT, MOB as i32, &tx, &mut mgr).await);
    let inst = mgr.get_entity_mut(MOB).unwrap().active_effects[0].clone();
    assert_eq!(inst.cast_id, Some(3), "the instance holds its cast");
    for i in &mut mgr.get_entity_mut(MOB).unwrap().active_effects {
        i.next_pulse_at = Instant::now() - Duration::from_secs(1);
    }
    effect_pulse_tick(&NoContentEvents, &tx, &mut mgr).await;

    let all = logs.all();
    let cast_id = launch_cast_id(&all);
    assert_eq!(cast_id, "3");
    assert_cast(
        &row(&all, "active_effect_registered"),
        &cast_id,
        "the DoT registration",
    );
    let pulse = row(&all, "pulse_ticked");
    assert_cast(&pulse, &cast_id, "the pulse row");
    assert!(pulse.has_field("player_id", &A_PID.to_string()));
    let span = all
        .iter()
        .find(|c| c.target == "span:combat.effect_tick")
        .expect("the pulse opens combat.effect_tick");
    assert_cast(span, &cast_id, "the combat.effect_tick span");
    assert!(span.has_field("effect_id", &DOT_EFFECT.to_string()));
}

/// **Regression guard (AB-T3).** A DoT pulse logs `abilities.pulse`
/// `pulse_ticked` with its amount, its path and the target's pools before
/// and after, and the natural end logs `pulse_ended`. The pools match the
/// target: the pulse row is written after the pulse landed, from the same
/// stats the client is sent.
#[tokio::test]
async fn a_dot_tick_logs_the_pools_before_and_after() {
    let mut mgr = dot_mgr(2);
    let (tx, _rx) = mpsc::channel(256);
    assert!(handle_use_ability(A, DOT, MOB as i32, &tx, &mut mgr).await);
    let health = |mgr: &SpaceManager| {
        mgr.get_entity(MOB)
            .unwrap()
            .stats
            .get(cimmeria_entity::stats::HEALTH)
            .unwrap()
            .cur
    };
    let before = health(&mgr);
    for i in &mut mgr.get_entity_mut(MOB).unwrap().active_effects {
        i.next_pulse_at = Instant::now() - Duration::from_secs(1);
    }
    let logs = LogCapture::install();

    effect_pulse_tick(&NoContentEvents, &tx, &mut mgr).await;

    let after = health(&mgr);
    assert!(after < before, "the pulse dealt damage");
    let all = logs.all();
    let pulse = row(&all, "pulse_ticked");
    assert_eq!(pulse.target, "abilities.pulse");
    for (field, want) in [
        ("health_before", before.to_string()),
        ("health_after", after.to_string()),
        ("health_amount", "5".to_string()),
        ("path", "nvp".to_string()),
        ("effect_id", DOT_EFFECT.to_string()),
        ("player_id", A_PID.to_string()),
    ] {
        assert!(
            pulse.has_field(field, &want),
            "pulse_ticked {field} = {want}: {pulse:?}"
        );
    }
    let ended = row(&all, "pulse_ended");
    assert_eq!(ended.target, "abilities.pulse");
    assert!(ended.has_field("reason", "natural_end"));
    assert!(ended.has_field("player_id", &A_PID.to_string()));
}

/// **Regression guard (AB-T3 with #1170 god mode).** A DoT pulse on a
/// god-mode target: `pulse_ticked` reports the pools the target kept (read
/// after the restore), `god_mode = true`, and what was put back. Reading
/// the pools before the restore would claim a Health loss that never stuck.
#[tokio::test]
async fn a_god_mode_targets_pulse_row_reports_the_kept_pools() {
    let mut mgr = dot_mgr(2);
    let (tx, _rx) = mpsc::channel(256);
    assert!(handle_use_ability(A, DOT, MOB as i32, &tx, &mut mgr).await);
    let mob = mgr.get_entity_mut(MOB).unwrap();
    mob.god_mode = true;
    let before = mob.stats.get(cimmeria_entity::stats::HEALTH).unwrap().cur;
    for i in &mut mob.active_effects {
        i.next_pulse_at = Instant::now() - Duration::from_secs(1);
    }
    let logs = LogCapture::install();

    effect_pulse_tick(&NoContentEvents, &tx, &mut mgr).await;

    let pulse = row(&logs.all(), "pulse_ticked");
    assert!(pulse.has_field("god_mode", "true"), "{pulse:?}");
    assert!(
        pulse.has_field("health_before", &before.to_string())
            && pulse.has_field("health_after", &before.to_string()),
        "the row reports the kept Health: {pulse:?}"
    );
    let restored: i32 = pulse.fields["god_mode_restored_health"].parse().unwrap();
    assert!(restored > 0, "and what god mode put back: {pulse:?}");
}

/// Assert `c` carries the core fields (rule 5, AB-T3): the invoker as the
/// actor (`entity_id`, `player_id`), the ability and effect, the target,
/// `stage`, and `target_player_id` when the target is a player.
fn assert_pulse_core(c: &Captured, target: u32, target_pid: Option<i32>, stage: &str) {
    assert_eq!(c.target, "abilities.pulse", "{c:?}");
    for (field, want) in [
        ("entity_id", A.to_string()),
        ("player_id", A_PID.to_string()),
        ("ability_id", DOT.to_string()),
        ("effect_id", DOT_EFFECT.to_string()),
        ("target_id", target.to_string()),
        ("stage", stage.to_string()),
    ] {
        assert!(c.has_field(field, &want), "{field} = {want}: {c:?}");
    }
    if let Some(pid) = target_pid {
        assert!(c.has_field("target_player_id", &pid.to_string()), "{c:?}");
    }
}

/// `dot_mgr(2)` with A's DoT registered straight onto `target`, due now.
async fn dot_on(target: u32, health_damage: i32) -> SpaceManager {
    let mut mgr = dot_mgr(2);
    let effect = mgr.effect_defs.get_mut(&DOT_EFFECT).unwrap();
    effect
        .params
        .insert("HealthDamage".to_string(), health_damage.to_string());
    let effect = effect.clone();
    let (tx, _rx) = mpsc::channel(256);
    assert!(
        crate::cell::effects::register_active_effect(
            &mut mgr,
            target,
            A,
            &effect,
            Instant::now(),
            &tx
        )
        .await
    );
    for i in &mut mgr.get_entity_mut(target).unwrap().active_effects {
        i.next_pulse_at = Instant::now() - Duration::from_secs(1);
    }
    mgr
}

/// **Regression guard (AB-T3 review, rule 5).** Every `abilities.pulse` row
/// (`pulse_ticked`, `pulse_ended`, `pulse_skipped_dead_target`,
/// `pulse_surrender_floor`) names the invoker as the actor (`entity_id`),
/// the ability, the target and its `player_id`, and the `stage`. Dropping
/// any of them from a row fails here.
#[tokio::test]
async fn every_pulse_row_carries_the_core_fields() {
    use cimmeria_entity::cell_entity::AiState;
    use cimmeria_entity::stats::HEALTH;
    let (tx, _rx) = mpsc::channel(256);

    // A player target: a tick and the natural end.
    let mut mgr = dot_on(B, 5).await;
    let logs = LogCapture::install();
    effect_pulse_tick(&NoContentEvents, &tx, &mut mgr).await;
    let all = logs.all();
    assert_pulse_core(&row(&all, "pulse_ticked"), B, Some(B_PID), "pulse");
    assert_pulse_core(&row(&all, "pulse_ended"), B, Some(B_PID), "end");
    drop(logs);

    // A dead player target: the pulse is skipped.
    let mut mgr = dot_on(B, 5).await;
    let b = mgr.get_entity_mut(B).unwrap();
    b.stats.get_mut(HEALTH).unwrap().update(0, 0, 100);
    let logs = LogCapture::install();
    effect_pulse_tick(&NoContentEvents, &tx, &mut mgr).await;
    let all = logs.all();
    assert_pulse_core(
        &row(&all, "pulse_skipped_dead_target"),
        B,
        Some(B_PID),
        "pulse",
    );
    drop(logs);

    // A surrendered NPC: a lethal pulse is floored at 1.
    let mut mgr = dot_on(MOB, 1_000).await;
    let mob = mgr.get_entity_mut(MOB).unwrap();
    crate::cell::service::npc_ai::force_ai_state(mob, AiState::Submit);
    mob.stats.get_mut(HEALTH).unwrap().update(0, 5, 100);
    let logs = LogCapture::install();
    effect_pulse_tick(&NoContentEvents, &tx, &mut mgr).await;
    let all = logs.all();
    assert_pulse_core(&row(&all, "pulse_surrender_floor"), MOB, None, "pulse");
}
