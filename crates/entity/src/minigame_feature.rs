//! The `minigame.start_dialog` feature flag (MG-21, D-MG19).
//!
//! **Off by default.** With the flag off a minigame launch is direct: the
//! `StartMinigame` message goes to base at once. With it on, a launch first
//! shows the player the pre-game dialog (client 53) and waits for Play (cell
//! 24) or Cancel (cell 30); see `cimmeria_cell_world::cell::minigame_launch`.
//!
//! The flag is one process-wide switch: [`START_DIALOG_ENV`] read once at
//! server startup ([`init_start_dialog_from_env`], called from
//! `cimmeria-server`'s `main`), then [`start_dialog`] everywhere else.
//!
//! **Testing.** The logic is in [`resolve_start_dialog`], a pure function of
//! the raw variable, so a test never touches the global.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::ammo_feature::parse_flag;

/// The flag's name, as the telemetry and the docs spell it.
pub const START_DIALOG_FLAG: &str = "minigame.start_dialog";

/// The environment variable that turns the dialog on (`1`, `true`, `on`,
/// `yes`) or off (`0`, `false`, `off`, `no`), case-insensitively.
pub const START_DIALOG_ENV: &str = "CIMMERIA_MINIGAME_START_DIALOG";

/// The value when the variable is unset: off (D-MG19).
pub const START_DIALOG_DEFAULT: bool = false;

/// The value when the variable is set but unparseable: off.
pub const START_DIALOG_ON_INVALID: bool = false;

static START_DIALOG: AtomicBool = AtomicBool::new(START_DIALOG_DEFAULT);

/// Whether a minigame launch shows the pre-game dialog first.
pub fn start_dialog() -> bool {
    START_DIALOG.load(Ordering::Relaxed)
}

/// Set the flag for the whole process. Startup and nextest-only tests.
pub fn set_start_dialog(on: bool) {
    START_DIALOG.store(on, Ordering::Relaxed);
}

/// The flag value for an optional raw environment value: unset gives
/// [`START_DIALOG_DEFAULT`]; an unparseable value gives
/// [`START_DIALOG_ON_INVALID`] and is reported through the second field so the
/// caller can warn.
pub fn resolve_start_dialog(raw: Option<&str>) -> (bool, bool) {
    match raw {
        None => (START_DIALOG_DEFAULT, false),
        Some(v) => match parse_flag(v) {
            Some(on) => (on, false),
            None => (START_DIALOG_ON_INVALID, true),
        },
    }
}

/// Read [`START_DIALOG_ENV`], set the flag, log the result and return it.
/// Called once from `cimmeria-server`'s `main`.
pub fn init_start_dialog_from_env() -> bool {
    let raw = std::env::var(START_DIALOG_ENV).ok();
    let (on, invalid) = resolve_start_dialog(raw.as_deref());
    set_start_dialog(on);
    if invalid {
        tracing::warn!(
            target: "minigame",
            event = "feature_flag_invalid",
            flag = START_DIALOG_FLAG,
            value = raw.as_deref().unwrap_or(""),
            on,
            "unrecognised {START_DIALOG_ENV} value; the flag is off"
        );
    }
    tracing::info!(
        target: "minigame",
        event = "feature_flag",
        flag = START_DIALOG_FLAG,
        on,
        "minigame start dialog flag resolved"
    );
    on
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D-MG19: the dialog ships off when the variable is unset. Setting the
    /// default to `true` fails this test.
    #[test]
    fn unset_is_off() {
        assert_eq!(resolve_start_dialog(None), (false, false));
    }

    #[test]
    fn one_turns_it_on() {
        assert_eq!(resolve_start_dialog(Some("1")), (true, false));
    }

    /// A typo turns the dialog off and is flagged for the WARN. Making the
    /// invalid value fall back to on fails this test.
    #[test]
    fn bogus_is_off_and_flagged() {
        assert_eq!(resolve_start_dialog(Some("bogus")), (false, true));
    }
}
