//! Post-transition death side effects — the death animation, the kill-XP
//! grant, and the player Defeat Window.
//!
//! These used to live inline in `damage_apply`, which is why an
//! effect-script kill produced none of them (see [`super::resolve_death`]).
//! They sit behind the wire burst in [`super::apply_death_transition`]
//! because each one depends on the corpse already carrying `BSF_DEAD`:
//! the animation plays on a dead-state model, the XP grant is the
//! once-per-corpse payout, and the Defeat Window is only meaningful once
//! the client has flipped the dying player into the dead state.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::loot_drop::kill_xp;
use super::super::messaging::{send_entity_method, send_entity_method_to_self_and_witnesses};

/// Kismet event id for the death animation. Event set 1025 (Mob) drives
/// it for both NPCs and players today; if they ever diverge, branch on
/// `e.is_player` at the lookup below.
const EVENT_ENTITY_DEATH: i32 = 5001;

/// Broadcast the `onSequence` death animation for `target_eid`.
///
/// Fans to self+witnesses so a spectator sees the entity fall. Silently
/// no-ops when the entity's event set has no `Entity_Death` sequence —
/// content without a death anim is normal, not an error.
pub(super) async fn send_death_sequence(
    target_eid: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(esid) = space_mgr.get_entity(target_eid).map(|_| 1025) else {
        return;
    };
    let Some(&death_seq_id) = space_mgr.sequence_map.get(&(esid, EVENT_ENTITY_DEATH)) else {
        return;
    };

    let mut seq_args = Vec::with_capacity(28);
    seq_args.extend_from_slice(&death_seq_id.to_le_bytes()); // KismetEventSetSeqID
    seq_args.extend_from_slice(&(target_eid as i32).to_le_bytes()); // SourceID (dying entity)
    seq_args.extend_from_slice(&(target_eid as i32).to_le_bytes()); // TargetID (also dying entity — NOT killer, or client plays death anim on killer)
    seq_args.push(1); // PrimaryTarget
    seq_args.extend_from_slice(&0.0f32.to_le_bytes()); // ImpactTime
    seq_args.extend_from_slice(&0u32.to_le_bytes()); // NameValuePairs count
    seq_args.push(0); // ViewType
    seq_args.extend_from_slice(&0i32.to_le_bytes()); // InstanceId

    send_entity_method_to_self_and_witnesses(
        target_eid,
        crate::mercury::method_idx::ON_SEQUENCE,
        seq_args,
        tx,
        space_mgr,
    )
    .await;
    tracing::debug!(
        target: "abilities.sequence",
        event = "entity_death",
        source_id = target_eid,
        target_id = target_eid,
        sequence_id = death_seq_id,
        event_set_id = esid,
        "onSequence broadcast: Entity_Death (death animation)"
    );
}

/// Grant kill XP for a dead NPC to `attacker_id`.
///
/// No-op for player targets — PvP pays no XP. The `GrantXP` hop is the
/// only thing standing between a kill and the player's level bar, so a
/// send failure is an `error!`, not a silent drop.
pub(super) async fn grant_kill_xp(
    target_eid: u32,
    attacker_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(target) = space_mgr.get_entity(target_eid) else {
        return;
    };
    if target.is_player {
        return;
    }
    let xp = kill_xp(target.level);
    tracing::info!(
        attacker = attacker_id,
        target = target_eid,
        mob_level = target.level,
        xp,
        "Granting kill XP"
    );
    if let Err(e) = tx
        .send(CellToBaseMsg::GrantXP {
            entity_id: attacker_id,
            xp_amount: xp,
            // Mob-kill XP is not GM-sourced — no GM feedback line.
            gm_feedback_to: None,
        })
        .await
    {
        tracing::error!(
            attacker = attacker_id, target = target_eid, xp,
            error = %e,
            "GrantXP send to base failed -- player kill credit lost"
        );
    }
}

/// Send `onBeginAidWait` so a dead player sees the Defeat Window with the
/// respawner list for their world.
///
/// Reference: python/cell/SGWPlayer.py:1278 —
/// `self.client.onBeginAidWait(100, respawnerList)`.
pub(super) async fn send_begin_aid_wait(
    target_eid: u32,
    attacker_id: u32,
    ability_id: Option<i32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    // Look up respawners for the player's current world
    let world_name = space_mgr.get_entity_world_name(target_eid);
    let matching_respawners: Vec<_> = if let Some(ref wn) = world_name {
        space_mgr
            .respawners
            .iter()
            .filter(|r| r.world_name == *wn)
            .collect()
    } else {
        vec![]
    };

    let (px, py, pz) = space_mgr
        .get_entity(target_eid)
        .map_or((0.0, 0.0, 0.0), |p| {
            (p.position.x, p.position.y, p.position.z)
        });
    let killer_name = space_mgr
        .get_entity(attacker_id)
        .and_then(|k| k.npc_name.clone().or_else(|| k.character_name.clone()))
        .unwrap_or_default();
    let id = space_mgr.player_identity(target_eid);
    tracing::info!(
        target: "player.death",
        entity_id = target_eid,
        account_id = id.account_id,
        player_id = id.player_id,
        killer = attacker_id,
        killer_name = %killer_name,
        ability_id = ability_id.unwrap_or(-1),
        world = ?world_name,
        x = px,
        y = py,
        z = pz,
        "player death"
    );

    let mut aid_args = Vec::with_capacity(64);
    // INT32: TimeToAid (seconds until auto-respawn)
    aid_args.extend_from_slice(&30i32.to_le_bytes());

    if matching_respawners.is_empty() {
        // Fallback: single entry with chardef spawn position
        aid_args.extend_from_slice(&1u32.to_le_bytes()); // array count
        aid_args.extend_from_slice(&0i32.to_le_bytes()); // respawnerID = 0 (default)
        crate::mercury::write_wstring(&mut aid_args, "Respawn Point");
    } else {
        aid_args.extend_from_slice(&(matching_respawners.len() as u32).to_le_bytes());
        for resp in &matching_respawners {
            aid_args.extend_from_slice(&resp.respawner_id.to_le_bytes());
            crate::mercury::write_wstring(&mut aid_args, &resp.name);
        }
    }

    send_entity_method(
        target_eid,
        crate::mercury::method_idx::ON_BEGIN_AID_WAIT,
        aid_args,
        tx,
        space_mgr,
    )
    .await;
    tracing::info!(
        target = target_eid,
        world = ?world_name,
        respawner_count = if matching_respawners.is_empty() { 1 } else { matching_respawners.len() },
        respawner_ids = ?matching_respawners.iter().map(|r| r.respawner_id).collect::<Vec<_>>(),
        filter = "world_name_only",
        "Sent onBeginAidWait (Defeat Window)"
    );
    crate::cell::player_journal::note(
        target_eid,
        crate::cell::player_journal::kinds::DEATH,
        format!("world={world_name:?}"),
    );
}
