//! Player progression: crafting / discipline grants (category E,
//! `console/crafting.rs`), the mission gaps (`console/mission.rs`), and the
//! cash / XP grants (`console/give.rs`).

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[
    // ── E. crafting / discipline ───────────────────────────────────────────
    spec(
        "allcraft",
        0,
        0,
        Target::Player,
        "Grant all blueprints + max disciplines to the target",
    ),
    spec(
        "learndiscipline",
        1,
        2,
        Target::Player,
        "Learn/raise a discipline (disciplineId [expertise])",
    ),
    spec(
        "forgetdiscipline",
        1,
        1,
        Target::Player,
        "Forget a discipline (disciplineId)",
    ),
    // ── Mission gaps ───────────────────────────────────────────────────────
    spec(
        "missionfail",
        1,
        1,
        Target::Player,
        "Force-fail a mission on the target (designId)",
    ),
    spec(
        "missionrewards",
        1,
        1,
        Target::Player,
        "Preview a mission's reward set (designId)",
    ),
    // ── Player grants ──────────────────────────────────────────────────────
    spec(
        "givecash",
        1,
        1,
        Target::Player,
        "Grant naquadah to the target (amount)",
    ),
    spec(
        "givexp",
        1,
        1,
        Target::Player,
        "Grant experience to the target (amount)",
    ),
];
