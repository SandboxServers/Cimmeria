//! Clause evaluation tests: comparisons, selectors, the chat diff, and
//! SigNoz and packet grading.

use super::*;

fn clause(src: &str) -> ExpectSpec {
    let t = format!("id = \"c\"\ntext = \"t\"\n{src}");
    toml::from_str(&t).unwrap()
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

#[test]
fn lua_strings_compare_as_numbers() {
    assert!(compare(Some(Op::Eq), Some(&json!("40")), Some(&json!(40))).unwrap());
    assert!(compare(Some(Op::Gte), Some(&json!("6")), Some(&json!(6))).unwrap());
    assert!(!compare(Some(Op::Gt), Some(&json!("n/a")), Some(&json!(1))).unwrap());
    assert!(compare(None, Some(&json!("true")), None).unwrap());
    assert!(!compare(None, Some(&json!("false")), None).unwrap());
    assert!(compare(Some(Op::Absent), None, None).unwrap());
    assert!(compare(Some(Op::LenGte), Some(&json!([1, 2])), Some(&json!(2))).unwrap());
}

#[test]
fn selectors_pick_an_array_element_by_field() {
    let snap = json!({ "state": {
        "stats": [{ "stat_id": 4, "cur": 90 }, { "stat_id": 8, "cur": 140 }],
        "ledger": [{ "ability_id": 637, "stats": [{ "stat_id": 22, "requested": 200 }] }],
    }});
    let at = |p| json_at(&snap, Some(p)).cloned();
    assert_eq!(at("/state/stats[stat_id=8]/cur"), Some(json!(140)));
    assert_eq!(
        at("/state/ledger[ability_id=637]/stats[stat_id=22]/requested"),
        Some(json!(200))
    );
    assert_eq!(at("/state/stats[stat_id=9]/cur"), None);
    assert_eq!(at("/state/stats[stat_id=8"), None);
    assert_eq!(at("/state/stats/1/cur"), Some(json!(140)));
    // No selector: plain RFC 6901, as before.
    assert_eq!(at("/state/stats/0/stat_id"), Some(json!(4)));
}

#[test]
fn new_lines_follow_a_sliding_tail() {
    let before = s(&["a", "b", "c"]);
    assert_eq!(
        new_lines(&before, &s(&["b", "c", "d", "e"])),
        (s(&["d", "e"]), true)
    );
    // Repeated text: the longest overlap wins, so a second "c" is new.
    assert_eq!(
        new_lines(&before, &s(&["a", "b", "c", "c"])),
        (s(&["c"]), true)
    );
    // Cleared box (relog): everything is new, flagged.
    assert_eq!(new_lines(&before, &s(&["x"])), (s(&["x"]), false));
    assert_eq!(new_lines(&[], &s(&["x"])), (s(&["x"]), true));
}

#[test]
fn chat_count_catches_a_double_echo() {
    let c = clause("source = \"chat\"\ncontains = \"hi\"\ncount = 1");
    let (v, obs, _) = eval_chat(&c, &s(&["[Say] Labone: hi", "[Say] Labone: hi"]));
    assert_eq!(v, Verdict::Fail);
    assert_eq!(obs["match_count"], 2);
    assert_eq!(eval_chat(&c, &s(&["[Say] Labone: hi"])).0, Verdict::Pass);
}

#[test]
fn chat_capture_takes_group_one() {
    let c = clause("source = \"chat\"\nmatches = 'Bookmark (\\d+) recorded'");
    let (v, _, cap) = eval_chat(&c, &s(&["Bookmark 1790650000123 recorded: 3 of 3"]));
    assert_eq!(v, Verdict::Pass);
    assert_eq!(cap.as_deref(), Some("1790650000123"));
}

#[test]
fn absent_passes_only_with_no_match() {
    let c = clause("source = \"chat\"\ncontains = \"error\"\nabsent = true");
    assert_eq!(eval_chat(&c, &s(&["fine"])).0, Verdict::Pass);
    assert_eq!(eval_chat(&c, &s(&["an error"])).0, Verdict::Fail);
}

#[test]
fn signoz_rows_and_fields_grade() {
    let c = clause("source = \"signoz\"\nfilter = \"x\"");
    assert_eq!(grade_signoz(&c, 0, &[]).unwrap(), Verdict::Fail);
    assert_eq!(grade_signoz(&c, 2, &[]).unwrap(), Verdict::Pass);
    let c = clause(
        "source = \"signoz\"\nfilter = \"x\"\nfield = \"outcome\"\nop = \"eq\"\nvalue = \"up_to_date\"",
    );
    let ok = [json!({"outcome": "up_to_date"})];
    let bad = [json!({"outcome": "full_resync"})];
    assert_eq!(grade_signoz(&c, 1, &ok).unwrap(), Verdict::Pass);
    assert_eq!(grade_signoz(&c, 1, &bad).unwrap(), Verdict::Fail);
    assert!(grade_signoz(&c, 1, &[]).is_err());
    let none = clause("source = \"signoz\"\nfilter = \"x\"\nmax_rows = 0");
    assert_eq!(grade_signoz(&none, 0, &[]).unwrap(), Verdict::Pass);
    assert_eq!(grade_signoz(&none, 1, &[]).unwrap(), Verdict::Fail);
}

#[test]
fn approx_holds_within_the_tolerance_either_way() {
    let ap = |obs: Value| compare_tol(Some(Op::Approx), Some(&obs), Some(&json!(15)), Some(1.0));
    assert!(ap(json!(15)).unwrap());
    assert!(ap(json!(14.0)).unwrap());
    assert!(ap(json!("16")).unwrap());
    assert!(!ap(json!(16.5)).unwrap());
    assert!(!ap(json!("n/a")).unwrap());
    assert!(compare(Some(Op::Approx), Some(&json!(15)), Some(&json!(15))).is_err());
}

/// A canned `server_packet_tap_read` result: two timer sends for
/// entity 7 (decoded 15.2 s and 14.6 s), one for entity 9, and an
/// inbound cast request.
fn tap() -> Value {
    json!({
        "entity_id": 7, "capacity": 5000, "count": 4, "dropped": 0,
        "messages": [
            { "ts_ms": 1, "dir": "in", "msg_id": 9, "method_index": 12, "msg_name": "useAbility",
              "target_entity_id": null, "args_len": 8, "args_hex": "00", "decoded": { "abilityId": 1234 } },
            { "ts_ms": 2, "dir": "out", "msg_id": null, "method_index": 40, "msg_name": "onTimerUpdate",
              "target_entity_id": 7, "args_len": 12, "args_hex": "00", "decoded": { "complete_in_s": 15.2 } },
            { "ts_ms": 3, "dir": "out", "msg_id": null, "method_index": 40, "msg_name": "onTimerUpdate",
              "target_entity_id": 9, "args_len": 12, "args_hex": "00", "decoded": { "complete_in_s": 99 } },
            { "ts_ms": 4, "dir": "out", "msg_id": null, "method_index": 40, "msg_name": "onTimerUpdate",
              "target_entity_id": 7, "args_len": 12, "args_hex": "00", "decoded": { "complete_in_s": 14.6 } },
        ]
    })
}

fn packet(extra: &str) -> ExpectSpec {
    clause(&format!(
        "source = \"packet\"\nmessage = \"onTimerUpdate\"\ndirection = \"to_client\"\n{extra}"
    ))
}

#[test]
fn packet_clause_passes_on_matching_rows_and_fields() {
    let c = packet("field = \"complete_in_s\"\nop = \"approx\"\nvalue = 15\ntolerance = 1");
    let (v, obs, detail) = grade_packet(&c, Some(7), &tap());
    assert_eq!(v, Verdict::Pass, "{detail:?}");
    assert_eq!(obs["matching_rows"], 2);
    assert_eq!(obs["tapped_rows"], 4);
    // The tap columns survive beside the decoded fields.
    assert_eq!(obs["rows"][0]["msg_name"], "onTimerUpdate");
    // Inbound: the cast request is to_server, never to_client.
    let c = clause(
        "source = \"packet\"\nmessage = \"useAbility\"\ndirection = \"to_server\"\nfield = \"abilityId\"\nop = \"eq\"\nvalue = 1234",
    );
    assert_eq!(grade_packet(&c, None, &tap()).0, Verdict::Pass);
    let c = clause("source = \"packet\"\nmessage = \"useAbility\"\ndirection = \"to_client\"");
    assert_eq!(grade_packet(&c, None, &tap()).0, Verdict::Fail);
}

#[test]
fn packet_clause_fails_on_a_bad_field_or_count() {
    // Without the entity filter, entity 9's 99 s breaks "every row".
    let c = packet("field = \"complete_in_s\"\nop = \"approx\"\nvalue = 15\ntolerance = 1");
    assert_eq!(grade_packet(&c, None, &tap()).0, Verdict::Fail);
    let c = packet("max_rows = 1");
    assert_eq!(grade_packet(&c, Some(7), &tap()).0, Verdict::Fail);
    let c = packet("min_rows = 3");
    assert_eq!(grade_packet(&c, Some(7), &tap()).0, Verdict::Fail);
}

#[test]
fn an_empty_tap_fails_a_wanted_message_and_passes_an_absent_one() {
    let empty = json!({ "messages": [], "dropped": 0 });
    assert_eq!(grade_packet(&packet(""), None, &empty).0, Verdict::Fail);
    assert_eq!(
        grade_packet(&packet("max_rows = 0"), None, &empty).0,
        Verdict::Pass
    );
    // A read missing either field is not an empty, lossless tap.
    for bad in [
        json!({ "dropped": 0 }),
        json!({ "messages": [] }),
        json!({ "messages": {}, "dropped": 0 }),
        json!({ "messages": [], "dropped": "0" }),
    ] {
        let (v, _, why) = grade_packet(&packet("max_rows = 0"), None, &bad);
        assert_eq!(v, Verdict::Unverified, "{bad}");
        assert!(why.unwrap().contains("no messages array"));
    }
    // A ring that dropped messages cannot prove "none".
    let lossy = json!({ "messages": [], "dropped": 3 });
    let (v, _, why) = grade_packet(&packet("max_rows = 0"), None, &lossy);
    assert_eq!(v, Verdict::Unverified);
    assert!(why.unwrap().contains("dropped 3"));
}

#[test]
fn packet_match_fields_pick_the_messages_a_clause_counts() {
    // Two timer sends for entity 7: 15.2 s and 14.6 s.
    let c = packet("match_fields = { complete_in_s = 15.2 }\nmin_rows = 1\nmax_rows = 1");
    let (v, obs, _) = grade_packet(&c, Some(7), &tap());
    assert_eq!(v, Verdict::Pass);
    assert_eq!(obs["matching_rows"], 1);
    // A value no message carries matches nothing.
    let c = packet("match_fields = { complete_in_s = 3 }");
    assert_eq!(grade_packet(&c, Some(7), &tap()).0, Verdict::Fail);
}
