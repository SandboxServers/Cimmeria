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

mod code_sites;
pub use code_sites::*;

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

/// `USeqAct_Interp::Activated`, slot 85 of the class's vtable at
/// `0x018ad494` (`client.engine.matinee`).
pub const MATINEE_ACTIVATED_SLOT: SlotSite = SlotSite {
    name: "USeqAct_Interp::Activated slot",
    address: 0x018a_d5e8,
    expected: 0x007b_06a0,
};

/// `USeqAct_Interp::DeActivated`, slot 86.
pub const MATINEE_DEACTIVATED_SLOT: SlotSite = SlotSite {
    name: "USeqAct_Interp::DeActivated slot",
    address: 0x018a_d5ec,
    expected: 0x007a_6730,
};

/// `FNxOutputStream::reportError`, slot 0 of the vtable at `0x01839d94`
/// (`client.physx.error`).
pub const PHYSX_REPORT_ERROR_SLOT: SlotSite = SlotSite {
    name: "FNxOutputStream::reportError slot",
    address: 0x0183_9d94,
    expected: 0x0055_c5e0,
};

/// `FNxOutputStream::reportAssertViolation`, slot 1.
pub const PHYSX_REPORT_ASSERT_SLOT: SlotSite = SlotSite {
    name: "FNxOutputStream::reportAssertViolation slot",
    address: 0x0183_9d98,
    expected: 0x0055_c5d0,
};

/// Every vtable slot the gate checks.
pub const SLOT_SITES: [SlotSite; 7] = [
    CEGUI_LOG_EVENT_SLOT,
    AACTOR_TICK_SLOT,
    USEQUENCE_UPDATE_OP_SLOT,
    MATINEE_ACTIVATED_SLOT,
    MATINEE_DEACTIVATED_SLOT,
    PHYSX_REPORT_ERROR_SLOT,
    PHYSX_REPORT_ASSERT_SLOT,
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
mod tests;
