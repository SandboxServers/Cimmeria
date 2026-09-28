//! The `ammo.finite_special` feature flag (ammo campaign AM-F, issue #1026).
//!
//! Off by default. While off, a reload of a special ammo type refills for
//! free exactly as on `main` before the campaign; the flag gates only the
//! reserve **draw** (AM-02), never the `requestAmmoChange` whitelist
//! (AM-03), which is a security fix that ships regardless. AM-12 turns it
//! on after every packet through AM-11c has merged and the debug-hub UAT
//! (D-AM06) has passed.
//!
//! The repo had no feature-flag surface before this, so the flag is one
//! process-wide switch: [`FINITE_SPECIAL_ENV`] read once at server startup
//! ([`init_finite_special_from_env`], called from `cimmeria-server`'s
//! `main`), then [`finite_special`] everywhere else.
//!
//! **Testing.** Read the flag once, at the dispatch entrypoint, and pass the
//! `bool` into the logic you test, so a unit test never touches the global.
//! A test that must drive the whole handler with the flag on may call
//! [`set_finite_special`], but only under nextest (one process per test);
//! under `cargo test` the tests of one crate share the switch.

use std::sync::atomic::{AtomicBool, Ordering};

/// The flag's name, as the telemetry and the docs spell it.
pub const FINITE_SPECIAL_FLAG: &str = "ammo.finite_special";

/// The environment variable that turns the flag on (`1`, `true`, `on`,
/// `yes`) or off (`0`, `false`, `off`, `no`), case-insensitively.
pub const FINITE_SPECIAL_ENV: &str = "CIMMERIA_AMMO_FINITE_SPECIAL";

/// The value when the variable is unset or unparseable. AM-12 flips it.
pub const FINITE_SPECIAL_DEFAULT: bool = false;

static FINITE_SPECIAL: AtomicBool = AtomicBool::new(FINITE_SPECIAL_DEFAULT);

/// Whether reloads of special ammo draw from the bags.
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

/// The flag value for an optional raw environment value: unset gives the
/// default; an unparseable value gives the default too (the safe side, off)
/// and is reported through the second field so the caller can warn.
pub fn resolve_flag(raw: Option<&str>) -> (bool, bool) {
    match raw {
        None => (FINITE_SPECIAL_DEFAULT, false),
        Some(v) => match parse_flag(v) {
            Some(on) => (on, false),
            None => (FINITE_SPECIAL_DEFAULT, true),
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
            "unrecognised {FINITE_SPECIAL_ENV} value; using the default"
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

    #[test]
    fn default_is_off() {
        assert_eq!(resolve_flag(None), (false, false));
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

    #[test]
    fn unparseable_value_falls_back_to_off_and_is_flagged() {
        for v in ["", "2", "enable", "tru"] {
            assert_eq!(parse_flag(v), None, "{v:?}");
            assert_eq!(resolve_flag(Some(v)), (false, true), "{v:?}");
        }
    }
}
