//! What the router did with an allowlisted call, and the
//! `client.ability.press_dropped` row for a router refusal (rows 6 to 10).

use serde_json::json;

use super::super::layout::RoutePre;
use super::super::{Out, TARGET_DROPPED};
use super::{opt, site, DropReason, PendingSend};

/// What the router did with an allowlisted call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RouteOutcome {
    /// A `start*Message` ran: the method went on the wire.
    Sent,
    /// The router returned first.
    Dropped(DropReason, &'static str),
}

/// Classify a router call. `reached_start` is observed; `pre` is what
/// memory showed before the call.
pub(crate) fn route_outcome(reached_start: bool, pre: RoutePre) -> RouteOutcome {
    if reached_start {
        return RouteOutcome::Sent;
    }
    let (reason, at) = if pre.has_connection == Some(false) {
        (DropReason::NotConnected, site::NO_CONNECTION)
    } else if pre.connected == Some(false) {
        (DropReason::NotConnected, site::NOT_CONNECTED)
    } else if pre.local_player_found == Some(false) {
        (DropReason::NotConnected, site::NO_LOCAL_PLAYER)
    } else {
        (DropReason::ClassMismatch, site::CLASS)
    };
    RouteOutcome::Dropped(reason, at)
}

/// `client.ability.press_dropped` for a router refusal. `pending` is the
/// press it settles, if one was waiting; a send with no press (a GM slash
/// command, the respec button) reports `press_id: null`.
pub(crate) fn route_dropped(
    method: &'static str,
    ability_id: Option<i32>,
    pending: Option<PendingSend>,
    reason: DropReason,
    at: &'static str,
) -> Out {
    let mut f = vec![
        ("press_id", opt(pending.map(|p| p.press_id))),
        ("source", opt(pending.map(|p| p.source.as_str()))),
        ("method", json!(method)),
        ("ability_id", opt(ability_id)),
        ("reason", json!(reason.as_str())),
        ("drop_site", json!(at)),
    ];
    if reason == DropReason::ClassMismatch {
        // Row 9 (no type mapping) and row 10 (class chain) both return
        // before a start*Message, and only game code tells them apart.
        f.push(("route_rows", json!("9|10")));
    }
    Out {
        target: TARGET_DROPPED,
        level: "info",
        key: format!("{TARGET_DROPPED}:{}", reason.as_str()),
        fields: f,
    }
}
