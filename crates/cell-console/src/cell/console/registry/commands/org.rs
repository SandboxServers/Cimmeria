//! Teams and Commands (`console/org.rs`, organizations campaign ORG-06,
//! ORG-07 and ORG-10).

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[
    spec(
        "org_disband",
        1,
        1,
        Target::None,
        "Disband a Team or Command; refused while its vault holds anything (orgId)",
    ),
    spec(
        "org_join",
        1,
        2,
        Target::None,
        "Add an online player (default: you) to a Team or Command at its entry rank (orgId [player])",
    ),
    spec(
        "org_rank",
        2,
        3,
        Target::None,
        "Set a member's rank; never Leader (player rank [orgId])",
    ),
    spec(
        "org_info",
        0,
        1,
        Target::None,
        "List a player's (default: your) Teams and Commands with rank and permission mask ([player])",
    ),
    spec(
        "org_list",
        0,
        0,
        Target::None,
        "List every Team and Command with its member count and leader",
    ),
    spec(
        "org_set_perms",
        3,
        3,
        Target::None,
        "Set a rank's permission mask, clamped to the type's editor bits; never Leader (orgId rank mask)",
    ),
];
