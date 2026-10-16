//! Source-reviewed workbench contracts for dot patrol and entity authoring.

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

const fn bounded(name: &'static str, kind: &'static str, min: f64, max: f64) -> TypedArg {
    TypedArg {
        name,
        kind,
        required: true,
        min: Some(min),
        max: Some(max),
        choices: &[],
        help: name,
    }
}

const fn path() -> TypedArg {
    bounded("pathId", "integer", i32::MIN as f64, i32::MAX as f64)
}
const fn index() -> TypedArg {
    bounded("index", "integer", 0.0, i32::MAX as f64)
}

const NONE: &[TypedArg] = &[];
const PATH: &[TypedArg] = &[path()];
const PATH_DELAY: &[TypedArg] = &[path(), field("delay", "float", false)];
const PATH_INDEX: &[TypedArg] = &[path(), index()];
const PATH_INDEX_SEQ: &[TypedArg] = &[
    path(),
    index(),
    bounded("seqId", "integer", i32::MIN as f64, i32::MAX as f64),
];
const PATH_INDEX_DELAY: &[TypedArg] = &[path(), index(), field("delay", "float", true)];
const TAG: &[TypedArg] = &[field("tag", "text", true)];
const NAME: &[TypedArg] = &[field("nameWords", "text", true)];
const NAME_ID: &[TypedArg] = &[bounded(
    "nameId",
    "integer",
    i32::MIN as f64,
    i32::MAX as f64,
)];
const MESH: &[TypedArg] = &[field("mesh", "text", true)];
const BODY_SET: &[TypedArg] = &[field("bodySet", "text", true)];
const EVENT_SET: &[TypedArg] = &[bounded(
    "eventSetId",
    "integer",
    i32::MIN as f64,
    i32::MAX as f64,
)];
const FLAGS: &[TypedArg] = &[bounded(
    "flags",
    "integer",
    i32::MIN as f64,
    i32::MAX as f64,
)];
const COMPONENT: &[TypedArg] = &[field("component", "text", true)];
const BIT: &[TypedArg] = &[bounded("bit", "integer", 0.0, 31.0)];
const DIALOG: &[TypedArg] = &[
    bounded("templateId", "template", i32::MIN as f64, i32::MAX as f64),
    bounded("setMapId", "integer", i32::MIN as f64, i32::MAX as f64),
];
const ALIGNMENT: &[TypedArg] = &[TypedArg {
    name: "alignment",
    kind: "enum",
    required: true,
    min: None,
    max: None,
    choices: &["undefined", "praxis", "sgu"],
    help: "Alignment word, case-insensitive",
}];
const VISIBLE: &[TypedArg] = &[TypedArg {
    name: "visible",
    kind: "boolean",
    required: true,
    min: None,
    max: None,
    choices: &["0", "1", "false", "true"],
    help: "False hides from current witnesses; true only acknowledges a later AoI refresh",
}];

