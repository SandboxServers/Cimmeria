//! What `lab doctor` reads from the machine: the scheduled task, the daemon,
//! labd.env, account and seed folders, running `SGW.exe` processes, the two
//! installed exes and the tools/lab content check.

use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::checks::{sgw_exe_path, BinaryFacts, ContentCheck, Inputs, InstanceFacts};
use crate::daemon::env_file::{self, EnvLine};
use crate::daemon::status::{self, Endpoint, StatusReply};
use crate::daemon::{paths, token};

/// Gathers every fact the checks need.
pub fn gather() -> Inputs {
    let env = env_file::read_or_empty(&paths::labd_env_path());
    let status = status::fetch(&Endpoint::discover());
    let install_dir = env_file::get(&env, "CIMMERIA_LAB_INSTALL_DIR")
        .map(str::trim)
        .filter(|dir| !dir.is_empty())
        .map(str::to_string);
    let profile_root = paths::profile_root();
    let instances = labels_for(status.as_ref(), &env)
        .into_iter()
        .map(|label| {
            let account_file = install_dir
                .as_deref()
                .is_some_and(|dir| account_path(dir, &label).exists());
            let seeded = seed_path(&profile_root, &label).exists();
            InstanceFacts {
                label,
                account_file,
                seeded,
            }
        })
        .collect();
    let home = paths::lab_home();
    let version_sha = read_version_sha(&paths::installed_cli_dir().join("cli").join("VERSION"));
    let checkout = std::env::current_dir()
        .ok()
        .and_then(|dir| checkout_root(&dir));
    let content = content_check(checkout.as_deref(), version_sha.as_deref());
    Inputs {
        task_installed: task_installed(),
        listed_sgw: listed_pids(status.as_ref()),
        status,
        token_set: token::daemon_token().is_some(),
        sgw_exe_exists: install_dir
            .as_deref()
            .is_some_and(|dir| sgw_exe_path(dir).exists()),
        install_dir,
        profile_root: profile_root.to_string_lossy().into_owned(),
        instances,
        running_sgw: sgw_pids(),
        binaries: compare_binaries(
            &home.join("bin").join("cimmeria-lab.exe"),
            &home.join("labd").join("cimmeria-lab.exe"),
        ),
        version_sha,
        checkout,
        content,
    }
}

