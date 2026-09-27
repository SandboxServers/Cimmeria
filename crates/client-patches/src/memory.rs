//! Reading client memory without faulting.
//!
//! Every pointer the DLL follows comes from the game and may be null,
//! stale, or not yet set up (the hooks go in before the world loads). The
//! [`MemoryReader`] trait is the only way the portable logic reads memory:
//! in `SGW.exe` it is [`ProcessMemory`], which copies through
//! `ReadProcessMemory`; in tests it is a fake map of regions.

/// Bounded reads of another component's memory. A read that is not fully
/// readable returns `None`; it never faults.
pub trait MemoryReader {
    /// `len` bytes at `addr`, or `None` if any of them is unreadable.
    fn read_bytes(&self, addr: usize, len: usize) -> Option<Vec<u8>>;

    /// A little-endian `u32` at `addr`.
    fn read_u32(&self, addr: usize) -> Option<u32> {
        let bytes = self.read_bytes(addr, 4)?;
        Some(u32::from_le_bytes(bytes.try_into().ok()?))
    }

    /// A pointer-sized field at `addr` that must be non-null.
    fn read_ptr(&self, addr: usize) -> Option<usize> {
        match self.read_u32(addr)? {
            0 => None,
            p => Some(p as usize),
        }
    }
}

/// The live process. Reads go through `ReadProcessMemory` on the current
/// process, which fails instead of faulting.
///
/// A `VirtualQuery` check followed by a plain copy is not enough: another
/// thread can free or reprotect the page between the two, and the access
/// violation that follows is a structured exception, which `catch_unwind`
/// does not catch. The kernel's copy reports a page that went away as a
/// failed call. The `VirtualQuery` walk is kept in front as a cheap reject,
/// and because it refuses guard pages, which a read would otherwise trip.
#[cfg(all(windows, target_arch = "x86"))]
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessMemory;

#[cfg(all(windows, target_arch = "x86"))]
impl ProcessMemory {
    /// Fill `out` from `addr`. `false`, with `out` unspecified, if any byte
    /// is unreadable.
    pub fn read_into(&self, addr: usize, out: &mut [u8]) -> bool {
        is_readable(addr, out.len()) && os_copy(addr, out)
    }
}

#[cfg(all(windows, target_arch = "x86"))]
impl MemoryReader for ProcessMemory {
    fn read_bytes(&self, addr: usize, len: usize) -> Option<Vec<u8>> {
        let mut out = vec![0u8; len];
        self.read_into(addr, &mut out).then_some(out)
    }
}

/// `ReadProcessMemory` on this process: the whole of `out`, or `false`. It
/// never faults, whatever `addr` points at.
#[cfg(all(windows, target_arch = "x86"))]
fn os_copy(addr: usize, out: &mut [u8]) -> bool {
    use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    if out.is_empty() {
        return true;
    }
    let mut copied = 0usize;
    // SAFETY: `out` is a writable buffer of `out.len()` bytes. The source is
    // only read by the kernel, which validates it.
    let ok = unsafe {
        ReadProcessMemory(
            GetCurrentProcess(),
            addr as *const core::ffi::c_void,
            out.as_mut_ptr().cast(),
            out.len(),
            &mut copied,
        )
    };
    ok != 0 && copied == out.len()
}

/// Page protections that allow reading.
#[cfg(all(windows, target_arch = "x86"))]
const READABLE: u32 = windows_sys::Win32::System::Memory::PAGE_READONLY
    | windows_sys::Win32::System::Memory::PAGE_READWRITE
    | windows_sys::Win32::System::Memory::PAGE_WRITECOPY
    | windows_sys::Win32::System::Memory::PAGE_EXECUTE_READ
    | windows_sys::Win32::System::Memory::PAGE_EXECUTE_READWRITE
    | windows_sys::Win32::System::Memory::PAGE_EXECUTE_WRITECOPY;

/// Whether every byte of `[addr, addr + len)` is committed and readable
/// right now. Walks the range region by region with `VirtualQuery`, which
/// reads page metadata only, never the pages. A point-in-time answer: the
/// copy itself must still be one that cannot fault.
#[cfg(all(windows, target_arch = "x86"))]
fn is_readable(addr: usize, len: usize) -> bool {
    use windows_sys::Win32::System::Memory::{
        VirtualQuery, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_GUARD,
    };

    if addr == 0 {
        return false;
    }
    let Some(end) = addr.checked_add(len) else {
        return false;
    };
    let mut cur = addr;
    while cur < end {
        // SAFETY: MEMORY_BASIC_INFORMATION is plain old data.
        let mut mbi: MEMORY_BASIC_INFORMATION = unsafe { core::mem::zeroed() };
        // SAFETY: a valid out-parameter of the right size.
        let n = unsafe {
            VirtualQuery(
                cur as *const core::ffi::c_void,
                &mut mbi,
                core::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        };
        if n == 0
            || mbi.State != MEM_COMMIT
            || mbi.Protect & PAGE_GUARD != 0
            || mbi.Protect & READABLE == 0
        {
            return false;
        }
        let region_end = (mbi.BaseAddress as usize).saturating_add(mbi.RegionSize);
        if region_end <= cur {
            return false;
        }
        cur = region_end;
    }
    true
}

/// A fake address space for tests: a set of non-overlapping regions.
#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct FakeMemory {
    regions: Vec<(usize, Vec<u8>)>,
}

