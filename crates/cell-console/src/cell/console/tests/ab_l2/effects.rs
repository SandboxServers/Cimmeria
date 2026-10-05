//! `.effects [target]`: the AB-T5 snapshot as chat lines, of the selected
//! target in view, else the caller. Read-only.

use std::time::{Duration, Instant};

use cimmeria_entity::cell_entity::ActiveEffectInstance;
use tracing::Level;

use super::{aim, console, lines, timers, world, CALLER};
use crate::cell::console::abilities::effects::{format_lines, MAX_PER_LINE};
use crate::test_support::LogCapture;

#[tokio::test]
async fn ab_l2_effects_reads_the_callers_state_without_a_target() {
    let (mut mgr, _npc) = world(2);
    let caller = mgr.get_entity_mut(CALLER).unwrap();
    caller
        .abilities
        .start_ability_cooldown(592, Duration::from_secs(30));
    caller
        .apply_timed_effect(aim(CALLER), Instant::now())
        .unwrap();
    let logs = LogCapture::install();

    let msgs = console(&mut mgr, None, ".effects").await;

    let out = lines(&msgs);
    assert_eq!(out.len(), 4, "header, cooldowns, pulsing, ledger: {out:?}");
    assert!(
        out[0].starts_with(&format!("effects [{CALLER}]")),
        "{out:?}"
    );
    assert!(out[0].contains("warmup none"), "{out:?}");
    assert!(out[1].starts_with("cooldowns: 592 "), "{out:?}");
    assert_eq!(out[2], "pulsing: none");
    assert!(
        out[3].starts_with("ledger: 700 (ability 637) from 1, ")
            && out[3].contains("stat 11 +200")
            && out[3].ends_with("[cast 41]"),
        "{out:?}"
    );
    assert!(timers(&msgs).is_empty(), "read-only: nothing sent but chat");
    // The dispatcher's audit row is the command's one log row.
    let audit = logs
        .find_message(Level::INFO, "GM .-console command accepted")
        .expect("audit row");
    assert!(audit.has_field("command", "effects"));
    assert_eq!(
        mgr.get_entity(CALLER).unwrap().stat_buffs.entries.len(),
        1,
        "nothing changed"
    );
}

#[tokio::test]
async fn ab_l2_effects_reads_the_selected_target() {
    let (mut mgr, npc) = world(2);
    let now = Instant::now();
    mgr.get_entity_mut(npc)
        .unwrap()
        .active_effects
        .push(ActiveEffectInstance {
            effect_id: 5001,
            ability_id: 800,
            invoker_id: CALLER,
            remaining_pulses: 3,
            total_pulses: 5,
            next_pulse_at: now + Duration::from_secs(2),
            pulse_interval_secs: 2.0,
            invoker_position_at_register: None,
            cast_id: Some(12),
            invoker_identity: Default::default(),
            invoker_name: None,
        });

    let out = lines(&console(&mut mgr, Some(npc), ".effects").await);

    assert!(out[0].starts_with(&format!("effects [{npc}]")), "{out:?}");
    assert_eq!(
        out[2],
        "pulsing: 5001 (ability 800) from 1, 3/5 pulses left [cast 12]"
    );
}

/// A long list ends with `+N more` instead of flooding the chat window.
#[test]
fn ab_l2_effects_caps_each_line() {
    let (mut mgr, _npc) = world(2);
    let caller = mgr.get_entity_mut(CALLER).unwrap();
    for id in 0..(MAX_PER_LINE as i32 + 3) {
        caller
            .abilities
            .start_ability_cooldown(100 + id, Duration::from_secs(30));
    }
    let s = mgr.ability_state(CALLER).unwrap();
    let out = format_lines("Tester", &s);
    assert!(out[1].ends_with("; +3 more"), "{}", out[1]);
    assert_eq!(out[1].matches(';').count(), MAX_PER_LINE);
}
