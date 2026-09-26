//! SGWCombatant interface ClientMethods (indices 20–25).

/// Stat current/max value update (health, focus, etc).
pub const ON_STAT_UPDATE: u16 = 20;
/// Stat base value update.
pub const ON_STAT_BASE_UPDATE: u16 = 21;
/// Melee range changed.
pub const ON_MELEE_RANGE_UPDATE: u16 = 22;
/// Archetype changed.
pub const ON_ARCHETYPE_UPDATE: u16 = 23;
/// Alignment changed.
pub const ON_ALIGNMENT_UPDATE: u16 = 24;
/// Faction changed.
pub const ON_FACTION_UPDATE: u16 = 25;
