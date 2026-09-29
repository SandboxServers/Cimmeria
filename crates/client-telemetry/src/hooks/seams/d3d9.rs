//! Direct3D 9 device creation, reset and loss.
//!
//! The client renders through D3D9 (`d3d9.dll`; `SGW.exe` imports
//! `Direct3DCreate9` at IAT `0x017effd8`). A minimise, an alt-tab out of
//! fullscreen, a driver reset or a display-mode change makes the device
//! "lost": presenting fails until the application resets it with the same
//! or new presentation parameters. A client that handles that wrongly
//! freezes or shows a black screen, and nothing above this layer knows it
//! happened. These hooks make the sequence visible.
//!
//! # How it hooks without a fixed address
//!
//! The D3D9 vtables live inside `d3d9.dll`, so their addresses are not
//! properties of `SGW.exe`. The hook follows the COM objects instead, using
//! the interface layout Microsoft documents (`IDirect3D9`,
//! `IDirect3DDevice9`, both `__stdcall`, `this` first on the stack):
//!
//! 1. The IAT slot of `Direct3DCreate9` is swapped (checked against what
//!    `d3d9.dll` exports, like every IAT hook). When it returns an
//!    `IDirect3D9*`, the hook patches vtable slot 16, `CreateDevice`, once.
//! 2. `CreateDevice`'s detour reads the presentation parameters, calls the
//!    original, and on success patches the device's slots 3
//!    (`TestCooperativeLevel`) and 16 (`Reset`), once.
//! 3. Reset and TestCooperativeLevel then report.
//!
//! Only these three are hooked; `Present` (every frame) and the draw calls
//! are deliberately not touched.
//!
//! # Events
//!
//! - **`client.gfx.device_created`**: the `HRESULT`, adapter, behaviour
//!   flags and the requested mode (size, format, windowed, back buffers,
//!   multisample type, present interval, refresh rate).
//! - **`client.gfx.device_reset`**: the `HRESULT`, the requested mode, and
//!   how long `Reset` took in milliseconds (a slow reset is a driver
//!   re-initialising; a failed one leaves the device lost). A failure is a
//!   warning.
//! - **`client.gfx.device_state`**: `TestCooperativeLevel`'s answer, reported
//!   only when it *changes* (`S_OK`, `D3DERR_DEVICELOST`,
//!   `D3DERR_DEVICENOTRESET`, ...), so the loss and the recovery are two
//!   events, not one per frame.
//!
//! `D3DPRESENT_PARAMETERS` (x86): `BackBufferWidth +0x00`, `Height +0x04`,
//! `BackBufferFormat +0x08`, `BackBufferCount +0x0c`, `MultiSampleType
//! +0x10`, `SwapEffect +0x18`, `Windowed +0x20`, `FullScreen_RefreshRateInHz
//! +0x30`, `PresentationInterval +0x34`. The vtable indices are the SDK's
//! (`IDirect3D9::CreateDevice` 16; `IDirect3DDevice9::TestCooperativeLevel`
//! 3, `Reset` 16). A wrong layout would show up as absurd numbers, not a
//! crash: every read is fault-free.
//!
//! Static evidence only (the PE import directory and the documented COM
//! layout); not yet seen from a live client.

use serde_json::json;

use crate::hooks::sinks::emit::Fields;
use crate::hooks::sinks::mem::{self, Reader};

/// IAT slot of `Direct3DCreate9`.
pub const IAT_DIRECT3D_CREATE9: usize = 0x017e_ffd8;

/// `IDirect3D9` vtable index of `CreateDevice`.
pub const VT_CREATE_DEVICE: usize = 16;
/// `IDirect3DDevice9` vtable index of `TestCooperativeLevel`.
pub const VT_TEST_COOPERATIVE_LEVEL: usize = 3;
/// `IDirect3DDevice9` vtable index of `Reset`.
pub const VT_RESET: usize = 16;

/// Target of a device creation.
pub const CREATED_TARGET: &str = "client.gfx.device_created";
/// Target of a reset.
pub const RESET_TARGET: &str = "client.gfx.device_reset";
/// Target of a cooperative-level change.
pub const STATE_TARGET: &str = "client.gfx.device_state";

/// `D3DERR_DEVICELOST`.
pub const D3DERR_DEVICELOST: i32 = 0x8876_0868_u32 as i32;
/// `D3DERR_DEVICENOTRESET`.
pub const D3DERR_DEVICENOTRESET: i32 = 0x8876_0869_u32 as i32;

