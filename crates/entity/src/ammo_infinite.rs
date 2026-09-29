//! The GM infinite-ammo switch, `bInfiniteAmmo` (ammo campaign AM-06, issue
//! #1026; D-AM09).
//!
//! `.infiniteammo on|off` (the `.`-console twin of the client's native
//! `/gmsetinfiniteammo`, which has no server receiver) sets it. Its meaning is
//! narrow on purpose: a reload of **special** ammo draws nothing from the
//! bags, but the clip still empties and still needs a reload, so a tester
//! exercises the normal reload path. It never means an unlimited clip.
//!
//! `bInfiniteAmmo` is a `CELL_PUBLIC INT8` on `SGWAbilityManager.def`: other
//! cells may read it, no client ever does, so nothing is sent on the wire.
//! The switch is keyed by character (`sgw_player.player_id`), not by entity,
//! so it survives zone changes and relogs until the server restarts, like the
//! GM's `.aggro off` switch. It lives here, below both the cell and the base,
//! because the reader (AM-02's reload draw) is in `cimmeria-cell-combat`,
//! which the console crate depends on, not the other way round.
//!
//! **Reader contract (AM-02).** At the reload entrypoint, when the loaded
//! type is special and `ammo.finite_special` is on, check [`is_on`] first:
//! `true` means skip the reserve draw and refill as if the full request had
//! been drawn.

use std::collections::HashSet;
use std::sync::{Mutex, MutexGuard, OnceLock};

fn switches() -> MutexGuard<'static, HashSet<i32>> {
    static SET: OnceLock<Mutex<HashSet<i32>>> = OnceLock::new();
    // A poisoned lock only means a panic elsewhere while holding it; the set
    // itself is always valid, so keep serving it.
    match SET.get_or_init(Mutex::default).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// Whether `player_id`'s special-ammo reloads skip the reserve draw.
pub fn is_on(player_id: i32) -> bool {
    switches().contains(&player_id)
}

/// Turn the switch on or off for `player_id`. Returns `true` when that
/// changed the state, `false` when it was already so.
pub fn set(player_id: i32, on: bool) -> bool {
    let mut set = switches();
    if on {
        set.insert(player_id)
    } else {
        set.remove(&player_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keyed by character, and `set` reports whether anything changed.
    /// The ids are this test's own; the set is process-wide.
    #[test]
    fn set_reports_changes_and_is_keyed_by_character() {
        const A: i32 = 0x7AA0_0601;
        const B: i32 = 0x7AA0_0602;
        assert!(!is_on(A));
        assert!(set(A, true), "off -> on changes");
        assert!(!set(A, true), "on -> on does not");
        assert!(is_on(A));
        assert!(!is_on(B), "another character is unaffected");
        assert!(set(A, false));
        assert!(!set(A, false));
        assert!(!is_on(A));
    }
}
