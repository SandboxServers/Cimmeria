//! Native CEGUI input: the client's own `CEGUI::System` injectors, called
//! on the game's main thread through the bridge's `call_native`.
//!
//! The game feeds CEGUI's cursor only from DirectInput motion: its per-tick
//! pump calls `System::injectMousePosition` when DirectInput reports a
//! delta. A posted `WM_MOUSEMOVE` never reaches CEGUI, Lua's
//! `MouseCursor:setPosition` moves the cursor without a `MouseMove` event,
//! and the client's Lua has no `CEGUI.System` binding. So hover, drag
//! thresholds and drop targets only see the lab's cursor when the lab makes
//! the call the pump makes. Addresses, signatures and the drag/drop
//! sequence: `docs/reverse-engineering/findings/cegui-mouse-input-feed.md`
//! (sections 1, 4 and 7).
//!
//! Every address is a stock SGW.exe VA (image base `0x400000`), rebased by
//! the module slide the bridge reports. All injectors are `__thiscall` on
//! the `System` singleton; floats go over the wire as their IEEE-754 bit
//! patterns, since `call_native` passes every argument as a 32-bit word.

use serde_json::{json, Value};

use super::entity_table::decode_hex;
use super::ui::window_read::WALK_FN;
use super::Supervisor;
use crate::supervisor::flows::widgets::lua_quote;

/// `CEGUI::System::ms_Singleton` (`System*`). Null before the UI exists.
pub const SYSTEM_SINGLETON_VA: u32 = 0x01f0_10c8;
/// `void System::injectMousePosition(float x, float y)`, `ret 8`: sets the
/// cursor, then fires a real `MouseMove` through the hit-test.
pub const INJECT_MOUSE_POSITION_VA: u32 = 0x011a_eaa0;
/// `bool System::injectMouseButtonDown(MouseButton)`, `ret 4`: acts at the
/// CEGUI cursor; takes no coordinates.
pub const INJECT_MOUSE_BUTTON_DOWN_VA: u32 = 0x011a_fe40;
/// `bool System::injectMouseButtonUp(MouseButton)`, `ret 4`.
pub const INJECT_MOUSE_BUTTON_UP_VA: u32 = 0x011b_0040;
/// `void Window::notifyDragDropItemDropped(DragContainer*)`, `ret 4`, on
/// the target window: fires its `DragDropItemDropped` event.
pub const NOTIFY_DRAG_DROP_ITEM_DROPPED_VA: u32 = 0x011a_2930;
/// `CEGUI::DragContainer` vtable: the only object the lab will treat as a
/// dragged item.
pub const DRAG_CONTAINER_VTABLE_VA: u32 = 0x01aa_edc4;
/// `DragContainer::d_dropTarget` (`Window*`): set while the drag is over a
/// window flagged `DragDropTarget`, null otherwise.
pub const DROP_TARGET_OFFSET: u32 = 0x270;
/// The `DragContainer` flag bytes, read as one little-endian word: `+0x23c`
/// left button down, `+0x23d`, `+0x23e` dragging (findings §7.1). The
/// dragging byte is the one `onMouseMove` tests to pick `doDragging` and
/// `onCaptureLost` clears (§8), so it says whether *this* container is the
/// one being dragged; `getDragInfo` is global and cannot. A live drag read
/// the word as `0x00000001` after the press and `0x00010101` once dragging.
pub const DRAG_FLAGS_OFFSET: u32 = 0x23c;
/// CEGUI `LeftButton`.
pub const LEFT_BUTTON: u32 = 0;

/// A stock VA moved by the module slide.
pub fn rebase(va: u32, slide: i64) -> u32 {
    (i64::from(va) + slide) as u32
}

/// Whether a [`DRAG_FLAGS_OFFSET`] word has the dragging byte (`+0x23e`) set.
pub fn container_dragging(flags: u32) -> bool {
    (flags >> 16) & 0xff != 0
}

