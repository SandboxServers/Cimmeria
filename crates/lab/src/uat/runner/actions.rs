//! Executing actions: tool calls by name (with fallbacks), typed chat
//! lines, waits and captures; plus the up-front checks that BLOCK a row
//! before anything is driven, and evidence attachments.

use std::time::Duration;

use serde_json::{json, Map, Value};

use super::{now_ms, RowCtx, Runner};
use crate::uat::evidence::{clip, ActionRecord, Attachment};
use crate::uat::invoke::{ToolInvoker, ToolOutcome};
use crate::uat::spec::{ActionKind, ActionSpec, RowSpec, SectionSpec, Source};
use crate::uat::tier::{self, Role};

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

/// Colo rule 6: these need the owner's say-so in the run's
/// `owner_approvals` (the approval word is the second item).
pub const OWNER_ONLY: [(&str, &str); 5] = [
    (".announce", "announce"),
    (".bm_seed", "bm_seed"),
    (".mute", "mute"),
    ("/gmshout", "gmshout"),
    ("server_content_reload", "content_reload"),
];

/// Characters `client_type_text` can type today.
pub fn typable(line: &str) -> bool {
    line.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '/' | '.'))
}

/// `${name}` → the variable's value, in every string of `v`.
pub fn subst(v: &Value, vars: &Map<String, Value>) -> Value {
    match v {
        Value::String(s) => Value::String(subst_str(s, vars)),
        Value::Array(a) => Value::Array(a.iter().map(|x| subst(x, vars)).collect()),
        Value::Object(o) => {
            Value::Object(o.iter().map(|(k, x)| (k.clone(), subst(x, vars))).collect())
        }
        other => other.clone(),
    }
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

fn unresolved(s: &str) -> Option<String> {
    let start = s.find("${")?;
    let end = s[start..].find('}')?;
    Some(s[start..start + end + 1].to_string())
}

impl<I: ToolInvoker> Runner<'_, I> {
    /// Reasons this row cannot run at all, found before anything moves.
    pub(crate) fn static_blocks(&self, spec: &SectionSpec, row: &RowSpec) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(b) = &row.blocked {
            out.push(b.clone());
        }
        if row.players > 1 {
            out.push(format!(
                "needs {} players: a second lab instance or a wireclient puppet (matrix X1)",
                row.players
            ));
        }
        if spec.section.account == "non-gm" {
            out.push("needs a non-GM account (matrix X2): only the GM lab account exists".into());
        }
        let all = row.setup.iter().chain(&row.steps).chain(&row.teardown);
        for a in all {
            self.check_action(a, &mut out);
        }
        for c in row.expect.iter().filter(|c| c.required) {
            let reader = match c.source {
                Source::Chat => Some(CHAT_READ_TOOL),
                Source::Tool => c.tool.as_deref(),
                Source::Lua => Some("client_lua_eval"),
                Source::Wait => Some("client_wait_for"),
                _ => None,
            };
            if let Some(t) = reader {
                if !self.inv.has_tool(t) {
                    out.push(format!(
                        "clause {} needs tool {t}, which is not routed",
                        c.id
                    ));
                }
            }
        }
        out.dedup();
        out
    }

    fn check_action(&self, a: &ActionSpec, out: &mut Vec<String>) {
        let text = a
            .chat
            .clone()
            .or_else(|| a.tool.clone())
            .unwrap_or_default();
        for (pat, word) in OWNER_ONLY {
            let hit = text == pat || text.starts_with(&format!("{pat} "));
            if hit && !self.req.owner_approvals.iter().any(|w| w == word) {
                out.push(format!(
                    "colo rule 6: {pat} needs the owner's say-so (owner_approvals: [\"{word}\"])"
                ));
            }
        }
        match a.kind() {
            Ok(ActionKind::Tool) => {
                let t = a.tool.as_deref().unwrap_or_default();
                let alt = a.fallback.iter().any(|f| self.action_available(f));
                if !self.inv.has_tool(t) && !alt {
                    out.push(format!("needs tool {t}, which is not routed"));
                }
                if let Err(e) = tier::resolve_tool(t, a.tier) {
                    out.push(format!("spec: {e}"));
                }
            }
            Ok(ActionKind::Chat) => {
                let line = a.chat.as_deref().unwrap_or_default();
                if !self.inv.has_tool(CHAT_SEND_TOOL) {
                    for t in CHAT_MACRO_TOOLS {
                        if !self.inv.has_tool(t) {
                            out.push(format!("typing chat needs tool {t}, which is not routed"));
                        }
                    }
                    // `${var}` values are checked when the line is sent.
                    let bare = crate::uat::spec::without_vars(line);
                    if !typable(&bare) {
                        out.push(format!(
                            "chat line {line:?} has characters the lab cannot type yet (needs {CHAT_SEND_TOOL} / matrix L11)"
                        ));
                    }
                }
            }
            Ok(ActionKind::Capture) => {
                if !self.inv.has_tool(CHAT_READ_TOOL) {
                    out.push(format!("capture needs tool {CHAT_READ_TOOL}"));
                }
            }
            Ok(ActionKind::Wait) => {}
            Err(e) => out.push(format!("spec: {e}")),
        }
    }

    fn action_available(&self, a: &ActionSpec) -> bool {
        match a.kind() {
            Ok(ActionKind::Tool) => a.tool.as_deref().is_some_and(|t| self.inv.has_tool(t)),
            Ok(ActionKind::Chat) => {
                self.inv.has_tool(CHAT_SEND_TOOL)
                    || CHAT_MACRO_TOOLS.iter().all(|t| self.inv.has_tool(t))
            }
            _ => true,
        }
    }

    /// Run one action and record it. Tool fallbacks are tried in order
    /// only when the primary tool is not routed.
    pub(crate) async fn exec(
        &mut self,
        a: &ActionSpec,
        role: Role,
        ctx: &mut RowCtx,
    ) -> ActionRecord {
        let chosen: &ActionSpec = if a.tool.as_deref().is_some_and(|t| !self.inv.has_tool(t)) {
            a.fallback
                .iter()
                .find(|f| self.action_available(f))
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
        };
        let t0 = std::time::Instant::now();
        match chosen.kind() {
            Ok(ActionKind::Tool) => self.exec_tool(chosen, role, ctx, &mut rec).await,
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
                    self.send_chat(&line, &mut rec).await;
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
                match (self.read_chat(ctx).await, regex::Regex::new(&re)) {
                    (Ok(lines), Ok(re)) => {
                        let hit = lines.iter().rev().find_map(|l| {
                            re.captures(l)
                                .and_then(|c| c.get(1))
                                .map(|m| m.as_str().to_string())
                        });
                        match hit {
                            Some(v) => {
                                ctx.vars.insert(var.clone(), json!(v));
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
        let out = self.inv.call(&name, args).await;
        if let Some(chars) = out.json.get("characters").and_then(Value::as_array) {
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
    }

    /// Send one chat line: `client_chat_send` when routed, else the
    /// focus / Enter / type / Enter macro.
    async fn send_chat(&self, line: &str, rec: &mut ActionRecord) {
        if self.inv.has_tool(CHAT_SEND_TOOL) {
            rec.tool = Some(CHAT_SEND_TOOL.into());
            let out = self.inv.call(CHAT_SEND_TOOL, json!({ "line": line })).await;
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
            let out = self.inv.call(tool, args.clone()).await;
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

    /// The chat box's last lines (oldest first).
    pub(crate) async fn read_chat(&self, _ctx: &RowCtx) -> Result<Vec<String>, String> {
        let out = self
            .inv
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
            if !self.inv.has_tool(&e.tool) {
                ctx.attachments.push(Attachment {
                    name: e.name.clone(),
                    path: String::new(),
                    tool: format!("{} (not routed)", e.tool),
                    host_ms: now_ms(),
                });
                continue;
            }
            let args = subst(e.args.as_ref().unwrap_or(&json!({})), &ctx.vars);
            let out = self.inv.call(&e.tool, args).await;
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

fn fail(rec: &mut ActionRecord, msg: String) {
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
        assert_eq!(
            unresolved(".mail_expire ${mail_id}").as_deref(),
            Some("${mail_id}")
        );
        assert!(unresolved("no vars").is_none());
    }

    #[test]
    fn typable_matches_the_lab_charset() {
        assert!(typable(".bug uat M1-1"));
        assert!(typable("/gmgotolocation Harset 0 0 0"));
        assert!(!typable("/afk afk, back soon"));
        assert!(!typable(".bug what?"));
    }
}
