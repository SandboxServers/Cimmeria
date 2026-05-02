//! Stat ID constants and the public-stat allowlist.
//!
//! From `python/Atrea/enums.py:295-376` and `SGWBeing.publicStats`
//! (`python/cell/SGWBeing.py:235-247`).

/// Primary attributes
pub const COORDINATION: i32 = 0;
pub const ENGAGEMENT: i32 = 1;
pub const FORTITUDE: i32 = 2;
pub const MORALE: i32 = 3;
pub const PERCEPTION: i32 = 4;
pub const INTELLIGENCE: i32 = 5;

/// Movement
pub const MOVEMENT_SPEED_MOD: i32 = 6;

/// Pools
pub const HEALTH: i32 = 7;
pub const FOCUS: i32 = 8;
pub const HEALTH_REGEN: i32 = 9;
pub const FOCUS_REGEN: i32 = 10;

/// Combat modifiers
pub const ACCURACY: i32 = 11;
pub const DEFENSE: i32 = 12;
pub const QR_MOD: i32 = 13;

/// Armor factors
pub const PHYSICAL_AF: i32 = 18;
pub const ENERGY_AF: i32 = 23;
pub const HAZMAT_AF: i32 = 24;
pub const PSIONIC_AF: i32 = 28;

/// Resistances
pub const KINETIC_RES: i32 = 29;
pub const MENTAL_RES: i32 = 34;
pub const HEALTH_RES: i32 = 40;

/// Stealth
pub const STEALTH_RATING: i32 = 46;
pub const RANGE_MODIFIER: i32 = 47;
pub const COVER_QR_MODIFIER: i32 = 48;

/// Ammo slots
pub const AMMO_SLOT_1: i32 = 49;
pub const AMMO_SLOT_2: i32 = 50;
pub const AMMO_SLOT_3: i32 = 51;
pub const AMMO_SLOT_4: i32 = 52;
pub const AMMO_SLOT_5: i32 = 53;
pub const DEPLOYMENT_BAR_AMMO: i32 = 54;

/// Combat
pub const RESPONSE: i32 = 55;
pub const DAMAGE: i32 = 56;
pub const PENETRATION: i32 = 57;

/// Density
pub const PHYSICAL_DENSITY: i32 = 58;
pub const ENERGY_DENSITY: i32 = 59;
pub const HAZMAT_DENSITY: i32 = 60;
pub const PSIONIC_DENSITY: i32 = 61;

/// Awareness
pub const TRACKING: i32 = 62;
pub const STABILIZATION: i32 = 63;
pub const AWARENESS: i32 = 64;
pub const INTERRUPT_RES: i32 = 65;

/// Cover/crouch
pub const COVER_ACCURACY: i32 = 66;
pub const COVER_DEFENSE: i32 = 67;
pub const CROUCHING_ACCURACY: i32 = 68;
pub const CROUCHING_DEFENSE: i32 = 69;
pub const STEALTH_MOVEMENT: i32 = 70;

/// Reveal/disguise
pub const REVEAL_RATING: i32 = 71;
pub const NEGATION: i32 = 72;

/// Damage type percentages
pub const PHYSICAL_DAMAGE_PERCENT: i32 = 73;
pub const ENERGY_DAMAGE_PERCENT: i32 = 74;
pub const HAZMAT_DAMAGE_PERCENT: i32 = 75;
pub const PSIONIC_DAMAGE_PERCENT: i32 = 76;
pub const UNTYPED_DAMAGE_PERCENT: i32 = 77;

/// Disguise
pub const DISGUISE_RATING: i32 = 78;
pub const DISGUISE_DETECTION: i32 = 79;

/// Mitigation and movement
pub const MITIGATION: i32 = 80;
pub const ROTATION_SPEED_MOD: i32 = 81;
pub const ENERGY_POOL: i32 = 82;
pub const ENERGY_REGEN: i32 = 83;

/// Absorb stats
pub const ABSORB_PHYSICAL: i32 = 89;
pub const ABSORB_ENERGY: i32 = 90;
pub const ABSORB_HAZMAT: i32 = 91;
pub const ABSORB_PSIONIC: i32 = 92;
pub const ABSORB_UNTYPED: i32 = 93;
pub const ABSORB_PHYSICAL_ITEM: i32 = 94;
pub const ABSORB_ENERGY_ITEM: i32 = 95;
pub const ABSORB_HAZMAT_ITEM: i32 = 96;
pub const ABSORB_PSIONIC_ITEM: i32 = 97;
pub const ABSORB_UNTYPED_ITEM: i32 = 98;
pub const ABSORB_PHYSICAL_ENERGY: i32 = 99;
pub const ABSORB_ENERGY_ENERGY: i32 = 100;
pub const ABSORB_HAZMAT_ENERGY: i32 = 101;
pub const ABSORB_PSIONIC_ENERGY: i32 = 102;
pub const ABSORB_UNTYPED_ENERGY: i32 = 103;

/// Speed modifiers
pub const SPEED_RELOAD: i32 = 104;
pub const SPEED_GRENADE: i32 = 105;
pub const SPEED_DEPLOY: i32 = 106;
pub const SPEED_ATTACK: i32 = 107;
pub const RECOVERY: i32 = 108;
pub const RESTORATION: i32 = 109;
pub const SUBTLETY: i32 = 110;
pub const SPEED_PET: i32 = 111;

/// Stats visible to all nearby clients (not just the owner).
///
/// Mirrors `SGWBeing.publicStats` — `python/cell/SGWBeing.py:235-247`.
pub const PUBLIC_STATS: &[i32] = &[
    MOVEMENT_SPEED_MOD,
    HEALTH,
    FOCUS,
    AMMO_SLOT_1,
    AMMO_SLOT_2,
    AMMO_SLOT_3,
    AMMO_SLOT_4,
    AMMO_SLOT_5,
    ROTATION_SPEED_MOD,
    ENERGY_POOL,
    ENERGY_REGEN,
];
