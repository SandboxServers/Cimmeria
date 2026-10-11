//! LR-06: the live sampler that writes the tick lines of a recording.
//!
//! D-LR2: the sampler only reads. Every server read goes through
//! [`ServerInvoker::call`] (`server_sessions`, `server_db_query`,
//! `server_entity_get`, `server_log_tail` and the packet tap the recording
//! owns); every client read goes through [`ClientProbe`], whose live
//! implementation calls the supervisor directly (`camera_read`,
//! `ui_state`, `pump_events`). It never calls a client tool by name, never
//! drives input, camera or focus (the person owns the mouse), and never
//! writes to the database: the only SQL is the player lookup and the three
//! snapshot statements, built from a validated name or the integer
//! `player_id`.
//!
//! D-LR3: one tick per 500 ms (the caller's clock), each one in a fixed
//! order, so every input lands on the host clock and no client/server clock
//! alignment is needed. A failed read never stops a recording: it becomes
//! one `errors` entry and its field is left out.

use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};

use super::log::{
    write_line, EntitySample, RecordLine, RecordStart, RecordStop, Tick, RECORD_SCHEMA,
};
use crate::uat::invoke::ServerInvoker;

#[path = "sampler_reads.rs"]
mod reads;
#[path = "sampler_relog.rs"]
mod relog;
#[path = "sampler_server.rs"]
mod server;

use reads::{camera_sample, find_session, named_target, ui_sample};
pub use reads::{check_character, recording_id, snapshot_sql};
pub use relog::TapStopGuard;
use relog::{start_tap, TAP_REPLACED};
use server::{call_ok, db_snapshot, is_noise, lookup_player_id};

/// A boxed client read (the probe is a trait object, so its futures
/// cannot be `impl Future`).
pub type BoxedJson<'a> = Pin<Box<dyn Future<Output = Result<Value, String>> + Send + 'a>>;

/// The client reads a recording may make (fakes in tests). The live one is
/// the instance's `Supervisor` (`supervisor/record_slot.rs`).
pub trait ClientProbe: Send + Sync {
    /// `Supervisor::camera_read`: the camera readout, which moves nothing.
    fn camera(&self) -> BoxedJson<'_>;
    /// `Supervisor::ui_state(Some(0))`: windows and the open dialog.
    fn ui(&self) -> BoxedJson<'_>;
    /// `Supervisor::pump_events(true)`. LR-10's click subscription is
    /// installed, and its ring read, only inside this pump, so the sampler
    /// calls it once at begin and on every tick (LR-10 review outcome).
    fn pump(&self) -> BoxedJson<'_>;
}

/// key, tracing target, `event` field, log message substring (lab-spec-vocab F8).
pub const KNOWN_SERVER_EVENTS: &[(&str, &str, &str, &str)] = &[(
    "cinematic-hold",
    "aoi.cinematic_hold",
    "hold_released",
    "Cinematic AoI hold: released",
)];

/// Snapshots keep coming for this many quiet ticks after a non-noise
/// message: persistence to `sgw_mission` lags the wire (F5).
const SNAPSHOT_TAIL_TICKS: u32 = 10;
/// The known-server-event probe runs on every second tick.
const LOG_POLL_EVERY: u32 = 2;
const MAX_ERRORS: usize = 4;
const ERROR_CHARS: usize = 120;
/// Every client failure of a tick shares one entry with this prefix, so a
/// broken client cannot crowd the server's errors out of the tick.
const CLIENT_ERR: &str = "client: ";

/// Caps and the tap size for one recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SamplerConfig {
    /// `server_packet_tap_start` capacity (the server's ceiling).
    pub tap_capacity: u32,
    /// Ticks before `full()` says `max_minutes`.
    pub max_ticks: u32,
    /// Log bytes before `full()` says `size_cap`.
    pub max_bytes: u64,
}

impl Default for SamplerConfig {
    fn default() -> Self {
        Self {
            tap_capacity: 10_000,
            // 30 minutes of 500 ms ticks.
            max_ticks: 30 * 60 * 2,
            max_bytes: 64 * 1024 * 1024,
        }
    }
}

