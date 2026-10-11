//! The launch side of opt-in game telemetry: one server session per Play, and
//! the DLL joins the injection list only once that session's marker is on disk.
use super::*;
use crate::game_telemetry::session_loop::{GameExit, SessionLoop};
use crate::game_telemetry::transport::{Consent, Transport};
use crate::game_telemetry::{self, Outcome, Session};
use std::sync::Mutex;

/// Mint a session for a plan that carries the telemetry DLL. The choice is read
/// again here, so a player who opted out since admission gets no session.
/// `servers` is the list this Play writes into `LoginInternal.lua`: the session
/// comes from the server the game logs in to, not the install-time list.
pub(super) async fn session(
    state: &Arc<Mutex<DesktopState>>,
    plan: &Plan,
    servers: &[LoginServer],
) -> Option<Session> {
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
    match game_telemetry::start(&identity, servers, &tags).await {
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

/// Start the launcher's half of the session (log tailing, exit, bundle) on
/// the session Play minted, before the game is spawned. The choice is read
/// again under the lock; a transport that cannot be opened costs only the
/// telemetry, never the launch. Waits first for the previous Play's session
/// to finish sending (bounded, see `SessionLoop::start`).
pub(super) async fn begin_session(
    state: &Arc<Mutex<DesktopState>>,
    plan: &Plan,
    session: &Session,
    servers: &[LoginServer],
) -> Option<SessionLoop> {
    let consent = Consent::read(&*state.lock().ok()?).ok().flatten()?;
    let game = plan.installation.destination.join("game");
    match Transport::open(consent, session, servers, &game) {
        Ok(transport) => Some(SessionLoop::start(transport, &game).await),
        Err(code) => {
            tracing::warn!(operation = %plan.id, ?code, "game telemetry session not started");
            None
        }
    }
}

/// The supervisor's last observation, as the session records it. Only an
/// exit the helper reported carries a code; any other ending has none.
pub(super) fn game_exit(observation: Observation) -> Option<GameExit> {
    match observation {
        Observation::ProcessExited {
            host_pid,
            guest_pid,
            code,
            early,
        } => Some(GameExit {
            host_pid,
            guest_pid,
            code,
            early,
        }),
        _ => None,
    }
}

/// Best effort: a status that cannot be saved must not change what Play does.
pub(super) fn record(state: &Arc<Mutex<DesktopState>>, plan: &Plan, outcome: Outcome) {
    if let Ok(mut owner) = state.lock() {
        let _ = owner.record_game_telemetry(plan.id, outcome);
    }
}
