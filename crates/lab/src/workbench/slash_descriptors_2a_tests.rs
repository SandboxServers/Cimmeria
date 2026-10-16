//! Third slash slice source, safety and audit-preview guards.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

use super::commands::{
    validate_command, ActorSnapshot, ArgKind, Availability, CommandAction, CommandChannel,
    Compensation, ImpactKind, Refusal, SelectionRule, TargetKind,
};
use super::slash_descriptors_2a::{
    audit_argument, descriptors_for, slash_descriptors_2a, HandlerAvailability,
};
use super::slash_inventory::{parse_patch_table, SlashParameterType};

const TOML: &str =
    include_str!("../../../../data/client-patches/012-gm-slash-commands/commands.toml");
const NAMES: &[&str] = &[
    "/gmreloadinventory",
    "/gmusers",
    "/gmsethidegm",
    "/gmprintstats",
    "/gmdebugability",
    "/gmdebugcombat",
    "/gmdebugcombatverbose",
    "/gmdebugheal",
    "/gmdebugstartminigame",
    "/gmdebugspectateminigame",
    "/gmdebugjoinminigame",
    "/gmdebugabilityonmob",
    "/gmdebugbehaviorsonmob",
    "/gmdebugpathsonmob",
    "/gmdebugevents",
    "/gmmobdata",
    "/gmdebuginteract",
    "/gmemitbehavioreventonmob",
    "/gmaddbehavioreventset",
    "/gmremovebehavioreventset",
    "/gmspawnbycmd",
    "/gmdespawn",
    "/gmrechargeitem",
    "/gmsetmobattribute",
    "/gmrespawn",
    "/gmkilltarget",
    "/debugminigamecomplete",
];

#[test]
fn third_slice_exact_names_events_and_typed_arguments_equal_patch() {
    let source = parse_patch_table(TOML).unwrap();
    let forms = slash_descriptors_2a().unwrap();
    assert_eq!(forms.len(), 27);
    assert_eq!(
        forms
            .iter()
            .map(|f| f.source.name.as_str())
            .collect::<BTreeSet<_>>(),
        NAMES.iter().copied().collect()
    );
    for (form, original) in forms.iter().zip(&source[54..81]) {
        assert_eq!(&form.source, original);
        assert_eq!(form.descriptor.id, original.name);
        assert_eq!(form.descriptor.channel, CommandChannel::Slash);
        assert_eq!(form.descriptor.availability, Availability::Unsupported);
        assert!(form.descriptor.manual_pending);
        assert!(!form.descriptor.category.is_empty());
        assert!(!form.descriptor.summary.is_empty());
        assert!(!form.target_scope.is_empty());
        assert!(!form.readback_note.is_empty());
        assert!(form.descriptor.compensation.is_some());
        assert!(form
            .descriptor
            .disposition_reason
            .as_ref()
            .unwrap()
            .contains("unproved"));
        assert_eq!(form.descriptor.args.len(), original.parameters.len());
        for (arg, source_arg) in form.descriptor.args.iter().zip(&original.parameters) {
            assert_eq!(arg.name, source_arg.name);
            assert_eq!(arg.required, source_arg.required);
            assert_eq!(
                arg.kind,
                match source_arg.parameter_type {
                    SlashParameterType::Boolean => ArgKind::Boolean,
                    SlashParameterType::Integer => ArgKind::Integer,
                    SlashParameterType::Float => ArgKind::Float,
                    SlashParameterType::String => ArgKind::Text,
                }
            );
            assert_eq!(
                arg.sensitive,
                source_arg.parameter_type == SlashParameterType::String
            );
            assert!(arg.default.is_none());
            assert!(arg.range.is_none());
            assert!(!arg.help.is_empty());
        }
    }
    assert_eq!(forms.last().unwrap().source.access, "p");
}

