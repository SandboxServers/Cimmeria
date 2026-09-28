//! The build fingerprint gate.
//!
//! Every address in [`crate::addresses`] belongs to one `SGW.exe` build.
//! Before hooking anything, the DLL compares the first bytes of each
//! function it hooks or will call with the bytes that build has there. On
//! any mismatch it installs nothing: a different build turns into "the
//! Black Market is unavailable" instead of a crash.
//!
//! A hooked site may already start with `E9 rel32`: the telemetry DLL
//! hooks `FEngineLoop::Tick` and the drop callee with MinHook, and both
//! DLLs can be loaded at once. MinHook overwrites exactly the first five
//! bytes and leaves the rest, so such a site is accepted as
//! [`Prologue::Chained`] when everything after the jump still matches, and
//! MinHook chains on top of the earlier hook.
//!
//! The jump must also land inside a module known to hook these sites: the
//! telemetry DLL, whose MinHook (x86, so no relay) jumps straight to its
//! detour. A jump anywhere else, such as a stale patch or an unknown DLL,
//! is [`Prologue::UnknownHook`] and fails the gate: chaining would run
//! code nobody has vetted.

use core::ops::Range;

use crate::addresses;
use crate::memory::MemoryReader;

/// Modules whose MinHook detours may already sit on a chainable site: the
/// telemetry DLL, under the name the launcher installs it as and under
/// cargo's output name.
pub const HOOK_OWNER_MODULES: [&str; 2] = [
    "cimmeria-client-telemetry.dll",
    "cimmeria_client_telemetry.dll",
];

/// Length of the `E9 rel32` jump MinHook writes over a prologue.
pub const JMP_REL32_LEN: usize = 5;

/// A function the DLL hooks or calls, and its expected first bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Site {
    /// Name for the log.
    pub name: &'static str,
    /// Address in `SGW.exe`.
    pub address: usize,
    /// The first bytes of the function in this build.
    pub expected: &'static [u8],
    /// Whether another DLL may already have hooked it (see the module
    /// docs), so a leading `E9 rel32` is acceptable.
    pub may_be_chained: bool,
}

/// `Client_NetIn_EntityMethodDispatch`: `push -1; push 0x016f50bf; mov
/// eax, fs:[0]`. Nothing else hooks it.
pub const DISPATCHER: Site = Site {
    name: "Client_NetIn_EntityMethodDispatch",
    address: addresses::CLIENT_NET_IN_ENTITY_METHOD_DISPATCH,
    expected: &[
        0x6A, 0xFF, 0x68, 0xBF, 0x50, 0x6F, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00, 0x00,
    ],
    may_be_chained: false,
};

/// `EntityDescription_GetExposedClientMethodByIndex`: `push esi; push edi;
/// mov edi, [esp+0xc]; test edi, edi; mov esi, ecx`. The telemetry DLL's
/// drop oracle hooks it.
pub const DROP_CALLEE: Site = Site {
    name: "EntityDescription_GetExposedClientMethodByIndex",
    address: addresses::GET_EXPOSED_CLIENT_METHOD_BY_INDEX,
    expected: &[0x56, 0x57, 0x8B, 0x7C, 0x24, 0x0C, 0x85, 0xFF, 0x8B, 0xF1],
    may_be_chained: true,
};

/// `FEngineLoop::Tick`: `mov eax, fs:[0]; push -1; push 0x01677b9f`. The
/// telemetry DLL hooks it.
pub const ENGINE_TICK: Site = Site {
    name: "FEngineLoop::Tick",
    address: addresses::FENGINE_LOOP_TICK,
    expected: &[
        0x64, 0xA1, 0x00, 0x00, 0x00, 0x00, 0x6A, 0xFF, 0x68, 0x9F, 0x7B, 0x67, 0x01,
    ],
    may_be_chained: true,
};

/// `ServerConnection::startEntityMessage`: `mov eax, fs:[0]; push -1; push
/// 0x01705538`. Not hooked; the send natives call it.
pub const START_ENTITY_MESSAGE: Site = Site {
    name: "ServerConnection::startEntityMessage",
    address: addresses::SERVER_CONNECTION_START_ENTITY_MESSAGE,
    expected: &[
        0x64, 0xA1, 0x00, 0x00, 0x00, 0x00, 0x6A, 0xFF, 0x68, 0x38, 0x55, 0x70, 0x01,
    ],
    may_be_chained: false,
};

