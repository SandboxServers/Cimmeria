//! Unit tests for the Prison Boot scripts (MG-63b): the movement-only hold,
//! its release, and the counted reference a minigame hold shares with it.

use cimmeria_entity::abilities::EffectDef;
use cimmeria_wire::state_field::BSF_MOVEMENT_LOCK;

use super::test_fixtures::make_mgr_with_target;
use super::{dispatch_by_name, EffectContext};
use crate::cell::space_manager::SpaceManager;

/// The boot effect (chain 1023 casts ability 1597).
const BOOT_EFFECT: i32 = 1939;
/// Its release (chain 1025 casts ability 1598).
const RELEASE_EFFECT: i32 = 1942;

/// Dispatch `script` for entity 1 casting on itself, the effect `effect_id`
/// with `params`.
fn cast(mgr: &mut SpaceManager, script: &str, effect_id: i32, params: &[(&str, &str)]) {
    let effect = EffectDef {
        effect_id,
        ability_id: 1597,
        script_name: Some(script.to_string()),
        params: params
            .iter()
            .map(|&(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        ..Default::default()
    };
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: mgr,
    };
    assert!(dispatch_by_name(script, &mut ctx), "{script} is registered");
}

/// The boot on entity 1.
fn boot(mgr: &mut SpaceManager) {
    cast(mgr, "MovementHold", BOOT_EFFECT, &[]);
}

/// The release that names the boot.
fn release(mgr: &mut SpaceManager) {
    cast(
        mgr,
        "MovementRelease",
        RELEASE_EFFECT,
        &[("ReleasesEffect", "1939")],
    );
}

/// After the boot the wearer is locked for movement, and item and ability use
/// stay open (the incapacitated gates skip a movement-only entry). Reverting
/// the gate switch makes the last assertion fail.
#[test]
fn boot_locks_movement_but_not_item_or_ability_use() {
    let mut mgr = make_mgr_with_target();
    boot(&mut mgr);
    let e = mgr.get_entity(1).unwrap();
    assert!(e.has_state_flag(BSF_MOVEMENT_LOCK));
    assert!(e.holds_movement_only_entry());
    assert!(!e.holds_incapacitating_flag(BSF_MOVEMENT_LOCK));
}

/// The release removes the boot's entry and the lock clears with it.
#[test]
fn release_removes_the_boot_and_clears_the_lock() {
    let mut mgr = make_mgr_with_target();
    boot(&mut mgr);
    release(&mut mgr);
    let e = mgr.get_entity(1).unwrap();
    assert!(!e.holds_movement_only_entry());
    assert!(!e.has_state_flag(BSF_MOVEMENT_LOCK));
}

/// A minigame hold taken and released while booted leaves the lock set: the
/// boot's counted reference is still there. Only the release clears it.
#[test]
fn minigame_hold_inside_the_boot_keeps_the_lock_until_release() {
    let mut mgr = make_mgr_with_target();
    boot(&mut mgr);
    let e = mgr.get_entity_mut(1).unwrap();
    // The boot already holds the bit: the minigame's set and unset move the
    // counter only, so the bit must survive the minigame's release.
    e.set_state_flag(BSF_MOVEMENT_LOCK);
    e.unset_state_flag(BSF_MOVEMENT_LOCK);
    assert!(mgr.get_entity(1).unwrap().has_state_flag(BSF_MOVEMENT_LOCK));
    release(&mut mgr);
    assert!(!mgr.get_entity(1).unwrap().has_state_flag(BSF_MOVEMENT_LOCK));
}

/// A release with no `ReleasesEffect` param removes nothing.
#[test]
fn release_without_its_param_leaves_the_boot_on() {
    let mut mgr = make_mgr_with_target();
    boot(&mut mgr);
    cast(&mut mgr, "MovementRelease", RELEASE_EFFECT, &[]);
    assert!(mgr.get_entity(1).unwrap().holds_movement_only_entry());
}
