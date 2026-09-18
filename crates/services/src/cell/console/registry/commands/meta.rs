//! Console meta commands: `.help` and the seed-authoring session buffer
//! (`console/seed.rs`).

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[
    spec(
        "help",
        0,
        1,
        Target::None,
        "List console commands (optionally filter by substring)",
    ),
    spec(
        "seedconfirm",
        0,
        0,
        Target::None,
        "Emit your pending authoring changes per seed file (log/Discord)",
    ),
    spec(
        "seedpending",
        0,
        0,
        Target::None,
        "List your pending authoring changes",
    ),
    spec(
        "seedcancel",
        0,
        0,
        Target::None,
        "Discard your pending authoring changes",
    ),
];
