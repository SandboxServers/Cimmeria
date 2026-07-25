//! Server-authoritative GM gate for the cell-method dispatcher (#475 /
//! CAT-N-03).
//!
//! Some flattened cell-method indices map to GM / debug commands. Before
//! #475 the cell layer had no access to the caller's access level, so any
//! such handler was unauthenticated-by-default: a modified client could
//! send the wire shape and the handler would run. This module is the
//! single choke point that fixes that — [`requires_gm`] classifies the
//! restricted indices and [`enforce_gm_gate`] rejects a non-privileged
//! caller before the router ever reaches the handler.
//!
//! The access level read here is `CellEntity::access_level`, set once at
//! `InitPlayerState` from the login session's `account.accesslevel`
//! column. It is never derived from a client-supplied byte, so a player
//! cannot self-elevate on the wire.
//!
//! Adding the next GM/`gm*` method is a one-line change to
//! [`requires_gm`]; the enforcement, logging, and the wire-visible error
//! response are shared.

use tokio::sync::mpsc;

use cimmeria_commands::permissions::AccessLevel;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::constants::{
    CM_MG_DEBUG_INSTANCE, CM_MG_DEBUG_JOIN, CM_MG_DEBUG_SPECTATE, CM_MG_DEBUG_START,
    CM_TOGGLE_COMBAT_DEBUG, CM_TOGGLE_COMBAT_VERBOSE_DEBUG, CM_TOGGLE_HEAL_DEBUG,
    CM_WORLD_INSTANCE_RESET,
};

/// `onErrorCode` client method index (SGWPlayer flat index 121). The
/// server→client feedback channel already used by the ability path for
/// out-of-range rejections. We reuse it so a rejected GM call produces a
/// wire-visible response, not just a server log.
const ON_ERROR_CODE: u16 = 121;

/// `EErrorCodeSystem` token value for a server-authority rejection.
/// `ERRORCODE_SYSTEM_Ability = 0` is the only token the 2009
/// `EErrorCodeSystem` enum defines; we reuse it as the system id for the
/// GM-denial feedback (the client treats an unknown error tuple as a
/// no-op toast, so this is safe and the InstanceID/ErrorCodeID below
/// identify the real cause for anyone reading the wire log).
const ERRORCODE_SYSTEM_ABILITY: u8 = 0;

/// `EConditionHandlerFeedback` has no dedicated "permission denied"
/// token, so we send `CONDITION_FEEDBACK_InvalidEntity = 0` — the client
/// renders it as a generic "you can't do that" rather than crashing on
/// an out-of-range enum value. The authoritative reason lives in the
/// server `warn!` log; this byte is only the client-side toast hint.
const CONDITION_FEEDBACK_INVALID_ENTITY: u16 = 0;

/// The minimum access level a GM/debug cell method requires.
const REQUIRED: AccessLevel = AccessLevel::GameMaster;

/// First flattened cell-method index owned by SGWGmPlayer. SGWPlayer's own
/// methods occupy 0-108; SGWGmPlayer's 117 `<Exposed/>` gm*/debug methods
/// APPEND at 109-225 (append-at-end inheritance — see
/// `crate::mercury::SGWGMPLAYER_CLASS_ID`). The entire tail is GM-only by
/// construction, so one threshold secures the whole native GM surface —
/// including every gm* index we haven't implemented a handler for yet
/// (those hit the auth-gated router fall-through, harmless).
pub const SGWGMPLAYER_CELL_METHOD_BASE: u16 = 109;

