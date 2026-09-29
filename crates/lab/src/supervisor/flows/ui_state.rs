//! UI reads: `client_ui_state`, `client_wait_for`, and the Lua plumbing
//! the flows share (result extraction, condition polling, the screen
//! summary attached to flow errors).

use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};

use super::widgets::{self, DIALOG_BUTTONS, KNOWN_SCREENS, PROMPT_TEXT};
use crate::supervisor::Supervisor;

/// Chat lines `client_ui_state` returns by default.
pub const DEFAULT_CHAT_LINES: u32 = 10;

/// Pull a `lua_eval` result apart: `results` on success, the Lua error
/// otherwise. A capture-less bridge (results unavailable) is an error too:
/// every flow read depends on return values.
pub fn lua_results_of(v: &Value) -> Result<Vec<String>, String> {
    let ok = v.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let error = v.get("error").and_then(Value::as_str).unwrap_or_default();
    let results: Vec<String> = v
        .get("results")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|x| x.as_str().map_or_else(|| x.to_string(), str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if !ok {
        return Err(format!(
            "Lua error: {}",
            if error.is_empty() {
                "(no message)"
            } else {
                error
            }
        ));
    }
    if results.is_empty() && !error.is_empty() {
        return Err(format!("lua_eval returned no results: {error}"));
    }
    Ok(results)
}

/// The chunk one poll runs: evaluates `cond` (and, when it does not hold,
/// `fail`) under `pcall`, so a nil index during a screen transition reads
/// as "not yet" instead of aborting the wait. Returns three strings:
/// `met`, the condition's Lua error (or empty), the fail value (or empty).
pub fn poll_chunk(cond: &str, fail: Option<&str>) -> String {
    let fail_part = match fail {
        Some(f) => format!(
            "if not met then local okf, f = pcall(function() return {f} end) \
             if okf and f ~= nil then fv = tostring(f) end end "
        ),
        None => String::new(),
    };
    format!(
        "local okc, r = pcall(function() return not not ({cond}) end) \
         local met = okc and r local fv = '' {fail_part}\
         return tostring(met), okc and '' or tostring(r), fv"
    )
}

/// One poll's outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Poll {
    pub met: bool,
    pub error: Option<String>,
    pub fail: Option<String>,
}

pub fn parse_poll(results: &[String]) -> Result<Poll, String> {
    let nonempty = |s: Option<&String>| s.filter(|s| !s.is_empty()).cloned();
    match results.first() {
        Some(met) => Ok(Poll {
            met: met == "true",
            error: nonempty(results.get(1)),
            fail: nonempty(results.get(2)),
        }),
        None => Err("condition poll returned nothing".to_string()),
    }
}

/// How a wait ended.
#[derive(Debug, Clone)]
pub struct WaitOutcome {
    pub met: bool,
    pub elapsed_ms: u64,
    pub polls: u32,
    pub last_error: Option<String>,
    pub fail_text: Option<String>,
}

impl WaitOutcome {
    pub fn to_json(&self) -> Value {
        json!({
            "met": self.met,
            "elapsed_ms": self.elapsed_ms,
            "polls": self.polls,
            "last_error": self.last_error,
            "fail_text": self.fail_text,
        })
    }
}

/// The chunk that lists which known screens are visible, plus the text of
/// a visible prompt (attached to every flow error).
pub fn screens_chunk() -> String {
    let mut s = String::from("local out = {} ");
    for w in KNOWN_SCREENS {
        s.push_str(&format!(
            "do local okv, v = pcall(function() return {} end) \
             if okv and v then out[#out + 1] = {} end end ",
            widgets::visible(w),
            widgets::lua_quote(w)
        ));
    }
    s.push_str(&format!(
        "local okp, p = pcall(function() return {PROMPT_TEXT} end) \
         return table.concat(out, ','), (okp and p) or ''"
    ));
    s
}

