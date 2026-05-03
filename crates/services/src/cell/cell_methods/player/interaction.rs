use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::constants::*;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) -> bool {
    match method_index {
        WHO => {
            tracing::info!(entity_id, "UNIMPLEMENTED: who");
            true
        }

        INTERACT => {
            if args.len() >= 4 {
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
                        return true;
                    }
                };

                // Reroute interact→useAbility for ALIVE hostile NPCs only. A dead
                // hostile NPC is a lootable corpse — its right-click MUST reach
                // `handle_interact` so the loot window opens. Without the dead check,
                // every right-click on a corpse silently became an auto-attack and
                // the loot interaction never fired (proven via x32dbg trace at
                // FUN_00e84b20: client correctly sent interact, server intercepted).
                let is_hostile = space_mgr.get_entity(target_entity_u32).map_or(false, |t| {
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
                        return true;
                    }

                    // Snapshot alive-before so we only fire the content-engine
                    // death event on the alive→dead transition caused by this
                    // call. Mirrors `super::combat::dispatch`'s USE_ABILITY arm.
                    // Without this, kills via right-click auto-attack (this path)
                    // never fire `OnEntityDeath` chains — mission progress that
                    // depends on tagged kills (e.g., FindAmbernol drone → step
                    // 2144 → 2343) silently stalls.
                    let was_alive_before =
                        space_mgr.get_entity(target_entity_u32).map_or(false, |t| {
                            !t.is_player
                                && t.stats
                                    .get(cimmeria_entity::stats::HEALTH)
                                    .map_or(false, |s| s.cur > 0)
                        });

                    crate::cell::abilities::handle_use_ability(
                        entity_id,
                        592,
                        target_entity_id,
                        tx,
                        space_mgr,
                    )
                    .await;

                    if was_alive_before {
                        let just_died =
                            space_mgr.get_entity(target_entity_u32).map_or(false, |t| {
                                t.stats
                                    .get(cimmeria_entity::stats::HEALTH)
                                    .map_or(false, |s| s.cur <= 0)
                            });
                        if just_died {
                            let tag = space_mgr
                                .get_entity(target_entity_u32)
                                .and_then(|t| t.tag.clone());
                            if let Some(tag) = tag {
                                match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
                                    Some(player_id) => {
                                        crate::cell::content::fire_entity_death(
                                            entity_id, player_id, &tag, engine, tx, space_mgr,
                                        )
                                        .await;
                                    }
                                    None => {
                                        tracing::warn!(
                                            entity_id, npc_tag = %tag,
                                            "Skipping entity_death event (interact path): killer entity has no player_id"
                                        );
                                    }
                                }
                            }
                        }
                    }
                    return true;
                }

                let mut handled = false;
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
            true
        }

        DIALOG_BUTTON_CHOICE => {
            if args.len() >= 8 {
                let dialog_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let button_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, dialog_id, button_id, "dialogButtonChoice");

                let player_id = space_mgr
                    .get_entity(entity_id)
                    .and_then(|e| e.player_id)
                    .unwrap_or(0);
                crate::cell::content::fire_dialog_choice(
                    entity_id, player_id, dialog_id, button_id, engine, tx, space_mgr,
                )
                .await;
            }
            true
        }

        INITIAL_RESPONSE => {
            if args.len() >= 4 {
                let interaction_set_map_id =
                    i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, interaction_set_map_id, "initialResponse");

                crate::cell::interactions::handle_initial_response(
                    entity_id,
                    interaction_set_map_id,
                    engine,
                    tx,
                    space_mgr,
                )
                .await;
            } else {
                tracing::warn!(
                    entity_id,
                    args_len = args.len(),
                    "initialResponse: truncated args, dropping"
                );
            }
            true
        }

        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::cell_entity::NpcInteractionType;

    /// Standard test space setup: SpaceManager with a single Agnos space.
    fn make_space_manager() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(spaces_xml).unwrap();
        mgr.create_startup_spaces(cell_spaces_xml).unwrap();
        mgr
    }

    /// Right-clicking an ALIVE hostile NPC reroutes interact → useAbility.
    /// Baseline for the dead-corpse regression test below.
    #[tokio::test]
    async fn alive_hostile_reroutes_to_useability() {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc(npc_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(npc_id) {
            npc.faction = 10; // hostile
            npc.clear_all_state_flags(); // alive — hard reset (counters too)
        }

        let (tx, mut rx) = mpsc::channel(16);
        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        let mut args = Vec::with_capacity(4);
        args.extend_from_slice(&(npc_id as i32).to_le_bytes());
        let handled = dispatch(1, INTERACT, &args, &tx, &mut mgr, &engine).await;
        assert!(handled);

        // Hostile reroute path sends method 16 (onTargetUpdate) first, then
        // useAbility flows downstream. We just check at least one message
        // came through and that NO loot/dialog message did.
        let mut saw_target_update = false;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
                if method_index == 16 {
                    saw_target_update = true;
                }
                // Loot display would be method 114 — must not appear.
                assert_ne!(method_index, 114, "loot must not fire for an alive hostile");
            }
        }
        assert!(
            saw_target_update,
            "expected onTargetUpdate from hostile reroute"
        );
    }

    /// Right-clicking a DEAD hostile corpse with loot must reach
    /// `handle_interact`, set `looting_entity` on the player, and emit
    /// `onLootDisplay` (method 114). Regression for the
    /// "right-click on corpse silently rerouted to useAbility" bug —
    /// see [docs/reverse-engineering/findings/right-click-routing-on-corpse.md]
    /// and [interaction.rs:37-49] (the `is_dead_state` guard).
    #[tokio::test]
    async fn dead_hostile_with_loot_routes_to_interact() {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.player_id = Some(42);
        }
        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc(npc_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(npc_id) {
            npc.faction = 10; // hostile
            npc.set_state_flag(crate::cell::combat::BSF_DEAD);
            npc.interaction_type = Some(NpcInteractionType::Loot);
            // Seed real loot. Without this, the test only proves dispatch
            // routes to method 114 — the corpse could be empty and the test
            // would still pass. With a real LootItem in place, asserting
            // count > 0 below catches a future "empty list short-circuit"
            // regression in handle_interact (e.g., if it grows a guard like
            // `interaction_type = Some(Loot) AND target.loot non-empty`).
            npc.loot.push(cimmeria_entity::cell_entity::LootItem {
                design_id: None, // None = naquadah (cash)
                quantity: 50,
                index: 1,
            });
        }

        let (tx, mut rx) = mpsc::channel(16);
        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        let mut dispatch_args = Vec::with_capacity(4);
        dispatch_args.extend_from_slice(&(npc_id as i32).to_le_bytes());
        let handled = dispatch(1, INTERACT, &dispatch_args, &tx, &mut mgr, &engine).await;
        assert!(handled);

        // Player should now be marked as looting this corpse.
        assert_eq!(
            mgr.get_entity(1).and_then(|p| p.looting_entity),
            Some(npc_id),
            "interact handler must set looting_entity on the player"
        );

        // onLootDisplay (method 114) must have been queued for the player AND
        // the encoded loot count must be non-zero. Wire layout from
        // send_loot_display: `entity_id:i32, count:u32, items[..], initial:u8`.
        let mut saw_loot_display = false;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } = msg
            {
                if entity_id == 1 && method_index == 114 {
                    // We seeded exactly one LootItem; assert the encoded count
                    // matches that, not just `> 0`. A future bug that
                    // duplicated entries or announced the wrong length would
                    // otherwise still pass this regression guard.
                    let count = u32::from_le_bytes(args[4..8].try_into().unwrap());
                    assert_eq!(
                        count, 1,
                        "onLootDisplay must announce exactly the seeded loot count (1)"
                    );
                    // The trailing byte is the `initial` flag from
                    // send_loot_display: 1 for the first display (opens the
                    // loot window) and 0 for refresh-only packets. The
                    // first-open path is the one a right-click on a corpse
                    // takes, so this must be 1.
                    let initial = *args
                        .last()
                        .expect("onLootDisplay payload missing trailing initial byte");
                    assert_eq!(
                        initial, 1,
                        "first-display onLootDisplay must set initial=1 to open the loot window client-side"
                    );
                    saw_loot_display = true;
                }
                // useAbility / target reticle must NOT have been sent.
                assert_ne!(
                    method_index, 16,
                    "dead hostile must not fire onTargetUpdate"
                );
            }
        }
        assert!(
            saw_loot_display,
            "expected onLootDisplay (method 114) for dead lootable corpse"
        );
    }

    /// A neutral NPC (faction != 10) with no dialog/template/tag must NOT be
    /// rerouted to combat. The is_hostile early branch is gated on faction
    /// 10 specifically — if a future regression drops the faction check the
    /// fall-through would silently auto-attack neutral civilians on right-
    /// click.
    #[tokio::test]
    async fn neutral_npc_with_no_interaction_does_not_route_to_combat() {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.player_id = Some(42);
        }
        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc(npc_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(npc_id) {
            // Neutral / non-hostile faction. Faction 10 is the only value
            // that enables the hostile reroute — anything else (including
            // the default 0) must fall through to the dialog/interact path.
            npc.faction = 0;
            npc.clear_all_state_flags();
            // No tag, no template name, no interaction_type configured —
            // i.e. there's nothing for the dialog/interact path to do
            // either. The point of the test is that *nothing combat-related*
            // happens, not that some interaction succeeds.
            npc.tag = None;
            npc.npc_name = None;
            npc.interaction_type = None;
        }

        let (tx, mut rx) = mpsc::channel(16);
        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        let mut args = Vec::with_capacity(4);
        args.extend_from_slice(&(npc_id as i32).to_le_bytes());
        let handled = dispatch(1, INTERACT, &args, &tx, &mut mgr, &engine).await;
        assert!(handled, "INTERACT must always return true (handled)");

        // Method 16 = onTargetUpdate is the canonical hostile-reroute marker
        // emitted at the top of the is_hostile branch (interaction.rs:53-56).
        // Asserting it never appears is a precise probe for "the hostile
        // reroute didn't run" — the alive_hostile_reroutes_to_useability test
        // pins the converse direction, so together they bracket the branch.
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
                assert_ne!(
                    method_index, 16,
                    "neutral NPC must not fire onTargetUpdate from INTERACT",
                );
            }
        }
    }
}
