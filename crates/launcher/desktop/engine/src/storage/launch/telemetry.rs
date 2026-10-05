//! The launch side of opt-in game telemetry: one server session per Play, and
//! the DLL joins the injection list only once that session's marker is on disk.
use super::*;
use crate::game_telemetry::{self, Outcome, Session};
use std::sync::Mutex;

/// Mint a session for a plan that carries the telemetry DLL. The choice is read
/// again here, so a player who opted out since admission gets no session.
pub(super) async fn session(state: &Arc<Mutex<DesktopState>>, plan: &Plan) -> Option<Session> {
    plan.resources.client_telemetry.as_ref()?;
    let identity = {
        let owner = state.lock().ok()?;
        let choice = owner.game_telemetry().ok()?;
        choice.opted_in.then_some(choice.identity).flatten()?
    };
    let backend = if plan.runtime.is_some() {
        "wine"
    } else {
        "native"
    };
    let tags = ["desktop-launcher", std::env::consts::OS, backend];
    match game_telemetry::start(&identity, &plan.installation.login_servers, &tags).await {
        Ok(session) => Some(session),
        Err(outcome) => {
            record(state, plan, outcome);
            None
        }
    }
}

/// Write the session marker beside the game and return the DLL to inject after
/// the client patches. No marker, no DLL: it would load with nothing to send to.
pub(super) fn injected<'a>(plan: &'a Plan, session: Option<&Session>) -> Option<&'a Artifact> {
    let artifact = plan.resources.client_telemetry.as_ref()?;
    let game = plan.installation.destination.join("game");
    let marker = crate::telemetry_session::current_session_path(&game);
    preparation::contained(&game, &marker).ok()?;
    crate::telemetry_session::write_current_session(&game, session?.marker()).ok()?;
    Some(artifact)
}

/// Best effort: a status that cannot be saved must not change what Play does.
pub(super) fn record(state: &Arc<Mutex<DesktopState>>, plan: &Plan, outcome: Outcome) {
    if let Ok(mut owner) = state.lock() {
        let _ = owner.record_game_telemetry(plan.id, outcome);
    }
}