/// A readable name for the `HRESULT`s a device is likely to return.
pub fn hresult_name(hr: i32) -> &'static str {
    match hr as u32 {
        0 => "S_OK",
        0x8876_0868 => "D3DERR_DEVICELOST",
        0x8876_0869 => "D3DERR_DEVICENOTRESET",
        0x8876_0827 => "D3DERR_DRIVERINTERNALERROR",
        0x8876_086C => "D3DERR_INVALIDCALL",
        0x8876_086A => "D3DERR_NOTAVAILABLE",
        0x8876_017C => "D3DERR_OUTOFVIDEOMEMORY",
        0x8876_0B54 => "D3DERR_DEVICEHUNG",
        0x8007_000E => "E_OUTOFMEMORY",
        _ => "other",
    }
}

/// A `D3DFORMAT` name for the back buffer formats a game asks for.
pub fn format_name(format: u32) -> &'static str {
    match format {
        0 => "UNKNOWN",
        21 => "A8R8G8B8",
        22 => "X8R8G8B8",
        23 => "R5G6B5",
        25 => "A1R5G5B5",
        26 => "A4R4G4B4",
        35 => "A2R10G10B10",
        _ => "other",
    }
}

/// The parts of a `D3DPRESENT_PARAMETERS` the events report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresentParams {
    /// Back buffer width.
    pub width: u32,
    /// Back buffer height.
    pub height: u32,
    /// `D3DFORMAT` of the back buffer.
    pub format: u32,
    /// Number of back buffers.
    pub back_buffers: u32,
    /// `D3DMULTISAMPLE_TYPE`.
    pub multisample: u32,
    /// `D3DSWAPEFFECT`.
    pub swap_effect: u32,
    /// Windowed (`true`) or fullscreen.
    pub windowed: bool,
    /// Fullscreen refresh rate, Hz.
    pub refresh_hz: u32,
    /// `D3DPRESENT_INTERVAL_*` flags.
    pub present_interval: u32,
}

/// Read a `D3DPRESENT_PARAMETERS`.
pub fn read_params(read: Reader, addr: usize) -> Option<PresentParams> {
    let at = |off: usize| mem::read_u32(read, addr + off);
    Some(PresentParams {
        width: at(0x00)?,
        height: at(0x04)?,
        format: at(0x08)?,
        back_buffers: at(0x0c)?,
        multisample: at(0x10)?,
        swap_effect: at(0x18)?,
        windowed: at(0x20)? != 0,
        refresh_hz: at(0x30)?,
        present_interval: at(0x34)?,
    })
}

fn params_fields(f: &mut Fields, p: PresentParams) {
    f.push(("width", json!(p.width)));
    f.push(("height", json!(p.height)));
    f.push(("format", json!(format_name(p.format))));
    f.push(("windowed", json!(p.windowed)));
    f.push(("back_buffers", json!(p.back_buffers)));
    f.push(("multisample", json!(p.multisample)));
    f.push((
        "present_interval",
        json!(format!("0x{:08x}", p.present_interval)),
    ));
    f.push(("refresh_hz", json!(p.refresh_hz)));
}

/// The fields of a device creation.
pub fn created_fields(
    hr: i32,
    adapter: u32,
    behaviour: u32,
    params: Option<PresentParams>,
) -> Fields {
    let mut f: Fields = vec![
        ("hresult", json!(format!("0x{:08x}", hr as u32))),
        ("hresult_name", json!(hresult_name(hr))),
        ("adapter", json!(adapter)),
        ("behaviour_flags", json!(format!("0x{behaviour:08x}"))),
    ];
    if let Some(p) = params {
        params_fields(&mut f, p);
    }
    f
}

/// The fields of a reset.
pub fn reset_fields(hr: i32, elapsed_ms: f64, params: Option<PresentParams>) -> Fields {
    let mut f: Fields = vec![
        ("hresult", json!(format!("0x{:08x}", hr as u32))),
        ("hresult_name", json!(hresult_name(hr))),
        ("ok", json!(hr >= 0)),
        ("elapsed_ms", json!(elapsed_ms.round())),
    ];
    if let Some(p) = params {
        params_fields(&mut f, p);
    }
    f
}

/// The fields of a cooperative-level change.
pub fn state_fields(hr: i32, previous: Option<i32>) -> Fields {
    let mut f: Fields = vec![
        ("hresult", json!(format!("0x{:08x}", hr as u32))),
        ("hresult_name", json!(hresult_name(hr))),
        ("lost", json!(hr == D3DERR_DEVICELOST)),
        ("needs_reset", json!(hr == D3DERR_DEVICENOTRESET)),
    ];
    if let Some(prev) = previous {
        f.push(("previous", json!(hresult_name(prev))));
    }
    f
}

/// Telemetry level of a cooperative-level answer: a lost or hung device is a
/// warning, `S_OK` is `info`.
pub fn state_level(hr: i32) -> &'static str {
    if hr >= 0 {
        "info"
    } else {
        "warn"
    }
}

