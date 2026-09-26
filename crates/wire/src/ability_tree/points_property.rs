//! The one builder for the client's training-point counter.
//!
//! Both sends of `onEntityProperty(GENERICPROPERTY_TrainingPoints, n)` use
//! it: the base's level-up bundle (`progression::build_grant_xp_bundle`) and
//! the cell's trainer-purchase burst (`base_messages::ability_granted`).
//! The client routes the property to `Events.PropertyUpdated` →
//! `AbilityMod.onPropertyUpdated` → `refreshTrainingPoints()`, whether or
//! not the Ability window is open (AT-E1 question 3).

use crate::cell::cell_methods::inventory::build_entity_property_args;

/// `GENERICPROPERTY_TrainingPoints` in `entities/defs/enumerations.xml`.
pub const GENERICPROPERTY_TRAINING_POINTS: i32 = 1;

/// The 8-byte `onEntityProperty` payload: `INT32 propId = 1`, `INT32 value`,
/// both little-endian.
pub fn training_points_property_args(training_points: i32) -> Vec<u8> {
    build_entity_property_args(GENERICPROPERTY_TRAINING_POINTS, training_points)
}
