//! Ownership is the pet's summoner, not the owner's entity id (PT-01's
//! per-pet capture). Entity ids are reused: after the owner is destroyed,
//! its id can come back as another player before `pet_owner_sweep` takes the
//! pet. The old pet must not follow, teleport to, leash to or put in combat
//! the id's new holder; it holds until the sweep despawns it.

use super::*;
use crate::cell::combat::BSF_IN_COMBAT;
use crate::test_support::LogCapture;

/// Destroy `OWNER` and hand its id to a different player (another account
/// and character) at `holder_pos`.
fn reuse_owner_id(mgr: &mut SpaceManager, holder_pos: [f32; 3]) {
    mgr.destroy_entity(OWNER);
    add_pet_owner(mgr, OWNER, "Agnos", holder_pos, 12);
    let holder = mgr.get_entity_mut(OWNER).unwrap();
    holder.account_id = Some(9001);
    holder.player_id = Some(9002);
}

/// The pet's fight does not put the id's new holder in combat, and the pet
/// logs why it holds, naming its real summoner.
#[tokio::test]
async fn a_reused_owner_id_is_not_put_in_combat_by_the_old_pet() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [14.0, 0.0, 10.0], HOSTILE);
    mob_fights(&mut mgr, MOB, pet);
    reuse_owner_id(&mut mgr, [11.0, 0.0, 10.0]);

    let logs = LogCapture::install();
    let msgs = tick(&mut mgr).await;

    let holder = mgr.get_entity(OWNER).unwrap();
    assert!(
        holder.threatened_mobs.is_empty(),
        "{:?}",
        holder.threatened_mobs
    );
    assert_eq!(holder.state_field & BSF_IN_COMBAT, 0);
    assert!(state_update_to(&msgs, OWNER).is_none());
    let row = pets_ai_row(&logs, "pet_owner_missing").expect("owner_missing row");
    assert!(
        row.has_field("reason", "owner_identity_mismatch"),
        "{row:?}"
    );
    assert_owner_identity(&row);
}

/// A new holder far away does not pull the old pet across the map.
#[tokio::test]
async fn a_reused_owner_id_is_not_teleported_to() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    let before = pos(&mgr, pet);
    reuse_owner_id(&mut mgr, [300.0, 0.0, 300.0]);

    tick(&mut mgr).await;

    assert_eq!(pos(&mgr, pet), before, "the old pet stays put");
}

/// A fighting pet's leash is not anchored on the id's new holder.
#[tokio::test]
async fn a_reused_owner_id_is_no_leash_anchor() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    assert!(
        super::super::leash_anchor(&mgr, mgr.get_entity(pet).unwrap()).is_some(),
        "precondition: the real owner anchors the leash"
    );
    reuse_owner_id(&mut mgr, [11.0, 0.0, 10.0]);

    assert!(super::super::leash_anchor(&mgr, mgr.get_entity(pet).unwrap()).is_none());
}
