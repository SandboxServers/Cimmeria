use super::*;

fn ev(seq: u64, kind: &str, ts: i64, fields: Value) -> StoredEvent {
    StoredEvent {
        seq,
        kind: kind.into(),
        ts_ms: ts,
        fields,
    }
}

#[test]
fn a_landed_shot_reads_as_effect_applied_with_timings() {
    let evs = vec![
        ev(1, "net.out", 1010, json!({ "method": "useAbility" })),
        ev(
            2,
            "cme.event",
            1100,
            json!({ "event": "Event_NetIn_onTimerUpdate" }),
        ),
        ev(
            3,
            "cme.event",
            1120,
            json!({ "event": "Event_NetIn_onSequence" }),
        ),
        ev(
            4,
            "cme.event",
            1400,
            json!({ "event": "Event_NetIn_onEffectResults" }),
        ),
        ev(
            5,
            "combat.hit",
            1450,
            json!({ "ability_id": 1100, "hit_name": "Hit" }),
        ),
        ev(
            6,
            "combat.hit",
            1460,
            json!({ "ability_id": 999, "hit_name": "Hit" }),
        ),
    ];
    let o = classify(&evs, 1000, 1100);
    assert_eq!(o.verdict(), "effect_applied");
    assert!(o.settled());
    assert_eq!(o.sent, vec![(1, "useAbility".into())]);
    assert_eq!(o.first_ms.sent, Some(10));
    assert_eq!(o.first_ms.cast, Some(120));
    assert_eq!(o.first_ms.effect, Some(400));
    assert_eq!(o.combat.len(), 1, "another ability's hit is not ours");
}

#[test]
fn an_error_code_is_a_refusal() {
    let evs = vec![
        ev(1, "net.out", 1, json!({ "method": "useAbility" })),
        ev(
            2,
            "cme.event",
            2,
            json!({ "event": "Event_NetIn_onErrorCode" }),
        ),
        ev(
            3,
            "chat.line",
            3,
            json!({ "text": "Target is out of range", "speaker": "" }),
        ),
    ];
    let o = classify(&evs, 0, 1);
    assert_eq!(o.verdict(), "refused");
    assert_eq!(o.feedback, vec!["Target is out of range".to_string()]);
}

#[test]
fn client_side_refusals_send_nothing() {
    let evs = vec![ev(
        1,
        "chat.line",
        3,
        json!({ "text": "You must have a target", "speaker": "" }),
    )];
    assert_eq!(classify(&evs, 0, 1).verdict(), "refused_client_side");
    assert_eq!(classify(&[], 0, 1).verdict(), "nothing_observed");
}

#[test]
fn a_sent_use_with_no_reply_and_player_chat_is_not_feedback() {
    let evs = vec![
        ev(
            1,
            "net.out",
            1,
            json!({ "method": "useAbilityOnGroundTarget" }),
        ),
        ev(
            2,
            "chat.line",
            2,
            json!({ "text": "lol", "speaker": "Bob" }),
        ),
        ev(3, "net.out", 3, json!({ "method": "setTarget" })),
    ];
    let o = classify(&evs, 0, 1);
    assert_eq!(o.verdict(), "sent_no_reply");
    assert!(o.feedback.is_empty());
    assert_eq!(o.sent.len(), 1);
}

#[test]
fn a_cast_without_an_effect_yet() {
    let evs = vec![
        ev(1, "net.out", 1, json!({ "method": "useAbility" })),
        ev(
            2,
            "cme.event",
            2,
            json!({ "event": "Event_NetIn_onSequence" }),
        ),
    ];
    let o = classify(&evs, 0, 1);
    assert_eq!(o.verdict(), "cast_started");
    assert!(!o.settled());
}

// ---- the ability trace (`ability.*` rows) ------------------------------

/// Heal Focus (Self), one of the five starter abilities.
const HEAL_FOCUS: i64 = 597;

