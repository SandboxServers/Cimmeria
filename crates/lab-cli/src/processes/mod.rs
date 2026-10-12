//! The one place that lists running processes and decides which is a game
//! client. `lab clients`, `lab doctor`, the `lab uat` and `lab calibrate`
//! pre-flights and `lab watch` all ask here, so they agree on what `SGW.exe`
//! is: an image whose file name is exactly `SGW.exe`, in any case.

/// The environment variable that replaces the `SGW.exe` list in tests: a
/// comma-separated list of pids, which may be empty. Honoured only while
/// `CIMMERIA_LAB_HOME` is set (the tests' throw-away lab home), so a real
/// shell cannot hide a running client from the safety checks.
pub const TEST_SGW_PIDS_ENV: &str = "CIMMERIA_LAB_TEST_SGW_PIDS";

/// Whether an image name or path names `SGW.exe`: the last path component,
/// any case, the whole name (`SGW.exe.bak`, `notSGW.exe` and `sgw` are not it).
pub fn is_sgw_image(image: &str) -> bool {
    let name = image.rsplit(['\\', '/']).next().unwrap_or(image);
    name.eq_ignore_ascii_case("SGW.exe")
}

/// The pids of every running `SGW.exe`. Under the test seam
/// ([`TEST_SGW_PIDS_ENV`]) the seam's list instead of the machine's.
pub fn sgw_pids() -> Vec<u32> {
    if let Some(pids) = seam_pids() {
        return pids;
    }
    processes()
        .into_iter()
        .filter(|(_, name)| is_sgw_image(name))
        .map(|(pid, _)| pid)
        .collect()
}

/// The seam's pids: `Some` only when `CIMMERIA_LAB_HOME` is set and
/// [`TEST_SGW_PIDS_ENV`] is present (an empty value is an empty list).
fn seam_pids() -> Option<Vec<u32>> {
    if !crate::daemon::paths::lab_home_overridden() {
        return None;
    }
    let raw = std::env::var(TEST_SGW_PIDS_ENV).ok()?;
    Some(
        raw.split(',')
            .filter_map(|part| part.trim().parse().ok())
            .collect(),
    )
}

/// Every running process as `(pid, image name)`. The name is the executable's
/// file name (`pwsh.exe`, `SGW.exe`); an unreadable list is empty.
#[cfg(windows)]
pub fn processes() -> Vec<(u32, String)> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    let mut found = Vec::new();
    // SAFETY: the snapshot handle is checked and closed; `entry` is a zeroed
    // PROCESSENTRY32W with `dwSize` set as the API requires, and outlives each call.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return found;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut more = Process32FirstW(snapshot, &mut entry);
        while more != 0 {
            let len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            found.push((
                entry.th32ProcessID,
                String::from_utf16_lossy(&entry.szExeFile[..len]),
            ));
            more = Process32NextW(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
    }
    found
}

/// Every running process as `(pid, name)`, from `ps` (the name is the last path
/// segment of the command). No `ps` means an empty list.
#[cfg(not(windows))]
pub fn processes() -> Vec<(u32, String)> {
    let Ok(output) = std::process::Command::new("ps")
        .args(["-A", "-o", "pid=,comm="])
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let (pid, name) = line.trim_start().split_once(char::is_whitespace)?;
            let name = name
                .trim()
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or("")
                .to_string();
            Some((pid.parse().ok()?, name))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_image_named_sgw_exe_is_a_client() {
        assert!(is_sgw_image(r"C:\Game\Binaries\SGW.exe"));
        assert!(is_sgw_image(r"c:\game\sgw.EXE"));
        assert!(is_sgw_image("SGW.exe"));
        assert!(is_sgw_image("/opt/x/sgw.exe"));
        assert!(!is_sgw_image(r"C:\x\SGW.exe.bak"));
        assert!(!is_sgw_image(r"C:\x\notSGW.exe"));
        assert!(!is_sgw_image(r"C:\x\SGW"));
        assert!(!is_sgw_image("sgw"));
        assert!(!is_sgw_image("SGW2.exe"));
        assert!(!is_sgw_image(""));
    }

    #[cfg(windows)]
    #[test]
    fn the_process_list_includes_this_process() {
        let me = std::process::id();
        assert!(processes().iter().any(|(pid, _)| *pid == me));
    }

    #[cfg(unix)]
    #[test]
    fn the_process_list_includes_this_process_when_ps_exists() {
        if std::process::Command::new("ps")
            .arg("-p")
            .arg("1")
            .output()
            .is_err()
        {
            return;
        }
        let me = std::process::id();
        assert!(processes().iter().any(|(pid, _)| *pid == me));
    }

    // A real process named SGW.exe (a copy of ping.exe) is found by pid.
    #[cfg(windows)]
    #[test]
    fn sgw_pids_finds_a_running_sgw_exe() {
        use std::process::{Command, Stdio};
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
        let ping = std::path::Path::new(&root)
            .join("System32")
            .join("PING.EXE");
        let dir = tempfile::TempDir::new().unwrap();
        let fake = dir.path().join("SGW.exe");
        std::fs::copy(&ping, &fake).unwrap();
        let mut child = Command::new(&fake)
            .args(["-n", "20", "localhost"])
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let found = processes()
            .iter()
            .any(|(pid, name)| *pid == child.id() && is_sgw_image(name));
        let _ = child.kill();
        let _ = child.wait();
        assert!(found, "the SGW.exe copy's pid is not in the snapshot");
        assert!(!sgw_pids().contains(&std::process::id()));
    }
}
