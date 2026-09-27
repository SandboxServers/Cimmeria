//! Passive-ability effects (pets PT-08).
//!
//! A passive ability (`passive_yn`, e.g. 2852 Heed Our Calling) is never
//! cast; its `EF_AlwaysPersist` effect holds for as long as the player knows
//! the ability. The server had no passive support at all, so 4968 "Pet
//! Summon Speed increase" never reached `speedPet`. [`apply_passives`] runs
//! the scripts of those effects on the player (source and target both the
//! player) whenever the known set changes: at login (`InitPlayerState`), on a
//! trainer purchase (`AbilityGranted`) and on a respec (`AbilitiesReset`,
//! which removes them).
//!
//! Only scripts that declare themselves passive
//! (`pet_scripts::is_passive_script`) run here, so an `EF_AlwaysPersist`
//! row that happens to carry a heal script is never fired by a login.

use cimmeria_entity::abilities::EF_ALWAYS_PERSIST;

use super::pet_scripts::is_passive_script;
use super::{dispatch_by_name, dispatch_on_remove, EffectContext};
use crate::cell::space_manager::SpaceManager;

/// Whether the passive effects are being put on or taken off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassiveChange {
    /// The abilities were learned (or the player logged in knowing them).
    Learned,
    /// The abilities were unlearned (respec).
    Unlearned,
}

/// Apply (or remove) the passive effects of `ability_ids` on `entity_id`.
/// Returns how many effect scripts ran. The caller flushes the dirty stats.
pub fn apply_passives(
    space_mgr: &mut SpaceManager,
    entity_id: u32,
    ability_ids: &[i32],
    change: PassiveChange,
) -> usize {
    let mut effects = Vec::new();
    for ability_id in ability_ids {
        let Some(def) = space_mgr.ability_defs.get(ability_id) else {
            continue;
        };
        for effect_id in &def.effect_ids {
            let Some(effect) = space_mgr.effect_defs.get(effect_id) else {
                continue;
            };
            if effect.flags & EF_ALWAYS_PERSIST == 0 {
                continue;
            }
            if let Some(script) = effect.script_name.as_deref() {
                if is_passive_script(script) {
                    effects.push(effect.clone());
                }
            }
        }
    }
    let mut ran = 0;
    for effect in &effects {
        let Some(script) = effect.script_name.clone() else {
            continue;
        };
        let mut ctx = EffectContext {
            source_id: entity_id,
            target_id: entity_id,
            effect,
            space_mgr,
        };
        let found = match change {
            PassiveChange::Learned => dispatch_by_name(&script, &mut ctx),
            PassiveChange::Unlearned => dispatch_on_remove(&script, &mut ctx),
        };
        if found {
            ran += 1;
        }
    }
    ran
}
