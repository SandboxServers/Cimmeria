//! The one row for a client message the inbound bundle walk drops.
//!
//! Every drop in the walk (`encrypted/mod.rs`, `account_arms.rs`,
//! `cell_arms.rs`) goes through [`log_drop`] with a [`DropReason`] from a
//! closed set, so a SigNoz query on `event = "inbound_message_dropped"`
//! sees every way a client message can vanish. The walk is a hot path, so
//! the rows are throttled per session and reason (Pattern D).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_wire::names;
use tracing::Level;

/// Why the bundle walk dropped a client message. The set is closed: a new
/// drop gets a new variant here, never a free-text reason at the call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum DropReason {
    /// The message's length prefix or fixed width runs past the bundle;
    /// the walk stops and the rest of the bundle is lost.
    BundleTruncated,
    /// A system message id the walk has no arm for.
    UnhandledMsgId,
    /// An Account base method id with no handler at character select.
    UnhandledAccountMethod,
    /// A movement update while the session has no player entity.
    NoPlayerEntity,
    /// A cell method before the session has a player entity.
    BeforeWorldEntry,
    /// A cell method other than `mapLoaded` while world entry waits for it.
    MapLoadedPending,
    /// A cell method payload shorter than its 4-byte entityId prefix.
    MalformedArgs,
    /// A `0xBD` sub-slot cell method with no sub-index byte.
    SubslotMissingIndex,
}

impl DropReason {
    /// The `reason` field value.
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::BundleTruncated => "bundle_truncated",
            Self::UnhandledMsgId => "unhandled_msg_id",
            Self::UnhandledAccountMethod => "unhandled_account_method",
            Self::NoPlayerEntity => "no_player_entity",
            Self::BeforeWorldEntry => "before_world_entry",
            Self::MapLoadedPending => "map_loaded_pending",
            Self::MalformedArgs => "malformed_args",
            Self::SubslotMissingIndex => "subslot_missing_index",
        }
    }

    /// WARN for a drop a player can feel (a malformed or unknown message);
    /// DEBUG for the normal loading-time races, which a healthy login hits.
    pub(super) fn level(self) -> Level {
        match self {
            Self::NoPlayerEntity | Self::BeforeWorldEntry | Self::MapLoadedPending => Level::DEBUG,
            Self::BundleTruncated
            | Self::UnhandledMsgId
            | Self::UnhandledAccountMethod
            | Self::MalformedArgs
            | Self::SubslotMissingIndex => Level::WARN,
        }
    }
}

/// The message a drop row describes.
#[derive(Debug, Clone, Copy)]
pub(super) struct DroppedMsg<'a> {
    pub msg_id: u8,
    /// The clientIndex the walk read `msg_id` against, for `entity_type` and
    /// `method_name`.
    pub class_id: u8,
    pub payload: &'a [u8],
    /// The cell or base method index, when the walk decoded one.
    pub method_index: Option<u16>,
    /// The bundle offset of the message and the bundle length, for a
    /// truncation.
    pub offset: Option<usize>,
    pub body_len: Option<usize>,
}

/// How long one row covers per session and reason. Drops inside the window
/// are counted into the next row's `suppressed`.
const DROP_WINDOW: Duration = Duration::from_secs(5);

/// Above this many throttle keys, expired ones are pruned before a new key
/// is added, so a long uptime with many sessions cannot grow the map without
/// bound. A pruned key's swallowed count is lost; it was older than a window.
const MAX_KEYS: usize = 1024;

#[derive(Debug)]
struct DropWindow {
    opened: Instant,
    suppressed: u64,
}

type Throttle = HashMap<(DropReason, SocketAddr), DropWindow>;

static THROTTLE: LazyLock<Mutex<Throttle>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Log one dropped client message, throttled per `(reason, addr)`.
///
/// `who` runs only when a row is written, so a burst of drops pays for one
/// identity lookup per window. Callers holding the `connected` lock must
/// resolve the identity before calling, not inside `who`.
pub(super) fn log_drop(
    reason: DropReason,
    addr: SocketAddr,
    msg: DroppedMsg<'_>,
    who: impl FnOnce() -> PlayerIdentity,
) {
    let mut throttle = THROTTLE.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(suppressed) = admit(&mut throttle, reason, addr, Instant::now()) else {
        return;
    };
    drop(throttle);
    emit(reason, addr, msg, suppressed, who());
}

