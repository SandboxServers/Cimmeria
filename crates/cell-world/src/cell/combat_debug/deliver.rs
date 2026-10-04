//! Routing, rate limiting, the `abilities.debug` rows and the sends.
//!
//! [`flush`] runs where a cast's scope closes (the zero-warmup launch, the
//! warmup fire, each pulse, the ground cast). It finalizes every open record
//! that does not belong to the scope still open, and for each:
//!
//! 1. **Who wants it.** A watcher gets the record when
//!    - it cast the record or was touched by it, and the record's toggle is
//!      on: combat or verbose debug for a hostile cast, heal debug for a
//!      beneficial one, any of the three for a lone pulse; or
//!    - it cast the record or was touched by it, and the ability is in its
//!      `debugAbilityList`; or
//!    - the caster is a mob it turned mob debug on for, for that ability or
//!      for all of them.
//!
//!    A verbose watcher gets the verbose lines too. The lines go to the
//!    watcher's `debugAbilityTargetID` when that is a player still here,
//!    else to the watcher.
//! 2. **The budget.** Each recipient gets at most [`LINES_PER_WINDOW`] lines
//!    per [`WINDOW`]. A line over budget is not sent, but its row is still
//!    written (`delivery = suppressed`), and the next line the recipient is
//!    sent after the window ends is preceded by `[CD] +N lines suppressed`.
//!    A flush with no records still sends an owed notice, so a storm that
//!    stops is reported at the next cast or pulse in the cell.
//! 3. **The row and the line.** Every line writes one `abilities.debug`
//!    DEBUG `combat_debug_line` row whose `text` is the exact text. A line
//!    within budget logs its row after the send: `delivery =
//!    queued_to_base` once the cell-to-base queue took it (queued, not yet
//!    delivered, as AB-T4's `wire_sent`), `send_failed` when the queue was
//!    closed.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;

use super::format::{format_record, split_line, MAX_LINE_UNITS};
use super::record::{CastDebug, CastKind};
use super::settings::DebugSettings;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Lines one recipient is sent per [`WINDOW`].
pub const LINES_PER_WINDOW: u32 = 20;

/// The rate-limit window.
pub const WINDOW: Duration = Duration::from_secs(1);

/// One recipient's budget in the current window.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RateWindow {
    start: Instant,
    sent: u32,
    suppressed: u32,
}

/// Which line a row is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Simple,
    Verbose,
    SuppressedNotice,
}

impl LineKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Verbose => "verbose",
            Self::SuppressedNotice => "suppressed_notice",
        }
    }
}

/// The cast a line is about, for its row (`None` on a suppressed notice).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCast {
    pub caster_id: u32,
    /// Snapshotted when the record opened.
    pub caster: PlayerIdentity,
    pub cast_id: Option<i32>,
    pub ability_id: i32,
}

impl LineCast {
    fn of(rec: &CastDebug) -> Self {
        Self {
            caster_id: rec.caster_id,
            caster: rec.caster,
            cast_id: rec.cast_id,
            ability_id: rec.ability_id,
        }
    }
}

/// One line to send, with what its row needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outgoing {
    pub recipient: u32,
    pub text: String,
    pub kind: LineKind,
    pub cast: Option<LineCast>,
}

/// Finalize, route, format and send every finished record (module docs),
/// logging each line's row once its send is known.
pub async fn flush(tx: &mpsc::Sender<CellToBaseMsg>, mgr: &mut SpaceManager) {
    let out = prepare(mgr, Instant::now());
    for line in out {
        let queued = send_feedback_line(tx, mgr, line.recipient, &line.text).await;
        let delivery = if queued {
            "queued_to_base"
        } else {
            "send_failed"
        };
        log_line(mgr, &line, delivery);
    }
}

