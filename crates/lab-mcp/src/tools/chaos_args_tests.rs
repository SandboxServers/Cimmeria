//! CH-05 review: the chaos tools' argument parse. A malformed or partly
//! ignored request is a `bad chaos knob:` refusal with an audit row, never an
//! rmcp protocol error and never a silent drop.

use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use serde_json::{json, Value};

use cimmeria_admin_api::ws::broadcast_layer::LogBuffer;
use cimmeria_services::base::lab_chaos::FlushChaos;
use cimmeria_services::orchestrator::Orchestrator;

use super::chaos_args::{
    audit_args, built_label, check_seed_used, flush_from, net_from, parse, ClearArgs, ClearCall,
    FlushArgs, NetArgs, SetArgs, SetCall, ShiftArg,
};
use super::LabTools;
use crate::state::LabState;

fn tools() -> LabTools {
    let orchestrator = Arc::new(Orchestrator::new(Default::default()));
    LabTools::new(LabState::new(orchestrator, LogBuffer::new()))
}

fn refusal_of(raw: Value) -> String {
    parse::<SetArgs>(raw).expect_err("refused")
}

/// The call reaches the tool whatever its shape (rmcp deserializes the raw
/// wrapper from any JSON), and the reply is the parse's refusal. The test
/// binary installs no registry, so a valid call would read "lab chaos is
/// off": these replies prove the parse ran first and refused.
#[tokio::test]
async fn a_malformed_call_reaches_the_tool_and_is_refused_by_field() {
    let tools = tools();
    for (raw, field) in [
        (
            json!({ "entity_id": -1, "flush": { "mode": "pack", "records": 4 } }),
            "entity_id",
        ),
        (json!({ "entity_id": 4_294_967_296_u64 }), "entity_id"),
        (
            json!({ "entity_id": 7, "net": { "latency": -5 } }),
            "net.latency",
        ),
        (
            json!({ "entity_id": 7, "flush": { "mode": "pack", "records": 2.5 } }),
            "flush.records",
        ),
        (
            json!({ "entity_id": 7, "flush": { "mode": "shift", "shift": true } }),
            "flush.shift",
        ),
        (json!({ "entity_id": 7, "ttl_s": "60" }), "ttl_s"),
    ] {
        // rmcp hands the tool whatever it deserialized into the wrapper.
        let call: SetCall = serde_json::from_value(raw.clone()).expect("any JSON is a call");
        let result = tools
            .server_chaos_set(Parameters(call))
            .await
            .expect("a refusal is a successful call");
        assert_ne!(result.is_error, Some(true));
        let text = &result.content[0].as_text().expect("JSON text").text;
        let reply: Value = serde_json::from_str(text).expect("the reply parses");
        let refused = reply["refused"].as_str().unwrap_or_default();
        assert!(
            refused.starts_with(&format!("bad chaos knob: {field}: ")),
            "{raw} -> {reply}"
        );
    }

    let call: ClearCall = serde_json::from_value(json!({ "entity_id": "all" })).expect("a call");
    let result = tools
        .server_chaos_clear(Parameters(call))
        .await
        .expect("ok");
    let text = &result.content[0].as_text().expect("JSON text").text;
    assert!(text.contains("bad chaos knob: entity_id: "), "{text}");
}

/// The raw wrappers still advertise the real arguments' schema.
#[test]
fn the_tools_advertise_the_argument_schemas() {
    let tools = tools();
    let props = |name: &str| {
        let tool = tools.tool_router.get(name).expect("listed");
        let props = tool.input_schema["properties"]
            .as_object()
            .expect("an object schema with properties");
        let mut keys: Vec<String> = props.keys().cloned().collect();
        keys.sort();
        keys
    };
    assert_eq!(
        props("server_chaos_set"),
        [
            "account",
            "entity_id",
            "flush",
            "label",
            "net",
            "seed",
            "ttl_s"
        ]
    );
    assert_eq!(props("server_chaos_clear"), ["account", "entity_id"]);
}

#[test]
fn unknown_fields_are_refused_at_every_level() {
    let flush = json!({ "mode": "pack", "records": 4 });
    for (raw, field) in [
        (json!({ "entity_id": 7, "flush": flush, "ttl": 60 }), "ttl"),
        (
            json!({ "entity_id": 7, "net": { "latency_ms": 100 } }),
            "latency_ms",
        ),
        (
            json!({ "entity_id": 7, "flush": { "mode": "pack", "records": 4, "count": 2 } }),
            "count",
        ),
    ] {
        let refused = refusal_of(raw.clone());
        assert!(
            refused.starts_with("bad chaos knob: ") && refused.contains(&format!("`{field}`")),
            "{raw} -> {refused}"
        );
    }
    assert!(parse::<ClearArgs>(json!({ "entity": 7 })).is_err());
    assert_eq!(
        parse::<ClearArgs>(Value::Null).map(|a| a.entity_id),
        Ok(None),
        "absent arguments are an empty call"
    );
}

#[test]
fn an_empty_net_profile_is_refused() {
    for net in [
        NetArgs::default(),
        NetArgs {
            latency: Some(0),
            loss: Some(0.0),
            ..NetArgs::default()
        },
    ] {
        assert_eq!(
            net_from(&net, Some(7)),
            Err("bad chaos knob: net sets no knob".to_string())
        );
    }
}

