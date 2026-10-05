//! The cooked-data cache: what version the client reads out of each cache
//! PAK, whether that read worked, and what it holds when it asks the server.
//!
//! A `ZipStorageBase` (one per cooked category, `LibCategory<..., ZipStorage,
//! ServerSource>`) keeps the category's version at `+0x24`. It is filled from
//! the PAK's `MetaData` entry when the archive is opened, replaced by every
//! `onVersionInfo`, and sent back in `versionInfoRequest` at login. A client
//! whose read fails keeps 0 there and is resynced in full on every login,
//! with a valid cache on disk the whole time (seen 2026-10-04 under Wine on
//! macOS). The server only sees the 0. These hooks say which step failed.
//!
//! | Function | Address | Signature |
//! |---|---|---|
//! | `ZipStorageBase` read version | `0x00478f00` | `thiscall(this, u32* out, CZipArchive*)`, `ret 8` |
//! | `ZipStorageBase` read entry | `0x00478e10` | `thiscall(this, stream*, CZipArchive*, const wchar_t* name)`, `ret 0xc`, `bool` |
//! | `CZipArchive::FindFile` | `0x01396900` | `thiscall(archive, const wchar_t* name, int case, bool name_only)`, `ret 0xc`, `u16` (`0xffff` = not found) |
//! | `CZipArchive::ExtractFile` (to memory) | `0x01398af0` | `thiscall(archive, index, CZipMemFile*, bool, u32 buffer)`, `ret 0x10`, `bool` |
//! | `ServerSource_SetVersion` | `0x00479e90` | `thiscall(this, const u32* version)`, `ret 4` |
//!
//! The read (`0x00478f00`) builds a `std::strstream` over an empty dynamic
//! `strstreambuf` (constructor `0x00478970`, `??0strstreambuf@std@@QAE@H@Z`
//! with 0), calls read entry with the name `MetaData`, and, only if that
//! returns true, calls `basic_istream<char>::read(out, 4)` on the stream.
//! Read entry is `FindFile(name, 0, true)`, then `ExtractFile` into a memory
//! file, a length check, then `basic_ostream<char>::write(buffer, length)`
//! into the stream. So the four bytes go archive → memory file → stream →
//! `out`, and the last two hops are `MSVCP80.dll` code, not the game's.
//! Its three callers are the tail of `ZipStorageBase::OpenArchive`
//! (`0x004798e6`, with `out = this + 0x24`), `0x00479336` and the
//! source-archive update (`0x0047a36e`). `FindFile` has two call sites and
//! `ExtractFile` one, all in this code, so hooking the library functions
//! observes nothing else. Checked against the QA `SGW.exe` on 2026-10-04
//! (function entries, `ret N`, the call sites above, the imports called).
//!
//! `OpenArchive` itself (`0x00479340`) is not hooked: every entry read and
//! write calls it, and a checked memory read there would cost a full resync
//! tens of thousands of them.
//!
//! Events:
//!
//! - `client.cooked.version_read`: one per read. `outcome` is `read`,
//!   `stream_read_short`, `metadata_entry_not_found`,
//!   `metadata_extract_failed`, `metadata_empty`, `entry_read_failed` or
//!   `entry_read_not_attempted`. `warn` unless `read`. It carries each hop:
//!   the archive's directory record for the entry (`zip_*`), what the
//!   extraction left in the memory file (`extract_*`), what the stream held
//!   and gave back (`stream_*`), and the C-runtime file calls made underneath
//!   (`crt_*`, see [`imports`]).
//! - `client.cooked.version_set`: one per `onVersionInfo` stamp.
//! - `client.cooked.versions_held`: once per login, when the first
//!   `versionInfoRequest` leaves (see `net_out`): the version every known
//!   storage holds at that moment.
//!
//! The PAK name is the first `std::wstring` of the vector at `this + 0`
//! (element size `0x1c`, begin at `+0x04`, end at `+0x08`).

use serde_json::json;

use crate::hooks::entity_trace::map::Mem;
use crate::hooks::entity_trace::Fields;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod detours;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod import_detours;
mod imports;
#[cfg(test)]
mod tests;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) use detours::{install_all, note_version_request};

pub(super) const ADDR_READ_VERSION: usize = 0x0047_8f00;
pub(super) const ADDR_READ_ENTRY: usize = 0x0047_8e10;
pub(super) const ADDR_FIND_FILE: usize = 0x0139_6900;
pub(super) const ADDR_EXTRACT_FILE: usize = 0x0139_8af0;
pub(super) const ADDR_SET_VERSION: usize = 0x0047_9e90;