/// Whether `hr` differs from the last reported one (the first answer always
/// does).
pub fn state_changed(previous: Option<i32>, hr: i32) -> bool {
    previous != Some(hr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::sinks::mem::fake::FakeMemory;

    #[test]
    fn the_device_lost_hresults_have_names_and_the_right_values() {
        assert_eq!(D3DERR_DEVICELOST as u32, 0x8876_0868);
        assert_eq!(D3DERR_DEVICENOTRESET as u32, 0x8876_0869);
        assert_eq!(hresult_name(0), "S_OK");
        assert_eq!(hresult_name(D3DERR_DEVICELOST), "D3DERR_DEVICELOST");
        assert_eq!(hresult_name(D3DERR_DEVICENOTRESET), "D3DERR_DEVICENOTRESET");
        assert_eq!(hresult_name(0x8876_086C_u32 as i32), "D3DERR_INVALIDCALL");
        assert_eq!(
            hresult_name(0x8876_017C_u32 as i32),
            "D3DERR_OUTOFVIDEOMEMORY"
        );
        assert_eq!(hresult_name(1), "other");
    }

    #[test]
    fn a_state_is_reported_when_it_changes_and_the_first_time() {
        assert!(state_changed(None, 0));
        assert!(!state_changed(Some(0), 0));
        assert!(state_changed(Some(0), D3DERR_DEVICELOST));
        assert!(state_changed(Some(D3DERR_DEVICELOST), 0));
    }

    #[test]
    fn a_failed_answer_is_a_warning() {
        assert_eq!(state_level(0), "info");
        assert_eq!(state_level(D3DERR_DEVICELOST), "warn");
        assert_eq!(
            state_level(1),
            "info",
            "S_FALSE-like positives are not failures"
        );
    }

    #[test]
    fn presentation_parameters_read_at_the_sdk_offsets() {
        let mut m = FakeMemory::new();
        for (off, v) in [
            (0x00usize, 1280u32),
            (0x04, 720),
            (0x08, 21),
            (0x0c, 2),
            (0x10, 0),
            (0x18, 3),
            (0x20, 1),
            (0x30, 0),
            (0x34, 0x8000_0000),
        ] {
            m.put(0x1000 + off, &v.to_le_bytes());
        }
        let p = read_params(&m.reader(), 0x1000).unwrap();
        assert_eq!((p.width, p.height), (1280, 720));
        assert_eq!(format_name(p.format), "A8R8G8B8");
        assert_eq!(p.back_buffers, 2);
        assert_eq!(p.swap_effect, 3);
        assert!(p.windowed);
        assert_eq!(p.present_interval, 0x8000_0000);
        assert_eq!(read_params(&m.reader(), 0x9000), None);
    }

    #[test]
    fn a_reset_reports_the_outcome_the_time_and_the_mode() {
        let p = PresentParams {
            width: 1920,
            height: 1080,
            format: 22,
            back_buffers: 1,
            multisample: 0,
            swap_effect: 1,
            windowed: false,
            refresh_hz: 60,
            present_interval: 1,
        };
        let f = reset_fields(D3DERR_DEVICELOST, 412.6, Some(p));
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("ok"), Some(json!(false)));
        assert_eq!(get("hresult"), Some(json!("0x88760868")));
        assert_eq!(get("elapsed_ms"), Some(json!(413.0)));
        assert_eq!(get("refresh_hz"), Some(json!(60)));
        let f = reset_fields(0, 5.0, None);
        assert!(!f.iter().any(|(k, _)| *k == "width"));
    }

    #[test]
    fn a_state_change_says_lost_and_what_it_replaced() {
        let f = state_fields(D3DERR_DEVICELOST, Some(0));
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("lost"), Some(json!(true)));
        assert_eq!(get("needs_reset"), Some(json!(false)));
        assert_eq!(get("previous"), Some(json!("S_OK")));
        let f = state_fields(D3DERR_DEVICENOTRESET, None);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("needs_reset"), Some(json!(true)));
        assert_eq!(get("previous"), None);
    }

    #[test]
    fn the_vtable_indices_are_the_sdks() {
        assert_eq!(VT_CREATE_DEVICE, 16);
        assert_eq!(VT_TEST_COOPERATIVE_LEVEL, 3);
        assert_eq!(VT_RESET, 16);
        assert_eq!(IAT_DIRECT3D_CREATE9, 0x017e_ffd8);
    }

    #[test]
    fn a_device_creation_event_carries_the_flags() {
        let f = created_fields(0, 0, 0x40, None);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("behaviour_flags"), Some(json!("0x00000040")));
        assert_eq!(get("hresult_name"), Some(json!("S_OK")));
    }
}