#[test]
fn a_flush_knob_the_mode_does_not_use_is_refused() {
    let flush = |mode: &str, delay_ms, records, shift| FlushArgs {
        mode: mode.to_string(),
        delay_ms,
        records,
        shift,
    };
    for (args, knob) in [
        (flush("pack", Some(100), Some(4), None), "delay_ms"),
        (flush("delay", Some(100), Some(4), None), "records"),
        (
            flush("delay", Some(100), None, Some(ShiftArg::Bytes(4))),
            "shift",
        ),
        (flush("reorder", None, Some(4), None), "records"),
        (
            flush("shift", Some(5), None, Some(ShiftArg::Bytes(4))),
            "delay_ms",
        ),
    ] {
        assert_eq!(
            flush_from(&args, None),
            Err(format!(
                "bad chaos knob: flush {} does not use {knob}",
                args.mode
            ))
        );
    }
    assert_eq!(
        flush_from(&flush("burst", Some(1), None, None), None),
        Err("bad chaos knob: unknown flush mode \"burst\"".to_string()),
        "an unknown mode is named, not blamed on its knobs"
    );
}

#[test]
fn a_seed_nothing_reads_is_refused() {
    let net = net_from(
        &NetArgs {
            latency: Some(100),
            ..NetArgs::default()
        },
        Some(7),
    )
    .expect("valid");
    let unseeded = [
        FlushChaos::Pack { records: 4 },
        FlushChaos::Delay { ms: 100 },
        FlushChaos::Shift { len: 4 },
    ];
    for flush in &unseeded {
        assert_eq!(
            check_seed_used(Some(7), None, Some(flush)),
            Err("bad chaos knob: seed is not used by this profile".to_string())
        );
        assert_eq!(check_seed_used(Some(7), Some(&net), Some(flush)), Ok(()));
        assert_eq!(check_seed_used(None, None, Some(flush)), Ok(()));
    }
    for seeded in [
        FlushChaos::Reorder { seed: 7 },
        FlushChaos::ShiftSweep { seed: 7 },
    ] {
        assert_eq!(check_seed_used(Some(7), None, Some(&seeded)), Ok(()));
    }
}

#[test]
fn net_percentages_are_whole_tenths_or_refused() {
    let net = net_from(
        &NetArgs {
            latency: Some(120),
            loss: Some(1.2),
            dup: Some(12.3),
            ..NetArgs::default()
        },
        Some(7),
    )
    .expect("valid knobs");
    assert_eq!(net.latency_ms, 120);
    assert_eq!(net.loss_per_thousand, 12);
    assert_eq!(net.dup_per_thousand, 123);
    assert_eq!(net.seed, 7);
    assert_eq!(built_label(Some(&net), None), "net seed=7");

    // 0.04 would round to 0 (no loss at all), 0.05 to 1: both refused.
    for (loss, why) in [
        (0.04, "net loss 0.04 has more than one decimal"),
        (0.05, "net loss 0.05 has more than one decimal"),
        (-1.0, "net loss is negative"),
    ] {
        let args = NetArgs {
            latency: Some(100),
            loss: Some(loss),
            ..NetArgs::default()
        };
        assert_eq!(net_from(&args, None), Err(format!("bad chaos knob: {why}")));
    }
}

#[test]
fn flush_modes_resolve_and_a_missing_knob_is_refused() {
    let flush = |mode: &str, shift: Option<ShiftArg>| FlushArgs {
        mode: mode.to_string(),
        delay_ms: None,
        records: None,
        shift,
    };
    assert_eq!(
        flush_from(
            &flush("shift", Some(ShiftArg::Word("sweep".into()))),
            Some(9)
        ),
        Ok(FlushChaos::ShiftSweep { seed: 9 })
    );
    assert_eq!(
        flush_from(&flush("shift", Some(ShiftArg::Bytes(4))), None),
        Ok(FlushChaos::Shift { len: 4 })
    );
    assert_eq!(
        flush_from(&flush("reorder", None), None),
        Ok(FlushChaos::Reorder { seed: 0 })
    );
    for (mode, shift) in [
        ("shift", Some(ShiftArg::Word("wide".into()))),
        ("shift", None),
        ("delay", None),
        ("pack", None),
    ] {
        assert!(
            flush_from(&flush(mode, shift), None).is_err_and(|e| e.starts_with("bad chaos knob: ")),
            "{mode} must be refused"
        );
    }
    let reorder = FlushChaos::Reorder { seed: 3 };
    assert_eq!(built_label(None, Some(&reorder)), "flush:reorder seed=3");
}

/// A label set would refuse never reaches the audit row as text.
#[test]
fn the_audit_row_never_carries_a_refused_label() {
    let long = "x".repeat(500);
    let audited = audit_args(&json!({ "entity_id": 7, "label": long }));
    assert_eq!(audited["label"], "<label: 500 chars, refused>");
    assert_eq!(audited["entity_id"], 7);

    let forged = audit_args(&json!({ "label": "pack\nERROR forged row" }));
    assert_eq!(forged["label"], "<label: 21 chars, refused>");

    let fine = json!({ "entity_id": 7, "label": "flush:pack seed=7" });
    assert_eq!(audit_args(&fine), fine, "an acceptable label is kept");
}

#[test]
fn a_well_formed_call_parses() {
    let args: SetArgs = parse(json!({
        "entity_id": 7,
        "net": { "latency": 100, "loss": 1.5 },
        "flush": { "mode": "shift", "shift": "sweep" },
        "seed": 3,
        "ttl_s": 60,
        "label": "net+flush"
    }))
    .expect("valid");
    assert_eq!(args.entity_id, Some(7));
    assert_eq!(args.account, None);
    assert_eq!(args.seed, Some(3));
    assert!(matches!(
        args.flush.as_ref().and_then(|f| f.shift.as_ref()),
        Some(ShiftArg::Word(w)) if w == "sweep"
    ));
}
