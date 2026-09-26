//! Server / maintenance (category G): persistence, cache reloads, and the
//! log-plumbing toggles.

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[
    spec("save", 0, 0, Target::None, "Persist your player entity now"),
    spec(
        "reloadmap",
        0,
        0,
        Target::None,
        "Reload the current map on yourself",
    ),
    spec(
        "reloadres",
        0,
        1,
        Target::None,
        "Reload server resource caches",
    ),
    spec(
        "removerespawner",
        1,
        1,
        Target::Player,
        "Remove a respawner from the target (respawnerId)",
    ),
    spec(
        "loglevel",
        1,
        2,
        Target::None,
        "Set the server log level (level [category])",
    ),
    spec(
        "logclient",
        0,
        0,
        Target::None,
        "Toggle forwarding server logs to your client",
    ),
];
