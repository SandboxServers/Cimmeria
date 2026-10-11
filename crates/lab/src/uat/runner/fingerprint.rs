//! Fingerprint mode (lab-golden GD-06, README D-GD1): the runner only
//! captures raw data into the bundle, and `uat::golden` turns a run
//! directory into a fingerprint offline. Per row it adds:
//!
//! - a packet tap on every row that reaches the world. A row with a packet
//!   clause keeps the tap [`Runner::tap_start`] gives it. Any other row is
//!   tapped here, and when its character is not in the world yet the tap
//!   binds late, after the first action that brings it in (README F2):
//!   the session lookup before every action is silent, and only the start
//!   that succeeds is recorded.
//! - entity snapshots of the player's space when the tap binds and before
//!   it is read (README D-GD3), trimmed to the fields labels use.
//! - a window sample at the start and after every setup and step action
//!   (README F9: the client has no window event).
//!
//! None of it may change a grade. The server calls it makes are recorded
//! as setup actions, but appended to the row's actions only when the row
//! ends, so every step keeps the index the grade's reasons name. It never
//! sets `${player_entity_id}`, which a clause could read, and it never
//! late-binds a row with a packet clause, whose clauses grade the tap.
//! Attachments are listed in the lab-golden contract.

use serde_json::{json, Map, Value};

use super::packet::{
    as_entity, find_session, replaced_existing, timed_call, RowTap, ENTITY_VAR, NOT_CONFIGURED,
};
use super::{now_ms, RowCtx, Runner};
use crate::uat::evidence::{ActionRecord, Attachment};
use crate::uat::invoke::{ServerInvoker, ToolInvoker, ToolOutcome};
use crate::uat::spec::{ActionSpec, RowSpec};
use crate::uat::tier::Role;

/// The window reader (`client_ui_state`'s `windows`).
const WINDOW_TOOL: &str = "client_ui_state";
/// The entity fields a snapshot keeps (contract: entities.json).
const ENTITY_KEEP: [&str; 5] = ["entity_id", "is_player", "tag", "spawn_id", "template_id"];

/// One row's fingerprint-mode capture.
#[derive(Debug, Default)]
pub(crate) struct FpCapture {
    /// `{ after, windows | error }`, in order.
    windows: Vec<Value>,
    /// `{ at, space_id, capped, entities | error }`, in order.
    snapshots: Vec<Value>,
    /// The player's space at the first snapshot that found one.
    space_id: Option<u32>,
    /// The action the tap bound after; `None` when it bound at row start.
    bound_after: Option<String>,
    /// No server lab MCP is configured.
    unavailable: bool,
    /// This capture may start the tap itself: the row has no packet clause,
    /// and no start has failed yet.
    may_bind: bool,
    /// Why the tap is not bound (the last attempt).
    bind_error: Option<String>,
    /// Recorded server calls, appended to the row's actions by `fp_write`.
    actions: Vec<ActionRecord>,
}

impl FpCapture {
    /// `may_bind`: the row has no packet clause, so the tap is this
    /// capture's to start.
    pub(crate) fn new(unavailable: bool, may_bind: bool) -> Self {
        Self {
            unavailable,
            may_bind: may_bind && !unavailable,
            ..Default::default()
        }
    }

    /// The packet-tap body's `coverage`: `full`, `late:<after>`, `none` or
    /// `unavailable`.
    pub(crate) fn coverage(&self, bound: bool) -> String {
        match (&self.bound_after, bound) {
            _ if self.unavailable => "unavailable".into(),
            (_, false) => "none".into(),
            (Some(after), true) => format!("late:{after}"),
            (None, true) => "full".into(),
        }
    }
}

/// An action's `<after>` name: its label, else `<list>#<i>` (0-based).
pub(crate) fn after_name(a: &ActionSpec, list: &str, i: usize) -> String {
    a.label.clone().unwrap_or_else(|| format!("{list}#{i}"))
}

/// One `server_entity_query` entity, trimmed to the labelling fields.
fn trim_entity(e: &Value) -> Value {
    let kept: Map<String, Value> = ENTITY_KEEP
        .iter()
        .filter_map(|k| Some(((*k).to_string(), e.get(*k)?.clone())))
        .collect();
    Value::Object(kept)
}

fn tap_entity_of(ctx: &RowCtx) -> Option<u32> {
    ctx.tap.as_ref().and_then(|t| t.entity)
}