/// The synchronous half of [`flush`]: everything but the sends and the rows
/// of the lines to send, at `now`. Suppressed lines log their rows here.
pub fn prepare(mgr: &mut SpaceManager, now: Instant) -> Vec<Outgoing> {
    let debug = &mgr.combat_debug;
    if debug.open.is_empty() && debug.windows.is_empty() {
        return Vec::new();
    }
    let current = mgr.current_cast_id();
    let open = std::mem::take(&mut mgr.combat_debug.open);
    let (done, keep): (Vec<_>, Vec<_>) = open
        .into_iter()
        .partition(|r| current.is_none() || r.cast_id != current);
    mgr.combat_debug.open = keep;

    drop_stale_watchers(mgr);
    let mut out = Vec::new();
    for rec in &done {
        let wanted = recipients(mgr, rec);
        if wanted.is_empty() {
            continue;
        }
        let lines = format_record(mgr, rec);
        let cast = Some(LineCast::of(rec));
        for (recipient, verbose) in wanted {
            let simple = lines.simple.iter().map(|l| (l, LineKind::Simple));
            let detail = lines
                .verbose
                .iter()
                .filter(|_| verbose)
                .map(|l| (l, LineKind::Verbose));
            for (line, kind) in simple.chain(detail) {
                for text in split_line(line, MAX_LINE_UNITS) {
                    let line = Outgoing {
                        recipient,
                        text,
                        kind,
                        cast,
                    };
                    admit(mgr, now, line, &mut out);
                }
            }
        }
    }
    settle_windows(mgr, now, &mut out);
    out
}

/// Remove watchers whose entity is gone, is no longer a player, or now
/// plays another character. The row names the watcher by the identity
/// snapshotted when it turned debugging on.
fn drop_stale_watchers(mgr: &mut SpaceManager) {
    let stale: Vec<u32> = mgr
        .combat_debug
        .watchers
        .iter()
        .filter(|(&id, s)| {
            !mgr.get_entity(id)
                .is_some_and(|e| e.is_player && e.player_id == s.player_id)
        })
        .map(|(&id, _)| id)
        .collect();
    for id in stale {
        let s = mgr.combat_debug.watchers.remove(&id);
        tracing::debug!(
            target: "abilities.debug",
            event = "combat_debug_watcher_dropped",
            reason = "watcher_left",
            entity_id = id,
            account_id = s.as_ref().and_then(|s| s.account_id),
            player_id = s.as_ref().and_then(|s| s.player_id),
            "combat debug: watcher left; its toggles are cleared"
        );
    }
}

/// Whether watcher `w` with `s` wants `rec` (module docs, step 1).
fn wants(rec: &CastDebug, kind: CastKind, w: u32, s: &DebugSettings) -> bool {
    let involved = rec.involves(w);
    let toggle = match kind {
        CastKind::Hostile => s.combat || s.verbose,
        CastKind::Beneficial => s.heal,
        CastKind::Pulse => s.combat || s.verbose || s.heal,
    };
    let listed = s.abilities.contains(&rec.ability_id);
    let mob = s
        .mobs
        .iter()
        .any(|&(m, a)| m == rec.caster_id && (a == 0 || a == rec.ability_id));
    (involved && (toggle || listed)) || mob
}

/// `(recipient, verbose)` for `rec`, one entry per recipient.
fn recipients(mgr: &SpaceManager, rec: &CastDebug) -> Vec<(u32, bool)> {
    let kind = rec.kind();
    let mut out: Vec<(u32, bool)> = Vec::new();
    let mut ids: Vec<_> = mgr.combat_debug.watchers.keys().copied().collect();
    // Deterministic order: the map's is not.
    ids.sort_unstable();
    for w in ids {
        let s = &mgr.combat_debug.watchers[&w];
        if !wants(rec, kind, w, s) {
            continue;
        }
        let to = s
            .target
            .filter(|&t| mgr.get_entity(t).is_some_and(|e| e.is_player))
            .unwrap_or(w);
        match out.iter_mut().find(|(r, _)| *r == to) {
            Some(entry) => entry.1 |= s.verbose,
            None => out.push((to, s.verbose)),
        }
    }
    out
}