/// Return the reviewed command contract. Disabled entries are discoverable, never raw actions.
pub(super) fn metadata(name: &str) -> Option<WorkbenchMetadata> {
    let (category, availability, reason, args, forms, preconditions, impact, verify, compensation) = match name {
        "path_add" => ("Patrol authoring", "unsupported", "Shared path buffer and per-GM SQL queue can split parent/child ownership or attach a point to a wrong-world header", PATH, "pathId: signed i32",
            "GM; caller and world resolve; server must bind buffer to GM queue and validate point-set world and DB collision before enablement",
            "Append a shared in-memory waypoint; queue header on first buffered point and child INSERT for this GM. Confirm may write live DB; emitted SQL requires seed_commit_required",
            "Compare .path_show and caller .seedpending; after both base results query resources.point_sets set_id/world_id and ordered point_set_points; queue or positive child alone is insufficient",
            "Manual exact header/point reconstruction; .path_clear is destructive and .seedcancel leaves shared buffer"),
        "path_show" => ("Patrol", "runnable", "", PATH, "pathId: signed i32; session buffer only",
            "GM; parseable pathId; no selected target; shared cell-process buffer may contain another GM's points",
            "Read only: list indexed session-buffer waypoints; no DB read, SQL queue, or seed change",
            "GM coordinate feedback is the buffer readback; it does not establish a DB path", "None needed"),
        "path_clear" => ("Patrol authoring", "unsupported", "No path ownership/existence guard; two separate DELETEs can partially complete and erase preexisting rows", PATH, "pathId: signed i32",
            "GM; server-side path ownership/existence and destructive preview required before enablement",
            "Immediately remove shared buffer; queue child then parent DELETE. Confirm may separately write live DB; emitted SQL requires seed_commit_required; existing NPC patrols remain",
            "Compare .path_show plus both base results and exact set_id counts in point_set_points and point_sets after confirm; queued status is no deletion",
            "Manual reconstruction from captured header and all points; no automatic inverse"),
        "path_assign" => ("Patrol authoring", "unsupported", "Mob selection admits non-NPC targets; path world, delay bounds and exact patrol readback are not enforced", PATH_DELAY, "pathId [delay]: signed i32, optional finite f32 seconds (default 3)",
            "GM; selected same-space/in-view true NPC; nonempty session path in same world; bounded nonnegative delay and exact readback needed",
            "Copy session path into target, set Patrol AI. With spawn_id queue spawnlist patrol override; confirm may write live DB, seed_commit_required; without spawn_id memory only",
            "Capture entity_id, ai_state and spawn_id; exact patrol list/delay unavailable; after positive base result compare exact spawnlist override columns and point_set_points",
            "Manual restore of captured patrol/delay/AI; .path_unassign loses earlier assignment"),
        "path_unassign" => ("Patrol authoring", "unsupported", "Mob selection admits non-NPC targets and full before/after patrol list is unavailable", NONE, "selected target; no arguments",
            "GM; selected same-space/in-view true NPC and exact prior patrol readback required",
            "Clear in-memory patrol and set Idle; with spawn_id queue NULL spawnlist override; confirm may write live DB, seed_commit_required",
            "Compare entity_id, ai_state, spawn_id and exact spawnlist override columns after positive base result; idle alone may be a no-op",
            "Manual restore of captured points, delay and AI; .path_assign cannot recreate DB-only paths"),
        "path_set_seq" => waypoint(name, PATH_INDEX_SEQ, "pathId index seqId", "Restore captured sequence_id or clear if previously NULL"),
        "path_clear_seq" => waypoint(name, PATH_INDEX, "pathId index", "Restore captured sequence_id manually"),
        "path_set_tp" => waypoint(name, PATH_INDEX, "pathId index; destination = caller position", "Restore captured destination, arrival sequence and delay; .path_clear_tp also clears them"),
        "path_clear_tp" => waypoint(name, PATH_INDEX, "pathId index", "Restore all five captured values manually"),
        "path_set_tp_seq" => waypoint(name, PATH_INDEX_SEQ, "pathId index seqId", "Restore captured value; .path_clear_tp also clears other fields"),
        "path_set_tp_delay" => waypoint(name, PATH_INDEX_DELAY, "pathId index delay: finite f32 seconds", "Restore captured delay; .path_clear_tp also destroys destination/sequence"),
        "tag" => ("Entity authoring", "runnable", "", TAG, "tag: one token; exact lowercase none clears",
            "GM; fresh selected same-space/in-view spawnable; no namespace or length validation; separate .savespawn needs its own proof",
            "Transient CellEntity.tag edit only; no DB, queued SQL or seed change. Explicit later .savespawn may queue a valid spawnlist tag",
            "Compare server_entity_get entity_id/tag before and after; verify a separate save by exact spawnlist.tag only after confirm and positive base result",
            "Manually reapply captured tag or none; no atomic undo"),
        "name" => ("Entity authoring", "unsupported", "Registry accepts players but player snapshot prefers character_name and cannot verify the npc_name edit; no NPC/length guard", NAME, "one or more whitespace tokens, joined with single spaces; no quotes",
            "GM; selected same-space/in-view verified NPC; NPC-only and bounded-name guards required",
            "Set npc_name in memory even on player; no DB, SQL or seed change; NPC may show name on next AoI entry",
            "NPC: compare server_entity_get.name/entity_name; player readback unavailable; client AoI appearance unverified",
            "Reapply captured name; no clear-to-None command"),
        "alignment" => ("Entity authoring", "unsupported", "Previous alignment outside 0..2 cannot be restored through this command; subject class is too broad", ALIGNMENT, "undefined|praxis|sgu (case-insensitive)",
            "GM; selected verified being; capture old value and require representable enum before enablement",
            "Replace in-memory alignment; invalid enum refuses; no DB, SQL or seed change",
            "Compare server_entity_get.alignment or .info before/after; client display separate",
            "Reapply prior enum only if old value was 0..2"),
        "nameid" => ("Entity authoring", "unsupported", "No resource existence check or clear-to-None operation", NAME_ID, "nameId: signed i32",
            "GM; selected spawnable; seeded name resource lookup and prior value required",
            "Set in-memory name_id; .savespawn does not persist it; no SQL, DB or seed change",
            "Compare server_entity_get.name_id or .info; localized client text unverified", "Reapply old numeric ID; cannot restore prior None"),
        "staticmesh" => ("Entity authoring", "unsupported", "No mesh asset lookup, exact snapshot string or clear-to-None operation", MESH, "mesh: one token, no quotes",
            "GM; selected spawnable; validate asset and capture old mesh before enablement",
            "Set in-memory static_mesh; no immediate NPC appearance broadcast, DB, SQL or seed change",
            "Read exact mesh with .info; server_entity_get.has_static_mesh is only a boolean; client AoI render unverified",
            "Reapply old mesh if expressible; prior None cannot be restored"),
        "bodyset" => ("Entity authoring", "unsupported", "No body-set asset lookup, exact snapshot string or clear-to-None operation", BODY_SET, "bodySet: one token, no quotes",
            "GM; selected spawnable; validate asset and capture old body set before enablement",
            "Set in-memory body_set; no immediate NPC appearance broadcast, DB, SQL or seed change",
            "Read exact body set with .info; snapshot omits string and client AoI render is unverified",
            "Reapply old token if expressible; prior None cannot be restored"),
        "eventset" => ("Entity authoring", "unsupported", "No seeded event-set lookup or complete readback for zero versus None", EVENT_SET, "eventSetId: signed i32",
            "GM; selected spawnable; validate seeded event set and capture exact old value",
            "Set in-memory event_set_id; no event execution, DB, SQL or seed change",
            "Read nonzero event set with .info; server_entity_get omits it and zero versus None is unknown",
            "Reapply known old nonzero ID; prior None cannot be restored"),
        "interactiontype" => ("Entity authoring", "unsupported", "No validated flag mask or client prompt proof; prior i64 flags may exceed the setter's i32 range", FLAGS, "flags: signed i32 stored as i64",
            "GM; selected spawnable; validate flag mask and prior value before enablement",
            "Replace in-memory interaction_type_flags; no forced witness refresh, DB, SQL or seed change",
            "Compare server_entity_get.interaction_type_flags; client prompt behavior unverified",
            "Reapply captured flags only if old value fits i32"),
        "lookat" => ("Entity authoring", "unsupported", "No execution-time NPC-only/autosave-state guard; non-player pets or props may stop and queue spawnlist SQL", NONE, "selected target; no arguments",
            "GM; fresh verified NPC with captured old facing and explicit autosave state; coincident x/z refuses",
            "Turn toward caller; any non-player also gets home facing and movement stop. Autosave may queue spawnlist SQL; confirm/positive DB result and seed_commit_required for durable save",
            "Compare server_entity_get.direction; if autosave on, inspect refusal or .seedpending then exact spawnlist heading after positive base result; client facing unverified",
            "Manual .rotation from captured angles may itself autosave; stopped movement has no proven inverse"),
        "visible" => ("Entity authoring", "unsupported", "True is a no-op acknowledgement and no active show/readback path exists", VISIBLE, "0|1|false|true",
            "GM; selected spawnable; false affects only current witnesses; true cannot restore their visibility",
            "False sends EntityInvisible per current witness without a stored visibility flag; true sends feedback only; no DB, SQL or seed change",
            "server_witnesses identifies recipients, not client hidden/shown state; inspect per-client AoI to verify render",
            "No proven inverse; true does not actively show"),
        "setcombatant" | "unsetcombatant" => ("Entity authoring", "unsupported", "State-field ownership/refcounts are not enforced; reverse command can erase another system's state", BIT, "bit: integer 0..31",
            "GM; selected verified being; bit bounded 0..31; state owner must be established",
            if name == "setcombatant" { "OR bit into in-memory state_field; no DB, SQL or seed change" } else { "AND-clear bit from in-memory state_field; no DB, SQL or seed change" },
            "Compare server_entity_get.state_field; combat and client flag effects unverified",
            "Opposite command is not a safe automatic inverse without state ownership"),
        "addcomponent" | "delcomponent" => ("Entity authoring", "unsupported", "No exact component-list readback or ownership check; duplicate/absent operations report success", COMPONENT, "component: one token, no quotes",
            "GM; selected verified being; validate component asset and capture exact prior list",
            if name == "addcomponent" { "Add absent in-memory component; player requests RefreshAppearance; NPC waits for AoI; no DB, SQL or seed change" } else { "Remove matching in-memory component; player requests RefreshAppearance; NPC waits for AoI; no DB, SQL or seed change" },
            "server_entity_get.component_count cannot prove identity; exact list/client BeingAppearance application unavailable",
            "Manual opposite command may affect another owner or ordering; no safe inverse"),
        "adddialog" | "removedialog" => ("Entity authoring", "unsupported", "No target/template relation check or exact interaction-list readback; remove can report success for absent tuple", DIALOG, "templateId setMapId: signed i32 tokens",
            "GM; selected spawnable; setMapId must resolve in loaded dialog_set_maps; validate target/template relation",
            if name == "adddialog" { "Insert tuple into in-memory available_interactions[templateId] if absent; no DB, SQL, seed or client refresh" } else { "Remove matching tuple from in-memory available_interactions[templateId]; no DB, SQL, seed or client refresh" },
            "Feedback gives resolved dialog ID but snapshot omits interaction list; exact post-state and client prompt unknown",
            "Manual opposite command uses current map, possibly changed; no proven inverse"),
        "dynamicupdate" => ("Entity authoring", "unsupported", "Target::Spawnable admits NPCs, which receive no refresh, and player client application is unverified", NONE, "selected target; no arguments",
            "GM; selected player with player_id for a refresh request; NPC action has no immediate send",
            "Player queues RefreshAppearance; NPC gives success-shaped feedback only; no cell-field, DB, SQL or seed mutation",
            "Packet/client witness evidence needed; server enqueue alone is only refresh requested",
            "None; rebroadcast request has no inverse"),
        _ => return None,
    };
    let authoring = name.starts_with("path_") && name != "path_show";
    let verify_tools: &[&str] = if authoring {
        &["GM feedback", ".seedpending", "server_db_query"]
    } else if name == "path_show" {
        &["GM feedback"]
    } else if name == "lookat" || name == "tag" {
        &["server_entity_get", ".seedpending", "server_db_query"]
    } else if name == "staticmesh" || name == "bodyset" || name == "eventset" {
        &[".info", "server_entity_get"]
    } else if name == "visible" {
        &["server_witnesses", "client AoI"]
    } else {
        &["server_entity_get", "GM feedback"]
    };
    let comparison_fields: &[&str] = match name {
        "path_add" | "path_clear" => &["set_id", "world_id", "point_id", "rows_affected"],
        "path_show" => &["path_id", "index", "position"],
        "path_assign" | "path_unassign" => &[
            "entity_id",
            "ai_state",
            "spawn_id",
            "patrol_path_id",
            "patrol_point_delay",
        ],
        "path_set_seq" | "path_clear_seq" => {
            &["set_id", "point_id", "sequence_id", "rows_affected"]
        }
        "path_set_tp" => &[
            "set_id",
            "point_id",
            "teleport_x",
            "teleport_y",
            "teleport_z",
            "rows_affected",
        ],
        "path_clear_tp" => &[
            "set_id",
            "point_id",
            "teleport_x",
            "teleport_y",
            "teleport_z",
            "teleport_sequence_id",
            "teleport_delay",
            "rows_affected",
        ],
        "path_set_tp_seq" => &[
            "set_id",
            "point_id",
            "teleport_sequence_id",
            "rows_affected",
        ],
        "path_set_tp_delay" => &["set_id", "point_id", "teleport_delay", "rows_affected"],
        "tag" => &["entity_id", "tag"],
        "name" => &["entity_id", "name", "entity_name"],
        "alignment" => &["entity_id", "alignment"],
        "nameid" => &["entity_id", "name_id"],
        "staticmesh" => &["entity_id", "static_mesh"],
        "bodyset" => &["entity_id", "body_set"],
        "eventset" => &["entity_id", "event_set_id"],
        "interactiontype" => &["entity_id", "interaction_type_flags"],
        "lookat" => &["entity_id", "direction", "spawn_id", "heading"],
        "visible" => &["entity_id", "witness_id", "client_visible"],
        "setcombatant" | "unsetcombatant" => &["entity_id", "state_field"],
        "addcomponent" | "delcomponent" => &["entity_id", "component_list", "component_count"],
        "adddialog" | "removedialog" => &[
            "entity_id",
            "template_id",
            "set_map_id",
            "available_interactions",
        ],
        "dynamicupdate" => &[
            "entity_id",
            "player_id",
            "refresh_message",
            "client_appearance",
        ],
        _ => unreachable!("metadata match covers every row"),
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
        impact_kind: if authoring || name == "lookat" {
            "authoring"
        } else if name == "path_show" {
            "read_only"
        } else {
            "transient"
        },
        verify_tools,
        comparison_fields,
        compensation_kind: if name == "path_show" || name == "visible" || name == "dynamicupdate" {
            "none"
        } else {
            "manual"
        },
    })
}

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

