//! `wait_server` (D-SV5): wait for a server log line over the server lab
//! MCP's `server_log_tail` instead of sleeping.
//!
//! The watermark is the server's newest log timestamp *before* the action
//! that should cause the line ran ([`Runner::log_mark_for`], called from
//! `drive_steps`), so a fast line is not missed. Two traps from SV-02 shape
//! the polling:
//!
//! - `since_ms` is strictly after, at 1 ms resolution. A line logged in the
//!   watermark's own millisecond, after the mark was read, has
//!   `timestamp_ms == W`. So every poll asks from `W - 1` and skips the
//!   lines it has already examined at `W`.
//! - Every lab-mcp call writes a `lab.tool_call` audit row into the same
//!   ring. A poll always passes `contains`, and the runner drops audit rows
//!   itself, so the wait's own polls can never satisfy it.
//!
//! The runner re-applies every filter to the reply, because a server older
//! than SV-02 ignores `target`, `contains` and `since_ms`.

use std::collections::HashSet;
use std::time::Duration;

use serde_json::{json, Map, Value};

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
/// Lines one poll asks for.
const POLL_LIMIT: u32 = 50;
/// The lab endpoint's own audit target: never a line a wait is for.
const AUDIT_TARGET: &str = "lab.tool_call";
/// The var a field can name to mean "this client's own player entity".
pub(crate) const PLAYER_ENTITY_VAR: &str = "player_entity_id";
/// The client read that knows the player's entity id.
const FIND_TOOL: &str = "client_entity_find";

