//! AB-T4 (Copilot on #1175): the launch's cooldown and warmup timers, and
//! the interrupt's clears, carry the cast's `cast_id` on their
//! `abilities.wire` rows. The cast scope opens only at fire, so these sends
//! name the cast themselves; a revert to the plain `send_timer_update`
//! leaves the rows without `cast_id`.

use std::time::Instant;

use crate::test_support::{Captured, LogCapture, NoContentEvents};

use super::warmup::{warmup_mgr, WARMUP_ABILITY};
use super::*;
use crate::cell::abilities::resolve_warmups;

fn timer_rows(all: &[Captured]) -> Vec<Captured> {
    all.iter()
        .filter(|c| {
            c.target == "abilities.wire"
                && c.has_field("event", "wire_sent")
                && c.has_field("method", "onTimerUpdate")
        })
        .cloned()
        .collect()
}

#[tokio::test]
async fn launch_and_interrupt_timer_rows_carry_the_cast_id() {
    let mut mgr = warmup_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    let logs = LogCapture::install();

    assert!(handle_use_ability(1, WARMUP_ABILITY, 2, &tx, &mut mgr).await);
    let cast = mgr
        .get_entity(1)
        .unwrap()
        .pending_cast
        .as_ref()
        .unwrap()
        .effect_seq
        .to_string();
    let launch = timer_rows(&logs.all());
    let row = |origin: &str, kind: &str, rows: &[Captured]| -> Captured {
        rows.iter()
            .find(|c| c.has_field("origin", origin) && c.has_field("timer_type", kind))
            .unwrap_or_else(|| panic!("no {origin} {kind} row: {rows:#?}"))
            .clone()
    };
    for (origin, kind) in [("ability_launch", "cooldown"), ("ability_warmup", "warmup")] {
        let r = row(origin, kind, &launch);
        assert!(r.has_field("cast_id", &cast), "{r:?}");
        assert!(r.has_field("action", "start"), "{r:?}");
    }

    drain(&mut rx);
    mgr.get_entity_mut(1).unwrap().position.x += 1.0;
    resolve_warmups(Instant::now(), &tx, &mut mgr, &NoContentEvents).await;

    let all = timer_rows(&logs.all());
    let clears: Vec<_> = all
        .iter()
        .filter(|c| c.has_field("origin", "warmup_interrupt"))
        .cloned()
        .collect();
    assert_eq!(clears.len(), 2, "warmup and cooldown clears: {all:#?}");
    for r in &clears {
        assert!(r.has_field("cast_id", &cast), "{r:?}");
        assert!(r.has_field("action", "clear"), "{r:?}");
    }
}
