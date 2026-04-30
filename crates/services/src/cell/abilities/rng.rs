//! Deterministic pseudo-random for combat rolls.
//!
//! Used by `handle_use_ability` to seed the QR (quality-result) calculation.
//! Reproducible by design so unit tests can assert on combat outcomes — a
//! production-grade PRNG should replace this when we wire `rand` properly.

/// Generate a pseudo-random value in [0.0, 1.0) from entity/ability/sequence.
///
/// This is a simple hash-based PRNG for reproducible combat results.
/// A proper PRNG (e.g., `rand` crate) should replace this in production.
pub(super) fn pseudo_random(entity_id: u32, ability_id: i32, sequence: u32) -> f64 {
    let mut h = entity_id.wrapping_mul(2654435761);
    h ^= (ability_id as u32).wrapping_mul(2246822519);
    h ^= sequence.wrapping_mul(3266489917);
    h = h.wrapping_mul(668265263);
    h ^= h >> 15;
    // Divide by 2^32 (one greater than u32::MAX) so the result is strictly
    // in [0.0, 1.0); using u32::MAX as the divisor would map h == u32::MAX
    // to 1.0, breaking the half-open invariant the callers rely on.
    (h as f64) / (u32::MAX as f64 + 1.0)
}
