//! Why an atomic swap aborted, and what each player is told.
//!
//! Each abort maps to a per-side `ETradeResults` code
//! ([`trade_abort_to_results_codes`]), a low-cardinality metric / log
//! label ([`trade_abort_outcome_label`]), and the feedback line each
//! player gets ([`refusal_lines`]). The line is needed because the
//! shipped client's trade window only reacts to `Completed` and
//! `Cancelled` (`Trade.lua` `TradeMod.onTradeResult`): the four
//! space / cash codes reach it and show nothing.

use cimmeria_entity::inventory::INV_CRAFTING;
use cimmeria_entity::trade::{
    ETRADERESULTS_CANCELLED, ETRADERESULTS_NO_LOCAL_CASH, ETRADERESULTS_NO_LOCAL_SPACE,
    ETRADERESULTS_NO_REMOTE_CASH, ETRADERESULTS_NO_REMOTE_SPACE,
};

/// Reason the atomic swap aborted. Mapped to per-side asymmetric
/// `ETradeResults` codes via [`trade_abort_to_results_codes`]:
/// `InsufficientCash {p1|p2}` → `NoLocalCash`/`NoRemoteCash`,
/// `NotEnoughSlots {recipient_player_id}` →
/// `NoLocalSpace`/`NoRemoteSpace`, with the remaining catch-all
/// variants staying on the generic Cancelled code — those are internal
/// faults or server-authority rejections the client UI has no dedicated
/// string for.
#[derive(Debug)]
pub(in super::super) enum TradeAbort {
    DbError(sqlx::Error),
    PlayerMissing {
        which: &'static str,
        player_id: i32,
    },
    InsufficientCash {
        which: &'static str,
        player_id: i32,
        has: i32,
        wants: i32,
    },
    ItemMissing {
        which: &'static str,
        player_id: i32,
        item_id: i32,
    },
    /// The recipient's destination bag cannot hold what they receive.
    NotEnoughSlots {
        recipient_player_id: i32,
        container_id: i32,
        needed: usize,
        free: usize,
    },
    BoundItemOffered {
        which: &'static str,
        player_id: i32,
        item_id: i32,
    },
    DuplicateInstance {
        item_id: i32,
    },
    /// Item lives in a container that's not on the tradeable-container
    /// whitelist (anything other than the backpack and the crafting
    /// bag). Covers the dupe-strip-equipped-gear, mission-item-share,
    /// banker-gate-bypass, and bandolier-ammo-sync exploits — all the
    /// same shape: the server must independently verify *which*
    /// containers can leak items, not just whether the row is bound or
    /// in the buyback bag.
    IneligibleContainer {
        which: &'static str,
        player_id: i32,
        item_id: i32,
        container_id: i32,
    },
    /// The item's type has no `resources.items` row, so there is no
    /// `container_sets` to place it by. Refused rather than guessed at.
    NoDestination {
        which: &'static str,
        player_id: i32,
        item_id: i32,
        type_id: i32,
    },
}

impl std::fmt::Display for TradeAbort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TradeAbort::DbError(e) => write!(f, "db error: {e}"),
            TradeAbort::PlayerMissing { which, player_id } => {
                write!(f, "{which} player {player_id} missing")
            }
            TradeAbort::InsufficientCash {
                which,
                player_id,
                has,
                wants,
            } => write!(
                f,
                "{which} player {player_id} has {has} naquadah, offering {wants}"
            ),
            TradeAbort::ItemMissing {
                which,
                player_id,
                item_id,
            } => write!(
                f,
                "{which} player {player_id} doesn't own item instance {item_id}"
            ),
            TradeAbort::NotEnoughSlots {
                recipient_player_id,
                container_id,
                needed,
                free,
            } => write!(
                f,
                "recipient {recipient_player_id} needs {needed} free slots in \
                 container {container_id}, has {free}"
            ),
            TradeAbort::BoundItemOffered {
                which,
                player_id,
                item_id,
            } => write!(f, "{which} player {player_id} offered bound item {item_id}"),
            TradeAbort::DuplicateInstance { item_id } => {
                write!(f, "item instance {item_id} listed twice in proposal")
            }
            TradeAbort::IneligibleContainer {
                which,
                player_id,
                item_id,
                container_id,
            } => write!(
                f,
                "{which} player {player_id} offered item {item_id} from \
                 non-tradeable container {container_id} \
                 (whitelist: backpack and crafting bag)"
            ),
            TradeAbort::NoDestination {
                which,
                player_id,
                item_id,
                type_id,
            } => write!(
                f,
                "{which} player {player_id} offered item {item_id} of type \
                 {type_id}, which has no resources.items row to place it by"
            ),
        }
    }
}

impl From<sqlx::Error> for TradeAbort {
    fn from(e: sqlx::Error) -> Self {
        TradeAbort::DbError(e)
    }
}

/// Low-cardinality outcome label for the `trade_swaps_total{outcome=...}`
/// counter and the `reason` of the refusal event. Enumerated values only —
/// no `player_id` / `entity_id` (rule 4 in instrumentation-discipline.md).
pub(super) fn trade_abort_outcome_label(reason: &TradeAbort) -> &'static str {
    match reason {
        TradeAbort::DbError(_) => "db_error",
        TradeAbort::PlayerMissing { .. } => "player_missing",
        TradeAbort::InsufficientCash { .. } => "insufficient_cash",
        TradeAbort::ItemMissing { .. } => "item_missing",
        TradeAbort::NotEnoughSlots { container_id, .. } if *container_id == INV_CRAFTING => {
            "crafting_bag_full"
        }
        TradeAbort::NotEnoughSlots { .. } => "insufficient_slots",
        TradeAbort::BoundItemOffered { .. } => "bound_item",
        TradeAbort::DuplicateInstance { .. } => "duplicate_instance",
        TradeAbort::IneligibleContainer { .. } => "ineligible_container",
        TradeAbort::NoDestination { .. } => "no_destination",
    }
}

