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

/// Whether the runtime is present, with the registry read supplied by the caller.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn runtime_present_with(read: impl Fn() -> Option<String>) -> bool {
    read().is_some_and(|pv| version_is_present(&pv))
}

/// Exits the launcher with code 1 when the WebView2 runtime is missing, after
/// offering the download page. Returns when the runtime is present.
#[cfg(windows)]
pub fn ensure_runtime_or_exit() {
    if runtime_present_with(read_pv) {
        return;
    }
    if confirm_download() {
        open_download_page();
    }
    std::process::exit(1);
}

/// Reads the `pv` value from HKLM, then HKCU, each with and without WOW6432Node.
#[cfg(windows)]
fn read_pv() -> Option<String> {
    use windows_sys::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ,
    };

    let keys = [
        (HKEY_LOCAL_MACHINE, WOW_CLIENT_KEY),
        (HKEY_LOCAL_MACHINE, CLIENT_KEY),
        (HKEY_CURRENT_USER, WOW_CLIENT_KEY),
        (HKEY_CURRENT_USER, CLIENT_KEY),
    ];
    keys.into_iter().find_map(|(root, path)| {
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
    })
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

/// Opens the WebView2 runtime download page in the default browser.
#[cfg(windows)]
fn open_download_page() {
    use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

    let verb = wide("open");
    let url = wide(DOWNLOAD_URL);
    // SAFETY: every string is NUL-terminated and lives for the call; the
    // optional arguments are null, which the API accepts.
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            url.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
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
    fn missing_registry_value_is_not_present() {
        assert!(!runtime_present_with(|| None));
        assert!(!runtime_present_with(|| Some("0.0.0.0".to_string())));
        assert!(runtime_present_with(|| Some("128.0.2739.42".to_string())));
    }
}
