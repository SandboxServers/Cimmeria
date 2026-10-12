//! A fake client for the native CEGUI paths: client memory with a `System`
//! singleton, two named windows (`SrcSlot`, a `DragContainer`, and
//! `DstSlot`), and the injector calls recorded in order. Answers through
//! [`fake_bridge`], so the cursor and drag flows run without a game.
//!
//! The drag model follows the live client: the first injected move with
//! the button down crosses the threshold and starts the drag (setting the
//! container's dragging byte); button-up drops only when `d_dropTarget` is
//! set; `notifyDragDropItemDropped` drops directly. Either drop moves the
//! item from `Main` slot 1 to slot [`FakeCegui::lands_in`]. Slot ends
//! `Main` 1 and 2 are shown in `SrcSlot` and `DstSlot`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::cegui_native::{
    rebase, DRAG_CONTAINER_VTABLE_VA, DRAG_FLAGS_OFFSET, DROP_TARGET_OFFSET,
    INJECT_MOUSE_BUTTON_DOWN_VA, INJECT_MOUSE_BUTTON_UP_VA, INJECT_MOUSE_POSITION_VA,
    NOTIFY_DRAG_DROP_ITEM_DROPPED_VA, SYSTEM_SINGLETON_VA,
};
use super::events::fake_bridge::{lua_ok, Responder};

pub const SYSTEM: u32 = 0x0500_0000;
pub const CONTAINER: u32 = 0x0600_0000;
pub const TARGET: u32 = 0x0700_0000;
/// The stand-in `DragContainer` (`InventoryDragItemWin`) the stock Lua hands
/// a split drag to.
pub const STAND_IN: u32 = 0x0800_0000;
const STAND_USERDATA: u32 = 0x0a00_0030;
const SRC_USERDATA: u32 = 0x0a00_0010;
const DST_USERDATA: u32 = 0x0a00_0020;

/// The fake's state, shared with the test.
#[derive(Debug)]
pub struct FakeCegui {
    pub slide: i64,
    /// `*SYSTEM_SINGLETON`; 0 = the UI does not exist.
    pub system: u32,
    pub container_vtable: u32,
    /// What `d_dropTarget` reads at the end of the drag.
    pub drop_target: u32,
    /// Whether moves with the button down drag `SrcSlot`'s container.
    pub container_drags: bool,
    /// `getDragInfo` reports a drag of some other item throughout.
    pub other_drag: bool,
    /// What `injectMouseButtonDown` returns (CEGUI handled the press).
    pub down_handled: bool,
    /// Fail the n-th (1-based) `getDragInfo` read.
    pub fail_drag_state_on: Option<usize>,
    pub drag_state_reads: usize,
    /// The `Main` slot a drop puts the item in.
    pub lands_in: i64,
    /// The ordered native calls: `move x y`, `down b`, `up b`,
    /// `notify target item`.
    pub calls: Vec<String>,
    /// Every `input_modifiers` request, in order.
    pub modifiers: Vec<Value>,
    /// Lua chunks that placed the cursor through `setPosition`.
    pub lua_placements: usize,
    /// The stock UI hands the drag to `InventoryDragItemWin`: the source
    /// container stops dragging, the stand-in drags.
    pub stand_in: bool,
    /// Another window is drawn over `SrcSlot`: a press there lands on it, so
    /// the source container never drags until `raise_chunk` ran.
    pub src_covered: bool,
    /// The press of the current gesture reached `SrcSlot`.
    pub press_on_src: bool,
    /// Windows `moveToFront` was called for, in order.
    pub raises: Vec<String>,
    /// `moveToFront` is missing or throws: the window stays where it was.
    pub raise_fails: bool,
    /// A native right click (button 1) on `SrcSlot` uses the item.
    pub right_click_works: bool,
    /// A native left click on `TabBtn` shows the tab.
    pub tab_click_works: bool,
    pub tab_visible: bool,
    /// `contextSensitiveUseItem` calls and tab-handler calls made through Lua.
    pub lua_use_calls: usize,
    pub tab_handler_calls: usize,
    /// Vendor: the row-1 spinner value, whether its inc button works,
    /// whether Buy bought, and `setCurrentValue` calls through Lua.
    pub vendor_value: i64,
    pub inc_works: bool,
    pub bought: bool,
    /// Buy clicks (injected) the client loses before one lands.
    pub buy_swallow: usize,
    pub lua_buy_calls: usize,
    pub lua_set_calls: usize,
    pub cursor: (f32, f32),
    pub button_down: bool,
    pub dragging: bool,
    pub moved: bool,
}

