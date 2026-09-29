//! `client_hotbar`: the action bar as the stock UI holds it.
//!
//! The bar is client-local Lua (the server keeps no copy):
//! `ActionButtonMod.buttons[buttonId]` holds each registered button (ids
//! 1..100, windows `ActionButtons_<id>Button`) and its `actionId`;
//! `getActionInfo(actionId)` gives the action's type (`ActionType.Ability`,
//! item, macro, ...), its `subId` (the ability id for an ability), name,
//! quantity and cooldown; `getBindingKey('ActionButton<id>', 1|2)` gives
//! the keys a player presses for it (`ActionButtons.lua`,
//! `ActionProfiles.lua`). One read, pure parse.

use serde_json::{json, Value};

use super::binding::{parse_binding, Binding};
use crate::supervisor::Supervisor;

/// Button ids the stock profile code registers (`ActionProfileMod.MaxButtons`).
pub const MAX_BUTTONS: u32 = 100;

/// The Lua that reads the bar. One `button` line per registered button
/// (bound ones only unless `include_empty`), plus a `profile` line.
pub fn hotbar_chunk(include_empty: bool) -> String {
    format!(
        r#"local out = {{}}
local function add(...) out[#out + 1] = table.concat({{...}}, "\t") end
local function clean(s) if s == nil then return "" end return (string.gsub(tostring(s), "%c", " ")) end
local function enum_name(tbl, v)
  if type(tbl) ~= "table" then return "" end
  for k, x in pairs(tbl) do if x == v then return tostring(k) end end
  return ""
end
if type(ActionButtonMod) ~= "table" or type(ActionButtonMod.buttons) ~= "table" then
  return "error\tActionButtonMod is not loaded (is the client in the world?)"
end
local layer = ""
pcall(function() layer = GActionProfiles[GActionCurrentProfileId].currentLayer end)
add("profile", clean(GActionCurrentProfileId), clean(layer))
local function keyinfo(i, n)
  local ok, k = pcall(getBindingKey, "ActionButton" .. i, n)
  if not ok or type(k) ~= "table" then return "" end
  local parts = {{}}
  for kk, vv in pairs(k) do parts[#parts + 1] = clean(kk) .. "=" .. clean(vv) end
  table.sort(parts)
  return table.concat(parts, "\31")
end
for i = 1, {max} do
  local bi = ActionButtonMod.buttons[i]
  if bi ~= nil then
    local win = bi.buttonWindow
    local wname, vis = "", "0"
    if win ~= nil then
      pcall(function() wname = win:getName() end)
      pcall(function() if win:isVisible() then vis = "1" end end)
    end
    local action = tonumber(bi.actionId) or 0
    local atype, tname, sub, name, qty, rem, tot = "", "", "", "", "", "", ""
    if action > 0 then
      local ok, ai = pcall(getActionInfo, action)
      if ok and type(ai) == "table" and ai.id ~= nil then
        atype = clean(ai.type) tname = enum_name(ActionType, ai.type) sub = clean(ai.subId)
        name = clean(ai.name) qty = clean(ai.quantity) rem = clean(ai.remainingCooldown) tot = clean(ai.totalCooldown)
      end
    end
    if {include_empty} or action > 0 then
      add("button", i, clean(wname), vis, action, atype, tname, sub, name, qty, rem, tot, keyinfo(i, 1), keyinfo(i, 2))
    end
  end
end
return unpack(out)"#,
        max = MAX_BUTTONS,
        include_empty = include_empty,
    )
}

/// One hotbar button.
#[derive(Debug, Clone, PartialEq)]
pub struct HotbarButton {
    pub button: u32,
    pub window: String,
    pub visible: bool,
    /// 0 = empty.
    pub action_id: i64,
    pub action_type: Option<i64>,
    pub action_type_name: String,
    pub sub_id: Option<i64>,
    pub name: String,
    pub quantity: Option<i64>,
    pub cooldown_remaining: Option<f64>,
    pub cooldown_total: Option<f64>,
    pub keys: Vec<Binding>,
}

impl HotbarButton {
    /// The ability id this button fires, when its action is an ability.
    pub fn ability_id(&self) -> Option<i64> {
        (self.action_type_name == "Ability")
            .then_some(self.sub_id)
            .flatten()
    }

    /// The first bound key slot, if any.
    pub fn bound_key(&self) -> Option<&Binding> {
        self.keys.iter().find(|b| b.is_bound())
    }

    pub fn on_cooldown(&self) -> bool {
        self.cooldown_remaining.is_some_and(|r| r > 0.0)
    }

    pub fn to_json(&self) -> Value {
        json!({
            "button": self.button,
            "window": self.window,
            "visible": self.visible,
            "action_id": self.action_id,
            "action_type": self.action_type,
            "action_type_name": self.action_type_name,
            "ability_id": self.ability_id(),
            "sub_id": self.sub_id,
            "name": self.name,
            "quantity": self.quantity.filter(|q| *q >= 0),
            "cooldown": {
                "remaining_s": self.cooldown_remaining,
                "total_s": self.cooldown_total,
                "active": self.on_cooldown(),
            },
            "keys": self.keys.iter().map(Binding::to_json).collect::<Vec<_>>(),
        })
    }
}

/// A parsed hotbar read.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Hotbar {
    pub profile: String,
    pub layer: String,
    pub buttons: Vec<HotbarButton>,
}

impl Hotbar {
    /// Buttons holding `ability_id`, visible ones first.
    pub fn buttons_for_ability(&self, ability_id: i64) -> Vec<&HotbarButton> {
        let mut v: Vec<&HotbarButton> = self
            .buttons
            .iter()
            .filter(|b| b.ability_id() == Some(ability_id))
            .collect();
        v.sort_by_key(|b| (!b.visible, b.bound_key().is_none(), b.button));
        v
    }

    /// The first visible empty button (for placing an ability).
    pub fn first_empty_visible(&self) -> Option<&HotbarButton> {
        self.buttons.iter().find(|b| b.visible && b.action_id <= 0)
    }

    pub fn to_json(&self) -> Value {
        json!({
            "profile": self.profile,
            "layer": self.layer,
            "buttons": self.buttons.iter().map(HotbarButton::to_json).collect::<Vec<_>>(),
        })
    }
}

fn opt_i64(s: &str) -> Option<i64> {
    s.parse::<f64>().ok().map(|f| f as i64)
}

fn opt_f64(s: &str) -> Option<f64> {
    s.parse::<f64>().ok().filter(|f| f.is_finite())
}

/// Parse [`hotbar_chunk`]'s results.
pub fn parse_hotbar(lines: &[String]) -> Result<Hotbar, String> {
    let mut hb = Hotbar::default();
    for line in lines {
        let f: Vec<&str> = line.split('\t').collect();
        match f.as_slice() {
            ["error", msg] => return Err(msg.to_string()),
            ["profile", p, l] => {
                hb.profile = p.to_string();
                hb.layer = l.to_string();
            }
            ["button", id, window, vis, action, atype, tname, sub, name, qty, rem, tot, k1, k2] => {
                hb.buttons.push(HotbarButton {
                    button: id.parse().map_err(|e| format!("button id {id:?}: {e}"))?,
                    window: window.to_string(),
                    visible: *vis == "1",
                    action_id: opt_i64(action).unwrap_or(0),
                    action_type: opt_i64(atype),
                    action_type_name: tname.to_string(),
                    sub_id: opt_i64(sub),
                    name: name.to_string(),
                    quantity: opt_i64(qty),
                    cooldown_remaining: opt_f64(rem),
                    cooldown_total: opt_f64(tot),
                    keys: vec![parse_binding(1, k1), parse_binding(2, k2)],
                })
            }
            _ => return Err(format!("unexpected hotbar line {line:?}")),
        }
    }
    Ok(hb)
}

impl Supervisor {
    /// Read the hotbar.
    pub async fn read_hotbar(&self, include_empty: bool) -> Result<Hotbar, String> {
        let lines = self.lua_results(&hotbar_chunk(include_empty)).await?;
        parse_hotbar(&lines)
    }

    /// `client_hotbar`.
    pub async fn hotbar(&self, include_empty: bool) -> Result<Value, String> {
        let t0 = std::time::Instant::now();
        let hb = self.read_hotbar(include_empty).await?;
        let mut out = hb.to_json();
        out["native_level"] = super::NativeLevel::UiLuaRead.to_json();
        out["elapsed_ms"] = json!(t0.elapsed().as_millis() as u64);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const US: char = '\u{1f}';

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    fn button_line(id: u32, vis: &str, action: &str, tname: &str, sub: &str, k1: &str) -> String {
        format!(
            "button\t{id}\tActionButtons_{id}Button\t{vis}\t{action}\t1\t{tname}\t{sub}\tPistol Shot\t-1\t2.5\t4\t{k1}\t"
        )
    }

    #[test]
    fn chunk_reads_buttons_actions_and_both_binding_slots() {
        let c = hotbar_chunk(false);
        assert!(c.contains("ActionButtonMod.buttons[i]"));
        assert!(c.contains("getActionInfo, action"));
        assert!(c.contains(r#"keyinfo(i, 1), keyinfo(i, 2)"#));
        assert!(c.contains("for i = 1, 100 do"));
        assert!(c.contains("if false or action > 0"));
        assert!(hotbar_chunk(true).contains("if true or action > 0"));
    }

    #[test]
    fn a_bound_ability_button_parses() {
        let k1 = format!("key=49{US}vkeyShortText=1");
        let hb = parse_hotbar(&s(&[
            "profile\t1\t1",
            &button_line(1, "1", "7", "Ability", "1100", &k1),
        ]))
        .unwrap();
        assert_eq!(hb.profile, "1");
        let b = &hb.buttons[0];
        assert_eq!(b.ability_id(), Some(1100));
        assert_eq!(b.bound_key().unwrap().vk, Some(0x31));
        assert!(b.on_cooldown());
        let j = b.to_json();
        assert_eq!(j["quantity"], Value::Null, "-1 means no count");
        assert_eq!(j["cooldown"]["total_s"], 4.0);
        assert_eq!(j["keys"][0]["lab_key"], "1");
    }

    #[test]
    fn an_item_button_has_no_ability_id() {
        let hb = parse_hotbar(&s(&[&button_line(3, "1", "9", "Item", "2893", "")])).unwrap();
        assert_eq!(hb.buttons[0].ability_id(), None);
        assert!(hb.buttons[0].bound_key().is_none());
    }

    #[test]
    fn buttons_for_an_ability_prefer_visible_and_bound() {
        let bound = "key=50".to_string();
        let hb = parse_hotbar(&s(&[
            &button_line(30, "0", "5", "Ability", "1100", &bound),
            &button_line(12, "1", "6", "Ability", "1100", ""),
            &button_line(2, "1", "7", "Ability", "1100", &bound),
            &button_line(4, "1", "0", "", "", ""),
        ]))
        .unwrap();
        let ids: Vec<u32> = hb
            .buttons_for_ability(1100)
            .iter()
            .map(|b| b.button)
            .collect();
        assert_eq!(ids, vec![2, 12, 30]);
        assert_eq!(hb.first_empty_visible().unwrap().button, 4);
    }

    #[test]
    fn an_error_line_is_the_error() {
        let e = parse_hotbar(&s(&["error\tActionButtonMod is not loaded"])).unwrap_err();
        assert!(e.contains("not loaded"));
        assert!(parse_hotbar(&s(&["junk"])).is_err());
    }
}
