//! Granular per-domain stat readouts (category F) and the one stat setter
//! (`.speed`, category K) — the family `console/stats.rs` implements.

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[
    // ── F. granular stat readouts ──────────────────────────────────────────
    spec(
        "stats",
        0,
        0,
        Target::Being,
        "Show basic health/focus stats of the target",
    ),
    spec(
        "primarystats",
        0,
        0,
        Target::Being,
        "Show primary attribute stats of the target",
    ),
    spec(
        "speedstats",
        0,
        0,
        Target::Being,
        "Show movement/action speed stats of the target",
    ),
    spec(
        "armorstats",
        0,
        0,
        Target::Being,
        "Show armor + resistance stats of the target",
    ),
    spec(
        "qrstats",
        0,
        0,
        Target::Being,
        "Show QR-system combat stats of the target",
    ),
    spec(
        "absorbstats",
        0,
        0,
        Target::Being,
        "Show damage-absorption stats of the target",
    ),
    spec(
        "stealthstats",
        0,
        0,
        Target::Being,
        "Show stealth/disguise stats of the target",
    ),
    // ── K. stat setters ────────────────────────────────────────────────────
    spec(
        "speed",
        1,
        1,
        Target::Being,
        "Set the target's current movement and rotation speed together",
    ),
];
