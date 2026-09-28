//! # cimmeria-client-hookgate
//!
//! Rules the two injected `SGW.exe` DLLs share, so that either can be
//! loaded alone or both together, in either order:
//!
//! - **The prologue check** ([`classify`]). Every address a DLL hooks or
//!   calls belongs to one `SGW.exe` build. Before hooking anything, a DLL
//!   compares the first bytes at each [`Site`] with the bytes that build
//!   has there, and installs nothing on a mismatch. A different build then
//!   costs a feature, not a crash.
//! - **Chaining.** `cimmeria-client-telemetry` and
//!   `cimmeria-client-patches` both hook `FEngineLoop::Tick` and the
//!   entity-method drop callee with MinHook. MinHook overwrites exactly the
//!   first five bytes with `E9 rel32` and, when it hooks a function that
//!   already starts with one, relocates that jump into its trampoline. So a
//!   chainable site that starts with `E9` and matches after byte 5 is
//!   accepted as [`Prologue::Chained`], but only when the jump lands inside
//!   one of the [`HOOK_OWNER_MODULES`]. A jump anywhere else is an unknown
//!   patch and fails the check.
//! - **One installer at a time** ([`hook_lock_name`], and `os::HookLock` on
//!   the i686 target). The two DLLs link separate MinHook copies that do not
//!   coordinate. If both hooked the same function at once, the second
//!   `MH_EnableHook` would overwrite the first one's jump and silently drop
//!   its detour. Each DLL holds a per-process named mutex for the whole of
//!   its check-and-hook phase, so whichever goes second sees the first one's
//!   jump and chains onto it.
//!
//! Neither DLL ever unhooks: removing a MinHook hook restores the bytes it
//! saved, which would cut out a hook chained on top of it.

use core::ops::Range;

#[cfg(all(windows, target_arch = "x86"))]
pub mod os;

/// Modules whose MinHook detours may already sit on a chainable site, under
/// the names the launcher installs them as and under cargo's output names.
pub const HOOK_OWNER_MODULES: [&str; 4] = [
    "cimmeria-client-telemetry.dll",
    "cimmeria_client_telemetry.dll",
    "cimmeria-client-patches.dll",
    "cimmeria_client_patches.dll",
];

/// Length of the `E9 rel32` jump MinHook writes over a prologue.
pub const JMP_REL32_LEN: usize = 5;

/// `FEngineLoop::Tick` (`0x00416ec0`), which both DLLs hook: `mov eax,
/// fs:[0]; push -1; push 0x01677b9f`.
pub const ENGINE_TICK: Site = Site {
    name: "FEngineLoop::Tick",
    address: 0x0041_6ec0,
    expected: &[
        0x64, 0xA1, 0x00, 0x00, 0x00, 0x00, 0x6A, 0xFF, 0x68, 0x9F, 0x7B, 0x67, 0x01,
    ],
    may_be_chained: true,
};

/// `EntityDescription_GetExposedClientMethodByIndex` (`0x01590f30`), the
/// entity-method drop callee both DLLs hook: `push esi; push edi; mov edi,
/// [esp+0xc]; test edi, edi; mov esi, ecx`.
pub const DROP_CALLEE: Site = Site {
    name: "EntityDescription_GetExposedClientMethodByIndex",
    address: 0x0159_0f30,
    expected: &[0x56, 0x57, 0x8B, 0x7C, 0x24, 0x0C, 0x85, 0xFF, 0x8B, 0xF1],
    may_be_chained: true,
};

/// A function (or pointer slot) a DLL hooks or calls, and its expected
/// first bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Site {
    /// Name for the log.
    pub name: &'static str,
    /// Address in `SGW.exe`.
    pub address: usize,
    /// The first bytes at `address` in this build.
    pub expected: &'static [u8],
    /// Whether the other DLL may already have hooked it with MinHook, so
    /// a leading `E9 rel32` is acceptable.
    pub may_be_chained: bool,
}

