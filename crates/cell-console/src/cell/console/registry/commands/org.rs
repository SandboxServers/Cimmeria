//! Teams and Commands (`console/org.rs`, organizations campaign ORG-06).

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[spec(
    "org_disband",
    1,
    1,
    Target::None,
    "Disband a Team or Command; refused while its vault holds anything (orgId)",
)];
