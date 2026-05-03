//! Per-target damage application — extracted from `use_ability::handle_use_ability`
//! so AoE / ground-target paths can apply damage to multiple targets without
//! re-running the consume/cooldown/timer/sequence wire packets that should
//! fire exactly once per ability invocation.
//!
//! The function takes the already-resolved `effect_seq` and `ability_def`
//! plus a `needs_ammo_stat_send` flag that the caller controls — the primary
//! target gets `true` (the consume happened upstream and the ammo stat needs
//! to be flushed after the damage commits), AoE secondary targets get
//! `false` (the primary already flushed).

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{serialize_effect_results, AbilityDef, DT_PHYSICAL};
use cimmeria_entity::stats::HEALTH;

use super::super::combat;
use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;

use super::loot_drop::kill_xp;
use super::messaging::{flush_attacker_ammo_stat, send_entity_method};
use super::rng::pseudo_random_seed;

/// Resolve damage from `entity_id` to `target_eid` for ability `ability_id`.
///
/// Performs:
///   - Snapshot attacker stats; bail if attacker missing.
///   - Read damage NVPs from the ability's effect definitions.
///   - Compute QR + roll a hit result for this attacker/target pair.
///   - Apply health/focus damage to the target.
///   - Detect death; mutate the target's AI state, threat list, dead bit.
///   - Send `onEffectResults` to the attacker (witnesses pick it up via
///     entity routing) and to the target if the target is a player.
///   - Send `onStatUpdate` to the target.
///   - Optionally flush the attacker's dirty ammo stat (for the primary
///     consume path; AoE-secondary calls pass `false` so they don't
///     re-flush).
///   - On death, call into [`super::death::apply_death_transition`] +
///     send the death sequence + grant kill XP + onBeginAidWait for
///     player targets.
///   - On survival, call into [`combat::generate_threat`] which mirrors
///     the threat-table addition into the player's `threatened_mobs`
///     set (#92) and broadcasts the BSF_InCombat transition if needed.
///
/// `effect_seq` should be unique per (attacker, target) pair within the
/// same invocation — AoE callers should generate a fresh `effect_seq`
/// for each secondary target so the client can correlate per-target
/// effect packets independently.
pub(super) async fn apply_damage_to_target(
    entity_id: u32,
    target_eid: u32,
    ability_id: i32,
    ability_def: &Option<AbilityDef>,
    effect_seq: u32,
    needs_ammo_stat_send: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // We need both attacker and target stats. Since we can't borrow two
    // entities mutably at once, we snapshot the attacker stats first.
    let attacker_stats = match space_mgr.get_entity(entity_id) {
        Some(e) => e.stats.clone(),
        None => {
            // Defensive: entity vanished after the consume mutation. Flush
            // before exiting so the client still sees the ammo decrement.
            if needs_ammo_stat_send {
                flush_attacker_ammo_stat(entity_id, tx, space_mgr).await;
            }
            return;
        }
    };

    // Calculate QR
    let target = match space_mgr.get_entity(target_eid) {
        Some(e) => e,
        None => {
            tracing::debug!(target_eid, "apply_damage_to_target: target not found");
            // Flush ammo decrement even when target lookup fails — the shot
            // still left the chamber from the player's perspective.
            if needs_ammo_stat_send {
                flush_attacker_ammo_stat(entity_id, tx, space_mgr).await;
            }
            return;
        }
    };
    // Use the ability's actual ranged flag for the QR calculation. Defaults
    // to false when the AbilityDef is missing (unknown ability falls back to
    // a generic melee swing).
    let ability_is_ranged = ability_def.as_ref().map(|d| d.is_ranged).unwrap_or(false);
    let qr = combat::calculate_qr(&attacker_stats, &target.stats, ability_is_ranged);

    // Seed the beta-distribution sample from this ability invocation.
    // Per-(entity, ability, effect_seq) determinism — a fresh effect_seq per
    // AoE secondary target gives independent rolls without losing replay
    // reproducibility.
    let seed = pseudo_random_seed(entity_id, ability_id, effect_seq);
    let qr_result = combat::calculate_result(qr, seed);

    // Look up damage values from the ability's effect NVPs. When the
    // ability is known but exposes no positive HealthDamage (e.g. focus
    // drains, heals, buff-only abilities) we keep health_base_damage at 0
    // so the ability doesn't accidentally read as a 15-HP physical hit.
    // The 15-HP fallback is reserved for the unknown-ability case.
    let (health_base_damage, focus_base_damage) = if let Some(def) = ability_def {
        let mut h_dmg = 0i32;
        let mut f_dmg = 0i32;
        for &eid in &def.effect_ids {
            if let Some(effect) = space_mgr.effect_defs.get(&eid) {
                let hd = effect.param_i32("HealthDamage");
                let fd = effect.param_i32("FocusDamage");
                if hd > 0 {
                    h_dmg = hd;
                }
                if fd > 0 {
                    f_dmg = fd;
                }
            }
        }
        (h_dmg, f_dmg)
    } else {
        (15, 0)
    };
    // Temp: 2x player damage so players can kill NPCs before dying
    let is_player_attacker = space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player);
    let health_base_damage = if is_player_attacker {
        health_base_damage * 2
    } else {
        health_base_damage
    };

    // Apply health damage to target
    let target = match space_mgr.get_entity_mut(target_eid) {
        Some(e) => e,
        None => {
            // Flush ammo decrement before exiting — the shot was fired even
            // if the target was removed between the immutable lookup above
            // and the mutable lookup here.
            if needs_ammo_stat_send {
                flush_attacker_ammo_stat(entity_id, tx, space_mgr).await;
            }
            return;
        }
    };

    let (effect_results, _total_health_damage) = combat::calculate_damage(
        &qr_result,
        health_base_damage,
        DT_PHYSICAL,
        HEALTH,
        &attacker_stats,
        &mut target.stats,
    );

    // Apply focus damage if present
    if focus_base_damage > 0 {
        let _ = combat::calculate_damage(
            &qr_result,
            focus_base_damage,
            DT_PHYSICAL,
            cimmeria_entity::stats::FOCUS,
            &attacker_stats,
            &mut target.stats,
        );
    }

    // Check if target died — use entity's state_field so we preserve other flags
    let target_died = target.stats.get(HEALTH).is_some_and(|s| s.cur <= 0);
    if target_died {
        target.set_state_flag(combat::BSF_DEAD);
        target.set_state_flag(combat::BSF_MOVEMENT_LOCK); // prevent movement while dead
                                                          // Transition NPC AI to Dead so it stops fighting and moving
        if !target.is_player {
            target.ai_state = cimmeria_entity::cell_entity::AiState::Dead;
            target.threat_list.clear();
            target.nav_path.clear();
            target.velocity = [0.0; 3]; // Stop movement interpolation
                                        // NOTE: do NOT zero `interaction_type_flags` here. Python `SGWMob.onDead()`
                                        // OR-merges `INT_NormalLoot` and preserves all other bits — content-driven
                                        // bits (quest tags, mission interactions) must survive death. The dead-state
                                        // bit handles cursor distinction client-side; we don't need to clear
                                        // `INT_Attackable` to suppress the shootable cursor.
                                        // BSF_IN_COMBAT is intentionally written via raw bitmask ops
                                        // across the codebase (use_ability sets it on weapon-fire,
                                        // threat module clears it when the threat list drains). Mixing
                                        // it with `unset_state_flag` here would hit the zero-counter
                                        // no-op path and leave the bit stuck — NPCs would render as
                                        // still-in-combat after death. Mirrors python SGWMob.py:292.
            target.state_field &= !combat::BSF_IN_COMBAT;
        }
        tracing::info!(
            attacker = entity_id,
            target = target_eid,
            ability_id,
            is_npc = !target.is_player,
            "Target killed!"
        );
    }
    let target_state = target.state_field;

    // Serialize dirty stats for the target
    let target_stat_update = target.stats.serialize_dirty();
    let _target_public_stat_update = target.stats.serialize_dirty_public();
    target.stats.clear_dirty();

    // ── Send effect results ──

    // onEffectResults — send to both attacker and target, but avoid double-sending.
    // If attacker is a player and target is an NPC, the witness routing on the NPC
    // already reaches the player. So only send to the attacker directly + NPC witnesses.
    let effect_args = serialize_effect_results(
        entity_id as i32, // source
        ability_id,
        effect_seq as i32, // effect ID (using sequence as stub)
        target_eid as i32, // target
        qr_result.result_code,
        &effect_results,
    );

    let attacker_is_player = space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player);
    let target_is_player = space_mgr
        .get_entity(target_eid)
        .is_some_and(|e| e.is_player);

    // Send to attacker
    send_entity_method(
        entity_id,
        crate::mercury::method_idx::ON_EFFECT_RESULTS,
        effect_args.clone(),
        tx,
        space_mgr,
    )
    .await;

    // Only send to target if the target is a player (so they see incoming damage).
    // If target is an NPC, the attacker (as witness) already received it above via
    // send_entity_method routing.
    if target_is_player && !attacker_is_player {
        send_entity_method(
            target_eid,
            crate::mercury::method_idx::ON_EFFECT_RESULTS,
            effect_args,
            tx,
            space_mgr,
        )
        .await;
    }

    // ── Send stat updates ──

    // onStatUpdate to target (their health bar changes)
    send_entity_method(
        target_eid,
        crate::mercury::method_idx::ON_STAT_UPDATE,
        target_stat_update,
        tx,
        space_mgr,
    )
    .await;

    // onStatUpdate to attacker — drains AmmoSlot{N} dirty bits set by
    // `set_slot_ammo` on the consume path so the bandolier UI updates on every
    // shot. Skipped if the consume path didn't run (no ammo cost, NPC
    // attacker, or AoE secondary target where the primary already flushed).
    if needs_ammo_stat_send {
        flush_attacker_ammo_stat(entity_id, tx, space_mgr).await;
    }

    // ── Death side effects ──
    // Wire-protocol burst (target reticle, BSF_InCombat clear, loot + InteractionType,
    // dead-state flip) lives in `death::apply_death_transition` so the ordering
    // constraints documented there stay in one place.

    if target_died {
        super::death::apply_death_transition(
            target_eid,
            entity_id,
            target_state,
            attacker_is_player,
            target_is_player,
            tx,
            space_mgr,
        )
        .await;

        // Send death animation via onSequence (Entity_Death = event_id 5001)
        // Look up the death sequence from the target's event set via sequence_map
        {
            const EVENT_ENTITY_DEATH: i32 = 5001;
            // Event set 1025 (Mob) drives the death anim for both NPCs and
            // players today. If they ever diverge, branch on `e.is_player`.
            let event_set_id = space_mgr.get_entity(target_eid).map(|_| 1025);
            if let Some(esid) = event_set_id {
                if let Some(&death_seq_id) = space_mgr.sequence_map.get(&(esid, EVENT_ENTITY_DEATH))
                {
                    let mut seq_args = Vec::with_capacity(28);
                    seq_args.extend_from_slice(&death_seq_id.to_le_bytes()); // KismetEventSetSeqID
                    seq_args.extend_from_slice(&(target_eid as i32).to_le_bytes()); // SourceID (dying entity)
                    seq_args.extend_from_slice(&(target_eid as i32).to_le_bytes()); // TargetID (also dying entity — NOT killer, or client plays death anim on killer)
                    seq_args.push(1); // PrimaryTarget
                    seq_args.extend_from_slice(&0.0f32.to_le_bytes()); // ImpactTime
                    seq_args.extend_from_slice(&0u32.to_le_bytes()); // NameValuePairs count
                    seq_args.push(0); // ViewType
                    seq_args.extend_from_slice(&0i32.to_le_bytes()); // InstanceId
                    send_entity_method(
                        target_eid,
                        crate::mercury::method_idx::ON_SEQUENCE,
                        seq_args,
                        tx,
                        space_mgr,
                    )
                    .await;
                }
            }
        }

        // Grant XP to the attacker if the target is a non-player entity
        if let Some(target) = space_mgr.get_entity(target_eid) {
            if !target.is_player {
                let xp = kill_xp(target.level);
                tracing::info!(
                    attacker = entity_id,
                    target = target_eid,
                    mob_level = target.level,
                    xp,
                    "Granting kill XP"
                );
                if let Err(e) = tx
                    .send(CellToBaseMsg::GrantXP {
                        entity_id,
                        xp_amount: xp,
                    })
                    .await
                {
                    tracing::error!(
                        attacker = entity_id, target = target_eid, xp,
                        error = %e,
                        "GrantXP send to base failed -- player kill credit lost"
                    );
                }
            }
        }

        // Send onBeginAidWait to player so they see the Defeat Window
        // Reference: python/cell/SGWPlayer.py:1278 — self.client.onBeginAidWait(100, respawnerList)
        if target_is_player {
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
                "Sent onBeginAidWait (Defeat Window)"
            );
        }
    }

    // Generate threat on surviving NPCs so they aggro back. If this hit
    // is what put the player into combat (their threatened_mobs went
    // from empty → {target}), broadcast the new state_field so the
    // client flips its in-combat HUD/cursor routing.
    if !target_died {
        if let Some(new_state) = combat::generate_threat(
            space_mgr,
            entity_id,
            target_eid,
            _total_health_damage as f32,
        ) {
            send_entity_method(
                entity_id,
                crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
                new_state.to_le_bytes().to_vec(),
                tx,
                space_mgr,
            )
            .await;
        }
    }
}