/// The live Heal Focus press (colo, 2026-10-04) as the lab store held it:
/// the old net.out name, the trace's press and send, the warmup (type 1)
/// and cooldown (type 2) timers received and applied, then a stat update
/// received and applied. No `cme.event` rows: the reader that only knew
/// those names called this `sent_no_reply`.
fn heal_focus_press() -> Vec<StoredEvent> {
    vec![
        ev(
            1,
            "net.out",
            1010,
            json!({ "method": "useAbility", "entity_id": 77 }),
        ),
        ev(
            2,
            "ability.press",
            1005,
            json!({ "press_id": 3, "source": "hotbar", "slot": 1, "ability_id": HEAL_FOCUS }),
        ),
        ev(
            3,
            "ability.sent",
            1011,
            json!({ "send_id": 9, "press_id": 3, "method": "useAbility", "ability_id": HEAL_FOCUS }),
        ),
        ev(
            4,
            "ability.recv",
            1180,
            json!({ "method": "onTimerUpdate", "method_index": 12, "timer_type": 1,
                    "timer_id": HEAL_FOCUS, "path": "local_player" }),
        ),
        ev(
            5,
            "ability.applied",
            1181,
            json!({ "kind": "cooldown", "outcome": "applied", "ability_id": HEAL_FOCUS,
                    "timer_type": 1 }),
        ),
        ev(
            6,
            "ability.recv",
            1190,
            json!({ "method": "onTimerUpdate", "method_index": 12, "timer_type": 2,
                    "timer_id": HEAL_FOCUS, "path": "local_player" }),
        ),
        ev(
            7,
            "ability.applied",
            1191,
            json!({ "kind": "cooldown", "outcome": "applied", "ability_id": HEAL_FOCUS,
                    "timer_type": 2 }),
        ),
        ev(
            8,
            "ability.recv",
            2900,
            json!({ "method": "onStatUpdate", "method_index": 20, "stats_count": 1 }),
        ),
        ev(
            9,
            "ability.applied",
            2901,
            json!({ "kind": "stat", "entity_id": 77, "stats_count": 1,
                    "stats": [[1, 0, 640, 700]] }),
        ),
    ]
}

#[test]
fn the_live_heal_focus_press_reads_as_an_applied_effect() {
    let o = classify(&heal_focus_press(), 1000, HEAL_FOCUS);
    assert_eq!(o.verdict(), "effect_applied");
    assert!(o.settled());
    // Two timers, each seen as a recv and an applied row: counted once.
    assert_eq!(o.timer_updates, 2);
    assert_eq!(o.stat_applies, 1);
    assert_eq!(o.ability_sent, 1);
    assert_eq!(o.first_ms.timer, Some(180));
    assert_eq!(o.first_ms.effect, Some(1901));
    let j = o.to_json();
    assert_eq!(j["effect_applied"], true);
    assert_eq!(j["timer_updates"], 2);
}

#[test]
fn trace_timers_alone_mean_the_cast_was_accepted() {
    let evs: Vec<StoredEvent> = heal_focus_press().into_iter().take(7).collect();
    let o = classify(&evs, 1000, HEAL_FOCUS);
    assert_eq!(o.verdict(), "cast_started");
    assert_eq!(o.timer_updates, 2);
    assert!(!o.settled(), "a stat update may still follow");
    // The serialized flag agrees with the verdict.
    let j = o.to_json();
    assert_eq!(j["verdict"], "cast_started");
    assert_eq!(j["cast_started"], true);
    assert_eq!(j["sequences"], 0);
}

/// Copilot on #1197: the store slice starts at the baseline pump, before
/// the hotbar reads and any placement. A regen tick or another timer
/// during that setup must not answer the press.
#[test]
fn events_before_the_press_are_not_its_answer() {
    let evs = vec![
        ev(
            1,
            "ability.applied",
            900,
            json!({ "kind": "stat", "entity_id": 77, "stats": [[1, 0, 600, 700]] }),
        ),
        ev(
            2,
            "ability.applied",
            950,
            json!({ "kind": "cooldown", "outcome": "applied", "ability_id": HEAL_FOCUS }),
        ),
        ev(
            3,
            "cme.event",
            960,
            json!({ "event": "Event_NetIn_onEffectResults" }),
        ),
        ev(
            4,
            "ability.sent",
            1010,
            json!({ "method": "useAbility", "ability_id": HEAL_FOCUS }),
        ),
    ];
    let o = classify(&evs, 1000, HEAL_FOCUS);
    assert_eq!(o.verdict(), "sent_no_reply");
    assert_eq!(
        (o.stat_applies, o.timer_updates, o.effect_results),
        (0, 0, 0)
    );
    assert_eq!(o.first_ms.effect, None);
}

