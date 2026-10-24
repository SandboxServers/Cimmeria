//! A helper owns the Windows process handle until exit. stdout is bounded per
//! message; the game itself has no artificial lifetime deadline.
use super::*;
use crate::failure::FailureCode;
use crate::helper_supervisor::HelperCommand;
use cimmeria_runtime_probe::game_launch::{Event, Message, Request, MAX_MESSAGE};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::Command,
    sync::watch,
};
use tokio_util::sync::CancellationToken;

/// `run` for `plan`'s launch: each observation is recorded on the launch
/// record and published on `updates`, and a spawn failure is noted on the
/// record. Lazy: nothing is spawned until the future is awaited.
pub(super) fn run_recorded<'a>(
    state: &'a Mutex<DesktopState>,
    plan: &'a Plan,
    spec: HelperCommand,
    request: Request,
    cancel: CancellationToken,
    updates: &'a watch::Sender<Observation>,
) -> impl std::future::Future<Output = Observation> + 'a {
    run(
        spec,
        request,
        cancel,
        move |observation| {
            state
                .lock()
                .map_err(|_| StorageError::Io)?
                .record_launch(plan, observation)?;
            updates.send_replace(observation);
            Ok(())
        },
        move |code| failure::note(state, plan, code),
    )
}

pub(super) async fn run(
    spec: HelperCommand,
    request: Request,
    cancel: CancellationToken,
    observe: impl FnMut(Observation) -> Result<(), IntentError>,
    on_failure: impl FnOnce(FailureCode),
) -> Observation {
    let id = request.operation_id;
    if cancel.is_cancelled() {
        return Observation::Cancelled;
    }
    let mut startup_timer = Some(
        crate::phase_timing::Context::for_operation(id)
            .start(crate::phase_timing::Phase::PlayHelperStart),
    );
    let result = run_timed(spec, request, observe, on_failure, &mut startup_timer).await;
    if let Some(timer) = startup_timer {
        timer.finish(crate::phase_timing::Outcome::Failed);
    }
    result
}

