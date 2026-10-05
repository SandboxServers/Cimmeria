//! Offsets and memory readers for the ability press and send paths.
//!
//! Every offset is from the QA `SGW.exe` (finding
//! `ability-client-hook-anchors.md`, plus the GamePet send `0x00d3a820`
//! read for this packet on 2026-10-04). Every read goes through [`Mem`]:
//! the DLL backs it with `ReadProcessMemory`, so a stale pointer is `None`,
//! and nothing here calls game code.

use super::super::entity_trace::map::{self, Lookup, Mem};

/// `GameEntityManager::instance_` (what `0x00c66ad0` and `0x00dd05a0`
/// return).
pub(crate) const ENTITY_MANAGER: u32 = 0x01ef_244c;

/// `GameEntityManager` fields.
pub(crate) mod em {
    /// `ServerConnection*` (`RouteOutgoingEntityRpc` row 6 tests it).
    pub(crate) const CONNECTION: u32 = 0x08;
    /// The local player's entity id (`Route` passes it to `findEntity`).
    pub(crate) const LOCAL_PLAYER_ID: u32 = 0x14;
    /// The player context (`GameProxyPlayer`).
    pub(crate) const PLAYER_CTX: u32 = 0x8c;
}

/// Player-context (`GameProxyPlayer`) fields.
pub(crate) mod ctx {
    /// The local player entity.
    pub(crate) const PLAYER: u32 = 0x04;
    /// The action bar.
    pub(crate) const ACTION_BAR: u32 = 0x4c;
}

/// `GameBeing+0xfc`: the being's current target id (`setTargetId`
/// `0x00e003c0`).
pub(crate) const BEING_TARGET: u32 = 0xfc;

/// `ServerConnection+0x30c`: non-zero when connected (`0x00dd6130`).
pub(crate) const CONN_CONNECTED: u32 = 0x30c;

/// The action bar's slot vector (`FUN_00e3d190`).
pub(crate) mod bar {
    /// `begin` of the `Action*` vector.
    pub(crate) const BEGIN: u32 = 0x08;
    /// `end`.
    pub(crate) const END: u32 = 0x0c;
    /// `FUN_00e3d190` returns null for an index above `0xc7`.
    pub(crate) const SLOTS: u32 = 200;
}

/// `AbilitySet` fields.
pub(crate) mod set {
    /// `std::map<int, AbilityData*>` searched by `FUN_00d2a000`.
    pub(crate) const MAP: u32 = 0x28;
}

/// `AbilityData` record fields.
pub(crate) mod record {
    /// Targeting mode; `3` is a ground reticle (`FUN_00d2ae40`).
    pub(crate) const TARGETING: u32 = 0x48;
    /// `GROUND`.
    pub(crate) const TARGETING_GROUND: u32 = 3;
    /// Flags tested by the GamePet send (`0x00d3a868`).
    pub(crate) const FLAGS: u32 = 0x98;
    /// The bit whose presence drops a pet press (`0x00d3a86e`).
    pub(crate) const PET_REFUSE_BIT: u32 = 0x8;
}

/// `GamePet` fields read by its send (`0x00d3a820`).
pub(crate) mod pet {
    /// Flags; the send needs bit `0x400` (`0x00d3a83f`).
    pub(crate) const FLAGS: u32 = 0x38;
    /// The bit.
    pub(crate) const READY_BIT: u32 = 0x400;
    /// The pet's own `AbilitySet`.
    pub(crate) const ABILITY_SET: u32 = 0x174;
}

/// `Action` fields.
pub(crate) mod action {
    /// The ability id (`AbilityAction` and `PetAbilityAction`).
    pub(crate) const ABILITY_ID: u32 = 0x0c;
    /// The pet entity id (`PetAbilityAction`).
    pub(crate) const PET_ID: u32 = 0x10;
}

fn at(mem: &dyn Mem, base: u32, off: u32) -> Option<u32> {
    if base == 0 {
        return None;
    }
    mem.u32_at(base.wrapping_add(off))
}

/// The player context, `[EM+0x8c]`.
fn player_ctx(mem: &dyn Mem) -> Option<u32> {
    let em = mem.u32_at(ENTITY_MANAGER)?;
    at(mem, em, em::PLAYER_CTX).filter(|&c| c != 0)
}

/// `client_target_id`: `GameBeing+0xfc` of `[[EM+0x8c]+0x4]`, the field
/// the hotbar sends as `TargetID` (`0x00e3cdcd`). INFERRED that the entity
/// pointer needs no cast adjustment (finding, open question 1).
pub(crate) fn client_target_id(mem: &dyn Mem) -> Option<i32> {
    let player = at(mem, player_ctx(mem)?, ctx::PLAYER)?;
    at(mem, player, BEING_TARGET).map(|v| v as i32)
}

