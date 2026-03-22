//! SGWBeing interface + entity ClientMethods (indices 12–26).

// ── SGWBeing interface (8 methods, indices 12–19) ───────────────────────────

/// Ability/effect timer update.
pub const ON_TIMER_UPDATE: u16 = 12;
/// Effect user data (custom key-value pairs for effects).
pub const ON_EFFECT_USER_DATA: u16 = 13;
/// Combat effect results (damage numbers, hit/miss/crit).
pub const ON_EFFECT_RESULTS: u16 = 14;
/// Level change notification.
pub const ON_LEVEL_UPDATE: u16 = 15;
/// Current target changed.
pub const ON_TARGET_UPDATE: u16 = 16;
/// Being display name changed.
pub const ON_BEING_NAME_UPDATE: u16 = 17;
/// Movement top speed changed.
pub const ON_TOP_SPEED_UPDATE: u16 = 18;
/// State field bitfield update (dead, in combat, etc).
pub const ON_STATE_FIELD_UPDATE: u16 = 19;

// ── SGWBeing entity own (1 method, index 26) ────────────────────────────────

/// Full appearance update (body set + visual components).
pub const BEING_APPEARANCE: u16 = 26;
