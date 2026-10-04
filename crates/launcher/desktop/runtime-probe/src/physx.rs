//! SGW's observed 2.6.3 creation descriptor, not guessed modern SDK headers.
//! Address evidence and the null-default assumption: ../docs/physx-probe.md.
use serde::{Deserialize, Serialize};
use std::ffi::c_void;

pub const VERSION: u32 = 0x0206_0300;
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct Descriptor(pub [u32; 4]);
pub const DESCRIPTOR: Descriptor = Descriptor([65536, 256, 2048, 0]);

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum SdkResult {
    NotChecked {},
    UnverifiedLoader {},
    LoadFailed { win32_error: u32 },
    MissingExport {},
    CreateFailed { sdk_error: Option<u32> },
    InitializedAndReleased {},
}

/// Keep module ownership outside this call. Both functions are resolved first.
/// A crash/hang yields no successful report and is handled by the parent process.
pub fn exercise(
    create: impl FnOnce(u32, &Descriptor, &mut u32) -> *mut c_void,
    release: impl FnOnce(*mut c_void),
) -> SdkResult {
    let mut error = u32::MAX;
    let sdk = create(VERSION, &DESCRIPTOR, &mut error);
    if sdk.is_null() {
        SdkResult::CreateFailed {
            sdk_error: (error != u32::MAX).then_some(error),
        }
    } else {
        release(sdk);
        SdkResult::InitializedAndReleased {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observed_arguments_and_successful_teardown_are_exact() {
        let mut token = 0_u8;
        let pointer = (&mut token as *mut u8).cast::<c_void>();
        let releases = std::cell::Cell::new(0);
        let result = exercise(
            |version, descriptor, _| {
                assert_eq!(version, 0x02060300);
                assert_eq!(descriptor.0, [65536, 256, 2048, 0]);
                assert_eq!(std::mem::size_of::<Descriptor>(), 16);
                pointer
            },
            |sdk| {
                assert_eq!(sdk, pointer);
                releases.set(releases.get() + 1);
            },
        );
        assert_eq!(result, SdkResult::InitializedAndReleased {});
        assert_eq!(releases.get(), 1);
    }
    #[test]
    fn failed_creation_never_releases_and_retains_numeric_error() {
        let result = exercise(
            |_, _, error| {
                *error = 1;
                std::ptr::null_mut()
            },
            |_| panic!("null SDK must not be released"),
        );
        assert_eq!(result, SdkResult::CreateFailed { sdk_error: Some(1) });
        assert_eq!(
            exercise(|_, _, _| std::ptr::null_mut(), |_| panic!()),
            SdkResult::CreateFailed { sdk_error: None }
        );
    }
}
