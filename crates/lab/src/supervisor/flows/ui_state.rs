//! UI reads: `client_ui_state`, `client_wait_for`, and the Lua plumbing
//! the flows share (result extraction, condition polling, the screen
//! summary attached to flow errors).

use std::{
    future::Future,
    time::{Duration, Instant},
};

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
    if results.len() != 3 || !matches!(results[0].as_str(), "true" | "false") {
        return Err(format!("malformed condition poll result: {results:?}"));
    }
    let nonempty = |s: Option<&String>| s.filter(|s| !s.is_empty()).cloned();
    Ok(Poll {
        met: results[0] == "true",
        error: nonempty(results.get(1)),
        fail: nonempty(results.get(2)),
    })
}

/// How a wait ended.
#[derive(Debug, Clone)]
pub struct WaitOutcome {
    pub met: bool,
    pub elapsed_ms: u64,
    pub polls: u32,
    pub failed_polls: u32,
    pub last_error: Option<String>,
    pub fail_text: Option<String>,
}

impl WaitOutcome {
    pub fn to_json(&self) -> Value {
        json!({
            "met": self.met,
            "elapsed_ms": self.elapsed_ms,
            "polls": self.polls,
            "failed_polls": self.failed_polls,
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
    /// `timeout` passes. Bridge failures are retried until the deadline.
    pub async fn poll_until(
        &self,
        cond: &str,
        fail: Option<&str>,
        timeout: Duration,
        poll: Duration,
    ) -> Result<WaitOutcome, String> {
        let chunk = poll_chunk(cond, fail);
        poll_until_with(timeout, poll, || self.lua_results(&chunk)).await
    }
}

async fn poll_until_with<F, Fut>(
    timeout: Duration,
    poll: Duration,
    mut read: F,
) -> Result<WaitOutcome, String>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<Vec<String>, String>>,
{
    let t0 = Instant::now();
    let mut polls = 0u32;
    let mut failed_polls = 0u32;
    let mut last_error = None;
    let deadline = t0 + timeout;
    loop {
        if Instant::now() >= deadline {
            return Ok(WaitOutcome {
                met: false,
                elapsed_ms: t0.elapsed().as_millis() as u64,
                polls,
                failed_polls,
                last_error,
                fail_text: None,
            });
        }
        polls += 1;
        let response =
            tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), read()).await;
        // A response arriving at or after the deadline cannot satisfy the wait.
        if response.is_err() || Instant::now() >= deadline {
            return Ok(WaitOutcome {
                met: false,
                elapsed_ms: t0.elapsed().as_millis() as u64,
                polls,
                failed_polls,
                last_error,
                fail_text: None,
            });
        }
        let p = match response.expect("timeout returned above") {
            Ok(results) => Some(parse_poll(&results)?),
            Err(error) if transient_poll_error(&error) => {
                failed_polls += 1;
                last_error = Some(error);
                None
            }
            Err(error) => return Err(error),
        };
        let done = |met: bool, fail_text: Option<String>, last_error: Option<String>| WaitOutcome {
            met,
            elapsed_ms: t0.elapsed().as_millis() as u64,
            polls,
            failed_polls,
            last_error,
            fail_text,
        };
        if let Some(p) = p {
            if p.met {
                return Ok(done(true, None, None));
            }
            if p.error.is_some() {
                last_error = p.error;
            }
            if p.fail.is_some() {
                return Ok(done(false, p.fail, last_error));
            }
        }
        if t0.elapsed() >= timeout {
            return Ok(done(false, None, last_error));
        }
        tokio::time::sleep(poll.min(timeout.saturating_sub(t0.elapsed()))).await;
    }
}

/// Only retry bridge transport/dispatch failures. Lua, lease and malformed
/// response errors must surface immediately rather than waste the wait budget.
fn transient_poll_error(error: &str) -> bool {
    if !error.starts_with("bridge: ") {
        return false;
    }
    let detail = &error[8..];
    if detail.starts_with("bridge error ") {
        return detail.contains("dispatch timeout") || detail.contains("dispatch queue full");
    }
    [
        "connection reset",
        "connection aborted",
        "bridge connection reset while authenticating",
        "early eof",
        "unexpected eof",
        "unexpected end of file",
        "failed to fill whole buffer",
        "os error 10053",
        "os error 10054",
    ]
    .iter()
    .any(|part| detail.to_ascii_lowercase().contains(part))
}

impl Supervisor {
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
    use std::collections::VecDeque;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[tokio::test]
    async fn poll_retries_bridge_errors_until_condition_is_met() {
        let mut sequence = VecDeque::from([
            Err("bridge: bridge error -32603: dispatch timeout".to_string()),
            Err("bridge: connection reset by peer (os error 10054)".to_string()),
            Ok(s(&["true", "", ""])),
        ]);
        let result = poll_until_with(Duration::from_secs(1), Duration::from_millis(1), || {
            std::future::ready(sequence.pop_front().expect("poll sequence exhausted"))
        })
        .await
        .unwrap();

        assert!(result.met);
        assert_eq!(result.polls, 3);
        assert_eq!(result.failed_polls, 2);
        assert_eq!(result.to_json()["failed_polls"], 2);
        assert_eq!(result.last_error, None);
    }