fn waypoint(
    name: &str,
    args: &'static [TypedArg],
    forms: &'static str,
    compensation: &'static str,
) -> Detail {
    let reason = if name == "path_set_tp_delay" {
        "Delay bounds/old value and world ownership are unchecked; ordinal row can change before confirmation"
    } else {
        "Path/world ownership and prior values are unchecked; ordinal row can change before confirmation"
    };
    let preconditions = "GM; signed pathId, nonnegative i32 index, exact point_id/world ownership pinned server-side at confirmation; current caller position for path_set_tp";
    let impact = match name {
        "path_set_seq" => "Queue sequence_id = seqId by ordinal; no immediate memory/DB edit. Confirm may write live DB; emitted SQL requires seed_commit_required",
        "path_clear_seq" => "Queue sequence_id = NULL by ordinal; no immediate memory/DB edit. Confirm may write live DB; emitted SQL requires seed_commit_required",
        "path_set_tp" => "Queue teleport_x/y/z = caller position by ordinal; no immediate teleport or DB edit. Confirm may write live DB; emitted SQL requires seed_commit_required",
        "path_clear_tp" => "Queue clear of all five teleport columns by ordinal; no immediate memory/DB edit. Confirm may write live DB; emitted SQL requires seed_commit_required",
        "path_set_tp_seq" => "Queue teleport_sequence_id = seqId by ordinal; no immediate memory/DB edit. Confirm may write live DB; emitted SQL requires seed_commit_required",
        _ => "Queue teleport_delay = delay by ordinal; no immediate memory/DB edit. Confirm may write live DB; emitted SQL requires seed_commit_required",
    };
    let verify = match name {
        "path_set_seq" | "path_clear_seq" => "Compare exact point_id/sequence_id before/after positive base result; shared path_seq label cannot attribute a batch; zero rows means no edit",
        "path_set_tp_seq" => "Compare exact point_id/teleport_sequence_id before/after positive base result; shared path_seq label cannot attribute a batch; zero rows means no edit",
        "path_set_tp" => "Capture caller position, exact point_id and neighboring rows; compare teleport_x/y/z after positive base result; zero rows means no edit",
        "path_clear_tp" => "Compare exact point_id and all five NULL teleport columns after positive base result; zero rows means no edit",
        _ => "Compare exact point_id/teleport_delay before/after positive base result; zero rows means no edit",
    };
    // Keep the changed columns visible even when the shared readback recipe is reused.
    let forms = match name {
        "path_clear_tp" => {
            "pathId index; clears all five: teleport_x/y/z, teleport_sequence_id and teleport_delay"
        }
        _ => forms,
    };
    (
        "Patrol authoring",
        "unsupported",
        reason,
        args,
        forms,
        preconditions,
        impact,
        verify,
        compensation,
    )
}

#[cfg(test)]
mod tests;