pub(crate) const TARGET_READ: &str = "client.cooked.version_read";
pub(crate) const TARGET_SET: &str = "client.cooked.version_set";
pub(crate) const TARGET_HELD: &str = "client.cooked.versions_held";

/// Field offsets, each read off the QA `SGW.exe` on 2026-10-04.
pub(crate) mod layout {
    /// `ZipStorageBase`: `std::vector<std::wstring>` of PAK names, begin.
    pub const NAMES_BEGIN: u32 = 0x04;
    /// The same vector's end pointer.
    pub const NAMES_END: u32 = 0x08;
    /// `ZipStorageBase`: the category's version, as sent in
    /// `versionInfoRequest`.
    pub const VERSION: u32 = 0x24;

    /// `CZipMemFile`: bytes held. Read entry (`0x00478e79`) tests it and
    /// passes it to the stream write as the count.
    pub const MEMORY_FILE_LENGTH: u32 = 0x10;
    /// `CZipMemFile`: the buffer (`0x00478e81`, the write's source).
    pub const MEMORY_FILE_BUFFER: u32 = 0x14;

    /// `CZipArchive`: the central directory's vector of `CZipFileHeader*`,
    /// begin and end (`0x0139747d`: `headers[index]`, bounds-checked).
    pub const ARCHIVE_HEADERS_BEGIN: u32 = 0xbc;
    pub const ARCHIVE_HEADERS_END: u32 = 0xc0;
    /// `CZipFileHeader`: compression method, a `u16` (`0x01396a22`).
    pub const HEADER_METHOD: u32 = 0x0a;
    /// `CZipFileHeader`: CRC-32, then compressed and uncompressed sizes
    /// (`0x01398b81` hands `+0x18` to the progress callback as the total).
    pub const HEADER_CRC32: u32 = 0x10;
    pub const HEADER_COMPRESSED: u32 = 0x14;
    pub const HEADER_UNCOMPRESSED: u32 = 0x18;

    /// `basic_istream<char>`: the count of the last unformatted read
    /// (`gcount`). Its first word is the vbtable pointer, whose second entry
    /// is the offset from the stream to its `basic_ios`.
    pub const ISTREAM_COUNT: u32 = 0x04;
    /// `ios_base`: the state bits (eof 1, fail 2, bad 4).
    pub const IOS_STATE: u32 = 0x08;
    /// `basic_ios<char>`: the stream buffer.
    pub const IOS_STREAMBUF: u32 = 0x28;
    /// `basic_streambuf<char>`: pointers to the put area's first and next
    /// pointers. The difference of what they point at is what was written.
    pub const STREAMBUF_PUT_FIRST: u32 = 0x14;
    pub const STREAMBUF_PUT_NEXT: u32 = 0x24;
}

/// `CZipArchive::FindFile`'s "no such entry".
pub(crate) const NOT_FOUND: u16 = 0xffff;

/// What an extraction left in the memory file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Extracted {
    pub len: u32,
    /// The first four bytes, little-endian: the version, for `MetaData`.
    pub head: Option<u32>,
}

/// The archive's central-directory record of one entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ZipEntry {
    pub method: u16,
    pub crc32: u32,
    pub compressed: u32,
    pub uncompressed: u32,
}

/// One `basic_istream<char>::read` on the stream the entry was written to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StreamRead {
    /// Bytes in the stream's put area before the read: what read entry wrote.
    pub held: Option<u32>,
    pub requested: u32,
    /// `gcount` after the read.
    pub count: u32,
    /// The stream's state bits after the read.
    pub state: u32,
}

/// The zip library's C-runtime file calls under one version read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CrtIo {
    pub reads: u32,
    pub read_bytes: u32,
    /// Reads that failed or returned fewer bytes than asked for.
    pub short_reads: u32,
    pub seeks: u32,
    pub failed_seeks: u32,
}

/// What the calls under one version read did. `None` means the call was not
/// seen: it did not happen, or its hook is not installed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ReadProbe {
    pub find_index: Option<u16>,
    pub extract_ok: Option<bool>,
    pub entry_ok: Option<bool>,
    pub zip: Option<ZipEntry>,
    pub extracted: Option<Extracted>,
    pub stream: Option<StreamRead>,
    pub crt: Option<CrtIo>,
}

