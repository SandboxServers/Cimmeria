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
use super::actions::RING_CLICK_TOOL;
use super::{now_ms, RowCtx, Runner};
use crate::uat::clause::{describe, grade_packet};
use crate::uat::evidence::{clip, ActionRecord, Attachment, ClauseResult, Verdict};
use crate::uat::invoke::{ServerInvoker, ToolInvoker, ToolOutcome};
use crate::uat::spec::{ExpectSpec, RowSpec, Source};
use crate::uat::tier::{Role, Tier};

/// Ring size asked for: a row of casts and their fan-out fits; the
/// server clamps it to its own ceiling (10 000).
pub(crate) const TAP_CAPACITY: u32 = 5000;
/// A spec or caller var that names the session's player entity, skipping
/// the `server_sessions` lookup. The runner also sets it once resolved,
/// so a clause can filter with `entity = "${player_entity_id}"`.
pub(crate) const ENTITY_VAR: &str = "player_entity_id";
/// Why no tap: the runner has no server lab MCP.
pub(crate) const NOT_CONFIGURED: &str =
    "server lab MCP not configured (CIMMERIA_LAB_MCP_URL/_TOKEN): no packet tap";

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
    /// `server_packet_tap_start` replaced a tap already on the entity (an
    /// observer's `lab_timeline`, README F3): its rows are gone.
    pub replaced_existing: bool,
    /// `tap_finish` ran to the end: the stop was tried and the attachment
    /// written. A lost lease can cut it off before that ([`Runner::tap_abandon`]).
    pub finished: bool,
}

