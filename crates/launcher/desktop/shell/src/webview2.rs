//! WebView2 runtime check. Tauri renders the window through the Edge WebView2
//! Runtime, so a machine without it is offered an install (PD-41) and, when
//! that is declined or doesn't finish, a download link instead of a launcher
//! that exits without a word. The install runs silently behind a launcher
//! notice, and a second launch during it is turned away (PD-41 ruling 1).

#[path = "webview2_install.rs"]
mod install;

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE},
    System::Threading::GetCurrentThreadId,
    UI::WindowsAndMessaging::{MB_ICONERROR, MB_ICONQUESTION},
};

/// Edge update client key that records the installed WebView2 runtime version.
#[cfg(windows)]
const CLIENT_KEY: &str =
    r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
#[cfg(windows)]
const WOW_CLIENT_KEY: &str =
    r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
#[cfg(windows)]
const DOWNLOAD_URL: &str = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";

/// Whether a `pv` value names an installed runtime. Empty and `0.0.0.0` mean none.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn version_is_present(pv: &str) -> bool {
    let pv = pv.trim();
    !pv.is_empty() && pv != "0.0.0.0"
}

/// Whether the runtime is present given the `pv` read at each registry
/// location. Any one present version is enough: a `0.0.0.0` left in one key
/// after an uninstall must not hide a runtime installed under another.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn runtime_present_in(reads: &[Option<String>]) -> bool {
    reads.iter().flatten().any(|pv| version_is_present(pv))
}

/// The download-page question, led by [`install::INSTALL_DID_NOT_FINISH`] and
/// the cause when an install attempt came first.
#[cfg_attr(not(windows), allow(dead_code))]
fn download_prompt(cause: Option<install::Cause>) -> String {
    let ask =
        "The Microsoft Edge WebView2 Runtime is needed to run the launcher. Open the download page?";
    match cause {
        Some(c) => format!(
            "{}\n{}\n\n{ask}",
            install::INSTALL_DID_NOT_FINISH,
            c.sentence()
        ),
        None => ask.to_string(),
    }
}

/// Returns when the WebView2 runtime is present, or once an install the player
/// accepted leaves one behind. Otherwise offers the download page and exits
/// the launcher with code 1. The install is tried at most once per start.
#[cfg(windows)]
pub fn ensure_runtime_or_exit() {
    let reads = read_all_pv();
    if runtime_present_in(&reads) {
        return;
    }
    // Release builds have no console, but a support run from a terminal or a
    // redirected stderr shows what each location held.
    eprintln!("WebView2 runtime not found; pv reads (HKLM wow, HKLM, HKCU wow, HKCU): {reads:?}");
    // Held until this function returns, or the process exits.
    let (lock, _guard) = take_setup_lock();
    if install::setup_gate(lock) == install::Gate::AlreadyRunning {
        show_notice(install::ALREADY_SETTING_UP);
        std::process::exit(0);
    }
    let cause = if ask_yes_no(install::INSTALL_PROMPT, MB_ICONQUESTION) {
        // SAFETY: GetCurrentThreadId has no preconditions.
        let main_thread = unsafe { GetCurrentThreadId() };
        let result = install::with_notice(
            || install::install(DOWNLOAD_URL, || runtime_present_in(&read_all_pv())),
            || show_notice(install::INSTALLING_NOTICE),
            || close_thread_notices(main_thread),
        );
        let cause = match result {
            Ok(Ok(())) => return,
            Ok(Err(cause)) => cause,
            Err(_) => install::Cause::CouldNotStart,
        };
        eprintln!("WebView2 install did not finish: {cause:?}");
        Some(cause)
    } else {
        None
    };
    if ask_yes_no(&download_prompt(cause), MB_ICONERROR) && !open_download_page() {
        show_download_url();
    }
    std::process::exit(1);
}

/// Closes the setup mutex handle when `ensure_runtime_or_exit` returns.
#[cfg(windows)]
struct SetupLockGuard(HANDLE);

#[cfg(windows)]
impl Drop for SetupLockGuard {
    fn drop(&mut self) {
        // SAFETY: the handle came from CreateMutexW and is closed once.
        unsafe { CloseHandle(self.0) };
    }
}

/// Takes the per-session setup mutex. A launcher that finds it held is a
/// second launch during an install. A null handle means no lock at all.
#[cfg(windows)]
fn take_setup_lock() -> (install::SetupLock, Option<SetupLockGuard>) {
    use windows_sys::Win32::System::Threading::CreateMutexW;

    let name = wide(r"Local\CimmeriaLauncher-WebView2-Setup");
    // SAFETY: the name is NUL-terminated and lives for the call; null
    // security attributes and no initial owner are accepted.
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return (install::SetupLock::Unavailable, None);
    }
    // SAFETY: read straight after CreateMutexW on the same thread.
    let lock = if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        install::SetupLock::AlreadyExists
    } else {
        install::SetupLock::Created
    };
    (lock, Some(SetupLockGuard(handle)))
}

/// Shows an OK-only notice and returns when it is closed.
#[cfg(windows)]
fn show_notice(text: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND,
    };

    let text = wide(text);
    let caption = wide("Cimmeria Launcher");
    // SAFETY: both strings are NUL-terminated and live for the call; no owner window.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND,
        );
    }
}

