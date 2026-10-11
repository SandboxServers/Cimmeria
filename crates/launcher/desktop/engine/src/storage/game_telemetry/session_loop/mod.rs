//! One Play's game-telemetry session, from before the game starts to after
//! it exits: the launcher's half of what the egui launcher's
//! `telemetry::runner` did, on the [`Transport`].
//!
//! - **Baseline.** [`SessionLoop::begin`] runs before the helper spawns the
//!   game. Every tailed file already on disk is taken in at its end, so a
//!   previous session's lines are never sent again (they went with that
//!   session, or into its bundle). A file that appears later is this
//!   session's and is read from its first line.
//! - **Tailing.** On the server's flush cadence: `Binaries/SGWDebugLog*`
//!   (any case, anywhere under `Binaries`) and `Binaries/sessions/*.log` /
//!   `*.txt`, one event per complete line, converted by core's
//!   [`line_to_event`] so the fields are the egui launcher's and SigNoz
//!   queries keep working. Then a flush (at most eight chunks).
//! - **Exit.** Play's supervisor already watches the game: the launch worker
//!   hands its exit to [`SessionLoop::finish`], and this loop does not watch
//!   the process itself. Then one last tick, a `session_meta` event of kind
//!   `game_exited` with the exit code and pids, the end-of-session bundle,
//!   and [`Transport::shutdown`] (bounded by [`SHUTDOWN_BOUND`]).
//! - **Launcher closing.** The loop dies with the runtime. Queued events
//!   are on disk and go with the next session's first flush. Offsets are
//!   not saved: a restarted launcher does not resume the session (a
//!   recorded Play never proves the game is still alive), and the next
//!   Play's baseline starts every file at its end, so nothing ships twice.
//!   Lines the game writes after the launcher closed are not streamed.
//! - **Opt-out.** The transport refuses once the choice changes; the loop
//!   then stops at once, with no final flush and no bundle, and leaves no
//!   queue behind.
//!
//! Known limit, as in the egui launcher: a log the game truncates and
//! rewrites past its old length between two ticks is read from the old
//! length, so its first bytes are missed.
use super::transport::{Transport, TransportError};
use cimmeria_launcher_core::install_layout::binaries_dir;
use cimmeria_launcher_core::telemetry::events::{
    SessionMetaEvent, SessionMetaKind, TelemetryEvent,
};
use cimmeria_launcher_core::telemetry::runner::{is_debug_log, line_to_event};
use cimmeria_launcher_core::telemetry::tail::Tailer;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

/// The last flush of the whole backlog at game exit. Whatever it has not
/// sent by then stays queued for the next session.
pub const SHUTDOWN_BOUND: Duration = Duration::from_secs(30);

/// How the game ended, as Play's supervisor observed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameExit {
    pub host_pid: u32,
    pub guest_pid: u32,
    pub code: i32,
    /// It ran under ten seconds.
    pub early: bool,
}

/// What a finished session did. For logs and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SessionEnd {
    /// Events this session queued (the bundle's `event_count`).
    pub events: u64,
    pub exit: Option<GameExit>,
    pub bundle_sent: bool,
    /// The player opted out while it ran: it stopped sending.
    pub withdrawn: bool,
}

/// A running session. Dropping it ends the session as a finish with no
/// observed exit would (Play stopped before the game ran, or its outcome is
/// unknown); the task itself is never aborted.
pub struct SessionLoop {
    exit: oneshot::Sender<Option<GameExit>>,
    task: JoinHandle<SessionEnd>,
}

impl SessionLoop {
    /// Take the baseline and start the loop. On the async runtime, before
    /// the game is spawned. `game_dir` is the install's `game` directory.
    pub fn begin(transport: Transport, game_dir: &Path) -> Self {
        let binaries = binaries_dir(game_dir);
        let mut tailer = Tailer::new();
        tailer.refresh(&binaries, |path| tailed(&binaries, path));
        let (exit, exited) = oneshot::channel();
        let task = tokio::spawn(run(transport, tailer, binaries, exited));
        Self { exit, task }
    }

    /// The game is gone: hand over its exit, if it was observed. The
    /// returned handle resolves when the bundle and the last flush are done;
    /// the launch worker does not wait on it.
    pub fn finish(self, exit: Option<GameExit>) -> JoinHandle<SessionEnd> {
        let _ = self.exit.send(exit);
        self.task
    }
}

