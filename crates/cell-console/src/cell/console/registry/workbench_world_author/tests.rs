use std::collections::BTreeSet;

use super::*;
use crate::cell::console::{command_catalog, registry::commands};

#[test]
fn reviewed_inventory_equals_both_registered_families() {
    let expected: BTreeSet<_> = commands::patrol::SPECS
        .iter()
        .chain(commands::entity_authoring::SPECS)
        .map(|spec| spec.name.to_string())
        .collect();
    assert_eq!(expected.len(), 28);
    let actual: BTreeSet<_> = command_catalog()
        .into_iter()
        .filter(|row| metadata(&row.name).is_some())
        .map(|row| row.name)
        .collect();
    assert_eq!(actual, expected);
    for name in expected {
        let row = metadata(&name).unwrap();
        assert!(!row.forms.is_empty(), ".{name}");
        assert!(!row.preconditions.is_empty(), ".{name}");
        assert!(!row.impact.is_empty(), ".{name}");
        assert!(!row.verify.is_empty(), ".{name}");
        assert!(!row.compensation.is_empty(), ".{name}");
        assert!(!row.verify_tools.is_empty(), ".{name}");
        assert!(!row.comparison_fields.is_empty(), ".{name}");
        if row.availability != "runnable" {
            assert!(!row.disposition_reason.is_empty(), ".{name}");
        }
    }
}

type ExpectedArg = (
    &'static str,
    &'static str,
    bool,
    Option<f64>,
    Option<f64>,
    &'static [&'static str],
);

const SIGNED_PATH: ExpectedArg = (
    "pathId",
    "integer",
    true,
    Some(i32::MIN as f64),
    Some(i32::MAX as f64),
    &[],
);
const INDEX: ExpectedArg = (
    "index",
    "integer",
    true,
    Some(0.0),
    Some(i32::MAX as f64),
    &[],
);
const SEQUENCE: ExpectedArg = (
    "seqId",
    "integer",
    true,
    Some(i32::MIN as f64),
    Some(i32::MAX as f64),
    &[],
);
const DELAY_OPTIONAL: ExpectedArg = ("delay", "float", false, None, None, &[]);
const DELAY_REQUIRED: ExpectedArg = ("delay", "float", true, None, None, &[]);
const BIT: ExpectedArg = ("bit", "integer", true, Some(0.0), Some(31.0), &[]);
const TEMPLATE: ExpectedArg = (
    "templateId",
    "template",
    true,
    Some(i32::MIN as f64),
    Some(i32::MAX as f64),
    &[],
);
const SET_MAP: ExpectedArg = (
    "setMapId",
    "integer",
    true,
    Some(i32::MIN as f64),
    Some(i32::MAX as f64),
    &[],
);

const EXPECTED: &[(&str, &str, &[ExpectedArg])] = &[
    ("path_add", "none", &[SIGNED_PATH]),
    ("path_show", "none", &[SIGNED_PATH]),
    ("path_clear", "none", &[SIGNED_PATH]),
    ("path_assign", "mob", &[SIGNED_PATH, DELAY_OPTIONAL]),
    ("path_unassign", "mob", &[]),
    ("path_set_seq", "none", &[SIGNED_PATH, INDEX, SEQUENCE]),
    ("path_clear_seq", "none", &[SIGNED_PATH, INDEX]),
    ("path_set_tp", "none", &[SIGNED_PATH, INDEX]),
    ("path_clear_tp", "none", &[SIGNED_PATH, INDEX]),
    ("path_set_tp_seq", "none", &[SIGNED_PATH, INDEX, SEQUENCE]),
    (
        "path_set_tp_delay",
        "none",
        &[SIGNED_PATH, INDEX, DELAY_REQUIRED],
    ),
    (
        "tag",
        "spawnable",
        &[("tag", "text", true, None, None, &[])],
    ),
    (
        "name",
        "being",
        &[("nameWords", "text", true, None, None, &[])],
    ),
    (
        "alignment",
        "being",
        &[(
            "alignment",
            "enum",
            true,
            None,
            None,
            &["undefined", "praxis", "sgu"],
        )],
    ),
    (
        "nameid",
        "spawnable",
        &[(
            "nameId",
            "integer",
            true,
            Some(i32::MIN as f64),
            Some(i32::MAX as f64),
            &[],
        )],
    ),
    (
        "staticmesh",
        "spawnable",
        &[("mesh", "text", true, None, None, &[])],
    ),
    (
        "bodyset",
        "spawnable",
        &[("bodySet", "text", true, None, None, &[])],
    ),
    (
        "eventset",
        "spawnable",
        &[(
            "eventSetId",
            "integer",
            true,
            Some(i32::MIN as f64),
            Some(i32::MAX as f64),
            &[],
        )],
    ),
    (
        "interactiontype",
        "spawnable",
        &[(
            "flags",
            "integer",
            true,
            Some(i32::MIN as f64),
            Some(i32::MAX as f64),
            &[],
        )],
    ),
    ("lookat", "spawnable", &[]),
    (
        "visible",
        "spawnable",
        &[(
            "visible",
            "boolean",
            true,
            None,
            None,
            &["0", "1", "false", "true"],
        )],
    ),
    ("setcombatant", "being", &[BIT]),
    ("unsetcombatant", "being", &[BIT]),
    (
        "addcomponent",
        "being",
        &[("component", "text", true, None, None, &[])],
    ),
    (
        "delcomponent",
        "being",
        &[("component", "text", true, None, None, &[])],
    ),
    ("adddialog", "spawnable", &[TEMPLATE, SET_MAP]),
    ("removedialog", "spawnable", &[TEMPLATE, SET_MAP]),
    ("dynamicupdate", "spawnable", &[]),
];

