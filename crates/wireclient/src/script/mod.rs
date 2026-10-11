//! A scripted session: every message the server sends is decoded, applied to
//! the [`EntityMirror`] and kept in an ordered log, and the script waits on
//! that log instead of on the socket (README F3, D-WC5, D-WC9, D-WC12).
//!
//! `tests/it/support::wait_for` throws away everything that does not match
//! its predicate, so two waits in a row can miss a message and nothing can
//! assert the order of several answers to one call. Here nothing is thrown
//! away: [`ScriptSession::pump`] appends to the log, and the waits move a
//! cursor through it. A wait that times out fails with the last 30 entries.
//!
//! While the script waits, the session keeps sending what an idle client
//! sends: an unreliable `AUTHENTICATE` every [`ScriptSession::KEEPALIVE`],
//! which carries the piggyback ACKs the session owes, and one
//! `requestEntityUpdate` (`0x07`) per entity the server introduces.

use std::time::{Duration, Instant};

use bytes::Bytes;
use cimmeria_mercury::channel_bundle::IDBASE_SGW_PLAYER;

use crate::bundle::{decode_bundle_strict, S2CMessage};
use crate::error::{Error, Result};
use crate::mirror::{EntityMirror, EntityQuery, MirrorError};
use crate::semantic::S2CEvent;
use crate::session::GameSession;

/// `tickSync` (0x0D): the server's 10 Hz heartbeat.
const MSG_TICK_SYNC: u8 = 0x0D;
/// `CREATE_ENTITY` (0x09): an entity enters the client's world.
const MSG_CREATE_ENTITY: u8 = 0x09;
/// The client skips `requestEntityUpdate` for ids at or above this: they are
/// client-only entities (`EntityManager::onEntityEnter`,
/// docs/reverse-engineering/findings/request-entity-update-cache-stamp.md §1.1).
const CLIENT_ONLY_ID_FLOOR: u32 = 0x4000_0000;
/// How many log entries a timeout prints (D-WC9).
const TRAIL_ON_TIMEOUT: usize = 30;
/// A trail line cuts an event's debug text here, so one `onUpdateItem` with
/// a full bag cannot bury the other lines.
const TRAIL_EVENT_CHARS: usize = 160;

/// One server message as the script saw it.
#[derive(Debug, Clone)]
pub struct Observed {
    /// When the script took its bundle off the inbox, from the start of the
    /// script session. World-entry bundles all read about zero, and a bundle
    /// that queued while the script was not pumping reads the later time.
    pub at: Duration,
    /// The message as framed on the wire.
    pub msg: S2CMessage,
    /// Its semantic event, when the mirror decoded one.
    pub event: Option<S2CEvent>,
}

/// An in-world [`GameSession`] driven by a test script.
pub struct ScriptSession {
    /// The connection itself; public so a script can build calls with its
    /// builders and read `player_entity_id`.
    pub session: GameSession,
    /// What the client has been told, as of the last logged message.
    pub mirror: EntityMirror,
    log: Vec<Observed>,
    /// The first log entry the next wait may match.
    cursor: usize,
    started: Instant,
    /// The last time anything was sent, reliable or not: both carry the ACKs
    /// the session owes, so either resets the keep-alive clock.
    last_send: Instant,
    /// `requestEntityUpdate` messages not sent yet, back to back.
    pending_updates: Vec<u8>,
}

impl ScriptSession {
    /// How often an idle script sends a keep-alive (D-WC12; `sparbot`'s
    /// `HEARTBEAT_EVERY`).
    pub const KEEPALIVE: Duration = Duration::from_millis(250);

    /// Feeds `session.entry_bundles` (WC-10) through the mirror into the log
    /// first, so a script can wait for a message world entry already received.
    /// The bundles stay in `session.entry_bundles` as well.
    pub fn new(session: GameSession) -> Result<Self> {
        let now = Instant::now();
        let mut mirror = EntityMirror::new();
        if let Some(id) = session.player_entity_id {
            mirror.set_own_id(id);
        }
        let entry = session.entry_bundles.clone();
        let mut script = Self {
            session,
            mirror,
            log: Vec::new(),
            cursor: 0,
            started: now,
            last_send: now,
            pending_updates: Vec::new(),
        };
        for bundle in &entry {
            script.ingest(bundle)?;
        }
        Ok(script)
    }

    /// Everything observed so far, in arrival order.
    pub fn log(&self) -> &[Observed] {
        &self.log
    }

