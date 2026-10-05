//! Tests for the AB-C6 timing joins (`timing.rs`).

use super::*;

fn args<'a>(pairs: &'a [(&'a str, i64)]) -> impl Fn(&str) -> Option<i64> + 'a {
    move |k| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| *v)
}

const ME: i32 = 5;

/// A local reply of `kind` for ability `a`.
fn reply(kind: ReplyKind, a: i32, cast_id: Option<i32>) -> Reply {
    let method = match kind {
        ReplyKind::Warmup | ReplyKind::Cooldown => "onTimerUpdate",
        ReplyKind::Results => "onEffectResults",
        ReplyKind::Refusal => "onErrorCode",
    };
    Reply {
        method,
        entity_id: Some(ME),
        timer_id: matches!(kind, ReplyKind::Warmup | ReplyKind::Cooldown).then_some(a),
        answers: Some((a, kind)),
        cast_id,
        local: true,
    }
}

/// `(send_id, first)` of the send a reply joined.
fn joined(t: &mut Timing, r: Reply, now: u64) -> Option<(u32, bool)> {
    t.on_recv(r, now)
        .map(|a| (a.send_id, a.sent_to_recv_ms.is_some()))
}

#[test]
fn only_ability_replies_name_an_ability_and_a_kind() {
    use ReplyKind::*;
    let cases: [(&str, &[(&str, i64)], Option<(i32, ReplyKind)>); 7] = [
        (
            "onEffectResults",
            &[("ability_id", 597)],
            Some((597, Results)),
        ),
        (
            "onTimerUpdate",
            &[("timer_type", 1), ("timer_id", 597)],
            Some((597, Warmup)),
        ),
        (
            "onTimerUpdate",
            &[("timer_type", 2), ("timer_id", 597)],
            Some((597, Cooldown)),
        ),
        // An effect duration timer's ID is the effect, not the ability.
        (
            "onTimerUpdate",
            &[("timer_type", 5), ("timer_id", 77)],
            None,
        ),
        (
            "onErrorCode",
            &[("system_id", 0), ("instance_id", 597)],
            Some((597, Refusal)),
        ),
        (
            "onErrorCode",
            &[("system_id", 3), ("instance_id", 597)],
            None,
        ),
        ("onSequence", &[("ability_id", 597)], None),
    ];
    for (method, a, want) in cases {
        assert_eq!(recv_reply(method, args(a)), want, "{method} {a:?}");
    }
    assert_eq!(
        recv_reply("onEffectResults", args(&[("ability_id", 0)])),
        None
    );
}

/// A press, its send, a warmup timer 80 ms later, the effect results
/// 1.2 s later, and the cooldown applied 15 ms after its timer.
#[test]
fn a_cast_is_timed_from_send_to_first_answer_to_applied() {
    let mut t = Timing::default();
    t.note_sent(1, Some(10), "useAbility", Some(597), 1_000);
    // Another ability's answer does not claim this send.
    assert_eq!(
        joined(&mut t, reply(ReplyKind::Results, 42, Some(9)), 1_050),
        None
    );
    let first = t
        .on_recv(reply(ReplyKind::Warmup, 597, None), 1_080)
        .unwrap();
    assert_eq!(
        first,
        Answered {
            send_id: 1,
            press_id: Some(10),
            sent_method: "useAbility",
            sent_to_recv_ms: Some(80),
        }
    );
    // The later replies of the same cast follow it, with no interval.
    assert_eq!(
        joined(&mut t, reply(ReplyKind::Results, 597, Some(31)), 2_200),
        Some((1, false))
    );
    assert_eq!(
        joined(&mut t, reply(ReplyKind::Cooldown, 597, None), 2_210),
        Some((1, false))
    );
    assert_eq!(
        t.on_applied("cooldown", Some(ME), Some(597), 2_225),
        Some(15)
    );
    assert_eq!(t.on_applied("cooldown", Some(ME), Some(598), 2_225), None);
    assert_eq!(t.on_applied("cooldown", Some(6), Some(597), 2_225), None);
    // Too long after its receive: no join.
    assert_eq!(t.on_applied("cooldown", Some(ME), Some(597), 9_000), None);
    assert_eq!(t.on_applied("unknown_kind", Some(ME), None, 2_225), None);
}

#[test]
fn stat_and_state_flag_join_their_own_methods_by_entity() {
    let mut t = Timing::default();
    let plain = |method, entity| Reply {
        method,
        entity_id: Some(entity),
        timer_id: None,
        answers: None,
        cast_id: None,
        local: true,
    };
    t.on_recv(plain("onStatUpdate", 5), 100);
    t.on_recv(plain("onStateFieldUpdate", 9), 110);
    // The timer id of a non-timer kind is ignored.
    assert_eq!(t.on_applied("stat", Some(5), Some(3), 104), Some(4));
    assert_eq!(t.on_applied("state_flag", Some(9), None, 111), Some(1));
    assert_eq!(t.on_applied("stat_base", Some(5), None, 111), None);
    assert_eq!(t.on_applied("effect_bar_add", Some(5), Some(1), 111), None);
}

/// Copilot on #1185: a `trainAbility` (answered by `onKnownAbilitiesUpdate`,
/// never by a cast reply) waited in the queue and took the next cast's
/// answer. No method outside `ANSWERED_METHODS` is held.
#[test]
fn an_unanswerable_send_is_never_held() {
    let mut t = Timing::default();
    for (i, method) in [
        "trainAbility",
        "resetMyAbilities",
        "confirmationResponse",
        "petAbilityToggle",
        "gmDebugAbility",
        "gmDebugAbilityOnMob",
    ]
    .into_iter()
    .enumerate()
    {
        t.note_sent(i as u32, None, method, Some(597), 0);
    }
    assert!(t.pending.is_empty());
    assert_eq!(
        joined(&mut t, reply(ReplyKind::Cooldown, 597, None), 10),
        None
    );
    for method in ANSWERED_METHODS {
        t.note_sent(99, None, method, Some(597), 20);
    }
    assert_eq!(t.pending.len(), ANSWERED_METHODS.len());
}

#[test]
fn a_held_send_expires_quickly() {
    let mut t = Timing::default();
    t.note_sent(1, None, "useAbility", Some(7), 0);
    assert_eq!(
        joined(
            &mut t,
            reply(ReplyKind::Cooldown, 7, None),
            PENDING_TTL_MS + 1
        ),
        None
    );
    assert!(t.pending.is_empty());
}

/// Copilot on #1185: another player's results for the same ability claimed
/// our send. Only a reply addressed to the local player joins.
#[test]
fn a_witnessed_reply_never_claims_the_local_send() {
    let mut t = Timing::default();
    t.note_sent(1, None, "useAbility", Some(597), 0);
    let mut theirs = reply(ReplyKind::Results, 597, Some(70));
    theirs.entity_id = Some(900);
    theirs.local = false;
    assert_eq!(joined(&mut t, theirs, 10), None);
    assert_eq!(
        joined(&mut t, reply(ReplyKind::Results, 597, Some(71)), 20),
        Some((1, true))
    );
}

/// Copilot on #1185: with two sends of one ability held, cast 1's cooldown
/// took send 1 and its warmup took send 2. Every reply of one cast now joins
/// one send, and the next cast's first reply takes the next send (FIFO).
#[test]
fn two_held_sends_of_one_ability_join_one_cast_each() {
    use ReplyKind::*;
    let mut t = Timing::default();
    t.note_sent(1, None, "useAbility", Some(7), 0);
    t.note_sent(2, None, "useAbility", Some(7), 5);
    // Cast 1: cooldown, warmup, two results of one cast id.
    assert_eq!(
        joined(&mut t, reply(Cooldown, 7, None), 20),
        Some((1, true))
    );
    assert_eq!(joined(&mut t, reply(Warmup, 7, None), 21), Some((1, false)));
    assert_eq!(
        joined(&mut t, reply(Results, 7, Some(50)), 900),
        Some((1, false))
    );
    assert_eq!(
        joined(&mut t, reply(Results, 7, Some(50)), 901),
        Some((1, false))
    );
    // Cast 2.
    assert_eq!(
        joined(&mut t, reply(Cooldown, 7, None), 950),
        Some((2, true))
    );
    assert_eq!(
        joined(&mut t, reply(Results, 7, Some(51)), 960),
        Some((2, false))
    );
    // A late result of cast 1 still finds cast 1 by its cast id.
    assert_eq!(
        joined(&mut t, reply(Results, 7, Some(50)), 970),
        Some((1, false))
    );
    // Nothing is held and both casts have had their cooldown: a third
    // cast's cooldown joins no send.
    assert_eq!(joined(&mut t, reply(Cooldown, 7, None), 980), None);
}

/// A refusal of the second press does not join the cast still warming.
#[test]
fn a_refusal_answers_the_held_send() {
    use ReplyKind::*;
    let mut t = Timing::default();
    t.note_sent(1, None, "useAbility", Some(7), 0);
    assert_eq!(joined(&mut t, reply(Warmup, 7, None), 10), Some((1, true)));
    t.note_sent(2, None, "useAbility", Some(7), 500);
    assert_eq!(
        joined(&mut t, reply(Refusal, 7, None), 520),
        Some((2, true))
    );
    // With nothing held, a refusal (an interrupt) follows the cast.
    assert_eq!(
        joined(&mut t, reply(Refusal, 7, None), 600),
        Some((1, false))
    );
}

#[test]
fn the_tables_are_bounded() {
    let mut t = Timing::default();
    for i in 0..(MAX_SENDS as u32 + 10) {
        t.note_sent(i, None, "useAbility", Some(i as i32 + 1), 0);
    }
    assert_eq!(t.pending.len(), MAX_SENDS);
    assert_eq!(
        t.pending.front().unwrap().send_id,
        10,
        "the oldest go first"
    );
    for i in 10..(MAX_SENDS as i32 + 10) {
        t.on_recv(reply(ReplyKind::Cooldown, i + 1, None), 1);
    }
    assert!(t.casting.len() <= MAX_SENDS);
    for i in 0..(MAX_RECVS as i32 + 10) {
        let mut r = reply(ReplyKind::Cooldown, 1, None);
        r.answers = None;
        r.entity_id = Some(i);
        t.on_recv(r, 2);
    }
    assert!(t.recvs.len() <= MAX_RECVS);
}

fn get<'a>(f: &'a Fields, k: &str) -> Option<&'a Value> {
    f.iter().find(|(n, _)| *n == k).map(|(_, v)| v)
}