/// The memory file's length and first four bytes.
pub(crate) fn extracted(mem: &impl Mem, memory_file: u32) -> Option<Extracted> {
    let len = mem.u32_at(memory_file.wrapping_add(layout::MEMORY_FILE_LENGTH))?;
    let buffer = mem.u32_at(memory_file.wrapping_add(layout::MEMORY_FILE_BUFFER))?;
    let head = (len >= 4 && buffer != 0)
        .then(|| mem.u32_at(buffer))
        .flatten();
    Some(Extracted { len, head })
}

/// The directory record `CZipArchive` holds for entry `index`.
pub(crate) fn zip_entry(mem: &impl Mem, archive: u32, index: u32) -> Option<ZipEntry> {
    let begin = mem.u32_at(archive.wrapping_add(layout::ARCHIVE_HEADERS_BEGIN))?;
    let end = mem.u32_at(archive.wrapping_add(layout::ARCHIVE_HEADERS_END))?;
    if begin == 0 || end <= begin || index >= (end - begin) / 4 {
        return None;
    }
    let header = mem.u32_at(begin + index * 4).filter(|h| *h != 0)?;
    // The method is the high half of the word at `+0x08`.
    let method = mem.u32_at(header.wrapping_add(layout::HEADER_METHOD - 2))? >> 16;
    Some(ZipEntry {
        method: method as u16,
        crc32: mem.u32_at(header.wrapping_add(layout::HEADER_CRC32))?,
        compressed: mem.u32_at(header.wrapping_add(layout::HEADER_COMPRESSED))?,
        uncompressed: mem.u32_at(header.wrapping_add(layout::HEADER_UNCOMPRESSED))?,
    })
}

/// The `basic_ios` of a `basic_istream<char>`, through its vbtable.
fn ios_of(mem: &impl Mem, istream: u32) -> Option<u32> {
    let vbtable = mem.u32_at(istream)?;
    // Past any real stream object: this is not a stream.
    let offset = mem
        .u32_at(vbtable.wrapping_add(4))
        .filter(|o| *o < 0x1000)?;
    Some(istream.wrapping_add(offset))
}

/// Bytes sitting in the put area of the stream's buffer.
pub(crate) fn stream_held(mem: &impl Mem, istream: u32) -> Option<u32> {
    let ios = ios_of(mem, istream)?;
    let buffer = mem.u32_at(ios.wrapping_add(layout::IOS_STREAMBUF))?;
    let first = mem.u32_at(mem.u32_at(buffer.wrapping_add(layout::STREAMBUF_PUT_FIRST))?)?;
    let next = mem.u32_at(mem.u32_at(buffer.wrapping_add(layout::STREAMBUF_PUT_NEXT))?)?;
    if first == 0 {
        return Some(0);
    }
    next.checked_sub(first)
}

/// `gcount` and the state bits of a stream, after a read.
pub(crate) fn stream_read(
    mem: &impl Mem,
    istream: u32,
    requested: u32,
    held: Option<u32>,
) -> Option<StreamRead> {
    let count = mem.u32_at(istream.wrapping_add(layout::ISTREAM_COUNT))?;
    let state = mem.u32_at(ios_of(mem, istream)?.wrapping_add(layout::IOS_STATE))?;
    Some(StreamRead {
        held,
        requested,
        count,
        state,
    })
}

/// `ios_base` state bits by name: `good`, or `eof|fail`.
pub(crate) fn state_name(state: u32) -> String {
    let names: Vec<&str> = [(1, "eof"), (2, "fail"), (4, "bad")]
        .into_iter()
        .filter(|(bit, _)| state & bit != 0)
        .map(|(_, name)| name)
        .collect();
    if names.is_empty() {
        "good".into()
    } else {
        names.join("|")
    }
}

/// A zip compression method by name.
pub(crate) fn method_name(method: u16) -> &'static str {
    match method {
        0 => "stored",
        8 => "deflated",
        _ => "other",
    }
}

/// A version as the cache protocol uses it. The server stamps the bitwise
/// NOT of its version while a resync is in flight, which has the top bit set
/// for every real version.
pub(crate) fn version_kind(version: u32) -> &'static str {
    match version {
        0 => "zero",
        v if v >= 0x8000_0000 => "resync_pending",
        _ => "real",
    }
}