#[test]
fn changed_patch_word_event_access_and_argument_shape_fail_closed() {
    let source = parse_patch_table(TOML).unwrap();
    let mut changed = source.clone();
    changed[54].name = "/changed".into();
    assert!(descriptors_for(&changed).is_err());
    changed = source.clone();
    changed[74].event = "Event_SlashCmd_SpawnByCmd".into();
    assert!(descriptors_for(&changed).is_err());
    changed = source.clone();
    changed[80].access = "g".into();
    assert!(descriptors_for(&changed).is_err());
    changed = source.clone();
    changed[80].parameters[0].parameter_type = SlashParameterType::String;
    assert!(descriptors_for(&changed).is_err());
    changed = source;
    let extra = changed[80].parameters[0].clone();
    changed[75].parameters.push(extra);
    assert!(descriptors_for(&changed).is_err());
}

#[test]
fn unimplemented_item_and_minigame_paths_have_no_claimed_effect_or_readback() {
    let forms = slash_descriptors_2a().unwrap();
    for word in [
        "/gmreloadinventory",
        "/gmrechargeitem",
        "/gmdebugstartminigame",
        "/gmdebugspectateminigame",
        "/gmdebugjoinminigame",
        "/debugminigamecomplete",
    ] {
        let form = forms.iter().find(|form| form.source.name == word).unwrap();
        assert_eq!(form.handler_availability, HandlerAvailability::Absent);
        assert!(form.descriptor.impact.is_none(), "{word}");
        assert!(form.descriptor.verify.is_none(), "{word}");
        assert_eq!(form.descriptor.compensation, Some(Compensation::None));
        assert!(
            form.descriptor
                .disposition_reason
                .as_ref()
                .unwrap()
                .contains("Handler")
                || form
                    .descriptor
                    .disposition_reason
                    .as_ref()
                    .unwrap()
                    .contains("handler")
        );
    }
    let debug = forms.last().unwrap();
    assert_eq!(debug.source.name, "/debugminigamecomplete");
    assert_eq!(debug.descriptor.args.len(), 1);
    assert!(debug.descriptor.impact.is_none());
    assert!(debug
        .descriptor
        .disposition_reason
        .as_ref()
        .unwrap()
        .contains("no dot"));
    assert!(debug.target_scope.contains("GM-tail"));
    assert!(debug.readback_note.contains("separate game path"));
}

#[test]
fn native_argument_mismatches_and_authority_boundaries_are_explicit() {
    let forms = slash_descriptors_2a().unwrap();
    for (word, reason) in [
        ("/gmdespawn", "no TargetID"),
        ("/gmkilltarget", "INT64"),
        ("/gmsetmobattribute", "missing target"),
        ("/gmspawnbycmd", "semantic candidate"),
    ] {
        let form = forms.iter().find(|f| f.source.name == word).unwrap();
        assert!(
            form.descriptor
                .disposition_reason
                .as_ref()
                .unwrap()
                .contains(reason),
            "{word}"
        );
    }
    let users = forms.iter().find(|f| f.source.name == "/gmusers").unwrap();
    assert_eq!(
        users.descriptor.impact.as_ref().unwrap().kind,
        ImpactKind::ReadOnly
    );
    assert_eq!(
        users.descriptor.verify.as_ref().unwrap().read_tools,
        ["server_entity_query"]
    );
    assert!(users
        .readback_note
        .contains("server_sessions has no space ID"));
    let data = forms
        .iter()
        .find(|f| f.source.name == "/gmmobdata")
        .unwrap();
    assert!(data.target_scope.contains("not mob-only"));
    assert!(data
        .readback_note
        .contains("threat count has no independent"));
    let respawn = forms
        .iter()
        .find(|f| f.source.name == "/gmrespawn")
        .unwrap();
    assert!(respawn
        .readback_note
        .contains("feedback may print even after early exit"));
    let kill = forms
        .iter()
        .find(|f| f.source.name == "/gmkilltarget")
        .unwrap();
    assert_eq!(
        kill.descriptor.impact.as_ref().unwrap().kind,
        ImpactKind::Irreversible
    );
    assert!(matches!(
        kill.descriptor.compensation,
        Some(Compensation::Manual(_))
    ));
}