    /// Concatenate `msgs` into one reliable bundle, as the client batches its
    /// queue. Queued `requestEntityUpdate`s go first: the client queued each
    /// when its entity entered, before the script decided to send `msgs`.
    pub async fn send(&mut self, msgs: &[Vec<u8>]) -> Result<()> {
        let body = take_outgoing(&mut self.pending_updates, msgs);
        if body.is_empty() {
            return Ok(());
        }
        if let Err(err) = self.session.send_bundle(&body, true).await {
            tracing::warn!(bytes = body.len(), error = %err, "script: send failed");
            return Err(err.into());
        }
        self.last_send = Instant::now();
        Ok(())
    }

    /// Receive for up to `max`: strict decode, mirror, log; keep-alive as due.
    /// Returns how many messages were added to the log.
    pub async fn pump(&mut self, max: Duration) -> Result<usize> {
        let deadline = Instant::now() + max;
        let mut added = 0;
        loop {
            added += self.pump_step(deadline).await?;
            if Instant::now() >= deadline {
                return Ok(added);
            }
        }
    }

    /// First logged entry at or after the cursor matching `pred`; the cursor
    /// moves past it.
    pub async fn wait_event(
        &mut self,
        what: &str,
        timeout: Duration,
        mut pred: impl FnMut(&Observed) -> bool,
    ) -> Result<Observed> {
        let deadline = Instant::now() + timeout;
        match self.wait_until(deadline, &mut pred).await? {
            Some(i) => Ok(self.log[i].clone()),
            None => Err(self.timeout_error(what.to_string(), timeout)),
        }
    }

    /// Each step must match, in this order, gaps allowed; one deadline for
    /// all. A timeout names the first step that did not match.
    pub async fn expect_in_order(
        &mut self,
        what: &str,
        timeout: Duration,
        steps: &mut [(&str, &mut dyn FnMut(&Observed) -> bool)],
    ) -> Result<Vec<Observed>> {
        let deadline = Instant::now() + timeout;
        let total = steps.len();
        let mut matched = Vec::with_capacity(total);
        for (i, (name, pred)) in steps.iter_mut().enumerate() {
            match self.wait_until(deadline, &mut **pred).await? {
                Some(at) => matched.push(self.log[at].clone()),
                None => {
                    return Err(self.timeout_error(step_label(what, i, total, name), timeout));
                }
            }
        }
        Ok(matched)
    }

