//! Lab input injection: drive the game through its own DirectInput
//! devices, without window focus (see [`dinput`] for the hooks and
//! [`state`] for the queued input).
//!
//! Bridge methods:
//!
//! - `input_key {key | dik, down}` — press or release a key.
//! - `input_mouse {dx?, dy?, wheel?, button?, down?}` — relative motion,
//!   wheel, and button changes.
//! - `input_release` — let go of every held key and button.
//! - `input_focus {hwnd?}` — with a window handle, `GetForegroundWindow`
//!   reports that window, so the game keeps reading input while another
//!   window has focus; without one, focus reporting goes back to normal.
//! - `input_status` — the hooks' call counts, the devices seen, and what
//!   is held.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use serde::Deserialize;
use serde_json::{json, Value};

use super::dispatch::{RpcResponse, INTERNAL_ERROR, INVALID_PARAMS};

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub mod dinput;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub mod focus;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub mod modifiers;
pub mod state;

use state::{dik_for, InputState};

/// Process-global queued input.
pub fn input_state() -> &'static Mutex<InputState> {
    static S: OnceLock<Mutex<InputState>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(InputState::default()))
}

/// The window `GetForegroundWindow` reports while virtual focus is on
/// (`0` = off).
static VIRTUAL_FOCUS_HWND: AtomicUsize = AtomicUsize::new(0);

/// The window to report as foreground, when the lab has virtual focus on.
pub fn virtual_focus_hwnd() -> Option<usize> {
    match VIRTUAL_FOCUS_HWND.load(Ordering::Relaxed) {
        0 => None,
        h => Some(h),
    }
}

/// Virtual cursor in UI (client) pixels, packed `x << 32 | y` as `i32`s;
/// `u64::MAX` = none.
static VIRTUAL_CURSOR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);

/// The lab's cursor position, if it has set one.
pub fn virtual_cursor() -> Option<(i32, i32)> {
    match VIRTUAL_CURSOR.load(Ordering::Relaxed) {
        u64::MAX => None,
        v => Some(((v >> 32) as u32 as i32, v as u32 as i32)),
    }
}

fn set_virtual_cursor(pos: Option<(i32, i32)>) {
    let v = pos.map_or(u64::MAX, |(x, y)| {
        (u64::from(x as u32) << 32) | u64::from(y as u32)
    });
    VIRTUAL_CURSOR.store(v, Ordering::Relaxed);
}

#[derive(Deserialize, Default)]
struct CursorParams {
    x: Option<i32>,
    y: Option<i32>,
}

/// `input_cursor {x, y}` — place the virtual cursor (UI pixels); without
/// coordinates, hand the cursor back to the OS.
pub fn dispatch_cursor(id: Value, params: &Value) -> RpcResponse {
    let p: CursorParams = serde_json::from_value(params.clone()).unwrap_or_default();
    let pos = p.x.zip(p.y);
    set_virtual_cursor(pos);
    RpcResponse::ok(id, json!({ "virtual_cursor": pos }))
}

/// Install the DirectInput hooks. Called at bridge start, before the game
/// creates its devices.
pub fn install() -> Result<(), String> {
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    {
        // SAFETY: runs once during boot of a lab session; every IAT slot
        // is verified inside before it is swapped.
        unsafe {
            dinput::install()?;
            focus::install()?;
            modifiers::install()
        }
    }
    #[cfg(not(all(target_os = "windows", target_arch = "x86")))]
    {
        Err("DirectInput hooks exist only in the i686 client DLL".into())
    }
}

#[derive(Deserialize)]
struct KeyParams {
    key: Option<String>,
    dik: Option<u8>,
    down: bool,
}

#[derive(Deserialize, Default)]
struct MouseParams {
    #[serde(default)]
    dx: i32,
    #[serde(default)]
    dy: i32,
    #[serde(default)]
    wheel: i32,
    button: Option<usize>,
    down: Option<bool>,
}

#[derive(Deserialize, Default)]
struct FocusParams {
    hwnd: Option<usize>,
}

fn with_state<F: FnOnce(&mut InputState) -> Value>(id: Value, f: F) -> RpcResponse {
    match input_state().lock() {
        Ok(mut s) => RpcResponse::ok(id, f(&mut s)),
        Err(_) => RpcResponse::error(id, INTERNAL_ERROR, "input state poisoned"),
    }
}

fn held(s: &InputState) -> Value {
    json!({ "keys": s.held_keys(), "buttons": s.held_buttons() })
}

