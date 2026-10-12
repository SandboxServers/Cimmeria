//! Process checks for `lab watch`: which pids run `SGW.exe`, and whether the
//! process that wrote a batch's plan.json still runs.

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

/// Whether a process image name is a lab batch's owner: `pwsh...` (a
/// PowerShell `lab uat`) or `lab` (`lab.exe`), ignoring case and `.exe`.
pub fn name_is_batch_owner(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let stem = lower.strip_suffix(".exe").unwrap_or(&lower);
    stem.starts_with("pwsh") || stem == "lab"
}

/// Whether a process image name is the game client.
pub fn name_is_sgw(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "sgw.exe" || lower == "sgw"
}

/// Whether process `pid` exists and is a batch owner (`name_is_batch_owner`).
pub fn batch_owner_alive(pid: u32) -> bool {
    processes()
        .into_iter()
        .any(|(p, name)| p == pid && name_is_batch_owner(&name))
}

/// The pids of every `SGW.exe`.
pub fn sgw_pids() -> Vec<u32> {
    processes()
        .into_iter()
        .filter(|(_, name)| name_is_sgw(name))
        .map(|(pid, _)| pid)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pwsh_and_lab_own_batches() {
        for name in [
            "pwsh.exe",
            "PWSH.EXE",
            "pwsh",
            "pwsh-preview.exe",
            "lab.exe",
            "lab",
            "Lab.EXE",
        ] {
            assert!(name_is_batch_owner(name), "{name}");
        }
    }

    #[test]
    fn other_programs_do_not_own_batches() {
        for name in [
            "explorer.exe",
            "SGW.exe",
            "cimmeria-lab.exe",
            "labd.exe",
            "powershell.exe",
            "",
        ] {
            assert!(!name_is_batch_owner(name), "{name}");
        }
    }

    #[test]
    fn only_sgw_is_the_game_client() {
        assert!(name_is_sgw("SGW.exe") && name_is_sgw("sgw"));
        assert!(!name_is_sgw("SGW2.exe") && !name_is_sgw("pwsh.exe"));
    }

    #[test]
    fn a_pid_that_is_not_a_batch_owner_is_not_alive() {
        // This test binary is neither pwsh nor lab, and u32::MAX is no process.
        assert!(!batch_owner_alive(std::process::id()));
        assert!(!batch_owner_alive(u32::MAX));
    }

    /// A running `pwsh` (the writer of a batch before `lab.exe` took over) reads
    /// alive while it runs and not once it has ended. Skipped where `pwsh` is
    /// not installed.
    #[test]
    fn a_running_pwsh_owns_a_batch_until_it_ends() {
        let Ok(mut child) = std::process::Command::new("pwsh")
            .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 60"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        else {
            return;
        };
        let pid = child.id();
        let alive_while_running = batch_owner_alive(pid);
        let _ = child.kill();
        let _ = child.wait();
        assert!(alive_while_running, "pwsh pid {pid} should read as alive");
        assert!(!batch_owner_alive(pid), "pwsh pid {pid} has ended");
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
}
