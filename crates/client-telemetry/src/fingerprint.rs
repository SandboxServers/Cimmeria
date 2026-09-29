//! The build fingerprint gate.
//!
//! Every hook address in this DLL belongs to one `SGW.exe` build (the QA
//! build, image base `0x00400000`, ASLR off). Before hooking anything, the
//! bootstrap compares the bytes at every function it hooks or calls, and
//! the value in every vtable slot it swaps, with what that build has
//! there. On any mismatch it hooks nothing: a different build, or a
//! process that is not `SGW.exe` at all, costs the hook telemetry but
//! never the game. The uploader still runs, so the mismatch reaches SigNoz
//! as a `client.hooks.fingerprint` warning.
//!
//! `FEngineLoop::Tick` and the drop callee are also hooked by the
//! client-patches DLL. Either DLL may get there first, so those two sites
//! accept a MinHook jump into the other DLL (see
//! `cimmeria-client-hookgate`).
//!
//! The IAT slots are checked separately, one at a time, when they are
//! swapped (`hooks::iat_hooks`): a slot must hold the address its import
//! resolves to, which is not a build property.

use core::ops::Range;

pub use cimmeria_client_hookgate::{classify, hex, Prologue, Site};

/// `FFullScreenMovieBink::Tick`: `xorps xmm1, xmm1; sub esp, 8; push esi;
/// mov esi, ecx`.
pub const BINK_TICK: Site = code_site(
    "FFullScreenMovieBink::Tick",
    0x0050_bbc0,
    &[
        0x0F, 0x57, 0xC9, 0x83, 0xEC, 0x08, 0x56, 0x8B, 0xF1, 0xF3, 0x0F, 0x10,
    ],
);

/// `FArchiveAsync::Serialize`: `sub esp, 0x10; push ebx; mov ebx,
/// [esp+0x1c]; push ebp`.
pub const ARCHIVE_ASYNC_SERIALIZE: Site = code_site(
    "FArchiveAsync::Serialize",
    0x004c_7ae0,
    &[
        0x83, 0xEC, 0x10, 0x53, 0x8B, 0x5C, 0x24, 0x1C, 0x55, 0x8B, 0x2D, 0x54,
    ],
);

/// `UWorld::UpdateLevelStreamingInner`: `sub esp, 0x28; push ebx; push
/// ebp; push esi; mov esi, [esp+0x38]`.
pub const UPDATE_LEVEL_STREAMING_INNER: Site = code_site(
    "UWorld::UpdateLevelStreamingInner",
    0x0054_e9c0,
    &[
        0x83, 0xEC, 0x28, 0x53, 0x55, 0x56, 0x8B, 0x74, 0x24, 0x38, 0x57, 0x33,
    ],
);

/// `UObject::StaticLoadObject`: `push ebp; mov ebp, esp; push -1; push
/// 0x01682387; mov eax, fs:[0]`.
pub const STATIC_LOAD_OBJECT: Site = code_site(
    "UObject::StaticLoadObject",
    0x004a_8e10,
    &[
        0x55, 0x8B, 0xEC, 0x6A, 0xFF, 0x68, 0x87, 0x23, 0x68, 0x01, 0x64, 0xA1,
    ],
);