/// The row-level joins, through the process tables: a send, its cooldown
/// row, the cooldown the client applied from it, and a witnessed row.
/// Uses ids no other test touches, since the tables are shared.
#[test]
fn recv_and_applied_rows_carry_their_intervals() {
    let now = super::super::now_ms();
    with_timing(|t| t.note_sent(901, Some(902), "useAbility", Some(-31_337), now));
    let base: Fields = vec![
        ("method", json!("onTimerUpdate")),
        ("entity_id", json!(-77)),
        ("timer_id", json!(-31_337)),
        ("timer_type", json!(2)),
    ];
    // Someone else's cooldown of the same ability joins nothing.
    let mut witnessed = base.clone();
    annotate_recv(&mut witnessed, "onTimerUpdate", false, now + 10);
    assert_eq!(get(&witnessed, "send_id"), None);

    let mut recv = base.clone();
    annotate_recv(&mut recv, "onTimerUpdate", true, now + 40);
    assert_eq!(get(&recv, "send_id"), Some(&json!(901)));
    assert_eq!(get(&recv, "press_id"), Some(&json!(902)));
    assert_eq!(get(&recv, "sent_method"), Some(&json!("useAbility")));
    assert_eq!(get(&recv, "send_reply"), Some(&json!("first")));
    assert_eq!(get(&recv, "sent_to_recv_ms"), Some(&json!(40)));

    let mut applied: Fields = vec![
        ("kind", json!("cooldown")),
        ("entity_id", json!(-77)),
        ("ability_id", json!(-31_337)),
        ("timer_id", json!(-31_337)),
    ];
    annotate_applied(&mut applied, now + 52);
    assert_eq!(get(&applied, "recv_to_applied_ms"), Some(&json!(12)));

    // The cast's results follow the send, with no interval.
    let mut results: Fields = vec![
        ("method", json!("onEffectResults")),
        ("entity_id", json!(-77)),
        ("cast_id", json!(-4_444)),
        ("ability_id", json!(-31_337)),
    ];
    annotate_recv(&mut results, "onEffectResults", true, now + 60);
    assert_eq!(get(&results, "send_id"), Some(&json!(901)));
    assert_eq!(get(&results, "send_reply"), Some(&json!("follow_up")));
    assert_eq!(get(&results, "sent_to_recv_ms"), None);

    // A row with no kind, or a kind with no source, is left alone.
    let mut other: Fields = vec![("kind", json!("mystery")), ("entity_id", json!(-77))];
    annotate_applied(&mut other, now + 61);
    assert_eq!(other.len(), 2);
}

