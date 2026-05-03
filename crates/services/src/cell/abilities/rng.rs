//! Deterministic pseudo-random seed for combat rolls.
//!
//! Used by `handle_use_ability` to seed the QR (quality-result) calculation.
//! Reproducible by design so unit tests can assert on combat outcomes — same
//! (entity, ability, sequence) triple always produces the same beta sample.
//!
//! Returns a `u64` seed (rather than a uniform `f64`) because the QR
//! sampler now draws from a two-branch Beta distribution
//! (`AbilityManager.py:181-184`):
//!
//! ```text
//! if qr >= 0: betavariate(α, α + qr * mult)
//! else:       betavariate(α - qr * mult, α)
//! ```
//!
//! Beta sampling internally consumes multiple uniform draws, so the
//! caller hands it a seeded RNG rather than a single value.

/// Build a deterministic 64-bit seed from entity/ability/sequence. Caller
/// passes this to `ChaCha8Rng::seed_from_u64` (or any other
/// stable seeded RNG).
///
/// Folding three 32-bit prime multipliers into a 64-bit space keeps the
/// avalanche behavior of the prior 32-bit hash while giving the seed enough
/// entropy that two close-by (entity, ability, seq) triples produce
/// distinct, well-mixed RNG streams (otherwise rapid-fire shots would
/// share suspiciously similar beta samples).
pub(super) fn pseudo_random_seed(entity_id: u32, ability_id: i32, sequence: u32) -> u64 {
    let lo = (entity_id as u64).wrapping_mul(2654435761);
    let mid = ((ability_id as u32) as u64).wrapping_mul(2246822519);
    let hi = (sequence as u64).wrapping_mul(3266489917);
    let mut h = lo ^ (mid << 21) ^ (hi << 42);
    // splitmix64 finalizer for high-quality avalanche.
    h = (h ^ (h >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94d049bb133111eb);
    h ^ (h >> 31)
}
