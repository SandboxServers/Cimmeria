//! `${cast_id}` per press (ability-mechanics AB-L3). The server mints a
//! cast's `cast_id` from its `effect_seq` and puts it on every cast row
//! (AB-T1), so a row's SigNoz and server clauses can name exactly the cast
//! its press started. After every `client_use_ability` that ran (setup or
//! step), the runner looks for that cast, trying in order:
//!
//! 1. **`client_recv`** — the client's own `client.ability.recv`
//!    `onEffectResults` for the pressed ability, after the press's event
//!    seq (AB-C3): its `cast_id` (the effect id the server sent).
//! 2. **`seq_join`** — the press's `client.ability.sent` (AB-C1), its
//!    `client.ability.sent_seq` packet range, the server's
//!    `use_ability_recv` row whose `mercury_seq` falls in it (AB-T2), and
//!    that entity's next `ability_launched` row for the ability, read from
//!    `server_log_tail` (the server's DEBUG ring, 500 rows).
//! 3. **`press_window`** — the plan's fallback join: the
//!    `ability_launched` row for `(player entity, ability)` within 2 s of
//!    the press on the server clock (the anchor's offset), nearest first.
//!
//! The value goes to `${cast_id}` (the latest press) and, for a labelled
//! press, `${cast_id_<label>}`. The press's action records which path
//! found it, or every reason none did; a press with no cast found clears
//! `${cast_id}`, so a later clause cannot quietly use an earlier press's.

use serde_json::{json, Value};

use super::client_events::WAIT_EVENT_TOOL;
use super::packet::{find_session, ENTITY_VAR};
use super::players::{Who, P2_CHARACTER_VAR};
use super::{RowCtx, Runner};
use crate::uat::evidence::ActionRecord;
use crate::uat::invoke::{ServerInvoker, ToolInvoker};

pub(crate) const CAST_ID_VAR: &str = "cast_id";
/// How long the client receipt may take after `client_use_ability`
/// returns (the tool has already watched for up to 2.5 s).
const RECV_WAIT_MS: u64 = 1500;
/// How long the `sent_seq` row may lag its `sent` (network thread).
const SEQ_WAIT_MS: u64 = 1000;
/// The plan's fallback window around the press.
const PRESS_WINDOW_MS: i64 = 2000;
/// Server log rows read for the server joins (the ring's size).
const LOG_TAIL: u64 = 500;
/// The client counts packet seqs in 28 bits; a range may wrap.
const SEQ_MASK: u64 = (1 << 28) - 1;

/// Whether `seq` is in `[first, last]` on the 28-bit wrapping counter.
pub fn seq_in_range(seq: u64, first: u64, last: u64) -> bool {
    let (seq, first, last) = (seq & SEQ_MASK, first & SEQ_MASK, last & SEQ_MASK);
    if first <= last {
        (first..=last).contains(&seq)
    } else {
        seq >= first || seq <= last
    }
}

fn field<'a>(e: &'a Value, name: &str) -> Option<&'a Value> {
    e.get("fields").and_then(|f| f.get(name))
}

