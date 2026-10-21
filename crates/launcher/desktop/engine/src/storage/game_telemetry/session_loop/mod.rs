//! One Play's game-telemetry session, from before the game starts to after
//! it exits: the launcher's half of what the egui launcher's
//! `telemetry::runner` did, on the [`Transport`].
//!
//! - **Baseline.** [`SessionLoop::begin`] runs before the helper spawns the
//!   game. Every tailed file already on disk is taken in at its end, so a
//!   previous session's lines are never sent again (they went with that
//!   session, or into its bundle). A file created later is this session's
//!   and is read from its first line; an older file that only now matches
//!   (a log rotated by rename) starts at its end, and a renamed tracked
//!   file keeps its offset. A tick reads at most a megabyte per file.
//! - **Tailing.** On the server's flush cadence, the files core's
//!   [`is_tailed`] matches (what the bundle takes): `SGWDebugLog*` anywhere
//!   under `Binaries` and every `.log` / `.txt` under `Binaries/sessions` at
//!   any depth, case ignored. One event per complete line, converted by
//!   core's [`line_to_event`] so the fields are the egui launcher's and
//!   SigNoz queries keep working. Then a flush (at most eight chunks).
//! - **One session at a time.** The queue is shared, and a finishing
//!   session flushes all of it under its token, so [`SessionLoop::settle`]
//!   waits for the previous session (at most [`PREVIOUS_BOUND`], then stops
//!   it) before a new one queues anything.
//! - **Client patches.** Each tick, after the tail, the client-patches DLL's
//!   events ([`PatchEvents::poll`]: the boot verdict once it is logged, the
//!   counts when due) are queued too. At exit, after the last tail tick and
//!   before `game_exited`, what [`PatchEvents::finish`] still owes. Only a
//!   game that ran reports them: a session dropped before the game started
//!   skips the finish. The log reads are file I/O, so they run off the
//!   runtime.
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
use super::patch_events::PatchEvents;
use super::transport::events::reason as transport_reason;
use super::transport::{Transport, TransportError};
use cimmeria_launcher_core::install_layout::binaries_dir;
use cimmeria_launcher_core::telemetry::events::{
    SessionMetaEvent, SessionMetaKind, TelemetryEvent,
};
use cimmeria_launcher_core::telemetry::runner::{is_tailed, line_to_event};
use cimmeria_launcher_core::telemetry::tail::Tailer;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use tokio::sync::oneshot;
use tokio::task::{AbortHandle, JoinHandle};

pub(super) mod events;

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
    /// Events that could not be queued (lost).
    pub not_queued: u64,
}

/// How long a new Play waits for the previous session to finish its bundle
/// and last flush. Past it the previous session is stopped, and what it had
/// not sent goes with the new one.
pub const PREVIOUS_BOUND: Duration = Duration::from_secs(35);

/// A running session. Dropping it ends the session as a game that never
/// ran: like a finish with no observed exit, but with no client-patches
/// finish (Play stopped before the game started). The task itself is never
/// aborted, except by the next session after [`PREVIOUS_BOUND`].
pub struct SessionLoop {
    exit: oneshot::Sender<Option<GameExit>>,
    task: JoinHandle<SessionEnd>,
}

impl SessionLoop {
    /// Wait for the previous session on this queue to finish (see
    /// [`PREVIOUS_BOUND`]), before [`Settled::begin`]. The queue is shared,
    /// and a finishing session flushes all of it under its own token, so a
    /// new session's events queued while the old one is still flushing
    /// would go out under the old session id. Play settles before it
    /// commits Running (the wait stays cancellable) and begins once it has
    /// made the game's [`PatchEvents`].
    pub async fn settle(transport: Transport) -> Settled {
        settle_previous(transport.queue_dir(), PREVIOUS_BOUND).await;
        Settled(transport)
    }

    /// Take the baseline and start the loop, with no wait for a previous
    /// session: [`SessionLoop::settle`] is the one Play uses. On the async
    /// runtime, before the game is spawned. `game_dir` is the install's
    /// `game` directory; `patches`, when given, is reported through the
    /// session.
    pub fn begin(transport: Transport, game_dir: &Path, patches: Option<PatchEvents>) -> Self {
        let binaries = binaries_dir(game_dir);
        let queue_dir = transport.queue_dir().to_path_buf();
        // Before the first walk: a file created after this is the session's.
        // A second early, because filesystems stamp creation times from a
        // coarse clock (NTFS: the timer tick, up to ~16 ms behind a precise
        // `now()`), so a file made just after this could read as older.
        let baseline = SystemTime::now() - BASELINE_SLACK;
        let mut tailer = Tailer::new();
        tailer.refresh(&binaries, |path| is_tailed(&binaries, path));
        let (exit, exited) = oneshot::channel();
        let task = tokio::spawn(run(transport, tailer, binaries, baseline, patches, exited));
        previous_sessions()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(queue_dir, task.abort_handle());
        Self { exit, task }
    }

    /// The game ran and is gone: hand over its exit, if it was observed. The
    /// returned handle resolves when the bundle and the last flush are done;
    /// the launch worker does not wait on it.
    pub fn finish(self, exit: Option<GameExit>) -> JoinHandle<SessionEnd> {
        let _ = self.exit.send(exit);
        self.task
    }
}

/// A transport whose queue no earlier session is still sending from:
/// [`SessionLoop::settle`]'s result, ready to [`Settled::begin`].
pub struct Settled(Transport);

