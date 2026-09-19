//! Process memory + module inspection for the bridge.
//!
//! Two read-only primitives:
//!
//! - [`module_info`] — the loaded main module's runtime image base
//!   and the ASLR slide relative to its PE preferred base, so every
//!   other tool can accept Ghidra addresses and apply the slide.
//! - [`guarded_read`] — a page-validated memory read that **never
//!   faults**: it walks the target range with `VirtualQuery` and
//!   refuses any byte that isn't committed and readable before it
//!   copies anything.
//!
//! The pure arithmetic ([`aslr_slide`], [`to_hex`]) is split out so
//! the wire-facing math is unit-testable off Windows.

/// Hard cap on a single `mem_read`. Bounds the response size (hex is
/// 2 chars/byte) and the validation walk. 64 KiB is plenty for
/// probing a struct or a small buffer.
pub const MAX_READ_LEN: usize = 64 * 1024;

/// ASLR slide = runtime base − PE preferred base. Signed: a module
/// can load below its preferred base. All tools add this to a Ghidra
/// VA (which is stated against the preferred base) to reach the live
/// address.
///
/// Pure and platform-independent so the math is pinned by tests.
pub fn aslr_slide(runtime_base: usize, preferred_base: usize) -> i64 {
    runtime_base as i64 - preferred_base as i64
}

/// Lowercase hex of a byte slice, no separators — the `mem_read`
/// result encoding.
pub fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((b & 0x0f) as u32, 16).unwrap());
    }
    s
}