/// One recording's reader. Built by [`Sampler::begin`], then [`tick`]ed
/// every 500 ms until [`finish`].
///
/// [`tick`]: Sampler::tick
/// [`finish`]: Sampler::finish
pub struct Sampler<'a> {
    server: &'a dyn ServerInvoker,
    client: &'a dyn ClientProbe,
    cfg: SamplerConfig,
    character: String,
    player_id: i32,
    /// The tapped player entity; changes on a relog.
    entity: u32,
    /// The [`TapStopGuard`]'s copy of `entity`; 0 once `finish` stopped it.
    tap: Arc<AtomicU32>,
    /// Entity ids sampled or reported gone (the player included).
    seen: HashSet<u32>,
    /// Named ids waiting for a lookup (over the per-tick cap, or retrying).
    pending: VecDeque<u32>,
    /// Failed lookups per pending id.
    retries: HashMap<u32, u8>,
    /// Ids whose lookups failed past the retry limit.
    gave_up: HashSet<u32>,
    /// The tracked-id cap was hit (reported once).
    seen_full: bool,
    /// Tick error kinds already traced (each is traced once).
    error_kinds: HashSet<&'static str>,
    /// Snapshot ticks left after the last non-noise message.
    tail: u32,
    /// The next tick's `seq`, which is also the ticks written so far.
    seq: u32,
    bytes: u64,
    /// Per `KNOWN_SERVER_EVENTS` entry: the server timestamp the next poll
    /// reads after. `None` until a mark has been read.
    log_marks: Vec<Option<u64>>,
}

impl<'a> Sampler<'a> {
    /// Resolve the player's `player_id` (server_db_query on the exact
    /// `sgw_player.player_name`; the name must match
    /// `^[A-Za-z][A-Za-z ]{0,63}$`), then the session carrying that
    /// `player_id`, start the tap, take the first snapshot. `tap` is a
    /// [`TapStopGuard::cell`]; the tapped entity is written there before
    /// anything else can fail. Returns the start line and tick 0.
    #[allow(clippy::too_many_arguments)]
    pub async fn begin(
        server: &'a dyn ServerInvoker,
        client: &'a dyn ClientProbe,
        cfg: SamplerConfig,
        tap: Arc<AtomicU32>,
        section: &str,
        instance: &str,
        character: &str,
        now_ms: i64,
        date: &str,
    ) -> Result<(Self, RecordStart, Tick), String> {
        // Before any call: the name is spliced into the one SQL statement
        // that is not keyed by an integer, the instance into a directory name.
        check_character(character)?;
        let recording_id = recording_id(now_ms, instance)?;
        let player_id = lookup_player_id(server, character).await?;
        let sessions = call_ok(server, "server_sessions", json!({})).await?;
        let entity = find_session(&sessions, player_id)?;
        let replaced = start_tap(server, entity, cfg.tap_capacity).await?;
        tap.store(entity, Ordering::SeqCst);
        tracing::debug!(
            target: "lab.record",
            character, player_id, entity, section, instance, replaced,
            "recording: tap started"
        );

        let mut s = Self {
            server,
            client,
            cfg,
            character: character.to_string(),
            player_id,
            entity,
            tap,
            seen: HashSet::from([entity]),
            pending: VecDeque::new(),
            retries: HashMap::new(),
            gave_up: HashSet::new(),
            seen_full: false,
            error_kinds: HashSet::new(),
            tail: 0,
            seq: 0,
            bytes: 0,
            log_marks: vec![None; KNOWN_SERVER_EVENTS.len()],
        };
        let start = RecordStart {
            schema: RECORD_SCHEMA,
            recording_id,
            section: section.to_string(),
            instance: instance.to_string(),
            character: character.to_string(),
            player_id,
            player_entity: entity,
            host_ms: now_ms,
            date: date.to_string(),
            rolling_min: None,
        };
        s.bytes += write_line(&RecordLine::Start(start.clone())).len() as u64;

        let mut t = s.new_tick(now_ms);
        if replaced {
            s.err(&mut t, "tap_replaced", TAP_REPLACED);
        }
        for i in 0..KNOWN_SERVER_EVENTS.len() {
            s.mark_log(i, &mut t).await;
        }
        s.sample(&mut t, true).await;
        s.account(&t);
        Ok((s, start, t))
    }

