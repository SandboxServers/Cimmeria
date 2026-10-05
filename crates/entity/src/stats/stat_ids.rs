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

/// The client's `EStats` token for a stat ID, for the `stat_name` log field
/// next to `stat_id` (Rule 6). `None` for an ID the client does not declare.
pub const fn stat_name(stat_id: i32) -> Option<&'static str> {
    Some(match stat_id {
        0 => "coordination",
        1 => "engagement",
        2 => "fortitude",
        3 => "morale",
        4 => "perception",
        5 => "intelligence",
        6 => "movementSpeedMod",
        7 => "health",
        8 => "focus",
        9 => "healthRegen",
        10 => "focusRegen",
        11 => "accuracy",
        12 => "defense",
        13 => "qrMod",
        18 => "physicalAF",
        23 => "energyAF",
        24 => "hazmatAF",
        28 => "psionicAF",
        29 => "kineticRes",
        34 => "mentalRes",
        40 => "healthRes",
        46 => "stealthRating",
        47 => "rangeModifier",
        48 => "coverQRModifier",
        49 => "ammoSlot1",
        50 => "ammoSlot2",
        51 => "ammoSlot3",
        52 => "ammoSlot4",
        53 => "ammoSlot5",
        54 => "deploymentBarAmmo",
        55 => "response",
        56 => "damage",
        57 => "penetration",
        58 => "physicalDensity",
        59 => "energyDensity",
        60 => "hazmatDensity",
        61 => "psionicDensity",
        62 => "tracking",
        63 => "stabilization",
        64 => "awareness",
        65 => "interruptRes",
        66 => "coverAccuracy",
        67 => "coverDefense",
        68 => "crouchingAccuracy",
        69 => "crouchingDefense",
        70 => "stealthMovement",
        71 => "revealRating",
        72 => "negation",
        73 => "PhysicalDamagePercent",
        74 => "EnergyDamagePercent",
        75 => "HazmatDamagePercent",
        76 => "PsionicDamagePercent",
        77 => "UntypedDamagePercent",
        78 => "disguiseRating",
        79 => "disguiseDetection",
        80 => "mitigation",
        81 => "rotationSpeedMod",
        82 => "energy",
        83 => "energyRegen",
        89 => "absorbPhysical",
        90 => "absorbEnergy",
        91 => "absorbHazmat",
        92 => "absorbPsionic",
        93 => "absorbUntyped",
        94 => "absorbPhysicalItem",
        95 => "absorbEnergyItem",
        96 => "absorbHazmatItem",
        97 => "absorbPsionicItem",
        98 => "absorbUntypedItem",
        99 => "absorbPhysicalEnergy",
        100 => "absorbEnergyEnergy",
        101 => "absorbHazmatEnergy",
        102 => "absorbPsionicEnergy",
        103 => "absorbUntypedEnergy",
        104 => "speedReload",
        105 => "speedGrenade",
        106 => "speedDeploy",
        107 => "speedAttack",
        108 => "recovery",
        109 => "restoration",
        110 => "subtlety",
        111 => "speedPet",
        _ => return None,
    })
}