/// What `FUN_00e3d190(actionId - 1)` returns: `Some(0)` for an empty or
/// out-of-range slot, `Some(action)` otherwise, `None` when unreadable.
pub(crate) fn slot_action(mem: &dyn Mem, action_id: i32) -> Option<u32> {
    let idx = action_id.wrapping_sub(1) as u32;
    if idx >= bar::SLOTS {
        return Some(0);
    }
    let bar = at(mem, player_ctx(mem)?, ctx::ACTION_BAR)?;
    let begin = at(mem, bar, bar::BEGIN)?;
    let end = at(mem, bar, bar::END)?;
    if begin == 0 || idx >= end.wrapping_sub(begin) / 4 {
        // The game would take its invalid-parameter path here; report the
        // slot as unreadable rather than guess.
        return None;
    }
    mem.u32_at(begin.wrapping_add(idx * 4))
}

/// `FUN_00d2a000`: the record for `ability_id` in the `AbilitySet` at
/// `set`. `Some(0)` when absent (the game returns null), `None` when the
/// map cannot be read.
pub(crate) fn find_ability(mem: &dyn Mem, set: u32, ability_id: i32) -> Option<u32> {
    if set == 0 {
        return None;
    }
    match map::find(mem, set.wrapping_add(set::MAP), ability_id) {
        Lookup::Found(node) => mem.u32_at(node.wrapping_add(map::node::VALUE)),
        Lookup::Absent => Some(0),
        Lookup::Unreadable => None,
    }
}

/// Which branch of the GamePet send (`0x00d3a820`) drops the press, read
/// before it runs. `None` from a field means it could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PetGate {
    /// `[pet+0x38] & 0x400 != 0` (`JE` at `0x00d3a84a` when clear).
    pub ready: Option<bool>,
    /// The ability is in the pet's `AbilitySet` (`JE` at `0x00d3a862`).
    pub known: Option<bool>,
    /// `[record+0x98] & 8 == 0` (`JNE` at `0x00d3a875` when set).
    pub allowed: Option<bool>,
}

/// Read the three GamePet gates for `ability_id` on `pet`.
pub(crate) fn pet_gate(mem: &dyn Mem, pet: u32, ability_id: i32) -> PetGate {
    let ready = at(mem, pet, pet::FLAGS).map(|f| f & pet::READY_BIT != 0);
    let rec = at(mem, pet, pet::ABILITY_SET).and_then(|s| find_ability(mem, s, ability_id));
    let known = rec.map(|r| r != 0);
    let allowed = match rec {
        Some(r) if r != 0 => at(mem, r, record::FLAGS).map(|f| f & record::PET_REFUSE_BIT == 0),
        _ => None,
    };
    PetGate {
        ready,
        known,
        allowed,
    }
}

/// The checks `RouteOutgoingEntityRpc` makes before it picks a
/// `start*Message`, as far as memory shows them (rows 6 to 8). Rows 9 and
/// 10 (type mapping, class chain) need game calls and are observed instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct RoutePre {
    /// `[EM+8]` is non-null.
    pub has_connection: Option<bool>,
    /// `[conn+0x30c] != 0`.
    pub connected: Option<bool>,
    /// For a null entity: the local player is in the world map, or the
    /// cache map `findEntity(id, 1)` also searches.
    pub local_player_found: Option<bool>,
}

