//! NA43 on the AT-10 phases: a charged cast plays Ability_Begin at launch,
//! Ability_End in the warmup tick that fires it, and Ability_Interrupt when
//! the warmup is cancelled. Every phase reaches the caster and its
//! witnesses, and the NPC WARN rides the Ability_End wherever it fires.
//! Fixture and helpers are in `sequence.rs`.

use std::time::{Duration, Instant};

use tracing::Level;

use super::sequence::{first_u32, scene, sequence_warns, NPC, ONLOOKER, SHOOTER};
use super::*;
use crate::cell::abilities::resolve_warmups;
use crate::cell::abilities::use_ability::warmup::{interrupt_pending_cast, InterruptReason};
use crate::cell::spawner::{EVENT_ABILITY_BEGIN, EVENT_ABILITY_END, EVENT_ABILITY_INTERRUPT};
use crate::test_support::{LogCapture, NoContentEvents};

/// A charged ability for the phase tests: 1.5 s warmup, event set 16 with
/// all three phase sequences.
const CHARGED: i32 = 560;
const CHARGED_EVENT_SET: i32 = 16;
const BEGIN_SEQ: i32 = 1600;
const CHARGED_END_SEQ: i32 = 1601;
const INTERRUPT_SEQ: i32 = 1602;
const CHARGED_WARMUP: f32 = 1.5;

fn add_charged(mgr: &mut SpaceManager, caster: u32, event_set_id: Option<i32>) {
    mgr.get_entity_mut(caster)
        .unwrap()
        .abilities
        .add_ability(CHARGED);
    let mut def = make_ability(CHARGED, 0, 40);
    def.warmup = CHARGED_WARMUP;
    def.event_set_id = event_set_id;
    mgr.ability_defs.insert(CHARGED, def);
    mgr.sequence_map
        .insert((CHARGED_EVENT_SET, EVENT_ABILITY_BEGIN), BEGIN_SEQ);
    mgr.sequence_map
        .insert((CHARGED_EVENT_SET, EVENT_ABILITY_END), CHARGED_END_SEQ);
    mgr.sequence_map
        .insert((CHARGED_EVENT_SET, EVENT_ABILITY_INTERRUPT), INTERRUPT_SEQ);
}

fn after_charge() -> Instant {
    Instant::now() + Duration::from_secs_f32(CHARGED_WARMUP) + Duration::from_millis(50)
}

/// The `onSequence` ids player `witness` received for `source`.
fn witnessed(msgs: &[CellToBaseMsg], witness: u32, source: u32) -> Vec<i32> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index: method_idx::ON_SEQUENCE,
                args,
                ..
            } if *witness_id == witness && *entity_id == source => Some(first_u32(args)),
            _ => None,
        })
        .collect()
}

/// The `onSequence` ids `entity`'s own client received.
fn own(msgs: &[CellToBaseMsg], entity: u32) -> Vec<i32> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: method_idx::ON_SEQUENCE,
                args,
            } if *entity_id == entity => Some(first_u32(args)),
            _ => None,
        })
        .collect()
}

/// The AT-10 split, seen from a second player. Player 1 launches a charged
/// ability: player 3 sees the Ability_Begin at launch and the Ability_End
/// in the warmup tick that fires it, each once, and the shooter's own client
/// gets the same two. Reverting either the launch site
/// (`warmup::begin_warmup`) or the fire site (`fire::fire_cast`) to
/// `send_entity_method` leaves player 3 without that phase.
#[tokio::test]
async fn a_players_charge_and_shot_are_both_seen_by_a_second_player() {
    let mut mgr = scene();
    add_charged(&mut mgr, SHOOTER, Some(CHARGED_EVENT_SET));
    let (tx, mut rx) = mpsc::channel(256);

    assert!(handle_use_ability(SHOOTER, CHARGED, NPC as i32, &tx, &mut mgr).await);
    let launch = drain(&mut rx);
    assert_eq!(own(&launch, SHOOTER), vec![BEGIN_SEQ]);
    assert_eq!(
        witnessed(&launch, ONLOOKER, SHOOTER),
        vec![BEGIN_SEQ],
        "the onlooker sees the charge start, and only the charge: {launch:#?}"
    );

    assert_eq!(
        resolve_warmups(after_charge(), &tx, &mut mgr, &NoContentEvents).await,
        1,
        "control: the warmup fires"
    );
    let fire = drain(&mut rx);
    assert_eq!(own(&fire, SHOOTER), vec![CHARGED_END_SEQ]);
    assert_eq!(
        witnessed(&fire, ONLOOKER, SHOOTER),
        vec![CHARGED_END_SEQ],
        "the onlooker sees the shot when the warmup fires it: {fire:#?}"
    );
}

