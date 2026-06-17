//! SGWGmPlayer own CellMethods — the gm*/debug command tail that appends at
//! flattened cell-method index 109+.
//!
//! SGWGmPlayer declares `<Parent>SGWPlayer</Parent>` with empty
//! `<Implements>`, so its own `<Exposed/>` CellMethods land AFTER SGWPlayer's
//! own block (0-108) and the inherited indices do not renumber. Every index
//! in this range is a GM/debug command by construction, so the whole tail is
//! authorization-gated upstream in
//! [`crate::cell::dispatch::gm_gate`] (`requires_gm` returns `true` for any
//! `index >= 109`). By the time this dispatch runs, the caller is already
//! confirmed `access_level >= GameMaster` — these handlers do NOT re-check.
//!
//! Only a subset is implemented; every other 109+ index falls through the
//! router's auth-gated `warn!` arm (harmless — the call was already
//! authorized, it just has no handler yet). Each implemented index is pinned
//! to a constant and the constant is asserted against the document-order count
//! of `<Exposed/>` methods in `SGWGmPlayer.def`
//! (`tests::gm_indices_match_def_document_order`).
//!
//! Handlers are grouped by family into submodules:
//! - [`give`] — grant/remove (xp, item, cash).
//! - [`stats`] — set health/focus current+max.
//! - [`missions`] — assign/clear/advance + list/full/details.
//! - [`travel`] — goto-xyz / goto-location / goto / summon / DHD dial.
//! - [`world`] — kill / despawn / respawn / set-target.
//! - [`spawn`] — spawn-by-cmd (cell↔base template round-trip).
//! - [`query`] — inspection/show + users + test-LOS (report via [`feedback`]).
//! - [`feedback`] — single-recipient `onPlayerCommunication` delivery.
//!
//! The full 117-method inventory + handler-status map (DONE/REUSE/ADAPT/NEW)
//! lives in `docs/protocol/cell-method-dispatch-table.md`; the ADAPT roadmap
//! is in `docs/architecture/gm-cell-method-adapt-plan.md`.

pub(crate) mod feedback;
mod give;
mod missions;
mod query;
mod spawn;
mod stats;
mod travel;
mod world;

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

// ── Flattened SGWGmPlayer cell-method indices ────────────────────────────────
//
// Counted from SGWGmPlayer.def in document order, starting at 109 for the
// first own `<Exposed/>` method (gmMissionAssign). The def line references let
// a reviewer re-count without re-running the script; the offsets (index − 109)
// are re-asserted in `tests::gm_indices_match_def_document_order`. The three
// pcap-anchored DONE indices (133/163/190) are the alignment proof.

// -- Missions (109–120) -------------------------------------------------------
/// `gmMissionAssign(WSTRING DesignID, UINT8 popup)` — def line 65. Offset 0.
pub const GM_MISSION_ASSIGN: u16 = 109;
/// `gmMissionClear(WSTRING DesignID)` — def line 71. Offset 1. Abandons one
/// mission by numeric design id.
pub const GM_MISSION_CLEAR: u16 = 110;
/// `gmMissionList()` — def line 84. Offset 4. Lists the caller's active missions.
pub const GM_MISSION_LIST: u16 = 113;
/// `gmMissionListFull()` — def line 88. Offset 5. Lists all the caller's missions.
pub const GM_MISSION_LIST_FULL: u16 = 114;
/// `gmMissionDetails(WSTRING DesignID)` — def line 92. Offset 6.
pub const GM_MISSION_DETAILS: u16 = 115;
/// `gmMissionAdvance(WSTRING DesignID, INT32 step)` — def line 97. Offset 7.
pub const GM_MISSION_ADVANCE: u16 = 116;
/// `gmMissionAbandon(WSTRING DesignID)` — def line 120. Offset 11. Alias of
/// gmMissionClear with the same primitive.
pub const GM_MISSION_ABANDON: u16 = 120;

// -- Give / remove (132–135) --------------------------------------------------
/// `gmGiveXp(INT32 amount)` — def line 180. Offset 23. Grants XP to the caller.
pub const GM_GIVE_XP: u16 = 132;
/// `gmGiveItem(WSTRING DesignId, INT32 Quantity)` — def line 185. Offset 24.
pub const GM_GIVE_ITEM: u16 = 133;
/// `gmGiveCash(INT32 Amount)` — def line 191. Offset 25.
pub const GM_GIVE_CASH: u16 = 134;
/// `gmRemoveItem(ItemID itemID, INT16 quantity)` — def line 196. Offset 26.
/// `ItemID` resolves to INT32 (`entities/defs/alias.xml`).
pub const GM_REMOVE_ITEM: u16 = 135;

