//! Reading client memory without faulting.
//!
//! Every pointer the DLL follows comes from the game and may be null,
//! stale, or not yet set up (the hooks go in before the world loads). The
//! [`MemoryReader`] trait is the only way the portable logic reads memory:
//! in `SGW.exe` it is [`ProcessMemory`], which checks every page with
//! `VirtualQuery` before copying; in tests it is a fake map of regions.

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

/// The live process, with every read validated page by page.
#[cfg(all(windows, target_arch = "x86"))]
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessMemory;

#[cfg(all(windows, target_arch = "x86"))]
impl MemoryReader for ProcessMemory {
    fn read_bytes(&self, addr: usize, len: usize) -> Option<Vec<u8>> {
        if !is_readable(addr, len) {
            return None;
        }
        let mut out = vec![0u8; len];
        // SAFETY: every page of [addr, addr + len) was just checked to be
        // committed and readable.
        unsafe {
            core::ptr::copy_nonoverlapping(addr as *const u8, out.as_mut_ptr(), len);
        }
        Some(out)
    }
}

/// Page protections that allow reading.
#[cfg(all(windows, target_arch = "x86"))]
const READABLE: u32 = windows_sys::Win32::System::Memory::PAGE_READONLY
    | windows_sys::Win32::System::Memory::PAGE_READWRITE
    | windows_sys::Win32::System::Memory::PAGE_WRITECOPY
    | windows_sys::Win32::System::Memory::PAGE_EXECUTE_READ
    | windows_sys::Win32::System::Memory::PAGE_EXECUTE_READWRITE
    | windows_sys::Win32::System::Memory::PAGE_EXECUTE_WRITECOPY;

/// Whether every byte of `[addr, addr + len)` is committed and readable.
/// Walks the range region by region with `VirtualQuery`, which reads page
/// metadata only, never the pages.
#[cfg(all(windows, target_arch = "x86"))]
pub fn is_readable(addr: usize, len: usize) -> bool {
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
}