/// `ServerConnection::startAvatarMessage`: `mov eax, [esp+4]; push 0; push
/// eax; call startEntityMessage; ret 4`. Neither hooked nor called: the
/// whole body is checked, including the `rel32` to `startEntityMessage`, so
/// the `(conn, idx, entityId = 0)` call the send natives copy is this
/// build's.
pub const START_AVATAR_MESSAGE: Site = Site {
    name: "ServerConnection::startAvatarMessage",
    address: addresses::SERVER_CONNECTION_START_AVATAR_MESSAGE,
    expected: &[
        0x8B, 0x44, 0x24, 0x04, 0x6A, 0x00, 0x50, 0xE8, 0x44, 0xEA, 0xFF, 0xFF, 0xC2, 0x04, 0x00,
    ],
    may_be_chained: false,
};

/// `ServerConnection::isOnline`: `xor eax, eax; cmp [ecx+0x30c], eax; setne
/// al; ret`. Not called: pins the `+0x30c` online flag the send natives
/// read.
pub const IS_ONLINE: Site = Site {
    name: "ServerConnection::isOnline",
    address: addresses::SERVER_CONNECTION_IS_ONLINE,
    expected: &[
        0x33, 0xC0, 0x39, 0x81, 0x0C, 0x03, 0x00, 0x00, 0x0F, 0x95, 0xC0, 0xC3,
    ],
    may_be_chained: false,
};

/// The `GameEntityManager` getter: `mov eax, [0x01ef244c]; ret`. Not
/// called: pins the singleton address the send and receive sides read.
pub const GAME_ENTITY_MANAGER_GET: Site = Site {
    name: "GameEntityManager getter",
    address: addresses::GAME_ENTITY_MANAGER_GET,
    expected: &[0xA1, 0x4C, 0x24, 0xEF, 0x01, 0xC3],
    may_be_chained: false,
};

/// The engine's write-one-byte helper: `push esi; mov esi, ecx; mov eax,
/// [esi]; mov edx, [eax+0x10]; push 1; call edx`. Not called: pins the
/// bundle's `reserve` vtable slot the send natives call.
pub const BUNDLE_WRITE_U8: Site = Site {
    name: "Bundle write-u8 helper",
    address: addresses::BUNDLE_WRITE_U8,
    expected: &[
        0x56, 0x8B, 0xF1, 0x8B, 0x06, 0x8B, 0x50, 0x10, 0x6A, 0x01, 0xFF, 0xD2,
    ],
    may_be_chained: false,
};

/// Every site the gate checks: the three hooked functions, the one the
/// send natives call, and four whose bytes pin the send side's data
/// offsets. Any of them failing installs nothing, receive included.
pub const SITES: [Site; 8] = [
    DISPATCHER,
    DROP_CALLEE,
    ENGINE_TICK,
    START_ENTITY_MESSAGE,
    START_AVATAR_MESSAGE,
    IS_ONLINE,
    GAME_ENTITY_MANAGER_GET,
    BUNDLE_WRITE_U8,
];

/// What a site's first bytes turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prologue {
    /// Exactly the expected bytes.
    Stock,
    /// An `E9 rel32` to `jump_to`, then the expected bytes: hooked by
    /// another MinHook user, to be chained on top of.
    Chained {
        /// Where the existing jump goes.
        jump_to: usize,
    },
    /// An `E9 rel32` with the rest intact, but jumping outside every
    /// module in [`HOOK_OWNER_MODULES`]: not a hook this DLL may chain.
    UnknownHook {
        /// Where the existing jump goes.
        jump_to: usize,
    },
    /// Something else: a different build, or a patch this DLL does not
    /// know how to chain.
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

/// Read and classify one site.
pub fn check<M: MemoryReader>(mem: &M, site: &Site, hook_owners: &[Range<usize>]) -> Prologue {
    classify(
        site,
        mem.read_bytes(site.address, site.expected.len()).as_deref(),
        hook_owners,
    )
}

/// Read and classify every site in [`SITES`].
pub fn check_all<M: MemoryReader>(mem: &M, hook_owners: &[Range<usize>]) -> Vec<(Site, Prologue)> {
    SITES
        .iter()
        .map(|s| (*s, check(mem, s, hook_owners)))
        .collect()
}

