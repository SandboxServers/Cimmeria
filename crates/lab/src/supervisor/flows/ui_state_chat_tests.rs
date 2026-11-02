//! `client_ui_state` chat section against a post-relog tab layout.

use crate::supervisor::flows::ui_state::{parse_ui_state, ui_state_chunk};

const FIXTURE: &str = r#"
function tab(visible, lines)
  local items = {}
  for i, l in ipairs(lines) do items[i] = { getText = function() return l end } end
  local t = { isVisible = function() return visible end }
  local o = {
    getItemCount = function() return #items end,
    getItemFromIndex = function(_, i) return items[i + 1] end,
  }
  return t, o
end
"#;

fn read(setup: &str, chat_lines: u32) -> serde_json::Value {
    let lua = mlua::Lua::new();
    lua.load(FIXTURE).exec().unwrap();
    lua.load(setup).exec().unwrap();
    let vals: mlua::MultiValue = lua.load(ui_state_chunk(chat_lines)).eval().unwrap();
    let lines: Vec<String> = vals
        .iter()
        .map(|v| v.to_string().unwrap_or_default())
        .collect();
    parse_ui_state(&lines)
}

/// Regression (2026-11, DR-07 canary): after a relog Inst1 is empty and the
/// replies land in the visible Inst9 tab; the hard-coded Inst1 read returned [].
#[test]
fn reads_the_visible_tab_not_inst1() {
    let v = read(
        r#"
Inst1Chat_Tab, Inst1Chat_TabOutput = tab(false, {})
Inst8Chat_Tab, Inst8Chat_TabOutput = tab(false, {"old"})
Inst9Chat_Tab, Inst9Chat_TabOutput = tab(true, {"Welcome", "New Mission"})
"#,
        10,
    );
    assert_eq!(
        v["chat_tail"],
        serde_json::json!(["Welcome", "New Mission"])
    );
    assert_eq!(v["chat_tab"], "Inst9Chat_TabOutput");
}

#[test]
fn falls_back_to_highest_tab_with_items_then_inst1() {
    let v = read(
        r#"
Inst1Chat_Tab, Inst1Chat_TabOutput = tab(false, {"a"})
Inst5Chat_Tab, Inst5Chat_TabOutput = tab(false, {"b"})
Inst9Chat_Tab, Inst9Chat_TabOutput = tab(false, {})
"#,
        10,
    );
    assert_eq!(v["chat_tail"], serde_json::json!(["b"]));
    assert_eq!(v["chat_tab"], "Inst5Chat_TabOutput");

    let v = read(r#"Inst1Chat_Tab, Inst1Chat_TabOutput = tab(false, {})"#, 10);
    assert_eq!(v["chat_tail"], serde_json::json!([]));
    assert_eq!(v["chat_tab"], "Inst1Chat_TabOutput");
}

#[test]
fn chat_lines_limits_the_tail() {
    let v = read(
        r#"Inst2Chat_Tab, Inst2Chat_TabOutput = tab(true, {"1","2","3"})"#,
        2,
    );
    assert_eq!(v["chat_tail"], serde_json::json!(["2", "3"]));
}