/// Whether `schtasks /Query /TN CimmeriaLabDaemon` exits 0.
pub fn task_installed() -> bool {
    Command::new("schtasks")
        .args(["/Query", "/TN", "CimmeriaLabDaemon"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// The instances the checks cover: the daemon's, else labd.env's
/// `CIMMERIA_LAB_INSTANCES`, else `default`.
pub fn labels_for(status: Option<&StatusReply>, env: &[EnvLine]) -> Vec<String> {
    if let Some(status) = status {
        return status
            .instances
            .iter()
            .map(|row| row.instance.clone())
            .collect();
    }
    let listed: Vec<String> = env_file::get(env, "CIMMERIA_LAB_INSTANCES")
        .unwrap_or("")
        .split(',')
        .map(|label| label.trim().to_string())
        .filter(|label| !label.is_empty())
        .collect();
    if listed.is_empty() {
        vec!["default".to_string()]
    } else {
        listed
    }
}

/// The `client_pid` of every instance `/status` lists.
pub fn listed_pids(status: Option<&StatusReply>) -> Vec<u32> {
    status
        .map(|status| {
            status
                .instances
                .iter()
                .filter_map(|row| row.client_pid)
                .collect()
        })
        .unwrap_or_default()
}

/// `<install dir>\Binaries\sessions\lab-account[.<label>].json`.
pub fn account_path(install_dir: &str, label: &str) -> PathBuf {
    let file = if label == "default" {
        "lab-account.json".to_string()
    } else {
        format!("lab-account.{label}.json")
    };
    Path::new(install_dir)
        .join("Binaries")
        .join("sessions")
        .join(file)
}

/// `<profile root>\<label>\profile\Documents\My Games\Firesky\SGWGame`.
pub fn seed_path(profile_root: &Path, label: &str) -> PathBuf {
    profile_root
        .join(label)
        .join("profile")
        .join("Documents")
        .join("My Games")
        .join("Firesky")
        .join("SGWGame")
}

/// The first line of the VERSION file, trimmed; `None` when absent or blank.
pub fn read_version_sha(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let line = text
        .trim_start_matches('\u{feff}')
        .lines()
        .next()?
        .trim()
        .to_string();
    (!line.is_empty()).then_some(line)
}

/// Compares the installed exe with the daemon's copy.
pub fn compare_binaries(bin: &Path, daemon: &Path) -> BinaryFacts {
    let mut files = Vec::new();
    for exe in [bin, daemon] {
        match File::open(exe) {
            Ok(file) if exe.is_file() => files.push(file),
            _ => return BinaryFacts::Missing(exe.to_path_buf()),
        }
    }
    let (mut a, mut b) = (files.remove(0), files.remove(0));
    let (Ok(meta_a), Ok(meta_b)) = (a.metadata(), b.metadata()) else {
        return BinaryFacts::Differ;
    };
    if meta_a.len() != meta_b.len() {
        return BinaryFacts::Differ;
    }
    let mut buf_a = vec![0u8; 64 * 1024];
    let mut buf_b = vec![0u8; 64 * 1024];
    loop {
        let (Ok(n_a), Ok(n_b)) = (fill(&mut a, &mut buf_a), fill(&mut b, &mut buf_b)) else {
            return BinaryFacts::Differ;
        };
        if n_a != n_b || buf_a[..n_a] != buf_b[..n_b] {
            return BinaryFacts::Differ;
        }
        if n_a == 0 {
            return BinaryFacts::Same(meta_a.len());
        }
    }
}

/// Reads until `buf` is full or the file ends.
fn fill(reader: &mut impl Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        let n = reader.read(&mut buf[filled..])?;
        if n == 0 {
            break;
        }
        filled += n;
    }
    Ok(filled)
}

/// The top of the Cimmeria checkout that `start` is in (it has
/// `tools\lab\lab.ps1`), else `None`; also `None` when git is missing.
pub fn checkout_root(start: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(start)
        .args(["rev-parse", "--show-toplevel"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let top = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if top.is_empty() {
        return None;
    }
    let top = PathBuf::from(top);
    top.join("tools")
        .join("lab")
        .join("lab.ps1")
        .exists()
        .then_some(top)
}

/// `git -C <root> diff --quiet <sha> origin/main -- tools/lab`, run only when
/// both the checkout and the sha exist (else `None`, and git is not started).
pub fn content_check(root: Option<&Path>, sha: Option<&str>) -> Option<ContentCheck> {
    let (root, sha) = (root?, sha?);
    if sha.starts_with('-') {
        return Some(ContentCheck::Unknown);
    }
    let code = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["diff", "--quiet", sha, "origin/main", "--", "tools/lab"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
        .and_then(|status| status.code());
    Some(match code {
        Some(0) => ContentCheck::Same,
        Some(1) => ContentCheck::Differs,
        _ => ContentCheck::Unknown,
    })
}

/// Whether a process image name is the game client.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn is_sgw_image(name: &str) -> bool {
    name.eq_ignore_ascii_case("SGW.exe")
}

/// Pids of every running `SGW.exe`. LX-15 reuses this.
#[cfg(windows)]
pub(crate) fn sgw_pids() -> Vec<u32> {
    use std::mem::{size_of, zeroed};
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    let mut pids = Vec::new();
    // SAFETY: plain Win32 calls. `entry` is zeroed and its `dwSize` is set as
    // the API requires; the snapshot handle is closed before returning.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return pids;
        }
        let mut entry: PROCESSENTRY32W = zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        let mut more = Process32FirstW(snapshot, &mut entry) != 0;
        while more {
            let len = entry
                .szExeFile
                .iter()
                .position(|&unit| unit == 0)
                .unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
            if is_sgw_image(&name) {
                pids.push(entry.th32ProcessID);
            }
            more = Process32NextW(snapshot, &mut entry) != 0;
        }
        CloseHandle(snapshot);
    }
    pids
}

/// No process list off Windows: no `SGW.exe` runs there.
#[cfg(not(windows))]
pub(crate) fn sgw_pids() -> Vec<u32> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ops: the Get-Process -Name SGW filter (image name SGW.exe, any case)
    #[test]
    fn image_name_must_be_sgw_exe() {
        assert!(is_sgw_image("SGW.exe"));
        assert!(is_sgw_image("sgw.EXE"));
        assert!(!is_sgw_image("SGW2.exe"));
        assert!(!is_sgw_image("notSGW.exe"));
        assert!(!is_sgw_image("SGW"));
    }

    // A real process named SGW.exe (a copy of ping.exe) is found by pid.
    #[cfg(windows)]
    #[test]
    fn sgw_pids_finds_a_running_sgw_exe() {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
        let ping = Path::new(&root).join("System32").join("PING.EXE");
        let dir = tempfile::TempDir::new().unwrap();
        let fake = dir.path().join("SGW.exe");
        std::fs::copy(&ping, &fake).unwrap();
        let mut child = Command::new(&fake)
            .args(["-n", "20", "127.0.0.1"])
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let found = sgw_pids().contains(&child.id());
        let _ = child.kill();
        let _ = child.wait();
        assert!(found, "the SGW.exe copy's pid is not in the snapshot");
        assert!(!sgw_pids().contains(&std::process::id()));
    }
}