/// The address range of the PE image loaded at `base`, from its
/// `SizeOfImage`. `None` unless `base` holds a readable `MZ`/`PE` header.
pub fn image_range<M: MemoryReader>(mem: &M, base: usize) -> Option<Range<usize>> {
    if mem.read_bytes(base, 2)? != b"MZ" {
        return None;
    }
    let nt = base.checked_add(mem.read_u32(base.checked_add(0x3C)?)? as usize)?;
    if mem.read_bytes(nt, 4)? != b"PE\0\0" {
        return None;
    }
    // IMAGE_NT_HEADERS: 4-byte signature, 20-byte file header, then the
    // optional header, whose SizeOfImage is at +0x38.
    let size = mem.read_u32(nt.checked_add(0x50)?)? as usize;
    Some(base..base.checked_add(size)?)
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
    use crate::memory::FakeMemory;

    /// What MinHook leaves at `site` when it hooks it with a detour at
    /// `detour`: `E9 rel32`, then the original bytes from offset 5 on.
    /// A telemetry DLL image the test detours live in.
    const OWNER: Range<usize> = 0x7000_0000..0x7010_0000;

    fn minhooked(site: &Site, detour: usize) -> Vec<u8> {
        let mut bytes = site.expected.to_vec();
        let rel = (detour as u32).wrapping_sub(site.address as u32 + 5);
        bytes[0] = 0xE9;
        bytes[1..5].copy_from_slice(&rel.to_le_bytes());
        bytes
    }

    #[test]
    fn stock_bytes_pass() {
        for site in SITES {
            assert_eq!(classify(&site, Some(site.expected), &[]), Prologue::Stock);
        }
    }

    /// Tick and the drop callee hooked first by the telemetry DLL's MinHook
    /// are chained on top of, and the jump target is decoded.
    #[test]
    fn a_minhooked_chainable_site_is_chained() {
        for site in [DROP_CALLEE, ENGINE_TICK] {
            let bytes = minhooked(&site, 0x7000_1000);
            assert_eq!(
                classify(&site, Some(&bytes), &[OWNER]),
                Prologue::Chained {
                    jump_to: 0x7000_1000
                },
                "{}",
                site.name
            );
        }
    }

    /// A backwards jump (detour below the target) decodes too.
    #[test]
    fn chained_jump_target_handles_negative_displacement() {
        const LOW_OWNER: Range<usize> = 0x0010_0000..0x0020_0000;
        let bytes = minhooked(&ENGINE_TICK, 0x0010_0000);
        assert_eq!(
            classify(&ENGINE_TICK, Some(&bytes), &[LOW_OWNER]),
            Prologue::Chained {
                jump_to: 0x0010_0000
            }
        );
    }

    /// A MinHook-shaped jump that lands outside every known hook owner, or
    /// with no owner loaded at all, fails the gate instead of being chained
    /// through.
    #[test]
    fn a_chainable_site_jumping_outside_a_known_module_is_refused() {
        for site in [DROP_CALLEE, ENGINE_TICK] {
            for target in [OWNER.start - 1, OWNER.end, 0x1234_5678] {
                let bytes = minhooked(&site, target);
                let prologue = classify(&site, Some(&bytes), &[OWNER]);
                assert_eq!(
                    prologue,
                    Prologue::UnknownHook { jump_to: target },
                    "{} -> 0x{target:08x}",
                    site.name
                );
                assert!(!prologue.is_usable());
            }
            let bytes = minhooked(&site, OWNER.start);
            assert!(
                matches!(
                    classify(&site, Some(&bytes), &[]),
                    Prologue::UnknownHook { .. }
                ),
                "no telemetry DLL loaded: {}",
                site.name
            );
        }
    }

    /// A PE header yields its image range; anything else yields nothing.
    #[test]
    fn image_range_reads_size_of_image() {
        const BASE: usize = 0x6000_0000;
        let mut header = vec![0u8; 0x100];
        header[..2].copy_from_slice(b"MZ");
        header[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        header[0x80..0x84].copy_from_slice(b"PE\0\0");
        header[0xD0..0xD4].copy_from_slice(&0x0004_2000u32.to_le_bytes());
        let mut mem = FakeMemory::default();
        mem.put(BASE, &header);
        assert_eq!(image_range(&mem, BASE), Some(BASE..BASE + 0x0004_2000));

        header[0x80] = b'X';
        let mut bad = FakeMemory::default();
        bad.put(BASE, &header);
        assert_eq!(image_range(&bad, BASE), None, "no PE signature");
        assert_eq!(image_range(&FakeMemory::default(), BASE), None, "unmapped");
    }

    #[test]
    fn an_e9_on_a_site_nobody_else_hooks_is_a_mismatch() {
        for site in [DISPATCHER, START_ENTITY_MESSAGE] {
            let bytes = minhooked(&site, 0x7000_1000);
            assert!(
                matches!(
                    classify(&site, Some(&bytes), &[OWNER]),
                    Prologue::Mismatch { .. }
                ),
                "{}",
                site.name
            );
        }
    }

    /// A jump that also clobbered bytes after the first five is not a
    /// MinHook hook this DLL can chain.
    #[test]
    fn an_e9_with_a_changed_tail_is_a_mismatch() {
        let mut bytes = minhooked(&ENGINE_TICK, 0x7000_1000);
        bytes[6] = 0x90;
        assert!(matches!(
            classify(&ENGINE_TICK, Some(&bytes), &[OWNER]),
            Prologue::Mismatch { .. }
        ));
    }

    #[test]
    fn any_changed_byte_is_a_mismatch() {
        for site in SITES {
            for i in 0..site.expected.len() {
                let mut bytes = site.expected.to_vec();
                bytes[i] ^= 0x01;
                assert!(
                    !classify(&site, Some(&bytes), &[OWNER]).is_usable(),
                    "{} byte {i}",
                    site.name
                );
            }
        }
    }

    #[test]
    fn unreadable_or_short_is_refused() {
        assert_eq!(classify(&DISPATCHER, None, &[OWNER]), Prologue::Unreadable);
        assert!(!classify(&ENGINE_TICK, Some(&ENGINE_TICK.expected[..5]), &[OWNER]).is_usable());
        assert!(!Prologue::Unreadable.is_usable());
    }

    #[test]
    fn check_all_reads_every_site() {
        let mut mem = FakeMemory::default();
        mem.put(DISPATCHER.address, DISPATCHER.expected)
            .put(DROP_CALLEE.address, &minhooked(&DROP_CALLEE, 0x7000_0000))
            .put(ENGINE_TICK.address, &[0xCC; 13]);
        let results = check_all(&mem, &[OWNER]);
        assert_eq!(results.len(), SITES.len());
        assert_eq!(results[0].1, Prologue::Stock);
        assert!(matches!(results[1].1, Prologue::Chained { .. }));
        assert!(matches!(results[2].1, Prologue::Mismatch { .. }));
        assert_eq!(results[3].1, Prologue::Unreadable);
        assert!(!results.iter().all(|(_, p)| p.is_usable()));
    }

    /// The send side fails closed: with every other site stock, one moved
    /// send-side site makes the whole gate refuse.
    #[test]
    fn a_moved_send_site_fails_the_gate() {
        for moved in [
            START_ENTITY_MESSAGE,
            START_AVATAR_MESSAGE,
            IS_ONLINE,
            GAME_ENTITY_MANAGER_GET,
            BUNDLE_WRITE_U8,
        ] {
            let mut mem = FakeMemory::default();
            for site in SITES {
                if site != moved {
                    mem.put(site.address, site.expected);
                }
            }
            let mut other_build = moved.expected.to_vec();
            other_build[moved.expected.len() - 1] ^= 0xFF;
            mem.put(moved.address, &other_build);
            let results = check_all(&mem, &[]);
            let failed: Vec<&str> = results
                .iter()
                .filter(|(_, p)| !p.is_usable())
                .map(|(s, _)| s.name)
                .collect();
            assert_eq!(failed, [moved.name]);
        }
    }

    /// The pinning sites encode the offsets the send side reads, so a typo
    /// in either place shows up here.
    #[test]
    fn send_pins_agree_with_the_address_constants() {
        let gem = (addresses::GAME_ENTITY_MANAGER as u32).to_le_bytes();
        assert_eq!(GAME_ENTITY_MANAGER_GET.expected[1..5], gem);
        let online = (addresses::CONN_ONLINE as u32).to_le_bytes();
        assert_eq!(IS_ONLINE.expected[4..8], online);
        assert_eq!(
            BUNDLE_WRITE_U8.expected[7] as usize,
            addresses::BUNDLE_RESERVE
        );
        // startAvatarMessage pushes 0 (the local avatar) and calls
        // startEntityMessage: E8 rel32 from the end of the call.
        assert_eq!(
            START_AVATAR_MESSAGE.expected[5] as u32,
            addresses::LOCAL_AVATAR
        );
        let rel = i32::from_le_bytes(START_AVATAR_MESSAGE.expected[8..12].try_into().unwrap());
        let call_end = START_AVATAR_MESSAGE.address as i64 + 12;
        assert_eq!(
            (call_end + i64::from(rel)) as usize,
            addresses::SERVER_CONNECTION_START_ENTITY_MESSAGE
        );
    }

    #[test]
    fn hex_is_spaced_upper_case() {
        assert_eq!(hex(&[0x6A, 0xFF, 0x00]), "6A FF 00");
    }
}
