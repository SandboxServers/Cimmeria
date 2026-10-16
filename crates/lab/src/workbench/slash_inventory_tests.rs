//! Regression guards for the canonical slash roster and disabled descriptors.

use std::collections::HashSet;

use sha2::{Digest, Sha256};

use super::commands::{ArgKind, Availability, CommandChannel};
use super::slash_catalog::{patch_slash_catalog, stock_addendum, StockInventoryState};
use super::slash_inventory::{parse_patch_table, SlashParameterType};

const SOURCE: &str =
    include_str!("../../../../data/client-patches/012-gm-slash-commands/commands.toml");
const ROSTER_SHA256: &str = "22be088eccb01db3c9884ff033750fd5bc3ee9fbbe26815cfb3b7e06bdc242fb";

#[test]
fn patch_roster_is_exact_and_has_no_duplicates() {
    let catalog = patch_slash_catalog().unwrap();
    assert_eq!(catalog.len(), 162);
    assert_eq!(
        catalog
            .iter()
            .filter(|row| row.source.name.starts_with("/gm"))
            .count(),
        161
    );
    assert_eq!(
        catalog
            .iter()
            .filter(|row| row.source.name == "/debugminigamecomplete")
            .count(),
        1
    );
    let names: HashSet<_> = catalog.iter().map(|row| row.source.name.as_str()).collect();
    assert_eq!(names.len(), 162);
    let mut names: Vec<_> = names.into_iter().collect();
    names.sort_unstable();
    let digest = Sha256::digest(names.join("\n").as_bytes());
    let actual: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(actual, ROSTER_SHA256);
}

#[test]
fn mandatory_optional_order_and_types_survive_projection() {
    let source: toml::Value = toml::from_str(SOURCE).unwrap();
    let source_rows = source["command"].as_array().unwrap();
    let catalog = patch_slash_catalog().unwrap();
    assert_eq!(source_rows.len(), catalog.len());
    for (raw, row) in source_rows.iter().zip(catalog.iter()) {
        assert_eq!(raw["name"].as_str(), Some(row.source.name.as_str()));
        assert_eq!(raw["event"].as_str(), Some(row.source.event.as_str()));
        let expected: Vec<_> = ["params", "optional"]
            .into_iter()
            .flat_map(|key| {
                raw.get(key)
                    .and_then(toml::Value::as_array)
                    .into_iter()
                    .flatten()
                    .map(move |text| (text.as_str().unwrap(), key == "params"))
            })
            .collect();
        assert_eq!(row.source.parameters.len(), expected.len());
        assert_eq!(row.descriptor.args.len(), expected.len());
        for ((declaration, required), (parsed, arg)) in expected
            .iter()
            .zip(row.source.parameters.iter().zip(&row.descriptor.args))
        {
            let (kind, name) = declaration.split_once(' ').unwrap();
            assert_eq!(parsed.name, name);
            assert_eq!(parsed.required, *required);
            assert_eq!(arg.name, name);
            assert_eq!(arg.required, *required);
            assert_eq!(
                (parsed.parameter_type, arg.kind),
                match kind {
                    "Boolean" => (SlashParameterType::Boolean, ArgKind::Boolean),
                    "Integer" => (SlashParameterType::Integer, ArgKind::Integer),
                    "Float" => (SlashParameterType::Float, ArgKind::Float),
                    "String" => (SlashParameterType::String, ArgKind::Text),
                    _ => panic!("unhandled patch type: {kind}"),
                }
            );
        }
    }
}

#[test]
fn all_patch_descriptors_are_disabled_until_native_submission_is_proven() {
    let catalog = patch_slash_catalog().unwrap();
    for row in catalog {
        assert_eq!(row.descriptor.channel, CommandChannel::Slash);
        assert_eq!(row.descriptor.availability, Availability::Unsupported);
        assert_eq!(
            row.descriptor.disposition_reason.as_deref(),
            Some("native submission unproved")
        );
        assert!(row.descriptor.manual_pending);
        assert!(row.descriptor.impact.is_none());
        assert!(row.descriptor.verify.is_none());
        assert!(row.descriptor.compensation.is_none());
        if let Some(method) = row.method_candidate {
            assert!((109..=225).contains(&method.index));
            assert!(!method.binding_verified);
            assert!(matches!(method.status.as_str(), "DONE" | "ADAPT" | "NEW"));
        }
    }
}

#[test]
fn method_status_is_a_documented_candidate_not_a_verified_event_binding() {
    let catalog = patch_slash_catalog().unwrap();
    let mission = catalog
        .iter()
        .find(|row| row.source.name == "/gmmissionassign")
        .unwrap();
    let candidate = mission.method_candidate.as_ref().unwrap();
    assert_eq!(
        (
            candidate.index,
            candidate.name.as_str(),
            candidate.status.as_str()
        ),
        (109, "gmMissionAssign", "DONE")
    );
    assert!(!candidate.binding_verified);
    let debug = catalog
        .iter()
        .find(|row| row.source.name == "/debugminigamecomplete")
        .unwrap();
    assert!(debug.method_candidate.is_none());
    let client_only = catalog
        .iter()
        .find(|row| row.source.name == "/gmforceclientcrash")
        .unwrap();
    assert!(client_only.method_candidate.is_none());
}

