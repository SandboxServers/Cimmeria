//! `.dummy caster <abilityId> [intervalSecs]`: a lab dummy that casts one
//! ability at its owner every interval, through the real launch, so the UAT
//! can interrupt an NPC warmup (AB-U20) and cleanse an NPC's debuff
//! (AB-U22).
//!
//! Bug shapes: a caster that never fires, fires off its interval, at someone
//! else, or through a path that skips the warmup (so nothing can interrupt
//! it); a caster that keeps shooting a dead owner; a plain dummy that starts
//! attacking; a refused placement that still spawns something.

use std::time::{Duration, Instant};

use cimmeria_cell_world::test_fixtures::{seed_mechanic_effect, MECHANIC_FIXTURE_EFFECT};
use cimmeria_entity::abilities::AbilityDef;
use tokio::sync::mpsc;
use tracing::Level;

use super::dummy::dummy_world;
use super::{console, lines, EntityCount, CALLER};
use crate::cell::client_methods::spawnable_entity::ON_SEQUENCE;
use crate::cell::combat::BSF_DEAD;
use crate::cell::console::abilities::dummy_caster::cast_due;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{LabCaster, SpaceManager, LAB_DUMMY_MAX_PER_OWNER};
use crate::cell::spawner::{EVENT_ABILITY_BEGIN, EVENT_ABILITY_END, EVENT_ABILITY_INTERRUPT};
use crate::test_support::LogCapture;
use cimmeria_cell_world::cell::effects::interrupt_request::{InterruptCause, InterruptRequest};

/// An instant, cooldown-free attack: every launch is an `Ability_End`.
const SHOT: i32 = 9200;
/// A 4 s warmup attack, like Disabling Shot (1354), the AB-U20 pick.
const CHARGED: i32 = 9201;
const EVENT_SET: i32 = 950;
const SEQ_BEGIN: i32 = 9500;
const SEQ_END: i32 = 9501;
const SEQ_INTERRUPT: i32 = 9502;

fn attack(id: i32, warmup: f32, cooldown: f32) -> AbilityDef {
    AbilityDef {
        ability_id: id,
        name: format!("Lab Attack {id}"),
        cooldown,
        warmup,
        flags: 0,
        is_ranged: true,
        min_range: 0.0,
        max_range: 30.0,
        target_type_id: 0,
        effect_ids: vec![MECHANIC_FIXTURE_EFFECT],
        moniker_ids: vec![],
        required_ammo: 0,
        event_set_id: Some(EVENT_SET),
        velocity: 0.0,
        type_id: Default::default(),
        passive: false,
    }
}

fn caster_world() -> SpaceManager {
    let mut mgr = dummy_world();
    seed_mechanic_effect(&mut mgr);
    mgr.ability_defs.insert(SHOT, attack(SHOT, 0.0, 0.0));
    mgr.ability_defs.insert(CHARGED, attack(CHARGED, 4.0, 5.0));
    mgr.sequence_map
        .insert((EVENT_SET, EVENT_ABILITY_BEGIN), SEQ_BEGIN);
    mgr.sequence_map
        .insert((EVENT_SET, EVENT_ABILITY_END), SEQ_END);
    mgr.sequence_map
        .insert((EVENT_SET, EVENT_ABILITY_INTERRUPT), SEQ_INTERRUPT);
    mgr
}

/// Place a caster with `line`; returns its id and the instant taken just
/// before, which its first cast is one interval after.
async fn place(mgr: &mut SpaceManager, line: &str) -> (u32, Instant) {
    let t0 = Instant::now();
    let out = lines(&console(mgr, None, line).await);
    assert!(
        out.len() == 1 && out[0].contains("placed: caster"),
        "{out:?}"
    );
    let ids = mgr.lab_dummies_of(CALLER);
    let id = *ids.last().unwrap();
    let _ = mgr.compute_aoi_changes();
    (id, t0)
}

/// `(sequence, target)` of every `dummy` `onSequence` the owner's client got.
fn owner_sequences(msgs: &[CellToBaseMsg], dummy: u32) -> Vec<(i32, u32)> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::WitnessEntityMethod {
                witness_id: CALLER,
                entity_id,
                method_index: ON_SEQUENCE,
                args,
                ..
            } if *entity_id == dummy => Some((
                i32::from_le_bytes(args[0..4].try_into().unwrap()),
                u32::from_le_bytes(args[8..12].try_into().unwrap()),
            )),
            _ => None,
        })
        .collect()
}

/// Run the caster sweep at `at`; returns [`owner_sequences`].
async fn sweep(mgr: &mut SpaceManager, at: Instant, dummy: u32) -> Vec<(i32, u32)> {
    let (tx, mut rx) = mpsc::channel(512);
    cast_due(at, &tx, mgr).await;
    let msgs: Vec<CellToBaseMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    owner_sequences(&msgs, dummy)
}

