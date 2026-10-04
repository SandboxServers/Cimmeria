//! Portable stdio mechanics, used only by the Windows executable entry point.
//! The caller must exit after return: a dedicated stdio thread may still wait.
use super::*;
use crate::install::Progress;
use std::io::{BufReader, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

type Delivery = (WorkerEvent, Option<mpsc::Sender<bool>>);

fn write_events(mut output: impl Write, pending: mpsc::Receiver<Delivery>) {
    while let Ok((event, ack)) = pending.recv() {
        let sent = writeln!(output, "{}", serde_json::to_string(&event).unwrap())
            .and_then(|_| output.flush())
            .is_ok();
        if let Some(ack) = ack {
            let _ = ack.send(sent);
        }
        if !sent {
            return;
        }
    }
}

fn observe_controls(mut reader: impl BufRead, id: Uuid, cancel: CancellationToken) {
    loop {
        match read_frame(&mut reader) {
            Ok(Some(frame)) if cancellation_matches(&frame, id) => {
                cancel.cancel();
                return;
            }
            Ok(Some(frame))
                if serde_json::from_slice::<CancelRequest>(&frame)
                    .is_ok_and(|r| r.schema_version == 1) => {}
            _ => {
                cancel.cancel();
                return;
            }
        }
    }
}

async fn send_terminal(
    events: &mpsc::SyncSender<Delivery>,
    event: WorkerEvent,
    timeout: Duration,
) -> bool {
    let deadline = Instant::now() + timeout;
    let (ack, received) = mpsc::channel();
    let mut terminal = (event, Some(ack));
    loop {
        match events.try_send(terminal) {
            Ok(()) => break,
            Err(mpsc::TrySendError::Disconnected(_)) => return false,
            Err(mpsc::TrySendError::Full(value)) => terminal = value,
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    loop {
        match received.try_recv() {
            Ok(sent) => return sent,
            Err(mpsc::TryRecvError::Disconnected) => return false,
            Err(mpsc::TryRecvError::Empty) => {}
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

pub async fn serve_stdio() -> i32 {
    // A stalled stdout cannot block cancellation or extraction completion.
    let (events, pending) = mpsc::sync_channel(1);
    std::thread::spawn(move || write_events(std::io::stdout(), pending));
    let mut reader = BufReader::new(std::io::stdin());
    let request = read_frame(&mut reader)
        .ok()
        .flatten()
        .and_then(|frame| serde_json::from_slice::<ExtractRequest>(&frame).ok());
    let Some(request) = request else {
        send_terminal(
            &events,
            WorkerEvent {
                schema_version: 1,
                operation_id: None,
                event: EventKind::Finished {
                    error: Some(ExtractError::InvalidRequest),
                },
            },
            Duration::from_secs(2),
        )
        .await;
        return 2;
    };
    let id = request.operation_id;
    let cancel = CancellationToken::new();
    let observer = cancel.clone();
    std::thread::spawn(move || observe_controls(reader, id, observer));
    let (progress, mut receiver) = ProgressSink::latest();
    let worker_cancel = cancel.clone();
    let mut worker =
        tokio::task::spawn_blocking(move || extract(&request, worker_cancel, progress));
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let result = loop {
        tokio::select! {
            result=&mut worker=>break result.unwrap_or(Err(ExtractError::Io)),
            _=tick.tick()=>{
                if receiver.has_changed().unwrap_or(false) {
                    let latest=receiver.borrow_and_update().clone();
                    if let Some(Progress::Extracting {current,total,..})=latest {
                        let event=WorkerEvent {schema_version:1,operation_id:Some(id),event:EventKind::Progress {current:current as u64,total:total as u64}};
                        if matches!(events.try_send((event,None)),Err(mpsc::TrySendError::Disconnected(_))) {cancel.cancel();}
                    }
                }
            }
        }
    };
    // Only after the blocking extractor has returned may the process exit.
    let sent = send_terminal(
        &events,
        WorkerEvent {
            schema_version: 1,
            operation_id: Some(id),
            event: EventKind::Finished {
                error: result.err(),
            },
        },
        Duration::from_secs(2),
    )
    .await;
    if result.is_ok() && sent {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests;