#[test]
fn explicit_and_semantic_slash_candidates_keep_indices_without_claiming_binding() {
    let catalog = patch_slash_catalog().unwrap();
    for (word, event, index, name) in [
        (
            "/gmgivecash",
            "Event_SlashCmd_GiveNaqahdah",
            134,
            "gmGiveCash",
        ),
        ("/gmspawnbycmd", "Event_SlashCmd_Spawn", 185, "gmSpawnByCmd"),
        ("/gmsetfly", "Event_SlashCmd_SetFly", 221, "onPhysics"),
        ("/gmsetghost", "Event_SlashCmd_SetGhost", 221, "onPhysics"),
        (
            "/gmsendgmshout",
            "Event_SlashCmd_GMShout",
            222,
            "sendGMShout",
        ),
    ] {
        let row = catalog.iter().find(|row| row.source.name == word).unwrap();
        assert_eq!(row.source.event, event);
        let candidate = row.method_candidate.as_ref().unwrap();
        assert_eq!(
            (
                candidate.index,
                candidate.name.as_str(),
                candidate.status.as_str()
            ),
            (index, name, "DONE")
        );
        assert!(!candidate.binding_verified);
    }
    // The dispatch table names both gmDespawnByCmd and despawnMob, but does
    // not bind /gmdespawn to either one; an arbitrary choice would mislead.
    let despawn = catalog
        .iter()
        .find(|row| row.source.name == "/gmdespawn")
        .unwrap();
    assert!(despawn.method_candidate.is_none());
}

#[test]
fn malformed_or_duplicate_patch_entries_are_refused() {
    assert!(parse_patch_table(SOURCE).is_ok());
    assert!(parse_patch_table(
        &SOURCE.replace("name = \"/gmmissionclear\"", "name = \"/gmmissionassign\"")
    )
    .is_err());
    assert!(parse_patch_table(
        &SOURCE.replace("Boolean DisplayAcceptWindow", "Number DisplayAcceptWindow")
    )
    .is_err());
    assert!(parse_patch_table(
        &SOURCE.replace("Event_SlashCmd_MissionAssign", "Event_MissionAssign")
    )
    .is_err());
}

#[test]
fn inspected_stock_xml_does_not_silently_drop_the_role_marked_candidate() {
    let audit = stock_addendum().unwrap();
    assert_eq!(audit.stock_inventory, StockInventoryState::Complete);
    assert!(audit.missing_sources.is_empty());
    assert!(audit.verified_extras.is_empty());
    let stock_sources: HashSet<_> = audit
        .inspected_sources
        .iter()
        .filter(|source| source.starts_with("stock "))
        .map(String::as_str)
        .collect();
    let expected: HashSet<_> = [
        "stock Client/game/Common/xml/slash_commands/SlashCommands.xml sha256=6e34355a628bd3b5f95e31ef58e7a2e9700dcce6b9798c751bda2431ba0dcd66 commands=98",
        "stock Client/game/Common/xml/slash_commands/FinalSlashCommands.xml sha256=7d166f89d3b6829d2fe6c891c90104c5562d1de653864b10d7dd06fbf97263db commands=4",
        "stock Stargate Worlds-QA/Common/xml/slash_commands/SlashCommands.xml sha256=6e34355a628bd3b5f95e31ef58e7a2e9700dcce6b9798c751bda2431ba0dcd66 commands=98",
        "stock Stargate Worlds-QA/Common/xml/slash_commands/FinalSlashCommands.xml sha256=7d166f89d3b6829d2fe6c891c90104c5562d1de653864b10d7dd06fbf97263db commands=4",
    ]
    .into_iter()
    .collect();
    assert_eq!(stock_sources, expected);
    assert_eq!(
        audit
            .exclusions
            .iter()
            .filter(|reason| reason.contains("/reload") && reason.contains("Access=qd"))
            .count(),
        1
    );
    assert!(audit.exclusions.iter().any(|reason| {
        reason.contains("/reload")
            && reason.contains("role-marked")
            && reason.contains("not a verified GM")
    }));
    assert!(!parse_patch_table(SOURCE)
        .unwrap()
        .iter()
        .any(|entry| entry.name == "/reload"));
    assert!(audit
        .exclusions
        .iter()
        .any(|reason| reason.contains("/debugperformance") && reason.contains("no Access")));
    assert!(audit
        .exclusions
        .iter()
        .any(|reason| reason.contains("/worldreset") && reason.contains("Access")));
}
