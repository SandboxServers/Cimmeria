//! Finding the on-screen window for an item slot, and getting it on screen.
//!
//! The client shows a slot in one of three windows (from the stock Lua):
//!
//! | Containers | Window | Slot window |
//! |---|---|---|
//! | `Main`, `Mission`, `Crafting` | `InventoryWin` (`Core/Inventory/Inventory.lua`), one tab per container, 40 visible slots, scrolled by rows of 10 | `InventoryMod.SlotWindows[InventoryMod.getVisibleSlotFor(c, s)]` |
//! | equipment (`Head` .. `Artifact2`) and `Bandolier` | `CharacterWin` (`Core/Character/Character.lua`), Equipment tab | `CharacterMod.EquippedSlots[c]`, `CharacterMod.BandolierSlots[s]` |
//! | `Vault` | `VaultWin` (`Core/Vault/Vault.lua`), open only at a banker | `VaultMod.SlotWindows[VaultMod.getVisibleSlotFor(c, s)]` |
//!
//! [`Supervisor::reveal_slot`] opens the host window with its bound key,
//! clicks the right tab and the "All" filter, and scrolls when the slot is
//! outside the 40 visible ones. Keys and clicks are real input; opening a
//! window whose toggle is unbound, and scrolling, go through the window's
//! own Lua and are reported as such.

use std::time::Duration;

use serde_json::{json, Value};

use super::inventory::ITEMS_FN;
use super::{NativeLevel, NativeTrail, Supervisor};
use crate::supervisor::flows::widgets::lua_quote;

/// A container named by its `Container.*` name or its number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerRef {
    Name(String),
    Id(i64),
}

impl ContainerRef {
    /// Parse `"Main"`, `"main"` or `"1"`.
    pub fn parse(s: &str) -> Self {
        match s.trim().parse::<i64>() {
            Ok(n) => Self::Id(n),
            Err(_) => Self::Name(s.trim().to_string()),
        }
    }

    /// Lua expression for the container id (nil when the name is unknown).
    pub fn lua_id(&self) -> String {
        match self {
            Self::Id(n) => n.to_string(),
            Self::Name(n) => format!(
                "(function() local want = string.lower({}) \
                 for k, v in pairs(Container or {{}}) do if string.lower(k) == want then return v end end \
                 return nil end)()",
                lua_quote(n)
            ),
        }
    }
}

