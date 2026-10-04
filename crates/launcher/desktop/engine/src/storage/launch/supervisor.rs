//! A helper owns the Windows process handle until exit. stdout is bounded per
//! message; the game itself has no artificial lifetime deadline.
use super::*;
use crate::helper_supervisor::HelperCommand;
use cimmeria_runtime_probe::game_launch::{Event, Message, Request, MAX_MESSAGE};
use std::time::{Duration, Instant};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::Command,
};
use tokio_util::sync::CancellationToken;

pub(super) async fn run(
    spec: HelperCommand,
    request: Request,
    cancel: CancellationToken,
    mut observe: impl FnMut(Observation) -> Result<(), IntentError>,
) -> Observation {
    if cancel.is_cancelled() {
        return Observation::Cancelled;
    }
    let Ok(mut bytes) = serde_json::to_vec(&request) else {
        return Observation::NotStarted;
    };
    if bytes.len() > MAX_MESSAGE {
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
    let Ok(mut child) = command.spawn() else {
        return Observation::NotStarted;
    };
    let host_pid = child.id().unwrap_or(0);
    let host = Observation::HostStarted { host_pid };
    if host_pid == 0 || observe(host).is_err() {
        return Observation::Unknown;
    }
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
        _ => return Observation::Unknown,
    };
    let (guest_pid, started) = match event {
        Event::ProcessStarted { guest_pid } => {
            if observe(Observation::ProcessStarted {
                host_pid,
                guest_pid,
            })
            .is_err()
            {
                return Observation::Unknown;
            }
            (guest_pid, Instant::now())
        }
        Event::NotStarted => {
            return if tokio::time::timeout(Duration::from_secs(10), child.wait())
                .await
                .is_ok_and(|s| s.is_ok_and(|s| s.success()))
            {
                Observation::NotStarted
            } else {
                Observation::Unknown
            };
        }
        _ => return Observation::Unknown,
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
    let Ok(Event::ProcessExited {
        guest_pid: exited_pid,
        code,
    }) = event
    else {
        return Observation::Unknown;
    };
    if exited_pid != guest_pid {
        return Observation::Unknown;
    }
    if !tokio::time::timeout(Duration::from_secs(10), child.wait())
        .await
        .is_ok_and(|s| s.is_ok_and(|s| s.success()))
    {
        return Observation::Unknown;
    }
    Observation::ProcessExited {
        host_pid,
        guest_pid,
        code,
        early: started.elapsed() < Duration::from_secs(10),
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
