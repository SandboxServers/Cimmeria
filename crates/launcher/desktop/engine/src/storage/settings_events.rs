//! Settings and consent change events (launcher-telemetry LT-19; C6). Logged
//! after the save succeeds. An opt-out is logged after the gate has closed, so
//! it stays in the local log; an opt-in is forwarded. Logging never changes the
//! save: every helper returns nothing, and none takes a path or a URL.
use cimmeria_launcher_core::client_setup::login_servers::LoginServer;

/// Preferences were saved. `install_dir_changed` says the install folder moved.
pub(crate) fn preferences_saved(install_dir_changed: bool, setup_diagnostics: bool) {
    tracing::info!(
        target: "launcher.settings",
        event = "preferences_saved",
        install_dir_changed,
        setup_diagnostics,
        "settings: preferences saved"
    );
}

/// The setup-diagnostics choice changed; `opted_in` is its new value.
pub(crate) fn setup_diagnostics_changed(opted_in: bool) {
    tracing::info!(
        target: "launcher.settings",
        event = "setup_diagnostics_changed",
        opted_in,
        "settings: setup diagnostics changed"
    );
}

/// Client options were saved. `count` is the server list length (0 means the
/// defaults); `custom_servers` says a url is not one of the default servers.
pub(crate) fn client_options_saved(servers: &[LoginServer], client_patches_enabled: bool) {
    let defaults = crate::client_setup::login_servers::default_servers();
    let custom_servers = servers
        .iter()
        .any(|server| !defaults.iter().any(|default| default.url == server.url));
    let count = servers.len() as u64;
    tracing::info!(
        target: "launcher.settings",
        event = "client_options_saved",
        count,
        custom_servers,
        client_patches_enabled,
        "settings: client options saved"
    );
}

/// The game diagnostics choice changed; `opted_in` is its new value.
pub(crate) fn game_diagnostics_changed(opted_in: bool) {
    tracing::info!(
        target: "launcher.settings",
        event = "game_diagnostics_changed",
        opted_in,
        "settings: game diagnostics changed"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_capture::{capture, Captured};
    use crate::DesktopState;
    use std::sync::{Mutex, MutexGuard};

    static SERIAL: Mutex<()> = Mutex::new(());
    fn serial() -> MutexGuard<'static, ()> {
        SERIAL.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Runs `f` under capture, one test at a time, with a second live Dispatch.
    fn captured(f: impl FnOnce()) -> Vec<Captured> {
        let _serial = serial(); // first line: one capture test of this module at a time
                                // A second live Dispatch keeps callsite interest per dispatcher.
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let ((), events) = capture(|| {
            tracing::callsite::rebuild_interest_cache(); // first line in the closure
            f();
        });
        events
    }

    fn named<'a>(events: &'a [Captured], event: &str) -> Vec<&'a Captured> {
        events
            .iter()
            .filter(|e| e.field("event") == Some(event))
            .collect()
    }

    #[test]
    fn each_helper_logs_one_info_event_on_the_settings_target() {
        let events = captured(|| {
            preferences_saved(true, false);
            setup_diagnostics_changed(true);
            client_options_saved(&crate::client_setup::login_servers::default_servers(), true);
            game_diagnostics_changed(false);
        });
        let saved = named(&events, "preferences_saved");
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].level, tracing::Level::INFO);
        assert_eq!(saved[0].target, "launcher.settings");
        assert_eq!(saved[0].field("install_dir_changed"), Some("true"));
        assert_eq!(saved[0].field("setup_diagnostics"), Some("false"));
        let diagnostics = named(&events, "setup_diagnostics_changed");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].level, tracing::Level::INFO);
        assert_eq!(diagnostics[0].target, "launcher.settings");
        assert_eq!(diagnostics[0].field("opted_in"), Some("true"));
        let client = named(&events, "client_options_saved");
        assert_eq!(client.len(), 1);
        assert_eq!(client[0].level, tracing::Level::INFO);
        assert_eq!(client[0].target, "launcher.settings");
        assert_eq!(client[0].field("client_patches_enabled"), Some("true"));
        let game = named(&events, "game_diagnostics_changed");
        assert_eq!(game.len(), 1);
        assert_eq!(game[0].level, tracing::Level::INFO);
        assert_eq!(game[0].target, "launcher.settings");
        assert_eq!(game[0].field("opted_in"), Some("false"));
    }

    #[test]
    fn default_servers_are_not_custom_and_an_empty_list_is_the_defaults() {
        let defaults = crate::client_setup::login_servers::default_servers();
        let events = captured(|| {
            client_options_saved(&defaults, true);
            client_options_saved(&[], true);
            client_options_saved(
                &[LoginServer {
                    name: "Mine".into(),
                    url: "https://custom.example".into(),
                }],
                false,
            );
        });
        let saved = named(&events, "client_options_saved");
        assert_eq!(saved.len(), 3);
        assert_eq!(saved[0].field("custom_servers"), Some("false"));
        assert_eq!(
            saved[0].field("count"),
            Some(defaults.len().to_string().as_str())
        );
        assert_eq!(saved[1].field("count"), Some("0"));
        assert_eq!(saved[1].field("custom_servers"), Some("false"));
        assert_eq!(saved[2].field("count"), Some("1"));
        assert_eq!(saved[2].field("custom_servers"), Some("true"));
    }

    #[test]
    fn a_real_opt_in_logs_one_game_diagnostics_change() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = DesktopState::open(dir.path()).unwrap();
        let events = captured(|| {
            state.set_game_telemetry(true).unwrap();
            // Setting the same value again changes nothing, so it logs nothing.
            state.set_game_telemetry(true).unwrap();
        });
        let changed = named(&events, "game_diagnostics_changed");
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].target, "launcher.settings");
        assert_eq!(changed[0].field("opted_in"), Some("true"));
    }
}
