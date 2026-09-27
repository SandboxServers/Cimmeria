//! NA44 source-order guard for `CellService::start`: ability definitions
//! load before the startup NPC spawn, so each startup NPC's
//! `spawner.npc_behaviour` row reports real `event_set_ids`.

/// The startup spaces spawn their NPCs inside `CellService::start`, which
/// used to load the ability definitions only afterwards, so every startup
/// NPC's `event_set_ids` read 0 (Castle's guards included). The start-up
/// sequence needs a database and a message loop, so this pins the order in
/// the source instead. Revert proof: move the `load_ability_defs` block back
/// below `spawn_npcs_from_records` and this fails.
#[test]
fn spawn_behaviour_row_ability_defs_load_before_the_startup_spawn() {
    let src = include_str!("../startup.rs");
    let defs = src
        .find("spawner::load_ability_defs(")
        .expect("startup loads ability defs");
    let spawn = src
        .find("spawn_npcs_from_records(")
        .expect("startup spawns NPCs");
    assert!(
        defs < spawn,
        "ability defs must be loaded before the startup NPC spawn"
    );
}
