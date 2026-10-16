use serde_json::{json, Value};

use super::{assemble, assemble_evidence, Evidence, Stage};
use crate::supervisor::events::store::StoredEvent;

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
fn interleaved_clicks_join_by_identity_not_adjacent_time() {
    let rows = [
        click(1, "E:1", 1_000),
        click(2, "E:2", 1_001),
        row(
            3,
            "ability.sent",
            1_002,
            json!({"click_id":"E:2","send_id":22}),
        ),
        row(
            4,
            "ability.sent",
            1_003,
            json!({"click_id":"E:1","send_id":11}),
        ),
        row(5, "ability.recv", 1_004, json!({"send_id":22})),
        row(6, "ability.recv", 1_005, json!({"send_id":11})),
    ];
    let trace = assemble("E:1", &rows, &[], false);
    assert_eq!(
        stage(&trace, Stage::ClientSend)
            .evidence
            .as_ref()
            .unwrap()
            .seq,
        4
    );
    assert_eq!(
        stage(&trace, Stage::ClientReceive)
            .evidence
            .as_ref()
            .unwrap()
            .seq,
        6
    );
    assert_eq!(stage(&trace, Stage::ServerReceive).status, "unknown");
    assert_eq!(trace.first_unobserved_stage, Some(Stage::ServerReceive));
    assert!(trace.click_to_visible_ms.is_none());
}

#[test]
fn two_matching_sends_are_ambiguous_even_when_one_is_closer() {
    let rows = [
        click(1, "E:1", 1_000),
        row(
            2,
            "ability.sent",
            1_001,
            json!({"click_id":"E:1","send_id":11}),
        ),
        row(
            3,
            "ability.sent",
            1_002,
            json!({"click_id":"E:1","send_id":12}),
        ),
    ];
    let trace = assemble("E:1", &rows, &[], false);
    let send = stage(&trace, Stage::ClientSend);
    assert_eq!(send.status, "ambiguous");
    assert_eq!(send.competing_candidates, 2);
    assert!(send.evidence.is_none());
    assert_eq!(trace.first_unobserved_stage, Some(Stage::ClientSend));
}

#[test]
fn missing_client_apply_is_gap_even_if_handler_shown() {
    let rows = [
        click(1, "E:1", 1_000),
        row(
            2,
            "ability.sent",
            1_010,
            json!({"click_id":"E:1","send_id":11}),
        ),
        row(3, "ability.recv", 1_020, json!({"send_id":11,"cast_id":80})),
        row(4, "ability.shown", 1_030, json!({"cast_id":80})),
    ];
    let trace = assemble("E:1", &rows, &[], false);
    assert_eq!(stage(&trace, Stage::ClientApply).status, "unknown");
    assert_eq!(stage(&trace, Stage::VisibleResult).status, "observed");
    assert_eq!(trace.first_unobserved_stage, Some(Stage::ServerReceive));
    assert!(
        trace.click_to_visible_ms.is_none(),
        "host click clock is unbounded"
    );
}

fn evidence(seq: u64, stage: Stage, time: i64, fields: Value) -> Evidence {
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
fn complete_bounded_chain_exports_latency_interval_and_raw_evidence() {
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
        evidence(8, Stage::VisibleResult, 140, json!({"cast_id":80})),
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
