//! Exact combat feedback and broad entity-kind regression guards for slash slice 2a.

use std::collections::BTreeMap;

use serde_json::json;

use super::commands::{
    validate_command, ActorSnapshot, Availability, CommandAction, Refusal, TargetKind,
};
use super::slash_descriptors_2a::slash_descriptors_2a;

#[test]
fn feedback_prefixes_match_exact_command_and_reject_colliding_longer_names() {
    let forms = slash_descriptors_2a().unwrap();
    let cases = [
        (
            "/gmdebugability",
            "gmDebugAbility:",
            "gmDebugAbilityOnMob: enabled",
        ),
        (
            "/gmdebugcombat",
            "gmDebugCombat:",
            "gmDebugCombatVerbose: enabled",
        ),
        (
            "/gmdebugcombatverbose",
            "gmDebugCombatVerbose:",
            "gmDebugCombat: enabled",
        ),
        ("/gmdebugheal", "gmDebugHeal:", "gmDebugCombat: enabled"),
        (
            "/gmdebugabilityonmob",
            "gmDebugAbilityOnMob:",
            "gmDebugAbility: enabled",
        ),
    ];
    for (word, prefix, collision) in cases {
        let form = forms.iter().find(|form| form.source.name == word).unwrap();
        let recipe = form.descriptor.verify.as_ref().unwrap();
        assert_eq!(recipe.read_tools, ["client_chat_log"]);
        assert!(recipe
            .comparison_fields
            .contains(&format!("prefix={prefix}")));
        assert!(format!("{prefix} enabled").starts_with(prefix));
        assert!(
            !collision.starts_with(prefix),
            "{word} accepted {collision}"
        );
    }
}

#[test]
fn mob_data_accepts_fresh_same_instance_prop_but_not_unknown_or_stale_target() {
    let forms = slash_descriptors_2a().unwrap();
    let form = forms
        .iter()
        .find(|form| form.source.name == "/gmmobdata")
        .unwrap();
    assert_eq!(form.descriptor.target.kind, TargetKind::AnyEntity);
    assert!(form.target_scope.contains("caller space"));
    let mut descriptor = form.descriptor.clone();
    descriptor.availability = Availability::Runnable;
    descriptor.manual_pending = false;
    let action = CommandAction {
        descriptor_id: "/gmmobdata".into(),
        instance: "lab-a".into(),
        actor_entity_id: 10,
        selected_entity_id: None,
        typed_args: BTreeMap::from([("SpaceId".into(), json!(1)), ("TargetId".into(), json!(30))]),
        idempotency_key: "review-prop".into(),
    };
    let mut actor = ActorSnapshot {
        instance: "lab-a".into(),
        entity_id: 10,
        is_gm: true,
        selected_entity_id: None,
        selected_target_kind: None,
        argument_target_entity_id: Some(30),
        argument_target_kind: Some(TargetKind::Spawnable),
    };
    assert!(validate_command(&action, &descriptor, &actor).is_ok());
    actor.argument_target_kind = Some(TargetKind::None);
    assert_eq!(
        validate_command(&action, &descriptor, &actor).unwrap_err(),
        Refusal::WrongTargetKind
    );
    actor.argument_target_kind = Some(TargetKind::Spawnable);
    actor.argument_target_entity_id = Some(31);
    assert_eq!(
        validate_command(&action, &descriptor, &actor).unwrap_err(),
        Refusal::StaleTarget
    );
    actor.argument_target_entity_id = Some(30);
    actor.instance = "other-lab".into();
    assert_eq!(
        validate_command(&action, &descriptor, &actor).unwrap_err(),
        Refusal::StaleInstance
    );
}
