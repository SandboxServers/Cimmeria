//! A fake client for the native CEGUI paths: client memory with a `System`
//! singleton, two named windows (`SrcSlot`, a `DragContainer`, and
//! `DstSlot`), and the injector calls recorded in order. Answers through
//! [`fake_bridge`], so the cursor and drag flows run without a game.
//!
//! The drag model follows the live client: the first injected move with
//! the button down crosses the threshold and starts the drag; button-up
//! drops only when `d_dropTarget` is set; `notifyDragDropItemDropped`
//! drops directly. Either drop moves the item from slot 1 to slot 2.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::cegui_native::{
    rebase, DRAG_CONTAINER_VTABLE_VA, DROP_TARGET_OFFSET, INJECT_MOUSE_BUTTON_DOWN_VA,
    INJECT_MOUSE_BUTTON_UP_VA, INJECT_MOUSE_POSITION_VA, NOTIFY_DRAG_DROP_ITEM_DROPPED_VA,
    SYSTEM_SINGLETON_VA,
};
use super::events::fake_bridge::{lua_ok, Responder};

pub const SYSTEM: u32 = 0x0500_0000;
pub const CONTAINER: u32 = 0x0600_0000;
pub const TARGET: u32 = 0x0700_0000;
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
    /// The ordered native calls: `move x y`, `down b`, `up b`,
    /// `notify target item`.
    pub calls: Vec<String>,
    /// Lua chunks that placed the cursor through `setPosition`.
    pub lua_placements: usize,
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
            calls: Vec::new(),
            lua_placements: 0,
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

    fn memory(&self) -> HashMap<u32, u32> {
        HashMap::from([
            (rebase(SYSTEM_SINGLETON_VA, self.slide), self.system),
            (SRC_USERDATA, CONTAINER),
            (DST_USERDATA, TARGET),
            (CONTAINER, rebase(self.container_vtable, self.slide)),
            (CONTAINER + DROP_TARGET_OFFSET, self.drop_target),
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
        if addr == at(INJECT_MOUSE_POSITION_VA) {
            assert_eq!(args[0], self.system, "this is the System singleton");
            self.cursor = (f32::from_bits(args[1]), f32::from_bits(args[2]));
            self.calls
                .push(format!("move {} {}", self.cursor.0, self.cursor.1));
            if self.button_down {
                self.dragging = true;
            }
        } else if addr == at(INJECT_MOUSE_BUTTON_DOWN_VA) {
            self.button_down = true;
            self.calls.push(format!("down {}", args[1]));
        } else if addr == at(INJECT_MOUSE_BUTTON_UP_VA) {
            if self.dragging && self.drop_target != 0 {
                self.moved = true;
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
        Ok(json!({ "ret_u32": 1, "ret_i32": 1, "ret_hex": "0x00000001" }))
    }

    fn lua(&mut self, chunk: &str) -> Result<Value, String> {
        let window = |c: &str| {
            if c.contains("\"SrcSlot\"") {
                Some(("SrcSlot", [0, 0, 20, 20], SRC_USERDATA))
            } else if c.contains("\"DstSlot\"") {
                Some(("DstSlot", [100, 0, 120, 20], DST_USERDATA))
            } else {
                None
            }
        };
        let doc = if chunk.contains("getCash") {
            let slot = if self.moved { 2 } else { 1 };
            json!({ "cash": 0, "containers": [{ "name": "Main",
                "slots": [{ "slot": slot, "item_id": 7, "name": "Medkit", "qty": 1 }] }] })
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
        } else if chunk.contains("getDragInfo") {
            json!({ "drag_type": if self.dragging { json!(3) } else { Value::Null },
                    "drag_icon_visible": self.dragging })
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
            "lua_eval" => s.lua(params["chunk"].as_str().unwrap_or_default()),
            other => Err(format!("unexpected {other}")),
        }
    })
}