/// A watermark: the server's newest log timestamp at one moment, and the
/// lines matching the wait that were already there in that millisecond.
#[derive(Debug, Clone, Default)]
pub(crate) struct LogMark {
    pub newest_ms: u64,
    seen: HashSet<String>,
    /// The `server_log_tail` call that read it, for the action's `calls`.
    call: Value,
}

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

    /// Identity for the same-millisecond de-duplication. Two identical
    /// lines in one millisecond, one before the mark and one after, count
    /// as one: the after line is missed. That needs the same message with
    /// the same fields twice in 1 ms, which a wait's line does not do.
    fn key(&self) -> String {
        format!(
            "{}\u{1f}{}\u{1f}{}\u{1f}{}",
            self.ts, self.target, self.message, self.fields
        )
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

/// `server_log_tail` args for a wait: always `contains`, so the poll's
/// own audit rows are never what it reads (SV-02).
fn tail_args(log: &str, target: Option<&str>, since_ms: Option<u64>, limit: u32) -> Value {
    let mut args = json!({ "contains": log, "limit": limit });
    if let Some(t) = target {
        args["target"] = json!(t);
    }
    if let Some(s) = since_ms {
        args["since_ms"] = json!(s);
    }
    args
}

/// Read a watermark for `w` now.
async fn read_mark(
    server: &dyn ServerInvoker,
    w: &ServerWait,
    vars: &Map<String, Value>,
) -> Result<LogMark, String> {
    let log = subst_str(&w.log, vars);
    let target = w.target.as_ref().map(|t| subst_str(t, vars));
    let args = tail_args(&log, target.as_deref(), None, POLL_LIMIT);
    let out = server.call(LOG_TAIL_TOOL, args.clone()).await;
    if !out.ok {
        return Err(format!(
            "{LOG_TAIL_TOOL}: {}",
            out.error.unwrap_or_default()
        ));
    }
    // An empty ring has no `newest_ms`: every line from now on is new.
    let newest_ms = out
        .json
        .get("newest_ms")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let seen = entries(&out.json)
        .iter()
        .filter_map(Line::of)
        .filter(|l| l.ts == newest_ms && l.named_by(&log, target.as_deref()))
        .map(|l| l.key())
        .collect();
    Ok(LogMark {
        newest_ms,
        seen,
        call: json!({ "tool": LOG_TAIL_TOOL, "args": args, "ok": true, "mark_ms": newest_ms }),
    })
}

impl<I: ToolInvoker> Runner<'_, I> {
    /// The watermark for `next` when it is a `wait_server`, read before
    /// the action in front of it runs. `drive_steps` hands it to the wait
    /// through `RowCtx::log_mark` once that action has run. `None` when
    /// the server cannot be read: the wait then takes its own at its start
    /// (and fails there if the server is still unreadable).
    pub(crate) async fn log_mark_for(
        &self,
        next: Option<&ActionSpec>,
        ctx: &RowCtx,
    ) -> Option<LogMark> {
        let w = next?.wait_server.as_ref()?;
        read_mark(self.server?, w, &ctx.vars).await.ok()
    }

    pub(crate) async fn exec_server_wait(
        &self,
        w: &ServerWait,
        who: Who,
        ctx: &mut RowCtx,
        rec: &mut ActionRecord,
    ) {
        rec.kind = "wait_server".into();
        let log = subst_str(&w.log, &ctx.vars);
        let target = w.target.as_ref().map(|t| subst_str(t, &ctx.vars));
        rec.requested = format!("wait for server log '{log}'");
        // Taken before the previous action, or here for a row's first one.
        let mark = ctx.log_mark.take();
        let Some(server) = self.server else {
            return fail(rec, "wait_server needs the server lab MCP".into());
        };
        let want = match self.wait_fields(w, who, ctx, rec).await {
            Ok(f) => f,
            Err(e) => return fail(rec, e),
        };
        let mark = match mark {
            Some(m) => m,
            None => match read_mark(server, w, &ctx.vars).await {
                Ok(m) => m,
                Err(e) => return fail(rec, e),
            },
        };
        rec.calls.push(mark.call.clone());
        rec.args =
            json!({ "log": log, "target": target, "fields": want, "timeout_ms": w.timeout_ms });

        // Lines at `cursor` that were already examined; older ones are
        // excluded by `since_ms` (and by the refilter, for an old server).
        let mut cursor = mark.newest_ms;
        let mut seen = mark.seen;
        let deadline = tokio::time::Instant::now() + Duration::from_millis(w.timeout_ms);
        let mut polls = 0u32;
        loop {
            polls += 1;
            let since = cursor.checked_sub(1);
            let args = tail_args(&log, target.as_deref(), since, POLL_LIMIT);
            let out = server.call(LOG_TAIL_TOOL, args.clone()).await;
            if !out.ok {
                rec.calls.push(
                    json!({ "tool": LOG_TAIL_TOOL, "args": args, "ok": false, "polls": polls }),
                );
                return fail(
                    rec,
                    format!("{LOG_TAIL_TOOL}: {}", out.error.unwrap_or_default()),
                );
            }
            let mut newest = cursor;
            let mut fresh: Vec<Line> = Vec::new();
            for l in entries(&out.json).iter().filter_map(Line::of) {
                if l.ts < cursor
                    || (l.ts == cursor && seen.contains(&l.key()))
                    || !l.named_by(&log, target.as_deref())
                {
                    continue;
                }
                newest = newest.max(l.ts);
                fresh.push(l);
            }
            if let Some(hit) = fresh.iter().find(|l| l.has_fields(&want)) {
                rec.calls.push(
                    json!({ "tool": LOG_TAIL_TOOL, "args": args, "ok": true, "polls": polls }),
                );
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
            // seen, remembering the lines in that millisecond.
            if newest > cursor {
                seen.clear();
                cursor = newest;
            }
            seen.extend(fresh.iter().filter(|l| l.ts == cursor).map(|l| l.key()));

            let now = tokio::time::Instant::now();
            if now >= deadline {
                rec.calls.push(
                    json!({ "tool": LOG_TAIL_TOOL, "args": args, "ok": true, "polls": polls }),
                );
                let fields = if want.is_empty() {
                    String::new()
                } else {
                    format!(" {}", Value::Object(want))
                };
                return fail(
                    rec,
                    format!("no server log '{log}'{fields} within {} ms", w.timeout_ms),
                );
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
