//! `client_type_text` judged by the edit box it typed into (AX-46).
//!
//! Typing proves only that the keys were posted. With an `into` edit box the
//! box is clicked first and read back afterwards: it must hold the text that
//! was typed. Without one nothing is read back, so the result is
//! `posted_only`.

use std::time::Duration;

use serde_json::{json, Value};

use super::click_expect::ClickExpect;
use crate::proof::{Check, EvVerdict, Evidence, Source, Why};
use crate::supervisor::flows::widgets::lua_quote;
use crate::supervisor::Supervisor;

/// Lua body that reads the text of edit box `name`: `{ done, text }`, where
/// `done` is false when the window is missing or has no text.
pub fn read_text_chunk(name: &str) -> String {
    format!(
        "local w local p, c = string.match({n}, '^([^/]+)/(.+)$') if p then local pw = _G[p] or __jcall(getWindow, p) w = pw and __jcall(function() return pw:getChildRecursive(c) end) else w = _G[{n}] if w == nil then w = __jcall(getWindow, {n}) end end \
         local t = w ~= nil and __jcall(function() return w:getText() end) or nil \
         return __jenc({{ done = type(t) == 'string', text = t or '' }})",
        n = lua_quote(name)
    )
}

/// Pure: the evidence of one typing step. With `into`, the edit box must
/// hold the typed text afterwards; a failed read is `no_post_read`. Without
/// `into` nothing is read back, so the result is `posted_only`.
pub fn typed_evidence(into: Option<&str>, typed: &str, read_back: Option<&str>) -> Evidence {
    if into.is_none() {
        return Evidence::from_verdict(EvVerdict::Inconclusive, Why::PostedOnly, vec![]);
    }
    let check = Check::new(
        "text_entered",
        Source::Client,
        read_back.map(|t| t.contains(typed)),
        true,
    )
    .with_after(json!(read_back));
    let check = if read_back.is_none() {
        check.with_why(Why::NoPostRead)
    } else {
        check
    };
    Evidence::judge(vec![check])
}

impl Supervisor {
    /// The text of edit box `name` now, or `None` when the read failed.
    pub async fn read_edit_box(&self, name: &str) -> Option<String> {
        let v = self.lua_json(&read_text_chunk(name)).await.ok()?;
        if v["done"] == json!(true) {
            v["text"].as_str().map(str::to_string)
        } else {
            None
        }
    }

    /// Read edit box `name` until it holds `typed`, for about a second: the
    /// keys are posted, and the client applies them on its own frames.
    /// Returns the last read.
    pub async fn read_until_typed(&self, name: &str, typed: &str) -> Option<String> {
        let mut last = None;
        for _ in 0..10 {
            last = self.read_edit_box(name).await;
            if last.as_deref().is_some_and(|t| t.contains(typed)) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        last
    }

    /// `client_type_text`: click the edit box `into` (when given) with no
    /// expectation, type `text`, then read the box back. The result carries
    /// `typed_chars`, `into`, `read_back` (when read) and `evidence`; with
    /// `into`, the clicked box's `native_level` too.
    pub async fn type_into(&self, into: Option<&str>, text: &str) -> Result<Value, String> {
        let mut out = json!({ "typed_chars": text.chars().count() });
        if let Some(w) = into {
            let clicked = self.ui_click(w, 0, &ClickExpect::default()).await?;
            out["into"] = json!(w);
            if !clicked["native_level"].is_null() {
                out["native_level"] = clicked["native_level"].clone();
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        self.type_text(text).await?;
        let read_back = match into {
            Some(w) => self.read_until_typed(w, text).await,
            None => None,
        };
        if let Some(t) = &read_back {
            out["read_back"] = json!(t);
        }
        out["evidence"] = typed_evidence(into, text, read_back.as_deref()).to_value();
        Ok(out)
    }
}

#[cfg(test)]
#[path = "type_into_tests.rs"]
mod tests;
