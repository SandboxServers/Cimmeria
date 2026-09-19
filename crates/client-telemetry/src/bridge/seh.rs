//! Inner-tier structured-exception guard (issue #686 scope 1).
//!
//! This is the **inner** tier of the two-tier crash scheme the #686
//! spike settled on. Every native dispatch body — `mem_write`,
//! `call_native`, and the dynamic-hook capture path — runs inside
//! [`guard`], so a hardware fault (access violation, illegal
//! instruction, …) becomes a JSON-RPC error carrying the `Exception`
//! instead of tearing down the process. The **outer** tier
//! ([`super::crash`]) still owns `SetUnhandledExceptionFilter` +
//! minidump for what the inner tier can't safely absorb (stack
//! overflow, severe corruption); the two are separate mechanisms and
//! this module does not touch the outer one.
//!
//! # Drop-across-unwind rule (#686 spike, non-negotiable)
//!
//! A forced `__except` unwind back to the catch frame is **not**
//! guaranteed to run Rust `Drop` for the intervening stack frames.
//! Practical rule: **never hold a lock across a guarded call** — not a
//! `MutexGuard`, not the Lua-state lock, not a scoped
//! memory-protection guard. Acquire the lock, copy out what you need,
//! drop it, *then* [`guard`] the body. A fault mid-hold would
//! otherwise deadlock the next dispatch — worse than the crash it was
//! meant to absorb. Callers in this crate follow the acquire/copy/drop
//! pattern before every `guard(...)`.
//!
//! # Stack overflow is out of scope here
//!
//! Per the spike, `STATUS_STACK_OVERFLOW` cannot be caught reliably by
//! `__except` (the handler needs stack that may no longer exist) and is
//! unrecoverable. We deliberately do **not** try to route it through
//! [`guard`]; the outer tier's vectored handler +
//! `SetThreadStackGuarantee` deal with it (`super::crash`).

use serde_json::{json, Value};

/// A hardware fault absorbed by the inner-tier guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SehFault {
    /// Raw STATUS_* code (e.g. `0xC000_0005` access violation).
    pub raw_code: u32,
    /// Faulting instruction address.
    pub address: usize,
    /// Human description from microseh's `ExceptionCode`.
    pub description: String,
}

impl SehFault {
    /// The `Exception` object attached to a `NATIVE_FAULT` JSON-RPC
    /// error's `data` field, so the agent can correlate the fault with a
    /// Ghidra address.
    pub fn to_exception_json(&self) -> Value {
        json!({
            "Exception": {
                "code": format!("{:#010x}", self.raw_code),
                "address": format!("{:#x}", self.address),
                "description": self.description,
            }
        })
    }

    /// One-line message for the JSON-RPC error `message` field.
    pub fn message(&self) -> String {
        format!(
            "native fault {:#010x} at {:#x}: {}",
            self.raw_code, self.address, self.description
        )
    }
}

/// Run `f` under a structured-exception guard, converting a hardware
/// fault into `Err(SehFault)` instead of terminating the process.
///
/// See the module docs for the **Drop-across-unwind rule**: the caller
/// must not hold any lock across this call.
#[cfg(windows)]
pub fn guard<R>(f: impl FnMut() -> R) -> Result<R, SehFault> {
    microseh::try_seh(f).map_err(|e| SehFault {
        raw_code: e.raw_code(),
        address: e.address() as usize,
        description: e.code().to_string(),
    })
}

/// Off-Windows passthrough so the pure layers and the host unit tests
/// compile everywhere. There is no SEH off Windows, and the native
/// bodies are stubs on non-i686-Windows targets anyway, so there is
/// nothing to guard.
#[cfg(not(windows))]
pub fn guard<R>(mut f: impl FnMut() -> R) -> Result<R, SehFault> {
    Ok(f())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The happy path returns the closure's value unchanged. On the
    /// host this exercises the passthrough; on the MSVC test runner it
    /// exercises the real `try_seh` fault-free path.
    #[test]
    fn guard_returns_value_on_success() {
        let r = guard(|| 40 + 2);
        assert_eq!(r, Ok(42));
    }

    /// The `Exception` payload is the fixed shape the agent keys on.
    #[test]
    fn fault_serializes_exception_object() {
        let f = SehFault {
            raw_code: 0xC000_0005,
            address: 0x0141_6ec0,
            description: "access violation".to_string(),
        };
        let v = f.to_exception_json();
        assert_eq!(v["Exception"]["code"], "0xc0000005");
        assert_eq!(v["Exception"]["address"], "0x1416ec0");
        assert!(f.message().contains("0xc0000005"));
        assert!(f.message().contains("0x1416ec0"));
    }
}
