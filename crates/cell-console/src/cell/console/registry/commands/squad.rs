//! Squads (`console/squad.rs`, organizations campaign ORG-04). No legacy
//! counterpart: the Python server had no squads.

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[
    spec(
        "squad_invite",
        1,
        1,
        Target::None,
        "Invite an online player to your squad, as /squadinvite does (name)",
    ),
    spec(
        "squad_join",
        1,
        1,
        Target::None,
        "Join a player's squad with no invite; founds one they lead if they have none (name)",
    ),
    spec(
        "squad_info",
        0,
        1,
        Target::None,
        "Show your squad, or a named player's ([name])",
    ),
];
