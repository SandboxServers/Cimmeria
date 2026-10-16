use serde_json::{json, Value};

use super::{assemble, assemble_evidence, Evidence, Stage};
use crate::supervisor::events::{
    ingest_bridge, ingest_rings,
    lua_rings::RingRead,
    store::{StoreInner, StoredEvent},
};

fn click(seq: u64, id: &str, at_ms: i64) -> StoredEvent {
    let (epoch, number) = id.split_once(':').unwrap();
    StoredEvent {
        seq,
        kind: "ui.click".into(),
        ts_ms: at_ms,
        fields: json!({
            "event_id": id, "click_epoch": epoch,
            "click_seq": number.parse::<u64>().unwrap(),
            "client_pid": 7, "window": "ActionBar", "button": "Slot1",
            "selected_entity_id": 90
        }),
    }
}

fn row(seq: u64, kind: &str, at_ms: i64, fields: Value) -> StoredEvent {
    StoredEvent {
        seq,
        kind: kind.into(),
        ts_ms: at_ms,
        fields,
    }
}

fn stage(trace: &super::CausalTrace, wanted: Stage) -> &super::StageResult {
    trace.stages.iter().find(|row| row.stage == wanted).unwrap()
}

#[test]
fn two_real_shape_clicks_and_interleaved_sends_remain_unbound() {
    let mut store = StoreInner::with_cap(32);
    ingest_bridge(
        &mut store,
        &json!({"events":[
            {"kind":"ability.sent","ts_ms":1000,"fields":{"send_id":22,"press_id":2,"method":"useAbility"}},
            {"kind":"ability.sent","ts_ms":1001,"fields":{"send_id":11,"press_id":1,"method":"useAbility"}},
            {"kind":"ability.recv","ts_ms":1020,"fields":{"send_id":22,"cast_id":80}},
            {"kind":"ability.recv","ts_ms":1021,"fields":{"send_id":11,"cast_id":81}}
        ],"dropped":0}),
    );
    let ring = RingRead {
        epoch: "E".into(),
        click: vec![
            (1, click(1, "E:1", 1_000).fields),
            (2, click(2, "E:2", 1_001).fields),
        ],
        ..Default::default()
    };
    ingest_rings(&mut store, &ring, Some(7));
    let rows = store.since(0, 32).events;
    assert_eq!(rows[0].kind, "ability.sent");
    assert_eq!(rows[4].kind, "ui.click");
    let trace = assemble("E:1", &rows, &[], false);
    assert_eq!(stage(&trace, Stage::ClientSend).status, "unknown");
    assert_eq!(stage(&trace, Stage::ClientReceive).status, "unknown");
    assert_eq!(trace.first_unobserved_stage, Some(Stage::ClientSend));
    assert_eq!(trace.unjoined_context_events.len(), 4);
    assert!(trace.click_to_visible_ms.is_none());
}

#[test]
fn future_click_identity_contract_refuses_two_matching_sends() {
    let rows = [
        evidence(1, Stage::UiEvent, 1_000, json!({"event_id":"E:1"})),
        evidence(
            2,
            Stage::ClientSend,
            1_001,
            json!({"click_id":"E:1","send_id":11}),
        ),
        evidence(
            3,
            Stage::ClientSend,
            1_002,
            json!({"click_id":"E:1","send_id":12}),
        ),
    ];
    let trace = assemble_evidence("E:1", &rows, false);
    let send = stage(&trace, Stage::ClientSend);
    assert_eq!(send.status, "ambiguous");
    assert_eq!(send.competing_candidates, 2);
    assert!(send.evidence.is_none());
    assert_eq!(trace.first_unobserved_stage, Some(Stage::ClientSend));
}

#[test]
fn actual_receive_apply_and_shown_shapes_do_not_invent_missing_join_ids() {
    let rows = [
        click(1, "E:1", 1_000),
        row(
            2,
            "ability.sent",
            1_010,
            json!({"send_id":11,"press_id":1,"method":"useAbility"}),
        ),
        row(3, "ability.recv", 1_020, json!({"send_id":11,"cast_id":80})),
        row(
            4,
            "ability.applied",
            1_025,
            json!({"kind":"effect_bar_add","entity_id":7,"recv_to_applied_ms":5}),
        ),
        row(
            5,
            "ability.shown",
            1_030,
            json!({"kind":"combat_text","handler":"SCTMod.onUnitCombat","status":"ok"}),
        ),
    ];
    let trace = assemble("E:1", &rows, &[], false);
    assert_eq!(stage(&trace, Stage::ClientSend).status, "unknown");
    assert_eq!(stage(&trace, Stage::ClientReceive).status, "unknown");
    assert_eq!(stage(&trace, Stage::ClientApply).status, "unknown");
    assert_eq!(stage(&trace, Stage::VisibleResult).status, "unknown");
    assert_eq!(trace.first_unobserved_stage, Some(Stage::ClientSend));
    assert!(
        trace.click_to_visible_ms.is_none(),
        "host click clock is unbounded"
    );
}