/// A float argument as the 32-bit word `call_native` pushes.
pub fn float_word(v: f32) -> u32 {
    v.to_bits()
}

/// `call_native` params for a `__thiscall` with word arguments.
pub fn thiscall_params(func: u32, this: u32, args: &[u32], ret: &str) -> Value {
    let mut words = vec![format!("{this:#x}")];
    words.extend(args.iter().map(|a| format!("{a:#x}")));
    json!({
        "addr": format!("{func:#x}"),
        "conv": "thiscall",
        "args": words,
        "ret": ret,
    })
}

/// The address in a Lua `tostring(userdata)`: `"userdata: 0A1B2C3D"`
/// (MSVC `%p`, no prefix) or `"userdata: 0x0a1b2c3d"`.
pub fn parse_userdata_addr(s: &str) -> Option<u32> {
    let hex = s.trim().strip_prefix("userdata:")?.trim();
    let hex = hex
        .strip_prefix("0x")
        .or_else(|| hex.strip_prefix("0X"))
        .unwrap_or(hex);
    u32::from_str_radix(hex, 16).ok().filter(|a| *a != 0)
}

/// Refuse anything but a `DragContainer` as the dragged item: calling
/// `notifyDragDropItemDropped` with another object would hand the drop
/// handlers a wrong type.
pub fn check_drag_container(name: &str, vtable: u32, slide: i64) -> Result<(), String> {
    let want = rebase(DRAG_CONTAINER_VTABLE_VA, slide);
    if vtable == want {
        Ok(())
    } else {
        Err(format!(
            "{name} is not a CEGUI DragContainer (vtable {vtable:#010x}, expected {want:#010x})"
        ))
    }
}

/// Lua body: the userdata address of a window found by name (a global, a
/// `getWindow` name, or `Parent/Child`).
pub fn window_userdata_chunk(name: &str) -> String {
    format!(
        "{WALK_FN}\nlocal w = __lab_find({n})\n\
         if w == nil then return __jenc({{ found = false }}) end\n\
         return __jenc({{ found = true, ud = tostring(w) }})",
        n = lua_quote(name)
    )
}

/// What to do at the end of a drag, before the button goes up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPlan {
    /// CEGUI resolved a drop target: button-up drops there by itself.
    Natural,
    /// No drop target: fire the target's `DragDropItemDropped` directly.
    Notify,
    /// No drag, or no target and the explicit drop is not allowed: let go.
    Release,
}

/// The drop decision. `d_dropTarget` stays null in the live client even
/// over a slot (2026-10-10); the findings' leading suspect is that no
/// window from the hit slot up to the sheet has `DragDropTarget` set
/// (`+0xfc`), but that is not confirmed, so the explicit drop stays.
pub fn drop_plan(started: bool, drop_target: u32, allow_notify: bool) -> DropPlan {
    match (started, drop_target != 0, allow_notify) {
        (false, _, _) => DropPlan::Release,
        (true, true, _) => DropPlan::Natural,
        (true, false, true) => DropPlan::Notify,
        (true, false, false) => DropPlan::Release,
    }
}

/// A handle on the client's CEGUI `System`, valid for one tool call.
pub struct Cegui<'a> {
    sup: &'a Supervisor,
    slide: i64,
    system: u32,
}

impl Supervisor {
    /// Resolve the module slide and the `System` singleton. Fails when the
    /// UI does not exist yet (null singleton).
    pub async fn cegui(&self) -> Result<Cegui<'_>, String> {
        let info = self
            .bridge
            .call("module_info", json!({}))
            .await
            .map_err(|e| format!("module_info: {e}"))?;
        let slide = info.get("slide").and_then(Value::as_i64).unwrap_or(0);
        let mut c = Cegui {
            sup: self,
            slide,
            system: 0,
        };
        let system = c.read_u32(rebase(SYSTEM_SINGLETON_VA, slide)).await?;
        if system == 0 {
            return Err("CEGUI::System is not created yet (null singleton)".into());
        }
        c.system = system;
        Ok(c)
    }
}