/// Read [`RoutePre`]. `entity` is the router's first argument.
pub(crate) fn route_pre(mem: &dyn Mem, entity: u32) -> RoutePre {
    let Some(em) = mem.u32_at(ENTITY_MANAGER).filter(|&e| e != 0) else {
        return RoutePre::default();
    };
    let conn = at(mem, em, em::CONNECTION);
    let connected = conn
        .filter(|&c| c != 0)
        .and_then(|c| at(mem, c, CONN_CONNECTED))
        .map(|v| v != 0);
    let local_player_found = if entity == 0 {
        at(mem, em, em::LOCAL_PLAYER_ID).and_then(|id| {
            let look = |m: u32| map::find(mem, em.wrapping_add(m), id as i32);
            match (look(map::manager::WORLD_MAP), look(map::manager::CACHE_MAP)) {
                (Lookup::Found(_), _) | (_, Lookup::Found(_)) => Some(true),
                (Lookup::Absent, Lookup::Absent) => Some(false),
                _ => None,
            }
        })
    } else {
        // The router only looks the entity up when it was given none.
        Some(true)
    };
    RoutePre {
        has_connection: conn.map(|c| c != 0),
        connected,
        local_player_found,
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::entity_trace::map::fake::{build_map, FakeMem};
    use super::*;

    const EM: u32 = 0x1000_0000;
    const CTX: u32 = 0x1100_0000;
    const PLAYER: u32 = 0x1200_0000;
    const BAR: u32 = 0x1300_0000;
    const SLOTS: u32 = 0x1400_0000;

    fn world() -> FakeMem {
        let mut m = FakeMem::default();
        m.set(ENTITY_MANAGER, EM);
        m.set(EM + em::PLAYER_CTX, CTX);
        m.set(CTX + ctx::PLAYER, PLAYER);
        m.set(PLAYER + BEING_TARGET, 4321);
        m.set(CTX + ctx::ACTION_BAR, BAR);
        m.set(BAR + bar::BEGIN, SLOTS);
        m.set(BAR + bar::END, SLOTS + 200 * 4);
        for i in 0..200 {
            m.set(SLOTS + i * 4, 0);
        }
        m
    }

    #[test]
    fn the_client_target_is_the_local_beings_field() {
        assert_eq!(client_target_id(&world()), Some(4321));
        assert_eq!(client_target_id(&FakeMem::default()), None);
    }

    #[test]
    fn slots_are_one_based_and_bounded() {
        let mut m = world();
        m.set(SLOTS + 4, 0xAAAA_0000); // action id 2
        assert_eq!(slot_action(&m, 2), Some(0xAAAA_0000));
        assert_eq!(slot_action(&m, 1), Some(0), "empty slot");
        assert_eq!(slot_action(&m, 0), Some(0), "0 - 1 wraps above 199");
        assert_eq!(slot_action(&m, 201), Some(0), "above 200");
        assert_eq!(slot_action(&FakeMem::default(), 2), None);
    }

    #[test]
    fn an_ability_set_lookup_tells_absent_from_unreadable() {
        let mut m = FakeMem::default();
        let set = 0x2000_0000;
        build_map(&mut m, set + set::MAP, 0x2100_0040, &[(597, 0x3000_0000)]);
        assert_eq!(find_ability(&m, set, 597), Some(0x3000_0000));
        assert_eq!(find_ability(&m, set, 598), Some(0));
        assert_eq!(find_ability(&m, 0x5000_0000, 597), None);
    }

    #[test]
    fn the_pet_gate_reads_the_three_branches() {
        let mut m = FakeMem::default();
        let pet = 0x4000_0000;
        let set = 0x4100_0000;
        let rec = 0x4200_0000;
        m.set(pet + pet::FLAGS, 0x400);
        m.set(pet + pet::ABILITY_SET, set);
        build_map(&mut m, set + set::MAP, 0x4300_0040, &[(11, rec)]);
        m.set(rec + record::FLAGS, 0);
        let all = pet_gate(&m, pet, 11);
        assert_eq!(all.ready, Some(true));
        assert_eq!(all.known, Some(true));
        assert_eq!(all.allowed, Some(true));
        m.set(rec + record::FLAGS, 8);
        assert_eq!(pet_gate(&m, pet, 11).allowed, Some(false));
        let unknown = pet_gate(&m, pet, 12);
        assert_eq!(unknown.known, Some(false));
        assert_eq!(unknown.allowed, None);
        m.set(pet + pet::FLAGS, 0x3ff);
        assert_eq!(pet_gate(&m, pet, 11).ready, Some(false));
    }

    #[test]
    fn route_pre_reads_rows_six_to_eight() {
        let mut m = world();
        let conn = 0x6000_0000;
        m.set(EM + em::CONNECTION, conn);
        m.set(conn + CONN_CONNECTED, 1);
        m.set(EM + em::LOCAL_PLAYER_ID, 77);
        build_map(
            &mut m,
            EM + map::manager::WORLD_MAP,
            0x6100_0040,
            &[(77, PLAYER)],
        );
        build_map(&mut m, EM + map::manager::CACHE_MAP, 0x6200_0040, &[]);
        let pre = route_pre(&m, 0);
        assert_eq!(pre.has_connection, Some(true));
        assert_eq!(pre.connected, Some(true));
        assert_eq!(pre.local_player_found, Some(true));

        m.set(conn + CONN_CONNECTED, 0);
        assert_eq!(route_pre(&m, 0).connected, Some(false));
        m.set(EM + em::CONNECTION, 0);
        let pre = route_pre(&m, 0);
        assert_eq!(pre.has_connection, Some(false));
        assert_eq!(pre.connected, None);
    }
}