    /// One tick, in this order: drain the tap; camera; the player's
    /// `server_entity_get`; the UI; `server_entity_get` for ids an inbound
    /// call names that are not yet seen (at most 8 a tick); the
    /// known-server-event probe; a snapshot when this or the previous tick
    /// carried a non-noise message, or within 10 ticks after one.
    pub async fn tick(&mut self, now_ms: i64) -> Tick {
        let mut t = self.new_tick(now_ms);
        self.sample(&mut t, false).await;
        self.account(&t);
        t
    }

    /// Final snapshot, tap stop. Never fails: errors go in the tick.
    pub async fn finish(&mut self, now_ms: i64, reason: &str) -> (Tick, RecordStop) {
        let mut t = self.new_tick(now_ms);
        self.sample(&mut t, true).await;
        if let Err(e) = call_ok(
            self.server,
            "server_packet_tap_stop",
            json!({ "entity_id": self.entity }),
        )
        .await
        {
            self.err(&mut t, "tap_stop", e);
        }
        // The stop was tried: the guard must not stop this entity again
        // later, when another recording may own a tap on it.
        self.tap.store(0, Ordering::SeqCst);
        self.account(&t);
        let stop = RecordStop {
            host_ms: now_ms,
            ticks: self.seq,
            reason: reason.to_string(),
        };
        (t, stop)
    }

