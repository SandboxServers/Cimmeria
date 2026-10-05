//! An NPC's combat stance: `BSF_InCombat` while it is Fighting, and the
//! `onStateFieldUpdate` that tells its witnesses.
//!
//! **Why it exists.** The legacy `SGWMob.aiBeginCombat` set `BSF_InCombat`
//! the moment a mob entered combat and `doAiAction` cleared it when the
//! threat list emptied (`deprecated/python/cell/SGWMob.py:158-163,292`), and
//! `setStateFlag` broadcast every change to the mob's witnesses
//! (`SGWBeing.py:746-755`). The Rust restoration only ever set the bit on
//! *players* (`combat::generate_threat` → `enter_player_combat`); on the NPC
//! side "nothing ever calls `set_state_flag(BSF_IN_COMBAT)`"
//! (`lifecycle::npc_ai_submit`). An NPC therefore fired every shot from its
//! spawn stance.
//!
//! **What the client does with the bit.** `Event_NetIn_onStateFieldUpdate`
//! lands in `GameBeing::OnStateFieldUpdate` (`0x00e01c90`), which XORs the new
//! field against its cached copy and, for a `BSF_InCombat` (bit 3) change,
//! calls `UpdateCombatStanceWeaponSet` (`0x00e7b4c0`): the pawn's
//! stance / weapon-category animation set is re-keyed. Every weapon sequence
//! the server plays afterwards (`KIS-SA_Sing_Source`, `KIS-SA_Burst_Source`)
//! reads the Source pawn's weapon component slot (`SeqAct_ComponentSlotData`
//! → `SeqAct_ComponentSlotAttachment`) before it spawns the muzzle flash and
//! tracer emitters and before it looks up the weapon sound.
//!
//! **What the colo telemetry showed** (2026-09-29, session `dc4c716a`): the
//! same `onSequence` (ability 579 → sequence 3) spawned an
//! `EmitterSpawnable` and a `sing` sound within ~40 ms of arriving when the
//! *player* was the Source, and nothing at all for 11 consecutive shots from
//! guard 100307. `docs/reverse-engineering/findings/npc-attack-presentation.md`
//! has the evidence and what is still unproven.
//!
//! **The rule.** The bit is derived, never independently owned: it is set
//! exactly while `ai_state == Fighting`. [`sync_combat_stance`] is the only
//! writer; it runs at the top of every Fighting pass (so the stance precedes
//! the first shot on the reliable channel) and once per AI tick for every
//! other ticked state (so the clear follows a leash, a submit or a give-up).
//! `set_ai_state_on` cannot broadcast (no `tx`), which is why the derived bit
//! is reconciled here rather than at the transition.
//!
//! What was last announced is kept in an [`NpcCombatStance`] extension. The
//! flag is trusted only while the bit is still set: the death path, the
//! respawn reset (`clear_all_state_flags`) and `npc_ai_submit` clear the bit
//! without a wire message of their own, and each leaves the flag `true` with
//! the bit `0` -- which this module reads as "witnesses still believe the NPC
//! is in combat".

use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::AiState;

use crate::cell::abilities::send_entity_method_to_witnesses;
use crate::cell::combat::BSF_IN_COMBAT;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// What the witnesses of one NPC were last told about its combat stance.
///
/// Stored in `CellEntity::extensions`; absent means "never announced".
#[derive(Debug, Clone, Copy, Default)]
pub(in crate::cell) struct NpcCombatStance {
    /// The last `onStateFieldUpdate` this module sent carried `BSF_InCombat`.
    announced_in_combat: bool,
}

/// Whether `npc` is announced to its witnesses as being in combat right now:
/// the bit is set and the last announcement said so.
///
/// The ability-sequence WARN (`abilities.sequence`, `outcome =
/// "stance_not_announced"`) asks this for an NPC attacker.
pub(in crate::cell) fn stance_announced(npc: &cimmeria_entity::cell_entity::CellEntity) -> bool {
    npc.state_field & BSF_IN_COMBAT != 0
        && npc
            .extensions
            .get::<NpcCombatStance>()
            .is_some_and(|s| s.announced_in_combat)
}

/// Bring `npc_id`'s `BSF_InCombat` bit and its witnesses in line with its AI
/// state, sending one `onStateFieldUpdate` when the announced stance changes.
///
/// A no-op for a player, a missing entity, and an NPC whose witnesses already
/// hold the right answer.
pub(super) async fn sync_combat_stance(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(npc) = space_mgr.get_entity_mut(npc_id) else {
        return;
    };
    if npc.is_player {
        return;
    }
    let fighting = npc.ai_state() == AiState::Fighting;
    let bit_was_set = npc.state_field & BSF_IN_COMBAT != 0;
    let told_in_combat = npc
        .extensions
        .get::<NpcCombatStance>()
        .is_some_and(|s| s.announced_in_combat);

    // Keep the bit the AoI cascade reads (`NpcAoIData::state_field`) right
    // for a witness that arrives mid-fight, whether or not anything is sent.
    if fighting {
        npc.state_field |= BSF_IN_COMBAT;
    } else {
        npc.state_field &= !BSF_IN_COMBAT;
    }

    // Fighting: announce unless witnesses were told AND nothing has cleared
    // the bit since. Not fighting: announce the clear only if they were told.
    let announce = if fighting {
        !told_in_combat || !bit_was_set
    } else {
        told_in_combat
    };
    if !announce {
        return;
    }
    npc.extensions.insert(NpcCombatStance {
        announced_in_combat: fighting,
    });
    let state_field = npc.state_field;
    let template_id = npc.template_id;

    let witness_count = send_entity_method_to_witnesses(
        npc_id,
        crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
        state_field.to_le_bytes().to_vec(),
        tx,
        space_mgr,
    )
    .await;
    let book = cimmeria_names::book();
    tracing::debug!(
        target: "npc_ai.stance",
        event = if fighting { "in_combat" } else { "out_of_combat" },
        npc_id,
        npc_name = space_mgr.entity_label(npc_id),
        template_id,
        template_name = template_id.and_then(|t| book.template(t)),
        state_field,
        state_field_names = %cimmeria_wire::state_field::STATE_FLAGS.render(state_field),
        witness_count,
        "npc_ai: combat stance announced ({})",
        if fighting { "in combat" } else { "out of combat" },
    );
}
