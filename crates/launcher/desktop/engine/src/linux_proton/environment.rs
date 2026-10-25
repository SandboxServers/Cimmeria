//! The environment a Proton child process gets (C-4).
//!
//! The child starts from an empty map. Only the variables in [`PASSTHROUGH`]
//! are copied from the launcher, and the Steam and Wine variables the launcher
//! sets are added on top. Anything else, such as `PROTON_LOG`, `SteamAppId` or
//! `LD_PRELOAD`, is left out on purpose.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;

use super::prefix::Prefix;

/// The launcher variables a Proton child may inherit.
pub const PASSTHROUGH: &[&str] = &[
    "HOME",
    "USER",
    "LOGNAME",
    "PATH",
    "LANG",
    "LC_ALL",
    "DISPLAY",
    "WAYLAND_DISPLAY",
    "XAUTHORITY",
    "XDG_RUNTIME_DIR",
    "XDG_SESSION_TYPE",
    "XDG_CURRENT_DESKTOP",
    "XDG_DATA_DIRS",
    "DBUS_SESSION_BUS_ADDRESS",
    "PULSE_SERVER",
    "PIPEWIRE_RUNTIME_DIR",
    "SteamDeck",
    "GAMESCOPE_WAYLAND_DISPLAY",
];

/// The base environment for any Proton child. `get` reads the launcher's
/// environment; production passes `|k| std::env::var_os(k)`.
pub fn base(
    get: impl Fn(&str) -> Option<OsString>,
    prefix: &Prefix,
    steam_client: &Path,
) -> BTreeMap<OsString, OsString> {
    let mut env = BTreeMap::new();
    for &key in PASSTHROUGH {
        if let Some(value) = get(key) {
            env.insert(OsString::from(key), value);
        }
    }
    env.insert(
        "STEAM_COMPAT_DATA_PATH".into(),
        prefix.compat.clone().into_os_string(),
    );
    env.insert(
        "STEAM_COMPAT_CLIENT_INSTALL_PATH".into(),
        steam_client.as_os_str().to_owned(),
    );
    env.insert(
        "WINEDLLOVERRIDES".into(),
        "winemenubuilder.exe=d;mscoree=d;mshtml=d".into(),
    );
    env
}

/// The base environment plus what a game process needs: the frame cap, and
/// the DXVK log settings when `log_guest` names a log folder (a `Z:\` path).
pub fn game(
    mut base: BTreeMap<OsString, OsString>,
    frame_rate: u32,
    log_guest: Option<&str>,
) -> BTreeMap<OsString, OsString> {
    base.insert("DXVK_FRAME_RATE".into(), frame_rate.to_string().into());
    if let Some(folder) = log_guest {
        base.insert("DXVK_LOG_LEVEL".into(), "info".into());
        base.insert("DXVK_LOG_PATH".into(), folder.into());
    }
    base
}

#[cfg(test)]
#[path = "environment_tests.rs"]
mod tests;
