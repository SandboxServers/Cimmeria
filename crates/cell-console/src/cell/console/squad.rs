//! Squad console commands (organizations campaign ORG-04): `.squad_invite
//! <name>`, `.squad_join <name>` and `.squad_info [name]`, so one tester
//! can build and inspect a squad with a sentinel character.
//!
//! The work, the feedback lines and the `org.gm_action` audit row are
//! `cimmeria_cell_methods::cell::cell_methods::organization::squad::{gm_invite,
//! gm_join, gm_info}`, beside the squad handlers whose checks and fanout
//! they reuse. The console only routes the parsed line: the GM gate and the
//! argument count are the framework's (`dispatch`).

use tokio::sync::mpsc;

use cimmeria_cell_methods::cell::cell_methods::organization::squad;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Route one `.squad_*` command. `args` has passed the registry's count.
pub(super) async fn dispatch(
    name: &str,
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    match name {
        "squad_invite" => squad::gm_invite(caller_id, args[0], tx, space_mgr).await,
        "squad_join" => squad::gm_join(caller_id, args[0], tx, space_mgr).await,
        _ => squad::gm_info(caller_id, args.first().copied(), tx, space_mgr).await,
    }
}
