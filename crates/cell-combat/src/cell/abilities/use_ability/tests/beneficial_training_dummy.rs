//! D-DA7: a friendly training dummy is a heal target.
//!
//! `beneficial`'s fixture (player A casts, NPC 5 is a neutral mob). Marked
//! as a training dummy the caster may not attack, NPC 5 is an ally for a
//! beneficial `TargetTarget` cast, so 1646 Health Heal lands on it; an
//! unmarked neutral NPC still falls back to the caster (D-AB02), and a
//! hostile training dummy is never healed.
//!
//! Revert proof: drop the `TrainingDummy` arm from `support_shot::classify`
//! and `a_heal_lands_on_a_friendly_training_dummy` heals the caster instead.

use std::time::Instant;

use cimmeria_entity::stats::HEALTH;

use super::super::beneficial::{resolve_cast_target, CastTarget};
use super::beneficial::heal_mgr;
use super::duel_gate::{A, MOB};
use super::*;
use crate::cell::space_manager::TrainingDummy;

const HEALTH_HEAL: i32 = 1646;
const DUMMY: u32 = 5;
/// `.dummy friendly`'s faction: Friendly_Ambient.
const FRIENDLY_FACTION: u8 = 9;

fn mark(mgr: &mut SpaceManager, id: u32, faction: u8) {
    let e = mgr.get_entity_mut(id).unwrap();
    e.faction = faction;
    e.extensions.insert(TrainingDummy::new(Instant::now()));
}

fn health(mgr: &SpaceManager, id: u32) -> i32 {
    mgr.get_entity(id).unwrap().stats.get(HEALTH).unwrap().cur
}

#[test]
fn a_friendly_training_dummy_resolves_as_an_ally() {
    let mut mgr = heal_mgr(0.0);
    let def = mgr.ability_defs.get(&HEALTH_HEAL).cloned();
    assert_eq!(
        resolve_cast_target(&mgr, A, def.as_ref(), DUMMY as i32),
        CastTarget::Caster,
        "control: an unmarked neutral NPC is not an ally"
    );
    mark(&mut mgr, DUMMY, FRIENDLY_FACTION);
    assert_eq!(
        resolve_cast_target(&mgr, A, def.as_ref(), DUMMY as i32),
        CastTarget::Ally(DUMMY)
    );
    // A hostile training dummy is shot, never healed.
    mark(&mut mgr, MOB, crate::cell::combat::HOSTILE_FACTION);
    assert_eq!(
        resolve_cast_target(&mgr, A, def.as_ref(), MOB as i32),
        CastTarget::Caster
    );
}

#[tokio::test]
async fn a_heal_lands_on_a_friendly_training_dummy() {
    let mut mgr = heal_mgr(0.0);
    mark(&mut mgr, DUMMY, FRIENDLY_FACTION);
    let (caster_before, dummy_before) = (health(&mgr, A), health(&mgr, DUMMY));
    let (tx, _rx) = mpsc::channel(256);

    assert!(handle_use_ability(A, HEALTH_HEAL, DUMMY as i32, &tx, &mut mgr).await);

    assert!(
        health(&mgr, DUMMY) > dummy_before,
        "the dummy is healed: {} -> {}",
        dummy_before,
        health(&mgr, DUMMY)
    );
    assert_eq!(health(&mgr, A), caster_before, "not the caster");
}