/// `APlayerController::execConsoleCommand`: `push -1; push 0x0168a690;
/// mov eax, fs:[0]`.
pub const CONSOLE_COMMAND: Site = code_site(
    "APlayerController::execConsoleCommand",
    0x0053_9850,
    &[
        0x6A, 0xFF, 0x68, 0x90, 0xA6, 0x68, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `GameBeing::onStateFieldUpdate`: `push -1; push 0x01708552; mov eax,
/// fs:[0]`.
pub const STATE_FIELD_UPDATE: Site = code_site(
    "GameBeing::onStateFieldUpdate",
    0x00e0_1c90,
    &[
        0x6A, 0xFF, 0x68, 0x52, 0x85, 0x70, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `USGWAnimNotify_Event::Notify` (A): `push -1; push 0x01710399; mov eax,
/// fs:[0]`.
pub const ANIM_NOTIFY_A: Site = code_site(
    "USGWAnimNotify_Event::Notify (A)",
    0x00e9_74b0,
    &[
        0x6A, 0xFF, 0x68, 0x99, 0x03, 0x71, 0x01, 0x64, 0xA1, 0x00, 0x00, 0x00,
    ],
);

/// `USGWAnimNotify_Event::Notify` (B): `mov eax, fs:[0]; push -1; push
/// 0x0171028b`.
pub const ANIM_NOTIFY_B: Site = code_site(
    "USGWAnimNotify_Event::Notify (B)",
    0x00e9_7070,
    &[
        0x64, 0xA1, 0x00, 0x00, 0x00, 0x00, 0x6A, 0xFF, 0x68, 0x8B, 0x02, 0x71,
    ],
);

/// CME event-registry lookup, hooked for `client.cme.event`: `mov eax,
/// [esp+4]; sub esp, 8; push ebx; push ebp; push esi; push edi`. It was
/// fingerprinted before as "EventSignal lookup by name" for the removed
/// CME subscriber install, together with `0x0155f790`, `0x00a5c150` and
/// `0x00e04570`, which nothing calls any more.
pub const CME_EVENT_FACTORY: Site = code_site(
    "CME event registry lookup",
    0x00a5_c0f0,
    &[
        0x8B, 0x44, 0x24, 0x04, 0x83, 0xEC, 0x08, 0x53, 0x55, 0x56, 0x57, 0x8B,
    ],
);

/// Every function the gate checks.
pub const CODE_SITES: [Site; 11] = [
    cimmeria_client_hookgate::ENGINE_TICK,
    cimmeria_client_hookgate::DROP_CALLEE,
    BINK_TICK,
    ARCHIVE_ASYNC_SERIALIZE,
    UPDATE_LEVEL_STREAMING_INNER,
    STATIC_LOAD_OBJECT,
    CONSOLE_COMMAND,
    STATE_FIELD_UPDATE,
    ANIM_NOTIFY_A,
    ANIM_NOTIFY_B,
    CME_EVENT_FACTORY,
];

const fn code_site(name: &'static str, address: usize, expected: &'static [u8]) -> Site {
    Site {
        name,
        address,
        expected,
        may_be_chained: false,
    }
}

/// A vtable slot the DLL swaps, and the function pointer it holds in this
/// build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotSite {
    /// Name for the log.
    pub name: &'static str,
    /// Address of the slot in `.rdata`.
    pub address: usize,
    /// The function pointer the slot holds.
    pub expected: u32,
}

/// `CEGUI::DefaultLogger::logEvent`. The vtable's RTTI locator pointer is
/// at `0x01ac1ba8`, so slot 0 (the destructor) is `0x01ac1bac` and slot 1
/// is `0x01ac1bb0`, which holds `logEvent` at `0x012129e0` (`ret 8`).
pub const CEGUI_LOG_EVENT_SLOT: SlotSite = SlotSite {
    name: "CEGUI::DefaultLogger::logEvent slot",
    address: 0x01ac_1bb0,
    expected: 0x0121_29e0,
};

/// `AActor::Tick`, AActor vtable slot 88.
pub const AACTOR_TICK_SLOT: SlotSite = SlotSite {
    name: "AActor::Tick slot",
    address: 0x0183_c56c,
    expected: 0x005e_4200,
};

/// `USequence::UpdateOp`, USequence vtable slot 84.
pub const USEQUENCE_UPDATE_OP_SLOT: SlotSite = SlotSite {
    name: "USequence::UpdateOp slot",
    address: 0x0185_4bd4,
    expected: 0x006c_61c0,
};

/// Every vtable slot the gate checks.
pub const SLOT_SITES: [SlotSite; 3] = [
    CEGUI_LOG_EVENT_SLOT,
    AACTOR_TICK_SLOT,
    USEQUENCE_UPDATE_OP_SLOT,
];

/// What a vtable slot turned out to hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotState {
    /// The expected function pointer.
    Stock,
    /// Some other value.
    Mismatch {
        /// The value found.
        actual: u32,
    },
    /// The slot could not be read.
    Unreadable,
}

/// The gate's verdict over every site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Each function, in [`CODE_SITES`] order.
    pub code: Vec<(Site, Prologue)>,
    /// Each vtable slot, in [`SLOT_SITES`] order.
    pub slots: Vec<(SlotSite, SlotState)>,
}

impl Report {
    /// Whether every site matched, so the hooks may go in.
    pub fn is_usable(&self) -> bool {
        self.code.iter().all(|(_, p)| p.is_usable())
            && self.slots.iter().all(|(_, s)| *s == SlotState::Stock)
    }

    /// One log line per site, in the client-patches log's wording.
    pub fn lines(&self) -> Vec<String> {
        let code = self.code.iter().map(|(site, prologue)| {
            let head = format!("{} at 0x{:08x}", site.name, site.address);
            match prologue {
                Prologue::Stock => format!("{head}: stock"),
                Prologue::Chained { jump_to } => {
                    format!("{head}: already hooked (jump to 0x{jump_to:08x}), chaining")
                }
                Prologue::UnknownHook { jump_to } => format!(
                    "{head}: already hooked by a jump to 0x{jump_to:08x}, outside every known \
                     hook owner; not chaining"
                ),
                Prologue::Mismatch { actual } => format!(
                    "{head}: expected {}, found {}",
                    hex(site.expected),
                    hex(actual)
                ),
                Prologue::Unreadable => format!("{head}: unreadable"),
            }
        });
        let slots = self.slots.iter().map(|(slot, state)| {
            let head = format!("{} at 0x{:08x}", slot.name, slot.address);
            match state {
                SlotState::Stock => format!("{head}: stock"),
                SlotState::Mismatch { actual } => format!(
                    "{head}: expected 0x{:08x}, found 0x{actual:08x}",
                    slot.expected
                ),
                SlotState::Unreadable => format!("{head}: unreadable"),
            }
        });
        code.chain(slots).collect()
    }

    /// Per-site verdicts for the `client.hooks.fingerprint` event: site
    /// name to `stock` / `chained` / `unknown_hook` / `mismatch` /
    /// `unreadable`.
    pub fn verdicts(&self) -> Vec<(&'static str, &'static str)> {
        let code = self.code.iter().map(|(site, prologue)| {
            let verdict = match prologue {
                Prologue::Stock => "stock",
                Prologue::Chained { .. } => "chained",
                Prologue::UnknownHook { .. } => "unknown_hook",
                Prologue::Mismatch { .. } => "mismatch",
                Prologue::Unreadable => "unreadable",
            };
            (site.name, verdict)
        });
        let slots = self.slots.iter().map(|(slot, state)| {
            let verdict = match state {
                SlotState::Stock => "stock",
                SlotState::Mismatch { .. } => "mismatch",
                SlotState::Unreadable => "unreadable",
            };
            (slot.name, verdict)
        });
        code.chain(slots).collect()
    }
}

/// Check every site. `read(addr, len)` returns the bytes at `addr`, or
/// `None` if any is unreadable; `hook_owners` are the images a chainable
/// site's jump may land in (the loaded client-patches DLL).
pub fn check(
    read: impl Fn(usize, usize) -> Option<Vec<u8>>,
    hook_owners: &[Range<usize>],
) -> Report {
    let code = CODE_SITES
        .iter()
        .map(|site| {
            let actual = read(site.address, site.expected.len());
            (*site, classify(site, actual.as_deref(), hook_owners))
        })
        .collect();
    let slots = SLOT_SITES
        .iter()
        .map(|slot| {
            let state = match read(slot.address, 4)
                .and_then(|b| <[u8; 4]>::try_from(b.as_slice()).ok())
                .map(u32::from_le_bytes)
            {
                None => SlotState::Unreadable,
                Some(v) if v == slot.expected => SlotState::Stock,
                Some(actual) => SlotState::Mismatch { actual },
            };
            (*slot, state)
        })
        .collect();
    Report { code, slots }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A fake address space holding exactly the QA build's bytes at every
    /// site, which individual tests then disturb.
    fn stock_build() -> HashMap<usize, Vec<u8>> {
        let mut mem = HashMap::new();
        for site in CODE_SITES {
            mem.insert(site.address, site.expected.to_vec());
        }
        for slot in SLOT_SITES {
            mem.insert(slot.address, slot.expected.to_le_bytes().to_vec());
        }
        mem
    }

    fn reader(mem: &HashMap<usize, Vec<u8>>) -> impl Fn(usize, usize) -> Option<Vec<u8>> + '_ {
        move |addr, len| {
            mem.get(&addr)
                .filter(|b| b.len() >= len)
                .map(|b| b[..len].to_vec())
        }
    }

    #[test]
    fn the_stock_build_passes() {
        let mem = stock_build();
        let report = check(reader(&mem), &[]);
        assert!(report.is_usable(), "{:#?}", report.lines());
        assert_eq!(report.code.len(), CODE_SITES.len());
        assert_eq!(report.slots.len(), SLOT_SITES.len());
    }

    /// A process that is not `SGW.exe` (the test host, another build) has
    /// nothing readable at these addresses: nothing is hooked.
    #[test]
    fn an_empty_address_space_fails_closed() {
        let report = check(|_, _| None, &[]);
        assert!(!report.is_usable());
        assert!(report.lines().iter().all(|l| l.ends_with("unreadable")));
    }

    #[test]
    fn one_changed_byte_anywhere_fails_the_whole_gate() {
        for site in CODE_SITES {
            let mut mem = stock_build();
            mem.get_mut(&site.address).unwrap()[3] ^= 0xFF;
            assert!(!check(reader(&mem), &[]).is_usable(), "{}", site.name);
        }
    }

    #[test]
    fn a_moved_vtable_slot_fails_the_gate() {
        for slot in SLOT_SITES {
            let mut mem = stock_build();
            mem.insert(slot.address, 0x1234_5678u32.to_le_bytes().to_vec());
            let report = check(reader(&mem), &[]);
            assert!(!report.is_usable(), "{}", slot.name);
            assert!(report
                .lines()
                .iter()
                .any(|l| l.contains("expected 0x") && l.contains("found 0x12345678")));
        }
    }

    /// The client-patches DLL got to Tick and the drop callee first: its
    /// MinHook jumps are chained, but only into its own image.
    #[test]
    fn a_client_patches_hook_on_a_shared_site_is_chained() {
        const PATCHES: Range<usize> = 0x6000_0000..0x6010_0000;
        let mut mem = stock_build();
        for site in [
            cimmeria_client_hookgate::ENGINE_TICK,
            cimmeria_client_hookgate::DROP_CALLEE,
        ] {
            let bytes = mem.get_mut(&site.address).unwrap();
            let rel = 0x6000_1000u32.wrapping_sub(site.address as u32 + 5);
            bytes[0] = 0xE9;
            bytes[1..5].copy_from_slice(&rel.to_le_bytes());
        }
        let report = check(reader(&mem), &[PATCHES]);
        assert!(report.is_usable(), "{:#?}", report.lines());
        assert_eq!(
            report
                .verdicts()
                .iter()
                .filter(|(_, v)| *v == "chained")
                .count(),
            2
        );
        assert!(!check(reader(&mem), &[]).is_usable(), "owner not loaded");
    }

    /// The same jump on a site only this DLL hooks is not a chain.
    #[test]
    fn a_jump_on_a_telemetry_only_site_is_a_mismatch() {
        let mut mem = stock_build();
        mem.get_mut(&BINK_TICK.address).unwrap()[0] = 0xE9;
        let everywhere: Range<usize> = 0..usize::MAX;
        assert!(!check(reader(&mem), &[everywhere]).is_usable());
    }

    /// The CEGUI slot is slot 1 of the vtable, not the RTTI locator's
    /// neighbour (the destructor) the old anchor swapped.
    #[test]
    fn cegui_slot_is_log_event_not_the_destructor() {
        assert_eq!(CEGUI_LOG_EVENT_SLOT.address, 0x01ac_1ba8 + 2 * 4);
        assert_eq!(CEGUI_LOG_EVENT_SLOT.expected, 0x0121_29e0);
    }

    #[test]
    fn verdicts_name_every_site() {
        let mem = stock_build();
        let verdicts = check(reader(&mem), &[]).verdicts();
        assert_eq!(verdicts.len(), CODE_SITES.len() + SLOT_SITES.len());
        assert!(verdicts.iter().all(|(_, v)| *v == "stock"));
    }

    #[test]
    fn every_code_site_can_be_chained_over_by_minhook() {
        // MinHook needs at least five bytes, and a chained site compares
        // what follows them.
        for site in CODE_SITES {
            assert!(site.expected.len() > 5, "{}", site.name);
        }
    }
}
