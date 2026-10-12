//! The HKCU registry read behind `token::daemon_token` (Windows only). Each
//! test works in a throwaway key under `HKCU\Software`, never `Environment`.
#![cfg(windows)]

use std::ptr::{null, null_mut};

use cimmeria_lab_cli::daemon::token::read_user_registry_string;
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    KEY_WRITE, REG_DWORD, REG_EXPAND_SZ, REG_OPTION_NON_VOLATILE, REG_SZ,
};

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// A throwaway key that is deleted when dropped.
struct TempKey {
    path: String,
    handle: HKEY,
}

impl TempKey {
    fn create(tag: &str) -> Self {
        let path = format!(
            "Software\\cimmeria-lab-cli-test\\{}-{tag}",
            std::process::id()
        );
        let subkey = wide(&path);
        let mut handle: HKEY = null_mut();
        // SAFETY: `subkey` is NUL-terminated UTF-16 that outlives the call and
        // `handle` is a valid out pointer.
        let status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                0,
                null(),
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                null(),
                &mut handle,
                null_mut(),
            )
        };
        assert_eq!(status, ERROR_SUCCESS, "create {path}");
        TempKey { path, handle }
    }

    fn set(&self, name: &str, kind: u32, data: &[u8]) {
        let name = wide(name);
        // SAFETY: `name` is NUL-terminated UTF-16 and `data` holds `len` bytes.
        let status = unsafe {
            RegSetValueExW(
                self.handle,
                name.as_ptr(),
                0,
                kind,
                data.as_ptr(),
                data.len() as u32,
            )
        };
        assert_eq!(status, ERROR_SUCCESS, "set value");
    }

    fn set_text(&self, name: &str, kind: u32, text: &str) {
        let bytes: Vec<u8> = wide(text)
            .iter()
            .flat_map(|unit| unit.to_le_bytes())
            .collect();
        self.set(name, kind, &bytes);
    }
}

impl Drop for TempKey {
    fn drop(&mut self) {
        // SAFETY: the handle came from RegCreateKeyExW; the path is NUL-terminated.
        unsafe {
            RegCloseKey(self.handle);
            RegDeleteTreeW(HKEY_CURRENT_USER, wide(&self.path).as_ptr());
        }
    }
}

#[test]
fn reg_sz_is_read_as_text() {
    let key = TempKey::create("sz");
    key.set_text("V", REG_SZ, "plain-value");
    assert_eq!(
        read_user_registry_string(&key.path, "V"),
        Some("plain-value".to_string())
    );
}

#[test]
fn a_missing_value_is_none() {
    let key = TempKey::create("missing");
    assert_eq!(read_user_registry_string(&key.path, "NOPE"), None);
    assert_eq!(
        read_user_registry_string("Software\\cimmeria-lab-cli-test\\no-such-key", "V"),
        None
    );
}

#[test]
fn an_empty_value_is_none() {
    let key = TempKey::create("empty");
    key.set_text("V", REG_SZ, "");
    assert_eq!(read_user_registry_string(&key.path, "V"), None);
}

#[test]
fn a_value_of_another_type_is_none() {
    let key = TempKey::create("dword");
    key.set("V", REG_DWORD, &7u32.to_le_bytes());
    assert_eq!(read_user_registry_string(&key.path, "V"), None);
}

#[test]
fn a_long_value_is_read_whole() {
    let key = TempKey::create("long");
    let long = "x".repeat(10_000);
    key.set_text("V", REG_SZ, &long);
    assert_eq!(read_user_registry_string(&key.path, "V"), Some(long));
}

#[test]
fn reg_expand_sz_is_expanded() {
    let key = TempKey::create("expand");
    key.set_text("V", REG_EXPAND_SZ, "%SystemRoot%\\suffix");
    let root = std::env::var("SystemRoot").expect("SystemRoot is set on Windows");
    assert_eq!(
        read_user_registry_string(&key.path, "V"),
        Some(format!("{root}\\suffix"))
    );
}

// An expansion much longer than the stored text (the size query may report
// the unexpanded size, so this takes the ERROR_MORE_DATA retry).
#[test]
fn a_much_longer_expansion_is_read_whole() {
    let key = TempKey::create("expand-long");
    let long = "y".repeat(5_000);
    key.set_text("V", REG_EXPAND_SZ, "%CIMMERIA_LAB_CLI_TEST_LONG%");
    // The process environment decides the expansion; set it for this test only.
    std::env::set_var("CIMMERIA_LAB_CLI_TEST_LONG", &long);
    let got = read_user_registry_string(&key.path, "V");
    std::env::remove_var("CIMMERIA_LAB_CLI_TEST_LONG");
    assert_eq!(got, Some(long));
}
