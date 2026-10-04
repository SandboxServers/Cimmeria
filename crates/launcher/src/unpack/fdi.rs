//! Expand a chain of cabinets with Windows' File Decompression Interface
//! (`FDICreate` / `FDICopy` in `cabinet.dll`).
//!
//! FDI handles MSZIP, LZX and Quantum, and follows a file that continues
//! from one cabinet into the next. Its callbacks carry no user pointer, so
//! the state they need lives in a thread-local [`Ctx`] for the length of one
//! [`expand_chain`] call; FDI runs every callback on the calling thread.
//!
//! File handles given to FDI are `Box<File>` pointers cast to `isize`.
//! Cabinet names reach [`fdi_open`] as bare names (the cabinet path passed
//! to `FDICopy` is empty) and are resolved against the cabinet directory
//! here, so an install path outside the ANSI code page still works.
//!
//! Callbacks never panic: a panic cannot unwind across the FFI boundary.
//! Errors are parked in [`Ctx::error`] and the callback returns FDI's
//! failure value, which makes `FDICopy` return `FALSE`.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::ffi::{c_char, c_void, CStr, CString};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use windows_sys::core::PCSTR;
use windows_sys::Win32::Storage::Cabinets::{
    cpuUNKNOWN, fdintCLOSE_FILE_INFO as CLOSE_FILE_INFO, fdintCOPY_FILE as COPY_FILE,
    fdintNEXT_CABINET as NEXT_CABINET, FDICopy, FDICreate, FDIDestroy, ERF, FDINOTIFICATION,
    FDINOTIFICATIONTYPE,
};
use windows_sys::Win32::System::Memory::{GetProcessHeap, HeapAlloc, HeapFree};

use super::{dos_time, entry_inventory::EntryInventory, safe_relative, UnpackError, UnpackSink};

/// `_A_NAME_IS_UTF`: the entry name in `psz1` is UTF-8, not the ANSI
/// code page.
const A_NAME_IS_UTF: u16 = 0x80;

struct Ctx {
    cab_dir: PathBuf,
    dest: PathBuf,
    sink: UnpackSink,
    total: usize,
    done: usize,
    current: Option<PathBuf>,
    error: Option<UnpackError>,
    preflight: bool,
    inventory: EntryInventory,
    cabinets: BTreeSet<String>,
}

thread_local! {
    static CTX: RefCell<Option<Ctx>> = const { RefCell::new(None) };
}

/// Expand `cabinets` (bare file names in `cab_dir`, in chain order) into
/// `dest`. `total` is the file count from the set's INF; fewer files than
/// that is reported as an error rather than a partial install.
pub(super) fn expand_chain(
    cab_dir: &Path,
    cabinets: &[String],
    dest: &Path,
    total: usize,
    sink: &UnpackSink,
) -> Result<(), UnpackError> {
    // Keep every source cabinet immutable across enumeration and expansion.
    // FDI reopens them by name; these handles permit reads but not replacement.
    use std::os::windows::fs::OpenOptionsExt;
    let mut cabinet_names = EntryInventory::default();
    let mut sources = Vec::new();
    for name in cabinets {
        sink.check_cancel()?;
        if name.contains(['/', '\\']) {
            return Err(UnpackError::UnsafePath(name.clone()));
        }
        cabinet_names.insert(name, false)?;
        sources.push(
            std::fs::OpenOptions::new()
                .read(true)
                .share_mode(windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ)
                .open(cab_dir.join(name))
                .map_err(|error| UnpackError::Cab(format!("cannot open {name}: {error}")))?,
        );
    }
    run_chain(cab_dir, cabinets, dest, total, sink, true)?;
    let result = run_chain(cab_dir, cabinets, dest, total, sink, false);
    drop(sources);
    result
}

