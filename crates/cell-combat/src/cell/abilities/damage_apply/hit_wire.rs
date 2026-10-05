//! A hit's wire burst: `onEffectResults` to the attacker's and (for a player
//! target) the target's audiences, then the target's `onStatUpdate`. Each
//! send writes its `abilities.wire` row (AB-T4).

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{serialize_effect_results, ClientEffectResult};

use super::super::messaging::WireRoute;
use super::super::wire_ledger::{self, WireCtx};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// One resolved hit, as the client hears it.
#[derive(Debug)]
pub(super) struct HitWire<'a> {
    pub entity_id: u32,
    pub target_eid: u32,
    pub ability_id: i32,
    /// The effect id the client receives; the cast's `cast_id` (AB-T1) for
    /// a single-target cast.
    pub effect_seq: u32,
    pub result_code: u8,
    pub effect_results: &'a [ClientEffectResult],
    /// The target's dirty stats, already serialized and cleared.
    pub target_stat_update: Vec<u8>,
}

pub(super) async fn send_hit_results(
    hit: HitWire<'_>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let HitWire {
        entity_id,
        target_eid,
        ability_id,
        effect_seq,
        result_code,
        effect_results,
        target_stat_update,
    } = hit;
    // onEffectResults — send to both attacker and target, but avoid double-sending.
    // If attacker is a player and target is an NPC, the witness routing on the NPC
    // already reaches the player. So only send to the attacker directly + NPC witnesses.
    let effect_args = serialize_effect_results(
        entity_id as i32, // source
        ability_id,
        effect_seq as i32, // effect ID (using sequence as stub)
        target_eid as i32, // target
        result_code,
        effect_results,
    );
    let target_is_player = space_mgr
        .get_entity(target_eid)
        .is_some_and(|e| e.is_player);
    let ctx = WireCtx::new("damage_apply").ability(ability_id);

    // Fan out the attacker's effect results to self + all AoI witnesses so a
    // spectator sees the ability fire. For NPC attackers the self send is a
    // no-op (NPCs have no client) and this collapses to witness-only.
    wire_ledger::send(
        entity_id,
        crate::mercury::method_idx::ON_EFFECT_RESULTS,
        effect_args.clone(),
        WireRoute::SelfAndWitnesses,
        ctx,
        tx,
        space_mgr,
    )
    .await;

    // On-target effect results — fan out for any player target so witnesses
    // see the hit land on them. For NPC targets the attacker's self+witness
    // send above already carries the result (entity_id = attacker, target_eid
    // in the payload). Player targets are tracked by a different entity_id, so
    // they need a separate fanout keyed on target_eid.
    if target_is_player {
        wire_ledger::send(
            target_eid,
            crate::mercury::method_idx::ON_EFFECT_RESULTS,
            effect_args,
            WireRoute::SelfAndWitnesses,
            ctx,
            tx,
            space_mgr,
        )
        .await;
    }

    // onStatUpdate to target — health bar changes must reach witnesses so the
    // spectator sees the health drain. Fan out to self+witnesses of the target.
    wire_ledger::send(
        target_eid,
        crate::mercury::method_idx::ON_STAT_UPDATE,
        target_stat_update,
        WireRoute::SelfAndWitnesses,
        ctx,
        tx,
        space_mgr,
    )
    .await;
}
