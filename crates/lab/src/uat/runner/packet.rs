//! Packet clauses (AB-L4): a server packet tap on the lab character's
//! session, started right after the row's anchor and read once and
//! stopped before teardown, on every path that started it. Its rows are
//! kept as the row's `packet_tap` attachment and graded by each
//! `source = "packet"` clause.
//!
//! The tap is a `cimmeria-lab-mcp` tool, so it has the same failure mode
//! as a server clause: an unreachable or unconfigured endpoint makes
//! every packet clause UNVERIFIED with the reason, never PASS.

use serde_json::{json, Value};

use super::actions::subst;
use super::{now_ms, RowCtx, Runner};
use crate::uat::clause::{describe, grade_packet};
use crate::uat::evidence::{clip, ActionRecord, Attachment, ClauseResult, Verdict};
use crate::uat::invoke::{ServerInvoker, ToolInvoker, ToolOutcome};
use crate::uat::spec::{ExpectSpec, RowSpec, Source};
use crate::uat::tier::Role;

/// Ring size asked for: a row of casts and their fan-out fits; the
/// server clamps it to its own ceiling (10 000).
pub(crate) const TAP_CAPACITY: u32 = 5000;
/// A spec or caller var that names the session's player entity, skipping
/// the `server_sessions` lookup. The runner also sets it once resolved,
/// so a clause can filter with `entity = "${player_entity_id}"`.
pub(crate) const ENTITY_VAR: &str = "player_entity_id";

/// One row's tap: the session it is on, or why there is none, and what
/// the read returned.
#[derive(Debug, Default)]
pub(crate) struct RowTap {
    pub entity: Option<u32>,
    pub error: Option<String>,
    pub read: Option<Value>,
    /// Why the tap could not be stopped (after one retry). A tap still
    /// running may have dropped or mixed in rows, so no clause passes.
    pub stop_error: Option<String>,
}