/// Build `client_ui_state`'s chunk. Each result is one tagged line,
/// `section<TAB>value…`; each section runs under its own `pcall` so one
/// missing widget does not blank the whole read.
pub fn ui_state_chunk(chat_lines: u32) -> String {
    let buttons = DIALOG_BUTTONS
        .iter()
        .filter(|b| !b.contains('/'))
        .map(|b| widgets::lua_quote(b))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        r#"local out = {{}}
local function add(...) out[#out + 1] = table.concat({{...}}, "\t") end
local function sec(name, f) local ok, e = pcall(f) if not ok then add("error", name .. ": " .. tostring(e)) end end
sec("windows", function()
  local root = getWindow("Root")
  for i = 0, root:getChildCount() - 1 do
    local w = root:getChildAtIdx(i)
    if w:isVisible() then add("window", w:getName()) end
  end
end)
sec("dialog", function()
  if DialogWin ~= nil and DialogWin:isVisible() then
    add("dialog_title", tostring(Dialog_NameText and Dialog_NameText:getText() or ""))
    add("dialog_text", tostring(Dialog_ScreenText and Dialog_ScreenText:getText() or ""))
    for _, b in ipairs({{{buttons}}}) do
      local w = _G[b]
      if w ~= nil and w:isVisible() then add("dialog_button", b, tostring(w:getText())) end
    end
  end
end)
sec("prompts", function()
  for i = 1, 5 do
    local w = _G["Prompt" .. i .. "_PromptWin"]
    if w ~= nil and w:isVisible() then
      local m = _G["Prompt" .. i .. "_Message"]
      add("prompt", tostring(i), tostring(w:getText()), tostring(m and m:getText() or ""))
    end
  end
end)
sec("missions", function()
  for i = 1, 16 do
    local w = _G["MissionTracker_WidgetText_" .. i]
    if w ~= nil and w:isVisible() then add("mission", w:getText()) end
    local s = _G["MissionTracker" .. i .. "StepWidgetText"]
    if s ~= nil and s:isVisible() then add("mission_step", s:getText()) end
    local o = _G["MissionTracker_ObjWidgetText_" .. i]
    if o ~= nil and o:isVisible() then add("mission_objective", o:getText()) end
  end
end)
sec("chat", function()
  local lb = Inst1Chat_TabOutput
  if lb ~= nil then
    local n = lb:getItemCount()
    for i = math.max(0, n - {chat_lines}), n - 1 do add("chat", lb:getItemFromIndex(i):getText()) end
  end
end)
return unpack(out)"#
    )
}

/// Fold the tagged lines into JSON.
pub fn parse_ui_state(lines: &[String]) -> Value {
    let mut windows = Vec::new();
    let mut dialog = Map::new();
    let mut dialog_buttons = Vec::new();
    let mut prompts = Vec::new();
    let mut missions = Vec::new();
    let mut chat = Vec::new();
    let mut errors = Vec::new();
    for line in lines {
        let mut parts = line.splitn(4, '\t');
        let tag = parts.next().unwrap_or_default();
        let a = parts.next().unwrap_or_default().to_string();
        let b = parts.next().map(str::to_string);
        let c = parts.next().map(str::to_string);
        match tag {
            "window" => windows.push(a),
            "dialog_title" => {
                dialog.insert("title".into(), json!(a));
            }
            "dialog_text" => {
                dialog.insert("text".into(), json!(a));
            }
            "dialog_button" => dialog_buttons.push(json!({ "window": a, "label": b })),
            "prompt" => prompts.push(json!({ "instance": a, "title": b, "message": c })),
            "mission" => missions.push(json!({ "kind": "mission", "text": a })),
            "mission_step" => missions.push(json!({ "kind": "step", "text": a })),
            "mission_objective" => missions.push(json!({ "kind": "objective", "text": a })),
            "chat" => chat.push(a),
            "error" => errors.push(a),
            _ => errors.push(format!("unknown line {line:?}")),
        }
    }
    let dialog = if dialog.is_empty() {
        Value::Null
    } else {
        dialog.insert("buttons".into(), json!(dialog_buttons));
        Value::Object(dialog)
    };
    json!({
        "windows": windows,
        "dialog": dialog,
        "prompts": prompts,
        "mission_tracker": missions,
        "chat_tail": chat,
        "errors": errors,
    })
}

impl Supervisor {
    /// Run a Lua chunk and return its (stringified) results.
    pub async fn lua_results(&self, chunk: &str) -> Result<Vec<String>, String> {
        let v = self
            .bridge_call("lua_eval", json!({ "chunk": chunk }))
            .await?;
        lua_results_of(&v)
    }

    /// Poll `cond` every `poll` until it holds, `fail` yields a value, or
    /// `timeout` passes. Bridge failures end the wait with `Err`.
    pub async fn poll_until(
        &self,
        cond: &str,
        fail: Option<&str>,
        timeout: Duration,
        poll: Duration,
    ) -> Result<WaitOutcome, String> {
        let t0 = Instant::now();
        let chunk = poll_chunk(cond, fail);
        let mut polls = 0u32;
        let mut last_error = None;
        loop {
            polls += 1;
            let p = parse_poll(&self.lua_results(&chunk).await?)?;
            let done =
                |met: bool, fail_text: Option<String>, last_error: Option<String>| WaitOutcome {
                    met,
                    elapsed_ms: t0.elapsed().as_millis() as u64,
                    polls,
                    last_error,
                    fail_text,
                };
            if p.met {
                return Ok(done(true, None, None));
            }
            if p.error.is_some() {
                last_error = p.error;
            }
            if p.fail.is_some() {
                return Ok(done(false, p.fail, last_error));
            }
            if t0.elapsed() >= timeout {
                return Ok(done(false, None, last_error));
            }
            tokio::time::sleep(poll).await;
        }
    }