#[cfg(test)]
impl FakeMemory {
    /// Map `bytes` at `addr`.
    pub(crate) fn put(&mut self, addr: usize, bytes: &[u8]) -> &mut Self {
        self.regions.push((addr, bytes.to_vec()));
        self
    }

    /// Map a little-endian `u32` at `addr`.
    pub(crate) fn put_u32(&mut self, addr: usize, value: u32) -> &mut Self {
        self.put(addr, &value.to_le_bytes())
    }
}

#[cfg(test)]
impl MemoryReader for FakeMemory {
    fn read_bytes(&self, addr: usize, len: usize) -> Option<Vec<u8>> {
        let end = addr.checked_add(len)?;
        self.regions.iter().find_map(|(base, bytes)| {
            let region_end = base + bytes.len();
            (addr >= *base && end <= region_end).then(|| bytes[addr - base..end - base].to_vec())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_reads_inside_a_region_only() {
        let mut mem = FakeMemory::default();
        mem.put(0x1000, &[1, 2, 3, 4, 5, 6]);
        assert_eq!(mem.read_bytes(0x1002, 2), Some(vec![3, 4]));
        assert_eq!(mem.read_bytes(0x1004, 4), None, "runs off the end");
        assert_eq!(mem.read_bytes(0x0FFF, 2), None, "starts before");
        assert_eq!(mem.read_u32(0x1000), Some(0x0403_0201));
    }

    #[test]
    fn read_ptr_rejects_null() {
        let mut mem = FakeMemory::default();
        mem.put_u32(0x10, 0).put_u32(0x20, 0x1234);
        assert_eq!(mem.read_ptr(0x10), None);
        assert_eq!(mem.read_ptr(0x20), Some(0x1234));
        assert_eq!(mem.read_ptr(0x30), None, "unmapped");
    }

    /// The live reader refuses null and unmapped memory rather than
    /// faulting, and reads its own stack.
    #[cfg(all(windows, target_arch = "x86"))]
    #[test]
    fn process_memory_never_faults() {
        let local = [0xAAu8, 0xBB, 0xCC, 0xDD];
        let mem = ProcessMemory;
        assert_eq!(
            mem.read_bytes(local.as_ptr() as usize, 4),
            Some(local.to_vec())
        );
        assert_eq!(mem.read_bytes(0, 4), None);
        assert_eq!(mem.read_bytes(0x10, 4), None, "the null page is unmapped");
        assert_eq!(mem.read_bytes(usize::MAX - 1, 4), None, "wraps");
    }

    /// The copy itself survives a page that is no longer readable, which is
    /// what a page freed or reprotected after the `VirtualQuery` check looks
    /// like. A plain pointer copy here would raise an access violation.
    #[cfg(all(windows, target_arch = "x86"))]
    #[test]
    fn os_copy_fails_on_a_page_that_went_away() {
        use windows_sys::Win32::System::Memory::{
            VirtualAlloc, VirtualFree, VirtualProtect, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE,
            PAGE_NOACCESS, PAGE_READWRITE,
        };

        const PAGE: usize = 4096;
        // SAFETY: a fresh private page, used only by this test.
        let page = unsafe {
            VirtualAlloc(
                core::ptr::null(),
                PAGE,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        } as usize;
        assert_ne!(page, 0);
        // SAFETY: the page is committed read-write.
        unsafe { (page as *mut u8).write_bytes(0x5A, PAGE) };
        let mut out = [0u8; 8];
        assert!(os_copy(page, &mut out));
        assert_eq!(out, [0x5A; 8]);

        let mut old = 0u32;
        // SAFETY: the page is ours.
        assert_ne!(
            unsafe { VirtualProtect(page as *const _, PAGE, PAGE_NOACCESS, &mut old) },
            0
        );
        assert!(!os_copy(page, &mut out), "reprotected after the check");
        assert_eq!(ProcessMemory.read_bytes(page, 8), None);

        // SAFETY: the page is ours and nothing refers to it any more.
        assert_ne!(unsafe { VirtualFree(page as *mut _, 0, MEM_RELEASE) }, 0);
        assert!(!os_copy(page, &mut out), "freed after the check");
    }
}
