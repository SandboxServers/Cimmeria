//! Evidence-backed dot-console contracts for progression, ability, stat, ammo and pet tools.

use super::{
    workbench::WorkbenchMetadata, workbench_world_move::TypedArg, COMMAND_METADATA_VERSION,
};

const fn field(name: &'static str, kind: &'static str, required: bool) -> TypedArg {
    TypedArg {
        name,
        kind,
        required,
        min: None,
        max: None,
        choices: &[],
        help: name,
    }
}

const fn bounded(name: &'static str, required: bool, min: f64, max: f64) -> TypedArg {
    TypedArg {
        name,
        kind: "integer",
        required,
        min: Some(min),
        max: Some(max),
        choices: &[],
        help: name,
    }
}

const fn choice(name: &'static str, required: bool, choices: &'static [&'static str]) -> TypedArg {
    TypedArg {
        name,
        kind: "enum",
        required,
        min: None,
        max: None,
        choices,
        help: name,
    }
}

const NONE: &[TypedArg] = &[];
const DISCIPLINE: &[TypedArg] = &[bounded("disciplineId", true, 1.0, i32::MAX as f64)];
const LEARN_DISCIPLINE: &[TypedArg] = &[
    bounded("disciplineId", true, 1.0, i32::MAX as f64),
    bounded("expertise", false, 1.0, i32::MAX as f64),
];
const BLUEPRINT: &[TypedArg] = &[bounded(
    "blueprintId",
    true,
    i32::MIN as f64,
    i32::MAX as f64,
)];
const CRAFT_KIT: &[TypedArg] = &[
    bounded("blueprintId", true, i32::MIN as f64, i32::MAX as f64),
    bounded("count", false, 1.0, 10.0),
];
const MISSION: &[TypedArg] = &[bounded("missionId", true, i32::MIN as f64, i32::MAX as f64)];
const AMOUNT: &[TypedArg] = &[bounded("amount", true, 1.0, i32::MAX as f64)];
const ABILITY: &[TypedArg] = &[bounded("abilityId", true, i32::MIN as f64, i32::MAX as f64)];
const COOLDOWNS: &[TypedArg] = &[
    choice("action", false, &["reset"]),
    bounded("abilityId", false, 1.0, i32::MAX as f64),
];
const DUMMY: &[TypedArg] = &[
    choice("mode", false, &["hostile", "friendly", "caster", "clear"]),
    bounded("templateOrAbilityId", false, 1.0, i32::MAX as f64),
    bounded("intervalSecs", false, 1.0, 290.0),
];
const SPEED: &[TypedArg] = &[bounded("speed", true, i32::MIN as f64, i32::MAX as f64)];
const AMMO: &[TypedArg] = &[
    field("ammoTypeOrReserveItemId", "text", true),
    bounded("rounds", true, 1.0, i32::MAX as f64),
];
const INFINITE_AMMO: &[TypedArg] = &[choice("state", false, &["on", "off"])];
const PET: &[TypedArg] = &[
    choice(
        "verb",
        true,
        &["summon", "dismiss", "stance", "info", "list"],
    ),
    bounded(
        "templateAbilityOrStanceId",
        false,
        i32::MIN as f64,
        i32::MAX as f64,
    ),
];

type Detail = (
    &'static str,
    &'static str,
    &'static str,
    &'static [TypedArg],
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

/// Return one reviewed family descriptor; unsupported entries stay visible and disabled.
pub(super) fn metadata(name: &str) -> Option<WorkbenchMetadata> {
    let (category, availability, reason, args, forms, preconditions, impact, verify, compensation): Detail =
        match name {
            "allcraft" => ("Progression", "unsupported", "Queued GM and subject entity IDs lack immutable owner fences; craft-anywhere failure can still report success", NONE, "selected player; no arguments",
                "GM; selected player; catalog, crafting row and DB available; capture complete prior crafting state and owner generations",
                "Persistent all-discipline expertise 100, all blueprints and paradigm level 7 grant; separate transient craft-anywhere and UI push can fail",
                "Query exact target player_id in sgw_player and sgw_player_discipline_expertise before/after and after relog; inspect push_failed and owner-bound craft-anywhere state; granted line is unverified delivery",
                "No proven whole-state inverse; manual restore from captured DB state"),
            "learndiscipline" => ("Progression", "unsupported", "Arbitrary positive IDs enter persistent state; Base lacks target-owner fence and caller-bound receipt", LEARN_DISCIPLINE, "disciplineId [expertise=1]; positive i32 delta, clamped expertise 0..100",
                "GM; selected player; catalog discipline validation and immutable owner fence required",
                "Persistent discipline registration and expertise delta; post-commit method 136 and feedback can reach reused subject ID",
                "Query exact player_id discipline_ids and sgw_player_discipline_expertise by discipline_id before/after and after relog; cell +N is queued intent, not commit",
                "Capture old expertise; .forgetdiscipline sets zero but leaves ID known"),
            "forgetdiscipline" => ("Progression", "unsupported", "The handler zeroes expertise but preserves or creates known discipline; Base lacks target-owner fence", DISCIPLINE, "disciplineId: positive i32",
                "GM; selected player; prior known-ID and expertise state; catalog and owner validation required",
                "Persistent expertise -100 clamped to zero, and registers absent discipline ID; does not forget the discipline",
                "Query exact player_id discipline_ids and expertise row before/after and after relog; feedback precedes persistence",
                "Manual restore of prior value and possibly removal of newly registered ID/row"),
            "craftkit" => ("Progression", "unsupported", "Queued GM/subject IDs lack immutable owner fences across DB awaits and inventory sends; multi-item preview absent", CRAFT_KIT, "blueprintId [count=1]; count 1..10; component set 1 only",
                "GM; selected player; catalog blueprint/component set, bag capacity and both owner generations",
                "Atomic persistent component-item inventory grant, possibly merging stacks; no blueprint unlock",
                "Wait for Base result; compare exact target character_id sgw_inventory item_id/type_id/stack_size/container_id/slot_id before/after and after relog; result does not prove client receipt",
                "Manual instance/quantity reconciliation only; no safe inverse after merges"),
            "learnblueprint" => ("Progression", "unsupported", "Queued craft grant has no immutable GM/subject generation or post-await send fence", BLUEPRINT, "blueprintId: signed i32, validated against catalog by Base",
                "GM; selected player; catalog blueprint exists and was unknown; immutable GM and subject owners required",
                "Persistent sorted blueprint_ids addition and known-crafts client push; no component-item grant",
                "Query exact player_id sgw_player.blueprint_ids before/after and after relog; feedback and current entity's client list are not owner-bound proof",
                "No console unlearn; manual restore of captured blueprint list"),
            "missionfail" => ("Progression", "unsupported", "Forced failure has no direct MissionUpdate persistence and no complete before-state inverse", MISSION, "missionId: signed i32",
                "GM; selected player; loaded mission tracker and active mission; capture repeats, objectives and prior status",
                "Cell mission status FAILED and repeats increment, visible update unless hidden; no direct Base persistence message",
                "Inspect cell mission status and exact sgw_mission player_id/mission_id row before/after and after relog; chat success is not durable completion",
                "No proven inverse for repeats, objectives or emitted content events"),
            "missionrewards" => ("Progression", "unsupported", "Handler omits loaded-state guard; empty/loading tracker can falsely say no mission and it does not show rewards", MISSION, "missionId: signed i32",
                "GM; selected player; tracker must be proven loaded before absence is meaningful",
                "Read-only current mission status/step or no-mission line; no reward-set lookup",
                "Compare status/step with loaded tracker and exact sgw_mission row; absent state is unknown until loaded-state proof",
                "None needed"),
            "givecash" => ("Progression", "unsupported", "Base lacks immutable caller/target owner fences for queued grant and post-commit sends", AMOUNT, "amount: positive i32",
                "GM; selected player; exact player_id and owner generations; capture old balance and preview mint impact",
                "Persistent additive naquadah grant; target property and GM feedback can reach reused entity IDs after SQL commit",
                "Query exact target player_id sgw_player.naquadah before/after and after relog; Base feedback proves DB update, not original-session client delivery",
                "No safe inverse after spending or concurrent grants; manual balance reconciliation"),
            "givexp" => ("Progression", "unsupported", "GrantXP lacks target player/session identity; reused entity ID can credit another character and overwrite its in-memory state", AMOUNT, "amount: positive i32; final XP delta may be capped",
                "GM; selected player; immutable target player/generation and post-await owner fence required",
                "Persistent XP, possible level/training-point/Applied Science Point increases and ability-tree unlock; Base resolves current entity holder",
                "Query intended and current player_id exp,level,training_points,applied_science_points before/after and after relog; correlate crafting.asp_earned; feedback cannot attribute original target",
                "No safe inverse after level/point spending; audit both characters on ID reuse"),
            "giveability" => ("Progression", "unsupported", "Base does not recheck original GM owner/access and lacks immutable subject generation before mirror/feedback sends", ABILITY, "abilityId: signed i32, exact abilities catalog membership",
                "GM; selected in-view player else caller; display resolved subject; ability known and not already granted; immutable owners required",
                "Persistent ability plus source_kind=gm provenance; no training-point cost, survives respec; stale mirror may reach relogged session",
                "Query exact player_id sgw_player.abilities and sgw_player_ability_grants before/after and after relog; forwarded event has persisted=false",
                "/gmresetabilities removes every GM grant, not just this one; manual restore from prior provenance"),
            "effects" => ("Abilities", "runnable", "", NONE, "no arguments; selected in-view entity else caller",
                "GM; live subject and ability-state snapshot; missing state reports no ability state",
                "Read-only cell warmup, cooldown, pulse, ledger, flags and stats snapshot; no DB write",
                "Compare line with server_ability_state for exact entity_id at same time; advancing timers can differ; missing snapshot is unknown",
                "None needed"),
            "cooldowns" => ("Abilities", "unsupported", "Reset forms share command-wide availability with list; confirmation and before-read cannot be enforced until per-form gating exists", COOLDOWNS, "no arguments: list | reset: all | reset abilityId: one positive i32 plus moniker groups",
                "GM; caller ability state; case-sensitive reset; confirm both reset forms and capture before-state; no catalog check on ID",
                "List is read-only; reset mutates caller transient cooldown maps and sends client timer clears, even if no timer existed",
                "Compare server_ability_state caller cooldowns before/after and .cooldowns feedback; hotbar sweep needs separate client observation",
                "No exact timer-history inverse; future casts restart cooldowns"),
            "dummy" => ("Abilities", "unsupported", "Caster form can harm caller; command-wide availability cannot disable that form until per-form gating exists", DUMMY, "[] | hostile [templateId] | friendly [templateId] | clear | caster abilityId [intervalSecs=8, 1..290]",
                "GM; caller in space; template cached; max four owned dummies; caster ability active/hostile/reachable and interval fits warmup/cooldown",
                "Static forms spawn transient 1,000,000-health NPC or clear owned dummies; caster launches real hostile ability at caller; no spawnlist write",
                "Capture returned ID and compare server_entity_get template/space/health/faction; check absence after clear with ID-reuse guard; caster needs combat telemetry",
                ".dummy clear removes owned dummies but not damage/effects already caused"),
            "cleareffects" => ("Abilities", "unsupported", "Selected-subject confirmation and before-effect snapshot cannot be enforced by command-wide availability", NONE, "no arguments; selected in-view entity else caller",
                "GM; exact live subject; capture before-state and confirm cleanse-all impact",
                "Strip timed ledger and pulsing effects, run on_remove, restore affected stats and send icon clears; cooldown/warmup unchanged",
                "Compare exact entity server_ability_state ledger/pulses/stats before/after; GM line lists IDs; client icon receipt needs separate observation",
                "No safe inverse of remaining durations or effect scripts; recast manually if appropriate"),
            "stats" | "primarystats" | "speedstats" | "armorstats" | "qrstats" | "absorbstats" | "stealthstats" => stat_read(name),
            "speed" => ("Stats", "unsupported", "Live min/max and two-stat prior-value equality must be checked; command-wide form cannot ensure exact restore", SPEED, "speed: signed i32 within both live movement and rotation stat bounds",
                "GM; selected in-view entity with both speed stats; capture both current/min/max and require equal prior current values for single-command restore",
                "Transient set of movementSpeedMod and rotationSpeedMod current values; dirty stat update to player or NPC witnesses; no DB write",
                "Compare both exact stat IDs in server_entity_get.stats before/after and after movement/relog; feedback is not persistence",
                "Reapply captured value only when prior currents matched; otherwise no automatic inverse"),
            "giveammo" | "gmgiveammo" => ("Ammo", "unsupported", "Base lacks immutable GM/subject owner fences; actual DB reserve item can differ from cell-captured ID; exact carried-bag preview absent", AMMO, "ammoTypeOrReserveItemId rounds; dot alias gmgiveammo; rounds >5000 clamp to 5000",
                "GM; selected in-view player else caller; catalog ammo mapping and exact current item identity; positive rounds; bags may partially fill",
                "Persistent special-ammo reserve stacks in carried bags, possibly partial; does not fill clip; post-commit inventory push can reach reused entity ID",
                "Wait for Base returned/remainder/stack_before/stack_after; server_db_query exact character_id inventory in container_id IN (1,15), truncated=false, item_id/type_id/stack_size/container_id/slot_id row diff and sum before/after/relog; actual DB type_id must match Cell msg.item_id; UI delivery unknown",
                "No safe inverse after merge/use/race; manual inventory reconciliation"),
            "infiniteammo" | "gmsetinfiniteammo" => ("Ammo", "unsupported", "Read-only form is safe but command-wide availability cannot disable toggle; selected-subject pinning and finite-feature readback absent", INFINITE_AMMO, "no arguments: read-only ON/OFF (safe, deferred) | on | off; dot alias gmsetinfiniteammo",
                "GM; selected in-view player else caller; exact player_id; capture prior switch and finite-special feature state",
                "No-arg read only; flag toggles process-global per-character free special-ammo reload, not clip size or DB grant; survives relog until restart",
                "Repeat no-arg read with same subject and inspect feedback; no independent lab snapshot or DB column; controlled reload observes behavior",
                "Restore prior ON/OFF for exact subject; restart clears all characters' switches"),
            "pet" => ("Pet", "unsupported", "Info/list are safe but command-wide availability cannot disable summon/dismiss/stance without owner and replacement preflight", PET, "info | list (read-only, safe, deferred) | summon templateIdOrAbilityId | dismiss | stance 0|1|2; verb-specific arity",
                "GM; caller owns mutation; summon requires live player and cached template, stance owned live pet; info may inspect selected foreign pet",
                "Session-only summon/replacement, dismiss or AI stance change; info/list read current pet state; no relog persistence",
                "Capture pet_id; server_entity_get template/space then .pet info owner/stance; dismissal checks exact IDs absent with reuse guard; client stance receipt separate",
                "Dismiss/replace cannot restore prior health/state/cooldowns; stance can be reapplied to same live pet"),
            _ => return None,
        };

    let read_only = matches!(
        name,
        "effects"
            | "stats"
            | "primarystats"
            | "speedstats"
            | "armorstats"
            | "qrstats"
            | "absorbstats"
            | "stealthstats"
            | "missionrewards"
    );
    let persistent = matches!(
        name,
        "allcraft"
            | "learndiscipline"
            | "forgetdiscipline"
            | "craftkit"
            | "learnblueprint"
            | "givecash"
            | "givexp"
            | "giveability"
            | "giveammo"
            | "gmgiveammo"
    );
    let verify_tools: &[&str] = match name {
        "allcraft" | "learndiscipline" | "forgetdiscipline" | "craftkit" | "learnblueprint"
        | "givecash" | "givexp" | "giveability" | "giveammo" | "gmgiveammo" => {
            &["server_db_query", "GM/Base feedback"]
        }
        "missionfail" | "missionrewards" => &["GM feedback", "server_db_query"],
        "effects" | "cooldowns" | "cleareffects" => &["server_ability_state", "GM feedback"],
        "dummy" | "pet" => &["server_entity_get", "GM feedback"],
        "infiniteammo" | "gmsetinfiniteammo" => &["GM feedback", ".infiniteammo"],
        _ => &["server_entity_get", "GM feedback"],
    };
    let comparison_fields: &[&str] = match name {
        "givexp" => &[
            "player_id",
            "exp",
            "level",
            "training_points",
            "applied_science_points",
        ],
        "givecash" => &["player_id", "naquadah"],
        "giveammo" | "gmgiveammo" | "craftkit" => &[
            "character_id",
            "item_id",
            "type_id",
            "stack_size",
            "container_id",
            "slot_id",
        ],
        "allcraft" => &[
            "player_id",
            "discipline_ids",
            "discipline_id",
            "expertise",
            "blueprint_ids",
            "racial_paradigm_levels",
            "craft_anywhere",
        ],
        "learnblueprint" => &["player_id", "blueprint_ids"],
        "learndiscipline" | "forgetdiscipline" => &["player_id", "discipline_id", "expertise"],
        "giveability" => &["player_id", "ability_id", "source_kind"],
        "missionfail" | "missionrewards" => {
            &["player_id", "mission_id", "status", "step", "repeats"]
        }
        "speed" => &[
            "entity_id",
            "movementSpeedMod",
            "rotationSpeedMod",
            "cur",
            "min",
            "max",
        ],
        "cooldowns" => &["entity_id", "cooldowns"],
        "dummy" => &["entity_id", "template_id", "space_id", "health", "faction"],
        "pet" => &["entity_id", "template_id", "space_id", "owner", "stance"],
        "infiniteammo" | "gmsetinfiniteammo" => &["player_id", "switch_state"],
        "effects" | "cleareffects" => &["entity_id", "ledger", "pulses", "stats"],
        _ => &["entity_id", "stats"],
    };
    Some(WorkbenchMetadata {
        category,
        availability,
        disposition_reason: reason,
        metadata_version: COMMAND_METADATA_VERSION,
        args,
        forms,
        preconditions,
        impact,
        verify,
        compensation,
        impact_kind: if read_only {
            "read_only"
        } else if persistent {
            "persistent"
        } else {
            "transient"
        },
        verify_tools,
        comparison_fields,
        compensation_kind: if read_only { "none" } else { "manual" },
    })
}

fn stat_read(name: &str) -> Detail {
    let forms = match name {
        "stats" => "health, focus and regeneration: four current/max pairs",
        "primarystats" => "coordination, engagement, fortitude, morale, perception, intelligence: six current/max pairs",
        "speedstats" => "movement/rotation, reload, grenade, deploy and attack speed: six current/max pairs",
        "armorstats" => "armor and resistance: eight current/max pairs",
        "qrstats" => "QR combat values: 20 current/max pairs",
        "absorbstats" => "typed, item and energy absorption: 15 current/max pairs",
        _ => "stealth, reveal and disguise: five current/max pairs",
    };
    (
        "Stats", "runnable", "", NONE, forms,
        "GM; required selected same-space/in-view entity; Target::Being does not guarantee a stat block",
        "Read-only live cell StatList; missing stat prints n/a, not zero; no DB write",
        "Compare reported IDs and current/max pairs to server_entity_get.stats for exact entity_id; absent entity or stats is unknown, not a failed mutation",
        "None needed",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::console::{command_catalog, registry::commands};
    use std::collections::BTreeSet;

    #[test]
    fn reviewed_names_equal_five_registry_families() {
        let source: BTreeSet<_> = [
            commands::progression::SPECS,
            commands::abilities::SPECS,
            commands::stats::SPECS,
            commands::ammo::SPECS,
            commands::pet::SPECS,
        ]
        .into_iter()
        .flatten()
        .map(|spec| spec.name)
        .collect();
        let catalog = command_catalog();
        let described: BTreeSet<_> = catalog
            .iter()
            .filter(|row| metadata(&row.name).is_some())
            .map(|row| row.name.as_str())
            .collect();
        assert_eq!(source.len(), 27);
        assert_eq!(described, source);
        for row in catalog
            .iter()
            .filter(|row| source.contains(row.name.as_str()))
        {
            assert!(!row.forms.is_empty(), ".{}", row.name);
            assert!(!row.verify.is_empty(), ".{}", row.name);
            assert!(!row.preconditions.is_empty(), ".{}", row.name);
            assert!(!row.impact.is_empty(), ".{}", row.name);
            assert!(!row.compensation.is_empty(), ".{}", row.name);
            assert_eq!(row.manual_pending, row.availability != "runnable");
        }
    }

    #[test]
    fn grants_and_mixed_forms_fail_closed() {
        for name in [
            "allcraft",
            "learndiscipline",
            "forgetdiscipline",
            "craftkit",
            "learnblueprint",
            "givecash",
            "givexp",
            "giveability",
            "giveammo",
            "gmgiveammo",
            "missionfail",
            "missionrewards",
            "speed",
            "dummy",
            "cooldowns",
            "cleareffects",
            "infiniteammo",
            "gmsetinfiniteammo",
            "pet",
        ] {
            let row = command_catalog()
                .into_iter()
                .find(|row| row.name == name)
                .unwrap();
            assert_eq!(row.availability, "unsupported", ".{name}");
            assert!(!row.disposition_reason.is_empty(), ".{name}");
        }
        let xp = metadata("givexp").unwrap();
        assert!(xp.comparison_fields.contains(&"applied_science_points"));
        assert!(xp.verify.contains("relog"));
        assert!(xp.verify.contains("before/after"));
        assert!(xp.verify.contains("intended and current player_id"));
        let ammo = metadata("giveammo").unwrap();
        assert!(ammo.verify.contains("container_id IN (1,15)"));
        assert!(ammo.verify.contains("truncated=false"));
        assert!(ammo.verify.contains("actual DB type_id"));
    }

    #[test]
    fn unsafe_ability_actions_stay_disabled_in_projected_catalog() {
        let catalog = command_catalog();
        for (name, required_reason) in [
            ("cooldowns", "per-form gating"),
            ("cleareffects", "before-effect snapshot"),
        ] {
            let descriptor = metadata(name).unwrap();
            let row = catalog.iter().find(|row| row.name == name).unwrap();
            assert_eq!(descriptor.availability, "unsupported", ".{name}");
            assert_eq!(row.availability, "unsupported", ".{name}");
            assert!(row.manual_pending, ".{name}");
            assert!(row.disposition_reason.contains(required_reason), ".{name}");
        }
    }

    #[test]
    fn typed_bounds_and_choices_match_handler_contracts() {
        let catalog = command_catalog();
        let get = |name| catalog.iter().find(|row| row.name == name).unwrap();
        assert_eq!(get("craftkit").args[1].min, Some(1.0));
        assert_eq!(get("craftkit").args[1].max, Some(10.0));
        assert_eq!(get("cooldowns").args[0].choices, ["reset"]);
        assert_eq!(get("cooldowns").args[1].min, Some(1.0));
        assert_eq!(get("dummy").args[2].max, Some(290.0));
        assert_eq!(
            get("pet").args[0].choices,
            ["summon", "dismiss", "stance", "info", "list"]
        );
        assert!(get("pet").forms.contains("stance 0|1|2"));
        assert!(get("missionrewards").verify.contains("unknown"));
    }

    #[test]
    fn ammo_registry_aliases_point_to_one_canonical_palette_action() {
        let catalog = command_catalog();
        for (canonical, alias) in [
            ("giveammo", "gmgiveammo"),
            ("infiniteammo", "gmsetinfiniteammo"),
        ] {
            let primary = catalog.iter().find(|row| row.name == canonical).unwrap();
            let raw_alias = catalog.iter().find(|row| row.name == alias).unwrap();
            assert_eq!(primary.aliases, [alias]);
            assert_eq!(primary.alias_of, None);
            assert_eq!(raw_alias.alias_of, Some(canonical));
            assert!(raw_alias.aliases.is_empty());
        }
    }

    #[test]
    fn read_only_and_unavailable_readback_are_distinct() {
        let catalog = command_catalog();
        for name in [
            "effects",
            "stats",
            "primarystats",
            "speedstats",
            "armorstats",
            "qrstats",
            "absorbstats",
            "stealthstats",
        ] {
            let row = catalog.iter().find(|row| row.name == name).unwrap();
            assert_eq!(row.availability, "runnable", ".{name}");
            assert_eq!(row.impact_kind, "read_only", ".{name}");
            assert!(
                row.verify.contains("unknown") || row.verify.contains("snapshot"),
                ".{name}"
            );
        }
        let mission = catalog
            .iter()
            .find(|row| row.name == "missionrewards")
            .unwrap();
        assert_eq!(mission.availability, "unsupported");
        assert!(mission.verify.contains("unknown"));
    }
}
