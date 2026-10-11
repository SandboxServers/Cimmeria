//! `wait_server` (D-SV5): wait for a server log line over the server lab
//! MCP's `server_log_tail` instead of sleeping.
//!
//! The watermark is the server's newest log timestamp *before* the action
//! that should cause the line ran ([`Runner::log_mark_for`], called from
//! `drive_steps` for setup actions and steps alike), so a fast line is not
//! missed. Three traps shape the polling:
//!
//! - `since_ms` is strictly after, at 1 ms resolution, and a line can be
//!   stamped a moment before it is pushed into the ring: a line stamped
//!   `W - 1` or `W` may arrive after a mark that read `newest_ms == W`. So
//!   every read asks from `W - 2` (lines at `W - 1` on) and skips the lines
//!   it has already examined there. A line stamped earlier than `W - 1`
//!   but pushed after the mark is still missed.
//! - Every lab-mcp call writes a `lab.tool_call` audit row into the same
//!   ring (SV-02). A poll always passes `contains`, and the runner drops
//!   audit rows itself, so the wait's own polls can never satisfy it.
//! - The ring holds 500 lines. A poll asks for all of them, so a burst of
//!   matching lines between two polls cannot push the waited-for one out
//!   of the reply.
//!
//! The runner re-applies every filter to the reply, because a server older
//! than SV-02 ignores `target`, `contains` and `since_ms`. Every call is
//! bounded by the wait's deadline: a server that does not answer fails the
//! wait as an expiry does.

use std::collections::HashSet;
use std::time::Duration;

use serde_json::{json, Map, Value};
use tokio::time::{timeout_at, Instant};

use super::actions::{fail, subst, subst_str, unresolved};
use super::players::Who;
use super::{RowCtx, Runner};
use crate::uat::clause::loose_eq;
use crate::uat::evidence::ActionRecord;
use crate::uat::invoke::{ServerInvoker, ToolInvoker};
use crate::uat::spec::{ActionSpec, ServerWait};

/// The server log reader.
pub(crate) const LOG_TAIL_TOOL: &str = "server_log_tail";
/// How often a wait polls. Each poll adds an audit row to the server's
/// 500-entry ring, so not faster than this (SV-02).
pub(crate) const WAIT_POLL: Duration = Duration::from_millis(250);
/// Lines one read asks for: the server ring's whole capacity (admin-api
/// `broadcast_layer::BUFFER_CAPACITY`).
const POLL_LIMIT: u32 = 500;
/// How far before the cursor a read reaches, for a line stamped before a
/// read but pushed into the ring after it.
const SLACK_MS: u64 = 1;
/// The lab endpoint's own audit target: never a line a wait is for.
const AUDIT_TARGET: &str = "lab.tool_call";
/// The var a field can name to mean "this client's own player entity".
pub(crate) const PLAYER_ENTITY_VAR: &str = "player_entity_id";
/// The client read that knows the player's entity id.
const FIND_TOOL: &str = "client_entity_find";

/// A watermark: the server's newest log timestamp at one moment, and the
/// lines matching the wait that were already there near it.
#[derive(Debug, Clone, Default)]
pub(crate) struct LogMark {
    /// The newest timestamp in the server's whole log ring when the mark
    /// was read (`server_log_tail`'s `newest_ms`, taken before its
    /// filters); 0 for an empty ring. The wait counts lines from
    /// `newest_ms - SLACK_MS` on, minus [`Self::seen`].
    pub newest_ms: u64,
    /// The matching lines already in the ring from `newest_ms - SLACK_MS`
    /// on: not the line the wait is for.
    seen: HashSet<LineKey>,
    /// The `server_log_tail` call that read it, for the action's `calls`.
    call: Value,
}

/// A line's identity for de-duplication: its timestamp, then target,
/// message and fields. Two identical lines in the slack window, one
/// before the mark and one after, count as one: the after line is missed.
/// That needs the same message with the same fields twice within 2 ms,
/// which a wait's line does not do.
type LineKey = (u64, String);

/// One line as `server_log_tail` returns it.
#[derive(Debug)]
struct Line<'a> {
    ts: u64,
    target: &'a str,
    message: &'a str,
    fields: &'a Value,
}

