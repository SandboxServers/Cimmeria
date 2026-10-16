//! Regression guards for the workbench's typed authority boundary.

use std::collections::BTreeMap;

use serde_json::json;

use super::commands::*;

fn descriptor() -> CommandDescriptor {
    CommandDescriptor {
        id: ".probe".into(),
        channel: CommandChannel::Dot,
        category: "test".into(),
        aliases: vec![],
        summary: "Inspect a selected entity".into(),
        example: None,
        availability: Availability::Runnable,
        disposition_reason: None,
        manual_pending: false,
        args: vec![ArgDescriptor {
            name: "count".into(),
            kind: ArgKind::Integer,
            required: true,
            default: None,
            range: Some(ArgRange {
                min: Some(1.0),
                max: Some(5.0),
            }),
            choices: vec![],
            help: "Number of observations".into(),
            sensitive: false,
        }],
        target: TargetPolicy {
            kind: TargetKind::Mob,
            selection_rule: SelectionRule::Required,
        },
        impact: Impact {
            kind: ImpactKind::ReadOnly,
            summary: "Inspect".into(),
            preconditions: vec![],
        },
        verify: VerifyRecipe {
            read_tools: vec!["server_sessions".into()],
            comparison_fields: vec!["entity_id".into()],
        },
        compensation: Compensation::None,
        metadata_version: COMMAND_METADATA_VERSION,
    }
}

fn action() -> CommandAction {
    CommandAction {
        descriptor_id: ".probe".into(),
        instance: "lab-a".into(),
        actor_entity_id: 10,
        selected_entity_id: Some(20),
        typed_args: BTreeMap::from([("count".into(), json!(2))]),
        idempotency_key: "request-1".into(),
    }
}

fn actor() -> ActorSnapshot {
    ActorSnapshot {
        instance: "lab-a".into(),
        entity_id: 10,
        is_gm: true,
        selected_entity_id: Some(20),
        selected_target_kind: Some(TargetKind::Mob),
        argument_target_entity_id: None,
        argument_target_kind: None,
    }
}

#[test]
fn typed_action_refuses_non_gm_even_with_valid_args() {
    let mut snapshot = actor();
    snapshot.is_gm = false;
    assert_eq!(
        validate_command(&action(), &descriptor(), &snapshot).unwrap_err(),
        Refusal::NotGm
    );
}

#[test]
fn typed_action_refuses_stale_selection_and_wrong_target_kind() {
    let mut snapshot = actor();
    snapshot.selected_entity_id = Some(21);
    assert_eq!(
        validate_command(&action(), &descriptor(), &snapshot).unwrap_err(),
        Refusal::StaleTarget
    );
    snapshot.selected_entity_id = Some(20);
    snapshot.selected_target_kind = Some(TargetKind::Player);
    assert_eq!(
        validate_command(&action(), &descriptor(), &snapshot).unwrap_err(),
        Refusal::WrongTargetKind
    );
}

#[test]
fn entity_id_target_requires_fresh_server_resolution() {
    let mut command = descriptor();
    command.target.selection_rule = SelectionRule::SelectedOrArgument;
    command.args.push(ArgDescriptor {
        name: "entity_id".into(),
        kind: ArgKind::Entity,
        required: false,
        default: None,
        range: None,
        choices: vec![],
        help: "Target entity".into(),
        sensitive: false,
    });
    let mut requested = action();
    requested.selected_entity_id = None;
    requested.typed_args.insert("entity_id".into(), json!(20));
    let mut snapshot = actor();
    snapshot.selected_entity_id = None;
    snapshot.selected_target_kind = None;
    assert_eq!(
        validate_command(&requested, &command, &snapshot).unwrap_err(),
        Refusal::StaleTarget
    );
    snapshot.argument_target_entity_id = Some(21);
    snapshot.argument_target_kind = Some(TargetKind::Mob);
    assert_eq!(
        validate_command(&requested, &command, &snapshot).unwrap_err(),
        Refusal::StaleTarget
    );
    snapshot.argument_target_entity_id = Some(20);
    assert!(validate_command(&requested, &command, &snapshot).is_ok());
    requested.selected_entity_id = Some(30);
    snapshot.selected_entity_id = Some(30);
    snapshot.selected_target_kind = Some(TargetKind::Player);
    assert!(validate_command(&requested, &command, &snapshot).is_ok());
}

#[test]
fn typed_action_refuses_malformed_missing_and_extra_args() {
    let mut requested = action();
    requested.typed_args.insert("count".into(), json!("2"));
    assert_eq!(
        validate_command(&requested, &descriptor(), &actor()).unwrap_err(),
        Refusal::MalformedArgument
    );
    requested.typed_args.clear();
    assert_eq!(
        validate_command(&requested, &descriptor(), &actor()).unwrap_err(),
        Refusal::MissingArgument
    );
    requested.typed_args.insert("count".into(), json!(6));
    assert_eq!(
        validate_command(&requested, &descriptor(), &actor()).unwrap_err(),
        Refusal::OutOfRange
    );
    requested.typed_args.insert("count".into(), json!(2));
    requested.typed_args.insert("extra".into(), json!(0));
    assert_eq!(
        validate_command(&requested, &descriptor(), &actor()).unwrap_err(),
        Refusal::UnknownArgument
    );
}

#[test]
fn manual_pending_metadata_is_never_executable() {
    let mut pending = descriptor();
    pending.manual_pending = true;
    assert_eq!(
        validate_command(&action(), &pending, &actor()).unwrap_err(),
        Refusal::Unsupported
    );
}

#[test]
fn typed_action_keeps_all_three_receipt_outcomes_separate() {
    let accepted = validate_command(&action(), &descriptor(), &actor()).unwrap();
    assert_eq!(accepted.resolved_args["count"], json!(2));
    let receipt = ExecutionReceipt {
        descriptor_id: accepted.action.descriptor_id,
        submitted: true,
        server_accepted: None,
        verified: None,
        feedback: vec![],
        before_reads: vec![],
        after_reads: vec![],
        audit_id: None,
        evidence_paths: vec![],
    };
    let value = serde_json::to_value(receipt).unwrap();
    assert_eq!(value["submitted"], true);
    assert!(value["server_accepted"].is_null());
    assert!(value["verified"].is_null());
}
