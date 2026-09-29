//! The `ammo.finite_special` feature flag (ammo campaign, issue #1026).
//!
//! **On by default** since the campaign's close-out (AM-12, decision D-AM11,
//! @Cadacious 2026-09-28). `CIMMERIA_AMMO_FINITE_SPECIAL=0` on the server
//! turns it off; that is the campaign's rollback lever.
//!
//! The flag gates every player-visible half of special ammo together:
//!
//! - the reserve **draw** and the switch return (AM-02): a special reload
//!   takes rounds from the bags, and switching type returns unfired rounds;
//! - the `ammo_modifiers` row on every shot (AM-04 and the family packets
//!   AM-08 to AM-11c): damage and penetration multipliers, damage type and
//!   on-hit effect;
//! - the support-dart ally path (AM-11d).
//!
//! It never gates the `requestAmmoChange` whitelist (AM-03), which is a
//! security fix that ships regardless, the loot rows (AM-05), the GM
//! commands (AM-06), or the pushed item definitions 9000-9014 (AM-07). With
//! the flag off, special rounds stay in the bags as ordinary items, reloads
//! refill for free, and every shot fires unmodified.
//!
//! The repo had no feature-flag surface before this, so the flag is one
//! process-wide switch: [`FINITE_SPECIAL_ENV`] read once at server startup
//! ([`init_finite_special_from_env`], called from `cimmeria-server`'s
//! `main`), then [`finite_special`] everywhere else.
//!
//! **Testing.** Read the flag once, at the dispatch entrypoint, and pass the
//! `bool` into the logic you test, so a unit test never touches the global.
//! A test that must drive the whole handler with a given value calls
//! [`set_finite_special`], but only under nextest (one process per test);
//! under `cargo test` the tests of one crate share the switch. The switch
//! starts on, so a test that needs the pre-campaign behaviour says so with
//! `set_finite_special(false)`.

use std::sync::atomic::{AtomicBool, Ordering};

/// The flag's name, as the telemetry and the docs spell it.
pub const FINITE_SPECIAL_FLAG: &str = "ammo.finite_special";

/// The environment variable that turns the flag on (`1`, `true`, `on`,
/// `yes`) or off (`0`, `false`, `off`, `no`), case-insensitively.
pub const FINITE_SPECIAL_ENV: &str = "CIMMERIA_AMMO_FINITE_SPECIAL";

/// The value when the variable is unset: on (AM-12, D-AM11).
pub const FINITE_SPECIAL_DEFAULT: bool = true;

/// The value when the variable is set but unparseable: off. Whoever sets the
/// variable is most likely reaching for the rollback lever, so a typo
/// (`CIMMERIA_AMMO_FINITE_SPECIAL=of`) must not leave the feature on.
pub const FINITE_SPECIAL_ON_INVALID: bool = false;

static FINITE_SPECIAL: AtomicBool = AtomicBool::new(FINITE_SPECIAL_DEFAULT);

/// Whether special ammo is finite and modifies the shot (see the module
/// docs for everything the flag gates).
pub fn finite_special() -> bool {
    FINITE_SPECIAL.load(Ordering::Relaxed)
}

/// Set the flag for the whole process. Startup and nextest-only tests; see
/// the module docs.
pub fn set_finite_special(on: bool) {
    FINITE_SPECIAL.store(on, Ordering::Relaxed);
}

/// Parse a flag value. `None` for anything that is not one of the spellings
/// [`FINITE_SPECIAL_ENV`] documents.
pub fn parse_flag(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Some(true),
        "0" | "false" | "off" | "no" => Some(false),
        _ => None,
    }
}

/// The flag value for an optional raw environment value: unset gives
/// [`FINITE_SPECIAL_DEFAULT`] (on); an unparseable value gives
/// [`FINITE_SPECIAL_ON_INVALID`] (off, the rollback side) and is reported
/// through the second field so the caller can warn.
pub fn resolve_flag(raw: Option<&str>) -> (bool, bool) {
    match raw {
        None => (FINITE_SPECIAL_DEFAULT, false),
        Some(v) => match parse_flag(v) {
            Some(on) => (on, false),
            None => (FINITE_SPECIAL_ON_INVALID, true),
        },
    }
}

/// Read [`FINITE_SPECIAL_ENV`], set the flag, log the result and return it.
/// Called once from `cimmeria-server`'s `main`.
pub fn init_finite_special_from_env() -> bool {
    let raw = std::env::var(FINITE_SPECIAL_ENV).ok();
    let (on, invalid) = resolve_flag(raw.as_deref());
    set_finite_special(on);
    if invalid {
        tracing::warn!(
            target: "ammo",
            event = "feature_flag_invalid",
            flag = FINITE_SPECIAL_FLAG,
            value = raw.as_deref().unwrap_or(""),
            on,
            "unrecognised {FINITE_SPECIAL_ENV} value; the flag is off"
        );
    }
    tracing::info!(
        target: "ammo",
        event = "feature_flag",
        flag = FINITE_SPECIAL_FLAG,
        on,
        "ammo feature flag resolved"
    );
    on
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D-AM11 (AM-12): the flag ships on when the variable is unset, and the
    /// process-wide switch starts at that default. Setting the default back
    /// to `false` fails this test.
    #[test]
    fn default_is_on() {
        assert_eq!(
            resolve_flag(None),
            (true, false),
            "D-AM11: ammo.finite_special ships on"
        );
    }

    /// The rollback lever: `CIMMERIA_AMMO_FINITE_SPECIAL=0` turns it off.
    #[test]
    fn zero_is_the_rollback_lever() {
        assert_eq!(resolve_flag(Some("0")), (false, false));
    }

    #[test]
    fn parses_every_documented_spelling() {
        for v in ["1", "true", "ON", " yes "] {
            assert_eq!(parse_flag(v), Some(true), "{v:?}");
            assert_eq!(resolve_flag(Some(v)), (true, false), "{v:?}");
        }
        for v in ["0", "False", "off", "no"] {
            assert_eq!(parse_flag(v), Some(false), "{v:?}");
            assert_eq!(resolve_flag(Some(v)), (false, false), "{v:?}");
        }
    }

    /// A typo in the variable turns the flag off (the rollback side), not on,
    /// and is flagged for the WARN.
    #[test]
    fn unparseable_value_falls_back_to_off_and_is_flagged() {
        for v in ["", "2", "enable", "of"] {
            assert_eq!(parse_flag(v), None, "{v:?}");
            assert_eq!(resolve_flag(Some(v)), (false, true), "{v:?}");
        }
    }
}
