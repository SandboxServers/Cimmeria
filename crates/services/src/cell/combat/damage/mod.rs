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
//! - [`pipeline`]: the multi-stage damage-application formula + shields.
//! - [`cover_qr`]: the cover terms of the QR roll (NA32).

mod cover_qr;
mod pipeline;
mod qr;

pub use cover_qr::{cover_shift, CoverShift, CoverSides, COVER_STAT_QR_PER_POINT};
pub use pipeline::calculate_damage;
pub use qr::{calculate_qr, calculate_result, qr_rand_to_result_code, QrResult};