    /// Pump until exactly one entity matches `q`; returns its id. More than
    /// one match at the deadline is reported as [`MirrorError::Ambiguous`],
    /// since waiting longer was not going to help.
    pub async fn wait_entity(&mut self, q: &EntityQuery, timeout: Duration) -> Result<u32> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.mirror.find_one(q) {
                Ok(entity) => return Ok(entity.entity_id),
                Err(err) if Instant::now() >= deadline => {
                    return Err(match err {
                        MirrorError::Ambiguous { .. } => Error::Mirror(err),
                        _ => self.timeout_error(format!("an entity matching {q}"), timeout),
                    });
                }
                Err(_) => {}
            }
            self.pump_step(deadline).await?;
        }
    }

    /// Move the cursor past everything logged so far, so the next wait can
    /// only match what arrives after this point: a script calls it right
    /// before a send whose answers must not be satisfied by earlier traffic.
    pub fn skip_to_end(&mut self) {
        self.cursor = self.log.len();
    }

    /// The last `n` log entries, one line each, for failure messages.
    pub fn trail(&self, n: usize) -> String {
        format_trail(&self.log, n, self.mirror.own_id())
    }

    /// Scan from the cursor, pumping one bundle at a time, until `pred`
    /// matches or `deadline` passes. Only entries added since the last scan
    /// are scanned again. `Ok(None)` is a timeout.
    async fn wait_until<P>(&mut self, deadline: Instant, pred: &mut P) -> Result<Option<usize>>
    where
        P: FnMut(&Observed) -> bool + ?Sized,
    {
        let mut from = self.cursor;
        loop {
            if let Some(i) = scan(&self.log, from, &mut *pred) {
                self.cursor = i + 1;
                return Ok(Some(i));
            }
            from = self.log.len();
            if Instant::now() >= deadline {
                // Whatever already arrived counts: a script that blocked on
                // something else (a DB poll) may have a backlog in the inbox,
                // and a zero timeout must still see it.
                self.drain_backlog().await?;
                return Ok(scan(&self.log, from, &mut *pred).inspect(|i| self.cursor = i + 1));
            }
            self.pump_step(deadline).await?;
        }
    }

    /// Send the keep-alive if it is due, then receive at most one bundle,
    /// waiting no later than `deadline` or the next keep-alive. Returning
    /// after every bundle lets a wait match as soon as its message is in.
    async fn pump_step(&mut self, deadline: Instant) -> Result<usize> {
        if keepalive_due(self.last_send, Instant::now()) {
            self.send_keepalive().await?;
        }
        let wait = step_wait(Instant::now(), deadline, self.last_send);
        let mut added = 0;
        // `recv_bundles(1, ..)` drains at most one bundle from the peer's
        // inbox; anything behind it stays queued for the next step.
        for bundle in self.session.recv_bundles(1, wait).await {
            added += self.ingest(&bundle)?;
        }
        Ok(added)
    }

    /// Ingest every bundle already in the inbox, without waiting.
    async fn drain_backlog(&mut self) -> Result<usize> {
        let mut added = 0;
        for bundle in self.session.recv_bundles(usize::MAX, Duration::ZERO).await {
            added += self.ingest(&bundle)?;
        }
        Ok(added)
    }

    /// The tick's packet: always unreliable (D-WC12). Queued
    /// `requestEntityUpdate`s ride behind the `AUTHENTICATE` in the same
    /// bundle ([`keepalive_body`]).
    ///
    /// The real client sends `0x07` reliably, but a reliable send here buys
    /// nothing: the wire client's peer never retransmits (nothing drives
    /// `LoopbackPeer::tick`), and the server answers `0x07` with nothing, so
    /// a lost one changes no test outcome. Keeping the tick unreliable keeps
    /// it off the reliable sequence and the TX window.
    async fn send_keepalive(&mut self) -> Result<()> {
        let body = keepalive_body(&mut self.pending_updates);
        if let Err(err) = self.session.send_bundle(&body, false).await {
            tracing::warn!(bytes = body.len(), error = %err, "script: keep-alive send failed");
            return Err(err.into());
        }
        self.last_send = Instant::now();
        Ok(())
    }

    /// Decode one bundle strictly and log every message in it.
    fn ingest(&mut self, bundle: &Bytes) -> Result<usize> {
        let msgs = decode_bundle_strict(bundle).inspect_err(|err| {
            tracing::warn!(error = %err, len = bundle.len(), "script: bundle failed strict decode");
        })?;
        let at = self.started.elapsed();
        Ok(ingest_messages(
            &mut self.mirror,
            msgs,
            at,
            &mut self.log,
            &mut self.pending_updates,
        )
        .inspect_err(|err| tracing::warn!(error = %err, "script: mirror rejected a message"))?)
    }

    fn timeout_error(&self, what: String, waited: Duration) -> Error {
        Error::ScriptTimeout {
            what,
            waited,
            trail: self.trail(TRAIL_ON_TIMEOUT),
        }
    }
}

/// Apply one decoded bundle to the mirror and log it. Returns how many
/// messages were logged.
///
/// A bundle of nothing but `tickSync` is skipped whole: at 10 Hz it would
/// bury everything else. A `tickSync` inside a bundle with anything else is
/// logged like any message, so no bundle is ever partly dropped.
///
/// A message the mirror rejects ends the script, but it and the rest of its
/// bundle are still logged (not applied: the mirror stops at the first bad
/// message), so the failure's trail shows them.
fn ingest_messages(
    mirror: &mut EntityMirror,
    msgs: Vec<S2CMessage>,
    at: Duration,
    log: &mut Vec<Observed>,
    pending_updates: &mut Vec<u8>,
) -> std::result::Result<usize, MirrorError> {
    if is_tick_sync_only(&msgs) {
        return Ok(0);
    }
    let count = msgs.len();
    let mut failure = None;
    for msg in msgs {
        let mut event = None;
        if failure.is_none() {
            match mirror.apply(&msg) {
                Ok(ev) => {
                    event = ev;
                    if let Some(id) = entity_update_target(&msg, mirror.own_id()) {
                        pending_updates.extend_from_slice(&GameSession::request_entity_update(id));
                    }
                }
                Err(err) => failure = Some(err),
            }
        }
        log.push(Observed { at, msg, event });
    }
    match failure {
        Some(err) => Err(err),
        None => Ok(count),
    }
}

/// True for an empty bundle or one holding only `tickSync`.
fn is_tick_sync_only(msgs: &[S2CMessage]) -> bool {
    msgs.iter().all(|m| m.msg_id == MSG_TICK_SYNC)
}

