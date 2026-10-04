//! Executing actions: tool calls by name (with fallbacks), typed chat
//! lines, waits and captures; and evidence attachments. The up-front
//! checks that BLOCK a row are in [`super::checks`].

use std::time::Duration;

use serde_json::{json, Map, Value};

use super::players::Who;
use super::{now_ms, RowCtx, Runner};
use crate::uat::evidence::{clip, ActionRecord, Attachment};
use crate::uat::invoke::{ToolInvoker, ToolOutcome};
use crate::uat::lab_commands;
use crate::uat::spec::{ActionKind, ActionSpec, RowSpec};
use crate::uat::tier::{self, Role};
use crate::uat::tools::TARGET_PLAYER_TOOL;

/// The press tool whose result carries the ability and the event seq the
/// cast-id capture starts from.
pub(crate) const USE_ABILITY_TOOL: &str = "client_use_ability";

/// The tools a typed chat line expands into when `client_chat_send` is
/// not routed: focus, Enter, type, Enter (the same keys `lab_logout`
/// uses).
pub const CHAT_MACRO_TOOLS: [&str; 3] =
    ["client_input_focus", "client_input_key", "client_type_text"];
/// A single-call chat sender a sibling change may add (matrix L11).
pub const CHAT_SEND_TOOL: &str = "client_chat_send"; // capability @chat_send
/// The chat reader: `client_ui_state`'s `chat_tail`.
pub const CHAT_READ_TOOL: &str = "client_ui_state";
/// Lines the chat reader asks for (the tool's maximum).
const CHAT_TAIL: u32 = 150;

/// `${name}` → the variable's value, in every string of `v`. A string
/// that is exactly one `${name}` takes the variable's own JSON value, so
/// `entity_id = "${dummy_id}"` reaches a tool as the number it holds.
pub fn subst(v: &Value, vars: &Map<String, Value>) -> Value {
    match v {
        Value::String(s) => match exact_var(s).and_then(|k| vars.get(k)) {
            Some(val) if !val.is_string() => val.clone(),
            _ => Value::String(subst_str(s, vars)),
        },
        Value::Array(a) => Value::Array(a.iter().map(|x| subst(x, vars)).collect()),
        Value::Object(o) => {
            Value::Object(o.iter().map(|(k, x)| (k.clone(), subst(x, vars))).collect())
        }
        other => other.clone(),
    }
}

/// A captured chat group as a var: a whole number stays a number (an
/// entity or mail id reaches a tool's integer argument as one); anything
/// else is a string. Either substitutes into a line as the same text.
pub(crate) fn captured_value(s: &str) -> Value {
    s.parse::<u64>().map_or_else(|_| json!(s), |n| json!(n))
}

/// `name` when `s` is exactly `${name}`.
fn exact_var(s: &str) -> Option<&str> {
    s.strip_prefix("${")
        .and_then(|r| r.strip_suffix('}'))
        .filter(|k| !k.contains(['$', '{', '}']))
}

pub fn subst_str(s: &str, vars: &Map<String, Value>) -> String {
    let mut out = s.to_string();
    for (k, v) in vars {
        let needle = format!("${{{k}}}");
        if out.contains(&needle) {
            let text = match v {
                Value::String(t) => t.clone(),
                other => other.to_string(),
            };
            out = out.replace(&needle, &text);
        }
    }
    out
}

pub(crate) fn unresolved(s: &str) -> Option<String> {
    let start = s.find("${")?;
    let end = s[start..].find('}')?;
    Some(s[start..start + end + 1].to_string())
}

