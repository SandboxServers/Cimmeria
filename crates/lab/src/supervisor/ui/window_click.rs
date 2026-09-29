//! `client_window_click`: click a named button, the widget showing a given
//! text, or a row of a list, with a real click at its screen rectangle.
//!
//! List rows are not windows, so a row's point is found by asking the
//! list which item sits under each pixel down its centre line
//! (`getItemAtPoint`), falling back to summing item heights. After the
//! click the row's selection is read back; when the click did not select
//! it (a point estimate off by a border), the row is selected through the
//! list's own Lua (`setItemSelectState`) and the step is reported at that
//! lower native level.

use std::time::Instant;

use serde_json::{json, Value};

use super::window_read::WALK_FN;
use super::{NativeLevel, NativeTrail, Supervisor};
use crate::supervisor::flows::widgets::lua_quote;

/// What to click.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClickTarget {
    /// A named window (`Loot_LootAllButton`), clicked at its centre.
    Named(String),
    /// The visible descendant of `root` whose text equals (else contains)
    /// `text`, case-insensitive.
    Text { root: String, text: String },
    /// A row of a Listbox or MultiColumnList, by index (0-based) or by text.
    Row {
        list: String,
        index: Option<u32>,
        text: Option<String>,
    },
}

/// Lua body that locates a target and returns
/// `{found, name, text, enabled, visible, point, rect, method, item_found}`.
pub fn locate_chunk(target: &ClickTarget) -> String {
    match target {
        ClickTarget::Named(name) => format!(
            "{WALK_FN}\nlocal w = __lab_find({n})\n\
             if w == nil then return __jenc({{ found = false }}) end\n\
             local r = __jcall(function() return w:getUnclippedPixelRect() end)\n\
             return __jenc({{ found = true, name = {n}, text = __jcall(function() return w:getText() end),\n\
               visible = __jcall(function() return w:isVisible() end), enabled = not __jcall(function() return w:isDisabled() end),\n\
               rect = r and {{ r.left, r.top, r.right, r.bottom }}, method = 'rect_centre' }})",
            n = lua_quote(name)
        ),
        ClickTarget::Text { root, text } => format!(
            r#"{WALK_FN}
local root = __lab_find({root})
if root == nil then return __jenc({{ found = false, error = 'no window named ' .. {root} }}) end
local want = string.lower({text})
local exact, partial = nil, nil
local function visit(x)
  if exact then return end
  if not __jcall(function() return x:isVisible() end) then return end
  local t = __jcall(function() return x:getText() end)
  if type(t) == 'string' and t ~= '' then
    local lt = string.lower(t)
    if lt == want then exact = x elseif partial == nil and string.find(lt, want, 1, true) then partial = x end
  end
  local cc = __jcall(function() return x:getChildCount() end) or 0
  for i = 0, cc - 1 do local c = __jcall(function() return x:getChildAtIdx(i) end) if c then visit(c) end end
end
visit(root)
local hit = exact or partial
if hit == nil then return __jenc({{ found = false }}) end
local r = __jcall(function() return hit:getUnclippedPixelRect() end)
return __jenc({{ found = true, name = __jcall(function() return hit:getName() end), text = __jcall(function() return hit:getText() end),
  visible = true, enabled = not __jcall(function() return hit:isDisabled() end), exact = exact ~= nil,
  rect = r and {{ r.left, r.top, r.right, r.bottom }}, method = 'text_match' }})"#,
            root = lua_quote(root),
            text = lua_quote(text)
        ),
        ClickTarget::Row { list, index, text } => format!(
            r#"{WALK_FN}
local lb = __lab_find({list})
if lb == nil then return __jenc({{ found = false, error = 'no list named ' .. {list} }}) end
local ty = __jcall(function() return lb:getType() end) or ''
local mcl = string.find(ty, 'MultiColumnList') ~= nil
local want_index, want_text = {index}, {text}
local function item_at(i)
  if mcl then return __jcall(function() return lb:getItemAtGridReference(CEGUI.MCLGridRef(i, 0)) end) end
  local it = __jcall(function() return lb:getListboxItemFromIndex(i) end)
  if it == nil then it = __jcall(function() return lb:getItemFromIndex(i) end) end
  return it
end
local n = (mcl and __jcall(function() return lb:getRowCount() end)) or __jcall(function() return lb:getItemCount() end) or 0
local row, item = nil, nil
for i = 0, n - 1 do
  local it = item_at(i)
  if it then
    local hit = (want_index ~= nil and i == want_index)
    if want_index == nil and want_text ~= nil then
      local t = string.lower(__jcall(function() return it:getText() end) or '')
      hit = string.find(t, string.lower(want_text), 1, true) ~= nil
    end
    if hit then row, item = i, it break end
  end
end
if item == nil then return __jenc({{ found = true, item_found = false, row_count = n }}) end
local function scan()
  local r = lb:getUnclippedPixelRect()
  local cx = math.floor((r.left + r.right) / 2)
  local ys, ye = nil, nil
  for y = math.floor(r.top), math.floor(r.bottom), 2 do
    local at = __jcall(function() return lb:getItemAtPoint(CEGUI.Vector2(cx, y)) end)
    if at == nil then at = __jcall(function() return lb:getItemAtPoint(CEGUI.Point(cx, y)) end) end
    if at ~= nil and at == item then if ys == nil then ys = y end ye = y elseif ys ~= nil then break end
  end
  if ys then return {{ cx, math.floor((ys + ye) / 2) }} end
  return nil
end
local point, method, scrolled = scan(), 'item_at_point', false
if point == nil then
  scrolled = __jcall(function() lb:ensureItemIsVisible(item) return true end) or false
  point = scan()
end
if point == nil then
  local r = lb:getUnclippedPixelRect()
  local y = r.top
  if mcl then
    local hdr = __jcall(function() return lb:getListHeader():getPixelSize().height end) or 0
    y = y + hdr
    for i = 0, row - 1 do y = y + (__jcall(function() return lb:getHighestRowItemHeight(i) end) or 0) end
    y = y + (__jcall(function() return lb:getHighestRowItemHeight(row) end) or 0) / 2
  else
    for i = 0, row - 1 do local it = item_at(i) y = y + (__jcall(function() return it:getPixelSize().height end) or 0) end
    y = y + (__jcall(function() return item:getPixelSize().height end) or 0) / 2
  end
  local sb = __jcall(function() return lb:getVertScrollbar() end)
  y = y - ((sb and __jcall(function() return sb:getScrollPosition() end)) or 0)
  point, method = {{ math.floor((r.left + r.right) / 2), math.floor(y) }}, 'height_estimate'
end
return __jenc({{ found = true, item_found = true, row = row, text = __jcall(function() return item:getText() end),
  visible = __jcall(function() return lb:isVisible() end), enabled = not __jcall(function() return lb:isDisabled() end),
  point = point, method = method, scrolled_into_view = scrolled, multi_column = mcl }})"#,
            list = lua_quote(list),
            index = index.map_or("nil".to_string(), |i| i.to_string()),
            text = text.as_deref().map_or("nil".to_string(), lua_quote)
        ),
    }
}