fn expire_cooldown(mgr: &mut SpaceManager, dummy: u32, ability: i32) {
    let d = mgr.get_entity_mut(dummy).unwrap();
    assert!(
        d.abilities.clear_ability_cooldown(ability),
        "the cast charged one"
    );
}

fn secs(t0: Instant, s: f32) -> Instant {
    t0 + Duration::from_secs_f32(s)
}

/// Revert proof: drop the `handle_use_ability` call from `cast_due` (or the
/// `next_cast_at` advance) and the counts below are 0 (or 2 at 12 s).
#[tokio::test]
async fn ab_l2_dummy_caster_fires_on_its_interval_at_its_owner() {
    let mut mgr = caster_world();
    let logs = LogCapture::install();
    let (dummy, t0) = place(&mut mgr, &format!(".dummy caster {SHOT} 8")).await;
    let mark = *mgr
        .get_entity(dummy)
        .unwrap()
        .extensions
        .get::<LabCaster>()
        .expect("caster mark");
    assert_eq!((mark.ability_id, mark.interval.as_secs()), (SHOT, 8));

    assert!(
        sweep(&mut mgr, secs(t0, 7.0), dummy).await.is_empty(),
        "not yet"
    );
    assert_eq!(
        sweep(&mut mgr, secs(t0, 9.0), dummy).await,
        vec![(SEQ_END, CALLER)],
        "one shot at its owner"
    );
    // The dummy stands 3 m along +Z from its owner: it turned to face -Z.
    let yaw = mgr.get_entity(dummy).unwrap().direction.y;
    assert!((yaw.abs() - std::f32::consts::PI).abs() < 1e-3, "yaw {yaw}");
    assert!(
        sweep(&mut mgr, secs(t0, 16.0), dummy).await.is_empty(),
        "one interval after the last attempt, not before"
    );
    // Cooldowns run on the real clock (a zero cooldown is charged 0.5 s);
    // the sweep's clock is the test's. Expire it as 8 s would have.
    expire_cooldown(&mut mgr, dummy, SHOT);
    assert_eq!(
        sweep(&mut mgr, secs(t0, 17.5), dummy).await,
        vec![(SEQ_END, CALLER)]
    );
    let row = logs
        .find_message(Level::INFO, "caster lab dummy cast at its owner")
        .expect("lab_caster_cast row");
    assert!(row.has_field("launched", "true"), "{row:#?}");
    assert!(row.has_field("dummy_id", &dummy.to_string()));
    assert!(row.has_field("player_id", "71"));
}

/// AB-U20's shape: the caster's cast has a real warmup, so a stun (the AT-10
/// incapacitated interrupt) breaks it, refunds its cooldown and plays the
/// interrupt; while a cast warms up the next attempt holds, and after the
/// interrupt it casts again. Revert proof: a launch that skipped the warmup
/// leaves nothing to interrupt (`is_casting` fails).
#[tokio::test]
async fn ab_l2_dummy_caster_warmup_is_interrupted_by_a_stun() {
    let mut mgr = caster_world();
    let (dummy, t0) = place(&mut mgr, &format!(".dummy caster {CHARGED}")).await;
    assert_eq!(
        sweep(&mut mgr, secs(t0, 9.0), dummy).await,
        vec![(SEQ_BEGIN, CALLER)],
        "the warmup starts at the default 8 s"
    );
    let pending = mgr.get_entity(dummy).unwrap().pending_cast.clone();
    assert_eq!(
        pending.map(|p| (p.ability_id, p.target_id)),
        Some((CHARGED, CALLER as i32))
    );

    let logs = LogCapture::install();
    assert!(
        sweep(&mut mgr, secs(t0, 17.5), dummy).await.is_empty(),
        "held while its cast still warms up"
    );
    let held = logs
        .find_message(tracing::Level::DEBUG, "caster lab dummy held its cast")
        .expect("lab_caster_held row");
    assert!(held.has_field("reason", "still_casting"), "{held:#?}");

    mgr.request_interrupt(InterruptRequest {
        source_id: CALLER,
        target_id: dummy,
        effect_id: 1,
        ability_id: 1,
        chance_pct: 100,
        cause: InterruptCause::Incapacitated,
        nonce: 0,
        cast_id: None,
    });
    let (tx, mut rx) = mpsc::channel(256);
    cimmeria_cell_combat::cell::effects::interrupt::resolve_interrupts_for(dummy, &tx, &mut mgr)
        .await;
    let burst: Vec<CellToBaseMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    let d = mgr.get_entity(dummy).unwrap();
    assert!(d.pending_cast.is_none(), "the warmup was broken");
    assert!(
        !d.abilities.is_on_cooldown(CHARGED),
        "and its cooldown refunded (AT-10)"
    );
    assert_eq!(
        owner_sequences(&burst, dummy),
        vec![(SEQ_INTERRUPT, CALLER)],
        "Ability_Interrupt played to the owner"
    );
    let row = logs
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "warmup_interrupted"))
        .expect("warmup_interrupted row");
    assert!(row.has_field("reason", "incapacitated"), "{row:#?}");

    assert_eq!(
        sweep(&mut mgr, secs(t0, 26.0), dummy).await,
        vec![(SEQ_BEGIN, CALLER)],
        "it casts again on the next interval"
    );
}

