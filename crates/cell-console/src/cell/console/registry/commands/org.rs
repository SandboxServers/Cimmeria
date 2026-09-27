//! Teams and Commands (`console/org.rs`, organizations campaign ORG-06 and
//! ORG-07).

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
];
