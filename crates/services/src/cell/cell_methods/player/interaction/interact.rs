//! The `interact` (right-click) handler: hostile-NPC combat reroute,
//! trainer-UI open, and the tag/template/dialog interaction fall-through.

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

/// Handle the `interact(target_entity_id)` cell method. Args are the raw
/// 4-byte LE target id; callers pass through the wire payload unchanged.
pub(super) async fn handle_interact(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    if args.len() < 4 {
        return;
    }
    let target_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
    tracing::info!(entity_id, target_entity_id, "interact");

    // Reject negative target_entity_id rather than sign-extending into a
    // high u32 that no real entity will match.
    let target_entity_u32 = match u32::try_from(target_entity_id) {
        Ok(v) => v,
        Err(_) => {
            tracing::warn!(
                entity_id,
                target_entity_id,
                "interact: negative target_entity_id, ignoring"
            );
            return;
        }
    };

    // Reroute interact→useAbility for ALIVE hostile NPCs only. A dead
    // hostile NPC is a lootable corpse — its right-click MUST reach
    // `handle_interact` so the loot window opens. Without the dead check,
    // every right-click on a corpse silently became an auto-attack and
    // the loot interaction never fired (proven via x32dbg trace at
    // FUN_00e84b20: client correctly sent interact, server intercepted).
    let is_hostile = space_mgr.get_entity(target_entity_u32).is_some_and(|t| {
        !t.is_player
            && t.faction == 10
            && !crate::cell::combat::is_dead_state(t.state_field)
    });
    if is_hostile {
        tracing::info!(
            entity_id,
            target_entity_id,
            "interact: targeting hostile NPC for combat"
        );
        let mut reply = Vec::with_capacity(4);
        reply.extend_from_slice(&target_entity_id.to_le_bytes());
        if let Err(e) = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: 16,
                args: reply,
            })
            .await
        {
            tracing::warn!(
                entity_id, target_entity_id,
                "interact: cell->base channel closed sending hostile-NPC combat method: {e}"
            );
            return;
        }

        // Resolve the ability for the equipped weapon via
        // `items_event_sets` (EVENT_ITEM_RANGED=7). Pre-fix
        // this was a hardcoded `592` (Pistol Shot), which
        // fired regardless of the weapon — so a P90 player
        // still got Pistol Shot animations and the SMG's
        // proper `559 Automatic Weapon Auto Attack` binding
        // was dead code.
        //
        // Two fallback paths:
        // - **Unarmed** (no item in the active bandolier slot)
        //   → `594 Strike`. Firing a gun animation while
        //   empty-handed renders nonsense; Strike is the
        //   correct melee primitive.
        // - **Item present but no `items_event_sets` row**
        //   (content gap) → `592 Pistol Shot`. Logged at
        //   `warn!` with stable `target: "abilities"` so SigNoz
        //   surfaces unbound items via
        //   `event = "weapon_unbound"` — operators can grep
        //   for content rows missing their RANGED binding.
        const RIGHT_CLICK_FALLBACK_RANGED: i32 = 592;
        const RIGHT_CLICK_FALLBACK_MELEE: i32 = 594;
        let active_item_id = space_mgr.get_entity(entity_id).and_then(|e| {
            let slot = e.active_bandolier_slot;
            e.bandolier_items.get(&slot).map(|b| b.item_id)
        });
        let resolved_ability = match active_item_id {
            None => {
                tracing::debug!(
                    entity_id,
                    target_entity_id,
                    "interact: unarmed → ability 594 (Strike)"
                );
                RIGHT_CLICK_FALLBACK_MELEE
            }
            Some(item_id) => crate::cell::abilities::ability_for_item(
                space_mgr,
                item_id,
                crate::cell::spawner::EVENT_ITEM_RANGED,
            )
            .unwrap_or_else(|| {
                tracing::warn!(
                    target: "abilities",
                    event = "weapon_unbound",
                    entity_id,
                    target_entity_id,
                    item_id,
                    "interact: no items_event_sets binding for active \
                     weapon (EVENT_ITEM_RANGED=7) — content gap; \
                     falling back to ability 592 (Pistol Shot)"
                );
                RIGHT_CLICK_FALLBACK_RANGED
            }),
        };

        // Single canonical kill-credit path — see
        // `handle_use_ability_with_kill_credit` for the
        // alive→dead detection + `fire_entity_death` wrap
        // that previously lived inline here. Every player-
        // attack path that reaches `handle_use_ability`
        // for a single target routes through this helper
        // so quest KillCount objectives advance uniformly,
        // regardless of which entry point fired the shot
        // (manual right-click, interact, auto-cycle loop,
        // queued attack-while-holstered).
        crate::cell::abilities::handle_use_ability_with_kill_credit(
            entity_id,
            resolved_ability,
            target_entity_id,
            engine,
            tx,
            space_mgr,
        )
        .await;
        return;
    }

    // Trainer NPC check — runs BEFORE the tag/template chain
    // dispatch so a trainer's UI opens directly rather than the
    // generic dialog. A trainer is any NPC whose template_id has
    // a non-NULL `trainer_ability_list_id` (loaded once at
    // startup into `space_mgr.template_trainer_lists`).
    //
    // `crate::cell::interactions::trainer::try_open_trainer`
    // is the single source-of-truth path. The fallback below
    // (`handle_interact`) handles non-trainer interaction types
    // and the deprecated `NpcInteractionType::Trainer` tag arm.
    let mut handled = crate::cell::interactions::try_open_trainer(
        entity_id,
        target_entity_u32,
        tx,
        space_mgr,
    )
    .await;

    if !handled {
        if let Some(target) = space_mgr.get_entity(target_entity_u32) {
            let tag = target.tag.clone();
            let template_name = target.npc_name.clone();
            let player_id = space_mgr
                .get_entity(entity_id)
                .and_then(|e| e.player_id)
                .unwrap_or(0);

            if let Some(ref tag) = tag {
                handled = crate::cell::content::fire_interact_tag(
                    entity_id,
                    player_id,
                    tag,
                    target_entity_u32,
                    engine,
                    tx,
                    space_mgr,
                )
                .await;
            }

            if !handled {
                if let Some(ref name) = template_name {
                    handled = crate::cell::content::fire_interact_template(
                        entity_id,
                        player_id,
                        name,
                        target_entity_u32,
                        engine,
                        tx,
                        space_mgr,
                    )
                    .await;
                }
            }
        }
    }

    if !handled {
        let dialog_id = crate::cell::interactions::handle_interact(
            entity_id,
            target_entity_u32,
            tx,
            space_mgr,
        )
        .await;

        if let Some(did) = dialog_id {
            let player_id = space_mgr
                .get_entity(entity_id)
                .and_then(|e| e.player_id)
                .unwrap_or(0);
            crate::cell::content::fire_dialog_open(
                entity_id, player_id, did, engine, tx, space_mgr,
            )
            .await;
        }
        // Hostile NPC fall-through removed: the early branch at the top of
        // INTERACT (lines 27-42) already handles `!t.is_player && t.faction == 10`
        // and returns true, so this path can only be reached when the target is
        // a player or a non-hostile faction — neither of which should trigger
        // combat from an interact request.
    }
}