/// Lua body: is row `row` of `list` selected (`{selected}`), and, when
/// `force` is set, select it through the list's own API first.
pub fn row_selected_chunk(list: &str, row: u32, force: bool) -> String {
    format!(
        r#"{WALK_FN}
local lb = __lab_find({list})
if lb == nil then return __jenc({{ selected = false }}) end
local mcl = string.find(__jcall(function() return lb:getType() end) or '', 'MultiColumnList') ~= nil
local item
if mcl then item = __jcall(function() return lb:getItemAtGridReference(CEGUI.MCLGridRef({row}, 0)) end)
else item = __jcall(function() return lb:getListboxItemFromIndex({row}) end) or __jcall(function() return lb:getItemFromIndex({row}) end) end
if item == nil then return __jenc({{ selected = false }}) end
if {force} then
  if mcl then __jcall(function() lb:setItemSelectState(CEGUI.MCLGridRef({row}, 0), true) end)
  else __jcall(function() lb:setItemSelectState(item, true) end) end
end
return __jenc({{ selected = __jcall(function() return item:isSelected() end) == true }})"#,
        list = lua_quote(list)
    )
}

/// The point to click from a locate result: an explicit `point`, else the
/// centre of `rect`.
pub fn click_point(loc: &Value) -> Option<(i32, i32)> {
    let num = |v: &Value| v.as_f64();
    if let Some(p) = loc["point"].as_array() {
        return Some((
            num(p.first()?)?.round() as i32,
            num(p.get(1)?)?.round() as i32,
        ));
    }
    let r = loc["rect"].as_array()?;
    let (l, t, rr, b) = (num(&r[0])?, num(&r[1])?, num(&r[2])?, num(&r[3])?);
    Some((
        ((l + rr) / 2.0).round() as i32,
        ((t + b) / 2.0).round() as i32,
    ))
}

/// Why a located target cannot be clicked, if it cannot.
pub fn refuse(target: &ClickTarget, loc: &Value) -> Option<String> {
    let what = match target {
        ClickTarget::Named(n) => n.clone(),
        ClickTarget::Text { root, text } => format!("text {text:?} in {root}"),
        ClickTarget::Row { list, index, text } => {
            format!(
                "row {:?} of {list}",
                text.clone().or(index.map(|i| i.to_string()))
            )
        }
    };
    if loc["found"] != json!(true) {
        let e = loc["error"].as_str().unwrap_or("not found");
        return Some(format!("{what}: {e}"));
    }
    if loc["item_found"] == json!(false) {
        return Some(format!(
            "{what}: no such row (the list has {} rows)",
            loc["row_count"]
        ));
    }
    if loc["visible"] == json!(false) {
        return Some(format!("{what} is not visible"));
    }
    if loc["enabled"] == json!(false) {
        return Some(format!("{what} is disabled"));
    }
    if click_point(loc).is_none() {
        return Some(format!("{what} has no screen rectangle"));
    }
    None
}

