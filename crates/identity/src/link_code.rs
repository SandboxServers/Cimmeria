//! GM-issued link codes (contract C6 and C10, C15 e): a short code that lets a
//! player attach a provider identity to a legacy account without its password.
//!
//! One live code per account: a new issue replaces the old one. A code is
//! single use and lasts 15 minutes. The GM console reaches the one
//! process-wide store through [`shared`]; the tests build their own.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use rand::RngExt;
use subtle::ConstantTimeEq;

/// How long a link code can be redeemed.
pub const LINK_CODE_LIFE: Duration = Duration::from_secs(15 * 60);

/// The characters a link code is drawn from (C6): no `0`, `1`, `I`, `O`.
const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// The length of a link code.
const CODE_LEN: usize = 10;

/// The live link codes, one per account: the code text and when it was issued.
#[derive(Default)]
pub struct LinkCodeStore {
    codes: Mutex<HashMap<u32, (String, Instant)>>,
}

impl LinkCodeStore {
    /// Issues a fresh code for `account_id`, replacing its previous one, and
    /// returns the code. Codes past their life are dropped on the way.
    pub fn issue(&self, account_id: u32, now: Instant) -> String {
        let mut rng = rand::rng();
        let code: String = (0..CODE_LEN)
            .map(|_| char::from(ALPHABET[rng.random_range(0..ALPHABET.len())]))
            .collect();
        let mut codes = locked(&self.codes);
        codes.retain(|_, (_, issued)| live(*issued, now));
        codes.insert(account_id, (code.clone(), now));
        code
    }

    /// Redeems `code` (case-insensitive) and returns the account it was issued
    /// for. The code is single use: a successful or expired redeem removes it.
    /// Every stored code is compared, in constant time per entry, so the time
    /// taken does not show which account matched.
    pub fn redeem(&self, code: &str, now: Instant) -> Option<u32> {
        let presented = code.to_ascii_uppercase();
        let mut codes = locked(&self.codes);
        let mut hit = None;
        for (account_id, (stored, _)) in codes.iter() {
            let same: bool = stored.as_bytes().ct_eq(presented.as_bytes()).into();
            if same {
                hit = Some(*account_id);
            }
        }
        let account_id = hit?;
        let (_, issued) = codes.remove(&account_id)?;
        live(issued, now).then_some(account_id)
    }

    /// Drops the codes past their life.
    pub fn reap(&self, now: Instant) {
        locked(&self.codes).retain(|_, (_, issued)| live(*issued, now));
    }
}

/// The process-wide store the GM console and the routes share.
static SHARED: LazyLock<LinkCodeStore> = LazyLock::new(LinkCodeStore::default);

/// The process-wide [`LinkCodeStore`].
pub fn shared() -> &'static LinkCodeStore {
    &SHARED
}

/// Whether a code issued at `issued` is still inside [`LINK_CODE_LIFE`].
fn live(issued: Instant, now: Instant) -> bool {
    now.saturating_duration_since(issued) < LINK_CODE_LIFE
}

/// Locks the map, ignoring poisoning: a panicked holder leaves it readable.
fn locked<T>(map: &Mutex<T>) -> MutexGuard<'_, T> {
    map.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_code_alphabet_and_reuse() {
        let store = LinkCodeStore::default();
        let t0 = Instant::now();
        let code = store.issue(42, t0);
        assert_eq!(code.len(), CODE_LEN);
        assert!(code.bytes().all(|b| ALPHABET.contains(&b)));

        let lower = code.to_ascii_lowercase();
        assert_eq!(store.redeem(&lower, t0), Some(42), "lowercase redeems");
        assert_eq!(store.redeem(&code, t0), None, "second redeem is refused");

        let first = store.issue(42, t0);
        let second = store.issue(42, t0);
        assert_eq!(store.redeem(&first, t0), None, "a new issue replaces");
        assert_eq!(store.redeem(&second, t0), Some(42));
    }

    #[test]
    fn link_code_expires_after_15_min() {
        let store = LinkCodeStore::default();
        let t0 = Instant::now();
        let code = store.issue(9, t0);
        let late = t0 + LINK_CODE_LIFE;
        assert_eq!(store.redeem(&code, late), None);
    }

    #[test]
    fn link_code_unknown_code_is_refused() {
        let store = LinkCodeStore::default();
        let t0 = Instant::now();
        store.issue(9, t0);
        assert_eq!(store.redeem("ZZZZZZZZZZ", t0), None);
    }
}
