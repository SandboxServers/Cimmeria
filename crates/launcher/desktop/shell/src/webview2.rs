//! WebView2 runtime check. Tauri renders the window through the Edge WebView2
//! Runtime, so a machine without it gets a message and a download link instead
//! of a launcher that exits without a word.

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

/// Exits the launcher with code 1 when the WebView2 runtime is missing, after
/// offering the download page. Returns when the runtime is present.
#[cfg(windows)]
pub fn ensure_runtime_or_exit() {
    let reads: Vec<Option<String>> = registry_locations()
        .into_iter()
        .map(|(root, path)| read_pv(root, path))
        .collect();
    if runtime_present_in(&reads) {
        return;
    }
    // Release builds have no console, but a support run from a terminal or a
    // redirected stderr shows what each location held.
    eprintln!("WebView2 runtime not found; pv reads (HKLM wow, HKLM, HKCU wow, HKCU): {reads:?}");
    if confirm_download() && !open_download_page() {
        show_download_url();
    }
    std::process::exit(1);
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

/// Asks whether to open the download page. Yes means open it.
#[cfg(windows)]
fn confirm_download() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, IDYES, MB_ICONERROR, MB_YESNO};

    let text = wide(
        "The Microsoft Edge WebView2 Runtime is needed to run the launcher. Open the download page?",
    );
    let caption = wide("Stargate Worlds Launcher");
    // SAFETY: both strings are NUL-terminated and live for the call; no owner window.
    let answer = unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_YESNO | MB_ICONERROR,
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
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    let text = wide(&format!(
        "The download page could not be opened. Download the Microsoft Edge WebView2 Runtime from:\n\n{DOWNLOAD_URL}"
    ));
    let caption = wide("Stargate Worlds Launcher");
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
    fn a_stale_zero_version_does_not_hide_a_later_runtime() {
        assert!(runtime_present_in(&[
            Some("0.0.0.0".to_string()),
            None,
            Some("128.0.2739.42".to_string()),
            None,
        ]));
    }
}
