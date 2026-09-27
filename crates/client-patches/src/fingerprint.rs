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

use crate::addresses;
use crate::memory::MemoryReader;

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
/// 0x01705538`. Not hooked; the send side will call it.
pub const START_ENTITY_MESSAGE: Site = Site {
    name: "ServerConnection::startEntityMessage",
    address: addresses::SERVER_CONNECTION_START_ENTITY_MESSAGE,
    expected: &[
        0x64, 0xA1, 0x00, 0x00, 0x00, 0x00, 0x6A, 0xFF, 0x68, 0x38, 0x55, 0x70, 0x01,
    ],
    may_be_chained: false,
};

/// Every site the gate checks.
pub const SITES: [Site; 4] = [DISPATCHER, DROP_CALLEE, ENGINE_TICK, START_ENTITY_MESSAGE];

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
pub fn classify(site: &Site, actual: Option<&[u8]>) -> Prologue {
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
        return Prologue::Chained { jump_to };
    }
    Prologue::Mismatch {
        actual: actual.to_vec(),
    }
}

/// Read and classify one site.
pub fn check<M: MemoryReader>(mem: &M, site: &Site) -> Prologue {
    classify(
        site,
        mem.read_bytes(site.address, site.expected.len()).as_deref(),
    )
}

/// Read and classify every site in [`SITES`].
pub fn check_all<M: MemoryReader>(mem: &M) -> Vec<(Site, Prologue)> {
    SITES.iter().map(|s| (*s, check(mem, s))).collect()
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
            assert_eq!(classify(&site, Some(site.expected)), Prologue::Stock);
        }
    }

    /// Tick and the drop callee hooked first by the telemetry DLL's MinHook
    /// are chained on top of, and the jump target is decoded.
    #[test]
    fn a_minhooked_chainable_site_is_chained() {
        for site in [DROP_CALLEE, ENGINE_TICK] {
            let bytes = minhooked(&site, 0x7000_1000);
            assert_eq!(
                classify(&site, Some(&bytes)),
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
        let bytes = minhooked(&ENGINE_TICK, 0x0010_0000);
        assert_eq!(
            classify(&ENGINE_TICK, Some(&bytes)),
            Prologue::Chained {
                jump_to: 0x0010_0000
            }
        );
    }

    #[test]
    fn an_e9_on_a_site_nobody_else_hooks_is_a_mismatch() {
        for site in [DISPATCHER, START_ENTITY_MESSAGE] {
            let bytes = minhooked(&site, 0x7000_1000);
            assert!(
                matches!(classify(&site, Some(&bytes)), Prologue::Mismatch { .. }),
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
            classify(&ENGINE_TICK, Some(&bytes)),
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
                    !classify(&site, Some(&bytes)).is_usable(),
                    "{} byte {i}",
                    site.name
                );
            }
        }
    }

    #[test]
    fn unreadable_or_short_is_refused() {
        assert_eq!(classify(&DISPATCHER, None), Prologue::Unreadable);
        assert!(!classify(&ENGINE_TICK, Some(&ENGINE_TICK.expected[..5])).is_usable());
        assert!(!Prologue::Unreadable.is_usable());
    }

    #[test]
    fn check_all_reads_every_site() {
        let mut mem = FakeMemory::default();
        mem.put(DISPATCHER.address, DISPATCHER.expected)
            .put(DROP_CALLEE.address, &minhooked(&DROP_CALLEE, 0x7000_0000))
            .put(ENGINE_TICK.address, &[0xCC; 13]);
        let results = check_all(&mem);
        assert_eq!(results.len(), 4);
        assert_eq!(results[0].1, Prologue::Stock);
        assert!(matches!(results[1].1, Prologue::Chained { .. }));
        assert!(matches!(results[2].1, Prologue::Mismatch { .. }));
        assert_eq!(results[3].1, Prologue::Unreadable);
        assert!(!results.iter().all(|(_, p)| p.is_usable()));
    }

    #[test]
    fn hex_is_spaced_upper_case() {
        assert_eq!(hex(&[0x6A, 0xFF, 0x00]), "6A FF 00");
    }
}