fn run_chain(
    cab_dir: &Path,
    cabinets: &[String],
    dest: &Path,
    total: usize,
    sink: &UnpackSink,
    preflight: bool,
) -> Result<(), UnpackError> {
    if !preflight {
        std::fs::create_dir_all(dest)?;
    }
    CTX.with(|c| {
        *c.borrow_mut() = Some(Ctx {
            cab_dir: cab_dir.to_path_buf(),
            dest: dest.to_path_buf(),
            sink: sink.clone(),
            total,
            done: 0,
            current: None,
            error: None,
            preflight,
            inventory: EntryInventory::default(),
            cabinets: cabinets.iter().map(|name| name.to_lowercase()).collect(),
        })
    });
    struct ClearCtx;
    impl Drop for ClearCtx {
        fn drop(&mut self) {
            CTX.with(|c| *c.borrow_mut() = None);
        }
    }
    let _clear = ClearCtx;

    let mut erf = ERF::default();
    // SAFETY: every callback matches the FDI signature, and `erf` outlives
    // the handle (the `Destroy` guard below drops first).
    let hfdi = unsafe {
        FDICreate(
            Some(fdi_alloc),
            Some(fdi_free),
            Some(fdi_open),
            Some(fdi_read),
            Some(fdi_write),
            Some(fdi_close),
            Some(fdi_seek),
            cpuUNKNOWN,
            &mut erf,
        )
    };
    if hfdi.is_null() {
        return Err(UnpackError::Cab(format!(
            "FDICreate failed: {}",
            fdi_error_text(erf.erfOper)
        )));
    }
    struct Destroy(*mut c_void);
    impl Drop for Destroy {
        fn drop(&mut self) {
            // SAFETY: the handle came from FDICreate and is destroyed once.
            unsafe { FDIDestroy(self.0) };
        }
    }
    let _destroy = Destroy(hfdi);

    let no_path = CString::default();
    for name in cabinets {
        sink.check_cancel()?;
        let cname = CString::new(name.as_str())
            .map_err(|_| UnpackError::Cab(format!("cabinet name {name:?} has a NUL byte")))?;
        // SAFETY: both strings are NUL-terminated and live across the call;
        // the notify callback matches PFNFDINOTIFY.
        let ok = unsafe {
            FDICopy(
                hfdi,
                cname.as_ptr() as PCSTR,
                no_path.as_ptr() as PCSTR,
                0,
                Some(fdi_notify),
                None,
                std::ptr::null(),
            )
        };
        if ok == 0 {
            let parked = CTX.with(|c| c.borrow_mut().as_mut().and_then(|ctx| ctx.error.take()));
            return Err(parked.unwrap_or_else(|| {
                UnpackError::Cab(format!("{name}: {}", fdi_error_text(erf.erfOper)))
            }));
        }
    }

    let done = CTX.with(|c| c.borrow().as_ref().map_or(0, |ctx| ctx.done));
    if done != total {
        return Err(UnpackError::Cab(format!(
            "the cabinets held {done} files but their index lists {total}"
        )));
    }
    Ok(())
}

fn fdi_error_text(code: i32) -> String {
    let what = match code {
        1 => "cabinet not found",
        2 => "not a cabinet",
        3 => "unknown cabinet version",
        4 => "corrupt cabinet",
        5 => "out of memory",
        6 => "unknown compression type",
        7 => "decompression failed",
        8 => "could not write a target file",
        9 => "reserve size mismatch",
        10 => "cabinet is not the next one in the set",
        11 => "cancelled",
        12 => "unexpected end of cabinet",
        _ => "unknown error",
    };
    format!("{what} (FDI error {code})")
}

fn into_handle(f: File) -> isize {
    Box::into_raw(Box::new(f)) as isize
}

/// # Safety
/// `hf` must be a live handle made by [`into_handle`].
unsafe fn file_mut<'a>(hf: isize) -> &'a mut File {
    unsafe { &mut *(hf as *mut File) }
}

/// # Safety
/// `psz` must be a valid NUL-terminated string.
unsafe fn cstr_bytes<'a>(psz: *const u8) -> &'a [u8] {
    unsafe { CStr::from_ptr(psz as *const c_char) }.to_bytes()
}

unsafe extern "system" fn fdi_alloc(cb: u32) -> *mut c_void {
    // SAFETY: plain process-heap allocation; FDI frees it with fdi_free.
    unsafe { HeapAlloc(GetProcessHeap(), 0, cb as usize) }
}

