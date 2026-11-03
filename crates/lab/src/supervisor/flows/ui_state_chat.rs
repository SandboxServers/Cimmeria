//! The chat section of `client_ui_state`: which chat tab to read.
//!
//! After a relog the client builds new tabs (`Inst5`..`Inst9`) inside
//! `Inst1ChatWin` and writes new lines to the newest one, so the stock
//! `Inst1Chat_TabOutput` stays empty and a hard-coded read sees no replies.

/// Lua for `sec("chat", ...)`'s body. Picks, in order: the visible
/// `InstNChat_Tab` (the selected tab), else the highest-index output with
/// items, else `Inst1`. Emits `chat_tab<TAB>name` then the last `chat_lines`
/// lines as `chat<TAB>text`. Expects `add` in scope.
pub fn chat_section_lua(chat_lines: u32) -> String {
    format!(
        r#"local pick
  for i = 1, 64 do
    local t, o = _G["Inst" .. i .. "Chat_Tab"], _G["Inst" .. i .. "Chat_TabOutput"]
    if t ~= nil and o ~= nil and t:isVisible() then pick = i end
  end
  if pick == nil then
    for i = 64, 1, -1 do
      local o = _G["Inst" .. i .. "Chat_TabOutput"]
      if o ~= nil and o:getItemCount() > 0 then pick = i break end
    end
  end
  pick = pick or 1
  local lb = _G["Inst" .. pick .. "Chat_TabOutput"]
  if lb ~= nil then
    add("chat_tab", "Inst" .. pick .. "Chat_TabOutput")
    local n = lb:getItemCount()
    for i = math.max(0, n - {chat_lines}), n - 1 do add("chat", lb:getItemFromIndex(i):getText()) end
  end"#
    )
}

#[cfg(test)]
#[path = "ui_state_chat_tests.rs"]
mod tests;
