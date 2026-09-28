//! Pets PT-03: summoning a pet through `useAbility`.
//!
//! The fixture mirrors the seed: 2826 Summon Straegis (warmup 6 s, cooldown
//! 5 s, flags 18192, target Self, event set 1121 with End 2292 and Interrupt
//! 2904), a `pet_summons` row 2826 -> 350 (`max_active` 1), and a cached pet
//! template 350. The owner is a connected player in a shared space.

use std::time::{Duration, Instant};

use cimmeria_cell_world::cell::pets::drain_arrivals;
use cimmeria_cell_world::test_fixtures::{add_pet_owner, seed_pet_template};
use cimmeria_common::EntityId;
use cimmeria_entity::abilities::{AF_CHANNEL_ALLOWS_MOVEMENT, AF_SPEED_PET};
use cimmeria_entity::stats::SPEED_PET;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use super::warmup::{calls, sequences};
use super::*;
use crate::cell::abilities::resolve_warmups;
use crate::cell::spawner::{PetSummon, PetSummonCatalog};
use crate::mercury::SGWPET_CLASS_ID;
use crate::test_support::NoContentEvents;

pub(super) const OWNER: u32 = 1;
const WATCHER: u32 = 3;
pub(super) const SUMMON: i32 = 2826;
pub(super) const TEMPLATE: i32 = 350;
const SUMMON_SET: i32 = 1121;
const SEQ_END: i32 = 2292;
pub(super) const SEQ_INTERRUPT: i32 = 2904;
const TARGET_SET: i32 = 1122;
const SEQ_TARGET: i32 = 2293;
/// Seeded flags of 2826: SpeedPet | Deactivate_AutoCycle |
/// DoNotActivate_AutoCycle | ForceStanding | Response.
const SUMMON_FLAGS: u32 = 18192;
pub(super) const WARMUP: f32 = 6.0;

pub(super) fn summon_def(id: i32) -> AbilityDef {
    AbilityDef {
        ability_id: id,
        name: "Summon Straegis".to_string(),
        cooldown: 5.0,
        warmup: WARMUP,
        flags: SUMMON_FLAGS,
        is_ranged: false,
        min_range: 0.0,
        max_range: 0.0,
        target_type_id: 1,
        effect_ids: vec![],
        moniker_ids: vec![],
        required_ammo: 0,
        event_set_id: Some(SUMMON_SET),
        velocity: 100.0,
    }
}

/// The seeded world: 2826 known by `OWNER`, its summon row and template,
/// and the two event sets' sequences.
pub(super) fn summon_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    add_pet_owner(&mut mgr, OWNER, "Castle", [0.0, 0.0, 0.0], 50);
    mgr.get_entity_mut(OWNER)
        .unwrap()
        .abilities
        .add_ability(SUMMON);
    mgr.ability_defs.insert(SUMMON, summon_def(SUMMON));
    mgr.pet_summons = PetSummonCatalog::from_rows([PetSummon {
        ability_id: SUMMON,
        template_id: TEMPLATE,
        max_active: 1,
    }]);
    seed_pet_template(&mut mgr, TEMPLATE);
    mgr.sequence_map.insert((SUMMON_SET, 1001), SEQ_END);
    mgr.sequence_map.insert((SUMMON_SET, 1002), SEQ_INTERRUPT);
    mgr.sequence_map.insert((TARGET_SET, 2000), SEQ_TARGET);
    mgr
}

pub(super) fn after_summon_warmup() -> Instant {
    Instant::now() + Duration::from_secs_f32(WARMUP) + Duration::from_millis(50)
}

/// Cast, check the launch committed with no pet yet, then run the warmup
/// tick past the warmup. Returns everything sent.
pub(super) async fn cast_and_complete(
    mgr: &mut SpaceManager,
    target_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
) -> Vec<CellToBaseMsg> {
    let pets_before = mgr.pets.pets_of(OWNER);
    assert!(
        handle_use_ability(OWNER, SUMMON, target_id, tx, mgr).await,
        "the summon must commit"
    );
    assert_eq!(
        mgr.pets.pets_of(OWNER),
        pets_before,
        "nothing spawns before the warmup expires"
    );
    assert_eq!(
        resolve_warmups(after_summon_warmup(), tx, mgr, &NoContentEvents).await,
        1
    );
    drain(rx)
}