/// Lua body: where slot `slot` of `container` is shown, or what must happen
/// first. Returns `{container, container_id, slot, host, need, click,
/// scroll_bar, scroll_to, window, rect, visible, empty, item_id, name, qty}`.
/// `need` is one of `open_host`, `click`, `scroll`, `unsupported`, or nil
/// when `window` is ready.
pub fn locate_chunk(container: &ContainerRef, slot: u32) -> String {
    format!(
        r#"{ITEMS_FN}
local cid, slot = {cid}, {slot}
local res = {{ slot = slot, container_id = cid }}
if cid == nil then res.need = 'unsupported' res.error = 'unknown container' return __jenc(res) end
res.container = __lab_cnames()[cid]
res.empty = __jcall(isSlotEmpty, cid, slot)
res.item_id = __jcall(getItemIDForSlot, cid, slot)
res.name = __jcall(getNameForSlot, cid, slot)
res.qty = __jcall(getQuantityForSlot, cid, slot)
local function vis(w) return w ~= nil and __jcall(function() return w:isVisible() end) == true end
local function done(w)
  if w == nil then res.need = 'unsupported' res.error = 'no slot window' return end
  res.window = __jcall(function() return w:getName() end)
  res.visible = vis(w)
  local r = __jcall(function() return w:getUnclippedPixelRect() end)
  if r then res.rect = {{ r.left, r.top, r.right, r.bottom }} end
end
local C = Container or {{}}
local inv = {{ [C.Main or -1] = {{ 'Inventory_GeneralTabActive', 'Inventory_GeneralTabInactive', 'Inventory_FilterAllActive', 'Inventory_FilterAllInactive' }},
              [C.Mission or -2] = {{ 'Inventory_MissionTabActive', 'Inventory_MissionTabInactive', 'Inventory_FilterMissionAllActive', 'Inventory_FilterMissionAllInactive' }},
              [C.Crafting or -3] = {{ 'Inventory_CraftingTabActive', 'Inventory_CraftingTabInactive', 'Inventory_FilterCraftingAllActive', 'Inventory_FilterCraftingAllInactive' }} }}
local equip = CharacterMod and CharacterMod.EquippedSlots and CharacterMod.EquippedSlots[cid]
if inv[cid] then
  local t = inv[cid]
  res.host, res.host_binding, res.host_toggle = 'InventoryWin', 'ToggleInventory', 'InventoryMod.onToggleInventory'
  if not vis(InventoryWin) then res.need = 'open_host' return __jenc(res) end
  if not vis(_G[t[1]]) then res.need = 'click' res.click = t[2] return __jenc(res) end
  if not vis(_G[t[3]]) then res.need = 'click' res.click = t[4] return __jenc(res) end
  local v = __jcall(InventoryMod.getVisibleSlotFor, cid, slot)
  if v == nil then res.need = 'scroll' res.scroll_bar = 'Inventory_Scrollbar' res.scroll_to = math.floor((slot - 1) / 10) return __jenc(res) end
  res.visible_slot = v
  done(InventoryMod.SlotWindows[v])
elseif cid == C.Bandolier or equip then
  res.host, res.host_binding, res.host_toggle = 'CharacterWin', 'ToggleCharacter', 'CharacterMod.onToggleCharacter'
  if not vis(CharacterWin) then res.need = 'open_host' return __jenc(res) end
  if not vis(Character_EquipmentTabActive) then res.need = 'click' res.click = 'Character_EquipmentTabInactive' return __jenc(res) end
  if cid == C.Bandolier then done(CharacterMod.BandolierSlots[slot]) else done(equip) end
elseif cid == C.Vault then
  res.host = 'VaultWin'
  if not vis(VaultWin) then res.need = 'unsupported' res.error = 'the vault window is closed; open it at a banker' return __jenc(res) end
  local v = __jcall(VaultMod.getVisibleSlotFor, cid, slot)
  if v == nil then res.need = 'scroll' res.scroll_bar = 'Vault_Scrollbar' res.scroll_to = math.floor((slot - 1) / 10) return __jenc(res) end
  res.visible_slot = v
  done(VaultMod.SlotWindows[v])
else
  res.need = 'unsupported'
  res.error = 'no window shows this container'
end
return __jenc(res)"#,
        cid = container.lua_id(),
    )
}

/// Lua body that sets a scrollbar's position (the list refreshes from its
/// own scroll-changed handler).
pub fn scroll_chunk(bar: &str, pos: i64) -> String {
    format!(
        "local b = _G[{b}] if b == nil then return __jenc({{ ok = false }}) end \
         local ok = pcall(function() b:setScrollPosition({pos}) end) \
         return __jenc({{ ok = ok }})",
        b = lua_quote(bar)
    )
}

/// Lua body that calls a host window's toggle handler (`InventoryMod.onToggleInventory`).
pub fn toggle_chunk(handler: &str) -> String {
    let path: Vec<&str> = handler.split('.').collect();
    let lookup = path
        .iter()
        .skip(1)
        .fold(format!("_G[{}]", lua_quote(path[0])), |acc, p| {
            format!("({acc} or {{}})[{}]", lua_quote(p))
        });
    format!(
        "local f = {lookup} local ok = type(f) == 'function' and pcall(f, nil) \
         return __jenc({{ ok = ok == true }})"
    )
}

/// How many reveal rounds (open, tab, filter, scroll) before giving up.
pub const MAX_REVEAL_ROUNDS: u32 = 5;

