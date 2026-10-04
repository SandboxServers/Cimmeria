//! The classification table: what is must-keep, what is summarized, and
//! the precedence of level and outcome over the target row.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::governor::classify::{
    classify, is_non_happy, rule_for, Class, KeepReason, Pattern, RULES,
};

fn fields(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect()
}

fn class_of(target: &str) -> Class {
    classify(target, "debug", &BTreeMap::new()).class
}

/// Every target the owner asked to keep at full fidelity, by name.
#[test]
fn the_must_keep_targets_are_never_summarized() {
    let cases: &[(&str, KeepReason)] = &[
        ("client.entity.create", KeepReason::EntityLifecycle),
        ("client.entity.enter", KeepReason::EntityLifecycle),
        ("client.entity.entered_world", KeepReason::EntityLifecycle),
        ("client.entity.leave", KeepReason::EntityLifecycle),
        ("client.entity.destroyed", KeepReason::EntityLifecycle),
        (
            "client.entity.appearance_request",
            KeepReason::EntityLifecycle,
        ),
        (
            "client.entity.appearance_request.scheduled",
            KeepReason::EntityLifecycle,
        ),
        ("client.entity.queue_replay", KeepReason::EntityLifecycle),
        ("client.mercury.error", KeepReason::MercuryAnomaly),
        ("client.mercury.rx_gap", KeepReason::MercuryAnomaly),
        ("client.mercury.fragment", KeepReason::MercuryAnomaly),
        ("client.mercury.bundle", KeepReason::MercuryAnomaly),
        (
            "client.mercury.request_misparse",
            KeepReason::MercuryAnomaly,
        ),
        ("client.mercury.unpack_fault", KeepReason::MercuryAnomaly),
        ("client.dispatch.method_dropped", KeepReason::MercuryAnomaly),
        ("client.hooks.fingerprint", KeepReason::HookInstall),
        ("client.hooks.capabilities", KeepReason::HookInstall),
        ("client.hooks.inline.installed", KeepReason::HookInstall),
        ("client.hooks.iat.install_complete", KeepReason::HookInstall),
        ("client.dll.attached", KeepReason::SessionBoot),
        ("client.cme.catalog_done", KeepReason::SessionBoot),
        ("client.lua.error", KeepReason::Failure),
        ("client.os.exception", KeepReason::Failure),
        ("client.ue3.assert", KeepReason::Failure),
        ("client.ue3.fatal_error", KeepReason::Failure),
        ("client.engine.load_failed", KeepReason::Failure),
        ("client.engine.hitch", KeepReason::Failure),
        ("client.telemetry.rollup", KeepReason::SelfReport),
        ("client.telemetry.health", KeepReason::SelfReport),
        ("client.ability.press", KeepReason::AbilityTrace),
        ("client.ability.press_dropped", KeepReason::AbilityTrace),
        ("client.ability.sent", KeepReason::AbilityTrace),
        ("client.ability.sent_seq", KeepReason::AbilityTrace),
        ("client.ability.recv", KeepReason::AbilityTrace),
        ("client.ability.applied", KeepReason::AbilityTrace),
        ("client.ability.shown", KeepReason::AbilityTrace),
    ];
    for (target, reason) in cases {
        assert_eq!(class_of(target), Class::MustKeep(*reason), "{target}");
    }
}

/// The streams the SigNoz measurement named as high-rate.
#[test]
fn the_measured_hot_streams_are_summarized() {
    for target in [
        "client.engine.sequence_tick",
        "client.lua.pcall",
        "client.lua.call",
        "client.engine.async_archive_serialize",
        "client.engine.static_load_object",
        "client.engine.tick",
        "client.engine.actor_tick",
        "client.engine.update_level_streaming",
    ] {
        assert_eq!(class_of(target), Class::Hot, "{target}");
    }
}

#[test]
fn per_entity_streams_keep_a_first_k() {
    for target in [
        "client.cme.event",
        "client.mercury.entity_method",
        "client.mercury.entity_property",
        "client.net.out",
        "client.sequence.dropped",
    ] {
        assert!(
            matches!(class_of(target), Class::PerEntity { first_k } if first_k > 0),
            "{target}"
        );
    }
}

/// No row means "budgeted", never "hot": an unknown new hook is forwarded
/// until it proves to be high-rate.
#[test]
fn an_unknown_target_is_budgeted() {
    assert_eq!(class_of("client.brand_new.thing"), Class::Budgeted);
    assert_eq!(class_of("client.ui.cegui_log"), Class::Budgeted);
    assert_eq!(class_of("client.mercury.packet_in"), Class::Budgeted);
}

/// Level wins over the table: a hot stream that reports a problem is kept.
#[test]
fn warn_and_error_are_must_keep_whatever_the_target() {
    for level in ["warn", "error", "WARN"] {
        let v = classify("client.engine.sequence_tick", level, &BTreeMap::new());
        assert_eq!(v.class, Class::MustKeep(KeepReason::Level), "{level}");
    }
    let v = classify("client.engine.sequence_tick", "info", &BTreeMap::new());
    assert_eq!(v.class, Class::Hot);
}

/// A failure reported in the fields wins over the table too.
#[test]
fn a_non_happy_outcome_is_must_keep() {
    let failing: &[&[(&str, Value)]] = &[
        &[("ok", json!(false))],
        &[("success", json!(false))],
        &[("failed", json!(true))],
        &[("error", json!("boom"))],
        &[("outcome", json!("rejected"))],
        &[("result", json!("TIMEOUT"))],
    ];
    for f in failing {
        let f = fields(f);
        assert!(is_non_happy(&f), "{f:?}");
        let v = classify("client.lua.pcall", "debug", &f);
        assert_eq!(v.class, Class::MustKeep(KeepReason::NonHappyOutcome));
    }
    let happy: &[&[(&str, Value)]] = &[
        &[("ok", json!(true))],
        &[("error", Value::Null)],
        &[("outcome", json!("entered_world"))],
        &[("nargs", json!(2))],
    ];
    for f in happy {
        assert!(!is_non_happy(&fields(f)), "{f:?}");
    }
}

/// First match wins, so an earlier row must never swallow a later one:
/// every row's own pattern resolves to that row.
#[test]
fn no_rule_is_shadowed_by_an_earlier_one() {
    for (i, rule) in RULES.iter().enumerate() {
        let probe = match rule.pattern {
            Pattern::Exact(t) => t.to_string(),
            Pattern::Prefix(p) => format!("{p}probe"),
        };
        // Compare by position, not address: `RULES` is a `const`, and two
        // uses of a const slice need not share one allocation (the i686
        // test build gave `rule_for` a different copy).
        let hit = RULES
            .iter()
            .position(|r| r.pattern.matches(&probe))
            .expect("a row");
        assert_eq!(hit, i, "row {i} ({probe}) is shadowed by row {hit}");
        assert!(rule_for(&probe).is_some());
    }
}
