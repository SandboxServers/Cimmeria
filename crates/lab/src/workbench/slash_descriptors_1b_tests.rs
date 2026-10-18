//! Regression guards for the second source-backed native slash form slice.

use std::collections::BTreeMap;

use serde_json::json;

use super::commands::{
    validate_command, ActorSnapshot, Availability, CommandAction, CommandChannel,
    CommandDescriptor, ImpactKind, Refusal, SelectionRule, TargetKind,
};
use super::slash_descriptors_1b::{descriptors_1b, NAMES};
use super::slash_inventory::{parse_patch_table, SlashParameterType};

const PATCH: &str =
    include_str!("../../../../data/client-patches/012-gm-slash-commands/commands.toml");

fn slice() -> Vec<super::slash_inventory::SlashEntry> {
    parse_patch_table(PATCH)
        .unwrap()
        .into_iter()
        .skip(27)
        .take(27)
        .collect()
}

fn runnable(
    index: usize,
    typed_args: BTreeMap<String, serde_json::Value>,
) -> (CommandDescriptor, CommandAction, ActorSnapshot) {
    let mut descriptor = descriptors_1b(&slice()).unwrap()[index].clone();
    descriptor.availability = Availability::Runnable;
    descriptor.manual_pending = false;
    let action = CommandAction {
        descriptor_id: descriptor.id.clone(),
        instance: "lab".into(),
        actor_entity_id: 7,
        selected_entity_id: None,
        typed_args,
        idempotency_key: "test".into(),
    };
    let actor = ActorSnapshot {
        instance: "lab".into(),
        entity_id: 7,
        is_gm: true,
        selected_entity_id: None,
        selected_target_kind: None,
        argument_target_entity_id: None,
        argument_target_kind: None,
    };
    (descriptor, action, actor)
}