/// A player's cancelled charge is seen by a second player too. Moving off
/// the spot interrupts the warmup; player 3 gets the Ability_Interrupt, so
/// the charge animation they were shown stops. Reverting
/// `warmup::interrupt_pending_cast` to `send_entity_method` fails this.
#[tokio::test]
async fn a_players_interrupted_charge_is_seen_by_a_second_player() {
    let mut mgr = scene();
    add_charged(&mut mgr, SHOOTER, Some(CHARGED_EVENT_SET));
    let (tx, mut rx) = mpsc::channel(256);
    assert!(handle_use_ability(SHOOTER, CHARGED, NPC as i32, &tx, &mut mgr).await);
    drain(&mut rx);

    mgr.get_entity_mut(SHOOTER).unwrap().position.x += 1.0;
    resolve_warmups(Instant::now(), &tx, &mut mgr, &NoContentEvents).await;
    let msgs = drain(&mut rx);

    assert!(
        mgr.get_entity(SHOOTER).unwrap().pending_cast.is_none(),
        "control: the move interrupted the warmup"
    );
    assert_eq!(own(&msgs, SHOOTER), vec![INTERRUPT_SEQ]);
    assert_eq!(
        witnessed(&msgs, ONLOOKER, SHOOTER),
        vec![INTERRUPT_SEQ],
        "the onlooker must see the charge cancelled: {msgs:#?}"
    );
}

/// An NPC's charged attack with a broken seed row WARNs when the shot
/// fires, not when the charge starts: Ability_End, whose absence is the
/// defect, is only looked up in the warmup tick (AT-10). The launch writes
/// nothing, so a charge the NPC never finishes is not reported as an
/// unanimated hit.
#[tokio::test]
async fn an_npc_charged_attack_warns_at_fire_not_at_launch() {
    let mut mgr = scene();
    add_charged(&mut mgr, NPC, Some(CHARGED_EVENT_SET));
    mgr.sequence_map
        .remove(&(CHARGED_EVENT_SET, EVENT_ABILITY_END));
    let logs = LogCapture::install();
    let (tx, mut rx) = mpsc::channel(256);

    assert!(handle_use_ability(NPC, CHARGED, SHOOTER as i32, &tx, &mut mgr).await);
    drain(&mut rx);
    assert!(
        sequence_warns(&logs.all(), "no_end_sequence").is_empty(),
        "nothing is known to be unanimated until the cast fires: {:#?}",
        logs.all()
    );

    assert_eq!(
        resolve_warmups(after_charge(), &tx, &mut mgr, &NoContentEvents).await,
        1,
        "control: the NPC's warmup fires"
    );
    let warns = sequence_warns(&logs.all(), "no_end_sequence");
    assert_eq!(warns.len(), 1, "{:#?}", logs.all());
    assert!(warns[0].has_field("ability_id", "560"), "{:?}", warns[0]);
    assert!(warns[0].has_field("event_set_id", "16"), "{:?}", warns[0]);
}

/// Interrupt is deliberately not a WARN seam: an interrupted NPC cast deals
/// no damage, so a missing Ability_Interrupt is not a hit from an invisible
/// attacker. An NPC whose event set has no Interrupt sequence, interrupted
/// by its own death, writes no `abilities.sequence` WARN.
#[tokio::test]
async fn an_npc_interrupt_with_no_interrupt_sequence_does_not_warn() {
    let mut mgr = scene();
    add_charged(&mut mgr, NPC, Some(CHARGED_EVENT_SET));
    mgr.sequence_map
        .remove(&(CHARGED_EVENT_SET, EVENT_ABILITY_INTERRUPT));
    let (tx, mut rx) = mpsc::channel(256);
    assert!(handle_use_ability(NPC, CHARGED, SHOOTER as i32, &tx, &mut mgr).await);
    drain(&mut rx);
    let logs = LogCapture::install();

    assert!(
        interrupt_pending_cast(NPC, InterruptReason::CasterDied, &tx, &mut mgr).await,
        "control: the NPC had a cast warming up"
    );
    assert!(
        logs.all()
            .iter()
            .all(|c| !(c.level == Level::WARN && c.target == "abilities.sequence")),
        "{:#?}",
        logs.all()
    );
}