impl Supervisor {
    /// Get slot `slot` of `container` on screen and return its locate
    /// result (with `window` and `rect`). Records each step's native level.
    pub async fn reveal_slot(
        &self,
        container: &ContainerRef,
        slot: u32,
        trail: &mut NativeTrail,
    ) -> Result<Value, String> {
        let mut last = Value::Null;
        for _ in 0..MAX_REVEAL_ROUNDS {
            let loc = self.lua_json(&locate_chunk(container, slot)).await?;
            let need = loc["need"].as_str().map(str::to_string);
            match need.as_deref() {
                None => {
                    if loc["visible"] != json!(true) {
                        return Err(format!(
                            "slot window {} for {} slot {slot} is not visible",
                            loc["window"], loc["container"]
                        ));
                    }
                    return Ok(loc);
                }
                Some("open_host") => {
                    let binding = loc["host_binding"].as_str().unwrap_or_default();
                    match self.press_ui_binding(binding).await {
                        Ok(key) => trail.push(
                            format!("open {} ({key})", loc["host"]),
                            NativeLevel::RealInput,
                        ),
                        Err(e) => {
                            let handler = loc["host_toggle"].as_str().unwrap_or_default();
                            let r = self.lua_json(&toggle_chunk(handler)).await?;
                            if r["ok"] != json!(true) {
                                return Err(format!("could not open {}: {e}", loc["host"]));
                            }
                            trail.push(
                                format!("open {} via {handler}", loc["host"]),
                                NativeLevel::ClientUiLua,
                            );
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(300)).await;
                }
                Some("click") => {
                    let w = loc["click"].as_str().unwrap_or_default().to_string();
                    self.ui_click(&w, 0).await?;
                    trail.push(format!("click {w}"), NativeLevel::RealInput);
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
                Some("scroll") => {
                    let bar = loc["scroll_bar"].as_str().unwrap_or_default();
                    let pos = loc["scroll_to"].as_i64().unwrap_or_default();
                    let r = self.lua_json(&scroll_chunk(bar, pos)).await?;
                    if r["ok"] != json!(true) {
                        return Err(format!("could not scroll {bar} to {pos}"));
                    }
                    trail.push(
                        format!("scroll {bar} to row {pos}"),
                        NativeLevel::ClientUiLua,
                    );
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
                Some(_) => {
                    return Err(format!(
                        "{} slot {slot}: {}",
                        loc["container"],
                        loc["error"].as_str().unwrap_or("cannot be shown")
                    ))
                }
            }
            last = loc;
        }
        Err(format!(
            "slot {slot} still not on screen after {MAX_REVEAL_ROUNDS} rounds (last state {last})"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_refs_parse_names_and_numbers() {
        assert_eq!(
            ContainerRef::parse("Main"),
            ContainerRef::Name("Main".into())
        );
        assert_eq!(ContainerRef::parse(" 7 "), ContainerRef::Id(7));
        assert_eq!(ContainerRef::Id(3).lua_id(), "3");
        let l = ContainerRef::Name("bandolier".into()).lua_id();
        assert!(l.contains(r#"string.lower("bandolier")"#));
        assert!(l.contains("pairs(Container or {})"));
    }

    #[test]
    fn locate_chunk_covers_the_three_hosts() {
        let c = locate_chunk(&ContainerRef::Name("Main".into()), 12);
        assert!(c.contains("local cid, slot = (function()"));
        assert!(c.contains(", 12\n"));
        for w in [
            "InventoryMod.SlotWindows[v]",
            "CharacterMod.BandolierSlots[slot]",
            "VaultMod.SlotWindows[v]",
        ] {
            assert!(c.contains(w), "{w}");
        }
        assert!(c.contains("'Inventory_FilterAllInactive'"));
    }

    #[test]
    fn toggle_chunk_walks_the_dotted_path() {
        let c = toggle_chunk("InventoryMod.onToggleInventory");
        assert!(c.contains(r#"(_G["InventoryMod"] or {})["onToggleInventory"]"#));
    }

    #[test]
    fn scroll_chunk_quotes_the_bar() {
        assert!(scroll_chunk("Inventory_Scrollbar", 2).contains(r#"_G["Inventory_Scrollbar"]"#));
        assert!(scroll_chunk("Inventory_Scrollbar", 2).contains("setScrollPosition(2)"));
    }
}
