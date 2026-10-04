//! Evaluating one expected clause against the live client (or queueing
//! it for attestation).

use serde_json::{json, Value};

use super::actions::{subst, subst_str, CHAT_READ_TOOL};
use super::players::Who;
use super::{now_ms, utc_of, RowCtx, Runner};
use crate::uat::clause::{compare_tol, describe, eval_chat, json_at, new_lines};
use crate::uat::evidence::{clip, ClauseResult, Verdict};
use crate::uat::invoke::ToolInvoker;
use crate::uat::spec::{ExpectSpec, Source};

/// How far around the row the SigNoz window reaches: a little before the
/// anchor, and long enough after for batched log export to land.
const SIGNOZ_BEFORE_MS: i64 = 10_000;
const SIGNOZ_AFTER_MS: i64 = 120_000;

impl<I: ToolInvoker> Runner<'_, I> {
    pub(crate) async fn eval_clause(&mut self, c: &ExpectSpec, ctx: &mut RowCtx) -> ClauseResult {
        let who = Who::of(c.client.as_deref());
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
            client: who.tag(),
            evidence_refs: vec![],
        };
        match c.source {
            Source::Chat => self.eval_chat_clause(who, c, ctx, &mut r).await,
            Source::Tool => {
                let tool = c.tool.clone().unwrap_or_default();
                let args = subst(c.args.as_ref().unwrap_or(&json!({})), &ctx.vars);
                self.eval_read(who, &tool, args, c, &mut r).await;
            }
            Source::Lua => {
                let chunk = subst_str(c.chunk.as_deref().unwrap_or_default(), &ctx.vars);
                self.eval_lua(who, &chunk, c, &mut r).await;
            }
            Source::Wait => {
                let cond = subst_str(c.lua_condition.as_deref().unwrap_or_default(), &ctx.vars);
                let args =
                    json!({ "lua_condition": cond, "timeout_ms": c.timeout_ms.unwrap_or(10_000) });
                let out = self.on(who).call("client_wait_for", args).await;
                if out.ok {
                    let met = out
                        .json
                        .get("met")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    r.verdict = if met { Verdict::Pass } else { Verdict::Fail };
                    r.observed = clip(&out.json, 500);
                } else {
                    r.detail = out.error;
                }
            }
            Source::Timing => {
                let label = c.action.as_deref().unwrap_or_default();
                match ctx
                    .actions
                    .iter()
                    .find(|a| a.label.as_deref() == Some(label))
                {
                    Some(a) => {
                        let ok = a.ok && a.elapsed_ms <= c.max_ms.unwrap_or(0);
                        r.verdict = if ok { Verdict::Pass } else { Verdict::Fail };
                        r.observed = json!({ "elapsed_ms": a.elapsed_ms, "action_ok": a.ok });
                    }
                    None => r.detail = Some(format!("action {label} did not run")),
                }
            }
            Source::Signoz => {
                r.verdict = Verdict::Pending;
                let from = ctx.started_ms - SIGNOZ_BEFORE_MS;
                let to = now_ms() + SIGNOZ_AFTER_MS;
                let mut filter = subst_str(c.filter.as_deref().unwrap_or_default(), &ctx.vars);
                if let Some(v) = self
                    .manifest
                    .server
                    .get("service_version")
                    .and_then(Value::as_str)
                {
                    if !filter.contains("service.version") {
                        filter = format!("service.version = '{v}' AND ({filter})");
                    }
                }
                r.query = Some(json!({
                    "base": "service.name = 'cimmeria-server'",
                    "filter": filter,
                    "from_utc": utc_of(from),
                    "to_utc": utc_of(to),
                    "from_ms": from,
                    "to_ms": to,
                    "bookmark_id": ctx.anchor.as_ref().and_then(|a| a.bookmark_id),
                    // Kept so lab_uat_attest grades the attested rows by
                    // the spec's rules without re-reading the spec.
                    "grading": {
                        "min_rows": c.min_rows, "max_rows": c.max_rows,
                        "field": c.field, "op": c.op, "value": c.value, "tolerance": c.tolerance,
                    },
                }));
                r.detail =
                    Some("run the query, then lab_uat_attest the row count and key rows".into());
            }
            Source::Server => match self.server {
                Some(server) => {
                    let tool = c.tool.clone().unwrap_or_default();
                    let args = subst(c.args.as_ref().unwrap_or(&json!({})), &ctx.vars);
                    let out = server.call(&tool, args).await;
                    if out.ok {
                        let v = json_at(&out.json, c.pointer.as_deref()).cloned();
                        judge(c, v, &mut r);
                    } else {
                        r.detail =
                            Some(format!("server lab MCP: {}", out.error.unwrap_or_default()));
                    }
                }
                None => {
                    r.detail = Some(
                        "server lab MCP not configured (CIMMERIA_LAB_MCP_URL/_TOKEN); check the SigNoz clauses instead"
                            .into(),
                    );
                }
            },
            Source::Packet => {
                // Graded from the tap read before teardown (`packet.rs`).
                r.detail = Some("a packet clause is graded from the row's tap".into());
            }
            Source::Human => {
                r.verdict = Verdict::NeedsHuman;
                r.evidence_refs = ctx.attachments.iter().map(|a| a.path.clone()).collect();
            }
        }
        r
    }

    async fn eval_chat_clause(
        &mut self,
        who: Who,
        c: &ExpectSpec,
        ctx: &mut RowCtx,
        r: &mut ClauseResult,
    ) {
        let after = match self.read_chat_of(who).await {
            Ok(a) => a,
            Err(e) => {
                r.detail = Some(format!("{CHAT_READ_TOOL}: {e}"));
                return;
            }
        };
        let key = who.mark(c.since.as_deref().unwrap_or_default());
        let before = ctx.chat_marks.get(&key).cloned().unwrap_or_default();
        let (lines, overlap) = new_lines(&before, &after);
        // Variables may appear in the pattern (`' - ${character} \('`).
        let mut c = c.clone();
        c.contains = c.contains.map(|s| subst_str(&s, &ctx.vars));
        c.matches = c.matches.map(|s| subst_str(&s, &ctx.vars));
        let (verdict, mut observed, captured) = eval_chat(&c, &lines);
        if !overlap {
            r.detail = Some(
                "the chat box was cleared or overflowed: every visible line was counted".into(),
            );
        }
        if let (Some(var), Some(v)) = (&c.capture_var, captured) {
            ctx.vars.insert(var.clone(), json!(v));
            observed["captured"] = json!({ var: v });
        }
        r.verdict = verdict;
        r.observed = observed;
    }

    async fn eval_read(
        &mut self,
        who: Who,
        tool: &str,
        args: Value,
        c: &ExpectSpec,
        r: &mut ClauseResult,
    ) {
        if !self.on(who).has_tool(tool) {
            r.verdict = Verdict::Blocked;
            r.detail = Some(format!("tool {tool} is not routed"));
            return;
        }
        let out = self.on(who).call(tool, args).await;
        if !out.ok {
            r.detail = Some(format!("{tool}: {}", out.error.unwrap_or_default()));
            return;
        }
        let v = json_at(&out.json, c.pointer.as_deref()).cloned();
        judge(c, v, r);
    }

    /// A Lua read: its first result, parsed as JSON when it is JSON. A
    /// Lua error means the reader broke, not the game: UNVERIFIED.
    async fn eval_lua(&mut self, who: Who, chunk: &str, c: &ExpectSpec, r: &mut ClauseResult) {
        let inv = self.on(who);
        if !inv.has_tool("client_lua_eval") {
            r.verdict = Verdict::Blocked;
            r.detail = Some("tool client_lua_eval is not routed".into());
            return;
        }
        let out = inv.call("client_lua_eval", json!({ "chunk": chunk })).await;
        let ok = out.ok && out.json.get("ok").and_then(Value::as_bool).unwrap_or(false);
        if !ok {
            let err = out
                .json
                .get("error")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or(out.error)
                .unwrap_or_default();
            r.detail = Some(format!("Lua read failed: {err}"));
            return;
        }
        let first = out
            .json
            .get("results")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .cloned()
            .map(|v| match &v {
                Value::String(s) => serde_json::from_str(s).unwrap_or(v),
                _ => v,
            });
        let v = match (&first, c.pointer.as_deref()) {
            (Some(f), p) => json_at(f, p).cloned(),
            (None, _) => None,
        };
        judge(c, v, r);
    }
}

/// Compare an observation and set the verdict. Every tool, server and
/// Lua clause comes through here, so `approx` gets its `tolerance`.
fn judge(c: &ExpectSpec, observed: Option<Value>, r: &mut ClauseResult) {
    match compare_tol(c.op, observed.as_ref(), c.value.as_ref(), c.tolerance) {
        Ok(ok) => r.verdict = if ok { Verdict::Pass } else { Verdict::Fail },
        Err(e) => r.detail = Some(e),
    }
    r.observed = observed.map_or(Value::Null, |v| clip(&v, 500));
}
