//! Fakes for `playCharacter`'s await points: a cell and transports that
//! replace the session mid-entry (a relaunch takeover) or fail the send.

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::test_default_connected_client_state;

pub(super) type Connected = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;

/// Puts a fresh session (new owner token, same account) at `addr`, as a
/// relaunch takeover does.
fn replace_session(connected: &Connected, addr: SocketAddr, account_id: u32) {
    let mut fresh = test_default_connected_client_state();
    fresh.account_id = account_id;
    connected.lock().unwrap().insert(addr, fresh);
}

/// What a fake cell received, in order: `("create", id)`, `("destroy", id)`
/// and `("disconnect", id)`.
pub(super) type CellLog = Arc<Mutex<Vec<(&'static str, u32)>>>;

/// A cell stand-in for `query_world_entry`'s no-DB path that records the
/// entity messages it gets. On a `CreateEntity` it first replaces the
/// session at `replace` when one is given, then drops the reply, which ends
/// the entry's wait.
fn fake_cell(
    replace: Option<(Connected, SocketAddr, u32)>,
) -> (mpsc::Sender<BaseToCellMsg>, CellLog) {
    let (tx, mut rx) = mpsc::channel(4);
    let log = CellLog::default();
    let seen = Arc::clone(&log);
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let row = match msg {
                BaseToCellMsg::CreateEntity { entity_id, .. } => {
                    if let Some((connected, addr, account_id)) = &replace {
                        replace_session(connected, *addr, *account_id);
                    }
                    ("create", entity_id)
                }
                BaseToCellMsg::DestroyEntity { entity_id } => ("destroy", entity_id),
                BaseToCellMsg::DisconnectEntity { entity_id, .. } => ("disconnect", entity_id),
                _ => continue,
            };
            seen.lock().unwrap().push(row);
        }
    });
    (tx, log)
}

/// [`fake_cell`] that replaces the session at `addr` on the entry's
/// `CreateEntity`, so the session changes inside the handler's first await
/// on the cell.
pub(super) fn cell_replacing_the_session(
    connected: &Connected,
    addr: SocketAddr,
    account_id: u32,
) -> (mpsc::Sender<BaseToCellMsg>, CellLog) {
    fake_cell(Some((Arc::clone(connected), addr, account_id)))
}

/// [`fake_cell`] that only records.
pub(super) fn recording_cell() -> (mpsc::Sender<BaseToCellMsg>, CellLog) {
    fake_cell(None)
}

/// The fake cell's rows once it has handled `count` of them (it runs on
/// its own task).
pub(super) async fn cell_rows(log: &CellLog, count: usize) -> Vec<(&'static str, u32)> {
    let wait = async {
        loop {
            if log.lock().unwrap().len() >= count {
                return;
            }
            tokio::task::yield_now().await;
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), wait)
        .await
        .expect("the fake cell got the expected messages");
    // A moment longer, so a row too many shows.
    tokio::task::yield_now().await;
    log.lock().unwrap().clone()
}

/// Sends nothing; replaces the session at `addr` during the send, so the
/// session changes between RESET_ENTITIES and the state the handler stores.
pub(super) struct ReplacingTransport {
    pub(super) connected: Connected,
    pub(super) addr: SocketAddr,
    pub(super) account_id: u32,
}

#[async_trait]
impl Transport for ReplacingTransport {
    async fn send_to(&self, bytes: &[u8], _addr: SocketAddr) -> io::Result<usize> {
        replace_session(&self.connected, self.addr, self.account_id);
        Ok(bytes.len())
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok("127.0.0.1:0".parse().unwrap())
    }
}

/// Every send fails, as a closed socket does.
pub(super) struct FailingTransport;

#[async_trait]
impl Transport for FailingTransport {
    async fn send_to(&self, _bytes: &[u8], _addr: SocketAddr) -> io::Result<usize> {
        Err(io::Error::other("send failed"))
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok("127.0.0.1:0".parse().unwrap())
    }
}