impl<I: ToolInvoker> Runner<'_, I> {
    /// Run one action and record it. Tool fallbacks are tried in order
    /// only when the primary tool is not routed.
    pub(crate) async fn exec(
        &mut self,
        a: &ActionSpec,
        role: Role,
        ctx: &mut RowCtx,
    ) -> ActionRecord {
        let who = Who::of(a.client.as_deref());
        let chosen: &ActionSpec = if a.tool.as_deref().is_some_and(|t| !self.routed(who, t)) {
            a.fallback
                .iter()
                .find(|f| self.action_available(who, f))
                .unwrap_or(a)
        } else {
            a
        };
        let fallback_used = !std::ptr::eq(chosen, a);
        let mut rec = ActionRecord {
            index: ctx.actions.len(),
            role,
            label: a.label.clone(),
            kind: String::new(),
            requested: a
                .tool
                .clone()
                .or_else(|| a.chat.clone())
                .unwrap_or_default(),
            tool: None,
            args: Value::Null,
            tier: None,
            tier_source: None,
            fallback_used,
            host_started_ms: now_ms(),
            elapsed_ms: 0,
            ok: true,
            error: None,
            result: Value::Null,
            calls: vec![],
            client: who.tag(),
        };
        let t0 = std::time::Instant::now();
        match chosen.kind() {
            Ok(ActionKind::Tool) => self.exec_tool(chosen, who, role, ctx, &mut rec).await,
            Ok(ActionKind::Chat) => {
                rec.kind = "chat".into();
                let line = subst_str(chosen.chat.as_deref().unwrap_or_default(), &ctx.vars);
                rec.requested = line.clone();
                match tier::resolve_chat(role, chosen.tier) {
                    Ok(r) => {
                        rec.tier = Some(r.tier);
                        rec.tier_source = Some(r.source.into());
                    }
                    Err(e) => fail(&mut rec, e),
                }
                if let Some(v) = unresolved(&line) {
                    fail(&mut rec, format!("unresolved variable {v}"));
                } else if rec.ok {
                    self.send_chat(who, &line, &mut rec).await;
                }
            }
            Ok(ActionKind::Wait) => {
                rec.kind = "wait".into();
                rec.requested = format!("wait {} ms", chosen.wait_ms.unwrap_or(0));
                tokio::time::sleep(Duration::from_millis(chosen.wait_ms.unwrap_or(0))).await;
            }
            Ok(ActionKind::Capture) => {
                rec.kind = "capture".into();
                let re = chosen.regex.clone().unwrap_or_default();
                let var = chosen.var.clone().unwrap_or_default();
                rec.requested = format!("capture {var} from chat /{re}/");
                match (self.read_chat_of(who).await, regex::Regex::new(&re)) {
                    (Ok(lines), Ok(re)) => {
                        let hit = lines.iter().rev().find_map(|l| {
                            re.captures(l)
                                .and_then(|c| c.get(1))
                                .map(|m| m.as_str().to_string())
                        });
                        match hit {
                            Some(v) => {
                                let v = captured_value(&v);
                                ctx.vars.insert(var.clone(), v.clone());
                                rec.result = json!({ var: v });
                            }
                            None => fail(&mut rec, "no chat line matched".into()),
                        }
                    }
                    (Err(e), _) => fail(&mut rec, e),
                    (_, Err(e)) => fail(&mut rec, e.to_string()),
                }
            }
            Err(e) => fail(&mut rec, e),
        }
        rec.elapsed_ms = t0.elapsed().as_millis() as u64;
        rec
    }

    async fn exec_tool(
        &mut self,
        a: &ActionSpec,
        who: Who,
        role: Role,
        ctx: &mut RowCtx,
        rec: &mut ActionRecord,
    ) {
        rec.kind = "tool".into();
        let name = a.tool.clone().unwrap_or_default();
        let args = subst(a.args.as_ref().unwrap_or(&json!({})), &ctx.vars);
        rec.tool = Some(name.clone());
        rec.args = args.clone();
        match tier::resolve_tool(&name, a.tier) {
            Ok(r) => {
                rec.tier = if r.source == "read" && role == Role::Step {
                    None
                } else {
                    Some(r.tier)
                };
                rec.tier_source = Some(r.source.into());
            }
            Err(e) => return fail(rec, e),
        }
        if let Some(v) = unresolved(&args.to_string()) {
            return fail(rec, format!("unresolved variable {v}"));
        }
        // `@dummy`, `@cooldowns_reset`, `@clear_effects`: typed dot commands.
        match lab_commands::build(&name, &args) {
            Ok(Some(cmd)) => return self.exec_lab_command(cmd, who, ctx, rec).await,
            Ok(None) => {}
            Err(e) => return fail(rec, e),
        }
        let out = if name == TARGET_PLAYER_TOOL {
            match self.target_player_args(who, args, ctx) {
                Ok(args) => {
                    rec.tool = Some("client_target".into());
                    rec.args = args.clone();
                    self.on(who).call("client_target", args).await
                }
                Err(e) => return fail(rec, e),
            }
        } else {
            self.on(who).call(&name, args).await
        };
        // Only p1's character list is the run's (p2 plays its own account).
        if let Some(chars) = out
            .json
            .get("characters")
            .and_then(Value::as_array)
            .filter(|_| who == Who::P1)
        {
            self.characters = chars.clone();
        }
        self.attach_images(ctx, &format!("action{}", rec.index), &name, &out);
        record_outcome(rec, &out);
        // A tool that reports how it drove the game (the world tools #1099,
        // the combat tools #1100) overrides the static tier when it fell back lower.
        if let Some((reported, word)) = out
            .json
            .get("native_level")
            .and_then(tier::from_reported_value)
        {
            if rec.tier.is_some_and(|t| reported > t) {
                rec.tier = Some(reported);
                rec.tier_source = Some(format!("reported:{word}"));
            }
        }
        // A press: find the cast it became, for `${cast_id}` (AB-L3).
        if name == USE_ABILITY_TOOL && out.ok && role != Role::Teardown {
            self.capture_cast_id(who, a.label.as_deref(), &out.json, ctx, rec)
                .await;
        }
    }

    /// Send one chat line: `client_chat_send` when routed, else the
    /// focus / Enter / type / Enter macro.
    pub(crate) async fn send_chat(&self, who: Who, line: &str, rec: &mut ActionRecord) {
        let inv = self.on(who);
        if inv.has_tool(CHAT_SEND_TOOL) {
            rec.tool = Some(CHAT_SEND_TOOL.into());
            let out = inv.call(CHAT_SEND_TOOL, json!({ "line": line })).await;
            rec.calls
                .push(json!({ "tool": CHAT_SEND_TOOL, "ok": out.ok }));
            record_outcome(rec, &out);
            return;
        }
        rec.tool = Some("chat_macro".into());
        let seq: [(&str, Value, u64); 4] = [
            ("client_input_focus", json!({ "on": true }), 0),
            ("client_input_key", json!({ "key": "Enter" }), 400),
            ("client_type_text", json!({ "text": line }), 0),
            ("client_input_key", json!({ "key": "Enter" }), 900),
        ];
        for (tool, args, settle) in seq {
            let out = inv.call(tool, args.clone()).await;
            rec.calls
                .push(json!({ "tool": tool, "args": args, "ok": out.ok, "error": out.error }));
            if !out.ok {
                return fail(rec, format!("{tool}: {}", out.error.unwrap_or_default()));
            }
            if settle > 0 && !self.req.no_settle {
                tokio::time::sleep(Duration::from_millis(settle)).await;
            }
        }
    }

    /// p1's chat box's last lines (oldest first).
    pub(crate) async fn read_chat(&self, _ctx: &RowCtx) -> Result<Vec<String>, String> {
        self.read_chat_of(Who::P1).await
    }

    /// `who`'s chat box's last lines (oldest first).
    pub(crate) async fn read_chat_of(&self, who: Who) -> Result<Vec<String>, String> {
        let out = self
            .on(who)
            .call(CHAT_READ_TOOL, json!({ "chat_lines": CHAT_TAIL }))
            .await;
        if !out.ok {
            return Err(out.error.unwrap_or_else(|| "chat read failed".into()));
        }
        Ok(out
            .json
            .get("chat_tail")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default())
    }

    /// Evidence whose `at` is `label` (or unset, for `None`).
    pub(crate) async fn evidence_at(
        &mut self,
        row: &RowSpec,
        label: Option<&String>,
        ctx: &mut RowCtx,
    ) {
        for e in row.evidence.iter().filter(|e| e.at.as_ref() == label) {
            let inv = self.on(Who::of(e.client.as_deref()));
            if !inv.has_tool(&e.tool) {
                ctx.attachments.push(Attachment {
                    name: e.name.clone(),
                    path: String::new(),
                    tool: format!("{} (not routed)", e.tool),
                    host_ms: now_ms(),
                });
                continue;
            }
            let args = subst(e.args.as_ref().unwrap_or(&json!({})), &ctx.vars);
            let out = inv.call(&e.tool, args).await;
            self.attach_images(ctx, &e.name, &e.tool, &out);
            let path = self
                .run
                .attach_dir(&ctx.section, &ctx.row_id)
                .join(format!("{}.json", e.name));
            let body = json!({ "ok": out.ok, "error": out.error, "result": clip(&out.json, 4000) });
            if self.run.write_json(&path, &body).is_ok() {
                ctx.attachments.push(Attachment {
                    name: e.name.clone(),
                    path: self.rel(&path),
                    tool: e.tool.clone(),
                    host_ms: now_ms(),
                });
            }
        }
    }

    /// A final screenshot for every row that ran, when the tool exists.
    pub(crate) async fn capture_final(&mut self, ctx: &mut RowCtx) {
        if self.inv.has_tool("lab_screenshot") && !ctx.attachments.iter().any(|a| a.name == "final")
        {
            let out = self.inv.call("lab_screenshot", json!({})).await;
            self.attach_images(ctx, "final", "lab_screenshot", &out);
        }
    }

    /// p2's final screenshot on a two-player row (`final-p2.png`).
    pub(crate) async fn capture_final_p2(&mut self, ctx: &mut RowCtx) {
        let inv = self.on(Who::P2);
        if self.p2.is_ok() && inv.has_tool("lab_screenshot") {
            let out = inv.call("lab_screenshot", json!({})).await;
            self.attach_images(ctx, "final-p2", "lab_screenshot", &out);
        }
    }

    pub(crate) fn attach_images(
        &self,
        ctx: &mut RowCtx,
        name: &str,
        tool: &str,
        out: &ToolOutcome,
    ) {
        for (i, (mime, bytes)) in out.images.iter().enumerate() {
            let ext = if mime.contains("png") { "png" } else { "bin" };
            let file = if i == 0 {
                format!("{name}.{ext}")
            } else {
                format!("{name}-{i}.{ext}")
            };
            let dir = self.run.attach_dir(&ctx.section, &ctx.row_id);
            let path = dir.join(&file);
            if std::fs::create_dir_all(&dir).is_ok() && std::fs::write(&path, bytes).is_ok() {
                ctx.attachments.push(Attachment {
                    name: name.to_string(),
                    path: self.rel(&path),
                    tool: tool.to_string(),
                    host_ms: now_ms(),
                });
            }
        }
    }

    pub(crate) fn rel(&self, p: &std::path::Path) -> String {
        p.strip_prefix(&self.run.root)
            .unwrap_or(p)
            .to_string_lossy()
            .replace('\\', "/")
    }
}

