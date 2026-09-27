//! Social (`console/social.rs`): the GM broadcast.

use super::{spec, Spec, Target};

// `min` is 0 on purpose: a bare `.announce` must reach
// `social::parse_announce`, which answers with the usage line and logs
// `chat.gm_broadcast_rejected reason=no_text`. The generic argc check would
// refuse it first with a generic line and no chat event.
pub(super) const SPECS: &[Spec] = &[spec(
    "announce",
    0,
    usize::MAX,
    Target::None,
    "Broadcast a server line to every online player, or to your space with `space` first ([space] text)",
)];