/// The entity a real client would send `requestEntityUpdate` for after this
/// message: one per `CREATE_ENTITY` of anything but its own player, for
/// networked ids only (README D-WC12; the client's `onEntityEnter`).
fn entity_update_target(msg: &S2CMessage, own_id: Option<u32>) -> Option<u32> {
    let id = msg.entity_id?;
    (msg.msg_id == MSG_CREATE_ENTITY && Some(id) != own_id && id < CLIENT_ONLY_ID_FLOOR)
        .then_some(id)
}

/// Index of the first entry at or after `from` that matches `pred`.
fn scan<P>(log: &[Observed], from: usize, pred: &mut P) -> Option<usize>
where
    P: FnMut(&Observed) -> bool + ?Sized,
{
    log.get(from..)?.iter().position(pred).map(|i| from + i)
}

/// True once [`ScriptSession::KEEPALIVE`] has passed since `last`.
fn keepalive_due(last: Instant, now: Instant) -> bool {
    now.saturating_duration_since(last) >= ScriptSession::KEEPALIVE
}

/// How long one pump step may wait in `recv`: until the deadline or the next
/// keep-alive, whichever is sooner, so neither is ever late.
fn step_wait(now: Instant, deadline: Instant, last_send: Instant) -> Duration {
    let until_keepalive =
        ScriptSession::KEEPALIVE.saturating_sub(now.saturating_duration_since(last_send));
    deadline.saturating_duration_since(now).min(until_keepalive)
}

/// The body of a script send: the queued `requestEntityUpdate`s first (the
/// client queued each when its entity entered, before the script's call),
/// then `msgs`. Empties the queue.
fn take_outgoing(pending: &mut Vec<u8>, msgs: &[Vec<u8>]) -> Vec<u8> {
    let mut body = std::mem::take(pending);
    for msg in msgs {
        body.extend_from_slice(msg);
    }
    body
}

/// The keep-alive tick's body: `AUTHENTICATE`, then any queued
/// `requestEntityUpdate`s. Empties the queue.
fn keepalive_body(pending: &mut Vec<u8>) -> Vec<u8> {
    let mut body = GameSession::authenticate();
    body.append(pending);
    body
}

/// What a timed-out `expect_in_order` says it was waiting for.
fn step_label(what: &str, index: usize, total: usize, name: &str) -> String {
    format!("{what}: step {} of {total} `{name}`", index + 1)
}

/// The last `n` entries of `log`, one [`trail_line`] each.
fn format_trail(log: &[Observed], n: usize, own_id: Option<u32>) -> String {
    let tail = &log[log.len().saturating_sub(n)..];
    if tail.is_empty() {
        return "(nothing observed)".to_string();
    }
    tail.iter()
        .map(|o| trail_line(o, own_id))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `+12.345s msg 0xbd entity 8 m105 DialogDisplay(..)`.
///
/// A `0xBD` method's index depends on its target's idbase (61 for a player,
/// 62 for an NPC), which the log entry does not carry. On the own player the
/// line says `m<index>`; on anything else it gives the raw sub-slot
/// (`sub<n>`) rather than guess.
fn trail_line(o: &Observed, own_id: Option<u32>) -> String {
    use std::fmt::Write as _;
    let msg = &o.msg;
    let mut line = format!("+{:.3}s msg {:#04x}", o.at.as_secs_f64(), msg.msg_id);
    if let Some(id) = msg.entity_id {
        let _ = write!(line, " entity {id}");
    }
    match (msg.msg_id, msg.sub_index) {
        (0xBD, Some(sub)) if msg.entity_id.is_some() && msg.entity_id == own_id => {
            let _ = write!(line, " m{}", u16::from(sub) + u16::from(IDBASE_SGW_PLAYER));
        }
        (0xBD, Some(sub)) => {
            let _ = write!(line, " sub{sub}");
        }
        (0x80..=0xFE, _) => {
            if let Some(index) = msg.method_index_for(IDBASE_SGW_PLAYER) {
                let _ = write!(line, " m{index}");
            }
        }
        _ => {}
    }
    match &o.event {
        Some(event) => {
            let text = format!("{event:?}");
            if text.chars().count() > TRAIL_EVENT_CHARS {
                let cut: String = text.chars().take(TRAIL_EVENT_CHARS).collect();
                let _ = write!(line, " {cut}...");
            } else {
                let _ = write!(line, " {text}");
            }
        }
        None => {
            let _ = write!(line, " len {}", msg.payload.len());
        }
    }
    line
}

#[cfg(test)]
mod tests;
