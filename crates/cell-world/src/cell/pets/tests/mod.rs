//! Pets tests: registry, spawn, owner-only intro, class filters and
//! teardown (PT-01); owner lifecycle hooks and the corpse timer (PT-02).

use crate::cell::space_manager::SpaceManager;
use crate::test_fixtures::{add_pet_owner, seed_pet_template, PET_FIXTURE_TEMPLATE_ID};

mod class_filters;
mod create_on_client;
mod owner_hooks;
mod registry;
mod spawn;
mod teardown;
mod owner_hooks_telemetry;
mod telemetry;

/// Owner entity id used throughout.
const OWNER: u32 = 7;
/// A second player who can see the pet but does not own it.
const OTHER: u32 = 8;

/// Agnos (shared) and Castle (shared) are loaded; Castle_CellBlock is
/// instanced, one space per `create_entity`.
fn make_world() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />
        <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="2400" MinY="0" MaxY="2400" />
        <Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
        </Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    seed_pet_template(&mut mgr, PET_FIXTURE_TEMPLATE_ID);
    mgr
}

/// Destroy `OWNER` and hand its entity id to a different player in the
/// same space (another account and character), the id-reuse window between
/// `destroy_entity` and the sweep (Copilot, #870).
fn reuse_owner_id_by_another_player(mgr: &mut SpaceManager) {
    mgr.destroy_entity(OWNER);
    add_pet_owner(mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    let impostor = mgr.get_entity_mut(OWNER).unwrap();
    impostor.account_id = Some(4242);
    impostor.player_id = Some(4243);
}

/// [`reuse_owner_id_by_another_player`], then the id's new holder summons
/// a pet of its own before the sweep runs. Returns that second pet. With an
/// owner-keyed capture, this summon would overwrite the first owner's
/// identity and make the OLD pet look like the new holder's (Copilot, #870).
fn reuse_owner_id_then_resummon(mgr: &mut SpaceManager) -> u32 {
    reuse_owner_id_by_another_player(mgr);
    mgr.spawn_pet_from_template(OWNER, PET_FIXTURE_TEMPLATE_ID, 1643)
        .expect("the id's new holder summons its own pet")
}

/// A world with `OWNER` (level 12) in Agnos and one pet summoned for it.
fn world_with_pet() -> (SpaceManager, u32) {
    let mut mgr = make_world();
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    let pet = mgr
        .spawn_pet_from_template(OWNER, PET_FIXTURE_TEMPLATE_ID, 1643)
        .expect("pet spawns");
    (mgr, pet)
}