#[test]
fn every_world_author_form_has_exact_target_and_ordered_typed_signature() {
    let catalog = command_catalog();
    assert_eq!(EXPECTED.len(), 28);
    for (name, target, expected_args) in EXPECTED {
        let row = catalog.iter().find(|row| row.name == *name).unwrap();
        assert_eq!(row.target_policy, *target, ".{name}");
        let actual: Vec<_> = row
            .args
            .iter()
            .map(|arg| {
                (
                    arg.name,
                    arg.kind,
                    arg.required,
                    arg.min,
                    arg.max,
                    arg.choices,
                )
            })
            .collect();
        assert_eq!(actual.as_slice(), *expected_args, ".{name}");
    }
    let names: BTreeSet<_> = EXPECTED.iter().map(|(name, _, _)| *name).collect();
    let family_names: BTreeSet<_> = commands::patrol::SPECS
        .iter()
        .chain(commands::entity_authoring::SPECS)
        .map(|spec| spec.name)
        .collect();
    assert_eq!(names, family_names);
}

#[test]
fn only_session_buffer_read_and_transient_tag_are_runnable() {
    let runnable: Vec<_> = command_catalog()
        .into_iter()
        .filter(|row| metadata(&row.name).is_some() && row.availability == "runnable")
        .map(|row| row.name)
        .collect();
    assert_eq!(runnable, ["tag", "path_show"]);
    assert_eq!(metadata("path_show").unwrap().impact_kind, "read_only");
    assert_eq!(metadata("tag").unwrap().impact_kind, "transient");
    let name = metadata("name").unwrap();
    assert_eq!(name.availability, "unsupported");
    assert!(name.verify.contains("player readback unavailable"));
    let lookat = metadata("lookat").unwrap();
    assert_eq!(lookat.availability, "unsupported");
    assert!(lookat.impact.contains("any non-player"));
    assert!(lookat.impact.contains("seed_commit_required"));
}

#[test]
fn ordinal_edits_are_disabled_and_pin_confirmation_time_identity_gap() {
    for name in [
        "path_set_seq",
        "path_clear_seq",
        "path_set_tp",
        "path_clear_tp",
        "path_set_tp_seq",
        "path_set_tp_delay",
    ] {
        let row = metadata(name).unwrap();
        assert_eq!(row.availability, "unsupported", ".{name}");
        assert!(
            row.disposition_reason.contains("ordinal row can change"),
            ".{name}"
        );
        assert!(
            row.preconditions
                .contains("point_id/world ownership pinned server-side"),
            ".{name}"
        );
        assert_eq!(row.args[1].name, "index");
        assert_eq!(row.args[1].min, Some(0.0));
    }
    assert!(metadata("path_clear_tp")
        .unwrap()
        .forms
        .contains("all five"));
    assert!(metadata("path_set_tp_seq")
        .unwrap()
        .verify
        .contains("shared path_seq label"));
}

#[test]
fn authoring_receipts_never_claim_a_committed_seed() {
    for row in command_catalog()
        .into_iter()
        .filter(|row| metadata(&row.name).is_some())
    {
        assert!(!row.impact.contains("committed seed"), ".{}", row.name);
        assert!(!row.verify.contains("committed seed"), ".{}", row.name);
        if row.impact_kind == "authoring" {
            assert!(row.impact.contains("seed_commit_required"), ".{}", row.name);
            assert!(
                row.verify_tools.contains(&"server_db_query"),
                ".{}",
                row.name
            );
        }
    }
    let path_add = metadata("path_add").unwrap();
    assert!(path_add.disposition_reason.contains("per-GM SQL queue"));
    assert!(path_add.verify.contains("world_id"));
    assert!(metadata("path_clear")
        .unwrap()
        .verify
        .contains("both base results"));
}