unsafe extern "system" fn fdi_free(pv: *const c_void) {
    // SAFETY: `pv` came from fdi_alloc.
    unsafe { HeapFree(GetProcessHeap(), 0, pv) };
}

/// FDI opens only cabinets through this callback, read-only.
unsafe extern "system" fn fdi_open(psz: PCSTR, _oflag: i32, _pmode: i32) -> isize {
    // SAFETY: FDI passes a NUL-terminated name.
    let name = String::from_utf8_lossy(unsafe { cstr_bytes(psz) }).into_owned();
    let path = CTX.with(|c| {
        let mut guard = c.borrow_mut();
        let ctx = guard.as_mut()?;
        if name.contains(['/', '\\', ':']) || !ctx.cabinets.contains(&name.to_lowercase()) {
            ctx.error = Some(UnpackError::UnsafePath(name.clone()));
            return None;
        }
        Some(ctx.cab_dir.join(&name))
    });
    match path.map(File::open) {
        Some(Ok(f)) => into_handle(f),
        _ => -1,
    }
}

unsafe extern "system" fn fdi_read(hf: isize, pv: *mut c_void, cb: u32) -> u32 {
    // SAFETY: FDI hands back a handle we made and a buffer of `cb` bytes.
    let (f, buf) = unsafe {
        (
            file_mut(hf),
            std::slice::from_raw_parts_mut(pv as *mut u8, cb as usize),
        )
    };
    let mut n = 0;
    while n < buf.len() {
        match f.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(k) => n += k,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return u32::MAX,
        }
    }
    n as u32
}

unsafe extern "system" fn fdi_write(hf: isize, pv: *const c_void, cb: u32) -> u32 {
    // SAFETY: as fdi_read.
    let (f, buf) = unsafe {
        (
            file_mut(hf),
            std::slice::from_raw_parts(pv as *const u8, cb as usize),
        )
    };
    match f.write_all(buf) {
        Ok(()) => cb,
        Err(e) => {
            CTX.with(|c| {
                if let Some(ctx) = c.borrow_mut().as_mut() {
                    ctx.error.get_or_insert(UnpackError::Io(e));
                }
            });
            u32::MAX
        }
    }
}

unsafe extern "system" fn fdi_close(hf: isize) -> i32 {
    // SAFETY: FDI closes each handle it was given exactly once.
    drop(unsafe { Box::from_raw(hf as *mut File) });
    0
}

unsafe extern "system" fn fdi_seek(hf: isize, dist: i32, seektype: i32) -> i32 {
    let pos = match seektype {
        0 => SeekFrom::Start(dist.max(0) as u64),
        1 => SeekFrom::Current(i64::from(dist)),
        2 => SeekFrom::End(i64::from(dist)),
        _ => return -1,
    };
    // SAFETY: a handle we made.
    match unsafe { file_mut(hf) }.seek(pos) {
        Ok(p) => i32::try_from(p).unwrap_or(-1),
        Err(_) => -1,
    }
}

unsafe extern "system" fn fdi_notify(
    fdint: FDINOTIFICATIONTYPE,
    pfdin: *mut FDINOTIFICATION,
) -> isize {
    // SAFETY: FDI passes a valid notification for the duration of the call.
    let n = unsafe { &*pfdin };
    CTX.with(|c| {
        let mut guard = c.borrow_mut();
        let Some(ctx) = guard.as_mut() else {
            return -1;
        };
        match fdint {
            COPY_FILE => copy_file(ctx, n),
            CLOSE_FILE_INFO => {
                // SAFETY: the handle copy_file returned for this file.
                let file = unsafe { Box::from_raw(n.hf as *mut File) };
                let path = ctx.current.take();
                // Stamp the cabinet's date/time before closing, as the
                // stock installer and expand.exe do. UE3 compares the
                // Default*.ini mtimes with the ones recorded in the
                // player's generated SGW*.ini and asks to regenerate on a
                // mismatch (see dos_time).
                dos_time::apply(
                    &file,
                    n.date,
                    n.time,
                    path.as_deref().unwrap_or(Path::new("")),
                );
                drop(file);
                ctx.done += 1;
                if let Some(path) = path {
                    ctx.sink
                        .report("expanding cabinets", ctx.done, ctx.total, &path);
                }
                1
            }
            // FDI reopens the next cabinet by name through fdi_open. A
            // non-zero fdie means that attempt already failed; stop rather
            // than loop.
            NEXT_CABINET if n.fdie != 0 => -1,
            // Cabinet info, partial files (continued from an earlier
            // cabinet, already written) and enumeration need nothing.
            _ => 0,
        }
    })
}

