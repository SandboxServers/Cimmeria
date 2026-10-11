//! The sampler's server reads: one `tools/call` shape, the player lookup,
//! the DB snapshot, the noise filter, the named-entity lookups and the
//! known-server-event poll. All of it goes through
//! [`ServerInvoker::call`]; the only SQL is read-only and keyed by a
//! validated name or the integer `player_id` (D-LR2).

use serde_json::{json, Value};

use super::super::log::{DbSnapshot, SnapItem, SnapMission, SnapPlayer, Tick};
use super::{snapshot_sql, Sampler, KNOWN_SERVER_EVENTS};
use crate::uat::golden::normalise::{glob_match, DEFAULT_IGNORE};
use crate::uat::invoke::ServerInvoker;

/// New entity lookups per tick; the rest wait for the next one.
pub(super) const MAX_LOOKUPS_PER_TICK: usize = 8;
/// Entity ids a recording tracks (seen, waiting, given up); past it no new
/// id is looked up.
pub(super) const MAX_TRACKED_IDS: usize = 2048;
/// Retries of a failed lookup (a null entity is final, not a failure).
pub(super) const MAX_ENTITY_RETRIES: u8 = 3;

/// One call; a failed outcome is `Err("<tool>: <error>")`.
pub(super) async fn call_ok(
    server: &dyn ServerInvoker,
    name: &str,
    args: Value,
) -> Result<Value, String> {
    let out = server.call(name, args).await;
    if out.ok {
        Ok(out.json)
    } else {
        Err(format!("{name}: {}", out.error.unwrap_or_default()))
    }
}

/// `server_db_query`'s rows (each one a JSON object). A truncated result
/// is a failure: a cut-off row list would read as rows disappearing.
async fn db_rows(server: &dyn ServerInvoker, sql: &str) -> Result<Vec<Value>, String> {
    let r = call_ok(server, "server_db_query", json!({ "sql": sql })).await?;
    if r.get("truncated").and_then(Value::as_bool) == Some(true) {
        return Err("server_db_query: truncated result".to_string());
    }
    r.get("rows")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| format!("server_db_query: no rows in {r}"))
}

/// The player's `sgw_player.player_id`, by exact name. `character` has
/// passed [`super::check_character`], so it holds no quote.
pub(super) async fn lookup_player_id(
    server: &dyn ServerInvoker,
    character: &str,
) -> Result<i32, String> {
    let sql = format!("SELECT player_id FROM sgw_player WHERE player_name = '{character}'");
    let rows = db_rows(server, &sql).await?;
    match rows.as_slice() {
        [one] => one
            .get("player_id")
            .and_then(Value::as_i64)
            .and_then(|v| i32::try_from(v).ok())
            .ok_or_else(|| format!("sgw_player row without a player_id: {one}")),
        [] => Err(format!("no sgw_player named {character:?}")),
        many => Err(format!(
            "{} sgw_player rows named {character:?}",
            many.len()
        )),
    }
}

/// The contract's three snapshot statements. One that fails drops the
/// whole snapshot: a partial one would read as missions disappearing.
pub(super) async fn db_snapshot(
    server: &dyn ServerInvoker,
    player_id: i32,
) -> Result<DbSnapshot, String> {
    let [missions, player, items] = snapshot_sql(player_id);
    let missions = db_rows(server, &missions)
        .await?
        .into_iter()
        .map(serde_json::from_value::<SnapMission>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("sgw_mission row: {e}"))?;
    let player = db_rows(server, &player)
        .await?
        .into_iter()
        .next()
        .map(serde_json::from_value::<SnapPlayer>)
        .transpose()
        .map_err(|e| format!("sgw_player row: {e}"))?;
    // `type_id` and `container_id` are nullable; a row without them names
    // no item a clause could check, so it is skipped.
    let items = db_rows(server, &items)
        .await?
        .into_iter()
        .filter_map(|r| serde_json::from_value::<SnapItem>(r).ok())
        .collect();
    Ok(DbSnapshot {
        missions,
        player,
        items,
    })
}

/// `DEFAULT_IGNORE` from lab-golden: messages whose count is time spent.
pub(super) fn is_noise(m: &Value) -> bool {
    let name = m.get("msg_name").and_then(Value::as_str).unwrap_or("");
    DEFAULT_IGNORE.iter().any(|g| glob_match(g, name))
}

impl Sampler<'_> {
    /// Queue each id this tick's inbound calls name that is not tracked
    /// yet, then look up at most [`MAX_LOOKUPS_PER_TICK`] queued ids. An id
    /// is seen once its read works or the server says it is gone; a failed
    /// read is retried on later ticks, [`MAX_ENTITY_RETRIES`] times.
    pub(super) async fn follow_named(&mut self, t: &mut Tick, named: Vec<u32>) {
        for id in named {
            if self.seen.contains(&id) || self.gave_up.contains(&id) || self.pending.contains(&id) {
                continue;
            }
            if self.seen.len() + self.pending.len() + self.gave_up.len() >= MAX_TRACKED_IDS {
                if !self.seen_full {
                    self.seen_full = true;
                    self.err(
                        t,
                        "entity_cap",
                        format!("entities: {MAX_TRACKED_IDS} ids tracked, no new lookups"),
                    );
                }
                continue;
            }
            self.pending.push_back(id);
        }

        let mut retry = Vec::new();
        for _ in 0..MAX_LOOKUPS_PER_TICK {
            let Some(id) = self.pending.pop_front() else {
                break;
            };
            match self.entity_get(id).await {
                Ok(Some(e)) => {
                    self.seen.insert(id);
                    self.retries.remove(&id);
                    t.entities.push(e);
                }
                Ok(None) => {
                    self.seen.insert(id);
                    self.retries.remove(&id);
                    self.err(t, "entity", format!("entity {id}: gone"));
                }
                Err(e) => {
                    let tries = self.retries.entry(id).or_insert(0);
                    *tries += 1;
                    if *tries > MAX_ENTITY_RETRIES {
                        self.retries.remove(&id);
                        self.gave_up.insert(id);
                        self.err(t, "entity", format!("entity {id}: {e}"));
                    } else {
                        retry.push(id);
                    }
                }
            }
        }
        if !self.pending.is_empty() {
            let n = self.pending.len();
            self.err(
                t,
                "entity_overflow",
                format!("entities: {n} lookups wait for the next tick"),
            );
        }
        // After the overflow check: a retry alone is not an overflow.
        self.pending.extend(retry);
    }

    /// Read where the server log is now, so a poll only sees what follows.
    pub(super) async fn mark_log(&mut self, i: usize, t: &mut Tick) {
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
            Err(e) => self.err(t, "log", e),
        }
    }

    /// SV-02's `server_log_tail { target, since_ms }`: record each
    /// `KNOWN_SERVER_EVENTS` key whose `event` field and message match.
    pub(super) async fn poll_logs(&mut self, t: &mut Tick) {
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
                    self.err(t, "log", e);
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