/// The throttle decision at `now`: `Some(suppressed)` to write a row, `None`
/// to count it.
fn admit(
    throttle: &mut Throttle,
    reason: DropReason,
    addr: SocketAddr,
    now: Instant,
) -> Option<u64> {
    if let Some(window) = throttle.get_mut(&(reason, addr)) {
        if now.saturating_duration_since(window.opened) < DROP_WINDOW {
            window.suppressed += 1;
            return None;
        }
        window.opened = now;
        return Some(std::mem::take(&mut window.suppressed));
    }
    if throttle.len() >= MAX_KEYS {
        throttle.retain(|_, w| now.saturating_duration_since(w.opened) < DROP_WINDOW);
    }
    throttle.insert(
        (reason, addr),
        DropWindow {
            opened: now,
            suppressed: 0,
        },
    );
    Some(0)
}

/// Write the row. `tracing` levels are static per callsite, hence the two
/// arms with one field list.
fn emit(
    reason: DropReason,
    addr: SocketAddr,
    msg: DroppedMsg<'_>,
    suppressed: u64,
    who: PlayerIdentity,
) {
    let method_name = match msg.method_index {
        // A decoded index names the SGWPlayer cell method; otherwise the
        // walk's own reading of the wire byte.
        Some(index) if (0x80..=0xBF).contains(&msg.msg_id) => names::player_cell_method(index),
        _ => names::inbound_method(msg.class_id, msg.msg_id, msg.payload),
    };
    macro_rules! drop_row {
        ($level:expr) => {
            tracing::event!(
                $level,
                %addr,
                event = "inbound_message_dropped",
                reason = reason.as_str(),
                account_id = who.account_id,
                account_name = who.account_name,
                player_id = who.player_id,
                player_name = who.player_name,
                msg_id = msg.msg_id,
                msg_name = names::server_msg_name(msg.msg_id),
                method_index = msg.method_index,
                method_name,
                entity_type = names::class_name(msg.class_id),
                payload_len = msg.payload.len(),
                offset = msg.offset,
                body_len = msg.body_len,
                suppressed,
                "Inbound client message dropped"
            )
        };
    }
    if reason.level() == Level::WARN {
        drop_row!(Level::WARN);
    } else {
        drop_row!(Level::DEBUG);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;

    fn msg() -> DroppedMsg<'static> {
        DroppedMsg {
            msg_id: 0x0A,
            class_id: names::ACCOUNT_CLASS_ID,
            payload: &[],
            method_index: None,
            offset: None,
            body_len: None,
        }
    }

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], port))
    }

    /// Pattern D burst: the first drop writes, the rest of its window only
    /// count, and the first row after the window carries the count.
    #[test]
    fn drop_burst_writes_one_row_per_window() {
        let mut throttle = Throttle::new();
        let start = Instant::now();
        let a = addr(1);
        assert_eq!(
            admit(&mut throttle, DropReason::UnhandledMsgId, a, start),
            Some(0)
        );
        for _ in 0..9 {
            assert_eq!(
                admit(&mut throttle, DropReason::UnhandledMsgId, a, start),
                None
            );
        }
        assert_eq!(
            admit(
                &mut throttle,
                DropReason::UnhandledMsgId,
                a,
                start + DROP_WINDOW
            ),
            Some(9)
        );
    }

    /// Pattern D independence: another session, or another reason from the
    /// same session, writes its first row inside the first key's window.
    #[test]
    fn drop_throttle_keys_are_independent() {
        let mut throttle = Throttle::new();
        let now = Instant::now();
        assert!(admit(&mut throttle, DropReason::UnhandledMsgId, addr(1), now).is_some());
        assert!(admit(&mut throttle, DropReason::UnhandledMsgId, addr(2), now).is_some());
        assert!(admit(&mut throttle, DropReason::BundleTruncated, addr(1), now).is_some());
    }

    /// A full map prunes expired keys before it grows.
    #[test]
    fn drop_throttle_prunes_expired_keys() {
        let mut throttle = Throttle::new();
        let start = Instant::now();
        for port in 0..MAX_KEYS as u16 {
            admit(&mut throttle, DropReason::UnhandledMsgId, addr(port), start);
        }
        let later = start + DROP_WINDOW;
        admit(
            &mut throttle,
            DropReason::UnhandledMsgId,
            addr(u16::MAX),
            later,
        );
        assert_eq!(throttle.len(), 1);
    }

    /// The `suppressed` count rides the written row.
    #[test]
    fn drop_row_carries_suppressed() {
        let capture = LogCapture::install();
        emit(
            DropReason::UnhandledMsgId,
            addr(3),
            msg(),
            4,
            PlayerIdentity::UNKNOWN,
        );
        let row = capture
            .find_event(
                Level::WARN,
                "Inbound client message dropped",
                "unhandled_msg_id",
            )
            .expect("drop row");
        assert!(row.has_field("suppressed", "4"));
    }
}
