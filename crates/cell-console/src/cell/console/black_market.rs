//! GM Black Market console commands (BM-07): `.bm_seed`, `.bm_expire` and
//! `.bm_list`, so a tester can fill the auction house and trigger a
//! settlement without a second player or a 12-hour wait.
//!
//! The cell parses and range-checks; the base does the rest
//! (`base-methods` `methods/black_market/gm/`), because every auction write is a
//! base transaction. None has a legacy counterpart; the wording is plain
//! English (the owner's preference for GM commands).
//!
//! - `.bm_seed [count]` lists `count` auctions (default 8, at most 60) from
//!   the system seller, cycling through the UAT set, and names their ids.
//! - `.bm_expire <auctionId>` makes one active auction due now; the base
//!   settles it at once by the sweep's own path and says how it went (sold,
//!   returned to the seller, or a system listing that simply ends).
//! - `.bm_list` shows the newest active auctions with their ids, which the
//!   client's window never shows.

use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::{BlackMarketCellToBase, BmGmActor, CellToBaseMsg};
use crate::cell::space_manager::SpaceManager;

/// How many listings a bare `.bm_seed` adds: one of each UAT listing.
pub(crate) const DEFAULT_SEED_COUNT: u8 = 8;
/// The most one `.bm_seed` may add (the base checks it again).
pub(crate) const MAX_SEED_COUNT: u8 = 60;

pub(crate) const SEED_USAGE: &str =
    ".bm_seed: the count must be a number from 1 to 60. Usage: .bm_seed [count]";
pub(crate) const EXPIRE_USAGE: &str =
    ".bm_expire: name the auction to expire. Usage: .bm_expire <auctionId> (ids: .bm_list)";

/// Why a GM Black Market command was refused on the cell: a stable
/// `reason` and the GM's line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Refusal {
    pub reason: &'static str,
    pub line: String,
}

/// Parse `.bm_seed [count]`.
pub(crate) fn parse_seed(args: &[&str]) -> Result<u8, Refusal> {
    match args.first() {
        None => Ok(DEFAULT_SEED_COUNT),
        Some(w) => w
            .parse::<u8>()
            .ok()
            .filter(|n| (1..=MAX_SEED_COUNT).contains(n))
            .ok_or(Refusal {
                reason: "invalid_count",
                line: SEED_USAGE.to_string(),
            }),
    }
}

/// Parse `.bm_expire <auctionId>`.
pub(crate) fn parse_expire(args: &[&str]) -> Result<i32, Refusal> {
    args.first()
        .and_then(|w| w.trim_start_matches('#').parse::<i32>().ok())
        .filter(|id| *id > 0)
        .ok_or(Refusal {
            reason: "no_auction_id",
            line: EXPIRE_USAGE.to_string(),
        })
}

/// The GM as the base needs them, or `None` for an entity with no
/// character (never a GM who passed the console gate in world).
fn actor(caller_id: u32, space_mgr: &SpaceManager) -> Option<BmGmActor> {
    let id = space_mgr.player_identity(caller_id);
    Some(BmGmActor {
        entity_id: caller_id,
        player_id: id.player_id?,
        account_id: id.account_id,
    })
}

/// `.bm_seed [count]`.
pub(super) async fn seed(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let count = match parse_seed(args) {
        Ok(n) => n,
        Err(r) => return refuse(caller_id, "bm_seed", r, tx, space_mgr).await,
    };
    let Some(actor) = actor(caller_id, space_mgr) else {
        return refuse(caller_id, "bm_seed", no_character("bm_seed"), tx, space_mgr).await;
    };
    let msg = BlackMarketCellToBase::GmSeed { actor, count };
    forward(caller_id, "bm_seed", msg, tx, space_mgr).await;
}

/// `.bm_expire <auctionId>`.
pub(super) async fn expire(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let sequence_id = match parse_expire(args) {
        Ok(id) => id,
        Err(r) => return refuse(caller_id, "bm_expire", r, tx, space_mgr).await,
    };
    let Some(actor) = actor(caller_id, space_mgr) else {
        let r = no_character("bm_expire");
        return refuse(caller_id, "bm_expire", r, tx, space_mgr).await;
    };
    let msg = BlackMarketCellToBase::GmExpire { actor, sequence_id };
    forward(caller_id, "bm_expire", msg, tx, space_mgr).await;
}

/// `.bm_list`.
pub(super) async fn list(
    caller_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(actor) = actor(caller_id, space_mgr) else {
        return refuse(caller_id, "bm_list", no_character("bm_list"), tx, space_mgr).await;
    };
    let msg = BlackMarketCellToBase::GmList { actor };
    forward(caller_id, "bm_list", msg, tx, space_mgr).await;
}

fn no_character(cmd: &str) -> Refusal {
    Refusal {
        reason: "no_player_id",
        line: format!(".{cmd}: you have no character loaded."),
    }
}

/// Hand a GM Black Market command to the base, which answers the GM.
async fn forward(
    caller_id: u32,
    cmd: &'static str,
    msg: BlackMarketCellToBase,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    if tx.send(CellToBaseMsg::BlackMarket(msg)).await.is_err() {
        let gm = space_mgr.player_identity(caller_id);
        tracing::warn!(
            event = "bm.gm_rejected",
            account_id = gm.account_id,
            player_id = gm.player_id,
            entity_id = caller_id,
            command = cmd,
            reason = "base_channel_closed",
            "GM Black Market command dropped: the base channel is closed"
        );
    }
}

/// Log a refused GM Black Market command (`bm.gm_rejected`) and tell the GM
/// why.
async fn refuse(
    caller_id: u32,
    cmd: &'static str,
    r: Refusal,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let gm = space_mgr.player_identity(caller_id);
    tracing::warn!(
        event = "bm.gm_rejected",
        account_id = gm.account_id,
        player_id = gm.player_id,
        entity_id = caller_id,
        command = cmd,
        reason = r.reason,
        "GM Black Market command refused: nothing was sent to the base"
    );
    send_gm_feedback(caller_id, &r.line, tx).await;
}
