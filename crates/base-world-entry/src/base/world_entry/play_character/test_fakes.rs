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

/// A cell stand-in for `query_world_entry`'s no-DB path: on the entry's
/// `CreateEntity` it replaces the session at `addr`, then drops the reply,
/// which ends the entry's wait. So the session changes inside the
/// handler's first await on the cell.
pub(super) fn cell_replacing_the_session(
    connected: &Connected,
    addr: SocketAddr,
    account_id: u32,
) -> mpsc::Sender<BaseToCellMsg> {
    let (tx, mut rx) = mpsc::channel(4);
    let connected = Arc::clone(connected);
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if matches!(msg, BaseToCellMsg::CreateEntity { .. }) {
                replace_session(&connected, addr, account_id);
            }
        }
    });
    tx
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
#[cfg(feature = "lab-chaos")]
pub(super) struct FailingTransport;

#[cfg(feature = "lab-chaos")]
#[async_trait]
impl Transport for FailingTransport {
    async fn send_to(&self, _bytes: &[u8], _addr: SocketAddr) -> io::Result<usize> {
        Err(io::Error::other("send failed"))
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok("127.0.0.1:0".parse().unwrap())
    }
}