// -- Crafting grants (139, 140) -----------------------------------------------
/// `gmGiveExpertise(INT32 aDisciplineId, INT32 aExpertise)` — offset 30.
/// Grants crafting expertise in one discipline. Routes through
/// `CellToBaseMsg::GrantExpertise` → `base::crafting::handle_grant_expertise`.
pub const GM_GIVE_EXPERTISE: u16 = 139;
/// `gmGiveAppliedSciencePoints(INT32 aPoints)` — offset 31. Grants
/// applied-science points. Routes through
/// `CellToBaseMsg::GrantAppliedSciencePoints` →
/// `base::crafting::handle_grant_applied_science`.
pub const GM_GIVE_APPLIED_SCIENCE_POINTS: u16 = 140;

// -- Set health / focus (147–150) ---------------------------------------------
/// `gmSetHealth(INT32 Amount, INT64 TargetId)` — def line 259. Offset 38.
pub const GM_SET_HEALTH: u16 = 147;
/// `gmSetHealthMax(INT32 Amount, INT64 TargetId)` — def line 265. Offset 39.
pub const GM_SET_HEALTH_MAX: u16 = 148;
/// `gmSetFocus(INT32 Amount, INT64 TargetId)` — def line 271. Offset 40.
pub const GM_SET_FOCUS: u16 = 149;
/// `gmSetFocusMax(INT32 Amount, INT64 TargetId)` — def line 277. Offset 41.
pub const GM_SET_FOCUS_MAX: u16 = 150;

// -- Set target (156) ---------------------------------------------------------
/// `gmSetTarget(WSTRING aNameOrID)` — def line 302. Offset 47. Numeric-id form
/// only (name resolution is not wired into the cell).
pub const GM_SET_TARGET: u16 = 156;

// -- Travel (159, 162, 163) ---------------------------------------------------
/// `gmDHD(INT8 aGateAddress)` — def line 325. Offset 50. Dials a stargate.
pub const GM_DHD: u16 = 159;
/// `gmGoto(WSTRING aNameOrID)` — def line 330. Offset 51. Teleport caller to a
/// target entity (numeric id, same-space).
pub const GM_GOTO: u16 = 160;
/// `gmSummon(WSTRING aNameOrID)` — def line 335. Offset 52. Move a target entity
/// to the caller (numeric id, same-space).
pub const GM_SUMMON: u16 = 161;
/// `gmGotoLocation(WSTRING aWorldName, FLOAT aX, aY, aZ)` — def line 340.
/// Offset 53. Cross-world teleport (full reload).
pub const GM_GOTO_LOCATION: u16 = 162;
/// `gmGotoXYZ(FLOAT aX, FLOAT aY, FLOAT aZ)` — def line 348. Offset 54.
/// Same-space snap teleport.
pub const GM_GOTO_XYZ: u16 = 163;

// -- Inspection / show (121–131) ----------------------------------------------
/// `gmShowTargetLocation()` — def line 127. Offset 12. Reports the current
/// target's (or caller's) position. FanMMORPG `.location`.
pub const GM_SHOW_TARGET_LOCATION: u16 = 121;
/// `gmShowRotation()` — def line 131. Offset 13. Reports facing/heading.
/// FanMMORPG `.rotation` / `.facing`.
pub const GM_SHOW_ROTATION: u16 = 122;
/// `listAbilities()` — def line 135. Offset 14. Lists known ability ids.
pub const LIST_ABILITIES: u16 = 123;
/// `gmShowFlag(INT32 flagId)` — def line 144. Offset 16. Tests a state-flag bit.
pub const GM_SHOW_FLAG: u16 = 125;
/// `gmGetMobAttribute(INT32 TargetID, WSTRING Attribute)` — def line 153.
/// Offset 18. Reports one hand-mapped attribute.
pub const GM_GET_MOB_ATTRIBUTE: u16 = 127;
/// `gmShowMobCount(INT32 SpaceID)` — def line 159. Offset 19. Counts NPCs.
pub const GM_SHOW_MOB_COUNT: u16 = 128;
/// `gmShowPlayer(INT32 TargetID)` — def line 174. Offset 22. Dumps entity info.
/// FanMMORPG `.info`.
pub const GM_SHOW_PLAYER: u16 = 131;