/// Returns `true` if the flattened cell-method `index` is a GM / debug
/// command that must be gated on `access_level >= GameMaster`.
///
/// This is the allow-list of *restricted* indices — everything not named
/// here is an ordinary player method and passes the gate untouched.
///
/// Two classes of restricted index:
/// - The GM/debug methods that live INSIDE the inherited SGWPlayer range
///   (0-108). The `>= SGWGMPLAYER_CELL_METHOD_BASE` range rule below cannot
///   reach them: they are interleaved with ordinary player methods because
///   they were declared on interfaces (`MinigamePlayer`, combat, ability
///   management) that SGWPlayer implements, so they got flattened positions
///   *below* the SGWGmPlayer base. There is no structural marker on the wire
///   that distinguishes them from a legitimate player call — the only thing
///   that gates them is being named here. Naming one is what makes it
///   GM-only; forgetting one leaves a debug surface reachable by any
///   modified client the moment somebody replaces its stub with a real
///   implementation.
///   - `2` `toggleCombatDebug`, `3` `toggleCombatVerboseDebug`,
///     `6` `toggleHealDebug` — combat/heal debug toggles.
///   - `20`-`23` `debugStartMinigame`, `debugSpectateMinigame`,
///     `debugJoinMinigame`, `debugMinigameInstance` — MinigamePlayer debug
///     entry points that force minigame state transitions. All four are
///     no-op `pass` bodies in the reference cell script, i.e. developer
///     hooks that were never meant to be player-reachable.
///   - `92` `onWorldInstanceReset` — tears down + recreates the current
///     space instance, kicking every co-located player (high impact).
/// - The ENTIRE SGWGmPlayer tail (`>= SGWGMPLAYER_CELL_METHOD_BASE`, i.e.
///   109+). Every method there is a gm*/debug command by construction, so
///   one range rule gates all of them at once — no per-method bookkeeping.
///
/// # What qualifies an index for this list
///
/// A sub-109 index belongs here only if it is *itself* a GM/debug surface.
/// "It has a GM-shaped sibling" is **not** the test — the SGWGmPlayer tail
/// deliberately mirrors several player-facing methods with uncosted GM
/// twins, and the twin being GM-only says nothing about the player-facing
/// original. Check what the method does, not what its name rhymes with.
///
/// # Deliberately NOT gated here
///
/// `resetMyAbilities` (CM 72) is the standing example, and gets proposed
/// for this list periodically on name shape alone. It is a **player-facing**
/// trainer respec: the reference cell script requires an active
/// `trainerEntity` within `MAX_INTERACT_DISTANCE` and then delegates to
/// `trainerEntity.onRespec`. Its gate is interaction *context*, not caller
/// *privilege*. The hardening it actually needs belongs in the handler —
/// charge the respec from `sgw_player.cash` and enforce a per-character
/// cooldown before refunding training points — and is explicitly not a
/// wire-shape match to the uncosted GM `gmRespec`.
///
/// `gmRespec` is the GM twin, and it lives on SGWGmPlayer above the base,
/// so the range rule already covers it. The two are different methods on
/// different index spaces; conflating them is what puts CM 72 on this list
/// by mistake. Gating CM 72 here would make respec unreachable for ordinary
/// players the moment the stub handler is implemented, and the symptom
/// would be a misleading privilege-escalation `warn!` rather than anything
/// pointing at this function.
///
/// Flattened cell-method positions are documented in
/// `docs/protocol/cell-method-dispatch-table.md`.
pub fn requires_gm(index: u16) -> bool {
    matches!(
        index,
        CM_TOGGLE_COMBAT_DEBUG
            | CM_TOGGLE_COMBAT_VERBOSE_DEBUG
            | CM_TOGGLE_HEAL_DEBUG
            | CM_MG_DEBUG_START
            | CM_MG_DEBUG_SPECTATE
            | CM_MG_DEBUG_JOIN
            | CM_MG_DEBUG_INSTANCE
            | CM_WORLD_INSTANCE_RESET
    ) || index >= SGWGMPLAYER_CELL_METHOD_BASE
}

