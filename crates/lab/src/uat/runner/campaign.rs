//! Campaign identity and row dependency rules shared across run/resume.

use std::collections::HashMap;

use serde_json::json;

use super::session;
use super::LoadedSpec;
use crate::uat::evidence::RunManifest;
use crate::uat::spec::{RowSpec, SectionSpec};

/// Reserve section-specific fresh identities in run.json before any call can
/// create a character. A resumed run keeps the saved identity even if the
/// caller supplied sections in a different order.
pub(super) fn prepare_fresh(
    sections: &[LoadedSpec],
    manifest: &mut RunManifest,
    resumed: bool,
) -> Result<HashMap<String, String>, String> {
    let mut fresh = HashMap::new();
    let legacy_identity = resumed && manifest.fresh_characters.is_empty();
    for loaded in sections {
        let section = &loaded.spec.section;
        if section.character != "fresh" {
            continue;
        }
        let archetype = &section
            .fresh
            .as_ref()
            .ok_or_else(|| {
                format!(
                    "section {}: fresh character has no [section.fresh]",
                    section.id
                )
            })?
            .archetype;
        let name = match manifest.fresh_characters.get(&section.id) {
            Some(record) => {
                if record["archetype"] != json!(archetype) {
                    return Err(format!(
                        "section {}: saved fresh archetype differs from spec",
                        section.id
                    ));
                }
                record["name"]
                    .as_str()
                    .ok_or_else(|| {
                        format!("section {}: saved fresh character has no name", section.id)
                    })?
                    .to_string()
            }
            None if legacy_identity && fresh.is_empty() => session::legacy_name(&manifest.run_id),
            None => {
                let mut nonce = 0;
                loop {
                    let candidate = session::fresh_name(&manifest.run_id, &section.id, nonce);
                    if !manifest
                        .fresh_characters
                        .values()
                        .any(|v| v["name"] == candidate)
                    {
                        break candidate;
                    }
                    nonce += 1;
                }
            }
        };
        let mut record = json!({ "name": name, "archetype": archetype });
        if manifest
            .fresh_characters
            .get(&section.id)
            .and_then(|saved| saved["intro_pending"].as_bool())
            == Some(true)
        {
            record["intro_pending"] = json!(true);
        }
        manifest.fresh_characters.insert(section.id.clone(), record);
        fresh.insert(section.id.clone(), name);
    }
    Ok(fresh)
}

/// A sequential section stops only on a previous FAIL/BLOCKED; pending
/// evidence and native shortfalls still allow the player to advance.
pub(super) fn blocks(spec: &SectionSpec, row: &RowSpec, manifest: &RunManifest) -> Vec<String> {
    let mut reasons = Vec::new();
    for prior in spec.rows.iter().take_while(|r| r.id != row.id) {
        if !spec.section.sequential && !row.depends_on.contains(&prior.id) {
            continue;
        }
        let key = format!("{}/{}", spec.section.id, prior.id);
        let result = manifest
            .rows
            .iter()
            .find(|entry| entry["row"] == key)
            .and_then(|entry| entry["result"].as_str());
        if matches!(result, Some("FAIL" | "BLOCKED"))
            || (row.depends_on.contains(&prior.id) && result.is_none())
        {
            reasons.push(format!(
                "prior row {} = {}",
                prior.id,
                result.unwrap_or("NOT_RUN")
            ));
            if spec.section.sequential {
                break;
            }
        }
    }
    reasons
}
