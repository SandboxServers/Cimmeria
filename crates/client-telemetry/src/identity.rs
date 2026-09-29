//! The client↔server join key: the local player's entity id.
//!
//! Read-only, no hooks, no fingerprint gate — just three chained reads
//! of the client's own live memory through `GameEntityManager`, the
//! same accessor pattern used by several independent RE findings
//! (never a heap-layout guess made for this task):
//!
//! ```text
//! g_EntityManager           (fixed VA, holds a pointer)   0x01ef244c
//!   -> [+0x08] ServerConnection*
//!        -> [+0x16c] playerEntityID_   (u32, 0 = none)
//! ```
//!
//! - `0x01ef244c` is `g_EntityManager`
//!   ([`docs/reverse-engineering/address-map.md`], confirmed singleton
//!   set in `EntityManager::EntityManager`).
//! - `[+0x08] = ServerConnection*` is confirmed by
//!   [`docs/reverse-engineering/findings/black-market-client-io.md`]
//!   §1 (`conn = [[0x01ef244c] + 0x08]`, used by the BM send path).
//! - `[+0x16c] = playerEntityID_` is confirmed by
//!   [`docs/reverse-engineering/findings/system-protocol-wire-formats.md`]'s
//!   `ServerConnection` field map, with assembly evidence at two
//!   independent sites: `RESET_ENTITIES`'s handler clears it
//!   (`MOV [ESI+0x16c], EBX`) and `ServerConnection::createBasePlayer`
//!   reads it back out to pass as `entityId` to
//!   `handler->onCreateBasePlayer` (`MOV EAX,[EDI+0x16c]`). The field's
//!   own contract is "0 if none" — which is exactly the value
//!   `RESET_ENTITIES` sets it to, so a `0` read here means "not in
//!   world yet," not a wrong offset.
//!
//! No inline hook, no vtable swap, no IAT slot — just a read of the
//! client's own data through
//! [`cimmeria_client_hookgate::os::read_bytes`], which goes through
//! `ReadProcessMemory` and reports an unmapped page as "unreadable"
//! rather than raising an access violation. A wrong offset on a
//! different build yields `None` (or a stray entity id that the
//! server-side join simply won't match against anything), never a
//! crash — there is no fingerprint gate to bypass here because there
//! is nothing to hook.
//!
//! Account name and the server address are **not** implemented here.
//! Neither survives to any client-side memory this pass could verify
//! statically:
//!
//! - The `Account` entity (`docs/reverse-engineering/findings/entity-types-wire-formats.md`
//!   §1) never sends `AccountName` to the client — its only base
//!   properties are `characterList` and `activePlayerID`. The name is
//!   used once, inside the SOAP login handshake, and never re-appears
//!   on the wire or in a documented client-side cache.
//! - The server address would need a hook on the connect path
//!   (`WSAConnect`/`connect` in `ws2_32.dll`) whose IAT slot in this
//!   exact build was not resolved in this pass — see the seam survey
//!   (`docs/reverse-engineering/findings/client-telemetry-seam-survey.md`,
//!   "Connection lifecycle" section) for the follow-up.
//!
//! Both are left out per the project's "verify or leave it out" rule
//! rather than guessed at.

/// `g_EntityManager` — see module docs.
const ADDR_GAME_ENTITY_MANAGER: usize = 0x01ef_244c;
/// `GameEntityManager` instance `+0x08` = `ServerConnection*`.
const OFFSET_GEM_SERVER_CONNECTION: usize = 0x08;
/// `ServerConnection` instance `+0x16c` = `playerEntityID_` (u32).
const OFFSET_SERVER_CONNECTION_PLAYER_ENTITY_ID: usize = 0x16c;

/// Read the local player's entity id via the chain documented above.
///
/// `read(addr, 4)` must return the 4 bytes at `addr`, or `None` if
/// unreadable — the same shape as [`crate::fingerprint::check`]'s
/// reader, so this is testable with a fake address space and, on the
/// real target, backed by `cimmeria_client_hookgate::os::read_bytes`.
///
/// Returns `None` if the manager singleton isn't constructed yet, the
/// `ServerConnection` pointer is null, or `playerEntityID_` is still
/// its "none" value of `0` (e.g. before world entry, or right after a
/// `RESET_ENTITIES` with `keepBase == false`).
pub fn local_player_entity_id(read: impl Fn(usize, usize) -> Option<Vec<u8>>) -> Option<u32> {
    let gem = read_u32(&read, ADDR_GAME_ENTITY_MANAGER)?;
    if gem == 0 {
        return None;
    }
    let conn = read_u32(&read, gem as usize + OFFSET_GEM_SERVER_CONNECTION)?;
    if conn == 0 {
        return None;
    }
    let id = read_u32(
        &read,
        conn as usize + OFFSET_SERVER_CONNECTION_PLAYER_ENTITY_ID,
    )?;
    (id != 0).then_some(id)
}

fn read_u32(read: &impl Fn(usize, usize) -> Option<Vec<u8>>, addr: usize) -> Option<u32> {
    let bytes = read(addr, 4)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

/// [`local_player_entity_id`] against the real process, via
/// [`cimmeria_client_hookgate::os::read_bytes`] (`ReadProcessMemory`,
/// never faults).
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub fn read_local_player_entity_id() -> Option<u32> {
    local_player_entity_id(cimmeria_client_hookgate::os::read_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn mem_with(gem: u32, conn: u32, entity_id: u32) -> HashMap<usize, [u8; 4]> {
        let mut mem = HashMap::new();
        mem.insert(ADDR_GAME_ENTITY_MANAGER, gem.to_le_bytes());
        if gem != 0 {
            mem.insert(
                gem as usize + OFFSET_GEM_SERVER_CONNECTION,
                conn.to_le_bytes(),
            );
        }
        if conn != 0 {
            mem.insert(
                conn as usize + OFFSET_SERVER_CONNECTION_PLAYER_ENTITY_ID,
                entity_id.to_le_bytes(),
            );
        }
        mem
    }

    fn reader(mem: HashMap<usize, [u8; 4]>) -> impl Fn(usize, usize) -> Option<Vec<u8>> {
        move |addr, len| {
            (len == 4)
                .then(|| mem.get(&addr))
                .flatten()
                .map(|b| b.to_vec())
        }
    }

    #[test]
    fn full_chain_resolves_the_entity_id() {
        let mem = mem_with(0x1000_0000, 0x2000_0000, 42);
        assert_eq!(local_player_entity_id(reader(mem)), Some(42));
    }

    #[test]
    fn unconstructed_manager_is_none() {
        let mem = mem_with(0, 0, 0);
        assert_eq!(local_player_entity_id(reader(mem)), None);
    }

    #[test]
    fn null_server_connection_is_none() {
        let mem = mem_with(0x1000_0000, 0, 0);
        assert_eq!(local_player_entity_id(reader(mem)), None);
    }

    /// `playerEntityID_ == 0` is the field's own documented "none"
    /// value (what `RESET_ENTITIES` sets it back to) — not a
    /// different-build mismatch, so it must not be reported as a
    /// found id.
    #[test]
    fn zero_entity_id_is_none_not_a_found_zero() {
        let mem = mem_with(0x1000_0000, 0x2000_0000, 0);
        assert_eq!(local_player_entity_id(reader(mem)), None);
    }

    #[test]
    fn an_unreadable_address_space_is_none() {
        assert_eq!(local_player_entity_id(|_, _| None), None);
    }
}