fn num(v: Option<&Value>) -> Option<i64> {
    match v? {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn is_event(e: &Value, name: &str) -> bool {
    field(e, "event").and_then(Value::as_str) == Some(name)
}

/// Path 2's server half: the `use_ability_recv` in the packet range, then
/// the same entity's next `ability_launched` for the ability.
pub fn join_by_seq(log: &[Value], ability: i64, first: u64, last: u64) -> Option<i64> {
    let recv_at = log.iter().position(|e| {
        is_event(e, "use_ability_recv")
            && num(field(e, "ability_id")) == Some(ability)
            && num(field(e, "mercury_seq"))
                .is_some_and(|s| s >= 0 && seq_in_range(s as u64, first, last))
    })?;
    let entity = num(field(&log[recv_at], "entity_id"))?;
    log[recv_at..]
        .iter()
        .find(|e| {
            is_event(e, "ability_launched")
                && num(field(e, "entity_id")) == Some(entity)
                && num(field(e, "ability_id")) == Some(ability)
        })
        .and_then(|e| num(field(e, "cast_id")))
}

/// Path 3: the `ability_launched` for `(entity, ability)` nearest the
/// press, within [`PRESS_WINDOW_MS`] (server clock).
pub fn join_by_window(
    log: &[Value],
    entity: i64,
    ability: i64,
    press_server_ms: i64,
) -> Option<i64> {
    log.iter()
        .filter(|e| {
            is_event(e, "ability_launched")
                && num(field(e, "entity_id")) == Some(entity)
                && num(field(e, "ability_id")) == Some(ability)
        })
        .filter_map(|e| {
            let ts = num(e.get("timestamp_ms"))?;
            let d = (ts - press_server_ms).abs();
            (d <= PRESS_WINDOW_MS).then_some((d, num(field(e, "cast_id"))?))
        })
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

impl<I: ToolInvoker> Runner<'_, I> {
    /// Find the cast a `client_use_ability` press became (see the module
    /// docs for the order) and store it. Never fails the press.
    pub(crate) async fn capture_cast_id(
        &mut self,
        who: Who,
        label: Option<&str>,
        out: &Value,
        ctx: &mut RowCtx,
        rec: &mut ActionRecord,
    ) {
        let Some(ability) = out.pointer("/ability/id").and_then(Value::as_i64) else {
            rec.calls
                .push(json!({ "cast_id": null, "tried": ["the press named no ability id"] }));
            ctx.vars.remove(CAST_ID_VAR);
            return;
        };
        let before = out.pointer("/event_seq/before").and_then(Value::as_u64);
        let mut tried: Vec<String> = Vec::new();
        let mut found = self
            .cast_from_client(who, ability, before, &mut tried)
            .await;
        let mut send_range = None;
        if found.is_none() {
            send_range = self.sent_seq_range(who, ability, before, &mut tried).await;
        }
        if found.is_none() {
            found = self
                .cast_from_server(
                    who,
                    ability,
                    send_range,
                    rec.host_started_ms,
                    ctx,
                    &mut tried,
                )
                .await;
        }
        match found {
            Some((cast, via)) => {
                ctx.vars.insert(CAST_ID_VAR.into(), json!(cast));
                if let Some(l) = label {
                    ctx.vars.insert(format!("{CAST_ID_VAR}_{l}"), json!(cast));
                }
                rec.calls
                    .push(json!({ "cast_id": cast, "via": via, "ability_id": ability }));
            }
            None => {
                ctx.vars.remove(CAST_ID_VAR);
                rec.calls
                    .push(json!({ "cast_id": null, "ability_id": ability, "tried": tried }));
            }
        }
    }

    /// One store read after the press: the first matching event, if any.
    async fn first_event(
        &self,
        who: Who,
        kind: &str,
        fields: Value,
        since: u64,
        timeout_ms: u64,
    ) -> Result<Option<Value>, String> {
        let args = json!({
            "kind": kind, "fields": fields, "since_seq": since,
            "count": 1, "timeout_ms": timeout_ms, "cursor": "uat_cast",
        });
        let out = self.on(who).call(WAIT_EVENT_TOOL, args).await;
        if !out.ok {
            return Err(out.error.unwrap_or_default());
        }
        Ok(out
            .json
            .get("matched")
            .and_then(Value::as_array)
            .and_then(|m| m.first())
            .cloned())
    }

    /// Path 1: the client's receipt of the cast's `onEffectResults`.
    async fn cast_from_client(
        &self,
        who: Who,
        ability: i64,
        before: Option<u64>,
        tried: &mut Vec<String>,
    ) -> Option<(i64, &'static str)> {
        let Some(since) = before else {
            tried.push("client_recv: the press reported no event seq".into());
            return None;
        };
        if !self.on(who).has_tool(WAIT_EVENT_TOOL) {
            tried.push(format!("client_recv: {WAIT_EVENT_TOOL} is not routed"));
            return None;
        }
        let fields = json!({ "method": "onEffectResults", "ability_id": ability });
        match self
            .first_event(who, "ability.recv", fields, since, RECV_WAIT_MS)
            .await
        {
            Ok(Some(e)) => match num(field(&e, "cast_id")).or_else(|| num(field(&e, "effect_id"))) {
                Some(c) => return Some((c, "client_recv")),
                None => tried.push("client_recv: the onEffectResults row has no cast_id".into()),
            },
            Ok(None) => tried.push(format!(
                "client_recv: no client.ability.recv onEffectResults for ability {ability} within {RECV_WAIT_MS} ms"
            )),
            Err(e) => tried.push(format!("client_recv: {e}")),
        }
        None
    }

    /// The packet range that carried the press's `useAbility`, from
    /// `client.ability.sent` joined to `client.ability.sent_seq` by
    /// `send_id`.
    async fn sent_seq_range(
        &self,
        who: Who,
        ability: i64,
        before: Option<u64>,
        tried: &mut Vec<String>,
    ) -> Option<(u64, u64)> {
        let since = before?;
        if !self.on(who).has_tool(WAIT_EVENT_TOOL) {
            return None;
        }
        let sent = match self
            .first_event(
                who,
                "ability.sent",
                json!({ "ability_id": ability }),
                since,
                0,
            )
            .await
        {
            Ok(Some(e)) => e,
            Ok(None) => {
                tried.push(format!(
                    "seq_join: no client.ability.sent for ability {ability}"
                ));
                return None;
            }
            Err(e) => {
                tried.push(format!("seq_join: {e}"));
                return None;
            }
        };
        let Some(send_id) = num(field(&sent, "send_id")) else {
            tried.push("seq_join: the sent row has no send_id".into());
            return None;
        };
        let range = self
            .first_event(
                who,
                "ability.sent_seq",
                json!({ "send_id": send_id }),
                since,
                SEQ_WAIT_MS,
            )
            .await;
        match range {
            Ok(Some(e)) => {
                let first = num(field(&e, "mercury_seq_first"))?;
                let last = num(field(&e, "mercury_seq_last")).unwrap_or(first);
                Some((first.max(0) as u64, last.max(0) as u64))
            }
            Ok(None) => {
                tried.push(format!(
                    "seq_join: no client.ability.sent_seq for send {send_id}"
                ));
                None
            }
            Err(e) => {
                tried.push(format!("seq_join: {e}"));
                None
            }
        }
    }

    /// Paths 2 and 3 over one `server_log_tail` read.
    async fn cast_from_server(
        &self,
        who: Who,
        ability: i64,
        range: Option<(u64, u64)>,
        press_host_ms: i64,
        ctx: &mut RowCtx,
        tried: &mut Vec<String>,
    ) -> Option<(i64, &'static str)> {
        let Some(server) = self.server else {
            tried.push("server joins: server lab MCP not configured".into());
            return None;
        };
        let out = server
            .call("server_log_tail", json!({ "limit": LOG_TAIL }))
            .await;
        let log = match (out.ok, out.json.get("entries").and_then(Value::as_array)) {
            (true, Some(l)) => l.clone(),
            _ => {
                tried.push(format!(
                    "server joins: server_log_tail: {}",
                    out.error.unwrap_or_else(|| "no entries".into())
                ));
                return None;
            }
        };
        if let Some((first, last)) = range {
            match join_by_seq(&log, ability, first, last) {
                Some(c) => return Some((c, "seq_join")),
                None => tried.push(format!(
                    "seq_join: no use_ability_recv with mercury_seq in {first}..={last} and its ability_launched in the server log tail"
                )),
            }
        }
        let entity = match self.press_entity(server, who, ctx).await {
            Ok(e) => i64::from(e),
            Err(e) => {
                tried.push(format!("press_window: {e}"));
                return None;
            }
        };
        let offset = ctx
            .anchor
            .as_ref()
            .and_then(|a| a.server_offset_ms)
            .unwrap_or(0);
        match join_by_window(&log, entity, ability, press_host_ms + offset) {
            Some(c) => Some((c, "press_window")),
            None => {
                tried.push(format!(
                    "press_window: no ability_launched for entity {entity}, ability {ability} within {PRESS_WINDOW_MS} ms"
                ));
                None
            }
        }
    }

    /// The pressing client's player entity: p1 is the lab character (the
    /// packet tap's lookup), p2 its own character by name.
    async fn press_entity(
        &self,
        server: &dyn ServerInvoker,
        who: Who,
        ctx: &mut RowCtx,
    ) -> Result<u32, String> {
        if who == Who::P1 {
            let e = self.tap_entity(server, ctx).await?;
            ctx.vars.insert(ENTITY_VAR.into(), json!(e));
            return Ok(e);
        }
        let name = ctx
            .vars
            .get(P2_CHARACTER_VAR)
            .and_then(Value::as_str)
            .ok_or("no p2 character name")?
            .to_string();
        let out = server.call("server_sessions", json!({})).await;
        if !out.ok {
            return Err(format!(
                "server_sessions: {}",
                out.error.unwrap_or_default()
            ));
        }
        find_session(&out.json, &name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(ts: i64, fields: Value) -> Value {
        json!({ "timestamp_ms": ts, "level": "DEBUG", "target": "abilities", "message": "", "fields": fields })
    }

    #[test]
    fn a_packet_range_wraps_at_28_bits() {
        assert!(seq_in_range(5, 4, 6));
        assert!(!seq_in_range(7, 4, 6));
        let top = SEQ_MASK;
        assert!(seq_in_range(top, top - 1, 1));
        assert!(seq_in_range(0, top - 1, 1));
        assert!(!seq_in_range(2, top - 1, 1));
    }

    #[test]
    fn the_seq_join_takes_the_receipt_entitys_next_launch() {
        let log = [
            // Another player's launch of the same ability, earlier.
            row(
                1,
                json!({ "event": "ability_launched", "entity_id": 9, "ability_id": 597, "cast_id": 11 }),
            ),
            row(
                2,
                json!({ "event": "use_ability_recv", "entity_id": 7, "ability_id": 597, "mercury_seq": 120 }),
            ),
            row(
                3,
                json!({ "event": "ability_launched", "entity_id": 9, "ability_id": 597, "cast_id": 12 }),
            ),
            row(
                4,
                json!({ "event": "ability_launched", "entity_id": 7, "ability_id": 597, "cast_id": 13 }),
            ),
        ];
        assert_eq!(join_by_seq(&log, 597, 119, 121), Some(13));
        assert_eq!(join_by_seq(&log, 597, 121, 125), None);
        assert_eq!(join_by_seq(&log, 1646, 119, 121), None);
    }

    #[test]
    fn the_window_join_picks_the_nearest_launch_within_two_seconds() {
        let launch = |ts, cast| {
            row(
                ts,
                json!({ "event": "ability_launched", "entity_id": 7, "ability_id": 637, "cast_id": cast }),
            )
        };
        let log = [launch(10_000, 1), launch(12_900, 2), launch(13_400, 3)];
        assert_eq!(join_by_window(&log, 7, 637, 13_000), Some(2));
        assert_eq!(join_by_window(&log, 7, 637, 20_000), None);
        assert_eq!(join_by_window(&log, 8, 637, 13_000), None);
    }
}
