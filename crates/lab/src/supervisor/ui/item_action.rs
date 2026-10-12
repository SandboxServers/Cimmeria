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
use super::window_click::{click_point, locate_chunk, ClickTarget};
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

/// Lua body: the stock right-click handler, `contextSensitiveUseItem`, called
/// the way `InventoryMod.onSlotItemMouseButtonUp` calls it
/// (`Inventory.lua:753`): `{ ok }`.
pub fn use_item_chunk(container: &ContainerRef, slot: u32) -> String {
    format!(
        "local cid = {cid} if cid == nil then return __jenc({{ ok = false }}) end \
         local ok = type(contextSensitiveUseItem) == 'function' and pcall(contextSensitiveUseItem, cid, {slot}) \
         return __jenc({{ ok = ok == true }})",
        cid = container.lua_id()
    )
}

impl Supervisor {
    /// Poll the inventory until it differs from `before` or `wait` passes.
    pub(super) async fn await_inventory_change(
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
        expect_change: bool,
        wait: Duration,
    ) -> Result<Value, String> {
        let t0 = Instant::now();
        let mut trail = NativeTrail::default();
        // A step that already saw the inventory change (so the tail need
        // not wait again).
        let mut settled: Option<(Value, Value)> = None;
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
                            self.click_at(x, y, 0, true)
                                .await?
                                .note_fallback(&mut trail);
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
                // The stock inventory has no double-click handler on a slot
                // (`Inventory.lua:745-757` acts on the right button only), so
                // a double-click would do nothing: do the right-click and say so.
                let verb = if verb == ItemVerb::DoubleClick {
                    out["requested"] = json!("doubleclick");
                    out["performed"] = json!("rightclick");
                    ItemVerb::RightClick
                } else {
                    verb
                };
                let revealed = self
                    .reveal_slot(&ContainerRef::Name(container.clone()), slot, &mut trail)
                    .await;
                match revealed {
                    Ok(loc) => {
                        let window = loc["window"].as_str().unwrap_or_default().to_string();
                        let r = self
                            .lua_json(&locate_chunk(&ClickTarget::Named(window.clone())))
                            .await?;
                        let (x, y) = click_point(&r)
                            .ok_or_else(|| format!("{window} has no screen rectangle"))?;
                        let step_wait = wait / 2;
                        let mut attempts: Vec<String> = Vec::new();
                        // A click that was sent but showed no change may still
                        // land late (a slow server round trip): the next step
                        // could then use the item a second time.
                        let mut sent_unseen = false;
                        // 1. CEGUI's own button injectors (N1).
                        match self.click_at_native(x, y, 1, false).await {
                            Ok(placed) => {
                                let (a, d) =
                                    self.await_inventory_change(&before, step_wait).await?;
                                if d["changed"] == json!(true) {
                                    placed.note_fallback(&mut trail);
                                    trail.push(
                                        format!("right-click {window} (CEGUI injectors)"),
                                        NativeLevel::NativeCegui,
                                    );
                                    settled = Some((a, d));
                                } else {
                                    sent_unseen = true;
                                    attempts
                                        .push("injected right-click: no inventory change".into());
                                }
                            }
                            Err(e) => attempts.push(format!("injected right-click: {e}")),
                        }
                        // A negative check (expect_change false) stops after the
                        // first native click: no extra use attempts (D-LH13).
                        let escalate = expect_change;
                        if !escalate && settled.is_none() {
                            out["escalation"] = json!("skipped_negative_check");
                        }
                        // 2. Posted button messages (N1).
                        if escalate && settled.is_none() {
                            match self.click_at(x, y, 1, false).await {
                                Ok(placed) => {
                                    let (a, d) =
                                        self.await_inventory_change(&before, step_wait).await?;
                                    if d["changed"] == json!(true) {
                                        placed.note_fallback(&mut trail);
                                        trail.push(
                                            format!("right-click {window}"),
                                            NativeLevel::RealInput,
                                        );
                                        settled = Some((a, d));
                                    } else {
                                        sent_unseen = true;
                                        attempts
                                            .push("posted right-click: no inventory change".into());
                                    }
                                }
                                Err(e) => attempts.push(format!("posted right-click: {e}")),
                            }
                        }
                        // 3. The stock handler the right-click would reach (N3).
                        if escalate && settled.is_none() && allow_fallback {
                            let r = self
                                .lua_json(&use_item_chunk(
                                    &ContainerRef::Name(container.clone()),
                                    slot,
                                ))
                                .await?;
                            if r["ok"] == json!(true) {
                                let (a, d) = self.await_inventory_change(&before, wait).await?;
                                trail.push(
                                    "contextSensitiveUseItem via Lua",
                                    NativeLevel::ClientUiLua,
                                );
                                settled = Some((a, d));
                            } else {
                                attempts.push("contextSensitiveUseItem: not callable".into());
                            }
                        }
                        out["slot_window"] = json!(window);
                        out["attempts"] = json!(attempts);
                        // Reported, not hidden: when this is true and the diff
                        // shows more than one use, an earlier click landed late.
                        out["double_use_risk"] = json!(sent_unseen && settled.is_some());
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
        let (after, d) = match settled.take() {
            Some(done) => done,
            None => self.await_inventory_change(&before, wait).await?,
        };
        if matches!(verb, ItemVerb::LootAll | ItemVerb::LootSlot) {
            let st = self.lua_json(LOOT_STATE).await?;
            out["loot_count_after"] = st["count"].clone();
            out["loot_window_open"] = st["visible"].clone();
        }
        out["inventory_changed"] = d["changed"].clone();
        out["diff"] = d;
        out["cash_after"] = after["cash"].clone();
        trail.stamp(&mut out);
        // An item action that changed nothing is no pass at any level
        // (#1660 item 3). Loot actions have their own counters; a caller who
        // expects no change (a negative check) passes expect_change false.
        if expect_change && !matches!(verb, ItemVerb::LootAll | ItemVerb::LootSlot) {
            let moved = out["inventory_changed"] == json!(true);
            out["effect_ok"] = json!(moved);
            if !moved {
                out["native_pass"] = json!(false);
            }
        }
        out["elapsed_ms"] = json!(t0.elapsed().as_millis() as u64);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supervisor::cegui_fake::{self, FakeCegui};
    use crate::supervisor::events::fake_bridge;
    use std::sync::{Arc, Mutex};

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

    fn main_slot_1() -> ItemSelector {
        ItemSelector {
            container: Some("Main".into()),
            slot: Some(1),
            ..Default::default()
        }
    }

    async fn use_item(
        state: FakeCegui,
        allow_fallback: bool,
        expect_change: bool,
    ) -> (Value, Arc<Mutex<FakeCegui>>) {
        let s = Arc::new(Mutex::new(state));
        let sup = fake_bridge::supervisor(cegui_fake::responder(s.clone())).await;
        let out = sup
            .item_action(
                ItemVerb::Use,
                &main_slot_1(),
                None,
                allow_fallback,
                expect_change,
                Duration::ZERO,
            )
            .await
            .unwrap();
        (out, s)
    }

    /// The injected right-click uses the item: N1, one injected click, the
    /// Lua handler never called.
    #[tokio::test]
    async fn an_injected_right_click_that_works_is_a_native_pass() {
        let (out, s) = use_item(
            FakeCegui {
                right_click_works: true,
                ..FakeCegui::default()
            },
            true,
            true,
        )
        .await;
        assert_eq!(out["effect_ok"], true);
        assert_eq!(out["native_level"], "native_cegui");
        assert_eq!(out["native_pass"], true);
        let s = s.lock().unwrap();
        assert_eq!(s.edges(), vec!["down 1", "up 1"]);
        assert_eq!(s.lua_use_calls, 0);
        assert_eq!(out["double_use_risk"], false);
    }

    /// Both clicks are lost: the stock handler uses the item, and the result
    /// says N3 and no native pass.
    #[tokio::test]
    async fn the_lua_handler_is_the_last_resort_and_is_n3() {
        let (out, s) = use_item(FakeCegui::default(), true, true).await;
        assert_eq!(out["effect_ok"], true);
        assert_eq!(out["native_level"], "client_ui_lua");
        assert_eq!(out["native_pass"], false);
        assert_eq!(out["double_use_risk"], true);
        assert_eq!(s.lock().unwrap().lua_use_calls, 1);
    }

    /// Regression guard (#1660 item 3): nothing changed, so the result is no
    /// pass. The old code stamped native_pass true here.
    #[tokio::test]
    async fn a_use_that_changed_nothing_is_no_pass() {
        let (out, _) = use_item(FakeCegui::default(), false, true).await;
        assert_eq!(out["effect_ok"], false);
        assert_eq!(out["native_pass"], false);
        assert_eq!(out["inventory_changed"], false);
        assert!(out["attempts"].as_array().unwrap().len() >= 2);
    }

    /// A negative check opts out of the effect test.
    #[tokio::test]
    async fn expect_change_false_leaves_effect_ok_unset() {
        let (out, _) = use_item(FakeCegui::default(), false, false).await;
        assert!(out.get("effect_ok").is_none());
    }

    /// D-LH13: a negative check sends one native click and no escalation,
    /// even when a later step would have used the item.
    #[tokio::test]
    async fn expect_change_false_does_not_escalate() {
        let (out, s) = use_item(FakeCegui::default(), true, false).await;
        let s = s.lock().unwrap();
        assert_eq!(s.edges(), vec!["down 1", "up 1"], "one injected click only");
        assert_eq!(s.lua_use_calls, 0, "the Lua handler must not run");
        assert_eq!(out["escalation"], "skipped_negative_check");
        assert_eq!(out["attempts"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn a_double_click_on_a_slot_is_performed_as_a_right_click() {
        let s = Arc::new(Mutex::new(FakeCegui {
            right_click_works: true,
            ..FakeCegui::default()
        }));
        let sup = fake_bridge::supervisor(cegui_fake::responder(s.clone())).await;
        let out = sup
            .item_action(
                ItemVerb::DoubleClick,
                &main_slot_1(),
                None,
                true,
                true,
                Duration::ZERO,
            )
            .await
            .unwrap();
        assert_eq!(out["requested"], "doubleclick");
        assert_eq!(out["performed"], "rightclick");
        assert_eq!(s.lock().unwrap().edges(), vec!["down 1", "up 1"]);
    }
}
