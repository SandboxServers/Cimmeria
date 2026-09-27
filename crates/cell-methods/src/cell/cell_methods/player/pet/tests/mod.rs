//! PT-04 pet command tests: the ownership guard (spoofed ids), invoke,
//! toggle and stance.
//!
//! The world: `OWNER` (7) and `OTHER` (8) in Agnos, each with one summoned
//! pet, a hostile mob 5 u from the owner's pet, and a friendly NPC beside it.
//! Commands go in through the pet module's `dispatch`, as the client's bytes
//! would.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::super::constants::{PET_ABILITY_TOGGLE, PET_CHANGE_STANCE, PET_INVOKE_ABILITY};
use crate::cell::client_methods::player::ON_ERROR_CODE;
use crate::cell::combat::HOSTILE_FACTION;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{
    add_pet_owner, seed_ability_defs, seed_pet_template, PET_FIXTURE_ABILITIES,
    PET_FIXTURE_TEMPLATE_ID,
};

mod guard;
mod invoke;
mod stance;
mod telemetry;
mod toggle;
mod warmup;

/// The player who owns `World::pet`.
const OWNER: u32 = 7;
/// `OWNER`'s `account_id` (`add_pet_owner` uses the entity id).
const OWNER_ACCOUNT_ID: u32 = OWNER;
/// `OWNER`'s `sgw_player.player_id`.
const OWNER_PLAYER_ID: i32 = 1007;
/// A second player with a pet of their own.
const OTHER: u32 = 8;
/// A hostile (faction 10) mob near the owner's pet.
const MOB: u32 = 200_001;
/// A friendly (faction 1) NPC near the owner's pet.
const FRIENDLY: u32 = 200_002;
/// An id nothing holds.
const NOBODY: u32 = 4_000_000;
/// The first ability on the fixture pet's bar (Pistol Shot).
const PET_ABILITY: i32 = PET_FIXTURE_ABILITIES[0];
/// An ability the fixture pet does not have.
const NOT_A_PET_ABILITY: i32 = 597;

struct World {
    mgr: SpaceManager,
    /// `OWNER`'s pet.
    pet: u32,
    /// `OTHER`'s pet.
    other_pet: u32,
}

/// Agnos, where everything starts, plus a second shared world (Castle) for
/// the cross-space tests. Both span the pet's coordinates.
fn two_world_space_manager() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />
        <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="2400" MinY="0" MaxY="2400" />
        </Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr
}

fn world() -> World {
    let mut mgr = two_world_space_manager();
    seed_pet_template(&mut mgr, PET_FIXTURE_TEMPLATE_ID);
    seed_ability_defs(&mut mgr, &PET_FIXTURE_ABILITIES);
    seed_ability_defs(&mut mgr, &[NOT_A_PET_ABILITY]);
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    // `add_pet_owner` sets `account_id = entity id`; give the owner a
    // character id too, so identity fields are distinguishable in logs.
    mgr.get_entity_mut(OWNER).unwrap().player_id = Some(OWNER_PLAYER_ID);
    add_pet_owner(&mut mgr, OTHER, "Agnos", [20.0, 0.0, 10.0], 12);
    let pet = mgr
        .spawn_pet_from_template(OWNER, PET_FIXTURE_TEMPLATE_ID, 0)
        .expect("owner's pet spawns");
    let other_pet = mgr
        .spawn_pet_from_template(OTHER, PET_FIXTURE_TEMPLATE_ID, 0)
        .expect("other player's pet spawns");
    let pet_pos = mgr.get_entity(pet).unwrap().position;
    for (id, faction, dx) in [(MOB, HOSTILE_FACTION, 5.0), (FRIENDLY, 1, -5.0)] {
        mgr.spawn_npc(
            id,
            "Agnos",
            [pet_pos.x + dx, pet_pos.y, pet_pos.z],
            [0.0; 3],
        )
        .unwrap();
        mgr.get_entity_mut(id).unwrap().faction = faction;
    }
    World {
        mgr,
        pet,
        other_pet,
    }
}