/// Clear the cooldown so a test can cast again at once.
pub(super) fn ready_again(mgr: &mut SpaceManager) {
    mgr.get_entity_mut(OWNER)
        .unwrap()
        .abilities
        .clear_ability_cooldown(SUMMON);
}

/// **Acceptance: casting 2826 spawns template 350 owned by the caster**,
/// after the 6 s warmup and not before, as an `SGWPet` recording the
/// summon ability. The source sequence 2292 plays on the caster with
/// TargetID = caster (python `targetId or ent.entityId`), and the cooldown
/// is charged.
#[tokio::test]
async fn summon_spawns_the_template_owned_by_the_caster_after_the_warmup() {
    let mut mgr = summon_mgr();
    let (tx, mut rx) = mpsc::channel(256);

    let sent = cast_and_complete(&mut mgr, OWNER as i32, &tx, &mut rx).await;

    let pets = mgr.pets.pets_of(OWNER);
    assert_eq!(pets.len(), 1, "exactly one pet");
    let pet = mgr.get_entity(pets[0]).expect("pet entity exists");
    assert_eq!(pet.class_id, SGWPET_CLASS_ID);
    let state = pet.pet.as_deref().expect("PetState");
    assert_eq!(state.owner_id, OWNER);
    assert_eq!(state.summon_ability_id, SUMMON);
    assert_eq!(
        mgr.owned_pet(OWNER, pets[0]),
        Ok(pets[0]),
        "the ownership guard answers for the new pet"
    );
    assert!(mgr
        .get_entity(OWNER)
        .unwrap()
        .abilities
        .is_on_cooldown(SUMMON));

    let end = calls(&sent)
        .into_iter()
        .find(|(e, m, a)| {
            *e == OWNER && *m == method_idx::ON_SEQUENCE && a[0..4] == SEQ_END.to_le_bytes()
        })
        .expect("Ability_End 2292 on the caster");
    assert_eq!(&end.2[4..8], &(OWNER as i32).to_le_bytes(), "SourceID");
    assert_eq!(
        &end.2[8..12],
        &(OWNER as i32).to_le_bytes(),
        "TargetID = caster"
    );
}

/// **Revert guard on the #444 gate.** A Self ability with **no**
/// `pet_summons` row, aimed at the caster, is still refused before the
/// cooldown, exactly as before PT-03. Fails if the gate is widened to let
/// self-targets through, or if the summon diversion answers for abilities
/// without a row.
#[tokio::test]
async fn non_summon_self_cast_is_still_rejected_by_the_gate() {
    let mut mgr = summon_mgr();
    const OTHER_SELF: i32 = 3491; // a "Summon Straegis" copy with no row
    mgr.ability_defs.insert(OTHER_SELF, summon_def(OTHER_SELF));
    mgr.get_entity_mut(OWNER)
        .unwrap()
        .abilities
        .add_ability(OTHER_SELF);
    let (tx, _rx) = mpsc::channel(64);

    assert!(
        !handle_use_ability(OWNER, OTHER_SELF, OWNER as i32, &tx, &mut mgr).await,
        "a self-targeted ability without a summon row must fail the #444 gate"
    );
    let owner = mgr.get_entity(OWNER).unwrap();
    assert!(!owner.abilities.is_on_cooldown(OTHER_SELF));
    assert!(owner.pending_cast.is_none());
    assert!(mgr.pets.is_empty());
}

/// **Acceptance: a second cast replaces the first pet** (D-PT04). The
/// first pet is despawned (gone from the world and the registry) and one
/// pet remains.
#[tokio::test]
async fn second_summon_replaces_the_first_pet() {
    let mut mgr = summon_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    cast_and_complete(&mut mgr, 0, &tx, &mut rx).await;
    let first = mgr.pets.pets_of(OWNER)[0];

    ready_again(&mut mgr);
    cast_and_complete(&mut mgr, 0, &tx, &mut rx).await;

    let pets = mgr.pets.pets_of(OWNER);
    assert_eq!(pets.len(), 1, "one active pet per owner");
    assert_ne!(pets[0], first, "the new pet replaced the first");
    assert!(
        mgr.get_entity(first).is_none(),
        "the first pet is despawned"
    );
    assert!(!mgr.pets.is_pet(first));
}

