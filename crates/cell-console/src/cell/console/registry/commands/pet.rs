//! Pet UAT tools (pets campaign PT-07, `console/pet.rs`). No legacy
//! counterpart: the Python server had no pet console command.

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[spec(
    "pet",
    1,
    2,
    Target::None,
    "Pet tools: summon <templateId|abilityId> | dismiss | stance <0-2> | info | list",
)];