// -- Admin / social (166) -----------------------------------------------------
/// `gmUsers()` — def line 363. Offset 57. Lists players in the caller's space
/// (the stock `/Users` / `/Who` console binding).
pub const GM_USERS: u16 = 166;

// -- Spawn / mob (185, 186, 189, 190) -----------------------------------------
/// `gmSpawnByCmd(WSTRING DesignId, FLOAT XOffset, FLOAT ZOffset)` — offset 76.
/// Spawns an NPC by template id at the caller's position + X/Z offset. Routes
/// through `CellToBaseMsg::GmSpawnNpc` → base template lookup →
/// `BaseToCellMsg::GmSpawnNpcReady`.
pub const GM_SPAWN_BY_CMD: u16 = 185;
/// `gmDespawnByCmd(INT32 TargetID)` — def line 461. Offset 77.
pub const GM_DESPAWN_BY_CMD: u16 = 186;
/// `gmRespawn()` — def line 478. Offset 80. Respawns the calling GM.
pub const GM_RESPAWN: u16 = 189;
/// `gmKillTarget(INT64 TargetId)` — def line 482. Offset 81. NPC-only.
pub const GM_KILL_TARGET: u16 = 190;

// -- Debug (180) --------------------------------------------------------------
/// `gmDebugMobData(INT32 aSpaceID, INT32 target)` — def line 427. Offset 71.
/// Dumps a mob's debug data via feedback.
pub const GM_DEBUG_MOB_DATA: u16 = 180;

// -- Test (213, 216) ----------------------------------------------------------
/// `despawnMob(INT32 entityID)` — def line 605. Offset 104. Test alias of
/// gmDespawnByCmd (same `destroy_entity` primitive).
pub const DESPAWN_MOB: u16 = 213;
/// `testLOS(INT32 source, INT32 target)` — def line 619. Offset 107. Reports
/// navmesh line-of-sight between two entities via the feedback channel.
pub const TEST_LOS: u16 = 216;

/// Dispatch an SGWGmPlayer own cell method (flattened index >= 109).
///
/// Returns `true` if the index was handled, `false` if it's an unimplemented
/// 109+ index (so the router falls through to its already-authorized
/// `warn!` arm). The caller is guaranteed GM by the dispatch-layer gate.
pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        // -- give / remove --
        GM_GIVE_XP => give::handle_give_xp(entity_id, args, tx, space_mgr).await,
        GM_GIVE_ITEM => give::handle_give_item(entity_id, args, tx, space_mgr).await,
        GM_GIVE_CASH => give::handle_give_cash(entity_id, args, tx, space_mgr).await,
        GM_REMOVE_ITEM => give::handle_remove_item(entity_id, args, tx, space_mgr).await,
        GM_GIVE_EXPERTISE => give::handle_give_expertise(entity_id, args, tx, space_mgr).await,
        GM_GIVE_APPLIED_SCIENCE_POINTS => {
            give::handle_give_applied_science(entity_id, args, tx, space_mgr).await
        }
        // -- stats --
        GM_SET_HEALTH => stats::handle_set_health(entity_id, args, false, tx, space_mgr).await,
        GM_SET_HEALTH_MAX => stats::handle_set_health(entity_id, args, true, tx, space_mgr).await,
        GM_SET_FOCUS => stats::handle_set_focus(entity_id, args, false, tx, space_mgr).await,
        GM_SET_FOCUS_MAX => stats::handle_set_focus(entity_id, args, true, tx, space_mgr).await,
        // -- missions --
        GM_MISSION_ASSIGN => missions::handle_mission_assign(entity_id, args, tx, space_mgr).await,
        GM_MISSION_CLEAR | GM_MISSION_ABANDON => {
            missions::handle_mission_clear(entity_id, args, tx, space_mgr).await
        }
        GM_MISSION_ADVANCE => {
            missions::handle_mission_advance(entity_id, args, tx, space_mgr).await
        }
        GM_MISSION_LIST => missions::handle_mission_list(entity_id, tx, space_mgr).await,
        GM_MISSION_LIST_FULL => missions::handle_mission_list_full(entity_id, tx, space_mgr).await,
        GM_MISSION_DETAILS => {
            missions::handle_mission_details(entity_id, args, tx, space_mgr).await
        }
        // -- travel --
        GM_GOTO_XYZ => travel::handle_goto_xyz(entity_id, args, tx, space_mgr).await,
        GM_GOTO => travel::handle_goto(entity_id, args, tx, space_mgr).await,
        GM_SUMMON => travel::handle_summon(entity_id, args, tx, space_mgr).await,
        GM_GOTO_LOCATION => travel::handle_goto_location(entity_id, args, tx, space_mgr).await,
        GM_DHD => travel::handle_dhd(entity_id, args, tx, space_mgr).await,
        // -- world / entity ops --
        GM_SET_TARGET => world::handle_set_target(entity_id, args, tx, space_mgr).await,
        GM_KILL_TARGET => world::handle_kill_target(entity_id, args, tx, space_mgr).await,
        GM_DESPAWN_BY_CMD | DESPAWN_MOB => {
            world::handle_despawn(entity_id, args, tx, space_mgr).await
        }
        GM_RESPAWN => world::handle_respawn_cmd(entity_id, tx, space_mgr).await,
        GM_SPAWN_BY_CMD => spawn::handle_spawn_by_cmd(entity_id, args, tx, space_mgr).await,
        // -- query (report text via the feedback channel) --
        GM_USERS => query::handle_users(entity_id, tx, space_mgr).await,
        TEST_LOS => query::handle_test_los(entity_id, args, tx, space_mgr).await,
        GM_SHOW_TARGET_LOCATION => {
            query::handle_show_target_location(entity_id, tx, space_mgr).await
        }
        GM_SHOW_ROTATION => query::handle_show_rotation(entity_id, tx, space_mgr).await,
        GM_SHOW_PLAYER => query::handle_show_player(entity_id, args, tx, space_mgr).await,
        LIST_ABILITIES => query::handle_list_abilities(entity_id, tx, space_mgr).await,
        GM_SHOW_FLAG => query::handle_show_flag(entity_id, args, tx, space_mgr).await,
        GM_GET_MOB_ATTRIBUTE => {
            query::handle_get_mob_attribute(entity_id, args, tx, space_mgr).await
        }
        GM_SHOW_MOB_COUNT => query::handle_show_mob_count(entity_id, args, tx, space_mgr).await,
        GM_DEBUG_MOB_DATA => query::handle_debug_mob_data(entity_id, args, tx, space_mgr).await,
        // Any other 109+ index is an unimplemented (but authorized) gm*
        // method — let the router fall through to its warn arm.
        _ => false,
    }
}

