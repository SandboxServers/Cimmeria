//! The engine layer's log sinks: everything the client's own logging
//! machinery, and the OS around it, can tell us, captured at the point where
//! the client writes it and shipped through the shared producer.
//!
//! The client has five logging paths, and a session's SigNoz stream used to
//! have none of them. This module puts a hook on each:
//!
//! | Path | Target | Where | Notes |
//! |---|---|---|---|
//! | BigWorld `DEBUG_MSG`..`CRITICAL_MSG`, `MF_ASSERT` | `client.bw.message` | inline `0x00a36460` | [`bw_message`]: priority, filtered flag, all callers |
//! | UE3 `GLog` (`debugf`, `warnf`, script `Log()`) | `client.ue3.log` | inline `0x004ce0b0` | [`ue3_log`]: category, suppressed-category flag |
//! | UE3 fatal error (`appErrorf`, `GError`) | `client.ue3.fatal_error` | inline `0x004ce3a0` | [`ue3_fatal`]: the last event of a session |
//! | UE3 `check()` failures | `client.ue3.assert` | inline `0x00486000` | [`ue3_assert`]: survived assertions, expr/file/line |
//! | log4cxx (`SGWDebugLog.log`) | `client.log4cxx.event` | IAT `0x017f0160`/`0x017f0188` | [`log4cxx`]: logger, level, file:line; `unfilter` |
//! | `OutputDebugStringA/W` | `client.os.debug_string` | IAT `0x017ef32c`/`0x017ef230` | [`os_debug`]: what only the OS path sees |
//! | First-chance exceptions | `client.os.exception` | vectored handler | [`exceptions`]: faults, not C++ throws |
//!
//! plus `client.hooks.capabilities`, emitted once after the installs
//! ([`caps`]): which of these went in, and the capture switches.
//!
//! # Budget
//!
//! Every sink rate-limits per distinct message ([`throttle`]): burst 8, then 4
//! a second, the swallowed count riding the next line through. The keys are
//! chosen per sink so a hot message cannot hide a rare one (the format
//! string's address for BigWorld; the digit-collapsed text for log4cxx and
//! debug strings, whose lock traces differ only by a thread id). `firehose`
//! raises the limits to 64/64; nothing removes them.
//!
//! # Switches
//!
//! [`crate::capture`]: `unfilter` lifts the client's own thresholds (the
//! BigWorld filter, the four log4cxx `is*Enabled` answers, the UE3
//! suppress flag on the categories that are logged); `firehose` raises the
//! rate limits.
//!
//! # Safety posture
//!
//! Every read of client memory goes through [`mem`]'s fault-free reader, every
//! detour keeps its Rust code inside `catch_unwind` and forwards every
//! argument, and detours of originals that can throw use the `-unwind` ABI
//! (see the module docs of `inline_hooks`). The inline sites are in the
//! fingerprint gate; the IAT sites are checked one by one against what the
//! import resolves to.

pub mod caps;
pub mod emit;
pub mod gnames;
pub mod mem;
pub mod nesting;
pub mod text;
pub mod throttle;

pub mod bw_message;
pub mod exceptions;
pub mod log4cxx;
pub mod os_debug;
pub mod ue3_assert;
pub mod ue3_fatal;
pub mod ue3_log;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod bw_message_detour;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(in crate::hooks) mod install;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod log4cxx_detours;

use crate::queue::Producer;

/// Install every sink. Called from `hooks::install_all`, after the older hooks
/// and MinHook's init; the capabilities event is emitted once, after the
/// seams have installed too.
pub fn install(_producer: Producer) {
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    unsafe {
        install_inner(&_producer);
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_inner(producer: &Producer) {
    bw_message::install(producer);
    ue3_log::install(producer);
    ue3_fatal::install(producer);
    ue3_assert::install(producer);
    log4cxx::install(producer);
    os_debug::install(producer);
    caps::record(
        "exception_handler",
        if exceptions::register() {
            caps::Outcome::Installed
        } else {
            caps::Outcome::Failed("AddVectoredExceptionHandler")
        },
    );
}

/// Emit `client.hooks.capabilities` from what has been recorded, and write
/// the same to the local log.
pub fn emit_capabilities(_producer: &Producer) {
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    emit_capabilities_x86(_producer);
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
fn emit_capabilities_x86(producer: &Producer) {
    let recorded = caps::snapshot();
    let fields = caps::fields(&recorded, crate::capture::current());
    let mut builder =
        crate::events::ClientNativeEvent::builder(caps::TARGET, caps::level(&recorded));
    let mut line = String::from("capabilities:");
    for (k, v) in fields {
        line.push_str(&format!(" {k}={v}"));
        builder = builder.field(&k, v);
    }
    crate::log::line(line);
    producer.try_emit(builder);
}

#[cfg(test)]
mod tests {
    /// Every inline-hooked address is a fingerprinted site, so a build whose
    /// bytes differ there installs nothing.
    #[test]
    fn every_inline_sink_is_fingerprinted() {
        let hooked = [
            super::bw_message::ADDR_BW_MESSAGE,
            super::ue3_log::ADDR_REDIRECTOR_SERIALIZE,
            super::ue3_fatal::ADDR_ERROR_SERIALIZE,
            super::ue3_assert::ADDR_CHECK_FAILED,
        ];
        for addr in hooked {
            assert!(
                crate::fingerprint::CODE_SITES
                    .iter()
                    .any(|s| s.address == addr),
                "0x{addr:08x} is hooked but not fingerprinted"
            );
        }
    }
}
