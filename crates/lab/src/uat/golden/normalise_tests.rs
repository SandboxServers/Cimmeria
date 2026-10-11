use std::collections::BTreeMap;

use serde_json::{json, Value};

use super::*;

fn out(name: &str, target: u32, decoded: Value, ts: i64) -> Value {
    json!({ "dir": "out", "msg_name": name, "method_index": 1, "msg_id": null,
            "target_entity_id": target, "decoded": decoded, "args_hex": "", "ts_ms": ts })
}

fn rendered(
    messages: &[Value],
    late: bool,
    v: &Variance,
    subst: &[(String, String)],
) -> Normalised {
    let mut labels = EntityLabels::new(8, None, None);
    normalise_tap(messages, late, &mut labels, v, subst)
}

fn vars(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn texts(n: &Normalised) -> Vec<String> {
    n.events.iter().map(Event::render).collect()
}

#[test]
fn globs_match_stars_only_and_case_sensitively() {
    assert!(glob_match("avatarUpdate*", "avatarUpdateExplicit"));
    assert!(glob_match("avatarUpdate*", "avatarUpdate"));
    assert!(glob_match("*", ""));
    assert!(glob_match("on*Update", "onStepUpdate"));
    assert!(glob_match("a*b*c", "abbc"));
    assert!(glob_match("perfStats", "perfStats"));
    assert!(!glob_match("perfStats", "perfstats"));
    assert!(!glob_match("perfStats", "perfStats2"));
    assert!(
        !glob_match("ab*b", "ab"),
        "prefix and suffix may not overlap"
    );
    assert!(!glob_match("on?Update", "onXUpdate"), "? is literal");
}

/// The run id never reaches a fingerprint (F12), also when it sits on the
/// 80-character cut, case-insensitively, and longest value first.
#[test]
fn the_run_id_never_reaches_the_fingerprint() {
    let v = resolve(None, None);
    let say = |text: &str| {
        out(
            "onPlayerCommunication",
            8,
            json!({ "Channel": 9, "Text": text }),
            0,
        )
    };

    let n = rendered(
        &[say("Uat Pxabcdef")],
        false,
        &v,
        &vars(&[("run_id", "abcdef")]),
    );
    assert_eq!(
        texts(&n),
        ["s2c onPlayerCommunication@self Channel=9 Text=Uat Px${run_id}"]
    );

    let both = vars(&[("run_id", "abcdef"), ("character", "Pxabcdef")]);
    let n = rendered(&[say("hello PXABCDEF, run abcdef")], false, &v, &both);
    assert_eq!(
        texts(&n),
        ["s2c onPlayerCommunication@self Channel=9 Text=hello ${character}, run ${run_id}"]
    );

    // The run id starts at character 77, so a cut before substitution
    // would keep "qwe" in one run and "zxc" in the other.
    let straddle = |id: &str| say(&format!("{}{id} tail", "-".repeat(77)));
    let a = rendered(
        &[straddle("qwerty")],
        false,
        &v,
        &vars(&[("run_id", "qwerty")]),
    );
    let b = rendered(
        &[straddle("zxcvbn")],
        false,
        &v,
        &vars(&[("run_id", "zxcvbn")]),
    );
    assert_eq!(texts(&a), texts(&b));
    let text = texts(&a)[0].split("Text=").nth(1).unwrap().to_string();
    assert_eq!(text, format!("{}${{r", "-".repeat(77)));
}

#[test]
fn a_short_value_never_matches_inside_a_placeholder() {
    let v = resolve(None, None);
    let msg = out("onPlayerCommunication", 8, json!({ "Text": "Pxrun" }), 0);
    let n = rendered(
        &[msg],
        false,
        &v,
        &vars(&[("character", "Pxrun"), ("run_id", "run")]),
    );
    assert_eq!(
        texts(&n),
        ["s2c onPlayerCommunication@self Text=${character}"]
    );
}

#[test]
fn allow_unordered_and_collapse_and_settle() {
    let mut v = resolve(None, None);
    v.allow_unordered = vec!["onEffectResults".into()];
    v.collapse = vec!["onTimer*".into()];
    v.key_args = BTreeMap::from([("onEffectResults".into(), vec!["effect".into()])]);
    let effect = |n: u32| out("onEffectResults", 8, json!({ "effect": n }), 0);
    let mark = out("onMark", 8, json!({}), 0);

    let a = rendered(&[effect(1), mark.clone(), effect(2)], false, &v, &[]);
    let b = rendered(&[effect(2), mark.clone(), effect(1)], false, &v, &[]);
    assert_eq!(a.unordered, b.unordered);
    assert_eq!(
        a.unordered,
        [
            "s2c onEffectResults@self effect=1",
            "s2c onEffectResults@self effect=2"
        ]
    );
    assert_eq!(texts(&a), ["s2c onMark@self"]);

    let timer = out("onTimerUpdate", 8, json!({}), 0);
    let n = rendered(
        &[
            timer.clone(),
            timer.clone(),
            timer.clone(),
            mark.clone(),
            timer.clone(),
        ],
        false,
        &v,
        &[],
    );
    assert_eq!(
        texts(&n),
        [
            "s2c onTimerUpdate@self",
            "s2c onMark@self",
            "s2c onTimerUpdate@self"
        ]
    );
    v.collapse.clear();
    let n = rendered(&[timer.clone(), timer.clone(), timer], false, &v, &[]);
    assert_eq!(n.events.len(), 3, "without collapse every event stays");

    // The cut is the first record's server time plus the settle window.
    let at = |ts: i64| out("onMark", 8, json!({}), ts);
    let tap = [at(1000), at(1500), at(1999), at(2000), at(2500)];
    assert_eq!(rendered(&tap, true, &v, &[]).events.len(), 2);
    assert_eq!(rendered(&tap, false, &v, &[]).events.len(), 5);
    v.late_bind_settle_ms = 0;
    assert_eq!(rendered(&tap, true, &v, &[]).events.len(), 5);
}

fn effects_unordered() -> Variance {
    let mut v = resolve(None, None);
    v.allow_unordered = vec!["onEffectResults".into()];
    v.key_args = BTreeMap::from([("onEffectResults".into(), vec!["effect".into()])]);
    v
}

/// Two untagged mobs met only in unordered events number the same in
/// either arrival order, and a later ordered event naming one agrees.
#[test]
fn unordered_labels_do_not_depend_on_arrival() {
    let v = effects_unordered();
    let hit = |mob: u32, effect: u32| out("onEffectResults", mob, json!({ "effect": effect }), 0);
    let mark = out("onMark", 500, json!({}), 0);

    let a = rendered(&[hit(500, 2), hit(600, 1), mark.clone()], false, &v, &[]);
    let b = rendered(&[hit(600, 1), hit(500, 2), mark], false, &v, &[]);
    assert_eq!(a, b);
    assert_eq!(texts(&a), ["s2c onMark@?1"]);
    assert_eq!(
        a.unordered,
        [
            "s2c onEffectResults@?1 effect=2",
            "s2c onEffectResults@?2 effect=1"
        ]
    );

    // Tied apart from the fresh ids: either order renders the same.
    let a = rendered(&[hit(500, 1), hit(600, 1)], false, &v, &[]);
    let b = rendered(&[hit(600, 1), hit(500, 1)], false, &v, &[]);
    assert_eq!(a, b);
    assert_eq!(
        a.unordered,
        [
            "s2c onEffectResults@?1 effect=1",
            "s2c onEffectResults@?2 effect=1"
        ]
    );
}

/// The ordered stream numbers first, even when the unordered event that
/// shares the id came earlier in the tap.
#[test]
fn a_shared_id_takes_its_number_from_the_ordered_stream() {
    let v = effects_unordered();
    let tap = [
        out("onEffectResults", 500, json!({ "effect": 7 }), 0),
        out("onMark", 600, json!({}), 0),
        out("onMark", 500, json!({}), 0),
    ];
    let n = rendered(&tap, false, &v, &[]);
    assert_eq!(texts(&n), ["s2c onMark@?1", "s2c onMark@?2"]);
    assert_eq!(n.unordered, ["s2c onEffectResults@?2 effect=7"]);
}

#[test]
fn unordered_duplicates_are_kept() {
    let v = effects_unordered();
    let hit = out("onEffectResults", 8, json!({ "effect": 3 }), 0);
    let n = rendered(&[hit.clone(), hit], false, &v, &[]);
    assert_eq!(
        n.unordered,
        [
            "s2c onEffectResults@self effect=3",
            "s2c onEffectResults@self effect=3"
        ]
    );
}

/// A leading record without a ts_ms does not switch the cut off: it starts
/// at the first record that has one.
#[test]
fn the_late_bind_cut_starts_at_the_first_timestamped_record() {
    let v = resolve(None, None);
    let at = |ts: i64| out("onMark", 8, json!({}), ts);
    let mut untimed = at(0);
    untimed["ts_ms"] = Value::Null;
    let tap = [untimed, at(1000), at(1500), at(2000)];
    assert_eq!(rendered(&tap, true, &v, &[]).events.len(), 2);
}

/// Ignore runs first: an ignored message is neither routed nor labelled,
/// and transitions are read before routing takes events out of order.
#[test]
fn ignore_runs_first_and_transitions_precede_routing() {
    let mut v = resolve(None, None);
    v.ignore.push("onNoise".into());
    v.allow_unordered = vec!["onNoise".into(), "onStepUpdate".into()];
    let tap = [
        out("onNoise", 500, json!({}), 0),
        out("onMark", 600, json!({}), 0),
        out("onStepUpdate", 8, json!({ "StepID": 2113, "Status": 1 }), 0),
        out(
            "onMissionUpdate",
            8,
            json!({ "MissionID": 1360, "Status": 0 }),
            0,
        ),
    ];
    let n = rendered(&tap, false, &v, &[]);
    assert_eq!(
        texts(&n),
        [
            "s2c onMark@?1",
            "s2c onMissionUpdate@self MissionID=1360 Status=0"
        ]
    );
    assert_eq!(n.unordered, ["s2c onStepUpdate@self StepID=2113 Status=1"]);
    assert_eq!(n.transitions, ["step 2113 -> 1", "mission 1360 -> 0"]);
}

#[test]
fn entity_fields_are_labels_and_missing_fields_are_left_out() {
    let v = resolve(None, None);
    let dialog = out(
        "onDialogDisplay",
        8,
        json!({ "dialog_id": 1, "entity_id": 777 }),
        0,
    );
    let unknown = json!({ "dir": "out", "msg_name": "unknown", "method_index": 27,
                          "target_entity_id": null, "decoded": null, "ts_ms": 0 });
    let n = rendered(&[dialog, unknown], false, &v, &[]);
    assert_eq!(
        texts(&n),
        [
            "s2c onDialogDisplay@self dialog_id=1 entity_id=?1",
            "s2c unknown#27"
        ]
    );
}

#[test]
fn events_from_tap_keeps_the_contract_pair() {
    let mut v = resolve(None, None);
    v.allow_unordered = vec!["onB".into()];
    let tap = [out("onA", 8, json!({}), 0), out("onB", 8, json!({}), 0)];
    let mut labels = EntityLabels::new(8, None, None);
    let (events, unordered) = events_from_tap(&tap, false, &mut labels, &v, &[]);
    assert_eq!(
        events.iter().map(Event::render).collect::<Vec<_>>(),
        ["s2c onA@self"]
    );
    assert_eq!(unordered, ["s2c onB@self"]);
}

#[test]
fn windows_are_the_net_opened_and_closed_sets() {
    let v = resolve(None, None);
    let body = json!({ "samples": [
        { "after": "setup#0", "error": "no client" },
        { "after": "start", "windows": ["A", "Tutorial"] },
        { "after": "s1", "windows": ["A", "B", "C"] },
        { "after": "s2", "error": "timeout" },
        { "after": "end", "windows": ["B", "Tutorial2"] },
    ] });
    let (opened, closed) = windows(Some(&body), &v);
    assert_eq!(opened, ["B", "C", "Tutorial2"]);
    assert_eq!(closed, ["A", "C", "Tutorial"]);

    let mut quiet = v.clone();
    quiet.ignore_windows = vec!["Tutorial*".into()];
    let (opened, closed) = windows(Some(&body), &quiet);
    assert_eq!(opened, ["B", "C"]);
    assert_eq!(closed, ["A", "C"]);

    let failed = json!({ "samples": [{ "after": "start", "error": "x" }] });
    assert_eq!(windows(Some(&failed), &v), (vec![], vec![]));
    assert_eq!(windows(None, &v), (vec![], vec![]));

    // A sample with neither `error` nor `windows` failed; read as an empty
    // set it would close every window the row left open.
    let blank = json!({ "samples": [
        { "after": "start", "windows": ["A"] },
        { "after": "end" },
    ] });
    assert_eq!(windows(Some(&blank), &v), (vec![], vec![]));
}

#[test]
fn resolve_layers_defaults_section_and_row() {
    let plain = resolve(None, None);
    assert_eq!(plain.ignore, DEFAULT_IGNORE);
    assert_eq!(plain.late_bind_settle_ms, DEFAULT_LATE_BIND_SETTLE_MS);
    assert!(plain.allow_unordered.is_empty() && plain.key_args.is_empty());

    let section = GoldenSpec {
        ignore: vec!["onNoise".into(), "perfStats".into()],
        allow_unordered: vec!["onEffectResults".into()],
        collapse: vec!["onTimer*".into()],
        ignore_windows: vec!["Tutorial*".into()],
        key_args: BTreeMap::from([
            ("onA".into(), vec!["x".into()]),
            ("onB".into(), vec!["y".into()]),
        ]),
        late_bind_settle_ms: Some(2000),
        skip: false,
    };
    let row = GoldenSpec {
        // The section ignores onNoise; the row's allow_unordered naming it
        // passes validation, and ignore wins.
        allow_unordered: vec!["onNoise".into(), "onHeal".into(), "onEffectResults".into()],
        collapse: vec!["onTimer*".into(), "onTick".into()],
        key_args: BTreeMap::from([("onB".into(), vec!["z".into()])]),
        ..GoldenSpec::default()
    };

    let v = resolve(Some(&section), Some(&row));
    assert_eq!(
        v.ignore,
        [
            "avatarUpdate*",
            "perfStats",
            "requestEntityUpdate",
            "onNoise"
        ]
    );
    assert_eq!(v.allow_unordered, ["onEffectResults", "onHeal"]);
    assert_eq!(v.collapse, ["onTimer*", "onTick"]);
    assert_eq!(v.ignore_windows, ["Tutorial*"]);
    assert_eq!(
        v.key_args,
        BTreeMap::from([
            ("onA".to_string(), vec!["x".to_string()]),
            ("onB".to_string(), vec!["z".to_string()]),
        ])
    );
    assert_eq!(
        v.late_bind_settle_ms, 2000,
        "the section's, the row has none"
    );

    let row_settle = GoldenSpec {
        late_bind_settle_ms: Some(50),
        ..GoldenSpec::default()
    };
    assert_eq!(
        resolve(Some(&section), Some(&row_settle)).late_bind_settle_ms,
        50
    );
    assert_eq!(resolve(None, Some(&row_settle)).late_bind_settle_ms, 50);

    // An ignored message is dropped even when a glob also routes it.
    let tap = [out("onNoise", 8, json!({}), 0)];
    let n = rendered(&tap, false, &v, &[]);
    assert!(n.events.is_empty() && n.unordered.is_empty());
}