    #[tokio::test]
    async fn poll_returns_last_bridge_error_at_deadline() {
        let mut polls = 0;
        let result = poll_until_with(Duration::from_millis(12), Duration::from_millis(4), || {
            polls += 1;
            std::future::ready(Err(
                "bridge: bridge error -32001: dispatch queue full".to_string()
            ))
        })
        .await
        .unwrap();

        assert!(!result.met);
        assert_eq!(
            result.last_error.as_deref(),
            Some("bridge: bridge error -32001: dispatch queue full")
        );
        assert_eq!(result.failed_polls, result.polls);
        assert_eq!(result.to_json()["failed_polls"], result.polls);
        assert_eq!(result.polls as usize, polls);
        assert!(result.polls >= 1);
        assert!(result.elapsed_ms >= 12);
    }

    #[tokio::test]
    async fn poll_preserves_explicit_condition_failure() {
        let result = poll_until_with(Duration::from_secs(1), Duration::from_millis(1), || {
            std::future::ready(Ok(s(&["false", "", "Login Failed"])))
        })
        .await
        .unwrap();

        assert!(!result.met);
        assert_eq!(result.fail_text.as_deref(), Some("Login Failed"));
        assert_eq!(result.polls, 1);
        assert_eq!(result.failed_polls, 0);
    }

    #[tokio::test]
    async fn permanent_lua_and_lease_errors_stop_without_retry() {
        for error in [
            "Lua error: attempt to index nil",
            "lease revoked before bridge lua_eval",
            "bridge: bridge response is not JSON",
            "bridge: bridge error -32603: Lua failed",
        ] {
            let mut polls = 0;
            let outcome = poll_until_with(Duration::from_secs(1), Duration::from_millis(1), || {
                polls += 1;
                std::future::ready(Err(error.to_string()))
            })
            .await;
            assert_eq!(outcome.unwrap_err(), error);
            assert_eq!(polls, 1);
        }
    }

    #[tokio::test]
    async fn slow_positive_response_cannot_outlive_the_deadline() {
        let result = poll_until_with(
            Duration::from_millis(20),
            Duration::from_millis(1),
            || async {
                tokio::time::sleep(Duration::from_millis(80)).await;
                Ok(s(&["true", "", ""]))
            },
        )
        .await
        .unwrap();
        assert!(!result.met);
        assert_eq!(result.polls, 1);
        assert_eq!(result.failed_polls, 0);
    }

    #[test]
    fn only_known_bridge_transport_errors_are_transient() {
        for error in [
            "bridge: bridge error -32603: dispatch timeout",
            "bridge: bridge error -32001: bridge dispatch queue full",
            "bridge: bridge connection reset while authenticating; client may be loading",
            "bridge: connection aborted by host (os error 10053)",
            "bridge: early eof",
        ] {
            assert!(transient_poll_error(error), "{error}");
        }
        for error in [
            "lease revoked: dispatch timeout",
            "Lua error: dispatch timeout",
            "bridge: bridge error -32603: Lua connection reset",
            "bridge: bridge response had neither result nor error",
        ] {
            assert!(!transient_poll_error(error), "{error}");
        }
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
        assert!(parse_poll(&s(&["true"])).is_err());
        assert!(parse_poll(&s(&["garbage", "", ""])).is_err());
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