fn evidence(seq: u64, stage: Stage, time: i64, fields: Value) -> Evidence {
    let mut fields = fields;
    fields["server_session_id"] = json!("S");
    Evidence {
        seq,
        stage,
        source: "fixture".into(),
        kind: format!("{stage:?}"),
        at_ms: time,
        clock_domain: "bounded_projected_clock".into(),
        uncertainty_ms: Some(2),
        fields,
    }
}

#[test]
fn future_bound_identity_contract_exports_complete_latency_interval() {
    let rows = [
        evidence(1, Stage::UiEvent, 100, json!({"event_id":"E:1"})),
        evidence(
            2,
            Stage::ClientSend,
            105,
            json!({"click_id":"E:1","send_id":7,"mercury_seq_first":15,"mercury_seq_last":16}),
        ),
        evidence(
            3,
            Stage::ServerReceive,
            110,
            json!({"send_id":7,"mercury_seq":16,"cast_id":80}),
        ),
        evidence(4, Stage::ServerState, 115, json!({"cast_id":80})),
        evidence(5, Stage::ServerSend, 120, json!({"cast_id":80})),
        evidence(6, Stage::ClientReceive, 125, json!({"cast_id":80})),
        evidence(7, Stage::ClientApply, 130, json!({"cast_id":80})),
        evidence(
            8,
            Stage::VisibleResult,
            140,
            json!({"cast_id":80,"status":"ok"}),
        ),
    ];
    let trace = assemble_evidence("E:1", &rows, false);
    assert_eq!(trace.first_unobserved_stage, None);
    let interval = trace.click_to_visible_ms.unwrap();
    assert_eq!(
        (
            interval.lower_ms,
            interval.upper_ms,
            interval.uncertainty_ms
        ),
        (36, 44, 4)
    );
    let exported = serde_json::to_value(&trace.stages).unwrap();
    assert_eq!(exported[2]["evidence"]["fields"]["mercury_seq"], 16);
}

#[test]
fn method_and_entity_alone_do_not_link_repeated_actions() {
    let rows = [
        click(1, "E:1", 1_000),
        row(
            2,
            "ability.sent",
            1_001,
            json!({"method":"useAbility","entity_id":90}),
        ),
    ];
    let trace = assemble("E:1", &rows, &[], false);
    assert_eq!(stage(&trace, Stage::ClientSend).status, "unknown");
}

#[test]
fn cme_and_mercury_context_are_exported_without_claiming_apply() {
    let rows = [
        click(1, "E:1", 1_000),
        row(2, "cme.event", 1_001, json!({"name":"Event_NetIn_Example"})),
        row(3, "mercury.socket_recv", 1_002, json!({"bytes":42})),
    ];
    let trace = assemble("E:1", &rows, &[], false);
    assert_eq!(stage(&trace, Stage::ClientApply).status, "unknown");
    assert_eq!(trace.unjoined_context_events.len(), 2);
    assert_eq!(trace.unjoined_context_events[0]["kind"], "cme.event");
    assert_eq!(
        trace.unjoined_context_events[1]["kind"],
        "mercury.socket_recv"
    );
}

#[test]
fn conflicting_send_id_cannot_be_rescued_by_same_cast_id() {
    let rows = [
        evidence(1, Stage::UiEvent, 100, json!({"event_id":"E:1"})),
        evidence(
            2,
            Stage::ClientSend,
            105,
            json!({"click_id":"E:1","send_id":7,"cast_id":80}),
        ),
        evidence(
            3,
            Stage::ServerReceive,
            110,
            json!({"send_id":8,"cast_id":80}),
        ),
    ];
    let trace = assemble_evidence("E:1", &rows, false);
    assert_eq!(stage(&trace, Stage::ServerReceive).status, "unknown");
}

#[test]
fn wrapped_mercury_range_links_only_its_member() {
    let rows = [
        evidence(1, Stage::UiEvent, 100, json!({"event_id":"E:1"})),
        evidence(
            2,
            Stage::ClientSend,
            105,
            json!({"click_id":"E:1","server_session_id":"S","mercury_seq_first":268_435_455u64,"mercury_seq_last":1}),
        ),
        evidence(
            3,
            Stage::ServerReceive,
            110,
            json!({"server_session_id":"S","mercury_seq":0}),
        ),
        evidence(
            4,
            Stage::ServerReceive,
            111,
            json!({"server_session_id":"S","mercury_seq":2}),
        ),
    ];
    let trace = assemble_evidence("E:1", &rows, false);
    assert_eq!(
        stage(&trace, Stage::ServerReceive)
            .evidence
            .as_ref()
            .unwrap()
            .seq,
        3
    );
}