/// A var holding an entity id, as a number or a numeric string.
fn as_entity(v: &Value) -> Option<u32> {
    match v {
        Value::Number(n) => n.as_u64().and_then(|n| u32::try_from(n).ok()),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// The one session whose character is `want`. The server lists the
/// character's name; a fresh character may be listed by its full name, so
/// any whole word of it also matches.
pub(crate) fn find_session(sessions: &Value, want: &str) -> Result<u32, String> {
    let list = sessions
        .get("sessions")
        .and_then(Value::as_array)
        .ok_or("server_sessions returned no sessions list")?;
    let hits: Vec<u32> = list
        .iter()
        .filter(|s| {
            s.get("name").and_then(Value::as_str).is_some_and(|n| {
                n.eq_ignore_ascii_case(want)
                    || n.split_whitespace().any(|w| w.eq_ignore_ascii_case(want))
            })
        })
        .filter_map(|s| s.get("entity_id").and_then(as_entity))
        .collect();
    match hits.as_slice() {
        [one] => Ok(*one),
        [] => Err(format!(
            "no in-world session for character {want:?} in server_sessions"
        )),
        many => Err(format!(
            "{} sessions match character {want:?}; pass vars.{ENTITY_VAR}",
            many.len()
        )),
    }
}

fn server_record(index: usize, role: Role, tool: &str, args: &Value) -> ActionRecord {
    ActionRecord {
        index,
        role,
        label: None,
        kind: "server".into(),
        requested: tool.into(),
        tool: Some(tool.into()),
        args: args.clone(),
        tier: None,
        tier_source: Some("server".into()),
        fallback_used: false,
        host_started_ms: now_ms(),
        elapsed_ms: 0,
        ok: true,
        error: None,
        result: Value::Null,
        calls: vec![],
        client: None,
    }
}

impl<I: ToolInvoker> Runner<'_, I> {
    /// A server tool call recorded on the row (tap start in setup, read
    /// and stop in teardown), so the bundle shows every server touch.
    async fn server_call(
        &self,
        server: &dyn ServerInvoker,
        role: Role,
        tool: &str,
        args: Value,
        ctx: &mut RowCtx,
    ) -> ToolOutcome {
        let mut rec = server_record(ctx.actions.len(), role, tool, &args);
        let t0 = std::time::Instant::now();
        let out = server.call(tool, args).await;
        rec.elapsed_ms = t0.elapsed().as_millis() as u64;
        rec.ok = out.ok;
        rec.error.clone_from(&out.error);
        // The read's rows go to the attachment, not into run.json.
        rec.result = if tool == "server_packet_tap_read" {
            json!({ "count": out.json.get("count"), "dropped": out.json.get("dropped") })
        } else {
            clip(&out.json, 500)
        };
        ctx.actions.push(rec);
        out
    }

    /// The player entity the tap goes on: `vars.player_entity_id`, else
    /// the character's row in `server_sessions`.
    pub(crate) async fn tap_entity(
        &self,
        server: &dyn ServerInvoker,
        ctx: &mut RowCtx,
    ) -> Result<u32, String> {
        if let Some(e) = ctx.vars.get(ENTITY_VAR).and_then(as_entity) {
            return Ok(e);
        }
        let want = ctx
            .character
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or("no character name to find the session by")?;
        let out = self
            .server_call(server, Role::Setup, "server_sessions", json!({}), ctx)
            .await;
        if !out.ok {
            return Err(format!("server lab MCP: {}", out.error.unwrap_or_default()));
        }
        find_session(&out.json, &want)
    }

    /// Start the row's tap. Never fails the row: a problem is kept and
    /// every packet clause reports it.
    pub(crate) async fn tap_start(&mut self, ctx: &mut RowCtx) {
        let Some(server) = self.server else {
            ctx.tap = Some(RowTap {
                error: Some(
                    "server lab MCP not configured (CIMMERIA_LAB_MCP_URL/_TOKEN): no packet tap"
                        .into(),
                ),
                ..Default::default()
            });
            return;
        };
        let mut tap = RowTap::default();
        match self.tap_entity(server, ctx).await {
            Ok(entity) => {
                ctx.vars.insert(ENTITY_VAR.into(), json!(entity));
                let args = json!({ "entity_id": entity, "capacity": TAP_CAPACITY });
                let out = self
                    .server_call(server, Role::Setup, "server_packet_tap_start", args, ctx)
                    .await;
                if out.ok {
                    tap.entity = Some(entity);
                } else {
                    tap.error = Some(format!(
                        "server_packet_tap_start: {}",
                        out.error.unwrap_or_default()
                    ));
                }
            }
            Err(e) => tap.error = Some(format!("packet tap not started: {e}")),
        }
        ctx.tap = Some(tap);
    }

    /// Read the tap and stop it. Called on every path after a start that
    /// reached the server, so a failed row never leaves a tap running.
    pub(crate) async fn tap_finish(&mut self, ctx: &mut RowCtx) {
        let (Some(server), Some(entity)) = (self.server, ctx.tap.as_ref().and_then(|t| t.entity))
        else {
            return;
        };
        let args = json!({ "entity_id": entity });
        let read = self
            .server_call(
                server,
                Role::Teardown,
                "server_packet_tap_read",
                args.clone(),
                ctx,
            )
            .await;
        // One retry: a timeout or a transient refusal must not leave the
        // tap buffering this session.
        let mut stop = self
            .server_call(
                server,
                Role::Teardown,
                "server_packet_tap_stop",
                args.clone(),
                ctx,
            )
            .await;
        if !stop.ok {
            stop = self
                .server_call(server, Role::Teardown, "server_packet_tap_stop", args, ctx)
                .await;
        }
        let path = self
            .run
            .attach_dir(&ctx.section, &ctx.row_id)
            .join("packet-tap.json");
        let body = json!({
            "entity_id": entity,
            "read_ok": read.ok,
            "read_error": read.error,
            "stopped": stop.ok,
            "stop_error": stop.error,
            "tap": read.json,
        });
        if self.run.write_json(&path, &body).is_ok() {
            ctx.attachments.push(Attachment {
                name: "packet_tap".into(),
                path: self.rel(&path),
                tool: "server_packet_tap_read".into(),
                host_ms: now_ms(),
            });
        }
        if let Some(tap) = ctx.tap.as_mut() {
            if !stop.ok {
                tap.stop_error = Some(format!(
                    "server_packet_tap_stop failed twice: {}",
                    stop.error.clone().unwrap_or_default()
                ));
            }
            if read.ok {
                tap.read = Some(read.json);
            } else {
                tap.error = Some(format!(
                    "server_packet_tap_read: {}",
                    read.error.unwrap_or_default()
                ));
            }
        }
    }

    /// Grade every packet clause from the one tap read.
    pub(crate) fn packet_clauses(
        &self,
        row: &RowSpec,
        ctx: &RowCtx,
        results: &mut [Option<ClauseResult>],
    ) {
        for (i, c) in row.expect.iter().enumerate() {
            if c.source == Source::Packet && results[i].is_none() {
                results[i] = Some(packet_clause(c, ctx));
            }
        }
    }
}

fn packet_clause(c: &ExpectSpec, ctx: &RowCtx) -> ClauseResult {
    let mut r = ClauseResult {
        id: c.id.clone(),
        text: c.text.clone(),
        source: c.source,
        required: c.required,
        expected: describe(c),
        verdict: Verdict::Unverified,
        observed: Value::Null,
        detail: None,
        evaluated_ms: now_ms(),
        query: None,
        client: None,
        evidence_refs: ctx
            .attachments
            .iter()
            .filter(|a| a.name == "packet_tap")
            .map(|a| a.path.clone())
            .collect(),
    };
    let tap = ctx.tap.as_ref();
    let Some(read) = tap.and_then(|t| t.read.as_ref()) else {
        r.detail = Some(
            tap.and_then(|t| t.error.clone())
                .unwrap_or_else(|| "the packet tap was not read".into()),
        );
        return r;
    };
    let entity = match &c.entity {
        None => None,
        Some(v) => match as_entity(&subst(v, &ctx.vars)) {
            Some(e) => Some(u64::from(e)),
            None => {
                r.detail = Some(format!("entity {v} is not an entity id"));
                return r;
            }
        },
    };
    let (verdict, observed, detail) = grade_packet(c, entity, read);
    r.verdict = verdict;
    r.observed = observed;
    r.detail = detail;
    // A tap that would not stop cannot back a PASS; a FAIL observed in
    // the read still stands.
    if let Some(e) = tap.and_then(|t| t.stop_error.as_ref()) {
        if r.verdict == Verdict::Pass {
            r.verdict = Verdict::Unverified;
            r.detail = Some(e.clone());
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_is_found_by_name_or_name_word() {
        let s = json!({ "count": 2, "sessions": [
            { "entity_id": 7, "name": "Labone" },
            { "entity_id": 9, "name": "Uat Dqzkfma" },
        ]});
        assert_eq!(find_session(&s, "labone").unwrap(), 7);
        assert_eq!(find_session(&s, "Dqzkfma").unwrap(), 9);
        assert!(find_session(&s, "Labtwo")
            .unwrap_err()
            .contains("no in-world"));
        let dup = json!({ "sessions": [
            { "entity_id": 7, "name": "Labone" }, { "entity_id": 8, "name": "Labone" },
        ]});
        assert!(find_session(&dup, "Labone")
            .unwrap_err()
            .contains("2 sessions"));
    }
}