/// Per-side ETradeResults code mapping for [`TradeAbort`].
///
/// Returns `(p1_code, p2_code)` in p1-then-p2 order. The asymmetric
/// `NoLocal*` / `NoRemote*` codes mirror Python `Trade.py:237-263`:
/// the failing player sees `NoLocal*`, the other sees `NoRemote*`.
///
/// `InsufficientCash` carries a `which: "p1"|"p2"` discriminant that
/// directly identifies the failing side. `NotEnoughSlots` carries
/// `recipient_player_id` — the side without room — which we resolve
/// against the caller-provided `p1_player_id` (recipient is either
/// p1 or p2 by construction in `swap::atomic_swap`).
///
/// Catch-all variants are internal faults or server-authority
/// validations the client UI has no dedicated string for — both sides
/// see Cancelled.
pub(super) fn trade_abort_to_results_codes(reason: &TradeAbort, p1_player_id: i32) -> (i32, i32) {
    match reason {
        TradeAbort::InsufficientCash { which: "p1", .. } => {
            (ETRADERESULTS_NO_LOCAL_CASH, ETRADERESULTS_NO_REMOTE_CASH)
        }
        TradeAbort::InsufficientCash { which: "p2", .. } => {
            (ETRADERESULTS_NO_REMOTE_CASH, ETRADERESULTS_NO_LOCAL_CASH)
        }
        TradeAbort::NotEnoughSlots {
            recipient_player_id,
            ..
        } if *recipient_player_id == p1_player_id => {
            (ETRADERESULTS_NO_LOCAL_SPACE, ETRADERESULTS_NO_REMOTE_SPACE)
        }
        TradeAbort::NotEnoughSlots { .. } => {
            (ETRADERESULTS_NO_REMOTE_SPACE, ETRADERESULTS_NO_LOCAL_SPACE)
        }
        _ => (ETRADERESULTS_CANCELLED, ETRADERESULTS_CANCELLED),
    }
}

pub(in super::super) const LOCAL_BACKPACK_FULL: &str =
    "Trade cancelled: your backpack does not have room for the items you would receive.";
pub(in super::super) const REMOTE_BACKPACK_FULL: &str =
    "Trade cancelled: your trade partner's backpack does not have room for your items.";
pub(in super::super) const LOCAL_CRAFTING_BAG_FULL: &str =
    "Trade cancelled: your crafting bag does not have room for the items you would receive.";
pub(in super::super) const REMOTE_CRAFTING_BAG_FULL: &str =
    "Trade cancelled: your trade partner's crafting bag does not have room for your items.";
pub(in super::super) const LOCAL_NO_CASH: &str =
    "Trade cancelled: you do not have the naquadah you offered.";
pub(in super::super) const REMOTE_NO_CASH: &str =
    "Trade cancelled: your trade partner does not have the naquadah they offered.";
pub(in super::super) const LOCAL_UNTRADEABLE_ITEM: &str =
    "Trade cancelled: one of the items you offered cannot be traded.";
pub(in super::super) const REMOTE_UNTRADEABLE_ITEM: &str =
    "Trade cancelled: one of the items your trade partner offered cannot be traded.";

/// The feedback line each side gets, `(p1, p2)`. `None` where the
/// client's own "Trade Cancelled" line already says all there is to say
/// (a server fault, or a stale / duplicated offer).
pub(super) fn refusal_lines(
    reason: &TradeAbort,
    p1_player_id: i32,
) -> (Option<&'static str>, Option<&'static str>) {
    let local_is_p1 = |which: &str| which == "p1";
    let pair = |p1_local: bool, local: &'static str, remote: &'static str| {
        if p1_local {
            (Some(local), Some(remote))
        } else {
            (Some(remote), Some(local))
        }
    };
    match reason {
        TradeAbort::NotEnoughSlots {
            recipient_player_id,
            container_id,
            ..
        } => {
            let (local, remote) = if *container_id == INV_CRAFTING {
                (LOCAL_CRAFTING_BAG_FULL, REMOTE_CRAFTING_BAG_FULL)
            } else {
                (LOCAL_BACKPACK_FULL, REMOTE_BACKPACK_FULL)
            };
            pair(*recipient_player_id == p1_player_id, local, remote)
        }
        TradeAbort::InsufficientCash { which, .. } => {
            pair(local_is_p1(which), LOCAL_NO_CASH, REMOTE_NO_CASH)
        }
        TradeAbort::BoundItemOffered { which, .. }
        | TradeAbort::IneligibleContainer { which, .. }
        | TradeAbort::NoDestination { which, .. } => pair(
            local_is_p1(which),
            LOCAL_UNTRADEABLE_ITEM,
            REMOTE_UNTRADEABLE_ITEM,
        ),
        TradeAbort::DbError(_)
        | TradeAbort::PlayerMissing { .. }
        | TradeAbort::ItemMissing { .. }
        | TradeAbort::DuplicateInstance { .. } => (None, None),
    }
}

/// The container a refusal names, for the refusal event (0 when none).
pub(super) fn refusal_container(reason: &TradeAbort) -> i32 {
    match reason {
        TradeAbort::NotEnoughSlots { container_id, .. }
        | TradeAbort::IneligibleContainer { container_id, .. } => *container_id,
        _ => 0,
    }
}
