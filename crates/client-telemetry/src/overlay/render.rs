//! One bounded, opaque D3D9 panel; no game state, allocations or text.

#[cfg(all(windows, target_arch = "x86"))]
use std::ffi::c_void;

use super::{STATUS_GAP, STATUS_RECORDING, STATUS_TRACING};

pub(super) trait Snapshot {
    fn restore(&mut self) -> Result<(), ()>;
}

pub(super) trait DeviceOps {
    type Saved: Snapshot;

    fn viewport(&mut self) -> Result<(u32, u32), ()>;
    fn snapshot(&mut self) -> Result<Self::Saved, ()>;
    fn clear(&mut self, rect: Rect, color: u32) -> Result<(), ()>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub(super) struct Rect {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

/// At most three D3D Clear calls per frame. `within_budget` is checked
/// before each mutation; a slow driver call cannot be preempted.
pub(super) fn draw_panel<D: DeviceOps>(
    device: &mut D,
    status: u32,
    within_budget: impl Fn() -> bool,
) -> Result<(), ()> {
    if !within_budget() {
        return Ok(());
    }
    let (width, height) = device.viewport()?;
    if width < 216 || height < 60 || !within_budget() {
        return Ok(());
    }
    let mut state = device.snapshot()?;
    let base = Rect {
        x1: 12,
        y1: 12,
        x2: 204,
        y2: 52,
    };
    let result = (|| {
        if !within_budget() {
            return Ok(());
        }
        device.clear(base, 0xff10_1720)?;
        if !within_budget() {
            return Ok(());
        }
        device.clear(
            Rect {
                x1: 18,
                y1: 18,
                x2: 112,
                y2: 46,
            },
            if status & STATUS_GAP != 0 {
                0xffe0_5b43
            } else if status & STATUS_TRACING != 0 {
                0xff44_a6e0
            } else {
                0xff66_7480
            },
        )?;
        if !within_budget() {
            return Ok(());
        }
        device.clear(
            Rect {
                x1: 118,
                y1: 18,
                x2: 198,
                y2: 46,
            },
            if status & STATUS_RECORDING != 0 {
                0xffe0_aa44
            } else {
                0xff42_4b54
            },
        )
    })();
    let restore = state.restore();
    result.and(restore)
}

/// Thin COM adapter. Only used on the 32-bit Windows injected DLL.
#[cfg(all(windows, target_arch = "x86"))]
pub(super) struct D3dDevice {
    ptr: usize,
}

#[cfg(all(windows, target_arch = "x86"))]
impl D3dDevice {
    pub fn new(ptr: usize) -> Self {
        Self { ptr }
    }
}

#[cfg(all(windows, target_arch = "x86"))]
mod native {
    use super::*;

    #[repr(C)]
    struct Viewport {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        min_z: f32,
        max_z: f32,
    }

    fn method(object: usize, index: usize) -> Result<usize, ()> {
        let vtable =
            crate::hooks::sinks::mem::read_u32(&crate::hooks::sinks::mem::process_reader, object)
                .ok_or(())? as usize;
        crate::hooks::sinks::mem::read_u32(
            &crate::hooks::sinks::mem::process_reader,
            vtable + index * 4,
        )
        .map(|p| p as usize)
        .filter(|p| *p != 0)
        .ok_or(())
    }

    pub(crate) struct D3dSnapshot {
        ptr: usize,
        restored: bool,
    }

    impl Snapshot for D3dSnapshot {
        fn restore(&mut self) -> Result<(), ()> {
            if self.restored {
                return Ok(());
            }
            let apply: unsafe extern "stdcall-unwind" fn(*mut c_void) -> i32 =
                unsafe { std::mem::transmute(method(self.ptr, 5)?) };
            let hr = unsafe { apply(self.ptr as *mut c_void) };
            self.restored = true;
            (hr >= 0).then_some(()).ok_or(())
        }
    }

    impl Drop for D3dSnapshot {
        fn drop(&mut self) {
            let _ = self.restore();
            if let Ok(release) = method(self.ptr, 2) {
                let release: unsafe extern "stdcall-unwind" fn(*mut c_void) -> u32 =
                    unsafe { std::mem::transmute(release) };
                unsafe { release(self.ptr as *mut c_void) };
            }
        }
    }

    impl DeviceOps for D3dDevice {
        type Saved = D3dSnapshot;

        fn viewport(&mut self) -> Result<(u32, u32), ()> {
            let get: unsafe extern "stdcall-unwind" fn(*mut c_void, *mut Viewport) -> i32 =
                unsafe { std::mem::transmute(method(self.ptr, 48)?) };
            let mut viewport = Viewport {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                min_z: 0.0,
                max_z: 0.0,
            };
            let hr = unsafe { get(self.ptr as *mut c_void, &mut viewport) };
            (hr >= 0)
                .then_some((viewport.width, viewport.height))
                .ok_or(())
        }

        fn snapshot(&mut self) -> Result<Self::Saved, ()> {
            let create: unsafe extern "stdcall-unwind" fn(
                *mut c_void,
                u32,
                *mut *mut c_void,
            ) -> i32 = unsafe { std::mem::transmute(method(self.ptr, 59)?) };
            let mut block = std::ptr::null_mut();
            let hr = unsafe { create(self.ptr as *mut c_void, 1, &mut block) };
            if hr < 0 || block.is_null() {
                return Err(());
            }
            Ok(D3dSnapshot {
                ptr: block as usize,
                restored: false,
            })
        }

        fn clear(&mut self, rect: Rect, color: u32) -> Result<(), ()> {
            let clear: unsafe extern "stdcall-unwind" fn(
                *mut c_void,
                u32,
                *const Rect,
                u32,
                u32,
                f32,
                u32,
            ) -> i32 = unsafe { std::mem::transmute(method(self.ptr, 43)?) };
            let hr = unsafe { clear(self.ptr as *mut c_void, 1, &rect, 1, color, 1.0, 0) };
            (hr >= 0).then_some(()).ok_or(())
        }
    }
}
