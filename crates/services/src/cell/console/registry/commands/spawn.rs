//! Spawn lifecycle + authoring / persistence (category B) — the family
//! `console/spawn/` implements.

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[
    spec(
        "spawn",
        1,
        1,
        Target::None,
        "Spawn one entity of a template at your position and facing (templateId)",
    ),
    // Legacy registered `.despawn` as `SGWSpawnableEntity`, which `SGWPlayer`
    // derives from — so the legacy command would destroy a logged-in player's
    // cell entity. Corrected to `Target::Mob` (NPC / non-player spawnable
    // only) per D02; `SpaceManager::despawn_npc` refuses players again on its
    // own so the guard survives a registry edit.
    spec(
        "despawn",
        0,
        0,
        Target::Mob,
        "Destroy the selected NPC (runtime only — its spawnlist row is untouched)",
    ),
    spec(
        "savespawn",
        0,
        0,
        Target::Spawnable,
        "Persist the target's spawn to the spawnlist",
    ),
    spec(
        "delspawn",
        0,
        0,
        Target::Spawnable,
        "Delete the target's spawnlist row",
    ),
    spec(
        "autosavespawn",
        1,
        1,
        Target::None,
        "Toggle auto-persisting newly spawned entities (1/0)",
    ),
    spec(
        "respawnall",
        0,
        0,
        Target::None,
        "Respawn every NPC in your space",
    ),
    spec(
        "spawnrandom",
        3,
        4,
        Target::None,
        "Spawn N random-scattered copies of a template (templateId xRange zRange [count])",
    ),
];
