//! The send side: native Lua functions that send the Black Market cell
//! methods 61–66 to the server.
//!
//! The stock client never learned to send these (its NetOut emitters were
//! shelved), so the DLL registers a global table [`NATIVE_TABLE`] in the UI
//! Lua and the overlay calls it:
//!
//! | Lua call | Cell method |
//! |---|---|
//! | `CimmeriaBMNative.search(opts)` | `BMSearch` (61) |
//! | `CimmeriaBMNative.create(itemInstanceId, startingPrice, buyoutPrice, auctionLength)` | `BMCreateAuction` (62) |
//! | `CimmeriaBMNative.bid(sequenceId, bidAmount)` | `BMPlaceBid` (63) |
//! | `CimmeriaBMNative.cancel(sequenceId)` | `BMCancelAuction` (64) |
//! | `CimmeriaBMNative.watch(itemDefId, enable)` | `BMStartWatchingItem` (65) or `BMStopWatchingItem` (66) |
//! | `CimmeriaBMNative.techCompetency(itemDefId)` | none: always `nil` in this build (D7) |
//!
//! A send native returns `true` once the message is on the engine's
//! outgoing bundle, or `nil, reason` with a [`Refusal::reason`]. It never
//! raises a Lua error on bad input. The argument rules are in [`args`], the
//! registration in [`register`].
//!
//! How a message is sent, per
//! `docs/reverse-engineering/findings/black-market-client-io.md` §1: on the
//! main thread, `ServerConnection::startEntityMessage(conn, 0x3D, 0)` starts
//! an extended method message for the local avatar and returns the bundle;
//! `bundle->reserve(1)` takes the sub-index byte (`index - 61`) and
//! `bundle->reserve(n)` the payload, which `cimmeria-patch-wire` encodes
//! exactly as the server decodes it. The engine side is the [`Engine`]
//! trait, so everything above it is tested on the host.

pub mod args;
pub mod register;

#[cfg(all(windows, target_arch = "x86"))]
pub(crate) mod engine;
#[cfg(all(windows, target_arch = "x86"))]
pub(crate) mod natives;

#[cfg(test)]
mod tests;

use std::panic::{catch_unwind, AssertUnwindSafe};

use cimmeria_patch_wire::black_market::CellMethod;
use cimmeria_patch_wire::Encode;

use crate::counters::{bump, is_log_worthy, Counters};
use crate::deliver::lua_stack::LuaStack;
use crate::log;

/// The global table the DLL creates. It is the DLL's own: the overlay's
/// `CimmeriaBM` table is never touched from this side.
pub const NATIVE_TABLE: &str = "CimmeriaBMNative";

/// `CimmeriaBMNative.version`: the DLL's version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Every refusal and send up to this count is logged; after it, only
/// powers of ten. Sends follow button presses, so a session rarely gets
/// near it; a script stuck in a loop cannot fill the log.
const LOG_EVERY_UP_TO: u64 = 100;

/// One function of [`NATIVE_TABLE`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Native {
    /// `search(opts)`.
    Search,
    /// `create(itemInstanceId, startingPrice, buyoutPrice, auctionLength)`.
    Create,
    /// `bid(sequenceId, bidAmount)`.
    Bid,
    /// `cancel(sequenceId)`.
    Cancel,
    /// `watch(itemDefId, enable)`.
    Watch,
    /// `techCompetency(itemDefId)`.
    TechCompetency,
}

impl Native {
    /// All six, in table order.
    pub const ALL: [Self; 6] = [
        Self::Search,
        Self::Create,
        Self::Bid,
        Self::Cancel,
        Self::Watch,
        Self::TechCompetency,
    ];

    /// The field name in [`NATIVE_TABLE`].
    pub const fn lua_name(self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::Create => "create",
            Self::Bid => "bid",
            Self::Cancel => "cancel",
            Self::Watch => "watch",
            Self::TechCompetency => "techCompetency",
        }
    }
}

/// Why a send native did not send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Called off the game's main thread, where the engine's bundle must
    /// not be touched.
    NotMainThread,
    /// An argument is missing, of the wrong type, out of range, or too long
    /// to encode. The text says which, for the log.
    BadArgs(String),
    /// No connection to the server, or no local player yet.
    Offline(&'static str),
    /// The engine failed: a null return, or a fault or C++ exception caught
    /// by the structured-exception guard.
    EngineError(String),
}

