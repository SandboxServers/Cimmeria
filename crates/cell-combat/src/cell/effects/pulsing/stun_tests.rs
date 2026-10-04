//! The stun lock through the pulse tick and the stat-buff flush (ability
//! mechanics AB-09a): the issue's repro shape, plus the state-field
//! broadcast it asked for.
//!
//! `make_mgr`'s fixture: player 1 at the origin and entity 2 five units
//! away, with the effect-script registry installed.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use cimmeria_entity::abilities::EffectDef;
use cimmeria_wire::state_field::BSF_MOVEMENT_LOCK;

use super::tests::make_mgr;
use super::{effect_pulse_tick, register_active_effect};
use crate::cell::effects::{flush_stat_buff_timers, stat_buff_tick_at, EffectContext};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx::ON_STATE_FIELD_UPDATE;
use crate::test_support::NoContentEvents;

/// A pulsing stun: three 1 s pulses on the target, from invoker 2.
fn pulsing_stun() -> EffectDef {
    EffectDef {
        effect_id: 7001,
        ability_id: 1355,
        script_name: Some("Stun".to_string()),
        pulse_count: 3,
        pulse_duration: 1.0,
        flags: 68,
        ..Default::default()
    }
}

fn setup() -> (SpaceManager, EffectDef) {
    let mut mgr = make_mgr();
    crate::test_support::install_effect_scripts(&mut mgr);
    let stun = pulsing_stun();
    mgr.effect_defs.insert(stun.effect_id, stun.clone());
    mgr.connect_entity(1);
    let _ = mgr.compute_aoi_changes();
    (mgr, stun)
}

/// The hit: `damage_apply` runs the script, then registers the instance.
async fn land(mgr: &mut SpaceManager, stun: &EffectDef, tx: &mpsc::Sender<CellToBaseMsg>) {
    let mut ctx = EffectContext {
        source_id: 2,
        target_id: 1,
        effect: stun,
        space_mgr: mgr,
    };
    assert!(crate::cell::effects::dispatch_by_name("Stun", &mut ctx));
    flush_stat_buff_timers(1, Instant::now(), tx, mgr).await;
    register_active_effect(mgr, 1, 2, stun, Instant::now(), tx).await;
}

async fn pulse_all_due(mgr: &mut SpaceManager, tx: &mpsc::Sender<CellToBaseMsg>) {
    let past = Instant::now() - Duration::from_secs(5);
    for inst in &mut mgr.get_entity_mut(1).unwrap().active_effects {
        inst.next_pulse_at = past;
    }
    effect_pulse_tick(&NoContentEvents, tx, mgr).await;
}

/// **Regression guard (the stun leak, the issue's repro).** A three-pulse
/// stun: the hit, two re-dispatching pulses and the sweep's single
/// `on_remove`. Afterwards `BSF_MovementLock` is clear and no reference is
/// left. The old script added a reference on every `on_apply`, ending at
/// a count of 2 with the bit set for good (`lock after expiry` fails).
#[tokio::test]
async fn a_pulsing_stun_clears_the_lock_when_it_ends() {
    let (mut mgr, stun) = setup();
    let (tx, _rx) = mpsc::channel(512);
    land(&mut mgr, &stun, &tx).await;
    assert!(mgr.get_entity(1).unwrap().has_state_flag(BSF_MOVEMENT_LOCK));

    for _ in 0..3 {
        pulse_all_due(&mut mgr, &tx).await;
    }
    let target = mgr.get_entity(1).unwrap();
    assert!(target.active_effects.is_empty(), "the instance was swept");
    assert!(
        !target.has_state_flag(BSF_MOVEMENT_LOCK),
        "lock after expiry"
    );
    assert_eq!(target.state_flag_counts.get(&BSF_MOVEMENT_LOCK), None);
}

/// Observer player 3, three units from the stunned player 1, in AoI.
const OBSERVER: u32 = 3;

/// Every `onStateFieldUpdate` about player 1: `(recipient, args)`, the
/// recipient being 1 for its own client and the witness id otherwise.
fn state_field_sends(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<(u32, Vec<u8>)> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: 1,
                method_index: ON_STATE_FIELD_UPDATE,
                args,
            } => out.push((1, args)),
            CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id: 1,
                method_index: ON_STATE_FIELD_UPDATE,
                args,
                entity_is_player,
            } => {
                assert!(entity_is_player, "player 1 is routed as a player");
                out.push((witness_id, args));
            }
            _ => {}
        }
    }
    out.sort();
    out
}

/// **Regression guard (the state-field broadcast).** With an observer in
/// AoI: the landed stun sends exactly one `onStateFieldUpdate` to the
/// stunned player's client and one to the observer, both the 4-byte LE
/// `0x40` (`BSF_MovementLock`); a same-caster refresh sends neither; the
/// expiry sends exactly one `0` to each. Sending only to the entity's own
/// client (`send_entity_method`) fails the observer's rows; dropping the
/// flush fails all of them.
#[tokio::test]
async fn the_lock_is_broadcast_once_on_and_once_off() {
    let (mut mgr, mut stun) = setup();
    mgr.create_entity(OBSERVER, "W", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    {
        let o = mgr.get_entity_mut(OBSERVER).unwrap();
        o.is_player = true;
        o.player_id = Some(300);
    }
    mgr.connect_entity(OBSERVER);
    let _ = mgr.compute_aoi_changes();
    assert!(mgr.get_witnesses_of(1).contains(&OBSERVER), "fixture");
    // Single-pulse: a plain 5 s ledger entry the stat-buff tick expires.
    stun.pulse_count = 1;
    stun.pulse_duration = 5.0;
    mgr.effect_defs.insert(stun.effect_id, stun.clone());
    let (tx, mut rx) = mpsc::channel(512);
    let on = vec![0x40, 0x00, 0x00, 0x00];
    let off = vec![0x00; 4];

    land(&mut mgr, &stun, &tx).await;
    assert_eq!(
        state_field_sends(&mut rx),
        vec![(1, on.clone()), (OBSERVER, on)],
        "on: one to each client"
    );

    // A same-caster re-hit refreshes the entry: nothing to either client.
    land(&mut mgr, &stun, &tx).await;
    assert_eq!(state_field_sends(&mut rx), Vec::new(), "refresh");

    let expired = stat_buff_tick_at(Instant::now() + Duration::from_secs(6), &tx, &mut mgr).await;
    assert_eq!(expired, 1);
    assert_eq!(
        state_field_sends(&mut rx),
        vec![(1, off.clone()), (OBSERVER, off)],
        "off: one to each client"
    );
}
