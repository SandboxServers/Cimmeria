//! Pets PT-05 tests: follow and teleport, stances, the owner's combat state,
//! and the owner-anchored leash. Fixture tests on state and log rows: the
//! fixtures have no navmesh, so nothing here asserts a route.

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::{AiState, PetStance};
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{add_pet_owner, seed_pet_template, PET_FIXTURE_TEMPLATE_ID};
use crate::test_support::{Captured, LogCaptureGuard};

mod ability;
mod follow;
mod leash;
mod owner_combat;
mod owner_identity;
mod stance;
mod telemetry;

/// Owner entity id; `add_pet_owner` makes it the account id too.
const OWNER: u32 = 7;
/// The owner's character (`sgw_player.player_id`), for the identity fields.
const OWNER_PLAYER_ID: i32 = 4242;
/// Mob ids used by the tests.
const MOB: u32 = 500;
const MOB_2: u32 = 501;
/// The faction every player-hostile mob carries.
const HOSTILE: u8 = 10;

/// Agnos, big enough for the teleport distances, with the pet template cached.
fn make_world() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />
        </Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#,
    )
    .unwrap();
    seed_pet_template(&mut mgr, PET_FIXTURE_TEMPLATE_ID);
    mgr
}

/// The owner at `owner_pos` with one pet summoned beside it (2 u behind:
/// the owner faces +z, so the pet starts at `owner_pos - (0, 0, 2)`).
fn world_with_pet(owner_pos: [f32; 3]) -> (SpaceManager, u32) {
    let mut mgr = make_world();
    add_pet_owner(&mut mgr, OWNER, "Agnos", owner_pos, 12);
    mgr.get_entity_mut(OWNER).unwrap().player_id = Some(OWNER_PLAYER_ID);
    let pet = mgr
        .spawn_pet_from_template(OWNER, PET_FIXTURE_TEMPLATE_ID, 0)
        .expect("pet spawns");
    (mgr, pet)
}

/// A living mob of `faction` at `pos`, 100/100 HEALTH.
fn add_mob(mgr: &mut SpaceManager, id: u32, pos: [f32; 3], faction: u8) {
    mgr.spawn_npc(id, "Agnos", pos, [0.0; 3]).unwrap();
    let mob = mgr.get_entity_mut(id).unwrap();
    mob.faction = faction;
    if let Some(h) = mob.stats.get_mut(cimmeria_entity::stats::HEALTH) {
        h.update(0, 100, 100);
        h.clear_dirty();
    }
}

/// `mob` is fighting `victim`: Fighting, with `victim` on its threat list.
fn mob_fights(mgr: &mut SpaceManager, mob: u32, victim: u32) {
    let e = mgr.get_entity_mut(mob).unwrap();
    crate::cell::service::npc_ai::force_ai_state(e, AiState::Fighting);
    e.threat_list.insert(victim, 10.0);
}

fn set_stance(mgr: &mut SpaceManager, pet: u32, stance: PetStance) {
    mgr.get_entity_mut(pet)
        .unwrap()
        .pet
        .as_deref_mut()
        .unwrap()
        .stance = stance;
}

fn move_to(mgr: &mut SpaceManager, id: u32, pos: [f32; 3]) {
    mgr.update_position_preserving_facing(id, pos, [0.0; 3]);
}

fn pos(mgr: &SpaceManager, id: u32) -> Vector3 {
    mgr.get_entity(id).unwrap().position
}

fn state(mgr: &SpaceManager, id: u32) -> AiState {
    mgr.get_entity(id).unwrap().ai_state()
}

/// One natural-cadence AI tick. Returns every message it sent.
async fn tick(mgr: &mut SpaceManager) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(4096);
    crate::cell::service::npc_ai::npc_ai_tick_for_test(
        &tx,
        mgr,
        &crate::test_support::NoContentEvents,
    )
    .await;
    drop(tx);
    let mut out = Vec::new();
    while let Some(m) = rx.recv().await {
        out.push(m);
    }
    out
}

/// The first `pets.ai` row with `decision_outcome = outcome`.
fn pets_ai_row(logs: &LogCaptureGuard, outcome: &str) -> Option<Captured> {
    logs.all()
        .into_iter()
        .find(|c| c.target == "pets.ai" && c.has_field("decision_outcome", outcome))
}

/// A `pets.ai` row carries the OWNER's identity (instrumentation-discipline
/// Rule 5): `account_id` and `player_id`.
fn assert_owner_identity(row: &Captured) {
    assert!(row.has_field("account_id", &OWNER.to_string()), "{row:?}");
    assert!(
        row.has_field("player_id", &OWNER_PLAYER_ID.to_string()),
        "{row:?}"
    );
    assert!(row.has_field("owner_id", &OWNER.to_string()), "{row:?}");
}

/// Whether `msgs` hold an `onStateFieldUpdate` addressed to `entity_id`.
fn state_update_to(msgs: &[CellToBaseMsg], entity_id: u32) -> Option<u32> {
    msgs.iter().find_map(|m| match m {
        CellToBaseMsg::EntityMethodCall {
            entity_id: e,
            method_index,
            args,
        } if *e == entity_id
            && *method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE =>
        {
            Some(u32::from_le_bytes(args[..4].try_into().unwrap()))
        }
        _ => None,
    })
}