/// The client's debug log anywhere under `Binaries`, and the session logs
/// in `Binaries/sessions`. One walk with one predicate: two walks of
/// overlapping roots would each drop the other's files from the tailer.
fn tailed(binaries: &Path, path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if is_debug_log(name) {
        return true;
    }
    path.parent() == Some(binaries.join("sessions").as_path())
        && path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("log") || ext.eq_ignore_ascii_case("txt"))
}

async fn run(
    transport: Transport,
    mut tailer: Tailer,
    binaries: PathBuf,
    mut exited: oneshot::Receiver<Option<GameExit>>,
) -> SessionEnd {
    let mut end = SessionEnd::default();
    let mut ticker = tokio::time::interval(transport.flush_interval().await);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    if queue(&transport, meta(SessionMetaKind::Started, None), &mut end)
        .await
        .is_err()
    {
        return withdrawn(end);
    }
    let exit = loop {
        tokio::select! {
            biased;
            exit = &mut exited => break exit.ok().flatten(),
            _ = ticker.tick() => {
                if tick(&transport, &mut tailer, &binaries, &mut end).await.is_err()
                    || transport.flush().await == Err(TransportError::NotOptedIn)
                {
                    return withdrawn(end);
                }
            }
        }
    };
    end.exit = exit;
    if tick(&transport, &mut tailer, &binaries, &mut end)
        .await
        .is_err()
    {
        return withdrawn(end);
    }
    if let Some(exit) = exit {
        if queue(
            &transport,
            meta(SessionMetaKind::GameExited, Some(exit)),
            &mut end,
        )
        .await
        .is_err()
        {
            return withdrawn(end);
        }
    }
    match transport.upload_bundle(end.events).await {
        Ok(sent) => end.bundle_sent = sent.is_some(),
        Err(TransportError::NotOptedIn) => return withdrawn(end),
        Err(code) => tracing::warn!(?code, "game session log bundle not sent"),
    }
    let shutdown = tokio::time::timeout(SHUTDOWN_BOUND, transport.shutdown()).await;
    if !matches!(shutdown, Ok(Ok(_))) {
        tracing::info!("game telemetry left events queued for the next session");
    }
    tracing::info!(
        events = end.events,
        bundle_sent = end.bundle_sent,
        exit_code = ?end.exit.map(|exit| exit.code),
        pid = ?end.exit.map(|exit| exit.guest_pid),
        "game telemetry session finished"
    );
    end
}

fn withdrawn(mut end: SessionEnd) -> SessionEnd {
    end.withdrawn = true;
    tracing::info!(
        events = end.events,
        "game telemetry turned off during the session; stopped sending"
    );
    end
}

/// Queue every complete line written since the last tick. `Err` only when
/// the player has opted out; other failures drop the line and go on.
async fn tick(
    transport: &Transport,
    tailer: &mut Tailer,
    binaries: &Path,
    end: &mut SessionEnd,
) -> Result<(), ()> {
    if !transport.consented() {
        return Err(());
    }
    tailer.follow(binaries, |path| tailed(binaries, path));
    for line in tailer.tick() {
        queue(transport, line_to_event(line), end).await?;
    }
    Ok(())
}

async fn queue(
    transport: &Transport,
    event: TelemetryEvent,
    end: &mut SessionEnd,
) -> Result<(), ()> {
    match transport.enqueue(event).await {
        Ok(()) => {
            end.events += 1;
            Ok(())
        }
        Err(TransportError::NotOptedIn) => Err(()),
        Err(code) => {
            tracing::warn!(?code, "game telemetry event not queued");
            Ok(())
        }
    }
}

fn meta(kind: SessionMetaKind, exit: Option<GameExit>) -> TelemetryEvent {
    let mut fields = serde_json::Map::new();
    if let Some(exit) = exit {
        fields.insert("exit_code".into(), exit.code.into());
        fields.insert("pid".into(), exit.guest_pid.into());
        fields.insert("host_pid".into(), exit.host_pid.into());
        fields.insert("early".into(), exit.early.into());
    }
    TelemetryEvent::SessionMeta(SessionMetaEvent {
        ts_ms: 0,
        seq: 0,
        kind,
        fields,
    })
}

#[cfg(test)]
mod tests;
