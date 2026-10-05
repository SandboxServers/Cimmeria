//! The imports the cache code calls: the `MSVCP80.dll` stream read that ends
//! a version read, and the `MSVCR80.dll` file calls of the zip library under
//! it.
//!
//! | Import | IAT slot | Callers in `SGW.exe` |
//! |---|---|---|
//! | `basic_istream<char>::read` (`MSVCP80.dll`) | `0x017ef6d0` | 72, among them the version read (`0x00478f60`) and the entry deserialisers |
//! | `_wsopen_s` (`MSVCR80.dll`) | `0x017ef954` | one: the zip library's file open (`0x0139d6bd`) |
//! | `_read` (`MSVCR80.dll`) | `0x017ef8b4` | three, one in the zip library (`0x01395ba4`) |
//! | `_lseek` (`MSVCR80.dll`) | `0x017ef960` | two, both in the zip library |
//!
//! Slots and call sites are from the QA `SGW.exe`'s import directory
//! (2026-10-04). The zip library reaches the file system through the C
//! runtime, never through the `CreateFileW` import that
//! [`file_io`](crate::hooks::seams::file_io) watches, which is why that hook
//! has never seen a cache archive opened.
//!
//! The stream read and the two file calls only record while a version read
//! is running on the thread (one thread-local test otherwise), into that
//! read's `client.cooked.version_read`. The open reports every `.pak` as
//! `client.io.pak_open` with `via: "crt"`, opened or not.

use serde_json::json;

use super::CrtIo;
use crate::hooks::entity_trace::Fields;

/// IAT slot of `basic_istream<char>::read`.
pub(crate) const IAT_ISTREAM_READ: usize = 0x017e_f6d0;
/// IAT slot of `_wsopen_s`.
pub(crate) const IAT_WSOPEN_S: usize = 0x017e_f954;
/// IAT slot of `_read`.
pub(crate) const IAT_READ: usize = 0x017e_f8b4;
/// IAT slot of `_lseek`.
pub(crate) const IAT_LSEEK: usize = 0x017e_f960;

/// `_O_CREAT`, `_O_TRUNC`, `_O_EXCL`, `_O_BINARY`.
const O_CREAT: i32 = 0x0100;
const O_TRUNC: i32 = 0x0200;
const O_EXCL: i32 = 0x0400;
const O_BINARY: i32 = 0x8000;

/// Whether an `_open` flag set asks to write.
pub(crate) fn crt_wants_write(oflag: i32) -> bool {
    oflag & 3 != 0
}

/// An `_open` flag set as the `CreateFile` disposition it becomes, in the
/// words `client.io.pak_open` already uses.
pub(crate) fn crt_disposition(oflag: i32) -> &'static str {
    match (
        oflag & O_CREAT != 0,
        oflag & O_TRUNC != 0,
        oflag & O_EXCL != 0,
    ) {
        (true, _, true) => "create_new",
        (true, true, false) => "create_always",
        (true, false, false) => "open_always",
        (false, true, _) => "truncate_existing",
        (false, false, _) => "open_existing",
    }
}

/// An `_SH_*` share flag by name.
pub(crate) fn crt_share(shflag: i32) -> &'static str {
    match shflag {
        0x10 => "deny_read_write",
        0x20 => "deny_write",
        0x30 => "deny_read",
        0x40 => "deny_none",
        _ => "other",
    }
}

/// The `errno` values an open is likely to fail with.
pub(crate) fn errno_name(error: i32) -> &'static str {
    match error {
        2 => "no_such_file",
        13 => "access_denied",
        17 => "already_exists",
        22 => "invalid_argument",
        24 => "too_many_open_files",
        _ => "other",
    }
}

/// `info` for an archive that opened, `warn` for one that did not.
pub(crate) fn crt_open_level(error: i32) -> &'static str {
    if error == 0 {
        "info"
    } else {
        "warn"
    }
}

/// The fields of one `client.io.pak_open` from the C runtime's open.
pub(crate) fn crt_open_fields(
    path: &str,
    oflag: i32,
    shflag: i32,
    error: i32,
    fd: Option<i32>,
    suppressed: u64,
) -> Fields {
    let mut f: Fields = vec![
        ("path", json!(path)),
        ("via", json!("crt")),
        ("opened", json!(error == 0)),
        ("write", json!(crt_wants_write(oflag))),
        ("disposition", json!(crt_disposition(oflag))),
        ("share_mode", json!(crt_share(shflag))),
        // Text mode would translate line ends in a zip's bytes.
        ("binary", json!(oflag & O_BINARY != 0)),
        ("oflag", json!(oflag)),
    ];
    if error != 0 {
        f.push(("error", json!(error)));
        f.push(("error_name", json!(errno_name(error))));
    }
    if let Some(fd) = fd {
        f.push(("fd", json!(fd)));
    }
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    f
}

/// Count one `_read(fd, buffer, requested)` that returned `result`.
pub(crate) fn count_read(io: &mut CrtIo, requested: u32, result: i32) {
    io.reads += 1;
    match u32::try_from(result) {
        Ok(read) => {
            io.read_bytes = io.read_bytes.saturating_add(read);
            if read < requested {
                io.short_reads += 1;
            }
        }
        Err(_) => io.short_reads += 1,
    }
}

/// Count one `_lseek` that returned `result` (`-1` is failure).
pub(crate) fn count_seek(io: &mut CrtIo, result: i32) {
    io.seeks += 1;
    if result == -1 {
        io.failed_seeks += 1;
    }
}
