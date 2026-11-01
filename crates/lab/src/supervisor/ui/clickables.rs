//! `client_ui_clickables`: list the CEGUI windows under a root with the full
//! names `getWindow` accepts, so an agent can find the stock UI's unnamed
//! `__auto_` children (LH-04 made them clickable; this makes them findable).
//!
//! One Lua walk per call, depth-first in child-index order. Windows before
//! `cursor` are stepped past reading only visibility and child indices, so a
//! page never inspects what an earlier page returned. The walk stops at
//! `limit` returned windows or [`VISIT_CAP`] inspected ones. The `CEGUI`
//! global is not exposed to the bridge, so only the global `getWindow` is used.

use serde_json::{json, Value};

use super::lua_json::list;
use super::Supervisor;
use crate::supervisor::flows::widgets::lua_quote;

/// Windows the walk inspects in one call before it reports `truncated`.
pub const VISIT_CAP: u32 = 2000;

/// Windows a page returns when the caller gives no `limit`.
pub const DEFAULT_LIMIT: u32 = 200;

/// The most windows one page may return.
const MAX_LIMIT: u32 = 500;

/// The walk body (the JSON prelude is added by `lua_json`). The `@...@`
/// placeholders are filled by [`clickables_chunk`]. Positions count every
/// window in walk order; pruned hidden subtrees take no position.
const CLICKABLES_WALK: &str = r#"
local top = __jcall(getWindow, @ROOT@)
if top == nil then return __jenc({ found = false }) end
local pos, inspected, truncated, nextpos = 0, 0, false, nil
local out, stop = {}, false
local function __lab_path(link)
  if link.path == nil then
    local nm = __jcall(function() return link.w:getName() end) or ''
    if link.up then link.path = __lab_path(link.up) .. '/' .. nm else link.path = nm end
  end
  return link.path
end
local function visit(w, up)
  if stop then return end
  local vis = __jcall(function() return w:isVisible() end) == true
  @HIDDEN_GUARD@
  local link = { w = w, up = up }
  if pos < @CURSOR@ then
    pos = pos + 1
  else
    if #out >= @LIMIT@ or inspected >= @CAP@ then
      stop, truncated, nextpos = true, true, pos
      return
    end
    pos = pos + 1
    inspected = inspected + 1
    local ty = nil
    if type(w.getType) == "function" then ty = __jcall(function() return w:getType() end) end
    local r = __jcall(function() return w:getUnclippedPixelRect() end)
    local node = {
      name = __lab_path(link),
      type = ty or '',
      text = __jcall(function() return w:getText() end) or '',
      visible = vis,
      enabled = __jcall(function() return w:isDisabled() end) ~= true,
      parent = up and __lab_path(up) or nil,
    }
    if r then node.rect = { r.left, r.top, r.right - r.left, r.bottom - r.top } end
    out[#out + 1] = node
  end
  local cc = __jcall(function() return w:getChildCount() end) or 0
  for i = 0, cc - 1 do
    if stop then return end
    local c = __jcall(function() return w:getChildAtIdx(i) end)
    if c then visit(c, link) end
  end
end
visit(top, nil)
return __jenc({ found = true, windows = out, truncated = truncated, visited = inspected, next = nextpos })
"#;

/// The Lua body for one page: walk from `root`, skip the first `cursor`
/// positions, return up to `limit` windows (clamped to 1..=500).
pub fn clickables_chunk(root: &str, include_hidden: bool, limit: u32, cursor: u32) -> String {
    let guard = if include_hidden {
        ""
    } else {
        "if not vis then return end"
    };
    CLICKABLES_WALK
        .replace("@HIDDEN_GUARD@", guard)
        .replace("@CURSOR@", &cursor.to_string())
        .replace("@LIMIT@", &limit.clamp(1, MAX_LIMIT).to_string())
        .replace("@CAP@", &VISIT_CAP.to_string())
        .replace("@ROOT@", &lua_quote(root))
}

/// One window as the tool reports it: `auto` from the name, `type` and
/// `rect` left out when the binding gave none, `parent` null for the root.
fn shape_window(mut w: Value) -> Value {
    let auto = w["name"].as_str().is_some_and(|n| n.contains("__auto_"));
    if let Some(o) = w.as_object_mut() {
        o.insert("auto".into(), json!(auto));
        let has_type = o
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|t| !t.is_empty());
        if !has_type {
            o.remove("type");
        }
        let has_rect = o
            .get("rect")
            .and_then(Value::as_array)
            .is_some_and(|r| !r.is_empty());
        if !has_rect {
            o.remove("rect");
        }
        o.entry("parent").or_insert(Value::Null);
    }
    w
}

impl Supervisor {
    /// `client_ui_clickables`: one page of the windows under `root` (default
    /// `Root`), with the paging fields `next` and `truncated`.
    pub async fn ui_clickables(
        &self,
        root: Option<&str>,
        include_hidden: bool,
        limit: Option<u32>,
        cursor: Option<u32>,
    ) -> Result<Value, String> {
        let root = root.unwrap_or("Root");
        let body = clickables_chunk(
            root,
            include_hidden,
            limit.unwrap_or(DEFAULT_LIMIT),
            cursor.unwrap_or(0),
        );
        let walk = self.lua_json(&body).await?;
        if walk["found"] == json!(false) {
            return Err(format!("no UI window named {root}"));
        }
        let windows: Vec<Value> = list(&walk["windows"])
            .into_iter()
            .map(shape_window)
            .collect();
        Ok(json!({
            "windows": windows,
            "truncated": walk["truncated"].as_bool().unwrap_or(false),
            "visited": walk["visited"].as_u64().unwrap_or(0),
            "next": walk.get("next").cloned().unwrap_or(Value::Null),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_walks_from_the_root_and_checks_get_type() {
        let c = clickables_chunk("Root", false, 9999, 0);
        assert!(c.contains("getChildCount") && c.contains("getChildAtIdx"));
        assert!(c.contains(r#"type(w.getType) == "function""#));
        assert!(c.contains(r#"__jcall(getWindow, "Root")"#));
        assert!(c.contains("#out >= 500"), "limit 9999 clamps to 500");
        assert!(clickables_chunk("Root", false, 0, 0).contains("#out >= 1"));
        assert!(c.trim_end().ends_with("})"));
    }

    #[test]
    fn hidden_windows_are_skipped_unless_asked() {
        let skipped = clickables_chunk("Root", false, 50, 0);
        assert!(skipped.contains("if not vis then return end"));
        let shown = clickables_chunk("Root", true, 50, 0);
        assert!(!shown.contains("if not vis then return end"));
    }

    #[test]
    fn chunk_carries_cursor_and_visit_cap() {
        let c = clickables_chunk("Root", false, 50, 400);
        assert!(c.contains("pos < 400"));
        assert!(c.contains("inspected >= 2000"));
        assert_eq!(VISIT_CAP, 2000);
        let at_position_check = c.find("pos < 400").unwrap();
        assert!(at_position_check < c.find("getText").unwrap());
    }
}

#[cfg(test)]
#[path = "clickables_tests.rs"]
mod call_tests;