#[test]
fn matching_send_id_across_unbounded_clocks_is_not_observed() {
    let mut rows = [
        evidence(1, Stage::UiEvent, 100, json!({"event_id":"E:1"})),
        evidence(
            2,
            Stage::ClientSend,
            105,
            json!({"click_id":"E:1","send_id":7}),
        ),
        evidence(3, Stage::ServerReceive, 110, json!({"send_id":7})),
    ];
    rows[2].clock_domain = "server_wall".into();
    let trace = assemble_evidence("E:1", &rows, false);
    assert_eq!(stage(&trace, Stage::ServerReceive).status, "unknown");
}

#[test]
fn repeated_send_id_after_window_with_unknown_bound_is_not_observed() {
    let mut rows = [
        evidence(1, Stage::UiEvent, 100, json!({"event_id":"E:1"})),
        evidence(
            2,
            Stage::ClientSend,
            105,
            json!({"click_id":"E:1","send_id":7}),
        ),
        evidence(3, Stage::ServerReceive, 31_106, json!({"send_id":7})),
    ];
    rows[2].uncertainty_ms = None;
    let trace = assemble_evidence("E:1", &rows, false);
    assert_eq!(stage(&trace, Stage::ClientSend).status, "observed");
    assert_eq!(stage(&trace, Stage::ServerReceive).status, "unknown");
    assert!(stage(&trace, Stage::ServerReceive).evidence.is_none());
}

#[test]
fn tail_click_exports_nearest_unjoined_context() {
    let mut rows: Vec<_> = (1..=100)
        .map(|seq| row(seq, "cme.event", seq as i64, json!({"name":"prior"})))
        .collect();
    rows.push(click(101, "E:1", 101));
    rows.extend((102..=104).map(|seq| {
        row(
            seq,
            "mercury.socket_recv",
            seq as i64,
            json!({"name":"after"}),
        )
    }));
    let trace = assemble("E:1", &rows, &[], false);
    assert_eq!(trace.unjoined_context_events.len(), 64);
    assert_eq!(trace.unjoined_context_events[0]["seq"], 40);
    assert_eq!(trace.unjoined_context_events[63]["seq"], 104);
    assert!(trace
        .unjoined_context_events
        .iter()
        .all(|row| row["seq"].as_u64().is_some_and(|seq| seq >= 40)));
    assert_eq!(stage(&trace, Stage::ClientSend).status, "unknown");
}

#[test]
fn future_click_id_can_join_a_bridge_row_stored_before_lua_pump() {
    let rows = [
        row(
            1,
            "ability.sent",
            900,
            json!({"click_id":"E:1","send_id":7}),
        ),
        click(2, "E:1", 1_000),
    ];
    let trace = assemble("E:1", &rows, &[], false);
    assert_eq!(stage(&trace, Stage::ClientSend).status, "observed");
    assert_eq!(
        stage(&trace, Stage::ClientSend)
            .evidence
            .as_ref()
            .unwrap()
            .seq,
        1
    );
}

#[test]
fn relogged_same_cast_id_cannot_claim_client_apply() {
    let mut rows = [
        evidence(1, Stage::UiEvent, 100, json!({"event_id":"E:1"})),
        evidence(
            2,
            Stage::ClientSend,
            105,
            json!({"click_id":"E:1","send_id":7}),
        ),
        evidence(
            3,
            Stage::ClientReceive,
            120,
            json!({"send_id":7,"cast_id":80}),
        ),
        evidence(4, Stage::ClientApply, 125, json!({"cast_id":80})),
    ];
    rows[3].fields["server_session_id"] = json!("S2");
    let trace = assemble_evidence("E:1", &rows, false);
    assert_eq!(stage(&trace, Stage::ClientReceive).status, "observed");
    assert_eq!(stage(&trace, Stage::ClientApply).status, "unknown");
}

#[test]
fn failed_shown_handler_is_failure_evidence_not_visible_result() {
    let rows = [
        evidence(1, Stage::UiEvent, 100, json!({"event_id":"E:1"})),
        evidence(
            2,
            Stage::ClientSend,
            105,
            json!({"click_id":"E:1","send_id":7}),
        ),
        evidence(
            3,
            Stage::ClientReceive,
            120,
            json!({"send_id":7,"cast_id":80}),
        ),
        evidence(
            4,
            Stage::VisibleResult,
            125,
            json!({"cast_id":80,"kind":"combat_text","status":"failed"}),
        ),
    ];
    let trace = assemble_evidence("E:1", &rows, false);
    let result = stage(&trace, Stage::VisibleResult);
    assert_eq!(result.status, "unknown");
    assert_eq!(result.reason, Some("visible_handler_failed_or_unverified"));
    assert_eq!(result.evidence.as_ref().unwrap().fields["status"], "failed");
    assert!(trace.click_to_visible_ms.is_none());
}
