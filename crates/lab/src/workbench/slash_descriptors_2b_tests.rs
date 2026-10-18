//! Regression guards for slash words 82–108 and their disabled palette forms.

use std::collections::BTreeMap;

use serde_json::json;

use super::commands::{
    validate_command, ActorSnapshot, Availability, CommandAction, CommandChannel, Compensation,
    ImpactKind, Refusal, SelectionRule, TargetKind, VerifyRecipe,
};
use super::slash_descriptors_2b::{
    descriptors_for, display_argument, shout_audience_preview, slash_descriptors_2b, HandlerStatus,
};
use super::slash_inventory::parse_patch_table;

const PATCH: &str =
    include_str!("../../../../data/client-patches/012-gm-slash-commands/commands.toml");

const EXPECTED: &[(&str, &str)] = &[
    (
        "/gmgiveminigamecontact",
        "ContactId:Integer:true,TargetId:Integer:true",
    ),
    (
        "/gmremoveminigamecontact",
        "ContactId:Integer:true,TargetId:Integer:true",
    ),
    (
        "/gmgivestargateaddress",
        "AddressId:Text:true,Hidden:Boolean:true",
    ),
    ("/gmremovestargateaddress", "AddressId:Text:true"),
    ("/gmloadconstants", ""),
    ("/gmloadability", "aAbilityId:Integer:true"),
    ("/gmreloadscripts", "nacsiId:Integer:true"),
    ("/gmloadabilityset", "aAbilitySetId:Integer:true"),
    ("/gmloadbehavior", "behaviorEventSetId:Integer:true"),
    ("/gmloadmob", "mobId:Integer:true"),
    ("/gmloaddialogset", "interactionSetId:Integer:true"),
    ("/gmloaditem", "itemId:Integer:true"),
    ("/gmloadmission", "missionId:Integer:true"),
    (
        "/gmsetmobvariable",
        "variable:Integer:true,value:Integer:true",
    ),
    ("/gmentererroraistate", ""),
    ("/gmexiterroraistate", ""),
    ("/gmperfstatsbychannel", "aOnOff:Boolean:true"),
    ("/gmshowinstanceflag", "aFlagNumber:Integer:true"),
    (
        "/gmsetinstanceflag",
        "aFlagNumber:Integer:true,aFlagValue:Integer:true",
    ),
    (
        "/gmspawnentityloot",
        "EntityId:Integer:true,LootTableID:Integer:true",
    ),
    ("/gmtestlos", "sourceID:Integer:true,targetID:Integer:true"),
    ("/gmtogglecombatlos", ""),
    ("/gmtrackmob", ""),
    ("/gmonxrayeyes", "on/off:Boolean:true"),
    ("/gmoninvisible", "on/off:Boolean:true"),
    ("/gmonphysics", "on/off:Boolean:true"),
    ("/gmsendgmshout", "isGlobal:Boolean:true,Text:Text:true"),
];

#[test]
fn exact_source_names_types_and_disabled_states() {
    let forms = slash_descriptors_2b().unwrap();
    assert_eq!(forms.len(), 27);
    let actual = forms
        .iter()
        .map(|form| {
            let signature = form
                .descriptor
                .args
                .iter()
                .map(|a| format!("{}:{:?}:{}", a.name, a.kind, a.required))
                .collect::<Vec<_>>()
                .join(",");
            (form.descriptor.id.as_str(), signature)
        })
        .collect::<Vec<_>>();
    let expected = EXPECTED
        .iter()
        .map(|(name, signature)| (*name, (*signature).to_string()))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    assert!(forms
        .iter()
        .all(|f| f.descriptor.channel == CommandChannel::Slash
            && f.descriptor.availability == Availability::Unsupported
            && f.descriptor.manual_pending
            && f.descriptor.example.is_none()
            && f.descriptor
                .args
                .iter()
                .all(|a| a.required && a.default.is_none())
            && f.descriptor
                .disposition_reason
                .as_deref()
                .unwrap()
                .contains("native")));
}

#[test]
fn changed_patch_event_type_or_access_fails_closed() {
    let mut entries = parse_patch_table(PATCH).unwrap();
    entries[81].event = "Event_SlashCmd_Other".into();
    assert!(descriptors_for(&entries).is_err());
    let mut entries = parse_patch_table(PATCH).unwrap();
    entries[107].parameters[1].name = "Message".into();
    assert!(descriptors_for(&entries).is_err());
    let mut entries = parse_patch_table(PATCH).unwrap();
    entries[101].access = "a".into();
    assert!(descriptors_for(&entries).is_err());
}