// The outer future owns the unfinished timer: dropping it still records an
// interruption, while a known return before ProcessStarted records failure.
async fn run_timed(
    spec: HelperCommand,
    request: Request,
    mut observe: impl FnMut(Observation) -> Result<(), IntentError>,
    on_failure: impl FnOnce(FailureCode),
    startup_timer: &mut Option<crate::phase_timing::Timer>,
) -> Observation {
    let id = request.operation_id;
    let Ok(mut bytes) = serde_json::to_vec(&request) else {
        events::launch_not_started(id, "request_invalid", FailureCode::SpawnFailed);
        on_failure(FailureCode::SpawnFailed);
        return Observation::NotStarted;
    };
    if bytes.len() > MAX_MESSAGE {
        events::launch_not_started(id, "request_too_large", FailureCode::SpawnFailed);
        on_failure(FailureCode::SpawnFailed);
        return Observation::NotStarted;
    }
    let mut command = Command::new(&spec.executable);
    command
        .args(&spec.arguments)
        .current_dir(&spec.directory)
        .env_clear()
        .envs(&spec.environment)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    // No kill-on-drop: killing the host is not equivalent to stopping its guest.
    let spawned = crate::phase_timing::Context::for_operation(id)
        .measure(crate::phase_timing::Phase::PlayHelperSpawn, || {
            command.spawn()
        });
    let mut child = match spawned {
        Ok(child) => child,
        Err(error) => {
            let failure = failure::classify_spawn(
                error.kind(),
                error.raw_os_error(),
                cfg!(target_os = "macos"),
            );
            events::launch_spawn_failed(id, failure, error.raw_os_error());
            on_failure(failure);
            return Observation::NotStarted;
        }
    };
    let host_pid = child.id().unwrap_or(0);
    let host = Observation::HostStarted { host_pid };
    if host_pid == 0 {
        events::launch_failed(id, "no_pid");
        return Observation::Unknown;
    }
    if observe(host).is_err() {
        events::launch_failed(id, "observe_failed");
        return Observation::Unknown;
    }
    let handshake_timer = crate::phase_timing::Context::for_operation(id)
        .start(crate::phase_timing::Phase::PlayHelperHandshake);
    let mut input = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    bytes.push(b'\n');
    if !tokio::time::timeout(Duration::from_secs(5), async {
        input.write_all(&bytes).await?;
        input.shutdown().await
    })
    .await
    .is_ok_and(|r| r.is_ok())
    {
        events::launch_failed(id, "stdin_write_failed");
        handshake_timer.finish(crate::phase_timing::Outcome::Failed);
        return Observation::Unknown;
    }
    drop(input);
    let mut reader = BufReader::new(stdout);
    let first = tokio::time::timeout(
        Duration::from_secs(60),
        line(&mut reader, request.operation_id),
    )
    .await;
    let event = match first {
        Ok(Ok(event)) => event,
        _ => {
            events::launch_failed(id, "no_first_event");
            handshake_timer.finish(crate::phase_timing::Outcome::Failed);
            return Observation::Unknown;
        }
    };
    let (guest_pid, started) = match event {
        Event::ProcessStarted { guest_pid } => {
            if observe(Observation::ProcessStarted {
                host_pid,
                guest_pid,
            })
            .is_err()
            {
                events::launch_failed(id, "observe_failed");
                handshake_timer.finish(crate::phase_timing::Outcome::Failed);
                return Observation::Unknown;
            }
            if let Some(timer) = startup_timer.take() {
                timer.finish(crate::phase_timing::Outcome::Succeeded);
            }
            handshake_timer.finish(crate::phase_timing::Outcome::Succeeded);
            events::game_started(id);
            (guest_pid, Instant::now())
        }
        Event::NotStarted { stage, win32 } => {
            handshake_timer.finish(crate::phase_timing::Outcome::Failed);
            return if tokio::time::timeout(Duration::from_secs(10), child.wait())
                .await
                .is_ok_and(|s| s.is_ok_and(|s| s.success()))
            {
                let failure = failure::classify_helper_not_started(stage, win32);
                events::launch_not_started_at(id, stage, win32, failure);
                on_failure(failure);
                Observation::NotStarted
            } else {
                events::launch_failed(id, "helper_unclean_exit");
                Observation::Unknown
            };
        }
        Event::Unknown { stage, win32 } => {
            handshake_timer.finish(crate::phase_timing::Outcome::Failed);
            events::launch_failed_at(id, stage, win32);
            return Observation::Unknown;
        }
        _ => {
            events::launch_failed(id, "unexpected_event");
            handshake_timer.finish(crate::phase_timing::Outcome::Failed);
            return Observation::Unknown;
        }
    };
    // Cancellation is admission/pre-spawn only. Closing the UI or cancelling a
    // start cannot silently kill a game whose own threads are already running.
    let event = tokio::select! {
        biased;
        result = line(&mut reader, request.operation_id) => result,
        _ = child.wait() => {
            match tokio::time::timeout(Duration::from_secs(2), line(&mut reader, request.operation_id)).await {
                Ok(result) => result,
                Err(_) => Err(()),
            }
        }
    };
    if let Ok(Event::Unknown { stage, win32 }) = event {
        events::launch_failed_at(id, stage, win32);
        return Observation::Unknown;
    }
    let Ok(Event::ProcessExited {
        guest_pid: exited_pid,
        code,
    }) = event
    else {
        events::launch_failed(id, "no_exit_event");
        return Observation::Unknown;
    };
    if exited_pid != guest_pid {
        events::launch_failed(id, "pid_mismatch");
        return Observation::Unknown;
    }
    if !tokio::time::timeout(Duration::from_secs(10), child.wait())
        .await
        .is_ok_and(|s| s.is_ok_and(|s| s.success()))
    {
        events::launch_failed(id, "helper_unclean_exit");
        return Observation::Unknown;
    }
    let ran = started.elapsed();
    let early = ran < Duration::from_secs(10);
    events::game_exited(id, code, early, ran);
    Observation::ProcessExited {
        host_pid,
        guest_pid,
        code,
        early,
    }
}
async fn line(reader: &mut (impl tokio::io::AsyncBufRead + Unpin), id: Uuid) -> Result<Event, ()> {
    let mut bytes = Vec::new();
    let count = reader
        .take((MAX_MESSAGE + 1) as u64)
        .read_until(b'\n', &mut bytes)
        .await
        .map_err(|_| ())?;
    if count == 0 || bytes.last() != Some(&b'\n') {
        return Err(());
    }
    Message::decode(&bytes, id).map_err(|_| ())
}
#[cfg(all(test, unix))]
#[path = "supervisor_tests.rs"]
mod tests;