/// `input_key`.
pub fn dispatch_key(id: Value, params: &Value) -> RpcResponse {
    let p: KeyParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => return RpcResponse::error(id, INVALID_PARAMS, format!("input_key: {e}")),
    };
    let dik = match (p.dik, p.key.as_deref()) {
        (Some(d), _) => d,
        (None, Some(name)) => match dik_for(name) {
            Some(d) => d,
            None => return RpcResponse::error(id, INVALID_PARAMS, format!("unknown key {name:?}")),
        },
        (None, None) => return RpcResponse::error(id, INVALID_PARAMS, "need `key` or `dik`"),
    };
    with_state(id, |s| {
        s.key(dik, p.down);
        held(s)
    })
}

/// `input_mouse`.
pub fn dispatch_mouse(id: Value, params: &Value) -> RpcResponse {
    let p: MouseParams = if params.is_null() {
        MouseParams::default()
    } else {
        match serde_json::from_value(params.clone()) {
            Ok(p) => p,
            Err(e) => return RpcResponse::error(id, INVALID_PARAMS, format!("input_mouse: {e}")),
        }
    };
    with_state(id, |s| {
        if p.dx != 0 || p.dy != 0 {
            s.mouse_move(p.dx, p.dy);
        }
        if p.wheel != 0 {
            s.mouse_wheel(p.wheel);
        }
        if let (Some(b), Some(down)) = (p.button, p.down) {
            s.mouse_button(b, down);
        }
        held(s)
    })
}

/// `input_release`.
pub fn dispatch_release(id: Value) -> RpcResponse {
    with_state(id, |s| {
        s.release_all();
        held(s)
    })
}

/// `input_focus`.
pub fn dispatch_focus(id: Value, params: &Value) -> RpcResponse {
    let p: FocusParams = serde_json::from_value(params.clone()).unwrap_or_default();
    VIRTUAL_FOCUS_HWND.store(p.hwnd.unwrap_or(0), Ordering::Relaxed);
    with_state(id, |s| {
        s.virtual_focus = p.hwnd.is_some();
        json!({ "virtual_focus_hwnd": p.hwnd })
    })
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
fn last_data_call() -> Value {
    let v = dinput::LAST_DATA_CALL.load(Ordering::Relaxed);
    json!({ "record_size": v >> 48, "capacity": (v >> 32) & 0xFFFF, "flags": v & 0xFFFF_FFFF })
}

#[derive(Deserialize, Default)]
struct ModifierParams {
    #[serde(default)]
    shift: bool,
    #[serde(default)]
    ctrl: bool,
    #[serde(default)]
    alt: bool,
}

/// `input_modifiers {shift?, ctrl?, alt?}` — the modifiers the game's
/// keyboard-state reads report held (all released when omitted).
pub fn dispatch_modifiers(id: Value, params: &Value) -> RpcResponse {
    let p: ModifierParams = serde_json::from_value(params.clone()).unwrap_or_default();
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    modifiers::set_held(
        (if p.shift { modifiers::SHIFT } else { 0 })
            | (if p.ctrl { modifiers::CTRL } else { 0 })
            | (if p.alt { modifiers::ALT } else { 0 }),
    );
    RpcResponse::ok(
        id,
        json!({ "shift": p.shift, "ctrl": p.ctrl, "alt": p.alt }),
    )
}

/// `input_status`.
pub fn dispatch_status(id: Value) -> RpcResponse {
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    let hooks = json!({
        "get_device_state_calls": dinput::CALLS_STATE.load(Ordering::Relaxed),
        "get_device_data_calls": dinput::CALLS_DATA.load(Ordering::Relaxed),
        "data_reads_keyboard": dinput::DATA_READS_KEYBOARD.load(Ordering::Relaxed),
        "data_reads_mouse": dinput::DATA_READS_MOUSE.load(Ordering::Relaxed),
        "events_delivered": dinput::EVENTS_DELIVERED.load(Ordering::Relaxed),
        "acquires_faked": dinput::ACQUIRES_FAKED.load(Ordering::Relaxed),
        "last_data_call": last_data_call(),
        "devices": dinput::devices()
            .into_iter()
            .map(|(obj, kind)| json!({ "object": format!("0x{obj:08x}"), "kind": format!("{kind:?}") }))
            .collect::<Vec<_>>(),
    });
    #[cfg(not(all(target_os = "windows", target_arch = "x86")))]
    let hooks = Value::Null;
    with_state(id, |s| {
        json!({
            "hooks": hooks,
            "held": held(s),
            "virtual_focus_hwnd": virtual_focus_hwnd(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Negative coordinates survive the packing (a cursor left of or
    /// above the client area), and clearing restores "no virtual cursor".
    #[test]
    fn virtual_cursor_round_trips_and_clears() {
        set_virtual_cursor(Some((512, 219)));
        assert_eq!(virtual_cursor(), Some((512, 219)));
        set_virtual_cursor(Some((-5, -7)));
        assert_eq!(virtual_cursor(), Some((-5, -7)));
        set_virtual_cursor(None);
        assert_eq!(virtual_cursor(), None);
    }
}