/// Copilot on #1197: a send and a cooldown that name another ability (an
/// overlapping automated cast) are not this press's.
#[test]
fn another_abilitys_send_and_cooldown_are_skipped() {
    let evs = vec![
        ev(
            1,
            "ability.sent",
            5,
            json!({ "method": "useAbility", "ability_id": 592 }),
        ),
        ev(
            2,
            "ability.applied",
            6,
            json!({ "kind": "cooldown", "outcome": "applied", "ability_id": 592, "timer_type": 2 }),
        ),
        ev(
            3,
            "ability.press_dropped",
            7,
            json!({ "ability_id": HEAL_FOCUS, "reason": "not_known" }),
        ),
    ];
    let o = classify(&evs, 0, HEAL_FOCUS);
    assert_eq!(o.ability_sent, 0);
    assert_eq!(o.timer_updates, 0);
    // The other cast's send no longer masks this press's drop.
    assert_eq!(o.verdict(), "refused_client_side");
    // Seen from the other ability, the same rows are its send and timer.
    let other = classify(&evs, 0, 592);
    assert_eq!(other.verdict(), "cast_started");
    assert_eq!(other.ability_sent, 1);
}

#[test]
fn a_trace_send_counts_without_the_old_net_out() {
    let evs = vec![ev(
        1,
        "ability.sent",
        5,
        json!({ "method": "useAbility", "ability_id": HEAL_FOCUS }),
    )];
    let o = classify(&evs, 0, HEAL_FOCUS);
    assert_eq!(o.verdict(), "sent_no_reply");
    assert_eq!(o.to_json()["sent"], true);
}

#[test]
fn a_dropped_press_is_a_client_side_refusal() {
    let evs = vec![
        ev(
            1,
            "ability.press",
            5,
            json!({ "press_id": 4, "source": "hotbar", "ability_id": HEAL_FOCUS }),
        ),
        ev(
            2,
            "ability.press_dropped",
            6,
            json!({ "press_id": 4, "source": "hotbar", "ability_id": HEAL_FOCUS,
                    "reason": "not_known", "drop_site": "0x00d2afcf" }),
        ),
    ];
    let o = classify(&evs, 0, HEAL_FOCUS);
    assert_eq!(o.verdict(), "refused_client_side");
    assert!(o.settled(), "a dropped press is final");
    assert_eq!(o.press_dropped, vec!["not_known".to_string()]);
    assert_eq!(o.to_json()["press_dropped"][0], "not_known");
    // Another ability's drop is not ours.
    assert_eq!(classify(&evs, 0, 1100).verdict(), "nothing_observed");
}

#[test]
fn trace_recv_effects_errors_and_shown_rows() {
    let effect = vec![ev(
        1,
        "ability.recv",
        5,
        json!({ "method": "onEffectResults", "ability_id": HEAL_FOCUS, "effect_id": 31 }),
    )];
    assert_eq!(classify(&effect, 0, HEAL_FOCUS).verdict(), "effect_applied");
    assert_eq!(classify(&effect, 0, 1100).verdict(), "nothing_observed");

    let refused = vec![
        ev(1, "ability.sent", 1, json!({ "method": "useAbility" })),
        ev(
            2,
            "ability.recv",
            2,
            json!({ "method": "onErrorCode", "error_code": 7 }),
        ),
        ev(
            3,
            "ability.shown",
            3,
            json!({ "kind": "feedback_line", "text": "Not enough focus" }),
        ),
        ev(
            4,
            "chat.line",
            3,
            json!({ "text": "Not enough focus", "speaker": "" }),
        ),
    ];
    let o = classify(&refused, 0, HEAL_FOCUS);
    assert_eq!(o.verdict(), "refused");
    assert_eq!(o.feedback, vec!["Not enough focus".to_string()], "once");

    let shown = vec![ev(
        1,
        "ability.shown",
        4,
        json!({ "kind": "combat_text", "ability_id": HEAL_FOCUS, "hit_type": 1 }),
    )];
    assert_eq!(classify(&shown, 0, HEAL_FOCUS).verdict(), "effect_applied");

    let buff = vec![ev(
        1,
        "ability.applied",
        4,
        json!({ "kind": "effect_bar_add", "effect_id": 1462 }),
    )];
    assert_eq!(classify(&buff, 0, HEAL_FOCUS).verdict(), "effect_applied");
}

#[test]
fn another_beings_cooldown_is_not_a_timer_for_this_press() {
    let evs = vec![
        ev(1, "ability.sent", 1, json!({ "method": "useAbility" })),
        ev(
            2,
            "ability.applied",
            2,
            json!({ "kind": "cooldown", "outcome": "other_source" }),
        ),
        ev(
            3,
            "ability.applied",
            3,
            json!({ "kind": "state_flag", "changed": 2 }),
        ),
    ];
    let o = classify(&evs, 0, HEAL_FOCUS);
    assert_eq!(o.timer_updates, 0);
    assert_eq!(o.verdict(), "sent_no_reply");
}