#[test]
fn target_words_never_infer_a_recipient_or_server_effect() {
    let forms = slash_descriptors_2b().unwrap();
    for form in &forms[..4] {
        assert_eq!(form.handler_status, HandlerStatus::Absent);
        assert!(form.descriptor.impact.is_none());
        assert!(form.descriptor.verify.is_none());
        assert_eq!(form.descriptor.target.kind, TargetKind::None);
        assert_eq!(form.descriptor.target.selection_rule, SelectionRule::None);
        assert!(form.target_scope.contains("target"));
        assert!(!form.precondition_note.is_empty());
    }
    assert!(forms[2].readback_note.contains("cannot establish"));
    let los = &forms[20];
    assert_eq!(los.handler_status, HandlerStatus::Implemented);
    assert_eq!(
        los.descriptor.impact.as_ref().unwrap().kind,
        ImpactKind::ReadOnly
    );
    assert_eq!(los.descriptor.target.kind, TargetKind::AnyEntity);
    assert_eq!(
        los.descriptor.target.selection_rule,
        SelectionRule::ArgumentRequired
    );
    assert_eq!(
        los.descriptor.target.argument_name.as_deref(),
        Some("targetID")
    );
    assert!(los.readback_note.contains("UNKNOWN"));
    assert!(los.target_scope.contains("including props"));
    assert!(los.precondition_note.contains("Both signed INT32 IDs"));
    assert!(los.precondition_note.contains("caller's space"));
    assert!(los.descriptor.verify.is_none()); // Caller feedback, no independent read tool.
}

#[test]
fn los_gate_accepts_a_prop_target_but_requires_both_explicit_ids() {
    let form = slash_descriptors_2b().unwrap().remove(20);
    // Exercise only the shared target/argument validator. The production form
    // stays disabled and has no accepted-effect verification recipe.
    let mut descriptor = form.descriptor;
    descriptor.availability = Availability::Runnable;
    descriptor.manual_pending = false;
    descriptor.verify = Some(VerifyRecipe {
        read_tools: vec!["client_chat_log".into()],
        comparison_fields: vec!["feedback".into()],
    });
    let mut action = CommandAction {
        descriptor_id: descriptor.id.clone(),
        instance: "lab".into(),
        actor_entity_id: 7,
        selected_entity_id: None,
        typed_args: BTreeMap::from([
            ("sourceID".into(), json!(31)),
            ("targetID".into(), json!(30)),
        ]),
        idempotency_key: "test".into(),
    };
    let actor = ActorSnapshot {
        instance: "lab".into(),
        entity_id: 7,
        is_gm: true,
        selected_entity_id: None,
        selected_target_kind: None,
        argument_target_entity_id: Some(30),
        argument_target_kind: Some(TargetKind::Spawnable),
    };
    assert!(validate_command(&action, &descriptor, &actor).is_ok());
    action.typed_args.remove("sourceID");
    assert_eq!(
        validate_command(&action, &descriptor, &actor),
        Err(Refusal::MissingArgument)
    );
    // The server handler independently checks both IDs in the caller's space;
    // ActorSnapshot has no source-space field, so this validator cannot prove it.
    assert!(form
        .precondition_note
        .contains("Both signed INT32 IDs must resolve in caller's space"));
}

#[test]
fn announcement_metadata_and_pure_preview_helpers_preserve_outcome_limits() {
    let forms = slash_descriptors_2b().unwrap();
    assert_eq!(
        forms
            .iter()
            .filter(|f| f.descriptor.category == "announcement")
            .count(),
        1
    );
    let shout = &forms[26];
    assert_eq!(shout.handler_status, HandlerStatus::Implemented);
    assert_eq!(
        shout.descriptor.impact.as_ref().unwrap().kind,
        ImpactKind::Irreversible
    );
    assert_eq!(shout.descriptor.compensation, Some(Compensation::None));
    assert!(shout.descriptor.args[1].sensitive);
    let secret = json!("Public test message");
    let redacted = display_argument(&shout.descriptor.args[1], &secret, false);
    assert_eq!(redacted, "[redacted]");
    assert!(!serde_json::to_string(&redacted)
        .unwrap()
        .contains("Public test message"));
    assert_eq!(
        display_argument(&shout.descriptor.args[1], &secret, true),
        secret.to_string()
    );
    assert!(!shout.descriptor.args[0].sensitive);
    assert!(shout
        .readback_note
        .contains("submitted line is not acceptance"));
    assert!(shout.readback_note.contains("differs from aggregate"));
    assert!(shout.readback_note.contains("no per-shout correlation"));
    assert!(shout.descriptor.verify.is_none());
    assert!(shout_audience_preview(false).contains("exact space instance"));
    assert!(shout_audience_preview(true).contains("across instances"));
}

#[test]
fn physics_scope_and_uncertain_boolean_polarity_are_explicit() {
    let forms = slash_descriptors_2b().unwrap();
    let physics = &forms[25];
    assert_eq!(physics.handler_status, HandlerStatus::Implemented);
    assert_eq!(
        physics.descriptor.impact.as_ref().unwrap().kind,
        ImpactKind::Transient
    );
    assert!(physics.target_scope.contains("Calling GM"));
    assert!(physics.descriptor.args[0]
        .help
        .contains("byte 0 enables bypass"));
    assert!(physics.readback_note.contains("no direct flag read tool"));
    assert!(physics.descriptor.verify.is_none());
    assert!(forms
        .iter()
        .enumerate()
        .all(|(i, f)| { matches!(i, 20 | 25 | 26) || f.handler_status == HandlerStatus::Absent }));
}
