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
