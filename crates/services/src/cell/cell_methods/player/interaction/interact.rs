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
    if !space_mgr
        .get_entity(entity_id)
        .is_some_and(|actor| !crate::cell::combat::is_dead_state(actor.state_field))
    {
        tracing::debug!(entity_id, "interact: actor missing or dead");
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
        !t.is_player && t.faction == 10 && !crate::cell::combat::is_dead_state(t.state_field)
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
                entity_id,
                target_entity_id,
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

    // Server-authority gate for everything below: the target must exist
    // and be within `MAX_INTERACT_DISTANCE`.
    //
    // `interactions::handle_interact` has always checked this, but it is
    // the LAST thing this function tries. The trainer UI and the
    // content-chain dispatch both run ahead of it and so never inherited
    // the check, which meant a client could name any tagged NPC anywhere
    // on the map and fire its chains — accepting missions, advancing
    // steps, launching minigames — from arbitrary distance. Moving the
    // `last_interaction_target` pin ahead of the chain dispatch (below)
    // made that worse, because `handle_initial_response` stamps the pin
    // straight onto the wire as an `onDialogDisplay` EntityId, so an
    // unvalidated id could reach the client.
    //
    // Deliberately placed AFTER the hostile-NPC combat reroute above:
    // attacks have their own range rules and gating them on the 5-unit
    // interaction distance would break every ranged weapon.
    if !crate::cell::interactions::interact_target_in_range(entity_id, target_entity_u32, space_mgr)
    {
        return;
    }

    // Pin the interaction target BEFORE any chain dispatch, mirroring
    // python's `SGWPlayer.interact()`, which writes
    // `lastInteractionTarget` as its first act.
    //
    // `interactions::handle_interact` also writes this pin, but it only
    // runs in the `if !handled` fall-through below — i.e. only when NO
    // content chain claimed the interact. That left the pin stale for
    // every chain-handled NPC, and the pin is the second resolution step
    // for `display_dialog`'s wire `EntityId`
    // (`content/executor/dialog.rs`): chain params carry
    // `target_entity_id` only for the `interact_tag` / `interact_template`
    // trigger itself, never for a follow-up. So a chain fired from
    // `dialog_choice`, from a minigame victory (`fire_chain_by_id` passes
    // empty params by construction), or from the deferred-action drain
    // could not resolve a speaker at all, and any NPC-speaker dialog it
    // tried to display hit the warn-and-bail branch and silently never
    // opened. Only monologue dialogs (every screen `speaker_id = 0`)
    // survived, because those bind the player and need no NPC at all.
    //
    // Deliberately placed after the hostile-combat reroute above: an
    // attack must not pin its victim as the next dialog's speaker. This
    // is also the reason the write is here rather than beside the
    // `target_entity_u32` binding at the top of the function.
    //
    // The target has already been validated at this point: the
    // `interact_target_in_range` gate directly above returned early
    // unless the entity exists and is within `MAX_INTERACT_DISTANCE`. So
    // the id pinned here is one the player could legitimately reach,
    // which matters because `interactions/dispatch/initial_response.rs`
    // stamps this pin straight onto the wire as an `onDialogDisplay`
    // EntityId.
    if let Some(player) = space_mgr.get_entity_mut(entity_id) {
        player.last_interaction_target = Some(target_entity_u32);
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
    let mut handled =
        crate::cell::interactions::try_open_trainer(entity_id, target_entity_u32, tx, space_mgr)
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
        let dialog_id =
            crate::cell::interactions::handle_interact(entity_id, target_entity_u32, tx, space_mgr)
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

#[cfg(test)]
mod liveness_tests {
    use super::*;
    use crate::cell::combat::BSF_DEAD;
    use crate::test_support::make_space_manager;
    use cimmeria_content_engine::{actions::Action, chain::Chain, triggers::Trigger};
    use cimmeria_entity::cell_entity::{LootItem, NpcInteractionType};
    use cimmeria_entity::stats::HEALTH;

    #[derive(Clone, Copy, Debug)]
    enum Route {
        Hostile,
        Trainer,
        Tag,
        Template,
        Loot,
    }

    fn dead_actor_fixture(route: Route) -> (SpaceManager, ChainEngine, u32) {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        let actor = mgr.get_entity_mut(1).unwrap();
        actor.player_id = Some(42);
        actor.archetype_id = Some(2);
        actor.set_state_flag(BSF_DEAD);
        actor.last_interaction_target = Some(99);
        actor.looting_entity = Some(98);
        actor.vendor_entity = Some(97);
        actor.offer_dialog(96);
        actor.counters.insert("alive_probe".into(), 7);
        assert!(actor.stats.get(HEALTH).unwrap().cur > 0);

        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc(npc_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        let npc = mgr.get_entity_mut(npc_id).unwrap();
        npc.faction = 1;
        npc.clear_all_state_flags();
        npc.npc_name = Some("LivenessProbe".into());
        let mut engine = ChainEngine::new();
        match route {
            Route::Hostile => npc.faction = 10,
            Route::Trainer => {
                npc.template_id = Some(700);
                mgr.template_trainer_lists.insert(700, 701);
            }
            Route::Loot => {
                npc.faction = 10;
                npc.set_state_flag(BSF_DEAD);
                npc.interaction_type = Some(NpcInteractionType::Loot);
                npc.loot.push(LootItem {
                    design_id: None,
                    quantity: 50,
                    index: 1,
                });
            }
            Route::Tag | Route::Template => {
                let trigger = if matches!(route, Route::Tag) {
                    npc.tag = Some("LivenessProbe".into());
                    Trigger::OnInteractTag {
                        entity_tag: "LivenessProbe".into(),
                    }
                } else {
                    Trigger::OnInteractTemplate {
                        template_name: "LivenessProbe".into(),
                    }
                };
                engine.register_chain(Chain {
                    id: 700,
                    name: "interaction liveness probe".into(),
                    enabled: true,
                    trigger,
                    conditions: vec![],
                    actions: vec![Action::IncrementCounter {
                        counter_name: "alive_probe".into(),
                        amount: 1,
                    }],
                    action_delays: vec![],
                    priority: 0,
                });
            }
        }
        (mgr, engine, npc_id)
    }

    #[tokio::test]
    async fn dead_actor_preserves_state_and_emits_nothing_on_every_interact_route() {
        for route in [
            Route::Hostile,
            Route::Trainer,
            Route::Tag,
            Route::Template,
            Route::Loot,
        ] {
            let (mut mgr, engine, npc_id) = dead_actor_fixture(route);
            let (tx, mut rx) = mpsc::channel(16);
            handle_interact(1, &(npc_id as i32).to_le_bytes(), &tx, &mut mgr, &engine).await;
            assert!(
                rx.try_recv().is_err(),
                "dead actor emitted a message via {route:?}"
            );
            let actor = mgr.get_entity(1).unwrap();
            assert_eq!(actor.last_interaction_target, Some(99), "{route:?}");
            assert_eq!(actor.looting_entity, Some(98), "{route:?}");
            assert_eq!(actor.vendor_entity, Some(97), "{route:?}");
            assert_eq!(actor.offered_dialogs(), vec![96], "{route:?}");
            assert_eq!(actor.counters.get("alive_probe"), Some(&7), "{route:?}");
            assert!(crate::cell::combat::is_dead_state(actor.state_field));
        }
    }

    #[tokio::test]
    async fn missing_actor_cannot_emit_hostile_target_update() {
        let (mut mgr, engine, npc_id) = dead_actor_fixture(Route::Hostile);
        let (tx, mut rx) = mpsc::channel(16);
        assert!(mgr.get_entity(2).is_none());
        handle_interact(2, &(npc_id as i32).to_le_bytes(), &tx, &mut mgr, &engine).await;
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn clearing_dead_state_allows_tag_and_template_interactions_again() {
        for route in [Route::Tag, Route::Template] {
            let (mut mgr, engine, npc_id) = dead_actor_fixture(route);
            mgr.get_entity_mut(1).unwrap().clear_all_state_flags();
            let (tx, _rx) = mpsc::channel(16);
            handle_interact(1, &(npc_id as i32).to_le_bytes(), &tx, &mut mgr, &engine).await;
            let actor = mgr.get_entity(1).unwrap();
            assert_eq!(actor.last_interaction_target, Some(npc_id), "{route:?}");
            assert_eq!(actor.counters.get("alive_probe"), Some(&8), "{route:?}");
        }
    }
}
