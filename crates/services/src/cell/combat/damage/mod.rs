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

mod pipeline;
mod qr;

pub use pipeline::calculate_damage;
pub use qr::{calculate_qr, calculate_result, qr_rand_to_result_code, QrResult};
