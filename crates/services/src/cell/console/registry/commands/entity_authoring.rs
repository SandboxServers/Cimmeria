//! Entity / content authoring (category A): the commands that edit a
//! spawnable's identity, appearance, and dialog wiring. Implemented by
//! `console/entity.rs`.

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[
    spec(
        "tag",
        1,
        1,
        Target::Spawnable,
        "Set the content tag of the target ('none' clears)",
    ),
    spec(
        "name",
        1,
        usize::MAX,
        Target::Being,
        "Set the display name of the target",
    ),
    spec(
        "alignment",
        1,
        1,
        Target::Being,
        "Set alignment (undefined|praxis|sgu) of the target",
    ),
    spec(
        "nameid",
        1,
        1,
        Target::Spawnable,
        "Set the localized name-id of the target",
    ),
    spec(
        "staticmesh",
        1,
        1,
        Target::Spawnable,
        "Set the static mesh name of the target",
    ),
    spec(
        "bodyset",
        1,
        1,
        Target::Spawnable,
        "Set the body set of the target",
    ),
    spec(
        "eventset",
        1,
        1,
        Target::Spawnable,
        "Set the kismet event-set id of the target",
    ),
    spec(
        "interactiontype",
        1,
        1,
        Target::Spawnable,
        "Set the interaction-type flags of the target",
    ),
    spec(
        "lookat",
        0,
        0,
        Target::Spawnable,
        "Rotate the target to face you",
    ),
    spec(
        "visible",
        1,
        1,
        Target::Spawnable,
        "Show/hide the target (1/0)",
    ),
    spec(
        "setcombatant",
        1,
        1,
        Target::Being,
        "Set a combatant state flag on the target",
    ),
    spec(
        "unsetcombatant",
        1,
        1,
        Target::Being,
        "Clear a combatant state flag on the target",
    ),
    spec(
        "addcomponent",
        1,
        1,
        Target::Being,
        "Add a body component to the target",
    ),
    spec(
        "delcomponent",
        1,
        1,
        Target::Being,
        "Remove a body component from the target",
    ),
    spec(
        "adddialog",
        2,
        2,
        Target::Spawnable,
        "Add a dialog choice (templateId setMapId) to the target",
    ),
    spec(
        "removedialog",
        2,
        2,
        Target::Spawnable,
        "Remove a dialog choice (templateId setMapId) from the target",
    ),
    spec(
        "dynamicupdate",
        0,
        0,
        Target::Spawnable,
        "Re-broadcast the target's dynamic properties to witnesses",
    ),
];
