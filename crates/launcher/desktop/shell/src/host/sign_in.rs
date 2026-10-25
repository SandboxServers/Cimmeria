//! The launcher's sign-in bridge (C13): one command for every sign-in action, the
//! system browser it opens, and the keep-running prompt while a relayed game is signed in.
use super::NativeHost;
use cimmeria_launcher_engine::sign_in::commands::{SignInCommand, SignInFailure, SignInStatus};
use cimmeria_launcher_engine::sign_in::driver::Browser;
use cimmeria_launcher_engine::sign_in::lease;
use std::sync::Arc;
use tauri_plugin_dialog::{
    DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult,
};
use tracing::{info, warn};

/// Opens sign-in pages in the player's own browser.
struct SystemBrowser;

impl Browser for SystemBrowser {
    fn open(&self, url: &str) -> Result<(), ()> {
        if !openable(url) {
            // The URL is never logged: it carries the sign-in state.
            warn!(reason = "browser_url_refused", "sign-in page not opened");
            return Err(());
        }
        match tauri_plugin_opener::open_url(url, None::<&str>) {
            Ok(()) => Ok(()),
            Err(_) => {
                // The URL is never logged: it carries the sign-in state.
                warn!(reason = "browser_open_failed", "sign-in page not opened");
                Err(())
            }
        }
    }
}

/// Whether a sign-in URL may reach the OS opener: `https`, or plain `http` only to a loopback
/// host. The opener runs whatever scheme it is given, so the check lives here as well as in the engine.
fn openable(url: &str) -> bool {
    let https = url.starts_with("https://");
    let loopback = ["http://127.0.0.1:", "http://localhost:", "http://[::1]:"]
        .iter()
        .any(|prefix| url.starts_with(prefix));
    // A userinfo part (`http://127.0.0.1:80@host/`) names another host than the prefix shows.
    let rest = url.split_once("://").map_or("", |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    (https || loopback) && !authority.contains('@')
}

/// Runs one sign-in request. Every command, `cancel` included, goes through here.
#[tauri::command]
pub(crate) async fn sign_in_command(
    request: SignInCommand,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<SignInStatus, SignInFailure> {
    let host = state.inner().clone();
    let io = || {
        warn!(
            reason = "state_unavailable",
            "sign-in state could not be opened"
        );
        SignInFailure::Io
    };
    // The store may wait up to 30 s on its first open, so it is opened on a worker.
    let worker = host.clone();
    let store = tauri::async_runtime::spawn_blocking(move || worker.store())
        .await
        .map_err(|_| io())?
        .map_err(|_| io())?;
    host.sign_in.run(&store, request, &SystemBrowser).await
}

/// Whether closing the window must ask first: a relayed game is still signed in.
pub(crate) fn hold_close(active_relays: usize) -> bool {
    active_relays > 0
}

/// Set while the close prompt is on screen, so a second click on the window's close button
/// does not stack another prompt.
static PROMPTING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The label of the button that closes the launcher.
const CLOSE_LABEL: &str = "Close anyway";

/// Whether the prompt's answer closes the launcher. Only the "Close anyway" button does;
/// Esc, the dialog's close button and "Keep running" all keep it open.
pub(crate) fn closes_anyway(result: &MessageDialogResult) -> bool {
    matches!(result, MessageDialogResult::Custom(label) if label == CLOSE_LABEL)
}

/// Asks before the window closes while a relayed game is signed in. The game cannot
/// change server or log in again once the launcher is gone, so "Close anyway" is the player's choice.
pub(crate) fn close_guard(window: &tauri::Window, event: &tauri::WindowEvent) {
    let tauri::WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    if !hold_close(lease::active_relays()) {
        return;
    }
    api.prevent_close();
    if PROMPTING.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    let target = window.clone();
    window
        .dialog()
        .message(
            "The game is signed in through this launcher. If you close it now, the game cannot change server or log in again until you press Play.",
        )
        .title("Game still signed in")
        .kind(MessageDialogKind::Warning)
        .parent(window)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Keep running".to_owned(),
            CLOSE_LABEL.to_owned(),
        ))
        .show_with_result(move |result| {
            PROMPTING.store(false, std::sync::atomic::Ordering::SeqCst);
            let closed = closes_anyway(&result);
            info!(
                event = "launcher.relay.close_prompt",
                outcome = if closed { "closed" } else { "kept" }
            );
            if closed {
                let _ = target.destroy();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_launcher_engine::DesktopState;
    use std::sync::Mutex;

    #[test]
    fn hold_close_only_with_active_relays() {
        assert!(!hold_close(0));
        assert!(hold_close(1));
    }

    #[test]
    fn dismissing_the_prompt_keeps_the_launcher() {
        assert!(!closes_anyway(&MessageDialogResult::Cancel));
        assert!(!closes_anyway(&MessageDialogResult::Custom(
            "Keep running".to_owned()
        )));
        assert!(!closes_anyway(&MessageDialogResult::Ok));
        assert!(closes_anyway(&MessageDialogResult::Custom(
            "Close anyway".to_owned()
        )));
    }

    #[test]
    fn only_https_and_loopback_urls_open() {
        assert!(openable("https://auth.example.org/auth/v1/authorize?x=1"));
        assert!(openable("http://127.0.0.1:8443/auth/v1/authorize"));
        assert!(openable("http://localhost:8443/auth/v1/authorize"));
        assert!(openable("http://[::1]:8443/auth/v1/authorize"));
        assert!(!openable("http://auth.example.org/x"));
        assert!(!openable("file:///C:/Windows/System32/calc.exe"));
        assert!(!openable("ms-msdt:/id"));
        assert!(!openable(""));
        assert!(!openable("http://127.0.0.1:80@evil.example/"));
        assert!(!openable("http://localhost:1@evil.example/"));
        assert!(!openable("https://user@auth.example.org/x"));
    }

    #[tokio::test]
    async fn unsupported_schema_needs_no_state() {
        let root = tempfile::tempdir().unwrap();
        let host = NativeHost::new(root.path().join("state"));
        let state = Mutex::new(DesktopState::open(&root.path().join("state")).unwrap());
        let request: SignInCommand = serde_json::from_value(serde_json::json!({
            "command": "inspect",
            "schema_version": 2
        }))
        .unwrap();
        let reply = host.sign_in.run(&state, request, &SystemBrowser).await;
        assert_eq!(reply, Err(SignInFailure::UnsupportedSchema));
    }

    #[test]
    fn failure_serializes_with_error_tag() {
        let value = serde_json::to_value(SignInFailure::Server {
            code: "rate_limited".to_owned(),
            retry_after: Some(5),
        })
        .unwrap();
        assert_eq!(
            value,
            serde_json::json!({"error": "server", "code": "rate_limited", "retry_after": 5})
        );
    }
}
