//! UI readers and item tools for automated in-game UAT.
//!
//! Readers ([`window_read`], [`chat_log`], [`inventory`], [`player_state`])
//! read the stock client UI's own state through its Lua bindings
//! (`getItemIDForSlot`, `getUnitStat`, `getEffectInfo`, the CEGUI window
//! tree). Actions ([`item_action`], [`drag_drop`], the window clicks) drive
//! the client the way a player does: the UI cursor on a widget's screen
//! rectangle, then real button and key messages through the lab's input
//! path ([`super::input`]). When an action cannot be done natively it
//! falls back one level and says so.
//!
//! Every result carries the **native level** it used ([`NativeLevel`]):
//! real input, then a slash command typed into chat, then a call into the
//! client UI's Lua, then a server shortcut. A step list records the level
//! of each step and the overall level is the least native one, so a UAT
//! verdict can refuse a pass that was not driven natively.

pub mod chat_log;
pub mod drag_drop;
pub mod inventory;
pub mod item_action;
pub mod lua_json;
pub mod player_state;
pub mod slots;
pub mod typed;
pub mod window_click;
pub mod window_read;

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde_json::{json, Value};

use super::flows::widgets::lua_quote;
use super::input::lparam_xy;
use super::{keys, process, Supervisor};

/// How natively a step drove the client, most native first. The derive
/// order is the ranking: a larger value is less native.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NativeLevel {
    /// Key and mouse messages through the game's own input handling.
    RealInput,
    /// A slash command typed into the chat box (the client builds the
    /// method itself).
    SlashCommand,
    /// A call into the stock UI's Lua (the code a button handler runs,
    /// minus the input event), or a read of it. The fourth level, a
    /// server-side shortcut (`server_console_exec`), is never used by these
    /// tools, so it has no variant.
    ClientUiLua,
}

impl NativeLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RealInput => "real_input",
            Self::SlashCommand => "slash_command",
            Self::ClientUiLua => "client_ui_lua",
        }
    }

    /// The UAT matrix tier (the same labels the world and combat tools use).
    pub fn tier(self) -> &'static str {
        match self {
            Self::RealInput => "N1",
            Self::SlashCommand => "N2",
            Self::ClientUiLua => "N3",
        }
    }
}

/// The per-step native levels of one tool call.
#[derive(Debug, Default, Clone)]
pub struct NativeTrail {
    steps: Vec<(String, NativeLevel)>,
}

impl NativeTrail {
    pub fn push(&mut self, step: impl Into<String>, level: NativeLevel) {
        self.steps.push((step.into(), level));
    }

    /// The least native level used, or `None` when nothing was driven.
    pub fn overall(&self) -> Option<NativeLevel> {
        self.steps.iter().map(|(_, l)| *l).max()
    }

    /// Stamp `native_level` and `native_steps` onto a result object.
    pub fn stamp(&self, out: &mut Value) {
        let overall = self.overall();
        out["native_level"] = json!(overall.map(NativeLevel::as_str));
        out["native_tier"] = json!(overall.map(NativeLevel::tier));
        // Only a step driven entirely by real input is a native pass.
        out["native_pass"] = json!(overall == Some(NativeLevel::RealInput));
        out["native_steps"] = json!(self
            .steps
            .iter()
            .map(|(s, l)| json!({ "step": s, "level": l.as_str(), "tier": l.tier() }))
            .collect::<Vec<_>>());
    }
}

/// Stamp a pure read: every reader goes through the client UI's Lua.
pub fn stamp_read(out: &mut Value, elapsed_ms: u64) {
    out["native_level"] = json!(NativeLevel::ClientUiLua.as_str());
    out["native_tier"] = json!("read");
    out["mode"] = json!("read");
    out["elapsed_ms"] = json!(elapsed_ms);
}

/// Supervisor-lifetime memory for the readers: inventory snapshots for
/// before/after diffs and the chat read cursors. One supervisor process
/// drives one client, so a process-wide store is enough.
#[derive(Debug, Default)]
pub struct UiMemory {
    pub snapshots: HashMap<String, Value>,
    pub chat_cursors: HashMap<String, u64>,
}

pub fn memory() -> &'static Mutex<UiMemory> {
    static MEM: OnceLock<Mutex<UiMemory>> = OnceLock::new();
    MEM.get_or_init(|| Mutex::new(UiMemory::default()))
}

/// Lua that returns the key a UI action is bound to, as the short text the
/// options screen shows (`"I"`, `"F5"`), or empty when it is unbound.
pub fn binding_key_chunk(action: &str) -> String {
    let a = lua_quote(action);
    format!(
        "for slot = 1, 2 do \
           local ok, k = pcall(getBindingKey, {a}, slot) \
           if ok and type(k) == 'table' and k.key and k.key > 0 then \
             return tostring(k.vkeyShortText or ''), tostring(k.key) end \
         end return '', ''"
    )
}

/// The key to press for a binding's short text: the named key when the lab
/// knows it, else the virtual-key code with its scan code looked up from
/// the lab's own table (letters and digits).
pub fn key_for_binding(short: &str, vk: &str) -> Option<keys::Key> {
    let short = short.trim();
    if !short.is_empty() {
        if let Some(k) = keys::named(short) {
            return Some(k);
        }
    }
    // VK codes 0x30..=0x39 and 0x41..=0x5A are the digit and letter keys
    // (their ASCII codes); every other code (numpad, OEM) is refused.
    let code: u8 = vk.trim().parse().ok()?;
    match code {
        b'0'..=b'9' | b'A'..=b'Z' => keys::named(&char::from(code).to_string()),
        _ => None,
    }
}

