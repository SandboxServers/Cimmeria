//! Bank (`console/bank.rs`): the GM's vault shortcut (bank-vault BV-02) and
//! the read-only vault listing (BV-04).

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[spec(
    "bank",
    0,
    0,
    Target::None,
    "Open your own personal vault here, without a Banker (GM session; any interact closes it)",
), spec(
    "bankdump",
    0,
    1,
    Target::None,
    "List a character's personal vault (container 17), read-only; your own with no name ([name])",
)];
