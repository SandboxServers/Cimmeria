//! Reviewed workbench contracts for the dot travel and spawn families.

use super::{workbench::WorkbenchMetadata, COMMAND_METADATA_VERSION};

pub(crate) struct TypedArg {
    pub name: &'static str,
    pub kind: &'static str,
    pub required: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub choices: &'static [&'static str],
    pub help: &'static str,
}

const fn arg(name: &'static str, kind: &'static str, required: bool) -> TypedArg {
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

const fn bounded(
    name: &'static str,
    kind: &'static str,
    required: bool,
    min: f64,
    max: f64,
) -> TypedArg {
    TypedArg {
        name,
        kind,
        required,
        min: Some(min),
        max: Some(max),
        choices: &[],
        help: name,
    }
}

const XYZ: &[TypedArg] = &[
    arg("x", "float", true),
    arg("y", "float", true),
    arg("z", "float", true),
];
const NAME: &[TypedArg] = &[arg("playerName", "text", true)];
const NO_ARGS: &[TypedArg] = &[];
const WORLD: &[TypedArg] = &[
    arg("worldName", "world", true),
    arg("x", "float", false),
    arg("y", "float", false),
    arg("z", "float", false),
];
const SPACE: &[TypedArg] = &[
    bounded("spaceId", "integer", true, 1.0, i32::MAX as f64),
    arg("x", "float", false),
    arg("y", "float", false),
    arg("z", "float", false),
];
const OPTIONAL_XYZ: &[TypedArg] = &[
    arg("x", "float", false),
    arg("y", "float", false),
    arg("z", "float", false),
];
const OPTIONAL_ROTATION: &[TypedArg] = &[
    arg("pitch", "float", false),
    arg("yaw", "float", false),
    arg("roll", "float", false),
];
const TEMPLATE: &[TypedArg] = &[bounded(
    "templateId",
    "template",
    true,
    1.0,
    i32::MAX as f64,
)];
const ENTITY_ID: &[TypedArg] = &[bounded("entityId", "entity", false, 0.0, u32::MAX as f64)];
const BOOLEAN: &[TypedArg] = &[arg("on", "boolean", true)];
const SPAWN_SET: &[TypedArg] = &[
    TypedArg {
        name: "action",
        kind: "enum",
        required: false,
        min: None,
        max: None,
        choices: &["list", "on", "off", "clear"],
        help: "List, activate, deactivate or clear spawn sets",
    },
    arg("setId", "integer", false),
];
const RANDOM: &[TypedArg] = &[
    bounded("templateId", "template", true, 1.0, i32::MAX as f64),
    TypedArg {
        name: "xRange",
        kind: "float",
        required: true,
        min: Some(0.0),
        max: None,
        choices: &[],
        help: "Nonnegative finite x ring radius",
    },
    TypedArg {
        name: "zRange",
        kind: "float",
        required: true,
        min: Some(0.0),
        max: None,
        choices: &[],
        help: "Nonnegative finite z ring radius",
    },
    bounded("count", "integer", false, 1.0, 50.0),
];

/// Look up a reviewed contract; all other registry rows remain pending.
pub(super) fn metadata(name: &str) -> Option<WorkbenchMetadata> {
    let (category, availability, reason, args, forms, preconditions, impact, verify, compensation) =
        match name {
            "gotoxyz" => ("Travel", "runnable", "", XYZ, "x y z",
                "GM; refresh selected subject in caller's space and view; finite coordinates",
                "Authoritative same-space position/grid snap; player gets BASEMSG_FORCED_POSITION; duel may end; base attempts player-position persistence",
                "Capture subject identity, space, position and direction with server_entity_get before and after; inspect wire.out.forced_position for player send; client application and DB persistence need separate proof",
                "Manual .gotoxyz to captured position; side effects are not atomically undone"),
            "goto" => ("Travel", "unsupported", "Cross-space subject GM level and GM-only destination policy cannot be verified before dispatch", NAME, "playerName",
                "GM; unique online destination player; subject in caller view; NPC only within its space; verify subject access for GM-only world",
                "Same-space snap or player GateTravel teardown/re-entry; base may redirect subject; trade, duel and pet state may change; arrival persistence attempted",
                "Capture destination and subject entity snapshots; after cross-space re-entry compare exact instance/position, then sgw_player world, coordinates and known_stargates; enqueue is not arrival",
                "Manual return to captured origin if loaded and authorized; gameplay side effects have no proven inverse"),
            "summon" => ("Travel", "unsupported", "Named subject's GM-only-world eligibility cannot be verified before cross-space dispatch", NAME, "playerName",
                "GM; unique online named subject; live caller destination; check named player's own access level",
                "Same-space player snap or GateTravel; non-GM may redirect or disconnect at GM-only world; arrival persistence attempted",
                "Capture both players before; compare named subject after re-entry; inspect forced-position send for snap or sgw_player world/position/known_stargates after transfer",
                "Manual return to captured origin; associated gameplay state has no automatic undo"),
            "gotolocation" => ("Travel", "runnable", "", WORLD, "worldName [x y z] (exactly 1 or 4 tokens)",
                "GM; known canonical world; live caller; finite complete tuple or resolvable world entry point",
                "Caller-only snap in current instance or GateTravel into world; arrival persistence attempted; no spawnlist write",
                "Capture caller before and after by identity; check exact world/space/position after re-entry and sgw_player world/position/known_stargates; forced-position send is not client application",
                "Manual travel to captured origin if reachable; no general inverse for associated state"),
            "gotospace" => ("Travel", "unsupported", "Exact cross-instance arrival can be redirected by subject's GM-only-world policy", SPACE, "spaceId [x y z] (exactly 1 or 4 tokens)",
                "GM; loaded positive instance; live subject; complete finite tuple or entry point; player required for cross-space",
                "Same-instance snap or GateTravel with teardown/re-entry; base may redirect; arrival persistence attempted",
                "Compare subject identity, exact space/world/position after re-entry; inspect forced-position send or sgw_player world/position/known_stargates as appropriate",
                "Manual .gotospace to captured loaded origin if authorized; no general inverse"),
            "location" => ("Placement", "unsupported", "Selected pet can be moved and autosaved to spawnlist; cell-side pet guard is absent", OPTIONAL_XYZ, "report (0 tokens) | set x y z",
                "GM; live selected spawnable in caller's view/space; finite complete set tuple",
                "Report reads position; set moves position/grid and NPC home; player gets forced-position; autosave may queue seed SQL",
                "Compare server_entity_get space/position/direction before and after; inspect forced-position send for player and .seedpending for autosave",
                "Manual set to old coordinates; .seedcancel discards the entire pending queue, not only this change"),
            "rotation" => ("Placement", "unsupported", "Pet autosave lacks a cell guard and player orientation is overwritten by client updates", OPTIONAL_ROTATION, "report (0 tokens) | set pitch yaw roll",
                "GM; live selected spawnable; finite complete radian tuple",
                "Report reads direction; set changes server direction and NPC home; autosave may queue seed SQL; player owner has no orientation snap",
                "Read server_entity_get.direction immediately and again after player avatar update; inspect .seedpending for autosave",
                "Manual NPC rotation to old value; pending queue cancel is broad and does not restore home"),
            "movehere" => ("Placement", "unsupported", "Selected pet can be settled and autosaved without a cell-side class guard", NO_ARGS, "no arguments",
                "GM; selected non-player in caller's space; finite caller position/yaw",
                "Moves NPC position/grid and facing, sets home and stops movement; autosave may queue seed SQL",
                "Compare target and caller server_entity_get position/space/direction; home and AI-path readback are unavailable; inspect .seedpending if autosave",
                "Manual position/rotation restore; prior AI path and home have no proven inverse"),
            "spawn" => ("Spawn", "runnable", "", TEMPLATE, "templateId",
                "GM; caller in space with finite position/yaw; positive template ID; base must find resources.entity_templates row",
                "Cell/base request creates one runtime NPC; no spawnlist write even with autosave on",
                "Wait for spawned npc <id> completion, then server_entity_get for ID/template/space/position and absent spawn_id; no completion means unknown",
                "Manual .despawn <returnedId> while in same space; no seed row to undo"),
            "despawn" => ("Spawn", "runnable", "", ENTITY_ID, "selected NPC | entityId",
                "GM; selected visible non-player or explicit ID in caller's space; ID form excludes pet/prop/player",
                "Removes runtime entity, combat links and witnesses; spawnlist row remains; ordinary respawn tick does not restore it",
                "Capture entity identity/space before; confirm completion ID and server_entity_get null after, accounting for ID reuse; check spawnlist separately",
                "No exact inverse; .spawn creates a different runtime ID and does not restore authored state"),
            "savespawn" => ("Spawn authoring", "unsupported", "Summoned pet can be queued as a persistent spawnlist INSERT; cell-side guard is absent", NO_ARGS, "selected non-player (no arguments)",
                "GM; selected in-view templated non-player with world; duplicate confirmed new spawn refuses",
                "Changes NPC home and queues INSERT/UPDATE; .seedconfirm writes live DB and emits SQL; seed commit remains manual",
                "Read .seedpending; after .seedconfirm query exact spawn_id or emitted row, then inspect committed spawnlist.sql separately",
                "Manual restoration; .seedcancel discards caller's entire pending authoring queue"),
            "delspawn" => ("Spawn authoring", "runnable", "", NO_ARGS, "selected seeded NPC (no arguments)",
                "GM; selected in-view templated non-player with real spawn_id",
                "Queues DELETE and immediately despawns runtime entity; .seedconfirm writes live DB; seed-file removal remains manual",
                "Capture spawn_id; verify runtime removal separately from .seedpending, confirmed DB row absence and committed seed diff",
                "No built-in inverse; restore runtime and exact row/seed manually; .seedcancel clears whole queue"),
            "autosavespawn" => ("Spawn authoring", "unsupported", "Turning on can autosave a selected pet through placement or .lookat; centralized cell-side guard is absent", BOOLEAN, "1|0|true|false",
                "GM; valid boolean; off only until pet/class guard covers every placement path",
                "Caller-local toggle; later NPC placement may queue SQL, requiring .seedconfirm and seed commit",
                "Feedback only; no independent preference readback exists; later .seedpending can show effects",
                "Reissue prior boolean; this does not cancel already queued SQL"),
            "respawnall" => ("Spawn", "unsupported", "Handler bypasses spatial grid, dead flags, loot cleanup and witness re-create lifecycle", NO_ARGS, "no arguments",
                "GM; caller in a space; complete canonical respawn and fan-out repair required",
                "Resets all SGWMob NPCs in caller space in memory; no spawnlist write",
                "Compare per-ID position, health, focus, flags and AI state; current snapshot cannot prove grid, loot, threat or witness state",
                "None proven"),
            "spawnset" => ("Spawn sets", "unsupported", "on can clear active peer sets before target space/member validation; command-level action gate unavailable", SPAWN_SET, "list | on setId | off setId | clear",
                "GM; on needs known nonempty spawnable set and loaded destination before peer teardown",
                "World-wide runtime set activation/deactivation; no DB/seed write; partial member failures possible",
                "Re-read .spawnset list and member state; Shown count 0 is not success; structured independent set-state readback unavailable",
                "off can reverse isolated on; peer teardown has no atomic inverse"),
            "spawnrandom" => ("Spawn", "unsupported", "Server accepts malformed count, nonpositive template and ranges that can overflow derived positions", RANDOM, "templateId xRange zRange [count]",
                "GM; positive template ID; finite nonnegative ranges; count 1..50; every derived position finite",
                "Requests 1..50 runtime copies; partial and late spawns possible; no spawnlist write",
                "Wait for final batch summary and inspect each returned ID with server_entity_get; timed-out copies remain unknown",
                "Manual .despawn for each returned live ID; no exact batch inverse"),
            _ => return None,
        };
    let impact_kind = match name {
        "savespawn" | "delspawn" => "authoring",
        "gotolocation" | "gotospace" | "goto" | "summon" => "persistent",
        _ => "transient",
    };
    let verify_tools: &[&str] = match name {
        "savespawn" | "delspawn" => &["server_entity_get", "server_db_query", ".seedpending"],
        "spawnset" => &[".spawnset list", "server_entity_query"],
        "autosavespawn" => &["GM feedback", ".seedpending"],
        "spawnrandom" => &["GM batch summary", "server_entity_get"],
        "spawn" => &["GM completion feedback", "server_entity_get"],
        "goto" | "summon" | "gotolocation" | "gotospace" => {
            &["server_entity_get", "server_sessions", "server_db_query"]
        }
        _ => &["server_entity_get"],
    };
    let comparison_fields: &[&str] = match name {
        "rotation" => &["entity_id", "space_id", "direction"],
        "savespawn" | "delspawn" => &["entity_id", "spawn_id", "template_id", "position"],
        "spawnset" => &["set_id", "active", "member_count"],
        "autosavespawn" => &["feedback", "pending_count"],
        "spawn" | "spawnrandom" => &[
            "entity_id",
            "template_id",
            "space_id",
            "position",
            "spawn_id",
        ],
        "respawnall" => &[
            "entity_id",
            "position",
            "health_cur",
            "focus_cur",
            "state_field",
            "ai_state",
        ],
        _ => &[
            "entity_id",
            "space_id",
            "world_name",
            "position",
            "direction",
        ],
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
        impact_kind,
        verify_tools,
        comparison_fields,
        compensation_kind: if name == "respawnall" {
            "none"
        } else {
            "manual"
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::console::registry::COMMANDS;

    #[test]
    fn travel_and_spawn_metadata_matches_exact_family_set() {
        let expected = [
            "gotoxyz",
            "goto",
            "summon",
            "gotolocation",
            "gotospace",
            "location",
            "rotation",
            "movehere",
            "spawn",
            "despawn",
            "savespawn",
            "delspawn",
            "autosavespawn",
            "respawnall",
            "spawnset",
            "spawnrandom",
        ];
        let found: Vec<_> = COMMANDS
            .iter()
            .filter(|s| metadata(s.name).is_some())
            .map(|s| s.name)
            .collect();
        assert_eq!(found.len(), expected.len());
        for name in expected {
            assert!(found.contains(&name), "missing .{name}");
        }
    }

    #[test]
    fn unsafe_forms_are_disabled_with_a_reason() {
        for name in [
            "goto",
            "summon",
            "gotospace",
            "location",
            "rotation",
            "movehere",
            "savespawn",
            "autosavespawn",
            "respawnall",
            "spawnset",
            "spawnrandom",
        ] {
            let row = metadata(name).unwrap();
            assert_eq!(row.availability, "unsupported", ".{name}");
            assert!(!row.disposition_reason.is_empty(), ".{name}");
        }
    }
}
