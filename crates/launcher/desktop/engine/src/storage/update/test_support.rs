//! Explicit portable test entry points. Production platform gates are unchanged.
use super::*;
use std::sync::Mutex;
use tokio::sync::oneshot;
pub fn prepare(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    url: String,
) -> Result<preparation::Preparation, IntentError> {
    preparation::start(state, id, reqwest::Client::new(), url)
}
pub fn commit(
    state: Arc<Mutex<DesktopState>>,
    prepared: preparation::Prepared,
    interrupt: bool,
) -> Result<oneshot::Receiver<Result<(), preparation::Failure>>, IntentError> {
    if !interrupt {
        return commit::start(state, prepared);
    }
    let (send, receive) = oneshot::channel();
    tokio::task::spawn_blocking(move || {
        let result = commit::replace(&state, &prepared, |point| {
            if point == commit::Point::AfterPromotion {
                Err(StorageError::Io)
            } else {
                Ok(())
            }
        });
        let _ = send.send(result);
    });
    Ok(receive)
}
pub fn recover(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    revision: u64,
) -> Result<oneshot::Receiver<Result<(), IntentError>>, IntentError> {
    recovery::dispatch(state, id, revision)
}
pub fn cleanup(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    revision: u64,
) -> Result<oneshot::Receiver<Result<(), IntentError>>, IntentError> {
    cleanup::dispatch(state, id, revision)
}

/// A second valid signed source, with a distinguishing entry for rollback UAT.
pub fn previous_archive() -> Vec<u8> {
    use std::io::{Cursor, Write};
    let mut archive =
        zip::ZipWriter::new_append(Cursor::new(crate::install_worker::fixtures::archive(true)))
            .unwrap();
    archive
        .start_file("previous.txt", zip::write::SimpleFileOptions::default())
        .unwrap();
    archive.write_all(b"previous signed release").unwrap();
    archive.finish().unwrap().into_inner()
}
pub fn abandon(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    revision: u64,
) -> Result<oneshot::Receiver<Result<(), IntentError>>, IntentError> {
    abandon::dispatch(state, id, revision, true)
}
pub fn discard(
    state: Arc<Mutex<DesktopState>>,
    id: Uuid,
    revision: u64,
) -> Result<oneshot::Receiver<Result<(), IntentError>>, IntentError> {
    discard::dispatch(state, id, revision, true)
}