impl Cegui<'_> {
    /// A fault-guarded read, straight to the bridge (reads are never
    /// journaled: see `entity_table`).
    async fn read_u32(&self, addr: u32) -> Result<u32, String> {
        let v = self
            .sup
            .bridge
            .call(
                "mem_read",
                json!({ "addr": format!("{addr:#x}"), "len": 4 }),
            )
            .await
            .map_err(|e| format!("mem_read {addr:#x}: {e}"))?;
        let hex = v
            .get("hex")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("mem_read {addr:#x}: no hex in {v}"))?;
        let b = decode_hex(hex)?;
        let w: [u8; 4] = b
            .get(..4)
            .and_then(|s| s.try_into().ok())
            .ok_or_else(|| format!("mem_read {addr:#x}: short read"))?;
        Ok(u32::from_le_bytes(w))
    }

    /// A journaled native call on the main thread; returns EAX.
    async fn call(&self, va: u32, this: u32, args: &[u32], ret: &str) -> Result<u32, String> {
        self.call_as(va, this, args, ret, false).await
    }

    /// [`Self::call`]; `release` skips the lease check (see
    /// `Supervisor::bridge_release_call`), for button-up only.
    async fn call_as(
        &self,
        va: u32,
        this: u32,
        args: &[u32],
        ret: &str,
        release: bool,
    ) -> Result<u32, String> {
        let func = rebase(va, self.slide);
        let params = thiscall_params(func, this, args, ret);
        let v = if release {
            self.sup.bridge_release_call("call_native", params).await
        } else {
            self.sup.bridge_call("call_native", params).await
        }
        .map_err(|e| format!("call_native {func:#x}: {e}"))?;
        if v.get("ok").and_then(Value::as_bool) == Some(false) {
            return Err(format!("call_native {func:#x}: {v}"));
        }
        Ok(v.get("ret_u32").and_then(Value::as_u64).unwrap_or(0) as u32)
    }

    /// `injectMousePosition(x, y)` in client (UI) pixels.
    pub async fn inject_mouse_position(&self, x: f32, y: f32) -> Result<(), String> {
        self.call(
            INJECT_MOUSE_POSITION_VA,
            self.system,
            &[float_word(x), float_word(y)],
            "void",
        )
        .await
        .map(|_| ())
    }

    /// `injectMouseButtonDown(button)`; true when CEGUI handled it.
    pub async fn button_down(&self, button: u32) -> Result<bool, String> {
        // The bool is in AL; the upper bytes of EAX are garbage.
        self.call(INJECT_MOUSE_BUTTON_DOWN_VA, self.system, &[button], "u32")
            .await
            .map(|r| r & 0xFF != 0)
    }

    /// `injectMouseButtonUp(button)`; true when CEGUI handled it. Runs even
    /// after the lease was revoked: a drag cut off mid-press still lets go.
    pub async fn button_up(&self, button: u32) -> Result<bool, String> {
        self.call_as(
            INJECT_MOUSE_BUTTON_UP_VA,
            self.system,
            &[button],
            "u32",
            true,
        )
        .await
        .map(|r| r & 0xFF != 0)
    }

    /// The C++ `Window*` behind a named window: the word stored at the
    /// address of its Lua userdata.
    pub async fn window_ptr(&self, name: &str) -> Result<u32, String> {
        let v = self.sup.lua_json(&window_userdata_chunk(name)).await?;
        if v["found"] != json!(true) {
            return Err(format!("no UI window named {name}"));
        }
        let ud = v["ud"].as_str().unwrap_or_default();
        let addr =
            parse_userdata_addr(ud).ok_or_else(|| format!("{name}: unexpected userdata {ud:?}"))?;
        let ptr = self.read_u32(addr).await?;
        if ptr == 0 {
            return Err(format!("{name}: its userdata holds a null Window*"));
        }
        Ok(ptr)
    }

    /// [`Self::window_ptr`], refused unless it is a `DragContainer`.
    pub async fn drag_container(&self, name: &str) -> Result<u32, String> {
        let ptr = self.window_ptr(name).await?;
        let vtable = self.read_u32(ptr).await?;
        check_drag_container(name, vtable, self.slide)?;
        Ok(ptr)
    }

    /// `container->d_dropTarget`, or 0.
    pub async fn drop_target(&self, container: u32) -> Result<u32, String> {
        self.read_u32(container.wrapping_add(DROP_TARGET_OFFSET))
            .await
    }

    /// Whether `container` itself is being dragged (its `+0x23e` byte).
    pub async fn is_dragging(&self, container: u32) -> Result<bool, String> {
        self.read_u32(container.wrapping_add(DRAG_FLAGS_OFFSET))
            .await
            .map(container_dragging)
    }

    /// `target->notifyDragDropItemDropped(item)`.
    pub async fn notify_dropped(&self, target: u32, item: u32) -> Result<(), String> {
        self.call(NOTIFY_DRAG_DROP_ITEM_DROPPED_VA, target, &[item], "void")
            .await
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_travel_as_their_ieee_bits() {
        assert_eq!(float_word(1.0), 0x3f80_0000);
        assert_eq!(float_word(512.0), 0x4400_0000);
        assert_eq!(float_word(0.0), 0);
        assert_eq!(float_word(-2.0), 0xc000_0000);
    }

    #[test]
    fn thiscall_params_put_this_first_as_hex_words() {
        let p = thiscall_params(
            INJECT_MOUSE_POSITION_VA,
            0x0abc_0000,
            &[float_word(100.0), float_word(50.0)],
            "void",
        );
        assert_eq!(p["addr"], "0x11aeaa0");
        assert_eq!(p["conv"], "thiscall");
        assert_eq!(p["ret"], "void");
        assert_eq!(p["args"], json!(["0xabc0000", "0x42c80000", "0x42480000"]));
    }

    #[test]
    fn rebase_applies_the_slide_both_ways() {
        assert_eq!(rebase(SYSTEM_SINGLETON_VA, 0), 0x01f0_10c8);
        assert_eq!(rebase(SYSTEM_SINGLETON_VA, 0x10_0000), 0x0200_10c8);
        assert_eq!(rebase(SYSTEM_SINGLETON_VA, -0x10_0000), 0x01e0_10c8);
    }

    #[test]
    fn userdata_addresses_parse_with_or_without_a_prefix() {
        assert_eq!(parse_userdata_addr("userdata: 0A1B2C3D"), Some(0x0a1b_2c3d));
        assert_eq!(
            parse_userdata_addr("userdata: 0x0a1b2c3d"),
            Some(0x0a1b_2c3d)
        );
        assert_eq!(parse_userdata_addr("table: 0A1B2C3D"), None);
        assert_eq!(parse_userdata_addr("userdata: 00000000"), None);
        assert_eq!(parse_userdata_addr("userdata: zz"), None);
    }

    #[test]
    fn only_a_drag_container_vtable_passes() {
        assert!(check_drag_container("Slot1", 0x01aa_edc4, 0).is_ok());
        assert!(check_drag_container("Slot1", 0x01ab_edc4, 0x1_0000).is_ok());
        let e = check_drag_container("InventoryWin", 0x01aa_0000, 0).unwrap_err();
        assert!(
            e.contains("InventoryWin is not a CEGUI DragContainer"),
            "{e}"
        );
        assert!(e.contains("0x01aaedc4"), "{e}");
    }

    /// The live words: pressed but not dragging, then dragging.
    #[test]
    fn the_dragging_flag_is_the_third_byte() {
        assert!(!container_dragging(0x0000_0001));
        assert!(container_dragging(0x0001_0101));
        assert!(container_dragging(0x0001_0000));
        assert!(!container_dragging(0x0100_0101));
    }

    #[test]
    fn a_null_drop_target_needs_the_explicit_drop() {
        assert_eq!(drop_plan(true, 0, true), DropPlan::Notify);
        assert_eq!(drop_plan(true, 0x0bad_f00d, true), DropPlan::Natural);
        assert_eq!(drop_plan(true, 0x0bad_f00d, false), DropPlan::Natural);
        assert_eq!(drop_plan(true, 0, false), DropPlan::Release);
        // No drag, nothing to drop, whatever the target reads.
        assert_eq!(drop_plan(false, 0, true), DropPlan::Release);
        assert_eq!(drop_plan(false, 0x0bad_f00d, true), DropPlan::Release);
    }

    use std::sync::{Arc, Mutex};

    use crate::supervisor::cegui_fake::{self, FakeCegui};
    use crate::supervisor::events::fake_bridge;
    use crate::supervisor::input::CursorVia;

    async fn fake(state: FakeCegui) -> (Supervisor, Arc<Mutex<FakeCegui>>) {
        let s = Arc::new(Mutex::new(state));
        (
            fake_bridge::supervisor(cegui_fake::responder(s.clone())).await,
            s,
        )
    }

    /// The cursor moves through `injectMousePosition` on the `System`
    /// singleton (a real `MouseMove`), never through Lua `setPosition`.
    #[tokio::test]
    async fn cursor_moves_through_native_injection() {
        let (sup, s) = fake(FakeCegui::default()).await;
        let p = sup.place_cursor(300, 200).await.unwrap();
        assert_eq!(p.at, (300, 200));
        assert_eq!(p.via, CursorVia::NativeInject);
        let s = s.lock().unwrap();
        assert_eq!(s.calls, vec!["move 300 200"]);
        assert_eq!(s.lua_placements, 0);
        assert_eq!(p.to_json()["native_level"], "native_cegui");
    }

    /// The injector addresses move with the module slide.
    #[tokio::test]
    async fn cursor_injection_follows_the_slide() {
        let (sup, s) = fake(FakeCegui {
            slide: 0x1_0000,
            ..FakeCegui::default()
        })
        .await;
        sup.place_cursor(5, 6).await.unwrap();
        assert_eq!(s.lock().unwrap().calls, vec!["move 5 6"]);
    }

    /// No UI yet (null `System`): the cursor is still placed, through Lua,
    /// and the result says why and at what level.
    #[tokio::test]
    async fn a_null_system_falls_back_to_lua_and_says_so() {
        let (sup, s) = fake(FakeCegui {
            system: 0,
            ..FakeCegui::default()
        })
        .await;
        let p = sup.place_cursor(40, 50).await.unwrap();
        assert_eq!(p.at, (40, 50));
        let CursorVia::LuaSetPosition { native_error } = &p.via else {
            panic!("expected the Lua fallback, got {:?}", p.via);
        };
        assert!(native_error.contains("null singleton"), "{native_error}");
        {
            let s = s.lock().unwrap();
            assert!(s.calls.is_empty(), "no native call on a null System");
            assert_eq!(s.lua_placements, 1);
        }
        let j = p.to_json();
        assert_eq!(j["cursor_via"], "lua_set_position");
        assert_eq!(j["native_level"], "client_ui_lua");
        let mut trail = crate::supervisor::ui::NativeTrail::default();
        p.note_fallback(&mut trail);
        assert_eq!(
            trail.overall(),
            Some(crate::supervisor::ui::NativeLevel::ClientUiLua)
        );
    }

    #[test]
    fn window_chunk_finds_by_name_and_returns_the_userdata() {
        let c = window_userdata_chunk("InventoryMod_Slot3");
        assert!(c.contains("__lab_find(\"InventoryMod_Slot3\")"));
        assert!(c.contains("ud = tostring(w)"));
        let full = crate::supervisor::ui::lua_json::chunk(&c);
        assert!(full.contains("local function __jcall"));
    }
}
