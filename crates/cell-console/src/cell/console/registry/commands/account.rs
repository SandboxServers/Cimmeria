//! Account (`console/account.rs`): the GM's sign-in methods tool (C10) —
//! list an account's methods, remove one, or issue a one-time link code.

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[spec(
    "account",
    2,
    3,
    Target::None,
    "Launcher sign-in methods of an account: list, remove one, or issue a one-time link code (identities <account> | unlink <account> <identity_id> | linkcode <account>)",
)];