impl Supervisor {
    /// `client_window_click`.
    pub async fn window_click(
        &self,
        target: ClickTarget,
        button: usize,
        double: bool,
        allow_lua_fallback: bool,
    ) -> Result<Value, String> {
        let t0 = Instant::now();
        let mut trail = NativeTrail::default();
        let loc = self.lua_json(&locate_chunk(&target)).await?;
        if let Some(why) = refuse(&target, &loc) {
            return Err(why);
        }
        if loc["scrolled_into_view"] == json!(true) {
            trail.push("scroll_row_into_view", NativeLevel::ClientUiLua);
        }
        let (x, y) = click_point(&loc).expect("refuse() checked the point");
        let at = self.click_at(x, y, button, double).await?;
        trail.push("click", NativeLevel::RealInput);
        let mut out = json!({
            "target": loc["name"].clone(),
            "text": loc["text"].clone(),
            "method": loc["method"].clone(),
            "clicked_at": [at.0, at.1],
            "button": button,
            "double": double,
        });
        if let ClickTarget::Row { list, .. } = &target {
            let row = loc["row"].as_u64().unwrap_or_default() as u32;
            out["row"] = json!(row);
            // Give the list a frame to apply the click.
            tokio::time::sleep(std::time::Duration::from_millis(120)).await;
            let sel = self.lua_json(&row_selected_chunk(list, row, false)).await?;
            let mut selected = sel["selected"] == json!(true);
            if !selected && allow_lua_fallback {
                let forced = self.lua_json(&row_selected_chunk(list, row, true)).await?;
                selected = forced["selected"] == json!(true);
                trail.push("select_row_via_list_lua", NativeLevel::ClientUiLua);
                out["fallback"] =
                    json!("the click did not select the row; selected through setItemSelectState");
            }
            out["selected"] = json!(selected);
            if !selected {
                return Err(format!(
                    "row {row} of {list} is not selected after a click at ({}, {}) ({})",
                    at.0, at.1, loc["method"]
                ));
            }
        }
        trail.stamp(&mut out);
        out["elapsed_ms"] = json!(t0.elapsed().as_millis() as u64);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_point_prefers_the_point_then_the_rect_centre() {
        assert_eq!(
            click_point(&json!({ "point": [10.4, 20.6] })),
            Some((10, 21))
        );
        assert_eq!(
            click_point(&json!({ "rect": [100, 200, 300, 240] })),
            Some((200, 220))
        );
        assert_eq!(click_point(&json!({})), None);
    }

    #[test]
    fn refusals_name_the_target_and_the_reason() {
        let n = ClickTarget::Named("Trainer_TrainBtn".into());
        let disabled =
            json!({ "found": true, "visible": true, "enabled": false, "rect": [0, 0, 2, 2] });
        assert_eq!(
            refuse(&n, &disabled).unwrap(),
            "Trainer_TrainBtn is disabled"
        );
        let hidden =
            json!({ "found": true, "visible": false, "enabled": true, "rect": [0, 0, 2, 2] });
        assert!(refuse(&n, &hidden).unwrap().contains("not visible"));
        assert!(refuse(&n, &json!({ "found": false }))
            .unwrap()
            .contains("not found"));
        let row = ClickTarget::Row {
            list: "Trainer_Choices".into(),
            index: None,
            text: Some("Snipe".into()),
        };
        let none = json!({ "found": true, "item_found": false, "row_count": 3 });
        assert!(refuse(&row, &none).unwrap().contains("the list has 3 rows"));
        let ok = json!({ "found": true, "item_found": true, "visible": true, "enabled": true, "point": [5, 5] });
        assert!(refuse(&row, &ok).is_none());
    }

    #[test]
    fn locate_chunks_quote_their_inputs() {
        let t = locate_chunk(&ClickTarget::Text {
            root: "GreetWin".into(),
            text: "Tell me \"more\"".into(),
        });
        assert!(t.contains(r#"string.lower("Tell me \"more\"")"#));
        let r = locate_chunk(&ClickTarget::Row {
            list: "Trainer_Choices".into(),
            index: Some(2),
            text: None,
        });
        assert!(r.contains("local want_index, want_text = 2, nil"));
        assert!(r.contains("getItemAtPoint(CEGUI.Vector2(cx, y))"));
        let n = locate_chunk(&ClickTarget::Named("Loot_LootAllButton".into()));
        assert!(n.contains(r#"__lab_find("Loot_LootAllButton")"#));
    }

    #[test]
    fn row_select_chunk_only_forces_when_asked() {
        assert!(row_selected_chunk("L", 1, false).contains("if false then"));
        assert!(row_selected_chunk("L", 1, true).contains("if true then"));
    }
}
