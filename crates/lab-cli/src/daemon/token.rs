//! The daemon bearer token that gates its loopback endpoint.

/// The variable the daemon's bearer token lives in.
pub const TOKEN_VAR: &str = "CIMMERIA_LAB_DAEMON_TOKEN";

/// The bearer token: the user environment (`HKCU\Environment`, Windows
/// only), else the process environment. An empty value is `None`. Never
/// print the result. Port of `Get-LabToken`.
///
/// When `CIMMERIA_LAB_HOME` is set (the test-only override of the lab home)
/// the registry is not read: only the process environment counts. Otherwise a
/// test run against a temp lab home would pick up the real daemon's token
/// from the developer's own `HKCU\Environment` and could drive the real daemon.
pub fn daemon_token() -> Option<String> {
    token_with(
        super::paths::lab_home_overridden(),
        || user_env(TOKEN_VAR),
        std::env::var(TOKEN_VAR).ok(),
    )
}

/// [`daemon_token`] with its inputs given. `read_user` is called only when
/// `home_overridden` is false; when it is true only `process` is used.
pub fn token_with(
    home_overridden: bool,
    read_user: impl FnOnce() -> Option<String>,
    process: Option<String>,
) -> Option<String> {
    let user = if home_overridden { None } else { read_user() };
    pick_token(user, process)
}

/// The first non-empty of the user value and the process value.
pub fn pick_token(user: Option<String>, process: Option<String>) -> Option<String> {
    [user, process]
        .into_iter()
        .flatten()
        .find(|token| !token.is_empty())
}

/// `HKCU\Environment\<name>` (REG_SZ or REG_EXPAND_SZ, expanded), or `None`
/// when it is absent, empty or unreadable.
#[cfg(windows)]
fn user_env(name: &str) -> Option<String> {
    read_user_registry_string("Environment", name)
}

/// `HKCU\<subkey>\<name>` as text (REG_SZ, or REG_EXPAND_SZ with its
/// `%VARS%` expanded), or `None` when the value is absent, empty, of another
/// type or unreadable. Public so a test can read a throwaway key.
#[cfg(windows)]
pub fn read_user_registry_string(subkey: &str, name: &str) -> Option<String> {
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{ERROR_MORE_DATA, ERROR_SUCCESS};
    use windows_sys::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ,
    };

    let subkey: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    let value: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let flags = RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ;

    // First call: ask for the size in bytes (includes the terminating NUL).
    let mut size: u32 = 0;
    // SAFETY: `subkey` and `value` are NUL-terminated UTF-16 that outlive the
    // call; a null data pointer with a size pointer only queries the size.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            value.as_ptr(),
            flags,
            null_mut(),
            null_mut(),
            &mut size,
        )
    };
    if status != ERROR_SUCCESS || size == 0 {
        return None;
    }

    // Second call: read into a buffer; retry a few times if the value grew.
    for _ in 0..3 {
        let mut buffer: Vec<u16> = vec![0; (size as usize) / 2 + 2];
        let mut bytes = (buffer.len() * 2) as u32;
        // SAFETY: `buffer` holds `bytes` bytes and outlives the call.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                value.as_ptr(),
                flags,
                null_mut(),
                buffer.as_mut_ptr().cast(),
                &mut bytes,
            )
        };
        if status == ERROR_MORE_DATA {
            size = bytes;
            continue;
        }
        if status != ERROR_SUCCESS {
            return None;
        }
        let units = ((bytes as usize) / 2).min(buffer.len());
        // The text ends at the first NUL: after an expansion `bytes` can still
        // count the longer unexpanded text, and the tail is not part of the value.
        let units = &buffer[..units];
        let end = units
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(units.len());
        let text = String::from_utf16_lossy(&units[..end]);
        return if text.is_empty() { None } else { Some(text) };
    }
    None
}

/// No user environment off Windows: only the process environment counts.
#[cfg(not(windows))]
fn user_env(_name: &str) -> Option<String> {
    None
}
