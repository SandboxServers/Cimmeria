//! Process checks for `lab watch`: whether the process that wrote a batch's
//! plan.json still runs. The process list itself is [`crate::processes`].

use crate::processes::processes;

/// Whether a process image name is a lab batch's owner: `pwsh...` (a
/// PowerShell `lab uat`) or `lab` (`lab.exe`), ignoring case and `.exe`.
pub fn name_is_batch_owner(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let stem = lower.strip_suffix(".exe").unwrap_or(&lower);
    stem.starts_with("pwsh") || stem == "lab"
}

/// Whether process `pid` exists and is a batch owner (`name_is_batch_owner`).
pub fn batch_owner_alive(pid: u32) -> bool {
    processes()
        .into_iter()
        .any(|(p, name)| p == pid && name_is_batch_owner(&name))
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
}
