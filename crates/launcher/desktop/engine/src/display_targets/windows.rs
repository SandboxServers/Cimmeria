//! Windows displays through `EnumDisplayMonitors`, by raw FFI to user32 and
//! shcore.
//!
//! The calls and the `MONITORINFOEXW`/`DEVMODEW` layouts are declared here, so the
//! engine's `windows-sys` feature list (and the hakari workspace-hack) stays
//! unchanged. `rcMonitor` is divided by the monitor's effective scale
//! (`GetDpiForMonitor`, `MDT_EFFECTIVE_DPI`), because SGW.exe is not DPI-aware
//! and Windows scales its window by that factor. In the per-monitor-aware Tauri
//! launcher that turns pixels into the units the game sees. In a DPI-unaware
//! process `rcMonitor` is already virtualized and the DPI reads 96, so the size
//! comes out the same, with `scale_percent` 100.
use super::DisplayInfo;
use std::ffi::c_void;

type Bool = i32;
type HMonitor = *mut c_void;
type Hdc = *mut c_void;
type LParam = isize;
type HResult = i32;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

/// `MONITORINFOEXW`.
#[repr(C)]
#[allow(dead_code)] // layout fields the API fills; only some are read
struct MonitorInfoExW {
    cb_size: u32,
    rc_monitor: Rect,
    rc_work: Rect,
    dw_flags: u32,
    sz_device: [u16; 32],
}

/// `DEVMODEW` (220 bytes); only the size, the frequency and the name are read.
#[repr(C)]
#[allow(dead_code)] // layout fields the API fills; only some are read
struct DevModeW {
    dm_device_name: [u16; 32],
    dm_spec_version: u16,
    dm_driver_version: u16,
    dm_size: u16,
    dm_driver_extra: u16,
    dm_fields: u32,
    printer_or_display: [u8; 16],
    dm_color_to_collate: [i16; 5],
    dm_form_name: [u16; 32],
    dm_log_pixels: u16,
    dm_bits_per_pel: u32,
    dm_pels_width: u32,
    dm_pels_height: u32,
    dm_display_flags: u32,
    dm_display_frequency: u32,
    dm_icm_to_panning: [u32; 8],
}
const _: () = assert!(std::mem::size_of::<DevModeW>() == 220);
const _: () = assert!(std::mem::size_of::<MonitorInfoExW>() == 104);

const MONITORINFOF_PRIMARY: u32 = 1;
const ENUM_CURRENT_SETTINGS: u32 = u32::MAX;
const MDT_EFFECTIVE_DPI: i32 = 0;

type MonitorEnumProc = unsafe extern "system" fn(HMonitor, Hdc, *mut Rect, LParam) -> Bool;

#[link(name = "user32")]
extern "system" {
    fn EnumDisplayMonitors(
        hdc: Hdc,
        clip: *const Rect,
        callback: MonitorEnumProc,
        data: LParam,
    ) -> Bool;
    fn GetMonitorInfoW(monitor: HMonitor, info: *mut MonitorInfoExW) -> Bool;
    fn EnumDisplaySettingsW(device: *const u16, mode: u32, devmode: *mut DevModeW) -> Bool;
}

#[link(name = "shcore")]
extern "system" {
    fn GetDpiForMonitor(
        monitor: HMonitor,
        dpi_type: i32,
        dpi_x: *mut u32,
        dpi_y: *mut u32,
    ) -> HResult;
}

/// More monitors than anyone connects; bounds the list.
const MAX_DISPLAYS: usize = 32;

unsafe extern "system" fn collect(monitor: HMonitor, _: Hdc, _: *mut Rect, data: LParam) -> Bool {
    // SAFETY: `data` is the `&mut Vec<HMonitor>` `list` passes, alive for the call.
    let monitors = unsafe { &mut *(data as *mut Vec<HMonitor>) };
    monitors.push(monitor);
    Bool::from(monitors.len() < MAX_DISPLAYS)
}

pub(super) fn list() -> Vec<DisplayInfo> {
    let mut monitors: Vec<HMonitor> = Vec::new();
    // SAFETY: the callback only pushes into `monitors`, which outlives the call.
    let ok = unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            collect,
            &mut monitors as *mut Vec<HMonitor> as LParam,
        )
    };
    if ok == 0 && monitors.is_empty() {
        return Vec::new();
    }
    monitors
        .into_iter()
        .filter_map(describe)
        .enumerate()
        .map(|(index, mut d)| {
            d.index = index as u32;
            d
        })
        .collect()
}

fn describe(monitor: HMonitor) -> Option<DisplayInfo> {
    // SAFETY: plain-data struct; all-zero is a valid value.
    let mut info: MonitorInfoExW = unsafe { std::mem::zeroed() };
    info.cb_size = std::mem::size_of::<MonitorInfoExW>() as u32;
    // SAFETY: `info` is a correctly sized MONITORINFOEXW with `cbSize` set.
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return None;
    }
    // SAFETY: plain-data struct; all-zero is a valid value.
    let mut mode: DevModeW = unsafe { std::mem::zeroed() };
    mode.dm_size = std::mem::size_of::<DevModeW>() as u16;
    // SAFETY: `sz_device` is the NUL-terminated name GetMonitorInfoW wrote.
    let refresh_hz = if unsafe {
        EnumDisplaySettingsW(info.sz_device.as_ptr(), ENUM_CURRENT_SETTINGS, &mut mode)
    } != 0
        && mode.dm_display_frequency > 1
    {
        mode.dm_display_frequency // 0 and 1 mean "hardware default"
    } else {
        0
    };
    let (mut dpi_x, mut dpi_y) = (0u32, 0u32);
    // SAFETY: `monitor` came from EnumDisplayMonitors; both out-pointers are live locals.
    let hr = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
    let scale_percent = if hr < 0 {
        100
    } else {
        super::scale_percent_from_dpi(dpi_x)
    };
    let rc = info.rc_monitor;
    let (x, y, width, height) =
        super::logical_rect(rc.left, rc.top, rc.right, rc.bottom, scale_percent);
    Some(DisplayInfo {
        index: 0,
        main: info.dw_flags & MONITORINFOF_PRIMARY != 0,
        x,
        y,
        width,
        height,
        scale_percent,
        refresh_hz,
    })
}
