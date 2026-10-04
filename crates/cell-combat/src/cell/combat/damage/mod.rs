//! QR (Quality Rating) damage resolution.
//!
//! Implements the QR system used for ability effects. The QR system
//! determines hit/miss/crit outcomes via a simplified beta distribution
//! model, then applies a multi-stage damage pipeline:
//!
//!   baseDamage × qrRand × QR_DAMAGE_MULTIPLIER × (1 + damageBonus%)
//!     × (1 - statResistance) × (1 + qr) - armorFactor - absorption
//!
//! Reference: `python/cell/AbilityManager.py:13-231` (DamageCalc class)
//!
//! Submodules:
//! - [`qr`]: QR scoring + beta-distribution roll → result code.
//! - [`pipeline`]: the multi-stage damage-application formula.
//! - [`absorb`]: the absorb-shield drain every damage seam shares (AB-10).
//! - [`cover_damage`]: cover as a per-node damage reduction (NA32, D-NA15a).

mod absorb;
mod cover_damage;
mod pipeline;
mod qr;

pub(crate) use absorb::{absorb_damage_nvps, drain_absorption_pools, script_damage_type};
pub use cover_damage::{
    attacker_cover_qr, cover_reduction, node_base_pct, CoverRatingTable, CoverReduction, CoverSide,
    COVER_MAX_PCT, COVER_MIN_PCT, COVER_RATING,
};
pub use pipeline::{
    calculate_damage, calculate_damage_penetrating, calculate_damage_scaled, resolve_damage,
    DamageOutcome,
};
pub use qr::{calculate_qr, calculate_result, qr_rand_to_result_code, QrResult};
