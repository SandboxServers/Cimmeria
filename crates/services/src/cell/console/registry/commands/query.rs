//! Read-only lookups: the legacy search/query family (category D) and the
//! entity / combat inspection commands (category I). Implemented by
//! `console/query.rs` and `console/entity.rs`.

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[
    // ── D. search / query ─────────────────────────────────────────────────
    spec(
        "searchitem",
        1,
        2,
        Target::None,
        "Search item designs by name",
    ),
    spec(
        "searchmission",
        1,
        2,
        Target::None,
        "Search mission designs by name",
    ),
    spec(
        "searchtemplate",
        1,
        2,
        Target::None,
        "Search entity templates by name",
    ),
    spec(
        "players",
        0,
        0,
        Target::None,
        "List players online on this CellApp service",
    ),
    spec(
        "listabilities",
        0,
        0,
        Target::Player,
        "List the target player's known abilities by name",
    ),
    // ── I. entity / combat inspection (read-only) ──────────────────────────
    spec(
        "info",
        0,
        1,
        Target::None,
        "Show detailed info about the target (selection wins over [entityId])",
    ),
    spec(
        "facing",
        0,
        0,
        Target::Spawnable,
        "Show facing angle/class and distance to the target",
    ),
    spec(
        "combatinfo",
        0,
        0,
        Target::Mob,
        "Diagnose combat-readiness issues on the targeted NPC",
    ),
];
