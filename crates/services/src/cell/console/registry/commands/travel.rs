//! Travel (`console/travel/`) and placement (category J,
//! `console/placement.rs`) — everything that moves or re-orients an entity.

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[
    // ── Travel ─────────────────────────────────────────────────────────────
    spec(
        "gotoxyz",
        3,
        3,
        Target::None,
        "Teleport the target (or yourself) to coordinates in this space (x y z)",
    ),
    spec(
        "goto",
        1,
        1,
        Target::None,
        "Move the target (or yourself) to a named online player's position/instance",
    ),
    spec(
        "summon",
        1,
        1,
        Target::None,
        "Move a named online player to the target's (or your) position/instance",
    ),
    spec(
        "gotolocation",
        4,
        4,
        Target::None,
        "Move the target (or yourself) to coordinates in a named world (worldName x y z)",
    ),
    spec(
        "gotospace",
        4,
        4,
        Target::None,
        "Move the target (or yourself) into an exact loaded space instance (spaceId x y z)",
    ),
    // ── J. placement (position / orientation) ──────────────────────────────
    spec(
        "location",
        0,
        3,
        Target::Spawnable,
        "Report (no args) or set (x y z) the target's position",
    ),
    spec(
        "rotation",
        0,
        3,
        Target::Spawnable,
        "Report (no args) or set (pitch yaw roll) the target's orientation",
    ),
];