/// Every interval lands in the uploader's histogram under its label,
/// method or kind, never an id; a follow-up reply adds none.
#[test]
fn intervals_feed_the_histograms_with_enumerated_labels() {
    let _ = ability_timing::drain();
    let mut t = Timing::default();
    Timing::press_sent("useAbility", 4);
    t.note_sent(1, None, "useAbility", Some(597), 0);
    t.on_recv(reply(ReplyKind::Warmup, 597, None), 50);
    t.on_recv(reply(ReplyKind::Results, 597, Some(3)), 70);
    let mut r = reply(ReplyKind::Warmup, 597, None);
    r.timer_id = Some(77);
    r.answers = None;
    t.on_recv(r, 50);
    t.on_applied("effect_bar_refresh", Some(ME), Some(77), 60);
    let mut got: Vec<(String, String, u64, u64)> = ability_timing::drain()
        .into_iter()
        .map(|f| {
            (
                f["stage"].as_str().unwrap().to_string(),
                f["label"].as_str().unwrap().to_string(),
                f["count"].as_u64().unwrap(),
                f["sum_ms"].as_u64().unwrap(),
            )
        })
        .collect();
    got.sort();
    assert_eq!(
        got,
        vec![
            ("press_to_sent".into(), "useAbility".into(), 1, 4),
            ("recv_to_applied".into(), "effect_bar".into(), 1, 10),
            ("sent_to_recv".into(), "onTimerUpdate".into(), 1, 50),
        ]
    );
}