/// A plain dummy beside a caster still never casts: the sweep drives only
/// the caster. (No AI turn for either is pinned in `cimmeria-cell`'s
/// `npc_ai::lab_dummy`.)
#[tokio::test]
async fn ab_l2_plain_dummy_never_attacks_beside_a_caster() {
    let mut mgr = caster_world();
    console(&mut mgr, None, ".dummy").await;
    let plain = mgr.lab_dummies_of(CALLER)[0];
    let yaw_before = mgr.get_entity(plain).unwrap().direction.y;
    let (caster, t0) = place(&mut mgr, &format!(".dummy caster {SHOT}")).await;
    assert_ne!(plain, caster);

    let (tx, mut rx) = mpsc::channel(512);
    cast_due(secs(t0, 9.0), &tx, &mut mgr).await;
    let msgs: Vec<CellToBaseMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    let senders: Vec<u32> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::WitnessEntityMethod {
                witness_id: CALLER,
                entity_id,
                method_index: ON_SEQUENCE,
                ..
            } => Some(*entity_id),
            _ => None,
        })
        .collect();
    assert_eq!(senders, vec![caster], "only the caster acts");
    let p = mgr.get_entity(plain).unwrap();
    assert!(p.extensions.get::<LabCaster>().is_none());
    assert!(p.pending_cast.is_none());
    assert_eq!(p.direction.y, yaw_before, "it never even turns");
}

/// Revert proof: drop the owner-dead hold and the dead owner is shot.
#[tokio::test]
async fn ab_l2_dummy_caster_holds_while_its_owner_is_dead() {
    let mut mgr = caster_world();
    let (dummy, t0) = place(&mut mgr, &format!(".dummy caster {SHOT}")).await;
    mgr.get_entity_mut(CALLER).unwrap().state_field |= BSF_DEAD;

    let logs = LogCapture::install();
    assert!(sweep(&mut mgr, secs(t0, 9.0), dummy).await.is_empty());
    let held = logs
        .find_message(tracing::Level::DEBUG, "caster lab dummy held its cast")
        .expect("lab_caster_held row");
    assert!(held.has_field("reason", "owner_dead"), "{held:#?}");

    // Revived: casting resumes on the schedule, one interval on.
    mgr.get_entity_mut(CALLER).unwrap().state_field &= !BSF_DEAD;
    assert!(
        sweep(&mut mgr, secs(t0, 16.0), dummy).await.is_empty(),
        "no burst on revival"
    );
    assert_eq!(
        sweep(&mut mgr, secs(t0, 17.5), dummy).await,
        vec![(SEQ_END, CALLER)]
    );
}

/// Every refusal answers with a line and spawns nothing.
#[tokio::test]
async fn ab_l2_dummy_caster_refusals_spawn_nothing() {
    let mut mgr = caster_world();
    let mut passive = attack(9202, 0.0, 0.0);
    passive.passive = true;
    mgr.ability_defs.insert(9202, passive);
    let mut short = attack(9203, 0.0, 0.0);
    short.max_range = 2.0;
    mgr.ability_defs.insert(9203, short);
    let before = mgr.entity_count();
    for (line, expect) in [
        (".dummy caster".to_string(), "usage"),
        (".dummy caster x".to_string(), "positive integer"),
        (".dummy caster 777777".to_string(), "no ability 777777"),
        (".dummy caster 9202".to_string(), "passive"),
        (".dummy caster 9203".to_string(), "reaches 2 m"),
        (format!(".dummy caster {SHOT} 0"), "from 1 to 600"),
        (
            format!(".dummy caster {CHARGED} 4"),
            "use an interval of at least 5 s",
        ),
    ] {
        let out = lines(&console(&mut mgr, None, &line).await);
        assert!(out.len() == 1 && out[0].contains(expect), "{line}: {out:?}");
        assert_eq!(mgr.entity_count(), before, "{line}: nothing spawned");
    }

    // The cap counts casters and plain dummies together.
    for _ in 0..LAB_DUMMY_MAX_PER_OWNER {
        console(&mut mgr, None, ".dummy").await;
    }
    let full = mgr.entity_count();
    let out = lines(&console(&mut mgr, None, &format!(".dummy caster {SHOT}")).await);
    assert!(out[0].contains("you already have 4 dummies"), "{out:?}");
    assert_eq!(mgr.entity_count(), full);
}
