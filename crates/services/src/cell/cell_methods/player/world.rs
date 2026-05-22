use crate::cell::client_methods::{being, spawnable_entity};
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
        SET_AUTO_CYCLE => {
            if !args.is_empty() {
                let enabled = args[0] != 0;
                tracing::info!(entity_id, enabled, "setAutoCycle");
                if enabled {
                    // Arm the flag AND light `BSF_AUTO_CYCLING` immediately
                    // so the client's gun-icon button highlights on the
                    // very first press. Players expect visual ack on every
                    // click — "armed silently, wait for fire" leaves the
                    // button looking broken until the player happens to
                    // right-click an enemy. The actual loop-arming
                    // (`auto_cycle_ability_id` stash) still happens at
                    // first `useAbility` commit;
                    // the BSF here is purely the "armed" indicator.
                    //
                    // Phase 2 enhancement: if the player has a target
                    // selected AND has previously fired an ability, we
                    // ALSO fire that ability immediately so the button
                    // press feels like an action ("start firing now"),
                    // not just a mode flip. The previous-fire requirement
                    // is the simplest server-side proxy for "what ability
                    // would the player fire?" — looking up via
                    // `items_event_sets` doesn't cover the common Pistol
                    // Shot case (it's an archetype ability, not an
                    // item-event-driven one), so we trade comprehensive
                    // coverage for a heuristic that works as soon as the
                    // player has fired anything in this session.
                    let (new_state, immediate_fire) = {
                        let entity = match space_mgr.get_entity_mut(entity_id) {
                            Some(e) => e,
                            None => return true,
                        };
                        entity.abilities.auto_cycle = true;
                        // Raw bit op — see auto_cycle module doc for why
                        // BSF_AUTO_CYCLING bypasses the ref-counted helpers.
                        let old = entity.state_field;
                        entity.state_field |= crate::cell::combat::BSF_AUTO_CYCLING;
                        let new_state = (entity.state_field != old).then_some(entity.state_field);
                        // Capture (ability, target) for immediate fire
                        // outside the mutable borrow. Both must be Some
                        // to fire; otherwise the loop just arms and
                        // waits for the first manual click.
                        let immediate_fire = entity
                            .abilities
                            .last_fired_ability_id
                            .zip(entity.current_target_id);
                        // Persist the loop's committed ability BEFORE
                        // we call handle_use_ability. If that call
                        // rejects (out of range, on cooldown, no ammo),
                        // its commit-time arm path never runs and the
                        // tick driver would have no ability id to
                        // re-fire with — the loop would be BSF-armed
                        // but functionally dead. Stashing here means
                        // the next cooldown-clear tick can pick up the
                        // loop regardless of whether the immediate
                        // fire succeeded.
                        if let Some((ability_id, _)) = immediate_fire {
                            entity.abilities.auto_cycle_ability_id = Some(ability_id);
                        }
                        (new_state, immediate_fire)
                    };
                    if let Some(new_state) = new_state {
                        super::super::super::abilities::send_entity_method(
                            entity_id,
                            crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
                            new_state.to_le_bytes().to_vec(),
                            tx,
                            space_mgr,
                        )
                        .await;
                    }
                    // Phase 2: immediate fire. Re-enter the standard
                    // ability-fire path; this stashes the loop's
                    // committed ability/target (via the normal commit-
                    // time arm) and emits the usual
                    // onTimerUpdate/onSequence/onEffectResults burst.
                    // Failure modes (out of range, ammo, etc.) leave the
                    // loop armed at BSF level — the next tick re-evaluates,
                    // exactly like a normal cooldown-driven re-fire.
                    //
                    // Gated on `new_state.is_some()` — the BSF must have
                    // ACTUALLY transitioned for this to be a fresh button
                    // press. CEGUI fires the Lua binding 3-4 times per
                    // physical click (observed in playtest as identical
                    // calls within ~150µs); without the transition gate
                    // each duplicate would re-attempt the fire and rely
                    // on the cooldown gate inside `handle_use_ability` to
                    // reject — wasting work and producing noisy logs.
                    if let (Some(_), Some((ability_id, target_id))) = (new_state, immediate_fire) {
                        tracing::info!(
                            entity_id,
                            ability_id,
                            target_id,
                            "setAutoCycle: immediate fire on enable (last_fired + current_target ready)"
                        );
                        let _ = super::super::super::abilities::handle_use_ability(
                            entity_id, ability_id, target_id, tx, space_mgr,
                        )
                        .await;
                    }
                } else {
                    // Explicit disable: drop the stash AND clear the bit so
                    // the client un-highlights the button. clear_auto_cycle
                    // returns Some only on bit transition — re-broadcasting
                    // for an already-off player would be wire noise.
                    if let Some(new_state) =
                        crate::cell::combat::clear_auto_cycle(space_mgr, entity_id)
                    {
                        super::super::super::abilities::send_entity_method(
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
            true
        }

        LOOT_ITEM => {
            if args.len() >= 4 {
                let index = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                crate::cell::interactions::handle_loot_item(entity_id, index, tx, space_mgr).await;
            }
            true
        }

        TRIGGER_REGION => {
            if args.len() >= 17 {
                let region_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let b_entering = args[4] != 0;
                let _x = f32::from_le_bytes([args[5], args[6], args[7], args[8]]);
                let _y = f32::from_le_bytes([args[9], args[10], args[11], args[12]]);
                let _z = f32::from_le_bytes([args[13], args[14], args[15], args[16]]);

                // Region IDs are wire-encoded as i32 but stored as u32 internally.
                // Reject negative values up-front rather than sign-extending them
                // into a high u32 that no real region will match.
                let (region_tag, db_set_id) = match u32::try_from(region_id) {
                    Ok(rid) => match space_mgr.get_region(rid) {
                        Some(r) => (Some(r.tag.clone()), Some(r.db_set_id)),
                        None => (None, None),
                    },
                    Err(_) => {
                        tracing::warn!(
                            entity_id,
                            region_id,
                            "triggerClientHintedGenericRegion: negative region_id, ignoring"
                        );
                        (None, None)
                    }
                };

                if let Some(tag) = region_tag {
                    tracing::info!(entity_id, region_id, %tag, b_entering, "triggerClientHintedGenericRegion");

                    let player_id = space_mgr
                        .get_entity(entity_id)
                        .and_then(|e| e.player_id)
                        .unwrap_or(0);

                    if b_entering {
                        crate::cell::content::fire_enter_region(
                            entity_id, player_id, &tag, engine, tx, space_mgr,
                        )
                        .await;
                    } else {
                        crate::cell::content::fire_exit_region(
                            entity_id, player_id, &tag, engine, tx, space_mgr,
                        )
                        .await;
                    }

                    // Forward to the ring transporter FSM if this region is a
                    // ring pad (point_set_id matches a loaded ring region).
                    if let Some(set_id) = db_set_id {
                        crate::cell::ring_transport::handle_region_trigger(
                            set_id, b_entering, entity_id, tx, space_mgr, engine,
                        )
                        .await;
                    }
                } else {
                    tracing::warn!(
                        entity_id,
                        region_id,
                        "Unknown region ID in triggerClientHintedGenericRegion"
                    );
                }
            }
            true
        }

        REQUEST_RELOAD => {
            if !args.is_empty() {
                let _reload_type = args[0];
                tracing::debug!(entity_id, "requestReload");
                handle_reload(entity_id, tx, space_mgr).await;
            }
            true
        }

        CHOSEN_REWARDS => {
            tracing::info!(entity_id, "UNIMPLEMENTED: chosenRewards");
            true
        }

        SET_RING_TRANSPORTER_DEST => {
            if args.len() >= 8 {
                let region_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let destination_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(
                    entity_id,
                    region_id,
                    destination_id,
                    "setRingTransporterDestination"
                );
                crate::cell::ring_transport::handle_select_destination(
                    region_id,
                    destination_id,
                    entity_id,
                    tx,
                    space_mgr,
                    engine,
                )
                .await;
            }
            true
        }

        WORLD_INSTANCE_RESET => {
            tracing::info!(entity_id, "UNIMPLEMENTED: onWorldInstanceReset");
            true
        }

        UPDATE_SYSTEM_OPTIONS => {
            tracing::info!(entity_id, "UNIMPLEMENTED: updateSystemOptions");
            true
        }

        _ => false,
    }
}

const ABILITY_RELOAD_WEAPON: i32 = 596;

/// How long the draw animation needs to play before the reload sequence
/// can fire. Hand needs to reach the hold position and grip the weapon
/// mesh; firing `Item_Reload` while the hand is mid-air mid-draw plays
/// the reload animation on a model that isn't in the reload-ready pose
/// — the client either ignores the request or visually skips the
/// reload anim (the symptom the user reported in playtest: "weapon
/// teleports into my hand and I still need to hit reload again").
///
/// Empirically tuned to 1 second; matches the rough length of the
/// `KIS-handling` kismet script's draw branch. Bump if the reload still
/// chains into the draw mid-animation; lower if the gap between draw
/// and reload becomes visually obvious.
pub(crate) const UNHOLSTER_DRAW_DURATION: std::time::Duration =
    std::time::Duration::from_millis(1000);

/// Fire a `Item_*` kismet sequence (`Item_Equip` 4000 / `Item_Unequip`
/// 4001 / `Item_Reload` 4002 / `Item_Use` 4003) keyed off the player's
/// archetype-keyed "Item handling" event set. Mirrors
/// `python/cell/SGWBeing.py:getItemSequence(eventId)` + `playSequence`.
///
/// No-op (with a debug log) when the archetype, the event set, or the
/// per-event sequence is missing — matches the python's `if eventSet
/// else None` fallthrough so callers don't crash on edge entities.
pub(crate) async fn fire_item_sequence(
    entity_id: u32,
    event_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let archetype_id = space_mgr.get_entity(entity_id).and_then(|e| e.archetype_id);
    let event_set = archetype_id.and_then(crate::cell::spawner::archetype_item_event_set);
    let seq_id = event_set.and_then(|esid| space_mgr.sequence_map.get(&(esid, event_id)).copied());
    tracing::info!(
        entity_id,
        event_id,
        archetype_id = ?archetype_id,
        event_set_id = ?event_set,
        seq_id = ?seq_id,
        "fire_item_sequence: archetype-keyed sequence lookup"
    );
    let Some(seq_id) = seq_id else {
        return;
    };
    // ON_SEQUENCE wire layout (26 bytes — matches use_ability.rs's fire
    // path so animations are emitted consistently with weapon-fire and
    // reload animations).
    let mut seq_args = Vec::with_capacity(28);
    seq_args.extend_from_slice(&seq_id.to_le_bytes());
    seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes());
    seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes());
    seq_args.push(1);
    seq_args.extend_from_slice(&0.0f32.to_le_bytes());
    seq_args.extend_from_slice(&0u32.to_le_bytes());
    seq_args.push(0);
    seq_args.extend_from_slice(&0i32.to_le_bytes());
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: spawnable_entity::ON_SEQUENCE,
            args: seq_args,
        })
        .await;
}