/// Enforce the GM gate for `method_index` against the caller entity's
/// access level.
///
/// Returns `true` when the call is **allowed** (either the method isn't
/// GM-restricted, or the caller is privileged enough). Returns `false`
/// when the call must be **rejected** — in which case this has already
/// emitted the `warn!` audit log and sent the `onErrorCode` wire response
/// to the caller, and the router must NOT route the method.
///
/// A missing entity fails closed (rejected): a GM-restricted method with
/// no resolvable caller has no business running.
#[must_use]
pub async fn enforce_gm_gate(
    entity_id: u32,
    method_index: u16,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> bool {
    if !requires_gm(method_index) {
        return true;
    }

    let access_level = space_mgr
        .get_entity(entity_id)
        .map(|e| e.access_level)
        .unwrap_or(0);

    let caller = access_level_from_u32(access_level);
    if caller.can_execute(REQUIRED) {
        tracing::info!(
            entity_id,
            method_index,
            access_level,
            "GM-gated cell method authorized"
        );
        return true;
    }

    // Rejected. Loud `warn!` so an ops dashboard surfaces attempted
    // privilege escalation, with the structured fields needed to pivot
    // by caller + method.
    tracing::warn!(
        entity_id,
        method_index,
        access_level,
        required = %REQUIRED,
        "GM-gated cell method rejected -- caller is not a GM (server-authority gate, #475)"
    );

    let mut err = Vec::with_capacity(7);
    err.push(ERRORCODE_SYSTEM_ABILITY);
    err.extend_from_slice(&(method_index as i32).to_le_bytes());
    err.extend_from_slice(&CONDITION_FEEDBACK_INVALID_ENTITY.to_le_bytes());
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_ERROR_CODE,
            args: err,
        })
        .await;

    false
}