    /// True when max_ticks or max_bytes is reached (the caller stops with that reason).
    pub fn full(&self) -> Option<&'static str> {
        if self.seq >= self.cfg.max_ticks {
            Some("max_minutes")
        } else if self.bytes >= self.cfg.max_bytes {
            Some("size_cap")
        } else {
            None
        }
    }

    /// The tapped player entity (it changes on a relog).
    #[cfg(test)]
    pub fn player_entity(&self) -> u32 {
        self.entity
    }

    fn new_tick(&self, now_ms: i64) -> Tick {
        Tick {
            seq: self.seq,
            host_ms: now_ms,
            ..Default::default()
        }
    }

    fn account(&mut self, t: &Tick) {
        self.seq += 1;
        self.bytes += write_line(&RecordLine::Tick(t.clone())).len() as u64;
    }

    /// Every read of one tick. `edge` is begin or finish: always snapshot,
    /// always poll the server log.
    async fn sample(&mut self, t: &mut Tick, edge: bool) {
        // The pump comes first so LR-11 reads this tick's clicks from the
        // store; a failed Lua step is reported but the bridge drain landed.
        match self.client.pump().await {
            Ok(r) => {
                if let Some(e) = r.get("lua_error").and_then(Value::as_str) {
                    self.client_err(t, "pump", format!("pump: {e}"));
                }
            }
            Err(e) => self.client_err(t, "pump", format!("pump: {e}")),
        }

        let mut relog_checked = false;
        let mut active = false;
        let mut named = Vec::new();
        match call_ok(
            self.server,
            "server_packet_tap_read",
            json!({ "entity_id": self.entity }),
        )
        .await
        {
            Ok(r) => {
                t.tap_dropped = r.get("dropped").and_then(Value::as_u64).unwrap_or(0);
                t.tap = r
                    .get("messages")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                active = t.tap.iter().any(|m| !is_noise(m));
                named = t.tap.iter().filter_map(named_target).collect();
            }
            Err(e) => {
                // A disconnect never stops a tap: it outlives its session.
                // "no active packet tap" means something stopped or
                // replaced it, so it is restarted if the player is still
                // on this entity; any failure may also be a relog.
                let missing = e.contains("no active packet tap");
                self.err(t, "tap_read", e);
                self.check_relog(t, missing).await;
                relog_checked = true;
            }
        }

        match self.client.camera().await.and_then(|v| camera_sample(&v)) {
            Ok(c) => t.camera = Some(c),
            Err(e) => self.client_err(t, "camera", format!("camera: {e}")),
        }

        let lost = match self.entity_get(self.entity).await {
            // A recycled id: the entity is someone else now.
            Ok(Some(p)) if !self.is_ours(&p) => Some(format!(
                "player: entity {} is now {:?}",
                self.entity,
                p.name.unwrap_or_default()
            )),
            Ok(Some(p)) => {
                t.player = Some(p);
                None
            }
            Ok(None) => Some(format!("player: no entity {}", self.entity)),
            Err(e) => Some(format!("player: {e}")),
        };
        if let Some(e) = lost {
            self.err(t, "player", e);
            if !relog_checked {
                self.check_relog(t, false).await;
            }
        }

        match self.client.ui().await {
            Ok(v) => {
                let (windows, dialog, errors) = ui_sample(&v);
                t.windows = windows;
                t.dialog = dialog;
                for e in errors {
                    self.client_err(t, "ui", format!("ui: {e}"));
                }
            }
            Err(e) => self.client_err(t, "ui", format!("ui: {e}")),
        }

        self.follow_named(t, named).await;

        if edge || (t.seq > 0 && t.seq.is_multiple_of(LOG_POLL_EVERY)) {
            self.poll_logs(t).await;
        }

        if active {
            self.tail = SNAPSHOT_TAIL_TICKS;
        }
        let snapshot = if active || edge {
            true
        } else if self.tail > 0 {
            self.tail -= 1;
            true
        } else {
            false
        };
        if snapshot {
            match db_snapshot(self.server, self.player_id).await {
                Ok(s) => t.snapshot = Some(s),
                Err(e) => self.err(t, "snapshot", format!("snapshot: {e}")),
            }
        }
    }

    /// The player sample is this character's, or names nobody (a reply
    /// without a name cannot show a recycled id).
    fn is_ours(&self, p: &EntitySample) -> bool {
        p.name
            .as_deref()
            .is_none_or(|n| n.eq_ignore_ascii_case(&self.character))
    }

    /// `server_entity_get` for `id`: `Ok(None)` only when the server says
    /// the entity is gone (`entity: null`), which is final; a failed call
    /// or an unreadable reply is an `Err` worth retrying.
    async fn entity_get(&self, id: u32) -> Result<Option<EntitySample>, String> {
        let v = call_ok(self.server, "server_entity_get", json!({ "entity_id": id })).await?;
        if v.get("entity").is_some_and(Value::is_null) {
            return Ok(None);
        }
        reads::entity_sample(&v)
            .map(Some)
            .ok_or_else(|| format!("unreadable entity {id}"))
    }

    /// One error entry of `kind`; the first of each kind is also traced.
    fn err(&mut self, t: &mut Tick, kind: &'static str, msg: impl AsRef<str>) {
        let msg = msg.as_ref();
        if self.error_kinds.insert(kind) {
            tracing::debug!(
                target: "lab.record",
                character = %self.character, player_id = self.player_id, kind, error = msg,
                "recording: first {kind} error"
            );
        }
        push_err(t, msg);
    }

    /// A pump, camera or ui failure: the tick's first one becomes its one
    /// `client: ` entry, the rest are dropped.
    fn client_err(&mut self, t: &mut Tick, kind: &'static str, msg: String) {
        if t.errors.iter().any(|e| e.starts_with(CLIENT_ERR)) {
            return;
        }
        self.err(t, kind, format!("{CLIENT_ERR}{msg}"));
    }
}

/// At most [`MAX_ERRORS`] per tick, each cut to [`ERROR_CHARS`].
fn push_err(t: &mut Tick, msg: impl AsRef<str>) {
    if t.errors.len() < MAX_ERRORS {
        t.errors
            .push(msg.as_ref().chars().take(ERROR_CHARS).collect());
    }
}

#[cfg(test)]
#[path = "sampler_tests.rs"]
mod tests;