/// Everything the handler queued for the base.
struct Sent(Vec<CellToBaseMsg>);

impl Sent {
    /// `(InstanceID, ErrorCodeID)` of every `onErrorCode` to `player`.
    fn error_codes_to(&self, player: u32) -> Vec<(i32, u16)> {
        self.0
            .iter()
            .filter_map(|m| match m {
                CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index,
                    args,
                } if *entity_id == player && *method_index == ON_ERROR_CODE => {
                    assert_eq!(args.len(), 7, "onErrorCode is u8 + i32 + u16");
                    assert_eq!(args[0], 0, "SystemID is ERRORCODE_SYSTEM_Ability");
                    Some((
                        i32::from_le_bytes(args[1..5].try_into().unwrap()),
                        u16::from_le_bytes(args[5..7].try_into().unwrap()),
                    ))
                }
                _ => None,
            })
            .collect()
    }

    /// Every `onErrorCode`, whoever it is addressed to.
    fn all_error_codes(&self) -> usize {
        self.0
            .iter()
            .filter(|m| {
                matches!(m, CellToBaseMsg::EntityMethodCall { method_index, .. }
                    if *method_index == ON_ERROR_CODE)
            })
            .count()
    }

    /// `(witness_id, entity_id, method_index, args)` of every single-recipient
    /// entity method.
    fn witness_calls(&self) -> Vec<(u32, u32, u16, Vec<u8>)> {
        self.0
            .iter()
            .filter_map(|m| match m {
                CellToBaseMsg::WitnessEntityMethod {
                    witness_id,
                    entity_id,
                    method_index,
                    args,
                    ..
                } => Some((*witness_id, *entity_id, *method_index, args.clone())),
                _ => None,
            })
            .collect()
    }
}

/// Run one pet command as `caller` and return what it sent.
async fn call(mgr: &mut SpaceManager, caller: u32, method: u16, args: &[u8]) -> Sent {
    let (tx, mut rx) = mpsc::channel(512);
    let engine = ChainEngine::new();
    let handled = super::dispatch(caller, method, args, &tx, mgr, &engine).await;
    assert!(handled, "the pet module handles method {method}");
    drop(tx);
    let mut sent = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        sent.push(msg);
    }
    Sent(sent)
}

fn invoke_args(pet: u32, ability: i32, target: u32) -> Vec<u8> {
    let mut a = Vec::with_capacity(12);
    a.extend_from_slice(&(pet as i32).to_le_bytes());
    a.extend_from_slice(&ability.to_le_bytes());
    a.extend_from_slice(&(target as i32).to_le_bytes());
    a
}

fn toggle_args(pet: u32, ability: i32, toggle: i8) -> Vec<u8> {
    let mut a = Vec::with_capacity(9);
    a.extend_from_slice(&(pet as i32).to_le_bytes());
    a.extend_from_slice(&ability.to_le_bytes());
    a.push(toggle as u8);
    a
}

fn stance_args(pet: u32, stance: i8) -> Vec<u8> {
    let mut a = (pet as i32).to_le_bytes().to_vec();
    a.push(stance as u8);
    a
}

async fn invoke(mgr: &mut SpaceManager, caller: u32, pet: u32, ability: i32, target: u32) -> Sent {
    call(
        mgr,
        caller,
        PET_INVOKE_ABILITY,
        &invoke_args(pet, ability, target),
    )
    .await
}

async fn toggle(mgr: &mut SpaceManager, caller: u32, pet: u32, ability: i32, on: i8) -> Sent {
    call(
        mgr,
        caller,
        PET_ABILITY_TOGGLE,
        &toggle_args(pet, ability, on),
    )
    .await
}

async fn stance(mgr: &mut SpaceManager, caller: u32, pet: u32, raw: i8) -> Sent {
    call(mgr, caller, PET_CHANGE_STANCE, &stance_args(pet, raw)).await
}
