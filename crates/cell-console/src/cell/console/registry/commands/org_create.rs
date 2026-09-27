//! Organization founding (`console/org_create.rs`, organizations campaign
//! ORG-05). No legacy counterpart: the Python server had no organizations.

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[spec(
    "org_create",
    2,
    usize::MAX,
    Target::None,
    "Found a Team or Command that you lead, without the registrar (team|command name)",
)];
