//! Ability mechanics AB-09c: an interrupt effect breaks the target's warmup.
//!
//! `warmup`'s fixture: player 1 and hostile NPC 2, ability 50 with a 1.5 s
//! warmup and the Begin / End / Interrupt sequences. NPC 2 starts casting 50
//! at the player; the player answers with Interrupting Shot (657, effect 723
//! "Interrupts target", the `Interrupt` script). The script queues the
//! interrupt, and the hit's `flush_stat_buff_timers` resolves it: the NPC's
//! `Ability_Interrupt` reaches the player in the same burst, and the warmup
//! never fires.

use std::collections::HashMap;

use cimmeria_entity::abilities::{EffectDef, EF_DONT_USE_QR};
use cimmeria_entity::stats::{COORDINATION, INTERRUPT_RES};

use super::warmup::{
    after_warmup, cast_ability, effect_results, sequences, warmup_mgr, SEQ_BEGIN, SEQ_INTERRUPT,
    WARMUP_ABILITY,
};
use super::*;
use crate::cell::abilities::resolve_warmups;
use crate::test_support::{LogCapture, NoContentEvents};

const INTERRUPTING_SHOT: i32 = 657;
const INTERRUPT_EFFECT: i32 = 723;

/// The fixture plus Interrupting Shot on the player. Effect 723 is the
/// seeded row (`Interrupt`, flags 512) with `EF_DontUseQR` added so the
/// test's hit can never roll a miss; the miss gate is AB-06's, tested there.
fn interrupt_mgr(npc_interrupt_res: i32) -> SpaceManager {
    let mut mgr = warmup_mgr();
    crate::test_support::install_effect_scripts(&mut mgr);
    mgr.ability_defs.insert(
        INTERRUPTING_SHOT,
        AbilityDef {
            effect_ids: vec![INTERRUPT_EFFECT],
            event_set_id: None,
            ..cast_ability(INTERRUPTING_SHOT, 0.0)
        },
    );
    mgr.effect_defs.insert(
        INTERRUPT_EFFECT,
        EffectDef {
            effect_id: INTERRUPT_EFFECT,
            ability_id: INTERRUPTING_SHOT,
            script_name: Some("Interrupt".to_string()),
            flags: 512 | EF_DONT_USE_QR,
            pulse_count: 1,
            params: HashMap::new(),
            ..Default::default()
        },
    );
    mgr.get_entity_mut(1)
        .unwrap()
        .abilities
        .add_ability(INTERRUPTING_SHOT);
    let npc = mgr.get_entity_mut(2).unwrap();
    npc.abilities.add_ability(WARMUP_ABILITY);
    for (stat, value) in [(INTERRUPT_RES, npc_interrupt_res), (COORDINATION, 0)] {
        if let Some(s) = npc.stats.get_mut(stat) {
            s.update(0, value, value.max(0));
        }
    }
    mgr
}

/// NPC 2 starts its warmup on the player.
async fn npc_starts_casting(mgr: &mut SpaceManager, tx: &mpsc::Sender<CellToBaseMsg>) {
    assert!(handle_use_ability(2, WARMUP_ABILITY, 1, tx, mgr).await);
    assert!(crate::cell::abilities::is_casting(mgr, 2));
}

/// **Regression guard (interrupt had no mechanic).** Interrupting Shot on an
/// NPC in its warmup sends the NPC's `Ability_Interrupt` sequence in the
/// same burst, clears the cast and refunds its cooldown (AT-10), and the
/// warmup never fires. Before AB-09 effect 723 had no script: the NPC kept
/// casting (`the NPC's interrupt sequence` fails) and hit the player when
/// the warmup ran out.
#[tokio::test]
async fn interrupting_shot_breaks_an_npc_warmup() {
    let mut mgr = interrupt_mgr(0);
    let (tx, mut rx) = mpsc::channel(512);
    npc_starts_casting(&mut mgr, &tx).await;
    assert_eq!(sequences(&drain(&mut rx), 2), vec![SEQ_BEGIN]);

    assert!(handle_use_ability(1, INTERRUPTING_SHOT, 2, &tx, &mut mgr).await);
    let burst = drain(&mut rx);
    assert_eq!(
        sequences(&burst, 2),
        vec![SEQ_INTERRUPT],
        "the NPC's interrupt sequence; got {burst:?}"
    );
    assert!(!crate::cell::abilities::is_casting(&mgr, 2));
    assert!(
        !mgr.get_entity(2)
            .unwrap()
            .abilities
            .is_on_cooldown(WARMUP_ABILITY),
        "the interrupted cast's cooldown is refunded (AT-10)"
    );

    resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await;
    let later = drain(&mut rx);
    assert_eq!(effect_results(&later, 2), 0, "the warmup never fires");
}

/// Full interrupt resistance (1000 points = 100 %, D-AB09) holds: the cast
/// goes on and fires, and the `interrupt_effect` row says `resisted`.
#[tokio::test]
async fn full_interrupt_resistance_keeps_the_warmup() {
    let mut mgr = interrupt_mgr(1000);
    let (tx, mut rx) = mpsc::channel(512);
    npc_starts_casting(&mut mgr, &tx).await;
    let _ = drain(&mut rx);

    let logs = LogCapture::install();
    assert!(handle_use_ability(1, INTERRUPTING_SHOT, 2, &tx, &mut mgr).await);
    assert!(
        sequences(&drain(&mut rx), 2).is_empty(),
        "no interrupt sent"
    );
    assert!(crate::cell::abilities::is_casting(&mgr, 2));
    let row = logs
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "interrupt_effect"))
        .unwrap_or_else(|| panic!("no interrupt_effect row: {:#?}", logs.all()));
    assert!(row.has_field("decision_outcome", "resisted"), "{row:#?}");
    assert!(row.has_field("entity_id", "1"), "the actor: {row:#?}");
    assert!(row.has_field("target_id", "2"), "the subject: {row:#?}");

    resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await;
    assert_eq!(effect_results(&drain(&mut rx), 2), 1, "the cast fires");
}

/// A target with no warmup and no channel is not rolled: nothing is sent
/// and the row says `nothing_to_interrupt`.
#[tokio::test]
async fn an_idle_target_has_nothing_to_interrupt() {
    let mut mgr = interrupt_mgr(0);
    let (tx, mut rx) = mpsc::channel(512);
    let logs = LogCapture::install();
    assert!(handle_use_ability(1, INTERRUPTING_SHOT, 2, &tx, &mut mgr).await);
    assert!(sequences(&drain(&mut rx), 2).is_empty());
    assert!(mgr.pending_interrupts.is_empty(), "the queue is drained");
    assert!(logs
        .all()
        .iter()
        .any(|c| c.has_field("event", "interrupt_effect")
            && c.has_field("decision_outcome", "nothing_to_interrupt")));
}