/// **Acceptance: an interrupted warmup spawns nothing.** Walking off the
/// spot interrupts the summon warmup. Also guards the flag fix: while
/// `AF_CHANNEL_ALLOWS_MOVEMENT` was bit 14 (the client's `SpeedPet`), every
/// summon was exempt from the move interrupt and this pet spawned.
#[tokio::test]
async fn interrupted_summon_warmup_spawns_nothing() {
    assert_eq!(
        SUMMON_FLAGS & AF_CHANNEL_ALLOWS_MOVEMENT,
        0,
        "a summon must not carry the move-exemption flag"
    );
    let mut mgr = summon_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    assert!(handle_use_ability(OWNER, SUMMON, 0, &tx, &mut mgr).await);
    drain(&mut rx);

    mgr.get_entity_mut(OWNER).unwrap().position.x += 2.0;
    assert_eq!(
        resolve_warmups(after_summon_warmup(), &tx, &mut mgr, &NoContentEvents).await,
        0,
        "the moved caster's summon must not fire"
    );
    let sent = drain(&mut rx);
    assert!(mgr.pets.is_empty(), "an interrupted summon spawned a pet");
    assert!(mgr.get_entity(OWNER).unwrap().pending_cast.is_none());
    assert!(
        sequences(&sent, OWNER).contains(&SEQ_INTERRUPT),
        "the interrupt plays 2904"
    );
}

