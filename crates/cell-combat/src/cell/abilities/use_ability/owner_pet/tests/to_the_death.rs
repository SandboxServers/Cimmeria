//! 2839 To The Death: the pet gets +400 Accuracy for 60 s, then dies through
//! the ordinary pet-death path, paying nobody.

use std::time::{Duration, Instant};

use cimmeria_cell_world::cell::pets::{pet_owner_sweep_at, PET_CORPSE_DESPAWN};
use cimmeria_entity::stats::{ACCURACY, HEALTH};
use cimmeria_wire::state_field::BSF_DEAD;
use tokio::sync::mpsc;

use super::*;
use crate::cell::abilities::use_ability::handle_use_ability;
use crate::cell::abilities::use_ability::owner_pet::owner_pet_tick_at;
use crate::cell::abilities::{kill_npc_out_of_band, resolve_warmups};
use crate::test_support::NoContentEvents;

fn is_dead(mgr: &SpaceManager, entity: u32) -> bool {
    mgr.get_entity(entity)
        .is_some_and(|e| e.state_field & BSF_DEAD != 0)
}

fn grants_xp(msgs: &[CellToBaseMsg]) -> bool {
    msgs.iter()
        .any(|m| matches!(m, CellToBaseMsg::GrantXP { .. }))
}

/// Cast To The Death and run its 2 s warmup. Returns the cast time.
async fn doom(
    mgr: &mut SpaceManager,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
) -> (Instant, Vec<CellToBaseMsg>) {
    let cast_at = Instant::now();
    assert!(handle_use_ability(OWNER, TO_THE_DEATH, 0, tx, mgr).await);
    assert_eq!(
        resolve_warmups(after_warmup(), tx, mgr, &NoContentEvents).await,
        1
    );
    (cast_at, drain(rx))
}

/// **Acceptance.** After the warmup the pet has +400 Accuracy and the owner
/// is told it will die. At 59 s nothing has happened; at 61 s the buff is
/// gone and the pet is a corpse, killed through `resolve_death` with no
/// `GrantXP` to anyone. The corpse then follows the ordinary pet path: the
/// sweep starts its 10 s timer and despawns it. Fails when the doom is
/// removed (the pet lives) or the kill pays XP.
#[tokio::test]
async fn to_the_death_buffs_the_pet_then_kills_it_with_no_credit() {
    let (mut mgr, pet) = world();
    let (tx, mut rx) = mpsc::channel(512);

    let (cast_at, sent) = doom(&mut mgr, &tx, &mut rx).await;
    assert_eq!(stat(&mgr, pet, ACCURACY), 400);
    assert!(got_line(
        &sent,
        OWNER,
        "Your pet fights to the death: it dies in 60 seconds."
    ));

    assert_eq!(
        owner_pet_tick_at(cast_at + Duration::from_secs(59), &tx, &mut mgr).await,
        0
    );
    assert!(!is_dead(&mgr, pet));
    assert_eq!(stat(&mgr, pet, ACCURACY), 400);

    let doom_at = cast_at + Duration::from_secs(63);
    assert_eq!(owner_pet_tick_at(doom_at, &tx, &mut mgr).await, 1);
    let sent = drain(&mut rx);
    assert!(is_dead(&mgr, pet), "the pet dies when the timer runs out");
    assert_eq!(stat(&mgr, pet, HEALTH), 0);
    assert_eq!(stat(&mgr, pet, ACCURACY), 0, "the +400 lapses with it");
    assert!(!grants_xp(&sent), "a doomed pet's death pays nobody");
    assert!(
        mgr.pets.pets_of(OWNER).contains(&pet),
        "a corpse first, not an instant despawn"
    );

    // The ordinary pet-death path (D-PT08): corpse timer, then despawn.
    let _ = pet_owner_sweep_at(doom_at, &tx, &mut mgr).await;
    assert!(mgr.get_entity(pet).is_some());
    let _ = pet_owner_sweep_at(
        doom_at + PET_CORPSE_DESPAWN + Duration::from_millis(100),
        &tx,
        &mut mgr,
    )
    .await;
    assert!(mgr.get_entity(pet).is_none(), "the corpse despawns");
    assert!(mgr.pets.pets_of(OWNER).is_empty());
}

/// A re-cast while the pet is doomed is refused with feedback and no
/// cooldown. Python's refresh would restart the timer, and with a 30 s
/// cooldown against a 60 s doom that is +400 Accuracy forever.
#[tokio::test]
async fn to_the_death_is_refused_while_it_runs() {
    let (mut mgr, pet) = world();
    let (tx, mut rx) = mpsc::channel(512);
    let _ = doom(&mut mgr, &tx, &mut rx).await;
    let doomed_at = mgr.get_entity(pet).unwrap().pet.as_ref().unwrap().doomed_at;
    ready_again(&mut mgr);

    assert!(!handle_use_ability(OWNER, TO_THE_DEATH, 0, &tx, &mut mgr).await);
    let sent = drain(&mut rx);
    assert_eq!(error_codes(&sent, OWNER), vec![(TO_THE_DEATH, 133)]);
    assert!(got_line(
        &sent,
        OWNER,
        "Your pet is already fighting to the death."
    ));
    assert!(!on_cooldown(&mgr, TO_THE_DEATH), "no cooldown charged");
    assert_eq!(
        mgr.get_entity(pet).unwrap().pet.as_ref().unwrap().doomed_at,
        doomed_at,
        "the timer is not restarted"
    );
}

/// A pet a mob killed before its doom ran out is not killed again: the tick
/// skips it and no second death burst goes out.
#[tokio::test]
async fn a_pet_that_died_first_is_not_killed_twice() {
    let (mut mgr, pet) = world();
    let (tx, mut rx) = mpsc::channel(512);
    let (cast_at, _) = doom(&mut mgr, &tx, &mut rx).await;
    assert!(kill_npc_out_of_band(pet, pet, false, false, &tx, &mut mgr).await);
    let _ = drain(&mut rx);

    assert_eq!(
        owner_pet_tick_at(cast_at + Duration::from_secs(63), &tx, &mut mgr).await,
        0
    );
    assert!(
        mgr.get_entity(pet)
            .unwrap()
            .pet
            .as_ref()
            .unwrap()
            .doomed_at
            .is_none(),
        "the doom is spent"
    );
    let sent = drain(&mut rx);
    assert!(
        !sent.iter().any(|m| matches!(m, CellToBaseMsg::WitnessEntityMethod {
            entity_id, method_index, ..
        } if *entity_id == pet && *method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE)),
        "no second death burst"
    );
}

/// The pet dies during the owner's 2 s warmup: the cast is interrupted at
/// the fire, the owner is told, and nothing is doomed.
#[tokio::test]
async fn a_pet_that_dies_during_the_warmup_interrupts_the_cast() {
    let (mut mgr, pet) = world();
    let (tx, mut rx) = mpsc::channel(512);
    assert!(handle_use_ability(OWNER, TO_THE_DEATH, 0, &tx, &mut mgr).await);
    assert!(kill_npc_out_of_band(pet, pet, false, false, &tx, &mut mgr).await);
    let _ = drain(&mut rx);

    let _ = resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await;
    let sent = drain(&mut rx);
    assert_eq!(error_codes(&sent, OWNER), vec![(TO_THE_DEATH, 14)]);
    assert!(got_line(&sent, OWNER, "Your pet is dead."));
    assert!(mgr
        .get_entity(pet)
        .unwrap()
        .pet
        .as_ref()
        .unwrap()
        .doomed_at
        .is_none());
    assert_eq!(stat(&mgr, pet, ACCURACY), 0);
}