#[test]
fn audit_preview_redacts_every_free_text_argument_and_never_builds_a_slash_line() {
    let forms = slash_descriptors_2a().unwrap();
    for form in &forms {
        for arg in &form.descriptor.args {
            if arg.kind == ArgKind::Text {
                assert!(arg.sensitive);
                assert_eq!(
                    audit_argument(&json!("secret words\n/another"), arg),
                    Some("(redacted)".into())
                );
                assert!(audit_argument(&json!(17), arg).is_none());
            }
        }
    }
    let spawn = forms
        .iter()
        .find(|f| f.source.name == "/gmspawnbycmd")
        .unwrap();
    assert_eq!(
        audit_argument(&json!(1.25), &spawn.descriptor.args[1]),
        Some("1.25".into())
    );
    assert!(audit_argument(&json!("1.25"), &spawn.descriptor.args[1]).is_none());
    assert!(audit_argument(&Value::Null, &spawn.descriptor.args[1]).is_none());
    let hide = forms
        .iter()
        .find(|f| f.source.name == "/gmsethidegm")
        .unwrap();
    assert_eq!(
        audit_argument(&json!(true), &hide.descriptor.args[0]),
        Some("true".into())
    );
    assert!(audit_argument(&json!("true"), &hide.descriptor.args[0]).is_none());
    let kill = forms
        .iter()
        .find(|f| f.source.name == "/gmkilltarget")
        .unwrap();
    assert_eq!(
        audit_argument(&json!(-1), &kill.descriptor.args[0]),
        Some("-1".into())
    );
    assert!(audit_argument(&json!(1.5), &kill.descriptor.args[0]).is_none());
}

#[test]
fn selected_npc_forms_require_fresh_selection_and_mob_kind_if_ever_enabled() {
    let forms = slash_descriptors_2a().unwrap();
    for (word, arguments) in [
        (
            "/gmdebugabilityonmob",
            BTreeMap::from([("AbilityID".into(), json!(0))]),
        ),
        ("/gmdespawn", BTreeMap::new()),
    ] {
        let form = forms.iter().find(|form| form.source.name == word).unwrap();
        assert_eq!(form.descriptor.target.kind, TargetKind::Mob);
        assert_eq!(
            form.descriptor.target.selection_rule,
            SelectionRule::Required
        );
        let mut descriptor = form.descriptor.clone();
        descriptor.availability = Availability::Runnable;
        descriptor.manual_pending = false;
        let mut action = action(word, arguments);
        action.selected_entity_id = Some(20);
        let mut actor = actor();
        actor.selected_entity_id = None;
        assert_eq!(
            validate_command(&action, &descriptor, &actor).unwrap_err(),
            Refusal::StaleTarget,
            "{word}"
        );
        actor.selected_entity_id = Some(20);
        actor.selected_target_kind = Some(TargetKind::Player);
        assert_eq!(
            validate_command(&action, &descriptor, &actor).unwrap_err(),
            Refusal::WrongTargetKind,
            "{word}"
        );
        actor.selected_target_kind = Some(TargetKind::Mob);
        assert!(
            validate_command(&action, &descriptor, &actor).is_ok(),
            "{word}"
        );
    }
}