/// Snapshot of the host module's load layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleInfo {
    /// Runtime base of the main executable (SGW.exe).
    pub image_base: usize,
    /// Preferred base from the PE optional header (`ImageBase`).
    pub preferred_base: usize,
    /// `image_base - preferred_base`.
    pub slide: i64,
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod win {
    use super::{aslr_slide, ModuleInfo, MAX_READ_LEN};
    use core::ffi::c_void;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Memory::{
        VirtualQuery, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_GUARD, PAGE_NOACCESS,
    };

    /// Read the main module's runtime base and its PE `ImageBase`.
    ///
    /// `GetModuleHandleW(NULL)` returns the runtime base of the .exe.
    /// The preferred base is read straight out of the mapped PE
    /// headers: DOS `e_lfanew` at base+0x3C, then the PE32 optional
    /// header `ImageBase` at NT+0x34. All reads are page-validated so
    /// a corrupt header can't fault us.
    pub fn module_info() -> Result<ModuleInfo, String> {
        // SAFETY: NULL module handle = the process's own image.
        let base = unsafe { GetModuleHandleW(core::ptr::null()) } as usize;
        if base == 0 {
            return Err("GetModuleHandleW(NULL) returned null".to_string());
        }
        let e_lfanew_bytes = guarded_read(base + 0x3c, 4)?;
        let e_lfanew = u32::from_le_bytes([
            e_lfanew_bytes[0],
            e_lfanew_bytes[1],
            e_lfanew_bytes[2],
            e_lfanew_bytes[3],
        ]) as usize;
        // PE32 IMAGE_OPTIONAL_HEADER.ImageBase is at optional-header
        // offset 0x1C; the optional header starts 0x18 after the NT
        // signature, so ImageBase = NT + 0x34.
        let image_base_bytes = guarded_read(base + e_lfanew + 0x34, 4)?;
        let preferred_base = u32::from_le_bytes([
            image_base_bytes[0],
            image_base_bytes[1],
            image_base_bytes[2],
            image_base_bytes[3],
        ]) as usize;
        Ok(ModuleInfo {
            image_base: base,
            preferred_base,
            slide: aslr_slide(base, preferred_base),
        })
    }

    /// Page-validated read that never faults.
    ///
    /// Walks `[addr, addr+len)` region by region with `VirtualQuery`;
    /// if any region isn't `MEM_COMMIT` with a read-permitting
    /// protection (not `PAGE_NOACCESS`, not `PAGE_GUARD`), the whole
    /// read is refused before a single byte is copied. Only after the
    /// entire range validates do we `copy` it out.
    pub fn guarded_read(addr: usize, len: usize) -> Result<Vec<u8>, String> {
        if len == 0 {
            return Ok(Vec::new());
        }
        if len > MAX_READ_LEN {
            return Err(format!("len {len} exceeds max {MAX_READ_LEN}"));
        }
        let end = addr
            .checked_add(len)
            .ok_or_else(|| "addr + len overflows".to_string())?;

        let mut cur = addr;
        while cur < end {
            // Zero-init the out-param: field set differs across
            // windows-sys versions and 32/64-bit, and VirtualQuery
            // fills it entirely. SAFETY: MEMORY_BASIC_INFORMATION is
            // plain POD.
            let mut mbi: MEMORY_BASIC_INFORMATION = unsafe { core::mem::zeroed() };
            // SAFETY: mbi is a valid out-param; VirtualQuery only
            // reads page metadata, never the page contents.
            let n = unsafe {
                VirtualQuery(
                    cur as *const c_void,
                    &mut mbi,
                    core::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
                )
            };
            if n == 0 {
                return Err(format!("VirtualQuery failed at {cur:#x}"));
            }
            if mbi.State != MEM_COMMIT
                || (mbi.Protect & PAGE_NOACCESS) != 0
                || (mbi.Protect & PAGE_GUARD) != 0
            {
                return Err(format!(
                    "address {cur:#x} not readable (state={:#x} protect={:#x})",
                    mbi.State, mbi.Protect
                ));
            }
            let region_end = (mbi.BaseAddress as usize)
                .checked_add(mbi.RegionSize)
                .ok_or_else(|| "region end overflows".to_string())?;
            // Guard against a degenerate region that wouldn't advance.
            if region_end <= cur {
                return Err(format!("VirtualQuery region did not advance at {cur:#x}"));
            }
            cur = region_end;
        }

        let mut out = vec![0u8; len];
        // SAFETY: every page in [addr, addr+len) validated committed
        // and readable above, so this copy cannot fault.
        unsafe {
            core::ptr::copy_nonoverlapping(addr as *const u8, out.as_mut_ptr(), len);
        }
        Ok(out)
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub use win::{guarded_read, module_info};

/// Off-target stubs so the crate's rlib builds and the dispatch layer
/// compiles everywhere; the real primitives only exist inside
/// SGW.exe (Windows i686).
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
pub fn module_info() -> Result<ModuleInfo, String> {
    Err("module_info is only available in the injected DLL (windows i686)".to_string())
}

#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
pub fn guarded_read(_addr: usize, _len: usize) -> Result<Vec<u8>, String> {
    Err("guarded_read is only available in the injected DLL (windows i686)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slide_positive_negative_zero() {
        // Loaded above preferred base.
        assert_eq!(aslr_slide(0x0142_0000, 0x0040_0000), 0x0102_0000);
        // Loaded below preferred base — slide is negative.
        assert_eq!(aslr_slide(0x0030_0000, 0x0040_0000), -0x0010_0000);
        // No relocation — the common SGW.exe case with ASLR off.
        assert_eq!(aslr_slide(0x0040_0000, 0x0040_0000), 0);
    }

    /// Applying the slide to a Ghidra VA recovers the runtime address,
    /// which is the whole point of reporting it.
    #[test]
    fn slide_applies_to_ghidra_va() {
        let runtime_base = 0x0142_0000usize;
        let preferred_base = 0x0040_0000usize;
        let slide = aslr_slide(runtime_base, preferred_base);
        let ghidra_va = 0x0041_6ec0i64; // FEngineLoop::Tick, stated vs preferred base
        let runtime = (ghidra_va + slide) as usize;
        assert_eq!(runtime, 0x0143_6ec0);
    }

    #[test]
    fn hex_encoding() {
        assert_eq!(to_hex(&[0x00, 0x0f, 0xa5, 0xff]), "000fa5ff");
        assert_eq!(to_hex(&[]), "");
        assert_eq!(to_hex(&[0xde, 0xad, 0xbe, 0xef]), "deadbeef");
    }
}
