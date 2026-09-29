//! Firing an ability from the Ability window (`Core/Ability/Ability.lua`).
//!
//! The window lists each training tree's abilities as `Ability_Button<i>`
//! (i = the index in `getTrainableList(tree)`), with the button's ID set to
//! the ability id; a click on a known ability runs
//! `AbilityMod.onAbilityPress` → `useAbility(id, Unit.Target)`. So a real
//! click there is a player's own way to use an ability that is not on the
//! bar. Tabs are `Ability_Tab1..3`; `AbilityMod.currentTab` is the shown
//! tree.

use std::time::Duration;

use serde_json::{json, Value};

use super::super::binding::{lab_key_for_vk, parse_binding, Binding};
use super::super::NativeLevel;
use crate::supervisor::flows::{settle, widgets, FlowError, FlowRun};
use crate::supervisor::Supervisor;

pub const ABILITY_WIN: &str = "AbilityWin";
/// The input action that toggles the window (`Actions.ToggleAbility`).
pub const TOGGLE_ACTION: &str = "ToggleAbility";

/// Lua: both bindings of an input action, encoded like the hotbar's.
pub fn action_binding_chunk(action: &str) -> String {
    let a = widgets::lua_quote(action);
    format!(
        r#"local function clean(s) if s == nil then return "" end return (string.gsub(tostring(s), "%c", " ")) end
local function keyinfo(n)
  local ok, k = pcall(getBindingKey, {a}, n)
  if not ok or type(k) ~= "table" then return "" end
  local parts = {{}}
  for kk, vv in pairs(k) do parts[#parts + 1] = clean(kk) .. "=" .. clean(vv) end
  table.sort(parts)
  return table.concat(parts, "\31")
end
return keyinfo(1), keyinfo(2)"#
    )
}

/// Lua: the window button's `ID`, visibility and enabled state.
pub fn button_state_chunk(index: u32) -> String {
    format!(
        r#"local w = _G["Ability_Button{index}"]
if w == nil then return "missing" end
return tostring(w:getID()), tostring(w:isVisible()), tostring(not w:isDisabled())"#
    )
}

/// The first bound binding the lab can press.
pub fn pressable(bindings: &[Binding]) -> Option<(Binding, String)> {
    bindings.iter().find_map(|b| {
        b.vk.filter(|_| b.is_bound())
            .and_then(lab_key_for_vk)
            .map(|k| (b.clone(), k))
    })
}

impl Supervisor {
    /// Press a binding with its modifiers held.
    pub async fn press_binding(&self, b: &Binding, key: &str) -> Result<(), String> {
        let mods = b.modifiers();
        for m in &mods {
            self.input_key(m, "down", None).await?;
        }
        let r = self.input_key(key, "tap", None).await;
        for m in mods.iter().rev() {
            self.input_key(m, "up", None).await?;
        }
        r.map(|_| ())
    }

    /// Open the Ability window, show `tree`, click `Ability_Button<index>`,
    /// and close the window again if it was closed. Returns the step detail
    /// and the level used to *open* the window (the click itself is real
    /// input).
    pub(super) async fn fire_from_ability_window(
        &self,
        run: &mut FlowRun<'_>,
        tree: u32,
        index: u32,
        ability_id: i64,
    ) -> Result<(Value, NativeLevel), FlowError> {
        let was_open = run.visible("ability_window", ABILITY_WIN).await?;
        let mut open_level = NativeLevel::RealInput;
        let mut toggle: Option<(Binding, String)> = None;
        if !was_open {
            let r = run
                .lua("toggle_binding", &action_binding_chunk(TOGGLE_ACTION))
                .await?;
            let bindings = [
                parse_binding(1, r.first().map_or("", String::as_str)),
                parse_binding(2, r.get(1).map_or("", String::as_str)),
            ];
            toggle = pressable(&bindings);
            let mut opened = false;
            if let Some((b, key)) = &toggle {
                self.press_binding(b, key)
                    .await
                    .map_err(|e| run.fail("open_ability_window", e))?;
                opened = self
                    .poll_until(
                        &widgets::visible(ABILITY_WIN),
                        None,
                        Duration::from_secs(2),
                        Duration::from_millis(200),
                    )
                    .await
                    .map_err(|e| run.fail("open_ability_window", e))?
                    .met;
            }
            if !opened {
                // No pressable binding (or it did not open): the window's
                // own toggle handler, reported as a UI Lua call.
                run.lua(
                    "open_ability_window_lua",
                    "AbilityMod.onToggleAbilityWin() return tostring(AbilityWin:isVisible())",
                )
                .await?;
                open_level = NativeLevel::UiLuaCall;
                toggle = None;
                run.wait(
                    "ability_window_open",
                    &widgets::visible(ABILITY_WIN),
                    None,
                    Duration::from_secs(3),
                )
                .await?;
            }
        }
        // Show the right tree.
        let tab = run
            .lua("current_tab", "return tostring(AbilityMod.currentTab)")
            .await?;
        if tab.first().map(String::as_str) != Some(&tree.to_string()) {
            run.click("select_tree", &format!("Ability_Tab{tree}"))
                .await?;
            run.wait(
                "tree_shown",
                &format!("AbilityMod.currentTab == {tree}"),
                None,
                Duration::from_secs(3),
            )
            .await?;
            settle(150).await;
        }
        let st = run
            .lua("ability_button", &button_state_chunk(index))
            .await?;
        match st.as_slice() {
            [id, vis, enabled] => {
                let id_ok = id.parse::<f64>().ok().map(|f| f as i64) == Some(ability_id);
                if !id_ok {
                    return Err(run
                        .fail_with_state(
                            "ability_button",
                            format!("Ability_Button{index} holds ability {id}, not {ability_id}"),
                        )
                        .await);
                }
                if vis != "true" || enabled != "true" {
                    return Err(run
                        .fail_with_state(
                            "ability_button",
                            format!(
                                "Ability_Button{index} is not clickable (visible {vis}, enabled {enabled}; training mode disables known abilities)"
                            ),
                        )
                        .await);
                }
            }
            other => {
                return Err(run.fail(
                    "ability_button",
                    format!("Ability_Button{index}: {other:?}"),
                ))
            }
        }
        run.click("click_ability", &format!("Ability_Button{index}"))
            .await?;
        // Put the window back the way it was.
        let mut closed_by = Value::Null;
        if !was_open {
            settle(150).await;
            if let Some((b, key)) = &toggle {
                if self.press_binding(b, key).await.is_ok() {
                    closed_by = json!("toggle key");
                }
            } else if self
                .ui_click(&format!("{ABILITY_WIN}/__auto_closebutton__"), 0)
                .await
                .is_ok()
            {
                closed_by = json!("close button");
            }
        }
        Ok((
            json!({
                "window_was_open": was_open,
                "opened_by": if was_open { Value::Null } else { open_level.to_json() },
                "tree": tree,
                "button": format!("Ability_Button{index}"),
                "closed_by": closed_by,
            }),
            open_level,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_name_the_stock_widgets() {
        assert!(
            action_binding_chunk(TOGGLE_ACTION).contains(r#"getBindingKey, "ToggleAbility", n"#)
        );
        let b = button_state_chunk(7);
        assert!(b.contains("Ability_Button7"));
        assert!(b.contains("w:getID()"));
    }

    #[test]
    fn pressable_skips_unbound_and_unmappable_bindings() {
        let unbound = parse_binding(1, "key=0");
        let numpad = parse_binding(1, "key=97");
        let k = parse_binding(2, "key=75");
        let (b, key) = pressable(&[unbound.clone(), numpad, k]).unwrap();
        assert_eq!(key, "K");
        assert_eq!(b.slot, 2);
        assert!(pressable(&[unbound]).is_none());
    }
}