    /// `client_wait_for`.
    pub async fn wait_for(
        &self,
        cond: &str,
        timeout_ms: Option<u64>,
        poll_ms: Option<u64>,
    ) -> Result<Value, String> {
        let timeout = Duration::from_millis(timeout_ms.unwrap_or(10_000).min(600_000));
        let poll = Duration::from_millis(poll_ms.unwrap_or(250).clamp(50, 60_000));
        let w = self.poll_until(cond, None, timeout, poll).await?;
        let mut out = w.to_json();
        out["condition"] = json!(cond);
        Ok(out)
    }

    /// Visible known screens and prompt text, for flow errors.
    pub async fn screen_summary(&self) -> Result<Value, String> {
        let r = self.lua_results(&screens_chunk()).await?;
        let screens: Vec<&str> = r
            .first()
            .map(|s| s.split(',').filter(|s| !s.is_empty()).collect())
            .unwrap_or_default();
        let prompt = r.get(1).filter(|p| !p.is_empty());
        Ok(json!({ "visible_screens": screens, "prompt": prompt }))
    }

    /// `client_ui_state` — one read of the visible UI.
    pub async fn ui_state(&self, chat_lines: Option<u32>) -> Result<Value, String> {
        let n = chat_lines.unwrap_or(DEFAULT_CHAT_LINES).min(150);
        let lines = self.lua_results(&ui_state_chunk(n)).await?;
        Ok(parse_ui_state(&lines))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn lua_results_surface_errors_and_values() {
        let ok = json!({ "ok": true, "status": 0, "error": "", "results": ["true", "3"] });
        assert_eq!(lua_results_of(&ok).unwrap(), s(&["true", "3"]));
        let bad =
            json!({ "ok": false, "status": 2, "error": "attempt to index nil", "results": [] });
        assert!(lua_results_of(&bad)
            .unwrap_err()
            .contains("attempt to index nil"));
        // Capture unavailable: ok, but no results and a reason.
        let degraded = json!({ "ok": true, "error": "capture unavailable", "results": [] });
        assert!(lua_results_of(&degraded).is_err());
        // A chunk with no return values is fine.
        let empty = json!({ "ok": true, "error": "", "results": [] });
        assert!(lua_results_of(&empty).unwrap().is_empty());
    }

    #[test]
    fn poll_chunk_pcalls_the_condition_and_the_fail_probe() {
        let c = poll_chunk("X:isVisible()", Some("PROMPT"));
        assert!(c.contains("pcall(function() return not not (X:isVisible()) end)"));
        assert!(c.contains("pcall(function() return PROMPT end)"));
        assert!(!poll_chunk("A", None).contains("okf"));
    }

    #[test]
    fn parse_poll_reads_met_error_and_fail() {
        assert_eq!(
            parse_poll(&s(&["true", "", ""])).unwrap(),
            Poll {
                met: true,
                error: None,
                fail: None
            }
        );
        let p = parse_poll(&s(&["false", "nil index", "Login Failed: bad password"])).unwrap();
        assert!(!p.met);
        assert_eq!(p.error.as_deref(), Some("nil index"));
        assert_eq!(p.fail.as_deref(), Some("Login Failed: bad password"));
        assert!(parse_poll(&[]).is_err());
    }

    #[test]
    fn ui_state_lines_fold_into_sections() {
        let lines = s(&[
            "window\tDialogWin",
            "window\tSelfStatusWin",
            "dialog_title\tSergeant Hill",
            "dialog_text\tWelcome to the SGC.",
            "dialog_button\tDialog_NextButton\tNext",
            "prompt\t1\tServer Message\tRestart in 5",
            "mission\tArm Yourself",
            "mission_step\tTalk to Hill",
            "chat\t[Say] Hi",
            "error\tchat: attempt to index nil",
        ]);
        let v = parse_ui_state(&lines);
        assert_eq!(v["windows"], json!(["DialogWin", "SelfStatusWin"]));
        assert_eq!(v["dialog"]["title"], "Sergeant Hill");
        assert_eq!(v["dialog"]["buttons"][0]["window"], "Dialog_NextButton");
        assert_eq!(v["prompts"][0]["message"], "Restart in 5");
        assert_eq!(v["mission_tracker"][1]["kind"], "step");
        assert_eq!(v["chat_tail"], json!(["[Say] Hi"]));
        assert_eq!(v["errors"][0], "chat: attempt to index nil");
    }

    #[test]
    fn ui_state_without_a_dialog_reports_null() {
        let v = parse_ui_state(&s(&["window\tLoginWin"]));
        assert!(v["dialog"].is_null());
    }

    #[test]
    fn ui_state_chunk_takes_the_chat_tail_length() {
        let c = ui_state_chunk(7);
        assert!(c.contains("n - 7"));
        assert!(c.contains("\"Dialog_DoneButton\""));
        // The closebutton path is not a Lua global; it is not probed here.
        assert!(!c.contains("auto_closebutton"));
    }
}
