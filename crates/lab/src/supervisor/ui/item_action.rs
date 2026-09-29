//! `client_item_action`: use, equip or unequip an item, loot a row, or
//! Loot All, the way a player does.
//!
//! In the stock UI a **right-click** on an inventory, equipment or
//! bandolier slot calls `contextSensitiveUseItem(container, slot)`
//! (`Inventory.lua` / `Character.lua` `onSlotItemMouseButtonUp`): it uses a
//! consumable, equips a wearable and unequips an equipped item. So `use`,
//! `equip`, `unequip` and `rightclick` are all one real right-click on the
//! item's slot, and the result says what the inventory did (a diff taken
//! before and after). When the slot cannot be put on screen the action
//! falls back to the slash command the client parses itself (`/useitem`,
//! `/equip`), reported as such.
//!
//! Loot: Loot All is a click on `Loot_LootAllButton`; one row is a
//! double-click on its icon (`LootMod.onItemDoubleClick`), paging with the
//! Next button first; the fallback is `/lootitem <index - 1>`.

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::inventory::{diff, find_items};
use super::slots::ContainerRef;
use super::{NativeLevel, NativeTrail, Supervisor};

/// What to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemVerb {
    Use,
    Equip,
    Unequip,
    RightClick,
    DoubleClick,
    LootAll,
    LootSlot,
}

impl ItemVerb {
    pub fn parse(s: &str) -> Result<Self, String> {
        Ok(match s.to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
            "use" => Self::Use,
            "equip" => Self::Equip,
            "unequip" => Self::Unequip,
            "rightclick" | "right_click" => Self::RightClick,
            "doubleclick" | "double_click" => Self::DoubleClick,
            "loot_all" | "lootall" => Self::LootAll,
            "loot_slot" | "loot" => Self::LootSlot,
            other => {
                return Err(format!(
                    "action must be use, equip, unequip, rightclick, doubleclick, loot_all or loot_slot, not {other:?}"
                ))
            }
        })
    }

    /// The slash command for the fallback, when there is one.
    pub fn slash(self, item_id: i64, loot_index: u32) -> Option<String> {
        match self {
            Self::Use | Self::RightClick | Self::DoubleClick => {
                Some(format!("/useitem {item_id} 0"))
            }
            Self::Equip => Some(format!("/equip {item_id}")),
            Self::LootSlot => Some(format!("/lootitem {}", loot_index.saturating_sub(1))),
            Self::Unequip | Self::LootAll => None,
        }
    }
}

/// Which item.
#[derive(Debug, Clone, Default)]
pub struct ItemSelector {
    pub container: Option<String>,
    pub slot: Option<u32>,
    pub item_id: Option<i64>,
    pub name: Option<String>,
}

/// Pick one item from a read: an explicit container + slot wins, else the
/// first match by id or name. Returns `(container, slot, item_id, name)`.
pub fn select_item(read: &Value, sel: &ItemSelector) -> Result<(String, u32, i64, String), String> {
    if let (Some(c), Some(s)) = (&sel.container, sel.slot) {
        let hits = find_items(read, None, None);
        let hit = hits.iter().find(|h| {
            h["slot"].as_u64() == Some(u64::from(s))
                && (h["container"]
                    .as_str()
                    .is_some_and(|n| n.eq_ignore_ascii_case(c))
                    || h["container_id"].as_i64().map(|n| n.to_string()).as_deref()
                        == Some(c.as_str()))
        });
        return hit
            .map(|h| {
                (
                    h["container"].as_str().unwrap_or(c).to_string(),
                    s,
                    h["item_id"].as_i64().unwrap_or_default(),
                    h["name"].as_str().unwrap_or_default().to_string(),
                )
            })
            .ok_or_else(|| format!("{c} slot {s} is empty"));
    }
    if sel.item_id.is_none() && sel.name.is_none() {
        return Err("name the item: container + slot, item_id, or name".to_string());
    }
    let hits = find_items(read, sel.item_id, sel.name.as_deref());
    let h = hits.first().ok_or_else(|| {
        format!(
            "no item matching {} in any loaded container",
            sel.item_id
                .map(|i| format!("item_id {i}"))
                .or(sel.name.as_ref().map(|n| format!("name {n:?}")))
                .unwrap_or_default()
        )
    })?;
    Ok((
        h["container"].as_str().unwrap_or_default().to_string(),
        h["slot"].as_u64().unwrap_or_default() as u32,
        h["item_id"].as_i64().unwrap_or_default(),
        h["name"].as_str().unwrap_or_default().to_string(),
    ))
}

/// Loot rows per page (`LootMod.LOOTS_PER_PAGE`).
pub const LOOTS_PER_PAGE: u32 = 4;