pub(crate) async fn handle_reload(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let reload_def = space_mgr.ability_defs.get(&ABILITY_RELOAD_WEAPON).cloned();
    let warmup = reload_def.as_ref().map_or(2.0f32, |d| d.warmup);
    let cooldown = reload_def.as_ref().map_or(1.0f32, |d| d.cooldown);

    // Phase A — reload-while-holstered: defer the actual reload until
    // the draw animation has had time to play. Fires `Item_Equip`
    // (event 4000), the bandolier-equip animation, as a stand-in for an
    // explicit draw sequence (the 2009 client never shipped one — the
    // archaeology agent confirmed `Event_NetOut_ChangeWeaponState` is
    // dead scaffolding). Re-running `handle_reload` after
    // `UNHOLSTER_DRAW_DURATION` (via `pending_reload_tick`) lands us in
    // Phase B with the weapon already drawn.
    //
    // Gate: weapon currently holstered + threatened_mobs empty (OOC) +
    // no pending phase already in flight. In-combat reload skips this
    // entirely (weapon's already drawn).
    let needs_phase_a = match space_mgr.get_entity(entity_id) {
        Some(e) => {
            e.weapon_holstered && e.threatened_mobs.is_empty() && e.pending_reload_at.is_none()
        }
        None => false,
    };
    if needs_phase_a {
        // Don't accidentally start a phase A for a player whose mag is
        // already full — the early-return below would catch it after
        // Phase B too, but skipping the wasted draw animation is the
        // right move.
        let ammo_already_full = match space_mgr.get_entity(entity_id) {
            Some(e) => e.active_ammo() >= e.active_clip_size() && e.reload_complete_at.is_none(),
            None => true,
        };
        if !ammo_already_full {
            if let Some(e) = space_mgr.get_entity_mut(entity_id) {
                e.combat_exit_at = Some(std::time::Instant::now());
                e.set_weapon_holstered(false);
                e.pending_reload_at = Some(std::time::Instant::now() + UNHOLSTER_DRAW_DURATION);
                // Cancel any in-flight holster Phase 2 — reload draws
                // the weapon BACK out, so a stale Phase 2 would yank
                // the mesh away mid-reload.
                e.holster_animation_complete_at = None;
            }
            tracing::info!(
                entity_id,
                draw_duration_ms = UNHOLSTER_DRAW_DURATION.as_millis() as u64,
                "reload-while-holstered: phase A — drawing weapon, reload deferred"
            );
            crate::cell::abilities::request_appearance_refresh(entity_id, tx, space_mgr).await;
            fire_item_sequence(
                entity_id,
                crate::cell::spawner::EVENT_ITEM_EQUIP,
                tx,
                space_mgr,
            )
            .await;
            return;
        }
    }

    // Reject second-press during the Phase A draw window. If
    // `pending_reload_at` is set and the timestamp hasn't elapsed yet,
    // the only legitimate entry path is the tick — but the tick fires
    // strictly after the timestamp, so a `now < pending_reload_at`
    // observation here means the player pressed R again mid-draw.
    // Without this gate, the second press falls through to Phase B,
    // clears `pending_reload_at` ahead of schedule, and starts the
    // reload cooldown immediately — defeating the draw window.
    if let Some(t) = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.pending_reload_at)
    {
        if std::time::Instant::now() < t {
            tracing::debug!(
                entity_id,
                "requestReload: ignoring while draw window in progress"
            );
            return;
        }
    }

    // Phase B (or a normal already-drawn reload). When entered from the
    // `pending_reload_tick`, clear the deferred-reload stamp so a
    // racing tick won't re-fire phase B.
    if let Some(e) = space_mgr.get_entity_mut(entity_id) {
        e.pending_reload_at = None;
    }

    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => {
            tracing::warn!(entity_id, "requestReload: entity not found");
            return;
        }
    };

    if entity.active_ammo() >= entity.active_clip_size() && entity.reload_complete_at.is_none() {
        tracing::debug!(entity_id, "requestReload: already at max ammo");
        return;
    }

    let old = entity.active_ammo();
    let target_ammo = entity.active_clip_size();

    let total_time = warmup + cooldown;
    entity.abilities.start_ability_cooldown(
        ABILITY_RELOAD_WEAPON,
        std::time::Duration::from_secs_f32(total_time),
    );

    // Defer the actual ammo refill until after the warmup. The reload-completion
    // tick promotes pending refills; the fire-path gates on `reload_complete_at`
    // to prevent shooting during the warmup.
    //
    // Pin the reload to the slot that started it. If the player swaps weapons
    // mid-reload, the tick must refill *this* slot — not whatever slot is
    // active when the deadline elapses.
    let warmup_duration = std::time::Duration::from_secs_f32(warmup.max(0.0));
    entity.reload_complete_at = Some(std::time::Instant::now() + warmup_duration);
    entity.reload_slot_id = Some(entity.active_bandolier_slot);

    tracing::info!(
        entity_id,
        old,
        target = target_ammo,
        warmup,
        cooldown,
        "Weapon reload started"
    );

    let timer_args = cimmeria_entity::abilities::serialize_timer_update(
        ABILITY_RELOAD_WEAPON,
        cimmeria_entity::abilities::TIMER_ABILITY_COOLDOWN,
        entity_id as i32,
        total_time,
        0.0,
    );
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: being::ON_TIMER_UPDATE,
            args: timer_args,
        })
        .await;

    // BSF_InCombat is intentionally not touched here: a reload in
    // isolation (no aggro) must not flip the in-combat HUD/cursor. The
    // bit is derived from `threatened_mobs` and only flips on via
    // `combat::generate_threat` → `enter_player_combat` when the player
    // actually generates threat on a surviving NPC.

    // Re-stamp `combat_exit_at` so the OOC holster timer fires
    // `OOC_HOLSTER_DELAY` seconds from reload start, never mid-animation.
    // (Phase A already stamped this; Phase B re-stamps so a normal
    // already-drawn reload also resets the OOC countdown.) In-combat
    // reload is untouched — `threatened_mobs.is_empty()` is false, so
    // we skip entirely and `combat_exit_at` stays None until the fight
    // ends naturally.
    if let Some(e) = space_mgr.get_entity_mut(entity_id) {
        if e.threatened_mobs.is_empty() {
            e.combat_exit_at = Some(std::time::Instant::now());
        }
    }

    // Fire the `Item_Reload` (event 4002) animation — the visible
    // drop-mag / insert-mag / chamber sequence. Mirrors
    // `python/cell/SGWBeing.py:863-874`'s `getItemSequence` +
    // `playSequence`. Archetype lookup + sequence dispatch lives in
    // `fire_item_sequence`.
    fire_item_sequence(
        entity_id,
        crate::cell::spawner::EVENT_ITEM_RELOAD,
        tx,
        space_mgr,
    )
    .await;

    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&7i32.to_le_bytes());
    let ammo_type = space_mgr
        .get_entity(entity_id)
        .map_or(0, |e| e.active_ammo_type());
    args.extend_from_slice(&ammo_type.to_le_bytes());
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: spawnable_entity::ON_ENTITY_PROPERTY,
            args,
        })
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::abilities::AbilityDef;
    use cimmeria_entity::cell_entity::BandolierItem;

    fn make_mgr_with_player() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.is_player = true;
            p.player_id = Some(100);
        }
        mgr.connect_entity(1);
        mgr
    }

    #[tokio::test]
    async fn dispatch_returns_false_for_unknown_method() {
        let mut mgr = make_mgr_with_player();
        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(8);
        let handled = dispatch(1, 9999, &[], &tx, &mut mgr, &engine).await;
        assert!(!handled);
    }

    /// SET_AUTO_CYCLE disable clears the full loop stash (flag,
    /// ability id, target id) AND the `BSF_AUTO_CYCLING` state-field
    /// bit when it was previously set. Regression guard: a refactor
    /// that drops the target-id clear or the BSF un-set would leave
    /// the button stuck on-screen highlighted with stale ids ready to
    /// re-fire on the next enable.
    #[tokio::test]
    async fn set_auto_cycle_disable_clears_stash_and_bsf() {
        use crate::cell::combat::BSF_AUTO_CYCLING;
        let mut mgr = make_mgr_with_player();
        // Simulate a previously-armed loop: ability stashed, BSF bit
        // set (the state arrived at by `arm_auto_cycle`).
        if let Some(e) = mgr.get_entity_mut(1) {
            e.abilities.auto_cycle = true;
            e.abilities.auto_cycle_ability_id = Some(597);
            e.set_state_flag(BSF_AUTO_CYCLING);
        }
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(8);

        // args = [0] → enabled = false
        let handled = dispatch(1, SET_AUTO_CYCLE, &[0], &tx, &mut mgr, &engine).await;
        assert!(handled);

        let e = mgr.get_entity(1).unwrap();
        assert!(!e.abilities.auto_cycle);
        assert!(
            e.abilities.auto_cycle_ability_id.is_none(),
            "disable must clear auto_cycle_ability_id"
        );
        assert_eq!(
            e.state_field & BSF_AUTO_CYCLING,
            0,
            "disable must clear BSF_AUTO_CYCLING so the client un-highlights the button"
        );

        // Verify the broadcast went out — the client requires this
        // `onStateFieldUpdate` to fire `EmitAutoCycleStateChanged`.
        let mut saw_state_field_update = false;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall {
                entity_id: 1,
                method_index,
                ..
            } = msg
            {
                if method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE {
                    saw_state_field_update = true;
                }
            }
        }
        assert!(
            saw_state_field_update,
            "disable with BSF set must broadcast onStateFieldUpdate so the client un-highlights the button"
        );
    }

    /// SET_AUTO_CYCLE disable when BSF was already clear must NOT
    /// emit a redundant `onStateFieldUpdate`. The transition gate
    /// inside `clear_auto_cycle` returns `None` and the handler
    /// short-circuits the send. Pin so a refactor that always
    /// broadcasts doesn't add wire noise on every disable.
    #[tokio::test]
    async fn set_auto_cycle_disable_when_bsf_clear_emits_no_broadcast() {
        let mut mgr = make_mgr_with_player();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.abilities.auto_cycle = true; // armed flag only
                                           // No BSF bit set — the loop never reached commit.
        }
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(8);

        let handled = dispatch(1, SET_AUTO_CYCLE, &[0], &tx, &mut mgr, &engine).await;
        assert!(handled);

        assert!(
            rx.try_recv().is_err(),
            "disable without prior BSF set must not broadcast"
        );
    }

    /// SET_AUTO_CYCLE enable sets the flag AND lights
    /// `BSF_AUTO_CYCLING` immediately so the client's gun-icon
    /// button highlights on the very first press. The
    /// ability/target stash stays empty — that's still set at
    /// first `useAbility` commit when the ids are actually known.
    ///
    /// Bug shape this prevents (the symptom that drove the change):
    /// players pressed the button, got no visual feedback, assumed
    /// the button was broken, and pressed it 5-10 more times.
    /// "Light on enable" closes that UX gap.
    #[tokio::test]
    async fn set_auto_cycle_enable_lights_bsf_and_broadcasts() {
        use crate::cell::combat::BSF_AUTO_CYCLING;
        let mut mgr = make_mgr_with_player();
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(8);

        let handled = dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;
        assert!(handled);

        let e = mgr.get_entity(1).unwrap();
        assert!(e.abilities.auto_cycle, "flag must be armed");
        assert!(
            e.abilities.auto_cycle_ability_id.is_none(),
            "ability id stash still empty — that arms at first commit",
        );
        assert_ne!(
            e.state_field & BSF_AUTO_CYCLING,
            0,
            "enable MUST light BSF_AUTO_CYCLING so the button highlights",
        );

        // The broadcast must hit the wire so the client's
        // `EmitAutoCycleStateChanged` fires and the button lights.
        let mut saw_state_field_update = false;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall {
                entity_id: 1,
                method_index,
                ..
            } = msg
            {
                if method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE {
                    saw_state_field_update = true;
                }
            }
        }
        assert!(
            saw_state_field_update,
            "enable must broadcast onStateFieldUpdate so the client lights the button"
        );
    }

    /// Disabling repeatedly when the bit is already clear is a no-op
    /// (no re-broadcast). Mirror of the enable-spam test. The CEGUI
    /// duplicate-click pattern affects disable presses too — without
    /// the transition gate inside `clear_auto_cycle`, each redundant
    /// disable would emit an `onStateFieldUpdate` carrying the same
    /// (already-cleared) `bStateField` and spam the wire.
    #[tokio::test]
    async fn set_auto_cycle_disable_spam_does_not_re_broadcast() {
        let mut mgr = make_mgr_with_player();
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(16);

        // Pre-state: armed (flag + BSF set, as if enable ran earlier).
        dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;
        // Drain the enable broadcast.
        while rx.try_recv().is_ok() {}

        // First disable: transitions the bit, broadcasts.
        dispatch(1, SET_AUTO_CYCLE, &[0], &tx, &mut mgr, &engine).await;
        let mut first_broadcasts = 0;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
                if method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE {
                    first_broadcasts += 1;
                }
            }
        }
        assert_eq!(first_broadcasts, 1, "first disable broadcasts exactly once");

        // Subsequent duplicate disables: must not broadcast.
        for _ in 0..5 {
            dispatch(1, SET_AUTO_CYCLE, &[0], &tx, &mut mgr, &engine).await;
        }
        assert!(
            rx.try_recv().is_err(),
            "duplicate disable calls must NOT re-broadcast — bit is already clear",
        );
    }

    /// Phase 2 immediate-fire: when the player presses the auto-cycle
    /// button AND they already have a target selected AND they've
    /// previously fired an ability in this session, the button press
    /// fires that ability at the target immediately. Closes the
    /// "press button → nothing visible" gap that drove the Phase 2
    /// work.
    ///
    /// Pre-conditions encoded: `current_target_id` (from setTargetID)
    /// + `last_fired_ability_id` (from a prior commit) — both must
    /// be Some. Either being None falls through to "just light BSF,
    /// wait for first manual fire" (existing Phase 1 behavior, which
    /// the other tests in this module cover).
    #[tokio::test]
    async fn set_auto_cycle_enable_fires_immediately_when_target_and_last_ability_set() {
        use cimmeria_entity::abilities::AbilityDef;
        let mut mgr = make_mgr_with_player();
        // Seed an NPC target close enough to be in range.
        mgr.spawn_npc(50, "Castle_CellBlock", [3.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            // Player has fired ability 7 earlier this session (stashed
            // by handle_use_ability commit).
            p.abilities.add_ability(7);
            p.abilities.last_fired_ability_id = Some(7);
            // Player has a target selected (setTargetID wrote this).
            p.current_target_id = Some(50);
            // Weapon drawn so the unholster queue doesn't intercept.
            p.weapon_holstered = false;
        }
        mgr.ability_defs.insert(
            7,
            AbilityDef {
                ability_id: 7,
                name: "test".to_string(),
                cooldown: 0.5,
                warmup: 0.0,
                flags: 0,
                is_ranged: false,
                min_range: 0,
                max_range: 30,
                target_type_id: 0,
                effect_ids: vec![],
                moniker_ids: vec![],
                required_ammo: 0,
                event_set_id: None,
                velocity: 0.0,
            },
        );
        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(64);

        let handled = dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;
        assert!(handled);

        // Immediate-fire signature: the ability's cooldown is now
        // running (handle_use_ability called start_ability_cooldown)
        // and the auto-cycle loop is committed (auto_cycle_ability_id
        // stashed from the arm step inside the fire).
        let p = mgr.get_entity(1).unwrap();
        assert!(
            p.abilities.is_on_cooldown(7),
            "immediate fire must have started the cooldown — proves handle_use_ability ran",
        );
        assert_eq!(
            p.abilities.auto_cycle_ability_id,
            Some(7),
            "the loop must be armed (committed ability) after the immediate fire",
        );
    }

    /// Regression: if `handle_use_ability` REJECTS the immediate fire
    /// (out of range, on cooldown, no ammo), the loop must still be
    /// armed at the ability-id level so the next tick can pick it up.
    /// Without this guard the BSF lights but `auto_cycle_ability_id`
    /// stays None → the driver tick has nothing to re-fire → loop
    /// silently dead until the player toggles off and back on.
    ///
    /// Fixture: target is on the OPPOSITE side of the map (out of
    /// range) so the fire fails validation but doesn't commit the
    /// cooldown.
    #[tokio::test]
    async fn set_auto_cycle_enable_persists_ability_even_when_immediate_fire_rejects() {
        use cimmeria_entity::abilities::AbilityDef;
        let mut mgr = make_mgr_with_player();
        // Target 200 units away — far beyond the 30-unit max_range.
        mgr.spawn_npc(50, "Castle_CellBlock", [200.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.abilities.add_ability(7);
            p.abilities.last_fired_ability_id = Some(7);
            p.current_target_id = Some(50);
            p.weapon_holstered = false;
        }
        mgr.ability_defs.insert(
            7,
            AbilityDef {
                ability_id: 7,
                name: "test".to_string(),
                cooldown: 0.5,
                warmup: 0.0,
                flags: 0,
                is_ranged: false,
                min_range: 0,
                max_range: 30, // target is at 200 → out of range
                target_type_id: 0,
                effect_ids: vec![],
                moniker_ids: vec![],
                required_ammo: 0,
                event_set_id: None,
                velocity: 0.0,
            },
        );
        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(64);

        dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;

        let p = mgr.get_entity(1).unwrap();
        assert!(p.abilities.auto_cycle, "flag must arm even if fire rejects");
        // The decisive assertion: ability id MUST be stashed BEFORE the
        // (rejected) fire so the tick has something to re-fire later
        // when the player moves into range.
        assert_eq!(
            p.abilities.auto_cycle_ability_id,
            Some(7),
            "auto_cycle_ability_id MUST persist even when immediate fire is rejected — \
             otherwise the loop is silently dead until toggle off+on",
        );
        assert!(
            !p.abilities.is_on_cooldown(7),
            "out-of-range fire was rejected, so cooldown is NOT running",
        );
    }

    /// Phase 2: if the player has never fired an ability this session
    /// (`last_fired_ability_id == None`), pressing the button just
    /// lights BSF — no immediate fire. The tick will pick up the loop
    /// after the player's first manual right-click fire. Mirrors
    /// Phase 1 behavior; pin so the immediate-fire path doesn't
    /// accidentally start firing at session-start before the player
    /// has had a chance to choose an ability.
    #[tokio::test]
    async fn set_auto_cycle_enable_does_not_fire_without_last_ability() {
        let mut mgr = make_mgr_with_player();
        if let Some(p) = mgr.get_entity_mut(1) {
            // Target selected but never fired anything yet.
            p.current_target_id = Some(50);
            // last_fired_ability_id stays None.
            p.weapon_holstered = false;
        }
        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(64);

        dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;

        let p = mgr.get_entity(1).unwrap();
        assert!(p.abilities.auto_cycle, "flag must still arm");
        assert!(
            p.abilities.auto_cycle_ability_id.is_none(),
            "no immediate fire happened → loop ability stash stays empty",
        );
    }

    /// Phase 2: if the player has fired earlier but currently has no
    /// target selected (`current_target_id == None`), pressing the
    /// button just lights BSF — no immediate fire. The tick will
    /// pick up the loop once the player selects a target via cursor.
    #[tokio::test]
    async fn set_auto_cycle_enable_does_not_fire_without_target() {
        let mut mgr = make_mgr_with_player();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.abilities.last_fired_ability_id = Some(7);
            // current_target_id stays None — no target selected.
            p.weapon_holstered = false;
        }
        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(64);

        dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;

        let p = mgr.get_entity(1).unwrap();
        assert!(p.abilities.auto_cycle, "flag must still arm");
        assert!(!p.abilities.is_on_cooldown(7), "no immediate fire happened");
    }

    /// Spamming `setAutoCycle(1)` repeatedly (the CEGUI button fires
    /// the Lua function 3-4 times per physical click, all within
    /// ~150µs) must NOT re-broadcast. The bit is already set after
    /// the first call; subsequent calls are idempotent. Pin: a
    /// regression where the raw bit-set check disappears would
    /// re-broadcast on every duplicate call and spam the wire.
    #[tokio::test]
    async fn set_auto_cycle_enable_spam_does_not_re_broadcast() {
        let mut mgr = make_mgr_with_player();
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(16);

        // First call: should broadcast.
        dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;
        let mut first_broadcasts = 0;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
                if method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE {
                    first_broadcasts += 1;
                }
            }
        }
        assert_eq!(first_broadcasts, 1, "first enable broadcasts exactly once");

        // Subsequent duplicate calls: must not broadcast.
        for _ in 0..5 {
            dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;
        }
        assert!(
            rx.try_recv().is_err(),
            "duplicate enable calls must NOT re-broadcast — bit is already set",
        );
    }

    /// TRIGGER_REGION with a negative region_id must be rejected by
    /// the explicit `u32::try_from` guard, NOT by accidentally
    /// missing a sign-extended u32 lookup. Pre-seed a real region at
    /// the sign-extended id (`-5i32 as u32 == 0xFFFFFFFB`); if the
    /// regression resurfaces (the cast slips through), the lookup
    /// would match the planted region and fire content events.
    /// With the negative-id guard in place the planted region must
    /// stay invisible.
    #[tokio::test]
    async fn trigger_region_with_negative_id_rejects_via_explicit_guard() {
        use crate::cell::space_manager::RegionData;
        let mut mgr = make_mgr_with_player();
        // Plant a region at the sign-extended id of -5. If a regression
        // reintroduces the `region_id as u32` cast, get_region(0xFFFFFFFB)
        // would match this row and fire ring_transport / fire_enter_region.
        let trap_id: u32 = (-5i32) as u32;
        mgr.regions.insert(
            trap_id,
            RegionData {
                runtime_id: trap_id,
                db_set_id: 9999,
                tag: "trap".to_string(),
                world_name: "Castle_CellBlock".to_string(),
                height: 0.0,
                radius: 0.0,
                flags: 0,
                points: vec![],
            },
        );

        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(8);

        // Layout: i32 region_id + u8 b_entering + 3 × f32 position.
        let mut args = Vec::with_capacity(17);
        args.extend_from_slice(&(-5i32).to_le_bytes());
        args.push(1);
        args.extend_from_slice(&0.0f32.to_le_bytes());
        args.extend_from_slice(&0.0f32.to_le_bytes());
        args.extend_from_slice(&0.0f32.to_le_bytes());

        let handled = dispatch(1, TRIGGER_REGION, &args, &tx, &mut mgr, &engine).await;
        assert!(
            handled,
            "TRIGGER_REGION must claim the method even when region_id is bogus"
        );
        // The planted trap region MUST NOT match. No fire_*_region
        // cascade, no ring_transport message.
        assert!(
            rx.try_recv().is_err(),
            "negative region_id must be rejected by u32::try_from before lookup, \
             so the trap region at 0xFFFFFFFB can't fire"
        );
    }

    /// `handle_reload` is a no-op when the active slot is at full
    /// clip and no reload is in flight. Pin so a refactor that
    /// always starts a reload (and therefore wastes ammo on every
    /// keypress) gets caught.
    #[tokio::test]
    async fn handle_reload_no_op_when_already_full() {
        let mut mgr = make_mgr_with_player();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.bandolier_items.insert(
                0,
                BandolierItem {
                    item_id: 1,
                    clip_size: 30,
                    default_ammo_type: 2,
                    current_ammo: 30, // full
                    cur_ammo_type: 2,
                },
            );
            e.active_bandolier_slot = 0;
        }
        let (tx, mut rx) = mpsc::channel(8);
        handle_reload(1, &tx, &mut mgr).await;

        let e = mgr.get_entity(1).unwrap();
        assert!(
            e.reload_complete_at.is_none(),
            "no reload should be queued when full"
        );
        assert!(rx.try_recv().is_err(), "no packets should be emitted");
    }

    /// `handle_reload` from an empty magazine pins the slot id at
    /// the time of issue. If the player swaps mid-reload, the
    /// completion tick must refill THIS slot, not whatever slot is
    /// active when the deadline elapses.
    #[tokio::test]
    async fn handle_reload_pins_reload_slot_id_to_current_active_slot() {
        let mut mgr = make_mgr_with_player();
        if let Some(e) = mgr.get_entity_mut(1) {
            // Weapon already drawn so Phase A (defer-reload-for-draw)
            // doesn't kick in — this test asserts the Phase B slot
            // pin, not the Phase A defer path.
            e.weapon_holstered = false;
            e.bandolier_items.insert(
                2,
                BandolierItem {
                    item_id: 1,
                    clip_size: 30,
                    default_ammo_type: 2,
                    current_ammo: 0,
                    cur_ammo_type: 2,
                },
            );
            e.active_bandolier_slot = 2;
        }
        // Seed the reload AbilityDef so warmup/cooldown/event_set are read.
        mgr.ability_defs.insert(
            596,
            AbilityDef {
                ability_id: 596,
                name: "reload".to_string(),
                cooldown: 1.0,
                warmup: 0.5,
                flags: 0,
                is_ranged: false,
                min_range: 0,
                max_range: 0,
                target_type_id: 0,
                effect_ids: vec![],
                moniker_ids: vec![],
                required_ammo: 0,
                event_set_id: None,
                velocity: 0.0,
            },
        );
        let (tx, _rx) = mpsc::channel(16);
        handle_reload(1, &tx, &mut mgr).await;

        let e = mgr.get_entity(1).unwrap();
        assert!(
            e.reload_complete_at.is_some(),
            "reload must arm the deadline"
        );
        assert_eq!(
            e.reload_slot_id,
            Some(2),
            "reload_slot_id must capture the active slot at issue time, not be re-read at completion"
        );
    }

    /// Reload-start must fire the `Item_Reload` (event 4002) sequence from
    /// the player's archetype-keyed "Item handling" event set so the
    /// client plays the visible reload animation. Mirrors
    /// `python/cell/SGWBeing.py:863-874` (`getItemSequence(Item_Reload)` +
    /// `playSequence`). Previously this site looked up the *reload
    /// ability's* `event_set_id`, which is NULL in the seed — the lookup
    /// short-circuited and no animation ever played in production.
    ///
    /// Bug shape this catches: a refactor that goes back to keying off
    /// `ability_defs[596].event_set_id` reintroduces the dead path.
    #[tokio::test]
    async fn handle_reload_sends_item_reload_sequence() {
        use crate::cell::client_methods::spawnable_entity::ON_SEQUENCE;
        use crate::cell::spawner::{EVENT_ABILITY_BEGIN, EVENT_ITEM_RELOAD};

        let mut mgr = make_mgr_with_player();
        if let Some(e) = mgr.get_entity_mut(1) {
            // Soldier archetype → event set 804 (the human "Item handling"
            // set per `archetype_item_event_set`).
            e.archetype_id = Some(1);
            // Weapon already drawn so Phase A (defer-reload-for-draw)
            // doesn't kick in — this test asserts the Phase B byte
            // layout of Item_Reload's ON_SEQUENCE.
            e.weapon_holstered = false;
            e.bandolier_items.insert(
                0,
                BandolierItem {
                    item_id: 1,
                    clip_size: 30,
                    default_ammo_type: 2,
                    current_ammo: 0,
                    cur_ammo_type: 2,
                },
            );
            e.active_bandolier_slot = 0;
        }
        // Seed the sequence_map so the (804, Item_Reload) lookup finds a
        // sentinel sequence id we can recognise on the wire.
        const HUMAN_ITEM_EVENT_SET: i32 = 804;
        const ITEM_RELOAD_SEQ_ID: i32 = 1874;
        mgr.sequence_map.insert(
            (HUMAN_ITEM_EVENT_SET, EVENT_ITEM_RELOAD),
            ITEM_RELOAD_SEQ_ID,
        );
        // Seed a decoy (ability's event set → Ability_Begin) — the
        // regression we're guarding against would have sent THIS one.
        const DECOY_EVENT_SET: i32 = 7777;
        const DECOY_BEGIN_SEQ_ID: i32 = 9001;
        mgr.ability_defs.insert(
            596,
            AbilityDef {
                ability_id: 596,
                name: "reload".to_string(),
                cooldown: 1.0,
                warmup: 0.5,
                flags: 0,
                is_ranged: false,
                min_range: 0,
                max_range: 0,
                target_type_id: 0,
                effect_ids: vec![],
                moniker_ids: vec![],
                required_ammo: 0,
                event_set_id: Some(DECOY_EVENT_SET),
                velocity: 0.0,
            },
        );
        mgr.sequence_map
            .insert((DECOY_EVENT_SET, EVENT_ABILITY_BEGIN), DECOY_BEGIN_SEQ_ID);

        let (tx, mut rx) = mpsc::channel(64);
        handle_reload(1, &tx, &mut mgr).await;

        // ON_SEQUENCE wire layout (26 bytes — matches use_ability.rs's fire path):
        //   sequence_id   i32 LE  @ 0..4
        //   source_id     i32 LE  @ 4..8
        //   target_id     i32 LE  @ 8..12
        //   primary       u8      @ 12
        //   impact_time   f32 LE  @ 13..17
        //   nvp_count     u32 LE  @ 17..21
        //   view_type     u8      @ 21
        //   instance_id   i32 LE  @ 22..26
        let mut item_reload_count = 0;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } = msg
            {
                if method_index == ON_SEQUENCE {
                    assert_eq!(
                        args.len(),
                        26,
                        "ON_SEQUENCE payload must be exactly 26 bytes — any drift \
                         in the serializer would silently corrupt the kismet event \
                         frame on the wire"
                    );
                    let seq_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                    assert_ne!(
                        seq_id, DECOY_BEGIN_SEQ_ID,
                        "reload-start must NOT fire the reload ability's Ability_Begin \
                         (that's the dead pre-fix path)",
                    );
                    if seq_id != ITEM_RELOAD_SEQ_ID {
                        continue;
                    }
                    let source_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                    let target_id = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                    let primary = args[12];
                    let impact_time = f32::from_le_bytes([args[13], args[14], args[15], args[16]]);
                    let nvp_count = u32::from_le_bytes([args[17], args[18], args[19], args[20]]);
                    let view_type = args[21];
                    let instance_id = i32::from_le_bytes([args[22], args[23], args[24], args[25]]);
                    assert_eq!(
                        source_id, 1,
                        "source = entity_id (player firing the reload)"
                    );
                    assert_eq!(target_id, 1, "reload targets self");
                    assert_eq!(primary, 1, "primary target flag set");
                    assert_eq!(impact_time, 0.0, "no projectile impact time for reload");
                    assert_eq!(nvp_count, 0, "no name-value pairs in payload");
                    assert_eq!(
                        view_type, 0,
                        "ViewType=0 (KISMET_VIEW_Witness) — matches use_ability.rs's \
                         fire path so reload-begin animates consistently with weapon \
                         fire animations"
                    );
                    assert_eq!(instance_id, 0, "no effect instance for the reload sequence");
                    item_reload_count += 1;
                }
            }
        }
        assert_eq!(
            item_reload_count, 1,
            "reload-start must send exactly one onSequence with the Item_Reload \
             sequence id; without it the client plays no visible reload animation. \
             Got {item_reload_count}.",
        );
    }

    /// Reload-while-holstered Phase A: a player who's OOC and
    /// holstered presses reload. The handler defers the actual reload
    /// to give the draw animation time to play. Phase A must:
    ///   1. Flip `weapon_holstered` to false.
    ///   2. Stamp `combat_exit_at` so the OOC re-holster timer fires
    ///      AFTER the eventual Phase B reload completes.
    ///   3. Set `pending_reload_at = now + UNHOLSTER_DRAW_DURATION` so
    ///      `pending_reload_tick` can promote Phase A → Phase B.
    ///   4. Dispatch `RefreshAppearance` (mesh attaches at hand socket).
    ///   5. NOT start the reload-completion timer or fire `Item_Reload`
    ///      yet — those land in Phase B.
    ///
    /// Bug shape this catches (the playtest report that drove the fix):
    /// firing `Item_Reload` and the appearance change in the same tick
    /// makes the weapon "teleport into the hand + reload anim plays on
    /// empty space", and the player has to press reload twice.
    #[tokio::test]
    async fn reload_while_holstered_phase_a_defers_reload() {
        let mut mgr = make_mgr_with_player();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.archetype_id = Some(1);
            e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
            e.weapon_holstered = true; // OOC + holstered
            e.combat_exit_at = None;
            e.pending_reload_at = None;
            e.bandolier_items.insert(
                0,
                BandolierItem {
                    item_id: 1,
                    clip_size: 30,
                    default_ammo_type: 2,
                    current_ammo: 0,
                    cur_ammo_type: 2,
                },
            );
            e.active_bandolier_slot = 0;
        }

        let (tx, mut rx) = mpsc::channel(64);
        handle_reload(1, &tx, &mut mgr).await;

        let e = mgr.get_entity(1).unwrap();
        assert!(!e.weapon_holstered, "Phase A must draw the weapon");
        assert!(
            e.combat_exit_at.is_some(),
            "Phase A must stamp combat_exit_at so OOC re-holster fires AFTER \
             the eventual reload completes",
        );
        assert!(
            e.pending_reload_at.is_some(),
            "Phase A must set pending_reload_at so the deferred-reload tick \
             can promote to Phase B once the draw window elapses",
        );
        assert!(
            e.reload_complete_at.is_none(),
            "Phase A must NOT start the reload-completion timer — the actual \
             reload hasn't started yet, only the draw. Firing the reload here \
             is the bug shape we're explicitly avoiding (user playtest: \
             'weapon teleports into my hand and I still need to hit reload again')",
        );

        let mut saw_refresh = false;
        while let Ok(msg) = rx.try_recv() {
            if matches!(
                msg,
                CellToBaseMsg::RefreshAppearance {
                    holstered: false,
                    ..
                }
            ) {
                saw_refresh = true;
                break;
            }
        }
        assert!(
            saw_refresh,
            "Phase A must dispatch RefreshAppearance(holstered=false) so the \
             client attaches the weapon mesh at the hand socket before the \
             draw animation triggers",
        );
    }

    /// Phase A → Phase B promotion: once the draw window has
    /// elapsed, calling `handle_reload` again (as the
    /// `pending_reload_tick` does) finds `pending_reload_at` set,
    /// clears it, and runs the normal Phase B reload start
    /// (`reload_complete_at` armed, `Item_Reload` sequence fired).
    ///
    /// Bug shape this catches: a refactor that forgets to clear
    /// `pending_reload_at` in Phase B leaves the tick re-firing
    /// `handle_reload` every 100ms forever.
    #[tokio::test]
    async fn reload_phase_a_to_phase_b_clears_pending_and_starts_reload() {
        let mut mgr = make_mgr_with_player();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.archetype_id = Some(1);
            e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
            // Already drawn by Phase A; `pending_reload_at` is what the
            // promotion key reads.
            e.weapon_holstered = false;
            e.combat_exit_at = Some(std::time::Instant::now());
            e.pending_reload_at = Some(std::time::Instant::now());
            e.bandolier_items.insert(
                0,
                BandolierItem {
                    item_id: 1,
                    clip_size: 30,
                    default_ammo_type: 2,
                    current_ammo: 0,
                    cur_ammo_type: 2,
                },
            );
            e.active_bandolier_slot = 0;
        }
        mgr.ability_defs.insert(
            596,
            AbilityDef {
                ability_id: 596,
                name: "reload".to_string(),
                cooldown: 1.0,
                warmup: 0.5,
                flags: 0,
                is_ranged: false,
                min_range: 0,
                max_range: 0,
                target_type_id: 0,
                effect_ids: vec![],
                moniker_ids: vec![],
                required_ammo: 0,
                event_set_id: None,
                velocity: 0.0,
            },
        );

        let (tx, _rx) = mpsc::channel(64);
        handle_reload(1, &tx, &mut mgr).await;

        let e = mgr.get_entity(1).unwrap();
        assert!(
            e.pending_reload_at.is_none(),
            "Phase B must clear pending_reload_at so the tick doesn't re-fire \
             handle_reload every 100ms forever",
        );
        assert!(
            e.reload_complete_at.is_some(),
            "Phase B must start the reload (set reload_complete_at) so the \
             completion tick can promote the ammo refill",
        );
    }

    /// Reload-while-in-OOC-grace (weapon already drawn): the timer
    /// must be RE-STAMPED so it doesn't fire `OOC_HOLSTER_DELAY`
    /// seconds after combat ended — which could land mid-reload and
    /// holster the weapon while the animation is still playing.
    #[tokio::test]
    async fn reload_during_ooc_grace_resets_holster_timer() {
        let mut mgr = make_mgr_with_player();
        let stale_stamp = std::time::Instant::now() - std::time::Duration::from_secs(8);
        if let Some(e) = mgr.get_entity_mut(1) {
            e.archetype_id = Some(1);
            e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
            e.weapon_holstered = false; // OOC but still drawn
            e.combat_exit_at = Some(stale_stamp);
            e.bandolier_items.insert(
                0,
                BandolierItem {
                    item_id: 1,
                    clip_size: 30,
                    default_ammo_type: 2,
                    current_ammo: 0,
                    cur_ammo_type: 2,
                },
            );
            e.active_bandolier_slot = 0;
        }

        let (tx, _rx) = mpsc::channel(64);
        handle_reload(1, &tx, &mut mgr).await;

        let e = mgr.get_entity(1).unwrap();
        assert!(!e.weapon_holstered, "already-drawn weapon stays drawn");
        let new_stamp = e.combat_exit_at.expect("timer must remain armed");
        assert!(
            new_stamp > stale_stamp,
            "timer must be re-stamped to current time so the existing \
             OOC_HOLSTER_DELAY countdown doesn't expire mid-reload",
        );
    }

    /// Second reload press during the Phase A draw window must be
    /// silently ignored. Without this gate, the second press falls
    /// through to Phase B, clears `pending_reload_at` early, and
    /// starts the reload cooldown immediately — defeating the draw
    /// animation timing.
    ///
    /// Bug shape: refactor drops the `now < pending_reload_at` check
    /// at the top of Phase B; a player mashing R during the draw
    /// window triggers Phase B prematurely and the reload anim
    /// chains in mid-draw (the symptom that drove the original
    /// two-phase split).
    #[tokio::test]
    async fn reload_second_press_during_draw_window_is_ignored() {
        let mut mgr = make_mgr_with_player();
        let future = std::time::Instant::now() + std::time::Duration::from_millis(800);
        if let Some(e) = mgr.get_entity_mut(1) {
            e.archetype_id = Some(1);
            e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
            e.weapon_holstered = false; // weapon drawn (Phase A finished its draw)
            e.combat_exit_at = Some(std::time::Instant::now());
            e.bandolier_items.insert(
                0,
                BandolierItem {
                    item_id: 1,
                    clip_size: 30,
                    default_ammo_type: 2,
                    current_ammo: 0,
                    cur_ammo_type: 2,
                },
            );
            e.active_bandolier_slot = 0;
            // Phase A already fired — Phase B is queued for the future.
            e.pending_reload_at = Some(future);
        }
        // No reload ability def needed — the gate fires before any
        // ability lookup.

        let (tx, _rx) = mpsc::channel(64);
        handle_reload(1, &tx, &mut mgr).await;

        let e = mgr.get_entity(1).unwrap();
        assert_eq!(
            e.pending_reload_at,
            Some(future),
            "second press must NOT clear pending_reload_at — the \
             tick still owns the Phase B promotion at the right time",
        );
        assert!(
            e.reload_complete_at.is_none(),
            "second press must NOT start the reload cooldown — Phase B \
             would otherwise fire mid-draw and chain the reload \
             animation before the unholster motion finishes",
        );
    }

    /// Reload-in-isolation regression: reloading without any aggro must
    /// NOT flip BSF_InCombat on the player. The previous bug: the
    /// reload handler set the bit raw, but reload doesn't generate
    /// threat on anything — so no NPC death would ever clear the bit,
    /// stranding the player in the in-combat HUD/cursor forever (and
    /// blocking the out-of-combat regen tick, which gates on
    /// `threatened_mobs.is_empty()`).
    ///
    #[tokio::test]
    async fn reload_in_isolation_does_not_flip_bsf_in_combat() {
        use crate::cell::combat::BSF_IN_COMBAT;

        let mut mgr = make_mgr_with_player();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.bandolier_items.insert(
                0,
                BandolierItem {
                    item_id: 1,
                    clip_size: 30,
                    default_ammo_type: 2,
                    current_ammo: 0,
                    cur_ammo_type: 2,
                },
            );
            e.active_bandolier_slot = 0;
        }
        // Seed the reload AbilityDef so the warmup path runs.
        mgr.ability_defs.insert(
            596,
            AbilityDef {
                ability_id: 596,
                name: "reload".to_string(),
                cooldown: 1.0,
                warmup: 0.5,
                flags: 0,
                is_ranged: false,
                min_range: 0,
                max_range: 0,
                target_type_id: 0,
                effect_ids: vec![],
                moniker_ids: vec![],
                required_ammo: 0,
                event_set_id: None,
                velocity: 0.0,
            },
        );

        let (tx, _rx) = mpsc::channel(64);
        handle_reload(1, &tx, &mut mgr).await;

        let s = mgr.get_entity(1).unwrap().state_field;
        assert_eq!(
            s & BSF_IN_COMBAT,
            0,
            "reload MUST NOT flip BSF_InCombat — reload-without-aggro had no \
             NPC-death clear path and the bit would strand forever"
        );
        assert!(
            mgr.get_entity(1).unwrap().threatened_mobs.is_empty(),
            "reload must leave threatened_mobs empty — the source of truth \
             for the in-combat state"
        );
    }
}
