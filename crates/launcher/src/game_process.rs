//! Is the game running from this install? A process probe that does not
//! depend on the launcher having started the game.
//!
//! The launcher follows the `SGW.exe` it starts itself (see
//! [`crate::worker`]'s `GameExited`), but the game can also be running
//! when the launcher never saw it start: a launcher window reopened while
//! the game plays, an Atera debug launch (the bat starts `SGW.exe`), or a
//! launch whose process could not be opened to follow. Install, path
//! changes, and any later repair or uninstall must not touch the files
//! under a running game, so they ask this probe as well.
//!
//! The probe is conservative: an `SGW.exe` whose image path cannot be
//! read (another user's process, say) counts as running from this
//! install. A false "running" only delays a file operation; a false
//! "not running" could rewrite files under the game.

use std::path::Path;

/// The image name the probe looks for, compared case-insensitively.
pub const GAME_IMAGE_NAME: &str = "SGW.exe";

/// One process the probe saw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSighting {
    pub pid: u32,
    /// The full image path, when the process could be opened to read it.
    pub image: Option<std::path::PathBuf>,
}

/// Pids of `SGW.exe` processes that run from `install_dir` (or whose
/// image path could not be read). Empty on non-Windows hosts.
pub fn running_game_pids(install_dir: &Path) -> Vec<u32> {
    select_game_pids(&platform::sgw_processes(), install_dir)
}

/// The probe's decision, separated from the OS walk so it is testable:
/// keep a sighting whose image is under `install_dir`, or unknown.
pub fn select_game_pids(seen: &[ProcessSighting], install_dir: &Path) -> Vec<u32> {
    let root = normalise(install_dir);
    seen.iter()
        .filter(|s| match &s.image {
            None => true,
            Some(image) => root.is_empty() || normalise(image).starts_with(&root),
        })
        .map(|s| s.pid)
        .collect()
}

/// Lower-case, backslash-separated, no `\\?\` prefix and a trailing
/// separator, so `C:\Games\SGW` does not match `C:\Games\SGW2`.
fn normalise(p: &Path) -> String {
    let s = p.to_string_lossy();
    let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
    let mut s = s.replace('/', "\\").to_lowercase();
    if s.is_empty() {
        return s;
    }
    if !s.ends_with('\\') {
        s.push('\\');
    }
    s
}

#[cfg(windows)]
mod platform {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };

    use super::{ProcessSighting, GAME_IMAGE_NAME};

    /// Closes the handle on drop.
    struct Handle(HANDLE);

    impl Drop for Handle {
        fn drop(&mut self) {
            // SAFETY: the handle came from a successful Win32 open and is
            // closed exactly once.
            unsafe { CloseHandle(self.0) };
        }
    }

    pub(super) fn sgw_processes() -> Vec<ProcessSighting> {
        let mut out = Vec::new();
        // SAFETY: plain Win32 calls; the entry is sized as the API asks
        // and the snapshot handle is closed by `Handle`.
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE || snap.is_null() {
                return out;
            }
            let snap = Handle(snap);
            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            let mut ok = Process32FirstW(snap.0, &mut entry);
            while ok != 0 {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = OsString::from_wide(&entry.szExeFile[..len]);
                if name.to_string_lossy().eq_ignore_ascii_case(GAME_IMAGE_NAME) {
                    out.push(ProcessSighting {
                        pid: entry.th32ProcessID,
                        image: image_path(entry.th32ProcessID),
                    });
                }
                ok = Process32NextW(snap.0, &mut entry);
            }
        }
        out
    }

    fn image_path(pid: u32) -> Option<PathBuf> {
        // SAFETY: OpenProcess returns null on failure; the handle is
        // closed by `Handle`. `buf` holds `len` u16s and the API writes
        // at most that.
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return None;
            }
            let h = Handle(h);
            let mut buf = vec![0u16; 32 * 1024];
            let mut len = buf.len() as u32;
            let ok =
                QueryFullProcessImageNameW(h.0, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len);
            (ok != 0).then(|| PathBuf::from(OsString::from_wide(&buf[..len as usize])))
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::ProcessSighting;

    /// The launcher targets Windows; elsewhere (CI's Linux runners) there
    /// is no `SGW.exe` to find.
    pub(super) fn sgw_processes() -> Vec<ProcessSighting> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn seen(pid: u32, image: Option<&str>) -> ProcessSighting {
        ProcessSighting {
            pid,
            image: image.map(PathBuf::from),
        }
    }

    #[test]
    fn a_game_under_the_install_counts_and_one_elsewhere_does_not() {
        let all = [
            seen(1, Some(r"C:\Games\SGW\Working\Binaries\SGW.exe")),
            seen(2, Some(r"D:\Other\SGW.exe")),
        ];
        assert_eq!(select_game_pids(&all, Path::new(r"C:\Games\SGW")), vec![1]);
    }

    // Bug shape: a prefix match without a separator treats a sibling
    // folder (`SGW2`) as the install, or misses a case-different path.
    #[test]
    fn the_match_is_by_whole_folder_and_ignores_case() {
        let all = [
            seen(1, Some(r"C:\Games\SGW2\SGW.exe")),
            seen(2, Some(r"\\?\c:\games\sgw\working\binaries\sgw.exe")),
        ];
        assert_eq!(select_game_pids(&all, Path::new(r"C:\Games\SGW\")), vec![2]);
    }

    // Conservative: a process the launcher cannot inspect may be ours.
    #[test]
    fn an_unreadable_image_counts_as_running() {
        let all = [seen(9, None)];
        assert_eq!(select_game_pids(&all, Path::new(r"C:\Games\SGW")), vec![9]);
    }

    #[test]
    fn with_no_install_path_every_game_counts() {
        let all = [seen(1, Some(r"D:\x\SGW.exe"))];
        assert_eq!(select_game_pids(&all, Path::new("")), vec![1]);
    }
}