impl Refusal {
    /// The second return value the overlay sees: one of `"not_main_thread"`,
    /// `"bad_args"`, `"offline"` and `"engine_error"`.
    pub const fn reason(&self) -> &'static str {
        match self {
            Self::NotMainThread => "not_main_thread",
            Self::BadArgs(_) => "bad_args",
            Self::Offline(_) => "offline",
            Self::EngineError(_) => "engine_error",
        }
    }

    /// The detail for the log.
    pub fn detail(&self) -> &str {
        match self {
            Self::NotMainThread => "called off the main thread",
            Self::BadArgs(d) | Self::EngineError(d) => d,
            Self::Offline(d) => d,
        }
    }
}

/// The engine calls a send makes. In `SGW.exe` this is
/// [`engine::EngineSender`]; the tests use a recorder.
pub trait Engine {
    /// Whether this is the game's main thread.
    fn on_main_thread(&self) -> bool;

    /// Start an extended entity message for the local avatar, then write
    /// `sub_index` and `payload` to it. [`Refusal::Offline`] when there is
    /// no connection or player to send as, [`Refusal::EngineError`] when
    /// the engine fails.
    fn send(&mut self, sub_index: u8, payload: &[u8]) -> Result<(), Refusal>;
}

/// A send that went out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sent {
    /// The cell method.
    pub method: CellMethod,
    /// The payload size, without the sub-index byte.
    pub payload_len: usize,
}

/// Run `native` as a Lua C function: read its arguments from `lua` (index
/// 1 is the first), send, record the outcome, and push the return values.
/// Returns how many values were pushed, which is the C function's return.
pub fn run<L: LuaStack, E: Engine>(
    lua: &mut L,
    native: Native,
    engine: &mut E,
    counters: &Counters,
) -> i32 {
    if native == Native::TechCompetency {
        // D7: no safe read of the item definition's tech competency has
        // been verified for this build, so the answer is always "unknown".
        // See `docs/architecture/client-patches.md`.
        let n = bump(&counters.tech_competency_nil);
        if is_log_worthy(n) {
            log::line(format_args!(
                "{NATIVE_TABLE}.techCompetency returns nil: no verified native getter in this build (#{n})"
            ));
        }
        lua.push_nil();
        return 1;
    }
    match send(lua, native, engine) {
        Ok(sent) => {
            let n = bump(&counters.sent);
            if n <= LOG_EVERY_UP_TO || is_log_worthy(n) {
                log::line(format_args!(
                    "sent {} (cell method {}, sub-index {}, {}-byte payload) (#{n})",
                    sent.method.name(),
                    sent.method.index(),
                    sent.method.sub_index(),
                    sent.payload_len
                ));
            }
            lua.push_boolean(true);
            1
        }
        Err(refusal) => {
            let n = bump(&counters.send_refused);
            if n <= LOG_EVERY_UP_TO || is_log_worthy(n) {
                log::line(format_args!(
                    "{NATIVE_TABLE}.{} refused: {} ({}) (#{n})",
                    native.lua_name(),
                    refusal.reason(),
                    refusal.detail()
                ));
            }
            lua.push_nil();
            lua.push_string(refusal.reason());
            2
        }
    }
}

/// Check the thread, read and encode the arguments, and hand the message
/// to the engine.
pub fn send<L: LuaStack, E: Engine>(
    lua: &mut L,
    native: Native,
    engine: &mut E,
) -> Result<Sent, Refusal> {
    if !engine.on_main_thread() {
        return Err(Refusal::NotMainThread);
    }
    let call = args::read(lua, native).map_err(Refusal::BadArgs)?;
    let payload = call
        .to_bytes()
        .map_err(|e| Refusal::BadArgs(e.to_string()))?;
    let method = call.method();
    // No Lua call happens in here, so nothing but a Rust panic can unwind
    // to this `catch_unwind`: engine faults and C++ exceptions stop at the
    // engine's structured-exception guard.
    let sent = catch_unwind(AssertUnwindSafe(|| {
        engine.send(method.sub_index(), &payload)
    }));
    match sent {
        Ok(result) => result?,
        Err(_) => return Err(Refusal::EngineError("the send panicked".into())),
    }
    Ok(Sent {
        method,
        payload_len: payload.len(),
    })
}