/// What a site's first bytes turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prologue {
    /// Exactly the expected bytes.
    Stock,
    /// An `E9 rel32` to `jump_to`, then the expected bytes: hooked by the
    /// other DLL's MinHook, to be chained on top of.
    Chained {
        /// Where the existing jump goes.
        jump_to: usize,
    },
    /// An `E9 rel32` with the rest intact, but jumping outside every
    /// module in [`HOOK_OWNER_MODULES`]: not a hook either DLL may chain.
    UnknownHook {
        /// Where the existing jump goes.
        jump_to: usize,
    },
    /// Something else: a different build, or a patch neither DLL knows
    /// how to chain.
    Mismatch {
        /// The bytes found.
        actual: Vec<u8>,
    },
    /// The bytes could not be read at all.
    Unreadable,
}

impl Prologue {
    /// Whether the DLL may hook or call the site.
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Stock | Self::Chained { .. })
    }
}

/// Classify `actual`, the bytes read at `site` (or `None` if unreadable).
/// `hook_owners` are the address ranges a chainable jump may land in: the
/// images of the loaded [`HOOK_OWNER_MODULES`].
pub fn classify(site: &Site, actual: Option<&[u8]>, hook_owners: &[Range<usize>]) -> Prologue {
    let Some(actual) = actual else {
        return Prologue::Unreadable;
    };
    if actual == site.expected {
        return Prologue::Stock;
    }
    let chained = site.may_be_chained
        && actual.len() == site.expected.len()
        && actual.len() > JMP_REL32_LEN
        && actual[0] == 0xE9
        && actual[JMP_REL32_LEN..] == site.expected[JMP_REL32_LEN..];
    if chained {
        let rel = i32::from_le_bytes([actual[1], actual[2], actual[3], actual[4]]);
        let jump_to = (site.address as u32)
            .wrapping_add(JMP_REL32_LEN as u32)
            .wrapping_add(rel as u32) as usize;
        if hook_owners.iter().any(|owner| owner.contains(&jump_to)) {
            return Prologue::Chained { jump_to };
        }
        return Prologue::UnknownHook { jump_to };
    }
    Prologue::Mismatch {
        actual: actual.to_vec(),
    }
}

/// The address range of the PE image loaded at `base`, from its
/// `SizeOfImage`. `read(addr, len)` returns the bytes at `addr`, or `None`
/// if any is unreadable. `None` unless `base` holds a readable `MZ`/`PE`
/// header.
pub fn image_range(
    read: impl Fn(usize, usize) -> Option<Vec<u8>>,
    base: usize,
) -> Option<Range<usize>> {
    let read_u32 =
        |addr: usize| -> Option<u32> { Some(u32::from_le_bytes(read(addr, 4)?.try_into().ok()?)) };
    if read(base, 2)? != b"MZ" {
        return None;
    }
    let nt = base.checked_add(read_u32(base.checked_add(0x3C)?)? as usize)?;
    if read(nt, 4)? != b"PE\0\0" {
        return None;
    }
    // IMAGE_NT_HEADERS: 4-byte signature, 20-byte file header, then the
    // optional header, whose SizeOfImage is at +0x38.
    let size = read_u32(nt.checked_add(0x50)?)? as usize;
    Some(base..base.checked_add(size)?)
}

/// Name of the per-process mutex both DLLs hold while they check and hook
/// sites. `Local\` keeps it inside the session; the pid keeps two game
/// processes from serialising each other.
pub fn hook_lock_name(pid: u32) -> String {
    format!("Local\\cimmeria-client-hooks-{pid}")
}

