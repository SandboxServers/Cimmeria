//! Low-level net / AI debug (category H): the raw client-message pokes
//! (`console/net.rs`) and the movement/threat debug toggles.

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[
    spec(
        "net_seq",
        1,
        2,
        Target::Spawnable,
        "Play a kismet sequence on the target",
    ),
    spec(
        "net_seqto",
        1,
        2,
        Target::None,
        "Play a sequence from you to the target",
    ),
    spec(
        "net_seqfrom",
        1,
        2,
        Target::Spawnable,
        "Play a sequence from the target to you",
    ),
    spec(
        "net_timer",
        2,
        4,
        Target::Spawnable,
        "Start a client timer on the target",
    ),
    spec(
        "net_mapinfo",
        3,
        5,
        Target::Player,
        "Send onMapInfo to the target",
    ),
    spec(
        "net_speak",
        1,
        2,
        Target::Spawnable,
        "Make the target speak (message [channel])",
    ),
    spec(
        "net_dialog",
        1,
        1,
        Target::None,
        "Open a dialog with the target",
    ),
    spec(
        "net_challenge",
        5,
        5,
        Target::None,
        "Send onClientChallenge to the target",
    ),
    spec(
        "debug_velocity",
        3,
        3,
        Target::Spawnable,
        "Set the velocity of the target",
    ),
    spec(
        "debug_controller",
        0,
        0,
        Target::Spawnable,
        "Toggle the debug movement controller on the target",
    ),
    spec(
        "debug_follow",
        0,
        0,
        Target::Spawnable,
        "Toggle the follow controller on the target",
    ),
    spec(
        "threaten",
        1,
        1,
        Target::Mob,
        "Generate threat on the targeted mob",
    ),
    spec(
        "aggression",
        1,
        1,
        Target::Mob,
        "Set the targeted mob's aggression override (1 hostile..5, 0 passive, 'clear')",
    ),
    spec(
        "aggro",
        0,
        1,
        Target::None,
        "Mobs notice you: '.aggro off' / '.aggro on' (no arg shows it)",
    ),
];
