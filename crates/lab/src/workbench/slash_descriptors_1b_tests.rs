//! Regression guards for the second source-backed native slash form slice.

use std::collections::BTreeMap;

use super::commands::{
    validate_command, ActorSnapshot, Availability, CommandAction, CommandChannel, ImpactKind,
    Refusal, SelectionRule, TargetKind,
};
use super::slash_descriptors_1b::{descriptors_1b, NAMES};
use super::slash_inventory::parse_patch_table;

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
    }
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
    assert!(descriptors[26]
        .verify
        .as_ref()
        .unwrap()
        .read_tools
        .iter()
        .any(|tool| tool == "client_org_view"));
    for unimplemented in [0, 3, 5, 6, 7, 8, 13, 14, 17, 19] {
        assert!(descriptors[unimplemented].impact.is_none());
        assert!(descriptors[unimplemented].verify.is_none());
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
