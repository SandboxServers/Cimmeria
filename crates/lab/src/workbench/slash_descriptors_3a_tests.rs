//! Regression guards for patch 012 slash forms 109–135.

use super::commands::{
    Availability, CommandChannel, Compensation, ImpactKind, SelectionRule, TargetKind,
};
use super::slash_descriptors_3a::{descriptors_3a, NAMES};
use super::slash_inventory::{parse_patch_table, SlashEntry, SlashParameterType};

const PATCH: &str =
    include_str!("../../../../data/client-patches/012-gm-slash-commands/commands.toml");

fn slice() -> Vec<SlashEntry> {
    parse_patch_table(PATCH)
        .unwrap()
        .into_iter()
        .skip(108)
        .take(27)
        .collect()
}

#[test]
fn exact_names_events_and_typed_parameters_match_patch_012() {
    let entries = slice();
    let descriptors = descriptors_3a(&entries).unwrap();
    let expected_names = [
        "/gmregeneratecoverlinks",
        "/gmchangecoverweight",
        "/gmchangecoverstanceweight",
        "/gmhelpfull",
        "/gmtestsequence",
        "/gmrepairitem",
        "/gmgiveammo",
        "/gmgivefaction",
        "/gmsetfaction",
        "/gminvokeability",
        "/gmsettechskill",
        "/gmgiveblueprint",
        "/gmgivegearset",
        "/gmgiveinventory",
        "/gmdebugminigameinstance",
        "/gmdebugflash",
        "/gmshowlog",
        "/gmsetinfiniteammo",
        "/gmsetinvulnerable",
        "/gmsetomnipotent",
        "/gmsetignorefocus",
        "/gmsetignorehealth",
        "/gmsetfly",
        "/gmsetghost",
        "/gmsetspectator",
        "/gmsetnotarget",
        "/gmsetpvp",
    ];
    assert_eq!(NAMES, expected_names);
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
        .map(|descriptor| {
            descriptor
                .args
                .iter()
                .map(|arg| format!("{}:{:?}:{}", arg.name, arg.kind, arg.required))
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect();
    assert_eq!(signatures, [
        "normalLimit:Float:true,maxLinks:Integer:true,maxDistance:Float:true",
        "distance:Float:true,defCover:Float:true,offCover:Float:true,move:Float:true,crossPath:Float:true,cover:Float:true",
        "stance:Text:true,distance:Float:true,defCover:Float:true,offCover:Float:true,move:Float:true,crossPath:Float:true,cover:Float:true",
        "", "", "ItemId:Integer:true,RepairRatio:Float:true", "", "", "", "", "", "", "", "",
        "instanceId:Integer:true,seed:Integer:true,techcomp:Integer:true", "",
        "on/off:Boolean:false", "on/off:Boolean:false", "on/off:Boolean:false", "",
        "on/off:Boolean:false", "on/off:Boolean:false", "on/off:Boolean:false", "on/off:Boolean:false",
        "on/off:Boolean:false", "on/off:Boolean:false", "on/off:Boolean:true",
    ]);
    for (index, entry) in entries.iter().enumerate() {
        assert_eq!(entry.name, expected_names[index]);
        let mut renamed = entries.clone();
        renamed[index].event.push('X');
        assert!(
            descriptors_3a(&renamed).is_err(),
            "{} event mutation passed",
            entry.name
        );
        if !entry.parameters.is_empty() {
            let mut mutated = entries.clone();
            mutated[index].parameters[0].required = !mutated[index].parameters[0].required;
            assert!(
                descriptors_3a(&mutated).is_err(),
                "{} required mutation passed",
                entry.name
            );
            let mut mutated = entries.clone();
            mutated[index].parameters[0].parameter_type = SlashParameterType::String;
            if entry.parameters[0].parameter_type != SlashParameterType::String {
                assert!(
                    descriptors_3a(&mutated).is_err(),
                    "{} type mutation passed",
                    entry.name
                );
            }
        }
    }
}

#[test]
fn all_forms_are_disabled_with_specific_reasons_and_no_fabricated_inverse() {
    let descriptors = descriptors_3a(&slice()).unwrap();
    for descriptor in &descriptors {
        assert_eq!(descriptor.channel, CommandChannel::Slash);
        assert_eq!(descriptor.availability, Availability::Unsupported);
        assert!(descriptor.manual_pending);
        assert!(descriptor
            .disposition_reason
            .as_ref()
            .is_some_and(|s| s.len() > 24));
        assert!(descriptor.args.iter().all(|arg| arg.default.is_none()));
        assert!(
            descriptor.verify.is_none(),
            "{} claims unavailable readback",
            descriptor.id
        );
        assert_eq!(descriptor.compensation, Some(Compensation::None));
        assert_eq!(descriptor.target.selection_rule, SelectionRule::None);
        assert!(descriptor.target.argument_name.is_none());
    }
    assert_eq!(
        descriptors
            .iter()
            .map(|d| d.disposition_reason.as_ref().unwrap())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        27,
    );
}

#[test]
fn cover_scope_and_pvp_authority_remain_unproved() {
    let descriptors = descriptors_3a(&slice()).unwrap();
    for cover in &descriptors[..3] {
        assert_eq!(cover.target.kind, TargetKind::None);
        assert_eq!(
            cover.impact.as_ref().unwrap().kind,
            ImpactKind::Irreversible
        );
        assert!(
            cover.summary.contains("world/space")
                || cover.impact.as_ref().unwrap().preconditions[1].contains("world/space")
        );
        assert_eq!(cover.compensation, Some(Compensation::None));
    }
    assert!(descriptors[0]
        .disposition_reason
        .as_ref()
        .unwrap()
        .contains("bounded work"));
    assert!(descriptors[2]
        .disposition_reason
        .as_ref()
        .unwrap()
        .contains("WSTRING"));
    let pvp = &descriptors[26];
    assert_eq!(pvp.target.kind, TargetKind::Player);
    assert!(pvp.summary.contains("never a grant of attack rights"));
    assert!(pvp.impact.is_none());
    assert!(pvp
        .disposition_reason
        .as_ref()
        .unwrap()
        .contains("duel registry"));
    assert!(pvp.args[0].required);
}

#[test]
fn optional_toggles_and_candidate_handlers_do_not_borrow_dot_or_selection() {
    let descriptors = descriptors_3a(&slice()).unwrap();
    assert!(descriptors[16..19].iter().all(|d| !d.args[0].required));
    assert!(descriptors[20..26].iter().all(|d| !d.args[0].required));
    assert!(descriptors[19].args.is_empty());
    assert!(descriptors[6].args.is_empty());
    assert!(descriptors[6]
        .disposition_reason
        .as_ref()
        .unwrap()
        .contains("dot"));
    assert!(descriptors[17]
        .disposition_reason
        .as_ref()
        .unwrap()
        .contains("dot"));
    for index in [14, 22, 23, 26] {
        assert_eq!(descriptors[index].target.kind, TargetKind::Player);
        assert_eq!(
            descriptors[index].target.selection_rule,
            SelectionRule::None
        );
    }
    for index in [22, 23] {
        assert_eq!(
            descriptors[index].impact.as_ref().unwrap().kind,
            ImpactKind::Transient
        );
        assert!(descriptors[index]
            .disposition_reason
            .as_ref()
            .unwrap()
            .contains("flag readback"));
    }
    assert!(descriptors[14].verify.is_none());
    assert!(descriptors[14]
        .disposition_reason
        .as_ref()
        .unwrap()
        .contains("unimplemented"));
    assert_eq!(
        descriptors[5].args[0].range.as_ref().unwrap().max,
        Some(i32::MAX as f64)
    );
}
