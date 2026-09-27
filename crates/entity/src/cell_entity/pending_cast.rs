//! An ability that has been launched but is still in its warmup.

use cimmeria_common::{SpaceId, Vector3};

/// A committed ability launch waiting for its warmup to expire (AT-10).
///
/// `handle_use_ability` validates the cast, charges the cooldown, sends
/// `Ability_Begin`, and parks the cast here when the ability's warmup is
/// positive. The cell's warmup tick fires it (ammo, `Ability_End`, damage)
/// once `fire_at` has passed, or interrupts it. Mirrors the 2009
/// `AbilityInstance.warmupTimer` / `afterWarmup` split in
/// `deprecated/python/cell/AbilityManager.py`.
///
/// One per caster: a second launch while this is `Some` is rejected, as
/// python's `canUseAbility` rejected a launch while `currentAbility` was set.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingCast {
    /// The ability being warmed up (after the weapon redirect).
    pub ability_id: i32,
    /// The `useAbility` target id; `0` for a target-less cast.
    pub target_id: i32,
    /// The ground point of a `useAbilityOnGroundTarget` cast, used to
    /// collect the AoE secondaries when the cast fires.
    pub ground: Option<[f32; 3]>,
    /// `InstanceId` minted at launch. `Ability_Begin`, `Ability_End` and
    /// `Ability_Interrupt` of one cast share it.
    pub effect_seq: i32,
    /// When the warmup expires.
    pub fire_at: std::time::Instant,
    /// Warmup length after the speed-stat modifiers, in seconds.
    pub warmup_secs: f32,
    /// Caster position at launch. The movement interrupt measures from here.
    pub anchor: Vector3,
    /// Caster space at launch. A cast never fires in another space.
    pub space_id: SpaceId,
    /// `instance_id` of the weapon in the caster's active bandolier slot at
    /// launch, if any. A different weapon at fire time interrupts the cast,
    /// so a swapped-in weapon never pays for the old one's shot.
    pub weapon_instance: Option<i32>,
}