pub(crate) fn fail(rec: &mut ActionRecord, msg: String) {
    rec.ok = false;
    rec.error = Some(msg);
}

fn record_outcome(rec: &mut ActionRecord, out: &ToolOutcome) {
    rec.ok = out.ok;
    rec.error.clone_from(&out.error);
    rec.result = clip(&out.json, 2000);
    if let Some(d) = &out.error_data {
        rec.result = json!({ "result": rec.result, "error_data": clip(d, 2000) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vars_substitute_everywhere_and_leftovers_are_found() {
        let mut vars = Map::new();
        vars.insert("row_id".into(), json!("M1-1"));
        vars.insert("n".into(), json!(3));
        let v = subst(&json!({"a": ["uat ${row_id}", "${n}x"]}), &vars);
        assert_eq!(v, json!({"a": ["uat M1-1", "3x"]}));
        // A whole-string var keeps its type: an entity id stays a number.
        assert_eq!(subst(&json!({"id": "${n}"}), &vars), json!({"id": 3}));
        assert_eq!(subst(&json!("${row_id}"), &vars), json!("M1-1"));
        assert_eq!(captured_value("4242"), json!(4242));
        assert_eq!(captured_value("Labone"), json!("Labone"));
        assert_eq!(captured_value("-3"), json!("-3"));
        assert_eq!(
            unresolved(".mail_expire ${mail_id}").as_deref(),
            Some("${mail_id}")
        );
        assert!(unresolved("no vars").is_none());
    }
}
