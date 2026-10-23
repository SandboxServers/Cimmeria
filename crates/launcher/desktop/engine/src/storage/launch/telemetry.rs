//! The launch side of opt-in game telemetry: one server session per Play, and
//! the DLL joins the injection list only once that session's marker is on disk.
use super::*;
use crate::game_telemetry::patch_events::PatchEvents;
use crate::game_telemetry::session_loop::{GameExit, SessionLoop, Settled};
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
        let Ok(owner) = state.lock() else {
            events::game_telemetry_skipped(plan.id, "state_lock_poisoned");
            return None;
        };
        let choice = match owner.game_telemetry() {
            Ok(choice) => choice,
            Err(_) => {
                events::game_telemetry_skipped(plan.id, "choice_unreadable");
                return None;
            }
        };
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
    let game = plan.installation.content_root();
    let marker = crate::telemetry_session::current_session_path(&game);
    if preparation::contained(&game, &marker).is_err() {
        events::game_telemetry_skipped(plan.id, "marker_outside_game_dir");
        return None;
    }
    let session = session?;
    if crate::telemetry_session::write_current_session(&game, session.marker()).is_err() {
        events::game_telemetry_skipped(plan.id, "marker_write_failed");
        return None;
    }
    Some(artifact)
}

/// Open the transport for the launcher's half of the session (log tailing,
/// client-patches events, exit, bundle) on the session Play minted, and wait
/// for the previous Play's session to finish sending (bounded, see
/// `SessionLoop::settle`). Before Running, so the Play can still be
/// cancelled during the wait. The choice is read again under the lock; a
/// transport that cannot be opened costs only the telemetry, never the
/// launch.
pub(super) async fn open_session(
    state: &Arc<Mutex<DesktopState>>,
    plan: &Plan,
    session: &Session,
    servers: &[LoginServer],
) -> Option<Settled> {
    let consent = {
        let Ok(owner) = state.lock() else {
            events::game_telemetry_skipped(plan.id, "state_lock_poisoned");
            return None;
        };
        match Consent::read(&owner) {
            Ok(Some(consent)) => consent,
            Ok(None) => return None,
            Err(_) => {
                events::game_telemetry_skipped(plan.id, "consent_unreadable");
                return None;
            }
        }
    };
    let game = plan.installation.content_root();
    match Transport::open(consent, session, servers, &game) {
        Ok(transport) => Some(SessionLoop::settle(transport).await),
        Err(code) => {
            tracing::warn!(operation = %plan.id, ?code, "game telemetry session not started");
            None
        }
    }
}

/// Start the session loop, before the game is spawned, so the tail's
/// baseline holds only earlier sessions' lines. It reports `patches`; with
/// no session open they come back for `patch_events::queue_at_exit`.
pub(super) fn begin_session(
    plan: &Plan,
    settled: Option<Settled>,
    patches: PatchEvents,
) -> Result<SessionLoop, PatchEvents> {
    let game = plan.installation.content_root();
    match settled {
        Some(settled) => Ok(settled.begin(&game, Some(patches))),
        None => Err(patches),
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
        if owner.record_game_telemetry(plan.id, outcome).is_err() {
            events::game_telemetry_record_failed(plan.id);
        }
    }
}