impl Default for FakeCegui {
    fn default() -> Self {
        Self {
            slide: 0,
            system: SYSTEM,
            container_vtable: DRAG_CONTAINER_VTABLE_VA,
            drop_target: 0,
            container_drags: true,
            other_drag: false,
            down_handled: true,
            fail_drag_state_on: None,
            drag_state_reads: 0,
            lands_in: 2,
            calls: Vec::new(),
            modifiers: Vec::new(),
            lua_placements: 0,
            stand_in: false,
            src_covered: false,
            press_on_src: true,
            raises: Vec::new(),
            raise_fails: false,
            right_click_works: false,
            tab_click_works: false,
            tab_visible: false,
            lua_use_calls: 0,
            tab_handler_calls: 0,
            vendor_value: 0,
            inc_works: false,
            bought: false,
            buy_swallow: 0,
            lua_buy_calls: 0,
            lua_set_calls: 0,
            cursor: (0.0, 0.0),
            button_down: false,
            dragging: false,
            moved: false,
        }
    }
}

impl FakeCegui {
    /// The native calls only (no cursor moves), in order.
    pub fn edges(&self) -> Vec<&str> {
        self.calls
            .iter()
            .filter(|c| !c.starts_with("move"))
            .map(String::as_str)
            .collect()
    }

    /// The source container's flag word as the live client lays it out.
    fn drag_flags(&self) -> u32 {
        let mut w = u32::from(self.button_down);
        if self.dragging && !self.stand_in {
            w |= 0x0001_0100;
        }
        w
    }

    /// The stand-in's flag word: dragging only after the hand-off.
    fn stand_in_flags(&self) -> u32 {
        if self.dragging && self.stand_in {
            0x0001_0101
        } else {
            0
        }
    }

    fn memory(&self) -> HashMap<u32, u32> {
        HashMap::from([
            (rebase(SYSTEM_SINGLETON_VA, self.slide), self.system),
            (SRC_USERDATA, CONTAINER),
            (DST_USERDATA, TARGET),
            (CONTAINER, rebase(self.container_vtable, self.slide)),
            (CONTAINER + DROP_TARGET_OFFSET, self.drop_target),
            (CONTAINER + DRAG_FLAGS_OFFSET, self.drag_flags()),
            (STAND_USERDATA, STAND_IN),
            (STAND_IN, rebase(self.container_vtable, self.slide)),
            (STAND_IN + DROP_TARGET_OFFSET, self.drop_target),
            (STAND_IN + DRAG_FLAGS_OFFSET, self.stand_in_flags()),
        ])
    }

    fn call_native(&mut self, params: &Value) -> Result<Value, String> {
        let word = |v: &Value| -> u32 {
            let s = v.as_str().unwrap_or_default();
            u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(u32::MAX)
        };
        let addr = word(&params["addr"]);
        let args: Vec<u32> = params["args"]
            .as_array()
            .map(|a| a.iter().map(word).collect())
            .unwrap_or_default();
        assert_eq!(params["conv"], "thiscall", "every CEGUI entry is thiscall");
        let at = |va| rebase(va, self.slide);
        let mut ret: u32 = 1;
        if addr == at(INJECT_MOUSE_POSITION_VA) {
            assert_eq!(args[0], self.system, "this is the System singleton");
            self.cursor = (f32::from_bits(args[1]), f32::from_bits(args[2]));
            self.calls
                .push(format!("move {} {}", self.cursor.0, self.cursor.1));
            if self.button_down && self.container_drags && self.press_on_src {
                self.dragging = true;
            }
        } else if addr == at(INJECT_MOUSE_BUTTON_DOWN_VA) {
            self.button_down = self.down_handled;
            self.press_on_src = !self.src_covered;
            // Only AL carries the bool: garbage above it must not count.
            ret = if self.down_handled { 1 } else { 0xdead_be00 };
            self.calls.push(format!("down {}", args[1]));
        } else if addr == at(INJECT_MOUSE_BUTTON_UP_VA) {
            if self.dragging && self.drop_target != 0 {
                self.moved = true;
            }
            // Clicks (no drag): which window the cursor is over decides.
            let (x, y) = self.cursor;
            if args[1] == 1 && self.right_click_works && x < 20.0 && y < 20.0 {
                self.moved = true;
            }
            if args[1] == 0 && self.tab_click_works && (300.0..320.0).contains(&x) {
                self.tab_visible = true;
            }
            if args[1] == 0 && self.inc_works && (400.0..420.0).contains(&x) {
                self.vendor_value += 1;
            }
            if args[1] == 0 && (500.0..540.0).contains(&x) && self.vendor_value >= 1 {
                if self.buy_swallow > 0 {
                    self.buy_swallow -= 1;
                } else {
                    self.bought = true;
                }
            }
            self.button_down = false;
            self.dragging = false;
            self.calls.push(format!("up {}", args[1]));
        } else if addr == at(NOTIFY_DRAG_DROP_ITEM_DROPPED_VA) {
            self.moved = true;
            self.calls
                .push(format!("notify {:#x} {:#x}", args[0], args[1]));
        } else {
            return Err(format!("unexpected native call {addr:#x}"));
        }
        Ok(json!({ "ret_u32": ret, "ret_i32": ret as i32, "ret_hex": format!("{ret:#010x}") }))
    }

