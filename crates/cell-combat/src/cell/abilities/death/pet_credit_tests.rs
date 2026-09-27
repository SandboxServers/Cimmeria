//! Pets PT-06: who a kill's XP and loot belong to when a pet is involved.
//!
//! Every kill funnels through [`super::resolve_death`], so these drive the
//! real resolver through [`super::kill_npc_out_of_band`] (the same call the
//! DoT pulse and `damage_apply`'s two arms reduce to) and read the
//! `GrantXP` messages it sends. Before the `credit_recipient` seam a pet kill
//! sent `GrantXP` to the pet's id, which has no base session, and a mob
//! that killed a pet sent `GrantXP` to the mob's id.

use super::side_effects::{kill_xp_payout, KillXpPayout, NoKillXp};
use super::*;
use crate::cell::combat::HOSTILE_FACTION;
use crate::cell::spawner::LootTableEntry;
use crate::test_support::{add_pet_owner, seed_pet_template, PET_FIXTURE_TEMPLATE_ID};
use cimmeria_entity::cell_entity::NpcInteractionType;

use super::super::loot_drop::INT_NORMAL_LOOT;

pub(super) const OWNER: u32 = 7;
pub(super) const OWNER_PLAYER_ID: i32 = 700;
/// Mob level 5 pays `kill_xp(5)` = 50 XP.
const MOB_LEVEL: u32 = 5;
pub(super) const MOB_XP: u64 = 50;
const LOOT_TABLE: i32 = 0x7000_0601;

/// `OWNER` (a connected player) in the shared Castle space with one pet
/// summoned at its side, and one hostile level-5 mob. Returns
/// `(mgr, pet, mob)`.
pub(super) fn world() -> (SpaceManager, u32, u32) {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    seed_pet_template(&mut mgr, PET_FIXTURE_TEMPLATE_ID);
    add_pet_owner(&mut mgr, OWNER, "Castle", [0.0; 3], 12);
    mgr.get_entity_mut(OWNER).unwrap().player_id = Some(OWNER_PLAYER_ID);
    let pet = mgr
        .spawn_pet_from_template(OWNER, PET_FIXTURE_TEMPLATE_ID, 2826)
        .expect("fixture: the pet spawns");
    let mob = spawn_mob(&mut mgr);
    let _ = mgr.compute_aoi_changes();
    (mgr, pet, mob)
}

