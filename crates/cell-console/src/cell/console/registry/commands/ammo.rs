//! Ammo campaign AM-06 GM tools (`console/gm/give_ammo.rs`,
//! `console/gm/set_infinite_ammo.rs`). The `gm*` rows alias the client's
//! native `/gmgiveammo` / `/gmsetinfiniteammo`, which never reach a server
//! handler (no `.def` method), so testers who type those names with a dot
//! land here too.

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[
    spec(
        "giveammo",
        2,
        2,
        Target::None,
        "Grant special ammo rounds as bag stacks (type name|id, rounds; selected player, else you)",
    ),
    spec("gmgiveammo", 2, 2, Target::None, "Alias of .giveammo"),
    spec(
        "infiniteammo",
        0,
        1,
        Target::None,
        "Special-ammo reloads draw nothing from the bags; the clip still empties ([on|off]; selected player, else you)",
    ),
    spec(
        "gmsetinfiniteammo",
        0,
        1,
        Target::None,
        "Alias of .infiniteammo",
    ),
];
