//! Resolving a UE3 `FName` index to its text, and the per-name flag the UE3
//! log devices honour.
//!
//! `FName::Names` is a `TArray<FNameEntry*>` at `0x01ecade0` (data pointer)
//! and `0x01ecade4` (count): `FName::StaticInit` (`0x0049ba90`) zeroes it
//! with `MOVQ [0x01ecade0], XMM0` and `MOV [0x01ecade8], 0`, and the
//! `FName`-to-string routine `0x0049b190` indexes it as
//! `entry = GNames.Data[index]` and copies the wide string at `entry + 0x10`.
//!
//! The entry layout, read from `FOutputDeviceDebug::Serialize`
//! (`0x004cc9f0`) and the same routine:
//!
//! | Offset | Field |
//! |---|---|
//! | `+0x00` | index |
//! | `+0x08` | flags (`u32`; bit `0x1000` = suppressed log name, `RF_Suppress`) |
//! | `+0x10` | the name, UTF-16, NUL-terminated |
//!
//! The debug device (and the file and console devices, which read the same
//! flag) skip a line whose name has `0x1000` set. That is how UE3 silences a
//! log category, so it is also what `unfilter` clears.

use super::mem::{self, Reader};
use super::text;

/// `FName::Names.Data`, the pointer to the array of `FNameEntry*`.
pub const GNAMES_DATA_ADDR: usize = 0x01ec_ade0;

/// `FName::Names.Num`.
pub const GNAMES_NUM_ADDR: usize = 0x01ec_ade4;

/// Offset of the flags word in an `FNameEntry`.
pub const ENTRY_FLAGS_OFFSET: usize = 0x08;

/// Offset of the wide name in an `FNameEntry`.
pub const ENTRY_NAME_OFFSET: usize = 0x10;

/// The suppressed-log-name flag.
pub const FLAG_SUPPRESS: u32 = 0x1000;

/// Longest name kept. UE3 names are at most `NAME_SIZE` (1024) wide, but
/// the ones this reads are class and log-category names, well under 100.
const MAX_NAME_CHARS: usize = 96;

/// A resolved `FName`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameInfo {
    /// The text.
    pub name: String,
    /// The entry's flags word.
    pub flags: u32,
    /// Address of the entry, for a caller that wants to write its flags.
    pub entry: usize,
}

impl NameInfo {
    /// Whether the log devices skip lines under this name.
    pub fn is_suppressed(&self) -> bool {
        self.flags & FLAG_SUPPRESS != 0
    }
}

/// Resolve `index`. `None` when the table is not set up yet (`StaticInit`
/// has not run, or a test process), the index is out of range or negative,
/// or the entry is null or unreadable.
pub fn resolve(read: Reader, index: i32) -> Option<NameInfo> {
    if index < 0 {
        return None;
    }
    let data = mem::read_u32(read, GNAMES_DATA_ADDR)? as usize;
    let num = mem::read_i32(read, GNAMES_NUM_ADDR)?;
    if data == 0 || index >= num {
        return None;
    }
    let entry = mem::read_u32(read, data + index as usize * 4)? as usize;
    if entry == 0 {
        return None;
    }
    let flags = mem::read_u32(read, entry + ENTRY_FLAGS_OFFSET)?;
    let units = mem::read_wide_z(read, entry + ENTRY_NAME_OFFSET, MAX_NAME_CHARS)?;
    Some(NameInfo {
        name: text::decode_wide(&units.units),
        flags,
        entry,
    })
}

/// Resolve `index` in the client's own memory.
#[cfg(windows)]
pub fn resolve_here(index: i32) -> Option<NameInfo> {
    resolve(&mem::process_reader, index)
}

#[cfg(test)]
mod tests {
    use super::super::mem::fake::FakeMemory;
    use super::*;

    /// A name table with `names[i]` at index `i`, all with `flags`.
    fn table(names: &[(&str, u32)]) -> FakeMemory {
        let mut m = FakeMemory::new();
        let array = 0x0100_0000usize;
        m.put(GNAMES_DATA_ADDR, &(array as u32).to_le_bytes());
        m.put(GNAMES_NUM_ADDR, &(names.len() as i32).to_le_bytes());
        for (i, (name, flags)) in names.iter().enumerate() {
            let entry = 0x0200_0000 + i * 0x100;
            m.put(array + i * 4, &(entry as u32).to_le_bytes());
            m.put(entry, &(i as u32).to_le_bytes());
            m.put(entry + ENTRY_FLAGS_OFFSET, &flags.to_le_bytes());
            m.put_wide(entry + ENTRY_NAME_OFFSET, name);
        }
        m
    }

    #[test]
    fn an_index_resolves_to_its_name_and_flags() {
        let m = table(&[("None", 0), ("Log", 0), ("DevNet", FLAG_SUPPRESS)]);
        let r = m.reader();
        let log = resolve(&r, 1).unwrap();
        assert_eq!(log.name, "Log");
        assert!(!log.is_suppressed());
        assert_eq!(log.entry, 0x0200_0100);
        let net = resolve(&r, 2).unwrap();
        assert_eq!(net.name, "DevNet");
        assert!(net.is_suppressed());
    }

    #[test]
    fn other_flag_bits_do_not_read_as_suppressed() {
        let m = table(&[("X", 0x0000_0200 | 0x0000_0800)]);
        let r = m.reader();
        assert!(!resolve(&r, 0).unwrap().is_suppressed());
    }

    /// The table is not there before `FName::StaticInit`, and never in a
    /// process that is not SGW.exe: no name, no fault.
    #[test]
    fn a_missing_table_resolves_to_nothing() {
        let m = FakeMemory::new();
        assert_eq!(resolve(&m.reader(), 0), None);
    }

    #[test]
    fn out_of_range_and_negative_indexes_resolve_to_nothing() {
        let m = table(&[("None", 0), ("Log", 0)]);
        let r = m.reader();
        assert_eq!(resolve(&r, 2), None);
        assert_eq!(resolve(&r, 1000), None);
        assert_eq!(resolve(&r, -1), None);
    }

    /// `StaticInit` leaves holes: an index below `Num` can still hold a
    /// null entry.
    #[test]
    fn a_null_entry_resolves_to_nothing() {
        let mut m = table(&[("None", 0), ("Log", 0)]);
        m.put(0x0100_0000 + 4, &0u32.to_le_bytes());
        assert_eq!(resolve(&m.reader(), 1), None);
    }

    #[test]
    fn the_table_addresses_are_the_ghidra_ones() {
        // `FName::StaticInit` writes the array header at 0x01ecade0 (data,
        // then count at +4); a moved address means a different build and the
        // fingerprint gate has already refused it, but the constants are the
        // load-bearing facts, so they are pinned.
        assert_eq!(GNAMES_DATA_ADDR, 0x01ec_ade0);
        assert_eq!(GNAMES_NUM_ADDR, GNAMES_DATA_ADDR + 4);
        assert_eq!(FLAG_SUPPRESS, 0x1000);
    }
}
