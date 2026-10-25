//! The launcher's sign-in bridge (C13): one command for every sign-in action, the
//! system browser it opens, and the keep-running prompt while a relayed game is signed in.
use super::NativeHost;
use cimmeria_launcher_engine::sign_in::commands::{SignInCommand, SignInFailure, SignInStatus};
use cimmeria_launcher_engine::sign_in::driver::Browser;
use cimmeria_launcher_engine::sign_in::lease;
use std::sync::Arc;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tracing::{info, warn};

/// Opens sign-in pages in the player's own browser.
struct SystemBrowser;

impl Browser for SystemBrowser {
    fn open(&self, url: &str) -> Result<(), ()> {
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

/// Runs one sign-in request. Every command, `cancel` included, goes through here.
#[tauri::command]
pub(crate) async fn sign_in_command(
    request: SignInCommand,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<SignInStatus, SignInFailure> {
    let host = state.inner().clone();
    // The store may wait up to 30 s on its first open, so it is opened on a worker.
    let worker = host.clone();
    let store = tauri::async_runtime::spawn_blocking(move || worker.store())
        .await
        .map_err(|_| SignInFailure::Io)?
        .map_err(|_| SignInFailure::Io)?;
    host.sign_in.run(&store, request, &SystemBrowser).await
}

/// Whether closing the window must ask first: a relayed game is still signed in.
pub(crate) fn hold_close(active_relays: usize) -> bool {
    active_relays > 0
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
            "Close anyway".to_owned(),
        ))
        .show(move |keep| {
            info!(
                event = "launcher.relay.close_prompt",
                outcome = if keep { "kept" } else { "closed" }
            );
            if !keep {
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
