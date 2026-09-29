//! `client_window_read`: read any CEGUI window's subtree (names, types,
//! visibility, enabled state, text, screen rectangles, list rows) plus,
//! for the windows UAT needs, the owning module's state through the stock
//! bindings ([`super::typed`]).
//!
//! The walk is one Lua chunk: every node is read under `pcall` so a widget
//! that lacks a method (a list without `getItemCount`) never sinks the
//! read. Hidden children are skipped unless asked for; the node count is
//! capped so a huge window cannot flood the bridge.

use std::time::Instant;

use serde_json::{json, Value};

use super::lua_json::list;
use super::typed::{self, TypedReader};
use super::{stamp_read, Supervisor};
use crate::supervisor::flows::widgets::lua_quote;

/// Walk limits and switches for one read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalkOptions {
    pub max_depth: u32,
    pub max_nodes: u32,
    pub include_hidden: bool,
    pub max_rows: u32,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self {
            max_depth: 8,
            max_nodes: 400,
            include_hidden: false,
            max_rows: 100,
        }
    }
}

/// Lua (after the JSON prelude) that defines `__lab_walk(name, ...)`,
/// returning `{window, found, visible, nodes, truncated}`. Shared by the
/// generic read and the typed readers (which walk each of their roots).
pub const WALK_FN: &str = r#"
local function __lab_find(name)
  local p, c = string.match(name, '^([^/]+)/(.+)$')
  if p then
    local pw = _G[p] or __jcall(getWindow, p)
    return pw and __jcall(function() return pw:getChildRecursive(c) end)
  end
  local w = _G[name]
  if w == nil then w = __jcall(getWindow, name) end
  return w
