use super::*;
use std::io::{self, Cursor, Read};

struct ControlledInput {
    chunks: mpsc::Receiver<Vec<u8>>,
    requested: mpsc::Sender<()>,
    current: Cursor<Vec<u8>>,
}
impl Read for ControlledInput {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if self.current.position() == self.current.get_ref().len() as u64 {
            let _ = self.requested.send(());
            match self.chunks.recv() {
                Ok(bytes) => self.current = Cursor::new(bytes),
                Err(_) => return Ok(0),
            }
        }
        self.current.read(output)
    }
}
#[test]
fn control_reader_ignores_other_id_and_cancels_on_match_eof_or_malformed() {
    let id = Uuid::new_v4();
    let cancel = CancellationToken::new();
    let (input, chunks) = mpsc::channel();
    let (requested, requests) = mpsc::channel();
    let observer = cancel.clone();
    let worker = std::thread::spawn(move || {
        observe_controls(
            BufReader::new(ControlledInput {
                chunks,
                requested,
                current: Cursor::new(vec![]),
            }),
            id,
            observer,
        )
    });
    requests.recv_timeout(Duration::from_secs(2)).unwrap();
    let line = |id| {
        format!(
            "{}\n",
            serde_json::json!({"schema_version":1,"operation_id":id,"cancel":true})
        )
        .into_bytes()
    };
    input.send(line(Uuid::new_v4())).unwrap();
    requests.recv_timeout(Duration::from_secs(2)).unwrap(); // Observer processed the wrong ID and read again.
    assert!(!cancel.is_cancelled());
    input.send(line(id)).unwrap();
    worker.join().unwrap();
    assert!(cancel.is_cancelled());
    for bytes in [b"".as_slice(), b"not json\n"] {
        let token = CancellationToken::new();
        observe_controls(Cursor::new(bytes), id, token.clone());
        assert!(token.is_cancelled());
    }
}
fn terminal() -> WorkerEvent {
    WorkerEvent {
        schema_version: 1,
        operation_id: Some(Uuid::new_v4()),
        event: EventKind::Finished { error: None },
    }
}
struct BlockedWriter {
    release: mpsc::Receiver<()>,
    entered: mpsc::Sender<()>,
}
impl Write for BlockedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let _ = self.entered.send(());
        let _ = self.release.recv();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[tokio::test]
async fn blocked_writer_does_not_extend_terminal_delivery_deadline() {
    let (release, unblock) = mpsc::channel();
    let (entered, writing) = mpsc::channel();
    let (events, pending) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        write_events(
            BlockedWriter {
                release: unblock,
                entered,
            },
            pending,
        )
    });
    events.send((terminal(), None)).unwrap();
    writing.recv_timeout(Duration::from_secs(2)).unwrap();
    events.send((terminal(), None)).unwrap(); // Fill the single pending slot.
    let start = Instant::now();
    assert!(!send_terminal(&events, terminal(), Duration::from_millis(30)).await);
    assert!(start.elapsed() < Duration::from_secs(1));
    drop(events);
    drop(release);
    worker.join().unwrap();
}
#[tokio::test]
async fn failed_output_is_not_reported_as_delivered() {
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let (events, pending) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || write_events(Broken, pending));
    assert!(!send_terminal(&events, terminal(), Duration::from_secs(1)).await);
    worker.join().unwrap();
}

#[tokio::test]
async fn queued_terminal_still_times_out_when_flush_cannot_be_acknowledged() {
    let (release, unblock) = mpsc::channel();
    let (entered, writing) = mpsc::channel();
    let (events, pending) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        write_events(
            BlockedWriter {
                release: unblock,
                entered,
            },
            pending,
        )
    });
    let result = tokio::time::timeout(
        Duration::from_secs(1),
        send_terminal(&events, terminal(), Duration::from_millis(30)),
    )
    .await;
    writing.recv_timeout(Duration::from_secs(2)).unwrap();
    drop(events);
    drop(release);
    worker.join().unwrap();
    assert!(!result.expect("terminal acknowledgement must obey its own deadline"));
}
