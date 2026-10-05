//! Native-owned helper supervision. No UI command accepts an executable/env map.
//! A lost protocol/process is uncertain; it is never promoted to installation success.
mod owned;
pub use owned::run_owned;

use crate::archive_worker::{
    CancelRequest, EventKind, ExtractError, ExtractRequest, WorkerEvent, MAX_FRAME,
};
use std::{collections::BTreeMap, ffi::OsString, path::PathBuf, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::{mpsc, watch},
    time::{timeout, Instant},
};
use tokio_util::sync::CancellationToken;

/// Construct only in native platform adapters, from verified runtime/helper paths.
pub struct HelperCommand {
    pub executable: PathBuf,
    pub arguments: Vec<OsString>,
    pub directory: PathBuf,
    pub environment: BTreeMap<OsString, OsString>,
}
#[derive(Clone, Copy)]
pub struct Deadlines {
    pub request: Duration,
    pub operation: Duration,
    pub cancel_grace: Duration,
    pub exit: Duration,
}
impl Default for Deadlines {
    fn default() -> Self {
        Self {
            request: Duration::from_secs(5),
            operation: Duration::from_secs(1800),
            cancel_grace: Duration::from_secs(30),
            exit: Duration::from_secs(3),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    InvalidRequest,
    Spawn,
    Ownership,
    Transport,
    Protocol,
    Deadline,
    Cancelled,
    Exit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Request was never sent; the trusted helper cannot have begun extraction.
    NotStarted(Fault),
    Completed,
    Cancelled,
    Failed(ExtractError),
    /// Caller must retain its journal/recovery gate. A host PID is not a guest handle.
    ReconciliationRequired(Fault),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub current: u64,
    pub total: u64,
}

/// Persist native operation admission before calling. `record_host` runs after
/// spawn but before any request is sent. It must durably record the host PID or
/// refuse dispatch. A Wine host PID alone does not identify every guest process.
/// Keep this future in the native operation scope, independent of webview lifetime.
pub async fn run(
    spec: HelperCommand,
    request: ExtractRequest,
    cancel: CancellationToken,
    progress: watch::Sender<Option<Progress>>,
    limits: Deadlines,
    record_host: impl FnOnce(u32) -> Result<(), ()>,
) -> Outcome {
    if !spec.executable.is_absolute()
        || !spec.directory.is_absolute()
        || request.schema_version != 1
    {
        return Outcome::NotStarted(Fault::InvalidRequest);
    }
    let Ok(mut frame) = serde_json::to_vec(&request) else {
        return Outcome::NotStarted(Fault::InvalidRequest);
    };
    frame.push(b'\n');
    if frame.len() > MAX_FRAME {
        return Outcome::NotStarted(Fault::InvalidRequest);
    }
    if cancel.is_cancelled() {
        return Outcome::NotStarted(Fault::Cancelled);
    }
    let mut command = Command::new(spec.executable);
    command
        .args(spec.arguments)
        .current_dir(spec.directory)
        .env_clear()
        .envs(spec.environment)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW, including console helpers.
    let Ok(mut child) = command.spawn() else {
        return Outcome::NotStarted(Fault::Spawn);
    };
    if child.id().is_none_or(|id| record_host(id).is_err()) {
        drop(child.stdin.take());
        stop(&mut child, limits.exit).await;
        return Outcome::NotStarted(Fault::Ownership);
    }
    let mut input = child.stdin.take().expect("piped stdin");
    let output = child.stdout.take().expect("piped stdout");
    let (events, mut received) = mpsc::channel(1);
    let reader = tokio::spawn(async move {
        let mut output = BufReader::new(output);
        loop {
            let mut frame = Vec::new();
            let read = (&mut output)
                .take(MAX_FRAME as u64 + 1)
                .read_until(b'\n', &mut frame)
                .await;
            let event = match read {
                Ok(0) => Ok(None),
                Ok(_) if frame.len() <= MAX_FRAME && frame.ends_with(b"\n") => {
                    serde_json::from_slice::<WorkerEvent>(&frame)
                        .map(Some)
                        .map_err(|_| Fault::Protocol)
                }
                Ok(_) => Err(Fault::Protocol),
                Err(_) => Err(Fault::Transport),
            };
            let last = !matches!(event, Ok(Some(_)));
            if events.send(event).await.is_err() || last {
                break;
            }
        }
    });
    // Abort the pipe reader on every return, including cancellation of this future.
    struct Reader(tokio::task::JoinHandle<()>);
    impl Drop for Reader {
        fn drop(&mut self) {
            self.0.abort();
        }
    }
    let _reader = Reader(reader);
    if !matches!(
        timeout(limits.request, input.write_all(&frame)).await,
        Ok(Ok(()))
    ) {
        drop(input);
        stop(&mut child, limits.cancel_grace).await;
        return Outcome::ReconciliationRequired(Fault::Transport);
    }
    let mut terminal = None;
    let mut status = None;
    let mut eof = false;
    let mut sent_cancel = false;
    let mut deadline = Instant::now() + limits.operation;
    let fault = loop {
        if eof && status.is_some() {
            break None;
        }
        tokio::select! {
            _=tokio::time::sleep_until(deadline)=>break Some(Fault::Deadline),
            _=cancel.cancelled(), if !sent_cancel && terminal.is_none()=>{
                sent_cancel=true;
                deadline=deadline.min(Instant::now()+limits.cancel_grace);
                let control=CancelRequest {schema_version:1,operation_id:request.operation_id,cancel:true};
                let mut bytes=serde_json::to_vec(&control).expect("control serializes");bytes.push(b'\n');
                if let Err(fault)=write_control(&mut input,&bytes,limits.request,deadline).await {break Some(fault);}
            },
            event=received.recv(), if !eof=>match event {
                Some(Ok(Some(event)))=>{
                    if event.schema_version!=1 || event.operation_id!=Some(request.operation_id) || terminal.is_some() {
                        break Some(Fault::Protocol);
                    }
                    match event.event {
                        EventKind::Progress {current,total}=>{progress.send_replace(Some(Progress {current,total}));},
                        EventKind::Finished {error}=>{
                            terminal=Some(error);
                            deadline=deadline.min(Instant::now()+limits.exit);
                        },
                    }
                },
                Some(Ok(None))=>{eof=true;deadline=deadline.min(Instant::now()+limits.exit);},
                Some(Err(fault))=>break Some(fault),
                None=>break Some(Fault::Transport),
            },
            exit=child.wait(), if status.is_none()=>match exit {
                Ok(exit)=>{status=Some(exit);deadline=deadline.min(Instant::now()+limits.exit);},
                Err(_)=>break Some(Fault::Exit),
            },
        }
    };
    drop(input); // EOF also asks a still-running helper to cancel cooperatively.
    if let Some(fault) = fault {
        let grace = if sent_cancel {
            deadline.saturating_duration_since(Instant::now())
        } else {
            limits.cancel_grace
        };
        stop(&mut child, grace).await;
        return Outcome::ReconciliationRequired(fault);
    }
    match (terminal, status) {
        (Some(None), Some(exit)) if exit.success() => Outcome::Completed,
        (Some(Some(ExtractError::Cancelled)), Some(exit)) if !exit.success() => Outcome::Cancelled,
        (Some(Some(error)), Some(exit)) if !exit.success() => Outcome::Failed(error),
        _ => Outcome::ReconciliationRequired(Fault::Exit),
    }
}

/// Never kill by process name. A forced Wine-host stop cannot prove guest death;
/// every caller of this path returns an uncertain result and retains recovery.
async fn stop(child: &mut Child, grace: Duration) {
    if !matches!(timeout(grace, child.wait()).await, Ok(Ok(_))) {
        let _ = child.start_kill();
        let _ = timeout(Duration::from_secs(3), child.wait()).await;
    }
}

/// A backpressured control pipe must not suspend the active cancellation budget.
async fn write_control(
    input: &mut (impl AsyncWrite + Unpin),
    bytes: &[u8],
    request_limit: Duration,
    deadline: Instant,
) -> Result<(), Fault> {
    let write_deadline = deadline.min(Instant::now() + request_limit);
    match tokio::time::timeout_at(write_deadline, input.write_all(bytes)).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(_)) => Err(Fault::Transport),
        Err(_) => Err(Fault::Deadline),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn blocked_control_write_obeys_active_deadline() {
        // Hold the reader alive without draining: the second byte cannot fit.
        let (mut writer, _reader) = tokio::io::duplex(1);
        let deadline = Instant::now() + Duration::from_millis(20);
        let result = timeout(
            Duration::from_secs(1),
            write_control(&mut writer, b"xx", Duration::from_secs(30), deadline),
        )
        .await
        .expect("active deadline must win over the request timeout");
        assert_eq!(result, Err(Fault::Deadline));
        assert!(Instant::now() >= deadline);
        assert_eq!(
            deadline.saturating_duration_since(Instant::now()),
            Duration::ZERO
        );
    }
}