#[test]
fn exact_names_and_toml_typed_forms_are_preserved() {
    let entries = slice();
    let descriptors = descriptors_1b(&entries).unwrap();
    assert_eq!(descriptors.len(), 27);
    assert_eq!(
        descriptors
            .iter()
            .map(|d| d.id.as_str())
            .collect::<Vec<_>>(),
        NAMES
    );
    let signatures: Vec<String> = descriptors
        .iter()
        .map(|d| {
            d.args
                .iter()
                .map(|a| format!("{}:{:?}:{}", a.name, a.kind, a.required))
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect();
    assert_eq!(signatures, [
        "RespawnerMobId:Integer:true",
        "disciplineId:Integer:true,expertise:Integer:true",
        "points:Integer:true", "racialParadigmId:Integer:true,levels:Integer:true",
        "on/off:Boolean:true", "", "", "on/off:Boolean:true", "Multiplier:Float:true",
        "Amount:Integer:true,TargetId:Integer:true",
        "Amount:Integer:true,TargetId:Integer:true",
        "Amount:Integer:true,TargetId:Integer:true",
        "Amount:Integer:true,TargetId:Integer:true",
        "aFlagId:Integer:true,aForceVal:Integer:true", "Level:Integer:true",
        "", "", "", "NameOrID:Text:true", "StanceID:Integer:true",
        "abilitySetId:Integer:false", "DestinationId:Integer:true",
        "TargetName:Text:true", "TargetName:Text:true",
        "WorldName:Text:true,X-Coordinate:Float:true,Y-Coordinate:Float:true,Z-Coordinate:Float:true",
        "X-Coordinate:Float:true,Y-Coordinate:Float:true,Z-Coordinate:Float:true", "",
    ]);
    assert!(descriptors
        .iter()
        .all(|d| d.channel == CommandChannel::Slash
            && d.availability == Availability::Unsupported
            && d.manual_pending
            && d.disposition_reason
                .as_ref()
                .is_some_and(|reason| !reason.is_empty())
            && d.args.iter().all(|arg| arg.default.is_none())));
}

#[test]
fn optional_and_wire_mismatches_are_visible_without_inventing_defaults() {
    let descriptors = descriptors_1b(&slice()).unwrap();
    let mob = &descriptors[20];
    assert_eq!(mob.id, "/gmsetmobabilityset");
    assert_eq!(mob.target.kind, TargetKind::Mob);
    assert_eq!(mob.target.selection_rule, SelectionRule::Required);
    assert!(!mob.args[0].required);
    assert_eq!(mob.args[0].default, None);
    assert!(mob
        .disposition_reason
        .as_ref()
        .unwrap()
        .contains("server requires INT32"));
    assert!(mob.verify.is_none());
    assert!(descriptors[21].args[0].help.contains("signed INT8"));
    assert_eq!(
        descriptors[21].args[0].range.as_ref().unwrap().max,
        Some(127.0)
    );
    for stat in &descriptors[9..13] {
        assert!(stat.args[1].help.contains("INT64"));
        assert_eq!(stat.args[1].range.as_ref().unwrap().min, Some(0.0));
        assert_eq!(
            stat.target.selection_rule,
            SelectionRule::ArgumentRequiredSelfZero
        );
        assert_eq!(stat.target.argument_name.as_deref(), Some("TargetId"));
    }
    assert_eq!(
        descriptors[22].target.selection_rule,
        SelectionRule::ArgumentRequired
    );
    assert_eq!(
        descriptors[23].target.selection_rule,
        SelectionRule::ArgumentRequiredOther
    );
    assert!(descriptors[22..24].iter().all(|descriptor| descriptor
        .target
        .argument_name
        .as_deref()
        == Some("TargetName")));
    assert!(descriptors[5].args.is_empty());
    assert!(descriptors[6].args.is_empty());
    assert!(descriptors[26].args.is_empty());
}

#[test]
fn grants_reload_and_travel_keep_distinct_effect_and_readback_contracts() {
    let descriptors = descriptors_1b(&slice()).unwrap();
    assert_eq!(
        descriptors[1].impact.as_ref().unwrap().kind,
        ImpactKind::Persistent
    );
    assert!(descriptors[1]
        .verify
        .as_ref()
        .unwrap()
        .comparison_fields
        .iter()
        .any(|field| field.contains("discipline_expertise")));
    let expertise_fields = &descriptors[1].verify.as_ref().unwrap().comparison_fields;
    for field in [
        "sgw_player.player_id",
        "sgw_player_discipline_expertise.player_id",
        "sgw_player_discipline_expertise.discipline_id",
        "sgw_player_discipline_expertise.expertise",
    ] {
        assert!(expertise_fields.contains(&field.into()));
    }
    assert_eq!(
        descriptors[2].impact.as_ref().unwrap().kind,
        ImpactKind::Persistent
    );
    assert!(descriptors[2]
        .verify
        .as_ref()
        .unwrap()
        .comparison_fields
        .iter()
        .any(|field| field.contains("applied_science_points")));
    assert_eq!(
        descriptors[21].impact.as_ref().unwrap().kind,
        ImpactKind::Persistent
    );
    assert!(descriptors[21]
        .verify
        .as_ref()
        .unwrap()
        .comparison_fields
        .iter()
        .any(|field| field.contains("known_stargates")));
    assert_eq!(
        descriptors[26].impact.as_ref().unwrap().kind,
        ImpactKind::Transient
    );
    assert!(descriptors[26].summary.contains("Re-push"));
    assert!(descriptors[26].verify.is_none());
    assert!(descriptors[26]
        .disposition_reason
        .as_ref()
        .unwrap()
        .contains("capture is pending"));
    for ability in &descriptors[15..17] {
        let fields = &ability.verify.as_ref().unwrap().comparison_fields;
        assert!(fields.contains(&"sgw_player_ability_grants.ability_id".into()));
        assert!(fields.contains(&"sgw_player_ability_grants.player_id".into()));
        assert!(fields.contains(&"sgw_player_ability_grants.source_kind".into()));
        assert!(fields.contains(&"sgw_player_ability_grants.source_id".into()));
        assert!(!fields
            .iter()
            .any(|field| field.contains("gm_grant_provenance")));
    }
    for unimplemented in [0, 3, 5, 6, 7, 8, 13, 14, 17, 19] {
        assert!(descriptors[unimplemented].impact.is_none());
        assert!(descriptors[unimplemented].verify.is_none());
    }
}

#[test]
fn signed_int32_arguments_stop_at_the_source_wire_limit() {
    let descriptors = descriptors_1b(&slice()).unwrap();
    for (index, arg_name) in [
        (1, "disciplineId"),
        (1, "expertise"),
        (2, "points"),
        (9, "Amount"),
    ] {
        let arg = descriptors[index]
            .args
            .iter()
            .find(|arg| arg.name == arg_name)
            .unwrap();
        assert_eq!(arg.range.as_ref().unwrap().max, Some(i32::MAX as f64));
    }
    assert_eq!(
        descriptors[20].args[0].range.as_ref().unwrap().max,
        Some(i32::MAX as f64)
    );
    let (expertise, mut action, actor) = runnable(
        1,
        BTreeMap::from([
            ("disciplineId".into(), json!(1)),
            ("expertise".into(), json!(i32::MAX)),
        ]),
    );
    assert!(validate_command(&action, &expertise, &actor).is_ok());
    action
        .typed_args
        .insert("expertise".into(), json!(i64::from(i32::MAX) + 1));
    assert_eq!(
        validate_command(&action, &expertise, &actor),
        Err(Refusal::OutOfRange)
    );
    action.typed_args.insert("expertise".into(), json!(1));
    action
        .typed_args
        .insert("disciplineId".into(), json!(i64::from(i32::MAX) + 1));
    assert_eq!(
        validate_command(&action, &expertise, &actor),
        Err(Refusal::OutOfRange)
    );

    let (stat, mut action, mut actor) = runnable(
        9,
        BTreeMap::from([
            ("Amount".into(), json!(i32::MAX)),
            ("TargetId".into(), json!(0)),
        ]),
    );
    actor.argument_target_entity_id = Some(7);
    actor.argument_target_kind = Some(TargetKind::Player);
    assert!(validate_command(&action, &stat, &actor).is_ok());
    action
        .typed_args
        .insert("Amount".into(), json!(i64::from(i32::MAX) + 1));
    assert_eq!(
        validate_command(&action, &stat, &actor),
        Err(Refusal::OutOfRange)
    );
}

#[test]
fn numeric_target_text_and_nonempty_world_name_are_enforced() {
    let (target, mut action, actor) =
        runnable(18, BTreeMap::from([("NameOrID".into(), json!("0"))]));
    assert!(validate_command(&action, &target, &actor).is_ok());
    action
        .typed_args
        .insert("NameOrID".into(), json!("not-a-number"));
    assert_eq!(
        validate_command(&action, &target, &actor),
        Err(Refusal::MalformedArgument)
    );
    action.typed_args.insert("NameOrID".into(), json!("-1"));
    assert_eq!(
        validate_command(&action, &target, &actor),
        Err(Refusal::OutOfRange)
    );
    action
        .typed_args
        .insert("NameOrID".into(), json!("2147483648"));
    assert_eq!(
        validate_command(&action, &target, &actor),
        Err(Refusal::OutOfRange)
    );
    action
        .typed_args
        .insert("NameOrID".into(), json!(i32::MAX.to_string()));
    assert!(validate_command(&action, &target, &actor).is_ok());

    let (world, mut action, actor) = runnable(
        24,
        BTreeMap::from([
            ("WorldName".into(), json!("Harset")),
            ("X-Coordinate".into(), json!(1.0)),
            ("Y-Coordinate".into(), json!(2.0)),
            ("Z-Coordinate".into(), json!(3.0)),
        ]),
    );
    assert!(validate_command(&action, &world, &actor).is_ok());
    for bad in ["", "  \t "] {
        action.typed_args.insert("WorldName".into(), json!(bad));
        assert_eq!(
            validate_command(&action, &world, &actor),
            Err(Refusal::MalformedArgument)
        );
    }
}

#[test]
fn source_drift_refuses_and_disabled_form_never_claims_acceptance() {
    let mut entries = slice();
    entries.swap(0, 1);
    assert!(descriptors_1b(&entries).is_err());
    let mut entries = slice();
    entries[0].access = "g".into();
    assert!(descriptors_1b(&entries).is_err());
    let mut entries = slice();
    entries[21].event = "Event_SlashCmd_AnotherDHD".into();
    assert!(descriptors_1b(&entries).is_err());
    let mut entries = slice();
    entries[21].parameters[0].parameter_type = SlashParameterType::Float;
    assert!(descriptors_1b(&entries).is_err());
    let mut entries = slice();
    entries[21].parameters[0].name = "AnotherDestination".into();
    assert!(descriptors_1b(&entries).is_err());
    let descriptors = descriptors_1b(&slice()).unwrap();
    let descriptor = &descriptors[1];
    let action = CommandAction {
        descriptor_id: descriptor.id.clone(),
        instance: "lab".into(),
        actor_entity_id: 7,
        selected_entity_id: None,
        typed_args: BTreeMap::new(),
        idempotency_key: "test".into(),
    };
    let actor = ActorSnapshot {
        instance: "lab".into(),
        entity_id: 7,
        is_gm: true,
        selected_entity_id: None,
        selected_target_kind: None,
        argument_target_entity_id: None,
        argument_target_kind: None,
    };
    assert_eq!(
        validate_command(&action, descriptor, &actor),
        Err(Refusal::Unsupported)
    );
}

#[test]
fn explicit_targets_require_fresh_argument_resolution_and_never_fall_back_to_selection() {
    let descriptors = descriptors_1b(&slice()).unwrap();
    for (index, key, value, target_id) in [
        (9, "TargetId", json!(42), 42),
        (22, "TargetName", json!("42"), 42),
        (23, "TargetName", json!("42"), 42),
    ] {
        let mut descriptor = descriptors[index].clone();
        descriptor.availability = Availability::Runnable;
        descriptor.manual_pending = false;
        let mut typed_args = BTreeMap::from([(key.into(), value)]);
        if index == 9 {
            typed_args.insert("Amount".into(), json!(10));
        }
        let mut action = CommandAction {
            descriptor_id: descriptor.id.clone(),
            instance: "lab".into(),
            actor_entity_id: 7,
            selected_entity_id: Some(99),
            typed_args,
            idempotency_key: "test".into(),
        };
        let mut actor = ActorSnapshot {
            instance: "lab".into(),
            entity_id: 7,
            is_gm: true,
            selected_entity_id: Some(99),
            selected_target_kind: Some(TargetKind::Player),
            argument_target_entity_id: Some(target_id),
            argument_target_kind: Some(TargetKind::Mob),
        };
        assert!(validate_command(&action, &descriptor, &actor).is_ok());
        actor.argument_target_entity_id = None;
        assert_eq!(
            validate_command(&action, &descriptor, &actor),
            Err(Refusal::StaleTarget)
        );
        actor.argument_target_entity_id = Some(target_id + 1);
        assert_eq!(
            validate_command(&action, &descriptor, &actor),
            Err(Refusal::StaleTarget)
        );
        actor.argument_target_entity_id = Some(target_id);
        actor.argument_target_kind = Some(TargetKind::Spawnable);
        assert_eq!(
            validate_command(&action, &descriptor, &actor),
            Err(Refusal::WrongTargetKind)
        );
        actor.argument_target_kind = Some(TargetKind::Mob);
        action.typed_args.remove(key);
        assert_eq!(
            validate_command(&action, &descriptor, &actor),
            Err(Refusal::MissingArgument)
        );
    }
}

#[test]
fn stat_zero_resolves_only_to_caller_and_summon_rejects_self() {
    let descriptors = descriptors_1b(&slice()).unwrap();
    let mut stat = descriptors[9].clone();
    stat.availability = Availability::Runnable;
    stat.manual_pending = false;
    let mut action = CommandAction {
        descriptor_id: stat.id.clone(),
        instance: "lab".into(),
        actor_entity_id: 7,
        selected_entity_id: None,
        typed_args: BTreeMap::from([("Amount".into(), json!(10)), ("TargetId".into(), json!(0))]),
        idempotency_key: "test".into(),
    };
    let mut actor = ActorSnapshot {
        instance: "lab".into(),
        entity_id: 7,
        is_gm: true,
        selected_entity_id: None,
        selected_target_kind: None,
        argument_target_entity_id: Some(7),
        argument_target_kind: Some(TargetKind::Player),
    };
    assert!(validate_command(&action, &stat, &actor).is_ok());
    actor.argument_target_entity_id = Some(8);
    assert_eq!(
        validate_command(&action, &stat, &actor),
        Err(Refusal::StaleTarget)
    );

    let mut summon = descriptors[23].clone();
    summon.availability = Availability::Runnable;
    summon.manual_pending = false;
    action.descriptor_id = summon.id.clone();
    action.typed_args = BTreeMap::from([("TargetName".into(), json!("7"))]);
    actor.argument_target_entity_id = Some(7);
    assert_eq!(
        validate_command(&action, &summon, &actor),
        Err(Refusal::StaleTarget)
    );
    action.typed_args.insert("TargetName".into(), json!("0"));
    actor.argument_target_entity_id = Some(0);
    assert_eq!(
        validate_command(&action, &summon, &actor),
        Err(Refusal::StaleTarget)
    );
}