/// Space-separated hex, for the log.
pub fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const TICK: Site = ENGINE_TICK;
    const NOT_CHAINABLE: Site = Site {
        may_be_chained: false,
        ..TICK
    };
    /// A DLL image the test detours live in.
    const OWNER: Range<usize> = 0x7000_0000..0x7010_0000;

    /// What MinHook leaves at `site` when it hooks it with a detour at
    /// `detour`: `E9 rel32`, then the original bytes from offset 5 on.
    fn minhooked(site: &Site, detour: usize) -> Vec<u8> {
        let mut bytes = site.expected.to_vec();
        let rel = (detour as u32).wrapping_sub(site.address as u32 + 5);
        bytes[0] = 0xE9;
        bytes[1..5].copy_from_slice(&rel.to_le_bytes());
        bytes
    }

    #[test]
    fn stock_bytes_pass() {
        assert_eq!(classify(&TICK, Some(TICK.expected), &[]), Prologue::Stock);
    }

    #[test]
    fn a_minhooked_chainable_site_is_chained() {
        let bytes = minhooked(&TICK, 0x7000_1000);
        assert_eq!(
            classify(&TICK, Some(&bytes), &[OWNER]),
            Prologue::Chained {
                jump_to: 0x7000_1000
            }
        );
    }

    #[test]
    fn a_backwards_jump_decodes() {
        const LOW: Range<usize> = 0x0010_0000..0x0020_0000;
        let bytes = minhooked(&TICK, 0x0010_0000);
        assert_eq!(
            classify(&TICK, Some(&bytes), &[LOW]),
            Prologue::Chained {
                jump_to: 0x0010_0000
            }
        );
    }

    #[test]
    fn a_jump_outside_every_owner_is_refused() {
        for target in [OWNER.start - 1, OWNER.end, 0x1234_5678] {
            let bytes = minhooked(&TICK, target);
            let prologue = classify(&TICK, Some(&bytes), &[OWNER]);
            assert_eq!(prologue, Prologue::UnknownHook { jump_to: target });
            assert!(!prologue.is_usable());
        }
        let bytes = minhooked(&TICK, OWNER.start);
        assert!(
            !classify(&TICK, Some(&bytes), &[]).is_usable(),
            "no owner loaded"
        );
    }

    #[test]
    fn an_e9_on_a_site_nobody_else_hooks_is_a_mismatch() {
        let bytes = minhooked(&NOT_CHAINABLE, 0x7000_1000);
        assert!(matches!(
            classify(&NOT_CHAINABLE, Some(&bytes), &[OWNER]),
            Prologue::Mismatch { .. }
        ));
    }

    #[test]
    fn an_e9_with_a_changed_tail_is_a_mismatch() {
        let mut bytes = minhooked(&TICK, 0x7000_1000);
        bytes[6] = 0x90;
        assert!(matches!(
            classify(&TICK, Some(&bytes), &[OWNER]),
            Prologue::Mismatch { .. }
        ));
    }

    #[test]
    fn any_changed_byte_is_refused() {
        for i in 0..TICK.expected.len() {
            let mut bytes = TICK.expected.to_vec();
            bytes[i] ^= 0x01;
            assert!(
                !classify(&TICK, Some(&bytes), &[OWNER]).is_usable(),
                "byte {i}"
            );
        }
    }

    #[test]
    fn unreadable_or_short_is_refused() {
        assert_eq!(classify(&TICK, None, &[OWNER]), Prologue::Unreadable);
        assert!(!classify(&TICK, Some(&TICK.expected[..5]), &[OWNER]).is_usable());
    }

    /// Each DLL accepts the other's hooks: the owner list names both, under
    /// both spellings.
    #[test]
    fn both_dlls_are_hook_owners() {
        for dll in ["cimmeria-client-telemetry", "cimmeria-client-patches"] {
            assert!(HOOK_OWNER_MODULES.contains(&format!("{dll}.dll").as_str()));
            let cargo_name = format!("{}.dll", dll.replace('-', "_"));
            assert!(HOOK_OWNER_MODULES.contains(&cargo_name.as_str()));
        }
    }

    #[test]
    fn image_range_reads_size_of_image() {
        const BASE: usize = 0x6000_0000;
        let mut header = vec![0u8; 0x100];
        header[..2].copy_from_slice(b"MZ");
        header[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        header[0x80..0x84].copy_from_slice(b"PE\0\0");
        header[0xD0..0xD4].copy_from_slice(&0x0004_2000u32.to_le_bytes());
        let read = |h: &Vec<u8>| {
            let h = h.clone();
            move |addr: usize, len: usize| {
                let off = addr.checked_sub(BASE)?;
                h.get(off..off + len).map(<[u8]>::to_vec)
            }
        };
        assert_eq!(
            image_range(read(&header), BASE),
            Some(BASE..BASE + 0x0004_2000)
        );
        header[0x80] = b'X';
        assert_eq!(image_range(read(&header), BASE), None, "no PE signature");
        assert_eq!(image_range(|_, _| None, BASE), None, "unmapped");
    }

    /// Both DLLs must compute the same name, or the lock serialises nothing.
    #[test]
    fn lock_name_is_per_process_and_session_local() {
        assert_eq!(hook_lock_name(4242), "Local\\cimmeria-client-hooks-4242");
    }

    #[test]
    fn hex_is_spaced_upper_case() {
        assert_eq!(hex(&[0x6A, 0xFF, 0x00]), "6A FF 00");
    }
}
