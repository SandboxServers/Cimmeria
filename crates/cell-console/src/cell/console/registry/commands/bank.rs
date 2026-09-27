//! Bank (`console/bank.rs`): the GM's vault shortcut (bank-vault BV-02) and
//! the read-only vault listing (BV-04), the GM vault expansion (BV-05), and
//! the Team vault expansion paid from the treasury (BV-09).

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
), spec(
    "bankexpand",
    0,
    0,
    Target::None,
    "Buy one +10 vault expansion for yourself at the seeded price (needs an open vault: .bank or a Banker)",
), spec(
    "orgvaultexpand",
    0,
    2,
    Target::None,
    "Quote, or with the current size buy, one +10 step of your Team vault from the Team treasury; leader only ([team|command] [from_slots])",
)];
