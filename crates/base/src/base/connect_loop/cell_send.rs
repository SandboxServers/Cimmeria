//! The base-to-cell `mpsc` send seam. Every base send to the cell goes
//! through [`send_to_cell`], which logs the two ways it goes wrong: a
//! dropped cell receiver (a failed send, WARN) and a full cell queue that
//! holds the base's receive loop (a stall, WARN, throttled).

use std::net::SocketAddr;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_wire::names;

use crate::cell::messages::BaseToCellMsg;

/// An await this long or longer on the cell queue is a stall worth a row.
const STALL_THRESHOLD: Duration = Duration::from_millis(50);

/// How long one `cell_channel_backpressure` row covers. Stalls inside the
/// window are counted and carried into the next row's `suppressed`.
const STALL_WINDOW: Duration = Duration::from_secs(5);

/// The stall throttle: when the open window started, and how many stalls
/// it has swallowed since its row.
#[derive(Debug)]
struct StallWindow {
    opened: Option<Instant>,
    suppressed: u64,
}

/// The process-wide throttle behind [`log_stall`]. One global key for the
/// whole base-to-cell channel.
static STALLS: Mutex<StallWindow> = Mutex::new(StallWindow {
    opened: None,
    suppressed: 0,
});

/// Send `msg` to the cell and log what the seam can see.
///
/// `kind` is the `msg_kind` of the message and `method_index` its cell
/// method pair, when it has one (an entity move has none). `addr` and `who`
/// name the session the message came from. A send to a dropped receiver is
/// a WARN `cell_send_failed`; an await of [`STALL_THRESHOLD`] or more is a
/// throttled WARN `cell_channel_backpressure`.
pub(super) async fn send_to_cell(
    tx: &mpsc::Sender<BaseToCellMsg>,
    msg: BaseToCellMsg,
    kind: &'static str,
    addr: SocketAddr,
    who: PlayerIdentity,
    method_index: Option<u16>,
) {
    let method_name = method_index.and_then(names::player_cell_method);
    let started = Instant::now();
    let sent = tx.send(msg).await;
    let waited = started.elapsed();
    match sent {
        Ok(()) => {
            if waited >= STALL_THRESHOLD {
                log_stall(waited, tx.capacity(), kind);
            }
        }
        Err(e) => {
            tracing::warn!(
                %addr,
                event = "cell_send_failed",
                reason = "cell_channel_closed",
                error = %e,
                account_id = who.account_id,
                account_name = who.account_name,
                player_id = who.player_id,
                player_name = who.player_name,
                msg_kind = kind,
                method_index,
                method_name,
                "Base-to-cell send failed -- cell receiver dropped"
            );
        }
    }
}

/// Count one base-to-cell stall and log it at most once per
/// [`STALL_WINDOW`]. Also bumps `base_cell_channel_stalls_total` for every
/// stall.
fn log_stall(waited: Duration, queue_free: usize, kind: &'static str) {
    cimmeria_observability::counter!("base_cell_channel_stalls_total");
    let mut window = STALLS.lock().unwrap_or_else(PoisonError::into_inner);
    log_stall_at(&mut window, Instant::now(), waited, queue_free, kind);
}

/// The throttle decision and the row, at `now`. The first stall writes at
/// once; the rest of its window only count.
fn log_stall_at(
    window: &mut StallWindow,
    now: Instant,
    waited: Duration,
    queue_free: usize,
    kind: &'static str,
) {
    match window.opened {
        Some(opened) if now.saturating_duration_since(opened) < STALL_WINDOW => {
            window.suppressed += 1;
        }
        _ => {
            let suppressed = std::mem::take(&mut window.suppressed);
            window.opened = Some(now);
            tracing::warn!(
                event = "cell_channel_backpressure",
                waited_ms = waited.as_millis() as u64,
                queue_free,
                msg_kind = kind,
                suppressed,
                "Base-to-cell send waited on a full cell queue"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;

    fn sample_msg() -> BaseToCellMsg {
        BaseToCellMsg::CellMethodCall {
            entity_id: 1,
            method_index: 0,
            args: Vec::new(),
            packet_seq: None,
        }
    }

    fn sample_addr() -> SocketAddr {
        "127.0.0.1:1".parse().unwrap()
    }

    #[tokio::test]
    async fn dropped_receiver_logs_cell_channel_closed() {
        let capture = LogCapture::install();
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        send_to_cell(
            &tx,
            sample_msg(),
            "cell_method_call",
            sample_addr(),
            PlayerIdentity::UNKNOWN,
            Some(3),
        )
        .await;
        assert!(capture
            .find_event(
                tracing::Level::WARN,
                "cell receiver dropped",
                "cell_channel_closed",
            )
            .is_some());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn stalled_send_logs_backpressure_row() {
        let capture = LogCapture::install();
        let (tx, mut rx) = mpsc::channel(1);
        // Fill the one slot, then drain it after 100 ms: the next send waits.
        // The drainer keeps the receiver open for the second message, or the
        // waiting send would find the channel closed.
        tx.send(sample_msg()).await.unwrap();
        let drainer = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            rx.recv().await;
            rx.recv().await
        });
        send_to_cell(
            &tx,
            sample_msg(),
            "cell_method_call",
            sample_addr(),
            PlayerIdentity::UNKNOWN,
            Some(3),
        )
        .await;
        drainer.await.unwrap();
        let row = capture
            .find_message(tracing::Level::WARN, "waited on a full cell queue")
            .expect("backpressure row");
        assert_eq!(
            row.fields.get("msg_kind").map(String::as_str),
            Some("cell_method_call")
        );
    }

    #[test]
    fn second_stall_in_window_is_suppressed() {
        let capture = LogCapture::install();
        let rows = || -> Vec<String> {
            capture
                .all()
                .iter()
                .filter(|c| c.message_contains("waited on a full cell queue"))
                .map(|c| c.fields.get("suppressed").cloned().unwrap_or_default())
                .collect()
        };
        let mut window = StallWindow {
            opened: None,
            suppressed: 0,
        };
        let start = Instant::now();
        let waited = Duration::from_millis(100);
        log_stall_at(&mut window, start, waited, 0, "cell_method_call");
        log_stall_at(&mut window, start, waited, 0, "cell_method_call");
        assert_eq!(rows(), vec!["0".to_string()]);

        // The first stall after the window writes the row that carries the one
        // swallowed inside it.
        log_stall_at(
            &mut window,
            start + STALL_WINDOW,
            waited,
            0,
            "cell_method_call",
        );
        assert_eq!(rows(), vec!["0".to_string(), "1".to_string()]);
    }
}
