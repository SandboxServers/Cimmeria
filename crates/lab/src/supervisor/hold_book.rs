//! The daemon's book of keys it holds down through the lab DLL (contract C5,
//! daemon half). Pure: the caller passes the clock, so host tests need no
//! bridge. The DLL ends a hold when its lease runs out, so the book's job is
//! to ask for a renewal before that happens.

use std::collections::BTreeMap;

/// Lease asked of the DLL for every hold (D-AX8: its maximum). Renewed at
/// half of it, so one lost renewal still leaves a full half-lease of slack.
pub const LEASE_MS: u32 = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    pub name: String,
    pub renew_at_ms: u64,
}

#[derive(Debug)]
pub struct HoldBook {
    held: BTreeMap<u8, Held>,
    lease_ms: u32,
    /// An old DLL answered `input_hold` with "method not found": post
    /// window messages directly from then on.
    fallback: bool,
}

impl Default for HoldBook {
    fn default() -> Self {
        Self {
            held: BTreeMap::new(),
            lease_ms: LEASE_MS,
            fallback: false,
        }
    }
}

impl HoldBook {
    fn renew_at(&self, now: u64) -> u64 {
        now.saturating_add(u64::from(self.lease_ms / 2))
    }

    /// Note a hold (or its renewal) sent at `now`.
    pub fn pressed(&mut self, vk: u8, name: &str, now: u64) {
        let renew_at_ms = self.renew_at(now);
        self.held.insert(
            vk,
            Held {
                name: name.to_string(),
                renew_at_ms,
            },
        );
    }

    /// Forget a hold; whether it was in the book.
    pub fn released(&mut self, vk: u8) -> bool {
        self.held.remove(&vk).is_some()
    }

    /// Keys whose renewal is due at `now`.
    pub fn due(&self, now: u64) -> Vec<u8> {
        self.held
            .iter()
            .filter(|(_, h)| h.renew_at_ms <= now)
            .map(|(vk, _)| *vk)
            .collect()
    }

    /// A renewal of `vk` went out at `now`. `posted` is the DLL saying it had
    /// to press the key again (the lease had already lapsed), which the
    /// caller logs; returns it.
    pub fn renewed(&mut self, vk: u8, now: u64, posted: bool) -> bool {
        let next = self.renew_at(now);
        if let Some(h) = self.held.get_mut(&vk) {
            h.renew_at_ms = next;
        }
        posted
    }

    /// Every held key with its name.
    pub fn all(&self) -> Vec<(u8, String)> {
        self.held
            .iter()
            .map(|(vk, h)| (*vk, h.name.clone()))
            .collect()
    }

    pub fn name_of(&self, vk: u8) -> Option<String> {
        self.held.get(&vk).map(|h| h.name.clone())
    }

    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    pub fn clear(&mut self) {
        self.held.clear();
    }

    /// Tests: a zero lease makes every hold due at once.
    #[cfg(test)]
    pub fn set_lease_ms(&mut self, ms: u32) {
        self.lease_ms = ms;
    }

    pub fn set_fallback(&mut self) {
        self.fallback = true;
    }

    pub fn is_fallback(&self) -> bool {
        self.fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renew_is_due_at_half_lease() {
        let mut b = HoldBook::default();
        b.pressed(0x57, "w", 0);
        assert!(b.due(4999).is_empty());
        assert_eq!(b.due(5000), vec![0x57]);
    }

    #[test]
    fn posted_renew_is_a_reassert() {
        let mut b = HoldBook::default();
        b.pressed(0x57, "w", 0);
        assert!(b.renewed(0x57, 6000, true));
        assert!(b.due(10_999).is_empty(), "renew_at must move to 11000");
        assert_eq!(b.due(11_000), vec![0x57]);
        assert!(!b.renewed(0x57, 11_000, false));
    }

    #[test]
    fn release_and_clear_empty_the_book() {
        let mut b = HoldBook::default();
        b.pressed(1, "a", 0);
        b.pressed(2, "b", 0);
        assert!(b.released(1));
        assert!(!b.released(1));
        b.clear();
        assert!(b.is_empty());
    }
}