/// A forged target on the summon is discarded: the cast is parked with
/// target 0 (so no fire-time target check can trip on it) and the pet still
/// spawns. The neutral NPC is never touched.
#[tokio::test]
async fn forged_target_on_a_summon_is_discarded() {
    let mut mgr = summon_mgr();
    mgr.create_entity(2, "Castle", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap(); // neutral NPC
    let (tx, mut rx) = mpsc::channel(256);

    assert!(handle_use_ability(OWNER, SUMMON, 2, &tx, &mut mgr).await);
    assert_eq!(
        mgr.get_entity(OWNER)
            .unwrap()
            .pending_cast
            .as_ref()
            .unwrap()
            .target_id,
        0
    );
    assert_eq!(
        resolve_warmups(after_summon_warmup(), &tx, &mut mgr, &NoContentEvents).await,
        1
    );
    drain(&mut rx);
    assert_eq!(mgr.pets.pets_of(OWNER).len(), 1);
}

/// The `onErrorCode` + `CHAN_FEEDBACK` pair a refused summon sends.
fn feedback(sent: &[CellToBaseMsg], code: u16, text: &str) -> bool {
    let mut err = vec![0u8];
    err.extend_from_slice(&SUMMON.to_le_bytes());
    err.extend_from_slice(&code.to_le_bytes());
    let chat = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
    let c = calls(sent);
    c.contains(&(OWNER, method_idx::ON_ERROR_CODE, err))
        && c.contains(&(OWNER, method_idx::ON_PLAYER_COMMUNICATION, chat))
}

/// **Feedback on failure (launch).** The template is not cached: the press
/// is refused before the cooldown, with `onErrorCode` and a visible chat
/// line, and no warmup starts.
#[tokio::test]
async fn summon_with_a_missing_template_is_refused_with_feedback() {
    let mut mgr = summon_mgr();
    mgr.spawn_templates.remove(&TEMPLATE);
    let (tx, mut rx) = mpsc::channel(64);

    assert!(!handle_use_ability(OWNER, SUMMON, 0, &tx, &mut mgr).await);
    let sent = drain(&mut rx);
    assert!(
        feedback(&sent, 0, super::super::summon::SUMMON_FAILED_TEXT),
        "the refused press must be answered: {sent:?}"
    );
    let owner = mgr.get_entity(OWNER).unwrap();
    assert!(!owner.abilities.is_on_cooldown(SUMMON));
    assert!(owner.pending_cast.is_none());
}

/// **Feedback on failure (fire).** The template vanishes during the
/// warmup: nothing spawns, the owner keeps its current pet, and the player
/// sees the cast cancelled (2904) and the feedback line.
#[tokio::test]
async fn summon_that_cannot_spawn_after_its_warmup_keeps_the_old_pet() {
    let mut mgr = summon_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    cast_and_complete(&mut mgr, 0, &tx, &mut rx).await;
    let first = mgr.pets.pets_of(OWNER)[0];

    ready_again(&mut mgr);
    assert!(handle_use_ability(OWNER, SUMMON, 0, &tx, &mut mgr).await);
    mgr.spawn_templates.remove(&TEMPLATE);
    assert_eq!(
        resolve_warmups(after_summon_warmup(), &tx, &mut mgr, &NoContentEvents).await,
        1
    );
    let sent = drain(&mut rx);

    assert_eq!(mgr.pets.pets_of(OWNER), vec![first], "the old pet stays");
    assert!(sequences(&sent, OWNER).contains(&SEQ_INTERRUPT));
    assert!(!sequences(&sent, OWNER).contains(&SEQ_END));
    assert!(feedback(&sent, 0, super::super::summon::SUMMON_FAILED_TEXT));
}

/// **Feedback on failure (spawn).** The warmup completes and every
/// fire-time check passes, but `spawn_pet_from_template` itself refuses.
/// The player sees the cast cancelled (2904, never the End 2292) and the
/// feedback line, like any other failed cast, and keeps the pet it had:
/// the spawn runs before End and before the old pet is retired. Before the
/// fix this path played End, despawned the old pet, then sent only the
/// feedback line.
///
/// A caster that is not a player is the one spawn failure a fixture can
/// reach past `fire_refusal` (`OwnerNotPlayer`), so this drives
/// `fire_summon` directly, as `fire_cast` does after the warmup. With the
/// caster no longer a player, the sequences are read off a watcher's
/// `WitnessEntityMethod`s (the self send is player-only).
#[tokio::test]
async fn summon_whose_spawn_fails_plays_the_interrupt_and_keeps_the_old_pet() {
    let mut mgr = summon_mgr();
    add_pet_owner(&mut mgr, WATCHER, "Castle", [4.0, 0.0, 0.0], 10);
    let (tx, mut rx) = mpsc::channel(256);
    cast_and_complete(&mut mgr, 0, &tx, &mut rx).await;
    let first = mgr.pets.pets_of(OWNER)[0];
    let _ = mgr.compute_aoi_changes();
    assert!(
        mgr.get_witnesses_of(OWNER).contains(&WATCHER),
        "fixture: the watcher sees the caster"
    );
    mgr.get_entity_mut(OWNER).unwrap().is_player = false;
    let summon = mgr.pet_summons.pet_summon_for(SUMMON).unwrap();

    super::super::summon::fire_summon(
        OWNER,
        SUMMON,
        1,
        summon,
        &Some(summon_def(SUMMON)),
        &tx,
        &mut mgr,
    )
    .await;
    let sent = drain(&mut rx);

    assert_eq!(mgr.pets.pets_of(OWNER), vec![first], "the old pet stays");
    assert!(mgr.get_entity(first).is_some(), "and is still in the world");
    let seqs = sequences(&sent, OWNER);
    assert!(
        seqs.contains(&SEQ_INTERRUPT),
        "the cast bar closes as cancelled: {seqs:?}"
    );
    assert!(!seqs.contains(&SEQ_END), "never as completed: {seqs:?}");
    assert!(
        feedback(&sent, 0, super::super::summon::SUMMON_FAILED_TEXT),
        "the failed summon must be answered: {sent:?}"
    );
}

/// D-PT10: `speedPet` shortens a `SpeedPet`-flagged warmup like the other
/// speed stats; at 0 the warmup is the seeded 6 s.
#[test]
fn speed_pet_stat_shortens_the_summon_warmup() {
    let mut mgr = summon_mgr();
    let def = summon_def(SUMMON);
    assert_ne!(def.flags & AF_SPEED_PET, 0);
    let w = super::super::warmup::effective_warmup(Some(&def), mgr.get_entity(OWNER).unwrap());
    assert!((w - WARMUP).abs() < 1e-5, "stat 0 leaves 6 s, got {w}");
    if let Some(stat) = mgr.get_entity_mut(OWNER).unwrap().stats.get_mut(SPEED_PET) {
        stat.update(0, 50, 100);
    }
    let w = super::super::warmup::effective_warmup(Some(&def), mgr.get_entity(OWNER).unwrap());
    assert!((w - 3.0).abs() < 1e-5, "50% off 6 s, got {w}");
}

/// A summon (`DoNotActivate_AutoCycle` / `Deactivate_AutoCycle`) is not
/// stashed as the last-fired ability, so a later `setAutoCycle(1)` press
/// cannot re-fire it.
#[tokio::test]
async fn summon_is_not_stashed_for_auto_cycle() {
    let mut mgr = summon_mgr();
    let (tx, _rx) = mpsc::channel(64);
    assert!(handle_use_ability(OWNER, SUMMON, 0, &tx, &mut mgr).await);
    assert_eq!(
        mgr.get_entity(OWNER)
            .unwrap()
            .abilities
            .last_fired_ability_id,
        None
    );
}

/// The target VFX 2293 waits for the pet's intro, then reaches every
/// witness of the pet, byte-exact: `onSequence` on the pet, SourceID =
/// owner, TargetID = pet, InstanceId 0 (python effect sequences). A player
/// who does not see the pet gets nothing.
#[tokio::test]
async fn summon_vfx_waits_for_the_intro_then_reaches_the_pets_witnesses() {
    let mut mgr = summon_mgr();
    add_pet_owner(&mut mgr, WATCHER, "Castle", [4.0, 0.0, 0.0], 10);
    add_pet_owner(&mut mgr, 4, "Castle", [700.0, 0.0, 700.0], 10); // far away
    let (tx, mut rx) = mpsc::channel(256);
    let sent = cast_and_complete(&mut mgr, 0, &tx, &mut rx).await;
    let pet = mgr.pets.pets_of(OWNER)[0];
    assert!(
        !sequences(&sent, pet).contains(&SEQ_TARGET),
        "the VFX must not go out before the pet's CREATE_ENTITY"
    );
    assert_eq!(
        drain_arrivals(Instant::now(), &tx, &mut mgr).await,
        0,
        "not yet introduced"
    );

    let intro = mgr.compute_aoi_changes();
    assert!(intro.iter().any(|m| matches!(
        m,
        CellToBaseMsg::EnteredAoI { witness_id: OWNER, entity_id, .. } if *entity_id == pet
    )));
    assert!(mgr
        .get_entity(OWNER)
        .unwrap()
        .witnesses
        .contains(&EntityId(pet as i32)));

    assert_eq!(drain_arrivals(Instant::now(), &tx, &mut mgr).await, 1);
    let vfx: Vec<(u32, Vec<u8>)> = drain(&mut rx)
        .into_iter()
        .filter_map(|m| match m {
            CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index,
                args,
                entity_is_player,
            } if entity_id == pet && method_index == method_idx::ON_SEQUENCE => {
                assert!(!entity_is_player);
                Some((witness_id, args))
            }
            _ => None,
        })
        .collect();
    let mut expected = Vec::new();
    expected.extend_from_slice(&SEQ_TARGET.to_le_bytes());
    expected.extend_from_slice(&(OWNER as i32).to_le_bytes());
    expected.extend_from_slice(&(pet as i32).to_le_bytes());
    expected.push(1);
    expected.extend_from_slice(&0.0f32.to_le_bytes());
    expected.extend_from_slice(&0u32.to_le_bytes());
    expected.push(0);
    expected.extend_from_slice(&0i32.to_le_bytes());
    let mut witnesses: Vec<u32> = vfx.iter().map(|(w, _)| *w).collect();
    witnesses.sort_unstable();
    assert_eq!(
        witnesses,
        vec![OWNER, WATCHER],
        "the pet's witnesses, and only them"
    );
    for (_, args) in &vfx {
        assert_eq!(args, &expected);
    }
    assert_eq!(
        drain_arrivals(Instant::now(), &tx, &mut mgr).await,
        0,
        "sent once"
    );
}