/// Posts the OK button's command to every top-level window of one thread, which
/// closes a message box that thread has open. Searching only that thread keeps
/// other processes' windows out of reach.
#[cfg(windows)]
fn close_thread_notices(thread_id: u32) {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumThreadWindows, PostMessageW, IDOK, WM_COMMAND,
    };

    unsafe extern "system" fn post_ok(hwnd: HWND, _: LPARAM) -> windows_sys::core::BOOL {
        // SAFETY: PostMessageW takes any window handle; a stale one just fails.
        unsafe { PostMessageW(hwnd, WM_COMMAND, IDOK as WPARAM, 0) };
        1
    }
    // SAFETY: the callback matches WNDENUMPROC and uses no state. The result
    // is FALSE when the thread has no windows, which is normal.
    unsafe { EnumThreadWindows(thread_id, Some(post_ok), 0) };
}

/// The `pv` read at every location [`registry_locations`] names, in its order.
#[cfg(windows)]
fn read_all_pv() -> Vec<Option<String>> {
    registry_locations()
        .into_iter()
        .map(|(root, path)| read_pv(root, path))
        .collect()
}

/// HKLM, then HKCU, each with and without WOW6432Node.
#[cfg(windows)]
fn registry_locations() -> [(windows_sys::Win32::System::Registry::HKEY, &'static str); 4] {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    [
        (HKEY_LOCAL_MACHINE, WOW_CLIENT_KEY),
        (HKEY_LOCAL_MACHINE, CLIENT_KEY),
        (HKEY_CURRENT_USER, WOW_CLIENT_KEY),
        (HKEY_CURRENT_USER, CLIENT_KEY),
    ]
}

/// Reads the `pv` string value under one key, or `None` when it is absent.
#[cfg(windows)]
fn read_pv(root: windows_sys::Win32::System::Registry::HKEY, path: &str) -> Option<String> {
    use windows_sys::Win32::System::Registry::{RegGetValueW, RRF_RT_REG_SZ};

    let subkey = wide(path);
    let value = wide("pv");
    let mut buffer = [0u16; 128];
    let mut bytes = std::mem::size_of_val(&buffer) as u32;
    // SAFETY: the key and value names are NUL-terminated, and the buffer
    // outlives the call with `bytes` set to its size in bytes.
    let status = unsafe {
        RegGetValueW(
            root,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    if status != 0 {
        return None;
    }
    let units = (bytes as usize / 2).min(buffer.len());
    Some(
        String::from_utf16_lossy(&buffer[..units])
            .trim_end_matches('\0')
            .to_string(),
    )
}

/// Asks a yes-or-no question with the given icon. True means Yes.
#[cfg(windows)]
fn ask_yes_no(
    question: &str,
    icon: windows_sys::Win32::UI::WindowsAndMessaging::MESSAGEBOX_STYLE,
) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, IDYES, MB_YESNO};

    let text = wide(question);
    let caption = wide("Cimmeria Launcher");
    // SAFETY: both strings are NUL-terminated and live for the call; no owner window.
    let answer = unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_YESNO | icon,
        )
    };
    answer == IDYES
}

/// Opens the WebView2 runtime download page in the default browser. False
/// when the shell could not open it (no browser, or a policy block).
#[cfg(windows)]
fn open_download_page() -> bool {
    use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

    let verb = wide("open");
    let url = wide(DOWNLOAD_URL);
    // SAFETY: every string is NUL-terminated and lives for the call; the
    // optional arguments are null, which the API accepts.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            url.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecuteW reports failure as a value of 32 or less.
    result as usize > 32
}

/// Shows the download address when the browser could not be opened.
#[cfg(windows)]
fn show_download_url() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_OK};

    let text = wide(&format!(
        "The download page could not be opened. Download the Microsoft Edge WebView2 Runtime from:\n\n{DOWNLOAD_URL}"
    ));
    let caption = wide("Cimmeria Launcher");
    // SAFETY: both strings are NUL-terminated and live for the call; no owner window.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

/// UTF-16 with a trailing NUL, as the Win32 wide-string APIs expect.
#[cfg(windows)]
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_zero_versions_are_missing() {
        assert!(!version_is_present(""));
        assert!(!version_is_present("   "));
        assert!(!version_is_present("0.0.0.0"));
    }

    #[test]
    fn released_version_is_present() {
        assert!(version_is_present("128.0.2739.42"));
    }

    #[test]
    fn no_location_with_a_version_is_missing() {
        assert!(!runtime_present_in(&[None, None, None, None]));
        assert!(!runtime_present_in(&[
            Some("0.0.0.0".to_string()),
            None,
            Some(String::new()),
            None,
        ]));
    }

    #[test]
    fn the_download_prompt_says_when_an_install_did_not_finish() {
        let plain = download_prompt(None);
        assert!(plain.starts_with("The Microsoft Edge WebView2 Runtime is needed"));
        let after = download_prompt(Some(install::Cause::TimedOut));
        assert!(after.starts_with("The automatic install didn't finish.\n"));
        assert!(after.contains(install::Cause::TimedOut.sentence()));
        assert!(after.ends_with(&plain));
    }

    #[test]
    fn a_stale_zero_version_does_not_hide_a_later_runtime() {
        assert!(runtime_present_in(&[
            Some("0.0.0.0".to_string()),
            None,
            Some("128.0.2739.42".to_string()),
            None,
        ]));
    }
}
