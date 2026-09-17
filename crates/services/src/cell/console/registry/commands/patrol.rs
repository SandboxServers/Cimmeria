//! Patrol authoring (category C) — the FanMMORPG `path_*` additions, which
//! legacy never had. Implemented by `console/patrol.rs`.

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[
    spec(
        "path_add",
        1,
        1,
        Target::None,
        "Append your position as the next waypoint of a path (pathId)",
    ),
    spec(
        "path_show",
        1,
        1,
        Target::None,
        "Show all waypoints of a path (pathId)",
    ),
    spec(
        "path_clear",
        1,
        1,
        Target::None,
        "Delete all waypoints of a path (pathId)",
    ),
    spec(
        "path_assign",
        1,
        2,
        Target::Mob,
        "Assign a path to the target NPC and start it (pathId [delay])",
    ),
    spec(
        "path_unassign",
        0,
        0,
        Target::Mob,
        "Remove the patrol from the target NPC",
    ),
    spec(
        "path_set_seq",
        3,
        3,
        Target::None,
        "Set a kismet sequence on a waypoint (pathId index seqId)",
    ),
    spec(
        "path_clear_seq",
        2,
        2,
        Target::None,
        "Clear the sequence on a waypoint (pathId index)",
    ),
    spec(
        "path_set_tp",
        2,
        2,
        Target::None,
        "Set a waypoint's teleport dest to your position (pathId index)",
    ),
    spec(
        "path_clear_tp",
        2,
        2,
        Target::None,
        "Clear a waypoint's teleport (pathId index)",
    ),
    spec(
        "path_set_tp_seq",
        3,
        3,
        Target::None,
        "Set a waypoint's arrival sequence (pathId index seqId)",
    ),
    spec(
        "path_set_tp_delay",
        3,
        3,
        Target::None,
        "Set a waypoint's teleport delay (pathId index delay)",
    ),
];
