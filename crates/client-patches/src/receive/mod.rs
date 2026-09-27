//! The receive side: claim the Black Market client methods the stock
//! client drops, on the network thread.
//!
//! Two detours cooperate
//! (`docs/reverse-engineering/findings/black-market-client-io.md` §2):
//!
//! 1. `Client_NetIn_EntityMethodDispatch` (`0x00c6f8f0`) records
//!    `(entity, stream)` in a thread-local for the length of the call, and
//!    restores the previous value when the call returns or unwinds, so a
//!    nested dispatch cannot leave a stale pointer behind.
//! 2. The drop callee (`0x01590f30`), which the dispatcher calls only when
//!    no handler is bound, calls the original first. If it returned a
//!    `MethodDescription` whose name is one of the six `onBM*` methods, and
//!    the recorded entity is the local player, it decodes that method's
//!    arguments from the recorded stream and queues them. It returns the
//!    original result unchanged either way, so every other dropped method
//!    behaves exactly as before.
//!
//! Matching by **name** rather than index means a server-side index slip
//! cannot make the DLL decode some other method's arguments as auction
//! data. Nothing here touches Lua or game state: [`claim`] is portable
//! logic over [`crate::memory::MemoryReader`], and the decoded call goes to
//! [`crate::EVENTS`] for the main thread.

pub mod claim;

#[cfg(all(windows, target_arch = "x86"))]
pub(crate) mod detours;
#[cfg(all(windows, target_arch = "x86"))]
pub(crate) mod stream;