impl<'a> Line<'a> {
    fn of(v: &'a Value) -> Option<Self> {
        Some(Self {
            ts: v.get("timestamp_ms")?.as_u64()?,
            target: v.get("target").and_then(Value::as_str).unwrap_or_default(),
            message: v.get("message").and_then(Value::as_str).unwrap_or_default(),
            fields: v.get("fields").unwrap_or(&Value::Null),
        })
    }

    fn key(&self) -> LineKey {
        let rest = format!("{}\u{1f}{}\u{1f}{}", self.target, self.message, self.fields);
        (self.ts, rest)
    }

    /// The wait's target and message text (the server's own filters,
    /// applied again). Audit rows never match.
    fn named_by(&self, log: &str, target: Option<&str>) -> bool {
        self.target != AUDIT_TARGET
            && target.is_none_or(|t| self.target == t)
            && self.message.contains(log)
    }

    /// Every wanted field equals the line's (numbers numerically).
    fn has_fields(&self, want: &Map<String, Value>) -> bool {
        want.iter()
            .all(|(k, v)| self.fields.get(k).is_some_and(|got| loose_eq(got, v)))
    }
}

/// The `entries` of a reply, in buffer order.
fn entries(reply: &Value) -> &[Value] {
    reply
        .get("entries")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// The oldest timestamp a read from `cursor` keeps.
fn floor(cursor: u64) -> u64 {
    cursor.saturating_sub(SLACK_MS)
}

/// `server_log_tail` args for a wait: always `contains`, so the poll's
/// own audit rows are never what it reads (SV-02).
fn tail_args(log: &str, target: Option<&str>, since_ms: Option<u64>) -> Value {
    let mut args = json!({ "contains": log, "limit": POLL_LIMIT });
    if let Some(t) = target {
        args["target"] = json!(t);
    }
    if let Some(s) = since_ms {
        args["since_ms"] = json!(s);
    }
    args
}

/// A reply that held fewer lines than matched (more than [`POLL_LIMIT`],
/// or a server that caps `limit`): the oldest ones were not returned.
fn warn_if_clipped(reply: &Value, log: &str) {
    let count = entries(reply).len() as u64;
    let total = reply
        .get("total_buffered")
        .and_then(Value::as_u64)
        .unwrap_or(count);
    if total > count {
        tracing::warn!(target: "lab.uat", event = "wait_server_clipped", log, count, total,
            "server_log_tail returned fewer lines than matched; older ones were not examined");
    }
}

/// `log` or `target` with a `${var}` the row never set.
fn unresolved_in(log: &str, target: Option<&str>) -> Option<String> {
    unresolved(log).or_else(|| target.and_then(unresolved))
}

/// Read a watermark for a wait on `log` (and `target`) now.
async fn read_mark(
    server: &dyn ServerInvoker,
    log: &str,
    target: Option<&str>,
) -> Result<LogMark, String> {
    let args = tail_args(log, target, None);
    let out = server.call(LOG_TAIL_TOOL, args.clone()).await;
    if !out.ok {
        return Err(format!(
            "{LOG_TAIL_TOOL}: {}",
            out.error.unwrap_or_default()
        ));
    }
    warn_if_clipped(&out.json, log);
    // An empty ring has no `newest_ms`: every line from now on is new.
    let newest_ms = out
        .json
        .get("newest_ms")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let seen = entries(&out.json)
        .iter()
        .filter_map(Line::of)
        .filter(|l| l.ts >= floor(newest_ms) && l.named_by(log, target))
        .map(|l| l.key())
        .collect();
    Ok(LogMark {
        newest_ms,
        seen,
        call: json!({ "tool": LOG_TAIL_TOOL, "args": args, "ok": true, "mark_ms": newest_ms }),
    })
}

/// Fail a wait whose deadline passed.
fn expire(rec: &mut ActionRecord, log: &str, polls: u32, e: String) {
    tracing::warn!(target: "lab.uat", event = "wait_server_expired", log, polls,
        error = %e, "wait_server timed out");
    fail(rec, e);
}

/// The timeout error, the same whichever call the deadline cut off.
fn expired(log: &str, want: &Map<String, Value>, timeout_ms: u64) -> String {
    let fields = if want.is_empty() {
        String::new()
    } else {
        format!(" {}", Value::Object(want.clone()))
    };
    format!("no server log '{log}'{fields} within {timeout_ms} ms")
}

impl<I: ToolInvoker> Runner<'_, I> {
    /// The watermark for `next` when it is a `wait_server`, read before
    /// the action in front of it runs. `drive_steps` hands it to the wait
    /// through `RowCtx::log_mark` once that action has run. `None` when
    /// `next` is not a wait (or names a var the wait will refuse);
    /// `Some(Err)` when the server could not be read: the wait records
    /// that and reads a late mark at its start.
    pub(crate) async fn log_mark_for(
        &self,
        next: Option<&ActionSpec>,
        ctx: &RowCtx,
    ) -> Option<Result<LogMark, String>> {
        let w = next?.wait_server.as_ref()?;
        let server = self.server?;
        let log = subst_str(&w.log, &ctx.vars);
        let target = w.target.as_ref().map(|t| subst_str(t, &ctx.vars));
        if unresolved_in(&log, target.as_deref()).is_some() {
            return None;
        }
        let mark = read_mark(server, &log, target.as_deref()).await;
        if let Err(e) = &mark {
            tracing::warn!(target: "lab.uat", event = "wait_server_mark_failed", log = %log,
                error = %e, "could not read the server log mark before the action; the wait will read a late one");
        }
        Some(mark)
    }

    /// Run one `wait_server` action: poll `server_log_tail` every
    /// [`WAIT_POLL`] from the watermark until a line with the wait's text
    /// (and target) carries every wanted field, or `timeout_ms` is up.
    /// The watermark is the one `drive_steps` read before the previous
    /// action, or one read now for a row's first action (or when that
    /// read failed). A read error, or a `${var}` left unset, fails it at
    /// once.
    pub(crate) async fn exec_server_wait(
        &self,
        w: &ServerWait,
        who: Who,
        ctx: &mut RowCtx,
        rec: &mut ActionRecord,
    ) {
        rec.kind = "wait_server".into();
        let deadline = Instant::now() + Duration::from_millis(w.timeout_ms);
        let log = subst_str(&w.log, &ctx.vars);
        let target = w.target.as_ref().map(|t| subst_str(t, &ctx.vars));
        rec.requested = format!("wait for server log '{log}'");
        // Taken before the previous action, or here for a row's first one.
        let pre = ctx.log_mark.take();
        if let Some(v) = unresolved_in(&log, target.as_deref()) {
            return fail(rec, format!("unresolved variable {v}"));
        }
        let Some(server) = self.server else {
            return fail(rec, "wait_server needs the server lab MCP".into());
        };
        let want = match timeout_at(deadline, self.wait_fields(w, who, ctx, rec)).await {
            Ok(Ok(f)) => f,
            Ok(Err(e)) => return fail(rec, e),
            Err(_) => {
                let e = format!(
                    "{} ({FIND_TOOL} did not answer)",
                    expired(&log, &Map::new(), w.timeout_ms)
                );
                return expire(rec, &log, 0, e);
            }
        };
        let mark = match pre {
            Some(Ok(m)) => m,
            pre => {
                if let Some(Err(e)) = pre {
                    rec.calls
                        .push(json!({ "tool": LOG_TAIL_TOOL, "ok": false, "pre_action_mark": e }));
                }
                match timeout_at(deadline, read_mark(server, &log, target.as_deref())).await {
                    Ok(Ok(m)) => m,
                    Ok(Err(e)) => {
                        tracing::warn!(target: "lab.uat", event = "wait_server_read_failed",
                            log = %log, error = %e, "wait_server could not read the server log");
                        return fail(rec, e);
                    }
                    Err(_) => return expire(rec, &log, 0, expired(&log, &want, w.timeout_ms)),
                }
            }
        };
        rec.calls.push(mark.call.clone());
        rec.args =
            json!({ "log": log, "target": target, "fields": want, "timeout_ms": w.timeout_ms });

        // Lines from `floor(cursor)` on that were already examined; older
        // ones are excluded by `since_ms` (and by the refilter, for an old
        // server).
        let mut cursor = mark.newest_ms;
        let mut seen = mark.seen;
        let mut polls = 0u32;
        loop {
            polls += 1;
            let since = cursor.checked_sub(SLACK_MS + 1);
            let args = tail_args(&log, target.as_deref(), since);
            let summary =
                |ok: bool| json!({ "tool": LOG_TAIL_TOOL, "args": args, "ok": ok, "polls": polls });
            let Ok(out) = timeout_at(deadline, server.call(LOG_TAIL_TOOL, args.clone())).await
            else {
                rec.calls.push(summary(false));
                return expire(rec, &log, polls, expired(&log, &want, w.timeout_ms));
            };
            if !out.ok {
                rec.calls.push(summary(false));
                let e = format!("{LOG_TAIL_TOOL}: {}", out.error.unwrap_or_default());
                tracing::warn!(target: "lab.uat", event = "wait_server_read_failed",
                    log = %log, polls, error = %e, "wait_server could not read the server log");
                return fail(rec, e);
            }
            warn_if_clipped(&out.json, &log);
            let mut newest = cursor;
            let mut fresh: Vec<Line> = Vec::new();
            for l in entries(&out.json).iter().filter_map(Line::of) {
                if l.ts < floor(cursor)
                    || seen.contains(&l.key())
                    || !l.named_by(&log, target.as_deref())
                {
                    continue;
                }
                newest = newest.max(l.ts);
                fresh.push(l);
            }
            if let Some(hit) = fresh.iter().find(|l| l.has_fields(&want)) {
                rec.calls.push(summary(true));
                let fields: Map<String, Value> = want
                    .keys()
                    .filter_map(|k| hit.fields.get(k).map(|v| (k.clone(), v.clone())))
                    .collect();
                rec.result = json!({ "matched": {
                    "timestamp_ms": hit.ts, "message": hit.message, "fields": fields,
                }});
                return;
            }
            // Each line is examined once: move the cursor to the newest one
            // seen, remembering the lines it still reaches.
            seen.extend(fresh.iter().map(Line::key));
            if newest > cursor {
                cursor = newest;
                seen.retain(|k| k.0 >= floor(cursor));
            }

            let now = Instant::now();
            if now >= deadline {
                rec.calls.push(summary(true));
                return expire(rec, &log, polls, expired(&log, &want, w.timeout_ms));
            }
            tokio::time::sleep(WAIT_POLL.min(deadline - now)).await;
        }
    }

    /// The wait's `fields` with `${var}`s filled in. An unset
    /// `${player_entity_id}` is read from the client first and kept in
    /// the row's vars.
    async fn wait_fields(
        &self,
        w: &ServerWait,
        who: Who,
        ctx: &mut RowCtx,
        rec: &mut ActionRecord,
    ) -> Result<Map<String, Value>, String> {
        let Some(fields) = &w.fields else {
            return Ok(Map::new());
        };
        let needle = format!("${{{PLAYER_ENTITY_VAR}}}");
        let names_player = fields.values().any(|v| v.as_str() == Some(needle.as_str()));
        if names_player && !ctx.vars.contains_key(PLAYER_ENTITY_VAR) {
            // SV-07: use player_entity
            let id = self.find_player_entity(who, rec).await?;
            ctx.vars.insert(PLAYER_ENTITY_VAR.into(), json!(id));
        }
        let filled = subst(&Value::Object(fields.clone()), &ctx.vars);
        if let Some(v) = unresolved(&filled.to_string()) {
            return Err(format!("unresolved variable {v}"));
        }
        Ok(filled.as_object().cloned().unwrap_or_default())
    }

    /// The client's own player entity id (`client_entity_find`
    /// `/player/id`), the same read SV-07's `player_entity` makes.
    async fn find_player_entity(&self, who: Who, rec: &mut ActionRecord) -> Result<u64, String> {
        let args = json!({ "include_player": true, "limit": 1, "project": false });
        let out = self.on(who).call(FIND_TOOL, args.clone()).await;
        rec.calls
            .push(json!({ "tool": FIND_TOOL, "args": args, "ok": out.ok }));
        if !out.ok {
            return Err(format!("{FIND_TOOL}: {}", out.error.unwrap_or_default()));
        }
        out.json
            .pointer("/player/id")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("{FIND_TOOL} reported no /player/id"))
    }
}