/// `(page, row)` of a 1-based loot index: page 0-based, row 1..=4.
pub fn loot_page_row(index: u32) -> (u32, u32) {
    let i = index.max(1) - 1;
    (i / LOOTS_PER_PAGE, i % LOOTS_PER_PAGE + 1)
}

const LOOT_STATE: &str = "return __jenc({ visible = LootWin ~= nil and LootWin:isVisible(), \
     count = __jcall(getLootCount) or 0, page = LootMod and LootMod.page or 0 })";

impl Supervisor {
    /// Poll the inventory until it differs from `before` or `wait` passes.
    async fn await_inventory_change(
        &self,
        before: &Value,
        wait: Duration,
    ) -> Result<(Value, Value), String> {
        let t0 = Instant::now();
        loop {
            tokio::time::sleep(Duration::from_millis(250)).await;
            let after = self.inventory_read(&[], false).await?;
            let d = diff(before, &after);
            if d["changed"] == json!(true) || t0.elapsed() >= wait {
                return Ok((after, d));
            }
        }
    }

    /// `client_item_action`.
    pub async fn item_action(
        &self,
        verb: ItemVerb,
        sel: &ItemSelector,
        loot_index: Option<u32>,
        allow_fallback: bool,
        wait: Duration,
    ) -> Result<Value, String> {
        let t0 = Instant::now();
        let mut trail = NativeTrail::default();
        let before = self.inventory_read(&[], false).await?;
        let mut out = json!({ "action": format!("{verb:?}").to_ascii_lowercase() });
        match verb {
            ItemVerb::LootAll | ItemVerb::LootSlot => {
                let st = self.lua_json(LOOT_STATE).await?;
                if st["visible"] != json!(true) {
                    return Err(
                        "the loot window is not open (open a corpse or crate first)".to_string()
                    );
                }
                out["loot_count_before"] = st["count"].clone();
                if verb == ItemVerb::LootAll {
                    self.ui_click("Loot_LootAllButton", 0).await?;
                    trail.push("click Loot_LootAllButton", NativeLevel::RealInput);
                } else {
                    let index = loot_index.ok_or("loot_slot needs loot_index (1-based)")?;
                    let count = st["count"].as_u64().unwrap_or_default() as u32;
                    if index == 0 || index > count {
                        return Err(format!("loot_index {index} is outside 1..={count}"));
                    }
                    let (page, row) = loot_page_row(index);
                    let mut cur = st["page"].as_u64().unwrap_or_default() as u32;
                    let mut paged = true;
                    while cur != page {
                        let btn = if cur < page {
                            "Loot_NextButton"
                        } else {
                            "Loot_PrevButton"
                        };
                        if self.ui_click(btn, 0).await.is_err() {
                            paged = false;
                            break;
                        }
                        trail.push(format!("click {btn}"), NativeLevel::RealInput);
                        tokio::time::sleep(Duration::from_millis(150)).await;
                        let now = self.lua_json(LOOT_STATE).await?;
                        let next = now["page"].as_u64().unwrap_or_default() as u32;
                        if next == cur {
                            paged = false;
                            break;
                        }
                        cur = next;
                    }
                    let icon = format!("Loot_ItemIcon_{row}");
                    let native = if paged {
                        self.lua_json(&super::window_click::locate_chunk(
                            &super::window_click::ClickTarget::Named(icon.clone()),
                        ))
                        .await
                        .ok()
                        .and_then(|loc| super::window_click::click_point(&loc))
                    } else {
                        None
                    };
                    match native {
                        Some((x, y)) => {
                            self.click_at(x, y, 0, true).await?;
                            trail.push(format!("double-click {icon}"), NativeLevel::RealInput);
                        }
                        None if allow_fallback => {
                            let line = verb.slash(0, index).expect("loot has a slash command");
                            self.chat_command(&line).await?;
                            trail.push(line, NativeLevel::SlashCommand);
                        }
                        None => {
                            return Err(format!("could not page the loot window to row {index}"))
                        }
                    }
                    out["loot_index"] = json!(index);
                }
            }
            _ => {
                let (container, slot, item_id, name) = select_item(&before, sel)?;
                out["item"] = json!({ "container": container, "slot": slot, "item_id": item_id, "name": name });
                let revealed = self
                    .reveal_slot(&ContainerRef::Name(container.clone()), slot, &mut trail)
                    .await;
                match revealed {
                    Ok(loc) => {
                        let window = loc["window"].as_str().unwrap_or_default().to_string();
                        let (button, double) = match verb {
                            ItemVerb::DoubleClick => (0, true),
                            _ => (1, false),
                        };
                        let r = self
                            .lua_json(&super::window_click::locate_chunk(
                                &super::window_click::ClickTarget::Named(window.clone()),
                            ))
                            .await?;
                        let (x, y) = super::window_click::click_point(&r)
                            .ok_or_else(|| format!("{window} has no screen rectangle"))?;
                        self.click_at(x, y, button, double).await?;
                        let how = if double {
                            "double-click"
                        } else {
                            "right-click"
                        };
                        trail.push(format!("{how} {window}"), NativeLevel::RealInput);
                        out["slot_window"] = json!(window);
                    }
                    Err(e) => {
                        let line = match verb.slash(item_id, 0) {
                            Some(l) if allow_fallback => l,
                            _ => return Err(format!("could not put the item on screen: {e}")),
                        };
                        self.chat_command(&line).await?;
                        trail.push(line, NativeLevel::SlashCommand);
                        out["fallback_reason"] = json!(e);
                    }
                }
            }
        }
        let (after, d) = self.await_inventory_change(&before, wait).await?;
        if matches!(verb, ItemVerb::LootAll | ItemVerb::LootSlot) {
            let st = self.lua_json(LOOT_STATE).await?;
            out["loot_count_after"] = st["count"].clone();
            out["loot_window_open"] = st["visible"].clone();
        }
        out["inventory_changed"] = d["changed"].clone();
        out["diff"] = d;
        out["cash_after"] = after["cash"].clone();
        trail.stamp(&mut out);
        out["elapsed_ms"] = json!(t0.elapsed().as_millis() as u64);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inv() -> Value {
        json!({
            "cash": 10,
            "containers": [
                { "id": 1, "name": "Main", "size": 40, "slots": [
                    { "slot": 4, "item_id": 5001, "name": "Field Medkit", "qty": 3 },
                    { "slot": 9, "item_id": 5002, "name": "Zat'nik'tel", "qty": 1 }
                ] },
                { "id": 7, "name": "Chest", "size": 1, "slots": [ { "slot": 1, "item_id": 6001, "name": "Vest", "qty": 1 } ] }
            ]
        })
    }

    #[test]
    fn select_by_slot_id_or_name() {
        let by_slot = ItemSelector {
            container: Some("main".into()),
            slot: Some(9),
            ..Default::default()
        };
        assert_eq!(
            select_item(&inv(), &by_slot).unwrap(),
            ("Main".into(), 9, 5002, "Zat'nik'tel".into())
        );
        let by_num = ItemSelector {
            container: Some("7".into()),
            slot: Some(1),
            ..Default::default()
        };
        assert_eq!(select_item(&inv(), &by_num).unwrap().2, 6001);
        let by_id = ItemSelector {
            item_id: Some(5001),
            ..Default::default()
        };
        assert_eq!(select_item(&inv(), &by_id).unwrap().1, 4);
        let by_name = ItemSelector {
            name: Some("vest".into()),
            ..Default::default()
        };
        assert_eq!(select_item(&inv(), &by_name).unwrap().0, "Chest");
    }

    #[test]
    fn select_refuses_empty_slots_and_unknown_items() {
        let empty = ItemSelector {
            container: Some("Main".into()),
            slot: Some(1),
            ..Default::default()
        };
        assert!(select_item(&inv(), &empty).unwrap_err().contains("empty"));
        let missing = ItemSelector {
            item_id: Some(1),
            ..Default::default()
        };
        assert!(select_item(&inv(), &missing)
            .unwrap_err()
            .contains("item_id 1"));
        assert!(select_item(&inv(), &ItemSelector::default()).is_err());
    }

    #[test]
    fn verbs_parse_and_map_to_the_client_slash_commands() {
        assert_eq!(ItemVerb::parse("Loot All").unwrap(), ItemVerb::LootAll);
        assert_eq!(
            ItemVerb::parse("right-click").unwrap(),
            ItemVerb::RightClick
        );
        assert!(ItemVerb::parse("eat").is_err());
        assert_eq!(ItemVerb::Use.slash(5001, 0).unwrap(), "/useitem 5001 0");
        assert_eq!(ItemVerb::Equip.slash(6001, 0).unwrap(), "/equip 6001");
        // /lootitem is 0-based; the tool's loot_index is 1-based like the UI.
        assert_eq!(ItemVerb::LootSlot.slash(0, 3).unwrap(), "/lootitem 2");
        assert!(ItemVerb::Unequip.slash(1, 0).is_none());
        for l in ["/useitem 5001 0", "/equip 6001", "/lootitem 2"] {
            // The lab types letters, digits, space and -_/. only.
            assert!(crate::supervisor::keys::plan_text(l).is_ok(), "{l}");
        }
    }

    #[test]
    fn loot_rows_page_by_four() {
        assert_eq!(loot_page_row(1), (0, 1));
        assert_eq!(loot_page_row(4), (0, 4));
        assert_eq!(loot_page_row(5), (1, 1));
        assert_eq!(loot_page_row(10), (2, 2));
    }
}