end
local function __lab_rows(x, ty, node, maxrows)
  if string.find(ty, 'MultiColumnList') then
    local rc = __jcall(function() return x:getRowCount() end) or 0
    local cc = __jcall(function() return x:getColumnCount() end) or 0
    node.row_count = rc
    node.column_count = cc
    local rows = {}
    for r = 0, math.min(rc, maxrows) - 1 do
      local cells, sel = {}, nil
      for c = 0, cc - 1 do
        local it = __jcall(function() return x:getItemAtGridReference(CEGUI.MCLGridRef(r, c)) end)
        cells[c + 1] = it and (__jcall(function() return it:getText() end) or '') or ''
        if it and sel == nil then sel = __jcall(function() return it:isSelected() end) end
      end
      rows[#rows + 1] = { index = r, cells = cells, selected = sel }
    end
    node.rows = rows
  elseif string.find(ty, 'List') or string.find(ty, 'Combo') then
    local n = __jcall(function() return x:getItemCount() end)
    if type(n) ~= 'number' then return end
    node.row_count = n
    local rows = {}
    for i = 0, math.min(n, maxrows) - 1 do
      local it = __jcall(function() return x:getListboxItemFromIndex(i) end)
      if it == nil then it = __jcall(function() return x:getItemFromIndex(i) end) end
      if it ~= nil then
        rows[#rows + 1] = {
          index = i,
          text = __jcall(function() return it:getText() end) or '',
          id = __jcall(function() return it:getID() end),
          selected = __jcall(function() return it:isSelected() end),
        }
      end
    end
    node.rows = rows
  end
end
local function __lab_walk(name, maxd, maxn, hidden, maxrows)
  local w = __lab_find(name)
  if w == nil then return { window = name, found = false } end
  local nodes, truncated = {}, false
  local function visit(x, depth, parent)
    if #nodes >= maxn then truncated = true return end
    local vis = __jcall(function() return x:isVisible() end)
    if not vis and not hidden and depth > 0 then return end
    local node = {
      name = __jcall(function() return x:getName() end),
      type = __jcall(function() return x:getType() end) or '',
      depth = depth,
      parent = parent,
      visible = vis,
      enabled = not __jcall(function() return x:isDisabled() end),
      id = __jcall(function() return x:getID() end),
    }
    local txt = __jcall(function() return x:getText() end)
    if txt ~= nil and txt ~= '' then node.text = txt end
    if vis then
      local r = __jcall(function() return x:getUnclippedPixelRect() end)
      if r then node.rect = { r.left, r.top, r.right, r.bottom } end
    end
    local sel = __jcall(function() return x:isSelected() end)
    if type(sel) == 'boolean' then node.selected = sel end
    __lab_rows(x, node.type, node, maxrows)
    nodes[#nodes + 1] = node
    if depth < maxd then
      local cc = __jcall(function() return x:getChildCount() end) or 0
      for i = 0, cc - 1 do
        local c = __jcall(function() return x:getChildAtIdx(i) end)
        if c then visit(c, depth + 1, node.name) end
      end
    end
  end
  visit(w, 0, nil)
  return { window = name, found = true, visible = __jcall(function() return w:isVisible() end), nodes = nodes, truncated = truncated }
end
"#;

/// The call expression that walks `root` with `opts`.
pub fn walk_call(root: &str, opts: WalkOptions) -> String {
    format!(
        "__lab_walk({}, {}, {}, {}, {})",
        lua_quote(root),
        opts.max_depth,
        opts.max_nodes,
        opts.include_hidden,
        opts.max_rows
    )
}

/// The generic reader chunk body: one root.
pub fn read_chunk(root: &str, opts: WalkOptions) -> String {
    format!("{WALK_FN}\nreturn __jenc({})", walk_call(root, opts))
}

/// Summarise a walked subtree for a caller that wants the gist: visible
/// buttons with their enabled state, visible text, and list rows. The raw
/// `nodes` stay in the result for anything the summary leaves out.
pub fn summarize(walk: &Value) -> Value {
    let nodes = list(&walk["nodes"]);
    let visible = |n: &&Value| n["visible"].as_bool().unwrap_or(false);
    let is_button = |n: &&Value| {
        let t = n["type"].as_str().unwrap_or_default();
        t.contains("Button") || t.contains("Checkbox") || t.contains("RadioButton")
    };
    let buttons: Vec<Value> = nodes
        .iter()
        .filter(visible)
        .filter(is_button)
        .map(|n| {
            json!({
                "name": n["name"],
                "text": n.get("text").cloned().unwrap_or(Value::Null),
                "enabled": n["enabled"],
                "selected": n.get("selected").cloned().unwrap_or(Value::Null),
            })
        })
        .collect();
    let texts: Vec<Value> = nodes
        .iter()
        .filter(visible)
        .filter(|n| !is_button(n))
        .filter_map(|n| {
            n.get("text")
                .map(|t| json!({ "name": n["name"], "text": t }))
        })
        .collect();
    let lists: Vec<Value> = nodes
        .iter()
        .filter(|n| n.get("rows").is_some())
        .map(|n| {
            json!({
                "name": n["name"],
                "type": n["type"],
                "row_count": n["row_count"],
                "rows": list(&n["rows"]),
            })
        })
        .collect();
    let title = nodes
        .first()
        .and_then(|n| n.get("text"))
        .cloned()
        .unwrap_or(Value::Null);
    json!({
        "window": walk["window"],
        "found": walk["found"],
        "visible": walk["visible"],
        "title": title,
        "buttons": buttons,
        "texts": texts,
        "lists": lists,
        "node_count": nodes.len(),
        "truncated": walk["truncated"],
    })
}

/// Fold one read into the tool result: the summary, plus the raw nodes
/// when asked for.
pub fn shape_read(walk: &Value, include_nodes: bool) -> Value {
    let mut out = summarize(walk);
    if include_nodes {
        out["nodes"] = walk["nodes"].clone();
    }
    out
}

impl Supervisor {
    /// `client_window_read` for a named window.
    pub async fn window_read(
        &self,
        window: &str,
        opts: WalkOptions,
        include_nodes: bool,
    ) -> Result<Value, String> {
        let t0 = Instant::now();
        let walk = self.lua_json(&read_chunk(window, opts)).await?;
        if walk["found"] == json!(false) {
            return Err(format!("no UI window named {window}"));
        }
        let mut out = shape_read(&walk, include_nodes);
        stamp_read(&mut out, t0.elapsed().as_millis() as u64);
        Ok(out)
    }

    /// `client_window_read` with `kind`: a typed reader.
    pub async fn window_read_typed(
        &self,
        kind: &str,
        opts: WalkOptions,
        include_nodes: bool,
    ) -> Result<Value, String> {
        let reader: &TypedReader = typed::find(kind).ok_or_else(|| {
            format!(
                "unknown window kind {kind:?}; known kinds: {}",
                typed::kinds().join(", ")
            )
        })?;
        let t0 = Instant::now();
        let v = self.lua_json(&typed::chunk(reader, opts)).await?;
        let mut out = typed::shape(reader, &v, include_nodes);
        stamp_read(&mut out, t0.elapsed().as_millis() as u64);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        // Shape of a live Loot window read: frame, a text, two buttons (one
        // disabled), a named row, and a listbox with one selected row.
        json!({
            "window": "LootWin", "found": true, "visible": true, "truncated": false,
            "nodes": [
                { "name": "LootWin", "type": "DefaultFrameWindow_2", "depth": 0, "visible": true, "enabled": true, "text": "Loot" },
                { "name": "Loot_ItemText_1", "type": "StaticText_2", "depth": 2, "visible": true, "enabled": true, "text": "Naquadah Chip" },
                { "name": "Loot_LootAllButton", "type": "TextButton_2", "depth": 1, "visible": true, "enabled": true, "text": "Loot All" },
                { "name": "Loot_NextButton", "type": "ImageButton_2", "depth": 1, "visible": true, "enabled": false },
                { "name": "Loot_Hidden", "type": "TextButton_2", "depth": 1, "visible": false, "enabled": true, "text": "x" },
                { "name": "Trainer_Choices", "type": "TaharezLook/Listbox", "depth": 1, "visible": true, "enabled": true,
                  "row_count": 2, "rows": [ { "index": 0, "text": "Rapid Fire", "id": 11, "selected": true }, { "index": 1, "text": "Snipe", "id": 12, "selected": false } ] }
            ]
        })
    }

    #[test]
    fn summary_lists_visible_buttons_with_enabled_state() {
        let s = summarize(&fixture());
        assert_eq!(s["title"], "Loot");
        let buttons = s["buttons"].as_array().unwrap();
        assert_eq!(buttons.len(), 2, "the hidden button is left out");
        assert_eq!(buttons[0]["name"], "Loot_LootAllButton");
        assert_eq!(buttons[1]["enabled"], false);
        assert_eq!(s["texts"][1]["text"], "Naquadah Chip");
        assert_eq!(s["lists"][0]["rows"][0]["text"], "Rapid Fire");
        assert_eq!(s["lists"][0]["rows"][0]["selected"], true);
        assert_eq!(s["node_count"], 6);
    }

    #[test]
    fn raw_nodes_only_when_asked() {
        assert!(shape_read(&fixture(), false).get("nodes").is_none());
        assert_eq!(
            shape_read(&fixture(), true)["nodes"]
                .as_array()
                .unwrap()
                .len(),
            6
        );
    }

    /// The encoder writes an empty list as `{}`: a window with no children
    /// must still summarise.
    #[test]
    fn empty_node_tables_summarise() {
        let s = summarize(&json!({ "window": "X", "found": true, "nodes": {} }));
        assert_eq!(s["node_count"], 0);
        assert!(s["title"].is_null());
    }

    #[test]
    fn read_chunk_quotes_the_root_and_passes_the_limits() {
        let c = read_chunk(
            "Vault\"Win",
            WalkOptions {
                max_depth: 3,
                max_nodes: 50,
                include_hidden: true,
                max_rows: 7,
            },
        );
        assert!(c.contains(r#"__lab_walk("Vault\"Win", 3, 50, true, 7)"#));
        assert!(c.contains("getItemAtGridReference(CEGUI.MCLGridRef(r, c))"));
        assert!(c.trim_end().ends_with(')'));
    }
}
