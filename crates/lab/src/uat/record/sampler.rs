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

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use serde_json::{json, Value};

use super::log::{
    write_line, EntitySample, RecordLine, RecordStart, RecordStop, Tick, RECORD_SCHEMA,
};
use crate::uat::invoke::ServerInvoker;

#[path = "sampler_reads.rs"]
mod reads;
#[path = "sampler_server.rs"]
mod server;

use reads::{camera_sample, entity_sample, find_session, named_target, ui_sample};
pub use reads::{check_character, recording_id, snapshot_sql};
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
    /// Entity ids already sampled (the player included).
    seen: HashSet<u32>,
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
    /// Resolve the player (server_sessions by character), its player_id
    /// (server_db_query on sgw_player.player_name; the name must match
    /// `^[A-Za-z][A-Za-z ]{0,63}$`), start the tap, take the first
    /// snapshot. Returns the start line and tick 0.
    #[allow(clippy::too_many_arguments)]
    pub async fn begin(
        server: &'a dyn ServerInvoker,
        client: &'a dyn ClientProbe,
        cfg: SamplerConfig,
        section: &str,
        instance: &str,
        character: &str,
        now_ms: i64,
        date: &str,
    ) -> Result<(Self, RecordStart, Tick), String> {
        // Before any call: the name is spliced into the one SQL statement
        // that is not keyed by an integer.
        check_character(character)?;
        let sessions = call_ok(server, "server_sessions", json!({})).await?;
        let entity = find_session(&sessions, character)?;
        let player_id = lookup_player_id(server, character).await?;
        call_ok(
            server,
            "server_packet_tap_start",
            json!({ "entity_id": entity, "capacity": cfg.tap_capacity }),
        )
        .await?;
        tracing::debug!(
            target: "lab.record",
            character, player_id, entity, section, instance,
            "recording: tap started"
        );

        let mut s = Self {
            server,
            client,
            cfg,
            character: character.to_string(),
            player_id,
            entity,
            seen: HashSet::from([entity]),
            tail: 0,
            seq: 0,
            bytes: 0,
            log_marks: vec![None; KNOWN_SERVER_EVENTS.len()],
        };
        let start = RecordStart {
            schema: RECORD_SCHEMA,
            recording_id: recording_id(now_ms, instance),
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
        for i in 0..KNOWN_SERVER_EVENTS.len() {
            s.mark_log(i, &mut t).await;
        }
        s.sample(&mut t, true).await;
        s.account(&t);
        Ok((s, start, t))
    }

    /// One tick, in this order: drain the tap; camera; the player's
    /// `server_entity_get`; the UI; `server_entity_get` for each id an
    /// inbound call names that is not yet seen; the known-server-event
    /// probe; a snapshot when this or the previous tick carried a
    /// non-noise message, or within 10 ticks after one.
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
            push_err(&mut t, e);
        }
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
                    push_err(t, format!("pump: {e}"));
                }
            }
            Err(e) => push_err(t, format!("pump: {e}")),
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
                // A session that ended takes its tap with it.
                push_err(t, e);
                self.check_relog(t).await;
                relog_checked = true;
            }
        }

        match self.client.camera().await.and_then(|v| camera_sample(&v)) {
            Ok(c) => t.camera = Some(c),
            Err(e) => push_err(t, format!("camera: {e}")),
        }

        match self.entity_get(self.entity).await {
            Ok(p) => t.player = Some(p),
            Err(e) => {
                push_err(t, format!("player: {e}"));
                if !relog_checked {
                    self.check_relog(t).await;
                }
            }
        }

        match self.client.ui().await {
            Ok(v) => {
                let (windows, dialog, errors) = ui_sample(&v);
                t.windows = windows;
                t.dialog = dialog;
                for e in errors {
                    push_err(t, format!("ui: {e}"));
                }
            }
            Err(e) => push_err(t, format!("ui: {e}")),
        }

        for id in named {
            // Seen whether or not the read works: one error per entity,
            // not one per tick.
            if !self.seen.insert(id) {
                continue;
            }
            match self.entity_get(id).await {
                Ok(e) => t.entities.push(e),
                Err(e) => push_err(t, format!("entity {id}: {e}")),
            }
        }

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
                Err(e) => push_err(t, format!("snapshot: {e}")),
            }
        }
    }

    async fn entity_get(&self, id: u32) -> Result<EntitySample, String> {
        let v = call_ok(self.server, "server_entity_get", json!({ "entity_id": id })).await?;
        entity_sample(&v).ok_or_else(|| format!("no entity {id}"))
    }

    /// The player's entity went away: find the character again and, when
    /// it came back as another entity, move the tap there.
    async fn check_relog(&mut self, t: &mut Tick) {
        let found = match call_ok(self.server, "server_sessions", json!({})).await {
            Ok(s) => find_session(&s, &self.character),
            Err(e) => Err(e),
        };
        let id = match found {
            Ok(id) if id != self.entity => id,
            // Same entity, or not in the world (yet): nothing to move.
            // TODO: a reconnect onto the same entity id also stops inbound
            // capture until the tap restarts (F4); nothing here sees that.
            Ok(_) | Err(_) => return,
        };
        match call_ok(
            self.server,
            "server_packet_tap_start",
            json!({ "entity_id": id, "capacity": self.cfg.tap_capacity }),
        )
        .await
        {
            Ok(_) => {
                self.entity = id;
                self.seen.insert(id);
                tracing::info!(target: "lab.record", character = %self.character, entity = id, "recording: relog, tap restarted");
                push_err(t, format!("relog: tap restarted on {id}"));
            }
            Err(e) => push_err(t, format!("relog: tap restart on {id} failed: {e}")),
        }
    }

    /// Read where the server log is now, so a poll only sees what follows.
    async fn mark_log(&mut self, i: usize, t: &mut Tick) {
        let (_, target, _, _) = KNOWN_SERVER_EVENTS[i];
        match call_ok(
            self.server,
            "server_log_tail",
            json!({ "target": target, "limit": 1 }),
        )
        .await
        {
            // An empty ring has no newest_ms: every line from now on is new.
            Ok(r) => {
                self.log_marks[i] = Some(r.get("newest_ms").and_then(Value::as_u64).unwrap_or(0))
            }
            Err(e) => push_err(t, e),
        }
    }

    /// SV-02's `server_log_tail { target, since_ms }`: record each
    /// `KNOWN_SERVER_EVENTS` key whose `event` field and message match.
    async fn poll_logs(&mut self, t: &mut Tick) {
        for (i, (key, target, event, message)) in KNOWN_SERVER_EVENTS.iter().enumerate() {
            let Some(since) = self.log_marks[i] else {
                // The mark could not be read at begin; read it now and
                // match from the next poll.
                self.mark_log(i, t).await;
                continue;
            };
            let args =
                json!({ "target": target, "contains": message, "since_ms": since, "limit": 50 });
            let r = match call_ok(self.server, "server_log_tail", args).await {
                Ok(r) => r,
                Err(e) => {
                    push_err(t, e);
                    continue;
                }
            };
            let mut hit = false;
            for line in r
                .get("entries")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let ts = line
                    .get("timestamp_ms")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                if ts <= since {
                    continue;
                }
                // Only lines this probe matched move the mark: a line
                // stamped with the ring's newest ms can still arrive.
                if line["fields"]["event"].as_str() == Some(*event)
                    && line["message"]
                        .as_str()
                        .is_some_and(|m| m.contains(message))
                {
                    hit = true;
                    self.log_marks[i] = Some(self.log_marks[i].unwrap_or(0).max(ts));
                }
            }
            if hit && !t.server_events.iter().any(|k| k == key) {
                t.server_events.push(key.to_string());
            }
        }
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
