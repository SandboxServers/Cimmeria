//! The randomness seam for the crafting verbs.
//!
//! Every roll a verb makes (research success, reverse-engineering
//! recovery, alloy outcome) is decided on the server and goes through
//! [`CraftRng`], so a test pins the outcome with [`ScriptedRng`] instead of
//! depending on chance. The induction engine hands each completing job a
//! fresh [`OsSeededRng`] in production.

use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

/// A source of uniform samples for crafting rolls.
pub trait CraftRng: Send {
    /// A uniform sample in `[0, 1)`.
    fn unit(&mut self) -> f64;
}

/// The production RNG: a `StdRng` seeded from the OS.
#[derive(Debug)]
pub struct OsSeededRng(StdRng);

impl OsSeededRng {
    pub fn new() -> Self {
        Self(StdRng::from_rng(&mut rand::rng()))
    }
}

impl Default for OsSeededRng {
    fn default() -> Self {
        Self::new()
    }
}

impl CraftRng for OsSeededRng {
    fn unit(&mut self) -> f64 {
        self.0.random::<f64>()
    }
}

/// A test RNG that replays a fixed sequence of samples, cycling when it
/// runs out. Values are clamped into `[0, 1)`.
#[derive(Debug, Clone)]
pub struct ScriptedRng {
    samples: Vec<f64>,
    next: usize,
}

impl ScriptedRng {
    /// # Panics
    ///
    /// When `samples` is empty: a roll with no scripted value is a test bug.
    pub fn new(samples: Vec<f64>) -> Self {
        assert!(!samples.is_empty(), "ScriptedRng needs at least one sample");
        Self { samples, next: 0 }
    }
}

impl CraftRng for ScriptedRng {
    fn unit(&mut self) -> f64 {
        let value = self.samples[self.next % self.samples.len()];
        self.next += 1;
        value.clamp(0.0, 1.0 - f64::EPSILON)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripted_rng_replays_and_cycles() {
        let mut rng = ScriptedRng::new(vec![0.25, 0.75]);
        assert_eq!(rng.unit(), 0.25);
        assert_eq!(rng.unit(), 0.75);
        assert_eq!(rng.unit(), 0.25);
    }

    #[test]
    fn scripted_rng_keeps_samples_in_the_unit_interval() {
        let mut rng = ScriptedRng::new(vec![-1.0, 1.0]);
        assert_eq!(rng.unit(), 0.0);
        assert!(rng.unit() < 1.0);
    }

    #[test]
    fn os_seeded_rng_samples_the_unit_interval() {
        let mut rng = OsSeededRng::new();
        for _ in 0..1000 {
            let v = rng.unit();
            assert!((0.0..1.0).contains(&v), "{v}");
        }
    }
}