/// Why a version read ended as it did.
pub(crate) fn read_outcome(probe: ReadProbe) -> &'static str {
    match (probe.find_index, probe.extract_ok, probe.entry_ok) {
        // The entry reached the stream and the stream did not give it back:
        // `out` keeps what it had, and the game is told nothing.
        (_, _, Some(true)) if probe.stream.is_some_and(|s| s.count < s.requested) => {
            "stream_read_short"
        }
        (_, _, Some(true)) => "read",
        (Some(NOT_FOUND), _, _) => "metadata_entry_not_found",
        (Some(_), Some(false), _) => "metadata_extract_failed",
        // Found and extracted, and still refused: the memory file was empty.
        (Some(_), Some(true), Some(false)) => "metadata_empty",
        (None, None, None) => "entry_read_not_attempted",
        _ => "entry_read_failed",
    }
}

/// `info` for a version that was read, `warn` for one that was not: the
/// client keeps whatever it had, usually 0, and says nothing.
pub(crate) fn read_level(probe: ReadProbe) -> &'static str {
    if read_outcome(probe) == "read" {
        "info"
    } else {
        "warn"
    }
}

/// The fields of one `client.cooked.version_read`.
pub(crate) fn version_read_fields(
    pak: Option<&str>,
    before: Option<u32>,
    after: Option<u32>,
    probe: ReadProbe,
) -> Fields {
    let mut f: Fields = vec![
        ("pak", json!(pak)),
        ("outcome", json!(read_outcome(probe))),
        ("version", json!(after)),
        ("version_kind", json!(after.map(version_kind))),
    ];
    if before != after {
        f.push(("previous", json!(before)));
    }
    if let Some(index) = probe.find_index.filter(|i| *i != NOT_FOUND) {
        f.push(("entry_index", json!(index)));
    }
    if let Some(zip) = probe.zip {
        f.push(("zip_method", json!(method_name(zip.method))));
        f.push(("zip_method_id", json!(zip.method)));
        f.push(("zip_compressed", json!(zip.compressed)));
        f.push(("zip_uncompressed", json!(zip.uncompressed)));
        f.push(("zip_crc32", json!(format!("{:08x}", zip.crc32))));
    }
    if let Some(ok) = probe.extract_ok {
        f.push(("extract_ok", json!(ok)));
    }
    if let Some(extracted) = probe.extracted {
        f.push(("extract_len", json!(extracted.len)));
        if let Some(head) = extracted.head {
            // The bytes in file order, and the version they spell.
            let bytes = head.to_le_bytes().map(|b| format!("{b:02x}")).join(" ");
            f.push(("extract_bytes", json!(bytes)));
            f.push(("extract_version", json!(head)));
        }
    }
    if let Some(stream) = probe.stream {
        f.push(("stream_held", json!(stream.held)));
        f.push(("stream_read_requested", json!(stream.requested)));
        f.push(("stream_read_count", json!(stream.count)));
        f.push(("stream_state", json!(state_name(stream.state))));
    }
    if let Some(crt) = probe.crt {
        f.push(("crt_reads", json!(crt.reads)));
        f.push(("crt_read_bytes", json!(crt.read_bytes)));
        f.push(("crt_short_reads", json!(crt.short_reads)));
        f.push(("crt_seeks", json!(crt.seeks)));
        f.push(("crt_failed_seeks", json!(crt.failed_seeks)));
    }
    f
}

/// The fields of one `client.cooked.version_set`.
pub(crate) fn version_set_fields(pak: Option<&str>, previous: Option<u32>, version: u32) -> Fields {
    vec![
        ("pak", json!(pak)),
        ("version", json!(version)),
        ("version_kind", json!(version_kind(version))),
        ("previous", json!(previous)),
    ]
}

/// The fields of one `client.cooked.versions_held`: every storage's version
/// when the client asks the server, and how many of them are 0.
pub(crate) fn versions_held_fields(held: &[(String, Option<u32>)]) -> Fields {
    let count = |kind: &str| {
        held.iter()
            .filter(|(_, v)| v.map(version_kind) == Some(kind))
            .count()
    };
    let versions: serde_json::Map<String, serde_json::Value> = held
        .iter()
        .map(|(pak, version)| (pak.clone(), json!(version)))
        .collect();
    vec![
        ("storages", json!(held.len())),
        ("zero", json!(count("zero"))),
        ("resync_pending", json!(count("resync_pending"))),
        ("real", json!(count("real"))),
        ("versions", serde_json::Value::Object(versions)),
    ]
}