fn copy_file(ctx: &mut Ctx, n: &FDINOTIFICATION) -> isize {
    if ctx.sink.cancel.is_cancelled() {
        ctx.error = Some(UnpackError::Cancelled);
        return -1;
    }
    // SAFETY: psz1 is the NUL-terminated entry name.
    let raw = unsafe { cstr_bytes(n.psz1) };
    let name = if n.attribs & A_NAME_IS_UTF != 0 {
        let Ok(name) = std::str::from_utf8(raw) else {
            ctx.error = Some(UnpackError::Cab("entry name is not valid UTF-8".into()));
            return -1;
        };
        name.to_owned()
    } else {
        // ANSI; the client's names are ASCII, and Latin-1 is the closest
        // total mapping for anything else.
        raw.iter().map(|&b| b as char).collect()
    };
    if ctx.preflight {
        if let Err(error) = ctx.inventory.insert(&name, false) {
            ctx.error = Some(error);
            return -1;
        }
        // COPY_FILE reports starts, not continued fragments. Returning zero
        // enumerates without creating a file; each cabinet is visited in turn.
        ctx.done += 1;
        return 0;
    }
    let Some(rel) = safe_relative(&name) else {
        ctx.error = Some(UnpackError::UnsafePath(name));
        return -1;
    };
    let out = ctx.dest.join(rel);
    let created = out
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| File::create(&out));
    match created {
        Ok(f) => {
            ctx.current = Some(out);
            into_handle(f)
        }
        Err(e) => {
            ctx.error = Some(UnpackError::Io(e));
            -1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_fixtures::{
        fixture_mtime, incompressible as payload, make_cab_set, sink,
    };
    use super::*;

    #[test]
    fn rejects_late_case_collision_before_creating_output() {
        let dir = tempfile::tempdir().unwrap();
        let files = vec![("a.txt", b"a".to_vec()), ("b.txt", b"b".to_vec())];
        let names = make_cab_set(dir.path(), &files, 1_000_000);
        assert_eq!(names.len(), 1);
        let path = dir.path().join(&names[0]);
        let mut bytes = std::fs::read(&path).unwrap();
        let offsets: Vec<_> = bytes
            .windows(6)
            .enumerate()
            .filter_map(|(i, value)| (value == b"b.txt\0").then_some(i))
            .collect();
        assert_eq!(offsets.len(), 1);
        bytes[offsets[0]] = b'A';
        std::fs::write(&path, bytes).unwrap();
        let (sink, _rx) = sink();
        let out = dir.path().join("out");
        assert!(matches!(
            expand_chain(dir.path(), &names, &out, 2, &sink),
            Err(UnpackError::EntryConflict(_))
        ));
        assert!(!out.exists());
    }

    #[test]
    fn rejects_conflicts_across_cabinets_before_creating_output() {
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let mut names = make_cab_set(dir.path(), &[("a.txt", b"a".to_vec())], 1_000_000);
        let second = make_cab_set(other.path(), &[("A.txt", b"b".to_vec())], 1_000_000);
        assert_eq!(names.len(), 1);
        assert_eq!(second.len(), 1);
        std::fs::copy(other.path().join(&second[0]), dir.path().join("OTHER.CAB")).unwrap();
        names.push("OTHER.CAB".into());
        let (sink, _rx) = sink();
        let out = dir.path().join("out");
        assert!(matches!(
            expand_chain(dir.path(), &names, &out, 2, &sink),
            Err(UnpackError::EntryConflict(_))
        ));
        assert!(!out.exists());
    }

    #[test]
    fn rejects_index_count_mismatch_before_creating_output() {
        let dir = tempfile::tempdir().unwrap();
        let names = make_cab_set(dir.path(), &[("a.txt", b"a".to_vec())], 1_000_000);
        let (sink, _rx) = sink();
        let out = dir.path().join("out");
        assert!(matches!(
            expand_chain(dir.path(), &names, &out, 0, &sink),
            Err(UnpackError::Cab(_))
        ));
        assert!(!out.exists());
    }

    // The bug shape this guards: a file that starts in DATA1.CAB and ends
    // in DATA2.CAB must come out whole, and nothing may be written twice
    // or dropped at the boundary.
    #[test]
    fn expands_a_spanning_set_into_the_installed_layout() {
        let dir = tempfile::tempdir().unwrap();
        let cabs = dir.path().join("Data");
        std::fs::create_dir_all(&cabs).unwrap();
        let files = vec![
            ("Working\\binaries\\SGW.exe", payload(1, 150_000)),
            ("Working\\SGWGame\\CookedPC\\a.upk", payload(2, 90_000)),
            ("Common\\res\\x.def", b"<root/>".to_vec()),
        ];
        let names = make_cab_set(&cabs, &files, 100_000);
        assert!(names.len() >= 2, "set must span cabinets, got {names:?}");

        let dest = dir.path().join("install");
        let (sink, _rx) = sink();
        expand_chain(&cabs, &names, &dest, files.len(), &sink).unwrap();
        for (name, data) in &files {
            let p = dest.join(name.replace('\\', "/"));
            assert_eq!(&std::fs::read(&p).unwrap(), data, "{name}");
        }
    }

    // Bug shape: expanded files kept the extraction time instead of the
    // cabinet's 2009-06-30 stamp, so UE3 saw every Default*.ini as newer
    // than the one recorded in the player's SGW*.ini and showed the "ini
    // file is outdated" dialog on launch. Covers a file continued across a
    // cabinet boundary too: its stamp arrives with the last piece.
    #[test]
    fn expanded_files_keep_the_cabinet_date_and_time() {
        let dir = tempfile::tempdir().unwrap();
        let cabs = dir.path().join("Data");
        std::fs::create_dir_all(&cabs).unwrap();
        let files = vec![
            (
                "Working\\SGWGame\\Config\\DefaultEditor.ini",
                b"[Editor]\r\n".to_vec(),
            ),
            ("Working\\binaries\\SGW.exe", payload(4, 150_000)),
        ];
        let names = make_cab_set(&cabs, &files, 100_000);
        assert!(names.len() >= 2, "set must span cabinets, got {names:?}");

        let dest = dir.path().join("install");
        let (sink, _rx) = sink();
        expand_chain(&cabs, &names, &dest, files.len(), &sink).unwrap();
        for (name, _) in &files {
            let p = dest.join(name.replace('\\', "/"));
            let modified = std::fs::metadata(&p).unwrap().modified().unwrap();
            assert_eq!(modified, fixture_mtime(), "{name}");
        }
    }

    #[test]
    fn a_missing_middle_cabinet_fails() {
        let dir = tempfile::tempdir().unwrap();
        let files = vec![("big.bin", payload(3, 250_000))];
        let names = make_cab_set(dir.path(), &files, 100_000);
        assert!(names.len() >= 3, "{names:?}");
        std::fs::remove_file(dir.path().join(&names[1])).unwrap();
        let (sink, _rx) = sink();
        let err = expand_chain(dir.path(), &names, &dir.path().join("out"), 1, &sink).unwrap_err();
        assert!(matches!(err, UnpackError::Cab(_)), "{err:?}");
    }

    #[test]
    fn cancel_stops_before_the_first_file() {
        let dir = tempfile::tempdir().unwrap();
        let names = make_cab_set(dir.path(), &[("a.txt", b"a".to_vec())], 1_000_000);
        let (sink, _rx) = sink();
        sink.cancel.cancel();
        let out = dir.path().join("out");
        let err = expand_chain(dir.path(), &names, &out, 1, &sink).unwrap_err();
        assert!(matches!(err, UnpackError::Cancelled), "{err:?}");
        assert!(!out.join("a.txt").exists());
    }
}