/// A var holding an entity id, as a number or a numeric string.
pub(super) fn as_entity(v: &Value) -> Option<u32> {
    match v {
        Value::Number(n) => n.as_u64().and_then(|n| u32::try_from(n).ok()),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// `server_packet_tap_start`'s `replaced_existing` (false when absent).
pub(super) fn replaced_existing(reply: &Value) -> bool {
    reply
        .get("replaced_existing")
        .and_then(Value::as_bool)
        .unwrap_or(false)
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

/// One server tool call, timed, with the action record it makes. The
/// caller decides where the record goes (the row, or fingerprint mode's
/// deferred list).
pub(super) async fn timed_call(
    server: &dyn ServerInvoker,
    role: Role,
    tool: &str,
    args: Value,
) -> (ActionRecord, ToolOutcome) {
    let mut rec = server_record(0, role, tool, &args);
    let t0 = std::time::Instant::now();
    let out = server.call(tool, args).await;
    rec.elapsed_ms = t0.elapsed().as_millis() as u64;
    rec.ok = out.ok;
    rec.error.clone_from(&out.error);
    rec.result = clip(&out.json, 500);
    (rec, out)
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
        optional: false,
    }
}

impl<I: ToolInvoker> Runner<'_, I> {
    /// A server tool call recorded on the row (tap start in setup, read
    /// and stop in teardown), so the bundle shows every server touch.
    pub(crate) async fn server_call(
        &self,
        server: &dyn ServerInvoker,
        role: Role,
        tool: &str,
        args: Value,
        ctx: &mut RowCtx,
    ) -> ToolOutcome {
        let (mut rec, out) = timed_call(server, role, tool, args).await;
        rec.index = ctx.actions.len();
        // The read's rows go to the attachment, not into run.json.
        if tool == "server_packet_tap_read" {
            rec.result =
                json!({ "count": out.json.get("count"), "dropped": out.json.get("dropped") });
        }
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
                error: Some(NOT_CONFIGURED.into()),
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
                    tap.replaced_existing = replaced_existing(&out.json);
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
        let body = json!({
            "entity_id": entity,
            "read_ok": read.ok,
            "read_error": read.error,
            "stopped": stop.ok,
            "stop_error": stop.error,
            "tap": read.json,
        });
        self.attach_tap(ctx, body);
        if let Some(tap) = ctx.tap.as_mut() {
            tap.finished = true;
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

    /// Write `packet-tap.json`, adding fingerprint mode's coverage fields.
    fn attach_tap(&self, ctx: &mut RowCtx, mut body: Value) {
        if let Some(fp) = &ctx.fp {
            body["coverage"] = json!(fp.coverage(true));
            body["replaced_existing"] =
                json!(ctx.tap.as_ref().is_some_and(|t| t.replaced_existing));
        }
        let path = self
            .run
            .attach_dir(&ctx.section, &ctx.row_id)
            .join("packet-tap.json");
        if self.run.write_json(&path, &body).is_ok() {
            ctx.attachments.push(Attachment {
                name: "packet_tap".into(),
                path: self.rel(&path),
                tool: "server_packet_tap_read".into(),
                host_ms: now_ms(),
            });
        }
    }

    /// After a lost lease cut the row off: stop a tap `tap_finish` never
    /// finished, then write fingerprint mode's attachments. The lab MCP is
    /// not lease-gated, and a tap left running makes the next run's start
    /// report `replaced_existing`, which a golden run refuses. One stop,
    /// best effort; the tap is not read (the row is BLOCKED anyway).
    pub(crate) async fn tap_abandon(&mut self, ctx: &mut RowCtx) {
        let running = ctx
            .tap
            .as_ref()
            .filter(|t| !t.finished)
            .and_then(|t| t.entity);
        if let (Some(server), Some(entity)) = (self.server, running) {
            let args = json!({ "entity_id": entity });
            let stop = self
                .server_call(server, Role::Teardown, "server_packet_tap_stop", args, ctx)
                .await;
            let body = json!({
                "entity_id": entity,
                "read_ok": false,
                "read_error": "lease revoked before the tap was read",
                "stopped": stop.ok,
                "stop_error": stop.error,
                "tap": null,
            });
            self.attach_tap(ctx, body);
            if let Some(tap) = ctx.tap.as_mut() {
                tap.finished = true;
            }
        }
        self.fp_write(ctx);
    }

    /// Grade every packet clause from the one tap read.
    pub(crate) fn packet_clauses(
        &self,
        row: &RowSpec,
        ctx: &mut RowCtx,
        results: &mut [Option<ClauseResult>],
    ) {
        for (i, c) in row.expect.iter().enumerate() {
            if c.source == Source::Packet && results[i].is_none() {
                results[i] = Some(packet_clause(c, ctx));
            }
        }
        promote_ring_candidates(&mut ctx.actions, &row.expect, results);
    }
}

/// The row tap is the only source of server proof. A closed map without a
/// required, positive inbound method-91 clause remains an N3 observation.
fn promote_ring_candidates(
    actions: &mut [ActionRecord],
    expects: &[ExpectSpec],
    results: &[Option<ClauseResult>],
) {
    let proven = expects.iter().zip(results).any(|(c, r)| {
        c.required
            && c.source == Source::Packet
            && c.direction.as_deref() == Some("to_server")
            && c.message.as_deref() == Some("setRingTransporterDestination")
            && c.min_rows.is_some_and(|n| n >= 1)
            && r.as_ref().is_some_and(|r| {
                r.verdict == Verdict::Pass
                    && r.observed["matching_rows"].as_u64().is_some_and(|n| n >= 1)
            })
    });
    if !proven {
        return;
    }
    for a in actions {
        if a.role == Role::Step
            && a.client.is_none()
            && a.tool.as_deref() == Some(RING_CLICK_TOOL)
            && a.ok
            && a.tier_source.as_deref() == Some("ring:awaiting_method91")
            && a.result["native_candidate"] == json!(true)
        {
            a.tier = Some(Tier::N1);
            a.tier_source = Some("ring:packet_method91".into());
            a.result["packet_verified"] = json!(true);
            a.result["native_pass"] = json!(true);
        }
    }
}

fn packet_clause(c: &ExpectSpec, ctx: &RowCtx) -> ClauseResult {
    let c = &super::clauses::with_vars(c, &ctx.vars);
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
    use crate::uat::evidence::RowResult;
    use crate::uat::grade::grade;

    fn ring_action() -> ActionRecord {
        let mut action = ActionRecord {
            index: 0,
            role: Role::Step,
            label: Some("pick".into()),
            kind: "tool".into(),
            requested: RING_CLICK_TOOL.into(),
            tool: Some(RING_CLICK_TOOL.into()),
            args: json!({ "x": 652, "y": 398 }),
            tier: Some(Tier::N1),
            tier_source: Some("tool".into()),
            fallback_used: false,
            host_started_ms: 0,
            elapsed_ms: 1,
            ok: true,
            error: None,
            result: json!({ "map_closed": true, "native_candidate": true, "native_pass": false }),
            calls: vec![],
            client: None,
            optional: false,
        };
        super::super::actions::hold_ring_click(&mut action, RING_CLICK_TOOL);
        action
    }

    fn ring_clause(min_rows: Option<u64>) -> ExpectSpec {
        let min = min_rows.map_or(String::new(), |n| format!("min_rows = {n}\n"));
        toml::from_str(&format!(
            "id = 'ring'\ntext = 'destination sent'\nsource = 'packet'\nmessage = 'setRingTransporterDestination'\ndirection = 'to_server'\n{min}"
        ))
        .unwrap()
    }

    fn packet_result(verdict: Verdict, rows: u64) -> Option<ClauseResult> {
        Some(ClauseResult {
            id: "ring".into(),
            text: "destination sent".into(),
            source: Source::Packet,
            required: true,
            expected: "packet to_server setRingTransporterDestination".into(),
            verdict,
            observed: json!({ "matching_rows": rows }),
            detail: None,
            evaluated_ms: 0,
            query: None,
            evidence_refs: vec![],
            client: None,
        })
    }

    #[test]
    fn closed_ring_map_without_packet_does_not_become_an_n1_pass() {
        let mut actions = vec![ring_action()];
        promote_ring_candidates(&mut actions, &[], &[]);
        assert_eq!(actions[0].tier, Some(Tier::N3));
        assert_eq!(actions[0].result["native_pass"], false);
        assert_eq!(
            grade(&[], &actions, &[], Tier::N1).result,
            RowResult::NativeShortfall
        );

        promote_ring_candidates(
            &mut actions,
            &[ring_clause(Some(1))],
            &[packet_result(Verdict::Fail, 0)],
        );
        assert_eq!(actions[0].tier, Some(Tier::N3));
        assert_eq!(actions[0].result["native_pass"], false);

        // A clause explicitly allowing zero rows cannot prove delivery.
        promote_ring_candidates(
            &mut actions,
            &[ring_clause(Some(0))],
            &[packet_result(Verdict::Pass, 0)],
        );
        assert_eq!(actions[0].tier, Some(Tier::N3));
    }

    #[test]
    fn required_inbound_method_91_promotes_only_a_native_candidate() {
        let clause = ring_clause(Some(1));
        let result = packet_result(Verdict::Pass, 1);
        let mut actions = vec![ring_action()];
        promote_ring_candidates(
            &mut actions,
            std::slice::from_ref(&clause),
            std::slice::from_ref(&result),
        );
        assert_eq!(actions[0].tier, Some(Tier::N1));
        assert_eq!(actions[0].result["native_pass"], true);
        assert_eq!(actions[0].result["packet_verified"], true);
        assert_eq!(grade(&[], &actions, &[], Tier::N1).result, RowResult::Pass);

        let mut closed_via_lua = ring_action();
        closed_via_lua.result["native_candidate"] = json!(false);
        let mut actions = vec![closed_via_lua];
        promote_ring_candidates(&mut actions, &[clause], &[result]);
        assert_eq!(actions[0].tier, Some(Tier::N3));
        assert_eq!(actions[0].result["native_pass"], false);
    }

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
