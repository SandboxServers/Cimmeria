//! Lab chaos at `playCharacter` (D-CH10, lab-chaos builds only): the
//! account's armed chaos moves onto this entry's session, and back to the
//! account if the entry then fails.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use cimmeria_base_session::base::lab_chaos::Promotion;

use super::super::super::ConnectedClientState;
use super::super::cell_dispatch;
use super::still_owner;

/// Promotes `account_id`'s arm onto the session at `addr`, tagged with
/// `entity_id`, only while that session is still the one this world entry
/// latched (`owner`). Under the `connected` lock, so a teardown or a
/// relaunch takeover cannot slip in between the check and the promotion;
/// the registry's own locks are taken inside it, the order
/// `chaos_wants_hold` already uses. A replaced session gets nothing and the
/// arm stays for the account's next world entry.
pub(super) fn promote_if_owner(
    connected: &Mutex<HashMap<SocketAddr, ConnectedClientState>>,
    addr: SocketAddr,
    owner: &Arc<AtomicBool>,
    account_id: u32,
    entity_id: u32,
) -> Option<Promotion> {
    let clients = connected.lock().ok()?;
    if !still_owner(&clients, addr, owner) {
        return None;
    }
    cell_dispatch::promote_armed(account_id, addr, entity_id)
}

/// Puts the arm `promotion` took back on its account, for a world entry that
/// failed or was abandoned after promoting.
pub(super) fn rearm(promotion: Promotion) {
    cell_dispatch::rearm_promotion(promotion);
}