    /// A `slots::locate_chunk` for `Main` (container 1): slot 1 is shown
    /// in `SrcSlot`, any other slot in `DstSlot`.
    fn locate_slot(chunk: &str) -> Value {
        let line = chunk
            .split("local cid, slot = ")
            .nth(1)
            .and_then(|s| s.lines().next())
            .unwrap_or_default();
        let slot: i64 = line
            .rsplit(", ")
            .next()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        let window = if slot == 1 { "SrcSlot" } else { "DstSlot" };
        json!({ "container": "Main", "container_id": 1, "slot": slot,
                "window": window, "visible": true })
    }

    fn lua(&mut self, chunk: &str) -> Result<Value, String> {
        let window = |c: &str| {
            if c.contains("\"SrcSlot\"") {
                Some(("SrcSlot", [0, 0, 20, 20], SRC_USERDATA))
            } else if c.contains("\"DstSlot\"") {
                Some(("DstSlot", [100, 0, 120, 20], DST_USERDATA))
            } else if c.contains("\"InventoryDragItemWin\"") {
                Some(("InventoryDragItemWin", [200, 0, 220, 20], STAND_USERDATA))
            } else if c.contains("\"TabBtn\"") {
                Some(("TabBtn", [300, 0, 320, 20], 0))
            } else if c.contains("\"Vendor_Item1Quantity__auto_incbtn__\"") {
                Some(("Vendor_Item1Quantity__auto_incbtn__", [400, 0, 420, 20], 0))
            } else if c.contains("\"Vendor_BuyButton\"") {
                Some(("Vendor_BuyButton", [500, 0, 540, 20], 0))
            } else {
                None
            }
        };
        let doc = if chunk.contains("moveToFront") {
            self.raises
                .push(window(chunk).map(|w| w.0).unwrap_or("?").to_string());
            if self.raise_fails {
                json!({ "found": true, "ok": false, "raised": "TopWin" })
            } else {
                self.src_covered = false;
                json!({ "found": true, "ok": true, "raised": "TopWin" })
            }
        } else if chunk.contains("getCash") {
            let slot = if self.moved { self.lands_in } else { 1 };
            json!({ "cash": if self.bought { -50 } else { 0 }, "containers": [{ "name": "Main",
                "slots": [{ "slot": slot, "item_id": 7, "name": "Medkit", "qty": if self.bought { 1 + self.vendor_value } else { 1 } }] }] })
        } else if chunk.contains("local cid, slot = ") {
            Self::locate_slot(chunk)
        } else if chunk.contains("rect_centre") {
            match window(chunk) {
                Some((name, rect, _)) => json!({ "found": true, "name": name,
                    "visible": true, "enabled": true, "rect": rect }),
                None => json!({ "found": false }),
            }
        } else if chunk.contains("ud = tostring(w)") {
            match window(chunk) {
                Some((_, _, ud)) => json!({ "found": true, "ud": format!("userdata: {ud:08X}") }),
                None => json!({ "found": false }),
            }
        } else if chunk.contains("InventoryMod") && chunk.contains("dragType") {
            json!({ "inventory": 1 })
        } else if chunk.contains("contextSensitiveUseItem") {
            self.lua_use_calls += 1;
            self.moved = true;
            json!({ "ok": true })
        } else if chunk.contains("onBuyClicked") {
            self.lua_buy_calls += 1;
            self.bought = self.vendor_value >= 1;
            json!({ "ok": true })
        } else if chunk.contains("pcall(f, w)") {
            self.tab_handler_calls += 1;
            self.tab_visible = true;
            json!({ "ok": true })
        } else if chunk.contains("done = w ~= nil") {
            json!({ "done": self.tab_visible })
        } else if chunk.contains("vendor_open") {
            json!({ "vendor_open": true, "buy_visible": true, "row_visible": true,
                    "current": self.vendor_value, "max": 10, "scroll": 0 })
        } else if chunk.contains("done = cur ==") {
            let want: i64 = chunk
                .split("done = cur == ")
                .nth(1)
                .map(|s| {
                    s.chars()
                        .take_while(char::is_ascii_digit)
                        .collect::<String>()
                })
                .and_then(|s| s.parse().ok())
                .unwrap_or(-1);
            json!({ "done": self.vendor_value == want })
        } else if chunk.contains("setCurrentValue(") {
            let n: i64 = chunk
                .split("setCurrentValue(")
                .nth(1)
                .map(|s| {
                    s.chars()
                        .take_while(char::is_ascii_digit)
                        .collect::<String>()
                })
                .and_then(|s| s.parse().ok())
                .unwrap_or(-1);
            self.lua_set_calls += 1;
            self.vendor_value = n;
            json!({ "ok": true, "via": "spinner", "current": n })
        } else if chunk.contains("getDragInfo") {
            self.drag_state_reads += 1;
            if self.fail_drag_state_on == Some(self.drag_state_reads) {
                return Err("lua_eval: scripted drag-state failure".into());
            }
            let active = self.dragging || self.other_drag;
            json!({ "drag_type": if active { json!(3) } else { Value::Null },
                    "drag_icon_visible": active })
        } else if chunk.contains("getPosition()") {
            if chunk.contains("setPosition") {
                // The Lua fallback: setPosition(CEGUI.Vector2(x, y)).
                self.lua_placements += 1;
                let xy = chunk
                    .split("Vector2(")
                    .nth(1)
                    .and_then(|s| s.split(')').next())
                    .unwrap_or_default();
                let mut it = xy.split(',').map(|s| s.trim().parse::<f32>().unwrap());
                self.cursor = (it.next().unwrap(), it.next().unwrap());
            }
            return Ok(lua_ok(&[
                self.cursor.0.to_string(),
                self.cursor.1.to_string(),
            ]));
        } else {
            return Err(format!("unexpected chunk: {chunk:.120}"));
        };
        Ok(lua_ok(&[doc.to_string()]))
    }

    fn mem_read(&self, params: &Value) -> Result<Value, String> {
        let s = params["addr"].as_str().unwrap_or_default();
        let addr =
            u32::from_str_radix(s.trim_start_matches("0x"), 16).map_err(|e| e.to_string())?;
        assert_eq!(params["len"], 4);
        let w = self.memory().get(&addr).copied().unwrap_or(0);
        let hex: String = w.to_le_bytes().iter().map(|b| format!("{b:02x}")).collect();
        Ok(json!({ "hex": hex }))
    }
}

/// A fake bridge over `state`.
pub fn responder(state: Arc<Mutex<FakeCegui>>) -> Responder {
    Arc::new(move |method, params| {
        let mut s = state.lock().unwrap();
        match method {
            "module_info" => Ok(json!({ "slide": s.slide })),
            "mem_read" => s.mem_read(params),
            "call_native" => s.call_native(params),
            "input_cursor" => Ok(json!({ "ok": true })),
            "input_modifiers" => {
                s.modifiers.push(params.clone());
                Ok(json!({ "ok": true }))
            }
            "lua_eval" => s.lua(params["chunk"].as_str().unwrap_or_default()),
            other => Err(format!("unexpected {other}")),
        }
    })
}