pub(super) fn spawn_mob(mgr: &mut SpaceManager) -> u32 {
    let mob = mgr.allocate_npc_id();
    mgr.spawn_npc(mob, "Castle", [4.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(mob).unwrap();
    e.level = MOB_LEVEL;
    e.faction = HOSTILE_FACTION;
    mob
}

/// Every `GrantXP` in the channel as `(entity_id, xp_amount)`.
fn grants(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<(u32, u64)> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        if let CellToBaseMsg::GrantXP {
            entity_id,
            xp_amount,
            ..
        } = m
        {
            out.push((entity_id, xp_amount));
        }
    }
    out
}

pub(super) async fn kill(mgr: &mut SpaceManager, target: u32, attacker: u32) -> Vec<(u32, u64)> {
    let (tx, mut rx) = mpsc::channel(512);
    assert!(
        kill_npc_out_of_band(target, attacker, false, true, &tx, mgr).await,
        "fixture: the kill must resolve"
    );
    grants(&mut rx)
}

/// **Guard (A-27).** A pet's kill pays its owner, and never the pet. With
/// the seam reverted the single `GrantXP` goes to the pet's id.
#[tokio::test]
async fn a_pet_kill_grants_the_kill_xp_to_the_owner() {
    let (mut mgr, pet, mob) = world();
    assert_eq!(kill(&mut mgr, mob, pet).await, vec![(OWNER, MOB_XP)]);
}

/// `transfer_xp` scales the owner's share. D-PT02 ships 1.0, which cannot
/// tell "scaled" from "not scaled", so this sets 0.5 to prove the factor is
/// read at all.
#[tokio::test]
async fn a_pet_kill_is_scaled_by_transfer_xp() {
    let (mut mgr, pet, mob) = world();
    mgr.get_entity_mut(pet)
        .unwrap()
        .pet
        .as_mut()
        .unwrap()
        .transfer_xp = 0.5;
    assert_eq!(kill(&mut mgr, mob, pet).await, vec![(OWNER, MOB_XP / 2)]);
}

/// **Guard (A-27).** A mob that kills a pet earns nothing, and no
/// `GrantXP` goes to the mob's id. Reverted, the resolver sends
/// `GrantXP { entity_id: mob }`.
#[tokio::test]
async fn a_mob_killing_a_pet_sends_no_grant_xp() {
    let (mut mgr, pet, mob) = world();
    assert_eq!(kill(&mut mgr, pet, mob).await, vec![]);
}

/// **Guard.** An ordinary NPC attacker never gets `GrantXP`, whoever it
/// kills.
#[tokio::test]
async fn an_npc_killing_an_npc_sends_no_grant_xp() {
    let (mut mgr, _pet, mob) = world();
    let other = spawn_mob(&mut mgr);
    assert_eq!(kill(&mut mgr, other, mob).await, vec![]);
}

/// Control: the player path is unchanged, full XP to the player.
#[tokio::test]
async fn a_player_kill_still_pays_the_player_in_full() {
    let (mut mgr, _pet, mob) = world();
    assert_eq!(kill(&mut mgr, mob, OWNER).await, vec![(OWNER, MOB_XP)]);
}

/// A pet whose owner is no longer registered to it (torn down between the
/// hit and the sweep) credits nobody rather than the pet.
#[tokio::test]
async fn an_orphaned_pet_kill_pays_nobody() {
    let (mut mgr, pet, mob) = world();
    mgr.pets.forget_pet(pet);
    assert_eq!(kill(&mut mgr, mob, pet).await, vec![]);
}

/// Loot from a pet kill. The corpse rolls its loot table exactly as it
/// would for a player kill. The server has no per-corpse loot owner: any
/// player in interact range may open a corpse (`handle_interact` /
/// `handle_loot_item` gate on range and `player_id` only), so "owned by the
/// owner" holds by the owner being a player who can loot it. This pins
/// that a pet killer does not suppress the roll.
#[tokio::test]
async fn a_pet_kill_rolls_the_corpse_loot() {
    let (mut mgr, pet, mob) = world();
    mgr.get_entity_mut(mob).unwrap().loot_table_id = Some(LOOT_TABLE);
    mgr.loot_tables.insert(
        LOOT_TABLE,
        vec![LootTableEntry {
            design_id: Some(7),
            min_quantity: 1,
            max_quantity: 1,
            probability: 1.0,
        }],
    );
    let _ = kill(&mut mgr, mob, pet).await;
    let corpse = mgr.get_entity(mob).unwrap();
    assert_eq!(corpse.loot.len(), 1, "the loot table must roll");
    assert_ne!(corpse.interaction_type_flags & INT_NORMAL_LOOT, 0);
    assert!(matches!(
        corpse.interaction_type,
        Some(NpcInteractionType::Loot)
    ));
}

/// The payout arithmetic: a player is paid in full, a pet's scale rounds,
/// and a zero, negative or non-finite scale fails closed to nothing.
#[test]
fn kill_xp_payout_fails_closed_on_a_bad_scale() {
    let (mut mgr, pet, mob) = world();
    assert_eq!(
        kill_xp_payout(&mgr, OWNER, 50),
        Ok(KillXpPayout {
            recipient: OWNER,
            xp: 50
        })
    );
    assert_eq!(
        kill_xp_payout(&mgr, mob, 50),
        Err(NoKillXp::NpcAttacker),
        "a mob credits nobody"
    );
    for (scale, want) in [
        (1.0, Some(50)),
        (0.25, Some(13)),
        (0.0, None),
        (-1.0, None),
        (f32::NAN, None),
        (f32::INFINITY, None),
    ] {
        mgr.get_entity_mut(pet)
            .unwrap()
            .pet
            .as_mut()
            .unwrap()
            .transfer_xp = scale;
        assert_eq!(
            kill_xp_payout(&mgr, pet, 50)
                .ok()
                .map(|p| (p.recipient, p.xp)),
            want.map(|xp| (OWNER, xp)),
            "transfer_xp = {scale}"
        );
    }
}