// ── Shared arg-parsing + target-resolution helpers ───────────────────────────

/// Read a little-endian `i32` at `offset`, or `None` if the slice is too short.
pub(super) fn read_i32(args: &[u8], offset: usize) -> Option<i32> {
    args.get(offset..offset + 4)
        .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Read a little-endian `i64` at `offset`, or `None` if the slice is too short.
pub(super) fn read_i64(args: &[u8], offset: usize) -> Option<i64> {
    args.get(offset..offset + 8)
        .map(|b| i64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
}

/// Resolve a GM command's `INT64 TargetId` to a concrete entity id, defaulting
/// to the caller themselves.
///
/// - `target == 0` (or the caller's own id) → operate on `caller_id` (the
///   common "set my own health" dev case).
/// - otherwise the target must be (a) representable as a `u32` entity id,
///   (b) resolvable in the space manager, and (c) in the **same space** as the
///   caller. A cross-space target is refused — a GM can't reach into another
///   instance from here, and an out-of-range / unknown id is a malformed call.
///
/// Returns `None` (after a `warn!`) on any failed guard so the caller no-ops.
pub(super) fn resolve_self_or_target(
    caller_id: u32,
    target: i64,
    space_mgr: &SpaceManager,
    cmd: &str,
) -> Option<u32> {
    if target == 0 || target == i64::from(caller_id) {
        return Some(caller_id);
    }
    let target_eid = match u32::try_from(target) {
        Ok(id) => id,
        Err(_) => {
            tracing::warn!(
                entity_id = caller_id,
                target,
                cmd,
                "GM cmd: target id out of u32 range"
            );
            return None;
        }
    };
    let caller_space = space_mgr.get_entity(caller_id).map(|e| e.space_id.0);
    match space_mgr.get_entity(target_eid) {
        Some(t) if Some(t.space_id.0) == caller_space => Some(target_eid),
        Some(t) => {
            tracing::warn!(
                entity_id = caller_id,
                target_eid,
                target_space = t.space_id.0,
                caller_space = ?caller_space,
                cmd,
                "GM cmd: target is in a different space — refused"
            );
            None
        }
        None => {
            tracing::warn!(
                entity_id = caller_id,
                target_eid,
                cmd,
                "GM cmd: target not found"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests;