fn notice(recipient: u32, owed: u32) -> Outgoing {
    Outgoing {
        recipient,
        text: suppressed_notice(owed),
        kind: LineKind::SuppressedNotice,
        cast: None,
    }
}

/// Spend one line of the recipient's budget on `line` (module docs, step 2).
fn admit(mgr: &mut SpaceManager, now: Instant, line: Outgoing, out: &mut Vec<Outgoing>) {
    let recipient = line.recipient;
    let mut w = *mgr
        .combat_debug
        .windows
        .entry(recipient)
        .or_insert(RateWindow {
            start: now,
            sent: 0,
            suppressed: 0,
        });
    if now.saturating_duration_since(w.start) >= WINDOW {
        let owed = w.suppressed;
        w = RateWindow {
            start: now,
            sent: 0,
            suppressed: 0,
        };
        if owed > 0 {
            w.sent += 1;
            out.push(notice(recipient, owed));
        }
    }
    if w.sent < LINES_PER_WINDOW {
        w.sent += 1;
        out.push(line);
    } else {
        w.suppressed += 1;
        log_line(mgr, &line, "suppressed");
    }
    mgr.combat_debug.windows.insert(recipient, w);
}

/// After the records: send the notices owed by windows that have ended,
/// and forget windows that are over with nothing owed.
fn settle_windows(mgr: &mut SpaceManager, now: Instant, out: &mut Vec<Outgoing>) {
    let mut ids: Vec<u32> = mgr.combat_debug.windows.keys().copied().collect();
    ids.sort_unstable();
    for id in ids {
        let w = mgr.combat_debug.windows[&id];
        if now.saturating_duration_since(w.start) < WINDOW {
            continue;
        }
        mgr.combat_debug.windows.remove(&id);
        if w.suppressed == 0 {
            continue;
        }
        out.push(notice(id, w.suppressed));
        // The notice opens a new window, so a storm still under way keeps
        // its budget.
        mgr.combat_debug.windows.insert(
            id,
            RateWindow {
                start: now,
                sent: 1,
                suppressed: 0,
            },
        );
    }
}

fn suppressed_notice(n: u32) -> String {
    format!(
        "[CD] +{n} lines suppressed (over {LINES_PER_WINDOW} per second); \
         the abilities.debug rows have them all"
    )
}

/// The `abilities.debug` row for one line.
fn log_line(mgr: &SpaceManager, line: &Outgoing, delivery: &'static str) {
    let who = mgr.player_identity(line.recipient);
    let cast = line.cast;
    tracing::debug!(
        target: "abilities.debug",
        event = "combat_debug_line",
        stage = "debug",
        account_id = who.account_id,
        player_id = who.player_id,
        entity_id = line.recipient,
        caster_id = cast.map(|c| c.caster_id),
        caster_account_id = cast.and_then(|c| c.caster.account_id),
        caster_player_id = cast.and_then(|c| c.caster.player_id),
        cast_id = cast.and_then(|c| c.cast_id),
        ability_id = cast.map(|c| c.ability_id),
        line_kind = line.kind.as_str(),
        delivery,
        text = line.text.as_str(),
        "combat debug line"
    );
}

/// Send one `onPlayerCommunication("SYSTEM", 0, CHAN_FEEDBACK, text)` to
/// `recipient`'s own client. Returns whether the cell-to-base queue took
/// it; a closed channel is logged, not dropped.
pub async fn send_feedback_line(
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &SpaceManager,
    recipient: u32,
    text: &str,
) -> bool {
    let args = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
    let msg = CellToBaseMsg::EntityMethodCall {
        entity_id: recipient,
        method_index: ON_PLAYER_COMMUNICATION,
        args,
    };
    if let Err(e) = tx.send(msg).await {
        let who = mgr.player_identity(recipient);
        tracing::warn!(
            target: "abilities.debug",
            event = "combat_debug_send_failed",
            account_id = who.account_id,
            player_id = who.player_id,
            entity_id = recipient,
            error = %e,
            "combat debug line not queued: base channel closed"
        );
        return false;
    }
    true
}