const WM_MOUSEMOVE: u32 = 0x0200;
const MK_LBUTTON: usize = 0x0001;

impl Supervisor {
    /// Run a reader chunk (the [`lua_json`] prelude is added) and parse its
    /// JSON result.
    pub async fn lua_json(&self, body: &str) -> Result<Value, String> {
        let r = self.lua_results(&lua_json::chunk(body)).await?;
        lua_json::decode(&r)
    }

    /// Put the UI cursor on `(x, y)` and click (or double-click) there
    /// with real button messages.
    pub async fn click_at(
        &self,
        x: i32,
        y: i32,
        button: usize,
        double: bool,
    ) -> Result<(i32, i32), String> {
        let at = self.move_cursor(x, y).await?;
        // A frame for the hover to register before the press.
        tokio::time::sleep(Duration::from_millis(40)).await;
        self.post_button(button, "click", at, None).await?;
        if double {
            // CEGUI makes a double click out of two presses inside its
            // double-click timeout (0.33 s by default) at the same spot.
            tokio::time::sleep(Duration::from_millis(60)).await;
            self.post_button(button, "click", at, None).await?;
        }
        Ok(at)
    }

    /// Move the UI cursor and post a matching `WM_MOUSEMOVE` (left button
    /// held when `dragging`), for drags: CEGUI starts a drag from motion.
    pub async fn drag_motion(&self, x: i32, y: i32, dragging: bool) -> Result<(i32, i32), String> {
        let at = self.move_cursor(x, y).await?;
        let hwnd = self.game_hwnd().await?;
        let wparam = if dragging { MK_LBUTTON } else { 0 };
        process::post_message(hwnd, WM_MOUSEMOVE, wparam, lparam_xy(at.0, at.1))?;
        Ok(at)
    }

    /// Press the key bound to a UI action (`ToggleInventory`,
    /// `ToggleCharacter`). Returns the key's name, or an error when the
    /// action is unbound or bound to a key the lab cannot press.
    pub async fn press_ui_binding(&self, action: &str) -> Result<String, String> {
        let r = self.lua_results(&binding_key_chunk(action)).await?;
        let short = r.first().cloned().unwrap_or_default();
        let vk = r.get(1).cloned().unwrap_or_default();
        let key = key_for_binding(&short, &vk).ok_or_else(|| {
            format!("{action} is bound to {short:?} (vk {vk:?}), which the lab cannot press")
        })?;
        let hwnd = self.game_hwnd().await?;
        self.post_key(hwnd, key, true).await?;
        tokio::time::sleep(Duration::from_millis(super::input::DEFAULT_HOLD_MS)).await;
        self.post_key(hwnd, key, false).await?;
        Ok(if short.is_empty() { vk } else { short })
    }

    /// Type a slash command into chat: Enter, the line, Enter. The line is
    /// checked against the lab's typing set before anything is pressed.
    pub async fn chat_command(&self, line: &str) -> Result<(), String> {
        keys::plan_text(line)?;
        self.input_key("Enter", "tap", None).await?;
        tokio::time::sleep(Duration::from_millis(150)).await;
        self.type_text(line).await?;
        tokio::time::sleep(Duration::from_millis(50)).await;
        self.input_key("Enter", "tap", None).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overall_level_is_the_least_native_step() {
        let mut t = NativeTrail::default();
        assert_eq!(t.overall(), None);
        t.push("open", NativeLevel::RealInput);
        t.push("scroll", NativeLevel::ClientUiLua);
        t.push("click", NativeLevel::RealInput);
        assert_eq!(t.overall(), Some(NativeLevel::ClientUiLua));
        let mut out = json!({});
        t.stamp(&mut out);
        assert_eq!(out["native_level"], "client_ui_lua");
        assert_eq!(out["native_tier"], "N3");
        assert_eq!(out["native_pass"], false);
        assert_eq!(out["native_steps"][0]["tier"], "N1");
        assert_eq!(out["native_steps"][1]["step"], "scroll");
    }

    #[test]
    fn native_levels_rank_real_input_first() {
        assert!(NativeLevel::RealInput < NativeLevel::SlashCommand);
        assert!(NativeLevel::SlashCommand < NativeLevel::ClientUiLua);
    }

    #[test]
    fn binding_keys_resolve_from_short_text_or_vk() {
        assert_eq!(key_for_binding("I", "73").unwrap().vk, b'I');
        assert_eq!(key_for_binding("", "66").unwrap().vk, b'B');
        assert_eq!(key_for_binding("F5", "116").unwrap().vk, 0x74);
        assert!(key_for_binding("", "").is_none());
        assert!(key_for_binding("Num*", "106").is_none());
    }

    #[test]
    fn binding_chunk_tries_both_binding_slots() {
        let c = binding_key_chunk("ToggleInventory");
        assert!(c.contains("pcall(getBindingKey, \"ToggleInventory\", slot)"));
        assert!(c.contains("for slot = 1, 2"));
    }
}