/// Map a raw `access_level` byte to the typed [`AccessLevel`]. Values
/// above the known range clamp to `Developer` (most privileged) — an
/// out-of-range stored level should never *lose* privilege, and the
/// source column is server-controlled so this is defense-in-depth, not a
/// trust boundary.
fn access_level_from_u32(level: u32) -> AccessLevel {
    match level {
        0 => AccessLevel::Player,
        1 => AccessLevel::Moderator,
        2 => AccessLevel::GameMaster,
        3 => AccessLevel::Admin,
        _ => AccessLevel::Developer,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restricted_indices_are_classified() {
        for idx in [2u16, 3, 6, 20, 21, 22, 23, 92] {
            assert!(requires_gm(idx), "index {idx} must be GM-gated");
        }
    }

    #[test]
    fn ordinary_player_methods_are_not_gated() {
        // A spread of non-GM indices across the inherited SGWPlayer interface
        // ranges. 108 is the LAST inherited player method — it must stay
        // ungated; 109 (first SGWGmPlayer method) must become gated.
        for idx in [0u16, 1, 25, 36, 52, 67, 72, 108] {
            assert!(!requires_gm(idx), "index {idx} must NOT be GM-gated");
        }
    }

    /// The debug methods that live BELOW the SGWGmPlayer base are gated
    /// *only* because they are named in `requires_gm` — the `>= 109` range
    /// rule cannot reach them, and nothing on the wire distinguishes them
    /// from a legitimate player call. Drop any one from the `matches!` arm
    /// and a non-GM client reaches a debug surface directly.
    ///
    /// The MinigamePlayer debug quartet (20-23) is the set the range rule
    /// silently missed: `debugStartMinigame`, `debugSpectateMinigame`,
    /// `debugJoinMinigame`, `debugMinigameInstance` force minigame state
    /// transitions and are no-ops in the reference cell script.
    ///
    /// Neighbours are pinned as *ungated* on purpose, so a "fix" that
    /// widens the arm into a range (`20..=24`) fails too — `minigameStart`
    /// (24) is an ordinary player call and gating it would break non-GM
    /// clients. This test trips on both the under-gate and the over-gate.
    #[test]
    fn debug_methods_below_the_gm_base_are_individually_gated() {
        // Pin the constants the dispatcher matches on — if a renumber moves
        // a debug method, the gate has to move with it.
        assert_eq!(CM_MG_DEBUG_START, 20);
        assert_eq!(CM_MG_DEBUG_SPECTATE, 21);
        assert_eq!(CM_MG_DEBUG_JOIN, 22);
        assert_eq!(CM_MG_DEBUG_INSTANCE, 23);

        for (idx, name) in [
            (CM_TOGGLE_COMBAT_DEBUG, "toggleCombatDebug"),
            (CM_TOGGLE_COMBAT_VERBOSE_DEBUG, "toggleCombatVerboseDebug"),
            (CM_TOGGLE_HEAL_DEBUG, "toggleHealDebug"),
            (CM_MG_DEBUG_START, "debugStartMinigame"),
            (CM_MG_DEBUG_SPECTATE, "debugSpectateMinigame"),
            (CM_MG_DEBUG_JOIN, "debugJoinMinigame"),
            (CM_MG_DEBUG_INSTANCE, "debugMinigameInstance"),
            (CM_WORLD_INSTANCE_RESET, "onWorldInstanceReset"),
        ] {
            assert!(
                idx < SGWGMPLAYER_CELL_METHOD_BASE,
                "{name} ({idx}) is supposed to be below the SGWGmPlayer base — \
                 if it moved above it, this test is guarding the wrong thing"
            );
            assert!(
                requires_gm(idx),
                "{name} ({idx}) is a debug surface below the SGWGmPlayer base, \
                 so the `>= {SGWGMPLAYER_CELL_METHOD_BASE}` range rule does not \
                 cover it — it must be named explicitly in requires_gm"
            );
        }

        // Immediate neighbours of the debug quartet must stay reachable by
        // ordinary players — this is what fails if the arm is widened to a
        // range instead of naming each index.
        for (idx, name) in [
            (19u16, "transferCash"),
            (24, "minigameStart"),
            (25, "endCurrentMinigame"),
        ] {
            assert!(
                !requires_gm(idx),
                "{name} ({idx}) is an ordinary player method — gating it would \
                 break non-GM clients"
            );
        }
    }

    /// `resetMyAbilities` (CM 72) is the player-facing trainer respec, not a
    /// GM command: the reference cell script gates it on an active
    /// `trainerEntity` within interact distance, then delegates to
    /// `trainerEntity.onRespec`. Its uncosted GM twin is `gmRespec`, which
    /// sits on SGWGmPlayer above the base and is already covered by the
    /// range rule.
    ///
    /// This is pinned as a test because CM 72 gets proposed for the
    /// restricted list on name-shape alone. Adding it would make respec
    /// unreachable for ordinary players the moment the handler stops being
    /// a stub — a regression that would otherwise only surface as a player
    /// bug report months later. The real hardening CM 72 needs (cash cost +
    /// per-character cooldown) belongs in the handler, not in this gate.
    #[test]
    fn trainer_respec_is_not_treated_as_a_gm_command() {
        assert_eq!(
            crate::cell::cell_methods::player::RESET_MY_ABILITIES,
            72,
            "the constant this test is about must still be 72"
        );
        assert!(
            !requires_gm(crate::cell::cell_methods::player::RESET_MY_ABILITIES),
            "resetMyAbilities is the player-facing trainer respec — gating it \
             on access_level breaks respec for every non-GM player"
        );
    }

    /// The entire SGWGmPlayer cell-method tail (>= 109) is
    /// GM-only by construction. One range rule must gate all of it — every
    /// gm*/debug index, implemented or not. The boundary is load-bearing:
    /// 108 ungated (last inherited player method) ↔ 109 gated (first GM
    /// method). A revert of the `index >= 109` arm lets a non-GM client
    /// reach the GM tail.
    #[test]
    fn entire_sgwgmplayer_tail_is_gated() {
        assert_eq!(SGWGMPLAYER_CELL_METHOD_BASE, 109);
        assert!(
            !requires_gm(108),
            "108 is the last inherited SGWPlayer method — must stay ungated"
        );
        // Spot-check the boundary + a spread of gm* indices including
        // implemented ones (133 gmGiveItem, 163 gmGotoXYZ, 190 gmKillTarget)
        // and the top of the table (225 changeCoverStanceWeight).
        for idx in [109u16, 110, 133, 160, 163, 190, 225, 300, u16::MAX] {
            assert!(
                requires_gm(idx),
                "index {idx} is in the SGWGmPlayer tail and must be GM-gated"
            );
        }
    }

    #[test]
    fn access_level_mapping_clamps_high_values() {
        assert_eq!(access_level_from_u32(0), AccessLevel::Player);
        assert_eq!(access_level_from_u32(2), AccessLevel::GameMaster);
        assert_eq!(access_level_from_u32(4), AccessLevel::Developer);
        assert_eq!(
            access_level_from_u32(999),
            AccessLevel::Developer,
            "out-of-range level must clamp to most-privileged, never lose access"
        );
    }
}