impl<I: ToolInvoker> Runner<'_, I> {
    /// Fingerprint this row: the run asked for it and the row is not
    /// `[row.golden] skip`.
    pub(crate) fn fp_wanted(&self, row: &RowSpec) -> bool {
        self.req.fingerprint && !row.golden.as_ref().is_some_and(|g| g.skip)
    }

    /// Row start, after any packet-clause tap: the `start` window sample,
    /// then the tap (or its `bind` snapshot when `tap_start` bound it).
    pub(crate) async fn fp_start(&mut self, ctx: &mut RowCtx) {
        let Some(may_bind) = ctx.fp.as_ref().map(|fp| fp.may_bind) else {
            return;
        };
        self.fp_sample_windows("start".into(), ctx).await;
        if tap_entity_of(ctx).is_some() {
            self.fp_snapshot("bind", ctx).await;
        } else if may_bind {
            self.fp_bind(None, ctx).await;
        }
    }

    /// After one setup or step action: a window sample, then, while the
    /// tap is unbound, one silent try to bind it.
    pub(crate) async fn fp_after_action(
        &mut self,
        a: &ActionSpec,
        list: &str,
        i: usize,
        ctx: &mut RowCtx,
    ) {
        let Some(may_bind) = ctx.fp.as_ref().map(|fp| fp.may_bind) else {
            return;
        };
        let after = after_name(a, list, i);
        self.fp_sample_windows(after.clone(), ctx).await;
        if may_bind && tap_entity_of(ctx).is_none() {
            self.fp_bind(Some(after), ctx).await;
        }
    }

    /// Before the tap is read: the `finish` snapshot.
    pub(crate) async fn fp_before_tap_read(&mut self, ctx: &mut RowCtx) {
        if ctx.fp.is_some() && tap_entity_of(ctx).is_some() {
            self.fp_snapshot("finish", ctx).await;
        }
    }

    /// Row end: write `windows.json`, `entities.json` and, when no tap
    /// bound, `packet-tap.json` with its coverage; then append the
    /// recorded server calls to the row's actions.
    pub(crate) fn fp_write(&self, ctx: &mut RowCtx) {
        let Some(fp) = ctx.fp.take() else {
            return;
        };
        let player = tap_entity_of(ctx);
        self.fp_attach(
            ctx,
            "windows",
            "windows.json",
            WINDOW_TOOL,
            &json!({ "samples": fp.windows }),
        );
        let mut entities = json!({ "snapshots": fp.snapshots });
        if let Some(p) = player {
            entities["player"] = json!(p);
        }
        if let Some(s) = fp.space_id {
            entities["space_id"] = json!(s);
        }
        self.fp_attach(
            ctx,
            "entities",
            "entities.json",
            "server_entity_query",
            &entities,
        );
        if player.is_none() {
            let error = if fp.unavailable {
                Some(NOT_CONFIGURED.to_string())
            } else {
                fp.bind_error
                    .clone()
                    .or_else(|| ctx.tap.as_ref().and_then(|t| t.error.clone()))
            };
            let body = json!({
                "coverage": fp.coverage(false),
                "replaced_existing": false,
                "error": error,
            });
            self.fp_attach(
                ctx,
                "packet_tap",
                "packet-tap.json",
                "server_packet_tap_start",
                &body,
            );
        }
        for mut rec in fp.actions {
            rec.index = ctx.actions.len();
            ctx.actions.push(rec);
        }
    }

    fn fp_attach(&self, ctx: &mut RowCtx, name: &str, file: &str, tool: &str, body: &Value) {
        let path = self.run.attach_dir(&ctx.section, &ctx.row_id).join(file);
        if self.run.write_json(&path, body).is_ok() {
            ctx.attachments.push(Attachment {
                name: name.into(),
                path: self.rel(&path),
                tool: tool.into(),
                host_ms: now_ms(),
            });
        }
    }

    /// One window sample. Not recorded as an action: `client_ui_state` is
    /// a read, and the grade must not see it.
    async fn fp_sample_windows(&self, after: String, ctx: &mut RowCtx) {
        let sample = if self.inv.has_tool(WINDOW_TOOL) {
            let out = self.inv.call(WINDOW_TOOL, json!({})).await;
            // A failed Lua windows section still answers `windows: []`, with
            // the cause in `errors`: that is an unknown, not "none open".
            let failed = out
                .json
                .get("errors")
                .and_then(Value::as_array)
                .and_then(|es| {
                    es.iter()
                        .filter_map(Value::as_str)
                        .find(|e| e.starts_with("windows:"))
                });
            match (failed, out.json.get("windows").filter(|w| w.is_array())) {
                (Some(e), _) if out.ok => json!({ "after": after, "error": e }),
                (None, Some(w)) if out.ok => json!({ "after": after, "windows": w }),
                _ if out.ok => {
                    json!({ "after": after, "error": format!("{WINDOW_TOOL} returned no windows list") })
                }
                _ => json!({ "after": after, "error": out.error.unwrap_or_default() }),
            }
        } else {
            json!({ "after": after, "error": format!("{WINDOW_TOOL} is not routed") })
        };
        if let Some(fp) = ctx.fp.as_mut() {
            fp.windows.push(sample);
        }
    }

    /// A server call kept on the capture's deferred action list.
    async fn fp_call(
        &self,
        server: &dyn ServerInvoker,
        tool: &str,
        args: Value,
        ctx: &mut RowCtx,
    ) -> ToolOutcome {
        let (mut rec, out) = timed_call(server, Role::Setup, tool, args).await;
        // The entity list goes to the attachment, not into run.json.
        if tool == "server_entity_query" {
            rec.result = json!({
                "returned": out.json.get("returned"),
                "total_matched": out.json.get("total_matched"),
                "capped": out.json.get("capped"),
            });
        }
        if let Some(fp) = ctx.fp.as_mut() {
            fp.actions.push(rec);
        }
        out
    }

    /// The character's session, looked up without recording anything: a
    /// row that enters the world mid-way would otherwise record a miss
    /// after every action before it.
    async fn fp_find_session(
        &self,
        server: &dyn ServerInvoker,
        ctx: &RowCtx,
    ) -> Result<u32, String> {
        if let Some(e) = ctx.vars.get(ENTITY_VAR).and_then(as_entity) {
            return Ok(e);
        }
        let want = ctx
            .character
            .get("name")
            .and_then(Value::as_str)
            .ok_or("no character name to find the session by")?;
        let out = server.call("server_sessions", json!({})).await;
        if !out.ok {
            return Err(format!("server lab MCP: {}", out.error.unwrap_or_default()));
        }
        find_session(&out.json, want)
    }

    /// Try to start the tap; `after` is the action it binds after (`None`
    /// at row start). A failed start is recorded and ends the tries.
    async fn fp_bind(&mut self, after: Option<String>, ctx: &mut RowCtx) {
        let Some(server) = self.server else {
            return;
        };
        let entity = match self.fp_find_session(server, ctx).await {
            Ok(e) => e,
            Err(e) => {
                if let Some(fp) = ctx.fp.as_mut() {
                    fp.bind_error = Some(format!("packet tap not started: {e}"));
                }
                return;
            }
        };
        let args = json!({ "entity_id": entity, "capacity": super::packet::TAP_CAPACITY });
        let out = self
            .fp_call(server, "server_packet_tap_start", args, ctx)
            .await;
        let Some(fp) = ctx.fp.as_mut() else {
            return;
        };
        if !out.ok {
            fp.bind_error = Some(format!(
                "server_packet_tap_start: {}",
                out.error.unwrap_or_default()
            ));
            fp.may_bind = false;
            return;
        }
        fp.bind_error = None;
        fp.bound_after = after;
        ctx.tap = Some(RowTap {
            entity: Some(entity),
            replaced_existing: replaced_existing(&out.json),
            ..Default::default()
        });
        self.fp_snapshot("bind", ctx).await;
    }

    /// One entity snapshot of the player's space (`at` = `bind` or
    /// `finish`). A failure is kept in the snapshot, never on the row.
    async fn fp_snapshot(&self, at: &str, ctx: &mut RowCtx) {
        let (Some(server), Some(player)) = (self.server, tap_entity_of(ctx)) else {
            return;
        };
        let got = self
            .fp_call(
                server,
                "server_entity_get",
                json!({ "entity_id": player }),
                ctx,
            )
            .await;
        let space = got.json.pointer("/entity/space_id").and_then(as_entity);
        let snap = match space {
            _ if !got.ok => json!({
                "at": at,
                "error": format!("server_entity_get: {}", got.error.unwrap_or_default()),
            }),
            None => json!({ "at": at, "error": "server_entity_get: the player has no space_id" }),
            Some(space) => {
                let q = self
                    .fp_call(
                        server,
                        "server_entity_query",
                        json!({ "space_id": space }),
                        ctx,
                    )
                    .await;
                match q.json.get("entities").and_then(Value::as_array) {
                    Some(list) if q.ok => json!({
                        "at": at,
                        "space_id": space,
                        "capped": q.json.get("capped").and_then(Value::as_bool).unwrap_or(false),
                        "entities": list.iter().map(trim_entity).collect::<Vec<_>>(),
                    }),
                    _ => json!({
                        "at": at,
                        "space_id": space,
                        "error": format!(
                            "server_entity_query: {}",
                            q.error.unwrap_or_else(|| "no entities list".into())
                        ),
                    }),
                }
            }
        };
        if let Some(fp) = ctx.fp.as_mut() {
            if fp.space_id.is_none() {
                fp.space_id = space;
            }
            fp.snapshots.push(snap);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_names_how_the_tap_bound() {
        let mut fp = FpCapture::new(false, true);
        assert_eq!(fp.coverage(true), "full");
        assert_eq!(fp.coverage(false), "none");
        fp.bound_after = Some("play".into());
        assert_eq!(fp.coverage(true), "late:play");
        let fp = FpCapture::new(true, true);
        assert!(!fp.may_bind);
        assert_eq!(fp.coverage(false), "unavailable");
    }

    #[test]
    fn an_entity_keeps_only_the_labelling_fields() {
        let e = json!({ "entity_id": 5, "is_player": false, "tag": null, "spawn_id": 9,
            "template_id": 14, "position": [1, 2, 3], "entity_name": "Frost" });
        assert_eq!(
            trim_entity(&e),
            json!({ "entity_id": 5, "is_player": false, "tag": null, "spawn_id": 9, "template_id": 14 })
        );
    }
}