impl Settled {
    /// [`SessionLoop::begin`] on the settled transport.
    pub fn begin(self, game_dir: &Path, patches: Option<PatchEvents>) -> SessionLoop {
        SessionLoop::begin(self.0, game_dir, patches)
    }
}

/// The last session begun on each queue directory. In memory only: a
/// session never outlives the process.
fn previous_sessions() -> &'static Mutex<BTreeMap<PathBuf, AbortHandle>> {
    static SESSIONS: Mutex<BTreeMap<PathBuf, AbortHandle>> = Mutex::new(BTreeMap::new());
    &SESSIONS
}

/// Wait until the last session begun on `queue_dir` has finished, for at
/// most `bound`; then stop it. A stopped session sends nothing more: what it
/// had queued goes with the next session's flushes.
async fn settle_previous(queue_dir: &Path, bound: Duration) {
    let Some(previous) = previous_sessions()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(queue_dir)
    else {
        return;
    };
    let deadline = tokio::time::Instant::now() + bound;
    while !previous.is_finished() {
        if tokio::time::Instant::now() >= deadline {
            events::previous_session_stopped(bound);
            previous.abort();
            // An abort lands at the task's next await point.
            while !previous.is_finished() {
                tokio::time::sleep(SETTLE_POLL).await;
            }
            return;
        }
        tokio::time::sleep(SETTLE_POLL).await;
    }
}

const SETTLE_POLL: Duration = Duration::from_millis(50);

/// How far before [`SessionLoop::begin`] the "created this session" line is
/// drawn. Only an older log renamed into the tailed set in the last second
/// before Play would be sent from its start.
const BASELINE_SLACK: Duration = Duration::from_secs(1);

async fn run(
    transport: Transport,
    mut tailer: Tailer,
    binaries: PathBuf,
    baseline: SystemTime,
    mut patches: Option<PatchEvents>,
    mut exited: oneshot::Receiver<Option<GameExit>>,
) -> SessionEnd {
    let started = std::time::Instant::now();
    let mut end = SessionEnd::default();
    let mut ticker = tokio::time::interval(transport.flush_interval().await);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    if queue(&transport, meta(SessionMetaKind::Started, None), &mut end)
        .await
        .is_err()
    {
        return withdrawn(end);
    }
    // A dropped sender is a Play that stopped before the game started.
    let (exit, ran) = loop {
        tokio::select! {
            biased;
            exit = &mut exited => break exit.map_or((None, false), |exit| (exit, true)),
            _ = ticker.tick() => {
                if tick(&transport, &mut tailer, &binaries, baseline, &mut end).await.is_err()
                    || queue_patches(&transport, &mut patches, PatchStep::Poll, &mut end)
                        .await
                        .is_err()
                    || transport.flush().await == Err(TransportError::NotOptedIn)
                {
                    return withdrawn(end);
                }
            }
        }
    };
    end.exit = exit;
    if tick(&transport, &mut tailer, &binaries, baseline, &mut end)
        .await
        .is_err()
    {
        return withdrawn(end);
    }
    // LX-08c's rule: a game that never ran has no client-patches log to
    // report on, so its owed events are not made up from a missing log.
    if ran
        && queue_patches(&transport, &mut patches, PatchStep::Finish, &mut end)
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
        Err(code) => events::bundle_not_sent(code),
    }
    match tokio::time::timeout(SHUTDOWN_BOUND, transport.shutdown()).await {
        Ok(Ok(_)) => {}
        Ok(Err(code)) => events::final_flush_failed(transport_reason(code)),
        Err(_) => events::final_flush_failed("timeout"),
    }
    if end.not_queued > 0 {
        events::session_events_lost(end.not_queued);
    }
    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    events::game_session_finished(&end, duration_ms);
    end
}

fn withdrawn(mut end: SessionEnd) -> SessionEnd {
    end.withdrawn = true;
    events::game_session_withdrawn(end.events);
    end
}

/// Queue every complete line written since the last tick. `Err` only when
/// the player has opted out; other failures drop the line and go on.
async fn tick(
    transport: &Transport,
    tailer: &mut Tailer,
    binaries: &Path,
    baseline: SystemTime,
    end: &mut SessionEnd,
) -> Result<(), ()> {
    if !transport.consented() {
        return Err(());
    }
    tailer.follow(binaries, |path| is_tailed(binaries, path), baseline);
    for line in tailer.tick() {
        queue(transport, line_to_event(line), end).await?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum PatchStep {
    Poll,
    Finish,
}

/// Queue the client-patches events `step` returns. The log is read off the
/// runtime; `patches` goes there and comes back, and is lost only if the
/// read panicked. `Err` only when the player has opted out.
async fn queue_patches(
    transport: &Transport,
    patches: &mut Option<PatchEvents>,
    step: PatchStep,
    end: &mut SessionEnd,
) -> Result<(), ()> {
    let Some(mut events) = patches.take() else {
        return Ok(());
    };
    let read = tokio::task::spawn_blocking(move || {
        let owed = match step {
            PatchStep::Poll => events.poll(std::time::Instant::now()),
            PatchStep::Finish => events.finish(),
        };
        (events, owed)
    })
    .await;
    let Ok((events, owed)) = read else {
        events::client_patches_read_failed();
        return Ok(());
    };
    *patches = Some(events);
    for event in owed {
        queue(transport, event, end).await?;
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
            end.not_queued += 1;
            events::event_queue_failed(code, end.not_queued);
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
