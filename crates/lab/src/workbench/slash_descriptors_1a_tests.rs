//! First slash slice contract and fail-closed preview guards.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::json;

use super::commands::{
    validate_command, ActorSnapshot, ArgKind, Availability, CommandAction, ExecutionReceipt,
    Refusal,
};
use super::slash_descriptors_1a::{
    descriptors_for, display_argument, slash_descriptors_1a, HandlerAvailability,
};
use super::slash_inventory::{parse_patch_table, SlashParameterType};
use super::slash_transport::{PlainShoutCandidate, SlashObservation, SlashReceipt, SlashStage};

const TOML: &str =
    include_str!("../../../../data/client-patches/012-gm-slash-commands/commands.toml");
const NAMES: &[&str] = &[
    "/gmmissionassign",
    "/gmmissionclear",
    "/gmmissionclearactive",
    "/gmmissionclearhistory",
    "/gmmissionlist",
    "/gmmissionlistfull",
    "/gmmissiondetails",
    "/gmmissionadvance",
    "/gmmissionreset",
    "/gmmissioncomplete",
    "/gmmissionsetavailable",
    "/gmmissionabandon",
    "/gmshowtargetlocation",
    "/gmlistabilities",
    "/gmshowflag",
    "/gmlistinteractions",
    "/gmgetmobattribute",
    "/gmshowmobcount",
    "/gmshowip",
    "/gmshowinventory",
    "/gmshowplayer",
    "/gmgivexp",
    "/gmgiveitem",
    "/gmgivecash",
    "/gmremoveitem",
    "/gmgiveability",
    "/gmgivetrainingpoints",
];

#[test]
fn first_slice_names_and_typed_arguments_equal_toml() {
    let source = parse_patch_table(TOML).unwrap();
    let forms = slash_descriptors_1a().unwrap();
    let expected: BTreeSet<_> = NAMES.iter().copied().collect();
    let actual: BTreeSet<_> = forms.iter().map(|form| form.source.name.as_str()).collect();
    assert_eq!(forms.len(), 27);
    assert_eq!(actual, expected);
    for (form, original) in forms.iter().zip(&source[..27]) {
        assert_eq!(&form.source, original);
        assert_eq!(form.descriptor.id, original.name);
        assert_eq!(form.descriptor.args.len(), original.parameters.len());
        assert_eq!(form.descriptor.availability, Availability::Unsupported);
        assert!(form.descriptor.manual_pending);
        assert!(!form.descriptor.category.is_empty());
        assert!(!form.descriptor.summary.is_empty());
        assert!(!form.target_scope.is_empty());
        assert!(!form.readback_note.is_empty());
        assert!(form.descriptor.impact.is_some());
        assert!(form.descriptor.compensation.is_some());
        assert!(form
            .descriptor
            .disposition_reason
            .as_ref()
            .unwrap()
            .contains("unproved"));
        for (arg, parameter) in form.descriptor.args.iter().zip(&original.parameters) {
            assert_eq!(arg.name, parameter.name);
            assert_eq!(arg.required, parameter.required);
            assert!(arg.default.is_none());
            assert!(!arg.help.is_empty());
        }
    }
}

#[test]
fn changed_toml_name_type_or_optional_shape_fails_closed() {
    let source = parse_patch_table(TOML).unwrap();
    let mut changed = source.clone();
    changed[0].name = "/different".into();
    assert!(descriptors_for(&changed).is_err());
    changed = source.clone();
    changed[0].parameters[1].parameter_type = SlashParameterType::Integer;
    assert!(descriptors_for(&changed).is_err());
    changed = source;
    changed[14].parameters[0].required = true;
    assert!(descriptors_for(&changed).is_err());
}

#[test]
fn boolean_and_optional_preview_preserves_toml_shape_without_native_serialization() {
    let forms = slash_descriptors_1a().unwrap();
    let assign = forms
        .iter()
        .find(|f| f.source.name == "/gmmissionassign")
        .unwrap();
    let popup = &assign.descriptor.args[1];
    assert_eq!(popup.kind, ArgKind::Boolean);
    assert!(popup.required);
    assert_eq!(
        display_argument(Some(&json!(true)), popup.kind).as_deref(),
        Some("true")
    );
    assert_eq!(
        display_argument(Some(&json!(false)), popup.kind).as_deref(),
        Some("false")
    );
    assert!(display_argument(Some(&json!(1)), popup.kind).is_none());
    let flag = forms
        .iter()
        .find(|f| f.source.name == "/gmshowflag")
        .unwrap();
    let optional = &flag.descriptor.args[0];
    assert!(!optional.required);
    assert!(optional.default.is_none());
    assert_eq!(
        display_argument(None, optional.kind).as_deref(),
        Some("(omitted)")
    );
    assert_eq!(
        flag.handler_availability,
        HandlerAvailability::OptionalArgumentMismatch
    );
    assert!(flag.descriptor.verify.is_none());
    assert!(flag
        .descriptor
        .disposition_reason
        .as_ref()
        .unwrap()
        .contains("requires INT32"));
}

#[test]
fn submission_alone_never_marks_server_acceptance_or_enables_form() {
    let forms = slash_descriptors_1a().unwrap();
    let grant = forms
        .iter()
        .find(|f| f.source.name == "/gmgivetrainingpoints")
        .unwrap();
    assert_eq!(grant.handler_availability, HandlerAvailability::Implemented);
    assert!(grant.descriptor.verify.is_some());
    let action = CommandAction {
        descriptor_id: grant.descriptor.id.clone(),
        instance: "lab-a".into(),
        actor_entity_id: 10,
        selected_entity_id: None,
        typed_args: BTreeMap::from([("numPoints".into(), json!(1))]),
        idempotency_key: "attempt-1".into(),
    };
    let actor = ActorSnapshot {
        instance: "lab-a".into(),
        entity_id: 10,
        is_gm: true,
        selected_entity_id: None,
        selected_target_kind: None,
        argument_target_entity_id: None,
        argument_target_kind: None,
    };
    assert_eq!(
        validate_command(&action, &grant.descriptor, &actor),
        Err(Refusal::Unsupported)
    );
    let submitted = ExecutionReceipt {
        descriptor_id: grant.descriptor.id.clone(),
        submitted: true,
        server_accepted: None,
        verified: None,
        feedback: vec![],
        before_reads: vec![],
        after_reads: vec![],
        audit_id: None,
        evidence_paths: vec![],
    };
    assert_eq!(submitted.server_accepted, None);
    assert_eq!(submitted.verified, None);
    let candidate = PlainShoutCandidate::parse("/gmsendgmshout 1 Probe123").unwrap();
    let mut receipt = SlashReceipt::new(candidate);
    let claim = SlashObservation {
        attempt_id: receipt.attempt_id,
        line: receipt.line.clone(),
        fingerprint: "caller-claim".into(),
        generation: 1,
        stage: SlashStage::ClientSubmitted,
    };
    assert!(receipt.observe(&claim).is_err());
    assert_eq!(receipt.stage(SlashStage::ClientSubmitted), None);
    assert!(!receipt.accepted());
}

#[test]
fn missing_or_mismatched_handlers_have_specific_disabled_reasons() {
    let forms = slash_descriptors_1a().unwrap();
    for form in forms {
        if form.handler_availability != HandlerAvailability::Implemented {
            assert!(form.descriptor.disposition_reason.as_ref().unwrap().len() > 75);
            assert!(form.descriptor.verify.is_none());
        }
    }
}
