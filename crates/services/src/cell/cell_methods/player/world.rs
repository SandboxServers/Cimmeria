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
                tracing::debug!(entity_id, enabled, "setAutoCycle");
                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    entity.abilities.auto_cycle = enabled;
                    if !enabled {
                        entity.abilities.auto_cycle_ability_id = None;
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

    /// SET_AUTO_CYCLE flips entity.abilities.auto_cycle. When
    /// disabled, must clear auto_cycle_ability_id too — otherwise a
    /// stale ability id would re-trigger on the next enable cycle.
    #[tokio::test]
    async fn set_auto_cycle_disable_clears_ability_id() {
        let mut mgr = make_mgr_with_player();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.abilities.auto_cycle = true;
            e.abilities.auto_cycle_ability_id = Some(597);
        }
        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(8);

        // args = [0] → enabled = false
        let handled = dispatch(1, SET_AUTO_CYCLE, &[0], &tx, &mut mgr, &engine).await;
        assert!(handled);

        let e = mgr.get_entity(1).unwrap();
        assert!(!e.abilities.auto_cycle);
        assert!(
            e.abilities.auto_cycle_ability_id.is_none(),
            "disable must also clear auto_cycle_ability_id"
        );
    }

    /// SET_AUTO_CYCLE enable doesn't touch auto_cycle_ability_id —
    /// that's set elsewhere. Pin so a refactor that conflates the
    /// two doesn't leak.
    #[tokio::test]
    async fn set_auto_cycle_enable_only_sets_flag() {
        let mut mgr = make_mgr_with_player();
        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(8);

        let handled = dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;
        assert!(handled);
        let e = mgr.get_entity(1).unwrap();
        assert!(e.abilities.auto_cycle);
        // auto_cycle_ability_id stays None (was never set)
        assert!(e.abilities.auto_cycle_ability_id.is_none());
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

    /// Reload-while-holstered Phase A (PR #338): a player who's OOC and
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

    /// Phase A → Phase B promotion (PR #338): once the draw window has
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
