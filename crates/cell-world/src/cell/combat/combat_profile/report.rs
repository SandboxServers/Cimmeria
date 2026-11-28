//! Diagnostics for resolved NPC combat profiles: one row per template when
//! the 592 placeholder or an unresolved profile is applied, and nothing for
//! an explicit ability set. The event names are a telemetry contract
//! (NC-44 greps for them), so do not rename them.

use super::{first_report, CombatProfileSource, NpcCombatProfile, WeaponEvidence};
use crate::cell::combat::NPC_DEFAULT_ABILITY;

/// Log how a template's combat profile was resolved. Placeholder and
/// unresolved profiles log once per template id for the process lifetime.
pub fn report_profile(
    template_id: i32,
    template_name: &str,
    body_set: &str,
    profile: &NpcCombatProfile,
) {
    let weapon_component = match &profile.weapon_evidence {
        WeaponEvidence::Component(component) => component.as_str(),
        WeaponEvidence::None => "none",
    };
    match profile.source {
        CombatProfileSource::ExplicitSet => {}
        CombatProfileSource::LegacyPlaceholder => {
            if first_report(template_id) {
                tracing::info!(
                    target: "npc.combat_profile",
                    event = "combat_profile_placeholder",
                    template_id,
                    template_name,
                    body_set,
                    weapon_component,
                    ability_id = NPC_DEFAULT_ABILITY,
                    ability_name = "Pistol Shot",
                    "NPC template has no ability set; using the Pistol Shot placeholder"
                );
            }
        }
        CombatProfileSource::Unresolved => {
            if first_report(template_id) {
                tracing::warn!(
                    target: "npc.combat_profile",
                    event = "combat_profile_unresolved",
                    template_id,
                    template_name,
                    body_set,
                    weapon_component,
                    "NPC template has no ability set and no weapon evidence; it holds fire"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{resolve_npc_combat_profile, PlaceholderPolicy};
    use super::*;
    use crate::cell::spawner::SpawnRecord;
    use crate::test_support::{make_space_manager, Captured, LogCapture, LogCaptureGuard};
    use tracing::Level;

    /// Sentinel template ids (R7), unique per test because the once-per-template
    /// set is process-wide.
    const PLACEHOLDER_ID: i32 = 0x7000_0205;
    const HUMAN_ID: i32 = 0x7000_0206;
    const UNRESOLVED_ID: i32 = 0x7000_0202;
    const EXPLICIT_ID: i32 = 0x7000_0203;

    /// A spawn record for the Agnos startup space with the given template.
    fn record(template_id: i32, ability_ids: Vec<i32>) -> SpawnRecord {
        SpawnRecord {
            spawn_id: 1,
            world_name: "Agnos".to_string(),
            x: 10.0,
            y: 0.0,
            z: 20.0,
            heading: 1.57,
            tag: None,
            template_id,
            template_name: "Sentinel Placeholder NPC".to_string(),
            class: "mob".to_string(),
            static_mesh: Some("Props.TestMesh".to_string()),
            body_set: "BS_Sentinel".to_string(),
            components: None,
            flags: 0,
            interaction_type: 0,
            event_set_id: None,
            level: Some(5),
            alignment: Some(0),
            faction: Some(1),
            name_id: Some(7031),
            speaker_id: None,
            static_interaction_sets: vec![],
            has_dynamic_properties: true,
            loot_table_id: None,
            is_stationary: false,
            ability_ids,
            respawn_secs: None,
            patrol_path: vec![],
            patrol_point_delay_secs: 2.0,
            wander_radius: 0.0,
            wander_min_dwell_secs: 3.0,
            wander_max_dwell_secs: 8.0,
            follow_min_distance: 2.0,
            follow_max_distance: 5.0,
            move_speed: 0.6,
            leash_distance: None,
            aggro_radius: None,
            assist_radius: None,
            aggression_override: None,
            use_cover: None,
            vault_scope: cimmeria_entity::cell_entity::VaultScope::Personal,
            training_dummy: false,
            display_name: None,
            tint: None,
        }
    }

    /// The `npc.combat_profile` rows carrying `event`.
    fn profile_rows(logs: &LogCaptureGuard, event: &str) -> Vec<Captured> {
        logs.all()
            .into_iter()
            .filter(|c| c.target == "npc.combat_profile" && c.has_field("event", event))
            .collect()
    }

    /// Spawn-path guard: a template with no ability set and no weapon
    /// evidence reports once, however many NPCs of the template spawn, and
    /// no placeholder row is logged (D-NC1: no 592 fallback).
    #[test]
    fn unresolved_profile_logs_once_per_template_through_spawn() {
        let logs = LogCapture::install();
        let mut mgr = make_space_manager();
        let rec = record(PLACEHOLDER_ID, vec![]);

        let first = mgr.allocate_npc_id();
        mgr.spawn_npc_from_record(first, &rec).unwrap();
        let rows = profile_rows(&logs, "combat_profile_unresolved");
        assert_eq!(rows.len(), 1, "one unresolved row: {rows:#?}");
        let row = &rows[0];
        assert_eq!(row.level, Level::WARN);
        assert!(row.has_field("template_id", &PLACEHOLDER_ID.to_string()));
        assert!(row.has_field("template_name", "Sentinel Placeholder NPC"));
        assert!(row.has_field("weapon_component", "none"));
        assert!(profile_rows(&logs, "combat_profile_placeholder").is_empty());

        let second = mgr.allocate_npc_id();
        mgr.spawn_npc_from_record(second, &rec).unwrap();
        assert_eq!(
            profile_rows(&logs, "combat_profile_unresolved").len(),
            1,
            "a second spawn of the same template must log nothing"
        );
    }

    /// Spawn-path guard: a human firearm template with no ability set gets the
    /// 592 placeholder and logs one INFO row, however many NPCs spawn.
    #[test]
    fn human_firearm_placeholder_logs_once_per_template_through_spawn() {
        let logs = LogCapture::install();
        let mut mgr = make_space_manager();
        let mut rec = record(HUMAN_ID, vec![]);
        rec.components = Some(vec!["WP-Human.WP_Pistol_01".to_string()]);

        let first = mgr.allocate_npc_id();
        mgr.spawn_npc_from_record(first, &rec).unwrap();
        let rows = profile_rows(&logs, "combat_profile_placeholder");
        assert_eq!(rows.len(), 1, "one placeholder row: {rows:#?}");
        let row = &rows[0];
        assert_eq!(row.level, Level::INFO);
        assert!(row.has_field("template_id", &HUMAN_ID.to_string()));
        assert!(row.has_field("weapon_component", "WP-Human.WP_Pistol_01"));
        assert!(row.has_field("ability_id", "592"));
        assert!(profile_rows(&logs, "combat_profile_unresolved").is_empty());

        let second = mgr.allocate_npc_id();
        mgr.spawn_npc_from_record(second, &rec).unwrap();
        assert_eq!(
            profile_rows(&logs, "combat_profile_placeholder").len(),
            1,
            "a second spawn of the same template must log nothing"
        );
    }

    /// An unresolved profile logs one WARN, naming the template.
    #[test]
    fn unresolved_profile_warns_once() {
        let logs = LogCapture::install();
        let components = vec!["WP-Jaffa.WP_Staff_Plasma_4A".to_string()];
        let profile = resolve_npc_combat_profile(
            &[],
            Some(&components),
            PlaceholderPolicy::RequireWeaponEvidence,
        );

        report_profile(
            UNRESOLVED_ID,
            "Sentinel Unresolved",
            "BS_Sentinel",
            &profile,
        );
        report_profile(
            UNRESOLVED_ID,
            "Sentinel Unresolved",
            "BS_Sentinel",
            &profile,
        );

        let rows = profile_rows(&logs, "combat_profile_unresolved");
        assert_eq!(rows.len(), 1, "one unresolved row: {rows:#?}");
        assert_eq!(rows[0].level, Level::WARN);
        assert!(rows[0].has_field("template_id", &UNRESOLVED_ID.to_string()));
        assert!(rows[0].has_field("weapon_component", "WP-Jaffa.WP_Staff_Plasma_4A"));
    }

    /// An explicit ability set logs nothing.
    #[test]
    fn explicit_profile_logs_nothing() {
        let logs = LogCapture::install();
        let profile = resolve_npc_combat_profile(&[584, 710], None, PlaceholderPolicy::Legacy);

        report_profile(EXPLICIT_ID, "Sentinel Explicit", "BS_Sentinel", &profile);

        assert!(profile_rows(&logs, "combat_profile_placeholder").is_empty());
        assert!(profile_rows(&logs, "combat_profile_unresolved").is_empty());
    }
}
