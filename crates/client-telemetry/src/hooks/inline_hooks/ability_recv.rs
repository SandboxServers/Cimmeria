//! `client.ability.recv` from inside the `onEntityMethod` detour.
//!
//! `EntityManager::onEntityMethod` (`0x00dd2b80`) runs on the game thread,
//! from `EntityMethodMessage::process` (`0x01561ac0`), with the message's
//! arguments in a `MemoryOStream` that `SGWMessageQueue` filled on the
//! network thread. The bytes are read here, through `ReadProcessMemory`,
//! before the original runs, and the cursor is never moved, so the game
//! reads exactly what it would have. Finding the bytes (both stream
//! layouts) and deciding what to emit is the portable
//! `hooks::ability_trace::recv_stream`; the decoding is
//! `hooks::ability_trace::recv`.

use std::ffi::c_void;

use crate::hooks::ability_trace::recv_stream;
use crate::hooks::emit::emit;
use crate::hooks::entity_trace::map::LiveMem;

/// Decode and emit the message, if it is one of ours; a candidate that
/// could not be decoded is emitted as `client.ability.recv_skipped`.
/// Called before the original; reads only.
pub(super) fn report(mgr: *mut c_void, id: i32, msg_id: u32, stream: *mut c_void) {
    if let Some((target, level, fields)) =
        recv_stream::observe(&LiveMem, mgr as u32, id, msg_id, stream as u32)
    {
        emit(target, level, fields);
    }
}
