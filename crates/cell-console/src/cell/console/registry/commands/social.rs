//! Social (`console/social.rs`): the GM broadcast.

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[spec(
    "announce",
    1,
    usize::MAX,
    Target::None,
    "Broadcast a server line to every online player, or to your space with `space` first ([space] text)",
)];