#[test]
fn explicit_target_forms_require_fresh_resolved_id_and_correct_kind_if_ever_enabled() {
    let forms = slash_descriptors_2a().unwrap();
    for (word, arguments, kind) in [
        (
            "/gmmobdata",
            BTreeMap::from([("SpaceId".into(), json!(1)), ("TargetId".into(), json!(30))]),
            TargetKind::AnyEntity,
        ),
        (
            "/gmkilltarget",
            BTreeMap::from([("TargetId".into(), json!(30))]),
            TargetKind::MobOrId,
        ),
    ] {
        let form = forms.iter().find(|form| form.source.name == word).unwrap();
        assert_eq!(form.descriptor.target.kind, kind);
        assert_eq!(
            form.descriptor.target.selection_rule,
            SelectionRule::SelectedOrArgument
        );
        assert_eq!(
            form.descriptor.target.argument_name.as_deref(),
            Some("TargetId")
        );
        let mut descriptor = form.descriptor.clone();
        descriptor.availability = Availability::Runnable;
        descriptor.manual_pending = false;
        let action = action(word, arguments);
        let mut actor = actor();
        assert_eq!(
            validate_command(&action, &descriptor, &actor).unwrap_err(),
            Refusal::StaleTarget,
            "{word}"
        );
        actor.argument_target_entity_id = Some(31);
        actor.argument_target_kind = Some(TargetKind::Mob);
        assert_eq!(
            validate_command(&action, &descriptor, &actor).unwrap_err(),
            Refusal::StaleTarget,
            "{word}"
        );
        actor.argument_target_entity_id = Some(30);
        actor.argument_target_kind = Some(TargetKind::Spawnable);
        if word == "/gmmobdata" {
            assert!(validate_command(&action, &descriptor, &actor).is_ok());
        } else {
            assert_eq!(
                validate_command(&action, &descriptor, &actor).unwrap_err(),
                Refusal::WrongTargetKind
            );
        }
        actor.argument_target_kind = Some(TargetKind::Mob);
        assert!(
            validate_command(&action, &descriptor, &actor).is_ok(),
            "{word}"
        );
    }
    let form = forms
        .iter()
        .find(|form| form.source.name == "/gmkilltarget")
        .unwrap();
    let mut descriptor = form.descriptor.clone();
    descriptor.availability = Availability::Runnable;
    descriptor.manual_pending = false;
    let mut action = action(
        "/gmkilltarget",
        BTreeMap::from([("TargetId".into(), json!(-1))]),
    );
    action.selected_entity_id = Some(20);
    let mut actor = actor();
    actor.selected_entity_id = Some(20);
    actor.selected_target_kind = Some(TargetKind::Mob);
    assert_eq!(
        validate_command(&action, &descriptor, &actor).unwrap_err(),
        Refusal::StaleTarget
    );
}

#[test]
fn real_read_sources_and_multi_layer_effect_proof_are_required() {
    let forms = slash_descriptors_2a().unwrap();
    for word in [
        "/gmdebugability",
        "/gmdebugcombat",
        "/gmdebugcombatverbose",
        "/gmdebugheal",
        "/gmdebugabilityonmob",
    ] {
        let form = forms.iter().find(|form| form.source.name == word).unwrap();
        let recipe = form.descriptor.verify.as_ref().unwrap();
        assert_eq!(recipe.read_tools, ["client_chat_log"], "{word}");
        assert!(recipe
            .comparison_fields
            .iter()
            .any(|field| field == "channel=Feedback"));
        assert!(recipe
            .comparison_fields
            .iter()
            .any(|field| field.starts_with("prefix=gmDebug")));
        assert!(recipe
            .comparison_fields
            .iter()
            .any(|field| field.contains("identity")));
    }
    for (word, required) in [
        (
            "/gmspawnbycmd",
            &["client_chat_log", "server_log_tail", "server_entity_get"][..],
        ),
        (
            "/gmdespawn",
            &[
                "client_chat_log",
                "server_entity_query",
                "server_packet_tap_read",
            ][..],
        ),
        (
            "/gmrespawn",
            &[
                "server_entity_get",
                "client_player_state",
                "server_packet_tap_read",
                "server_log_tail",
            ][..],
        ),
        (
            "/gmkilltarget",
            &[
                "client_chat_log",
                "server_entity_get",
                "server_log_tail",
                "server_packet_tap_read",
            ][..],
        ),
    ] {
        let form = forms.iter().find(|form| form.source.name == word).unwrap();
        assert_eq!(
            form.descriptor.verify.as_ref().unwrap().read_tools,
            required,
            "{word}"
        );
        assert!(
            form.readback_note.contains("alone") || form.readback_note.contains("unknown"),
            "{word}"
        );
    }
}

fn action(word: &str, typed_args: BTreeMap<String, Value>) -> CommandAction {
    CommandAction {
        descriptor_id: word.into(),
        instance: "lab-a".into(),
        actor_entity_id: 10,
        selected_entity_id: None,
        typed_args,
        idempotency_key: "review-test".into(),
    }
}

fn actor() -> ActorSnapshot {
    ActorSnapshot {
        instance: "lab-a".into(),
        entity_id: 10,
        is_gm: true,
        selected_entity_id: None,
        selected_target_kind: None,
        argument_target_entity_id: None,
        argument_target_kind: None,
    }
}
