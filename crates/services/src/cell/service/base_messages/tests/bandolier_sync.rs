use super::*;

/// `SyncBandolierItems` is the message base dispatches after a
/// player-driven `moveInventoryItem` lands a weapon in a bandolier
/// slot (the drag-from-backpack-to-bandolier flow — distinct from
/// the chain-engine `grantItem` path that goes through
/// `UpdateBandolierItem`). When the active slot just gained a
/// weapon, the cell must fire the equip-display chain: draw the
/// weapon, dispatch `RefreshAppearance(holstered=false)` for the
/// mesh attach, fire `Item_Equip` for the unholster animation, and
/// arm the OOC re-holster timer.
///
/// Bug shape this catches: a refactor that drops the
/// prev-vs-new comparison in the SyncBandolierItems handler
/// regresses to "weapon equip into bandolier doesn't unholster" —
/// the symptom that drove this fix.
#[tokio::test]
async fn sync_bandolier_items_active_slot_gained_weapon_draws_and_animates() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.archetype_id = Some(1);
        e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
        e.weapon_holstered = true; // before the equip
        e.active_bandolier_slot = 0;
        // Active slot starts empty — player hasn't equipped a weapon yet.
        e.bandolier_items.clear();
    }
    mgr.connect_entity(1);
    mgr.sequence_map
        .insert((804, crate::cell::spawner::EVENT_ITEM_EQUIP), 1872);

    let item = BandolierItem {
        item_id: 55,
        clip_size: 15,
        default_ammo_type: 2,
        current_ammo: 0,
        cur_ammo_type: 2,
    };

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::SyncBandolierItems {
            entity_id: 1,
            active_bandolier_slot: 0,
            bandolier_items: vec![(0, item)],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let e = mgr.get_entity(1).unwrap();
    assert!(
        !e.weapon_holstered,
        "player-driven equip into bandolier must draw the weapon — \
         this is the playtest symptom: 'on equip is when it needs to unholster'",
    );
    assert!(
        e.combat_exit_at.is_some(),
        "OOC re-holster timer must arm so the weapon goes away after \
         OOC_HOLSTER_DELAY, matching the grant path's behavior",
    );

    let mut refresh_count = 0u32;
    let mut sequence_count = 0u32;
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::RefreshAppearance {
                holstered: false, ..
            } => refresh_count += 1,
            CellToBaseMsg::EntityMethodCall { method_index, .. }
                if method_index == crate::cell::client_methods::spawnable_entity::ON_SEQUENCE =>
            {
                sequence_count += 1;
            }
            _ => {}
        }
    }
    assert_eq!(
        refresh_count, 1,
        "equip into bandolier must dispatch exactly one RefreshAppearance — \
         without it the weapon mesh never attaches on the client model",
    );
    assert_eq!(
        sequence_count, 1,
        "equip into bandolier must fire exactly one Item_Equip — without it the \
         weapon teleports into the hand with no unholster animation",
    );
}

/// `SyncBandolierItems` when the active slot LOST its weapon —
/// player-driven unequip via right-click or drag.
///
/// Must:
/// - Fire `Item_Unequip` (event 4001) so the client plays the
///   holster animation while the mesh is still attached.
/// - Arm `holster_animation_complete_at` so `holster_timer_tick`
///   Phase 2 fires `RefreshAppearance(holstered=true)` after the
///   animation has had time to play, dropping the mesh from the
///   `ComponentList`.
/// - Clear `combat_exit_at` so a stale OOC timer doesn't re-fire
///   the same animation.
/// - LEAVE `weapon_holstered` at its current value — Phase 2
///   flips it to true when it broadcasts. Setting it here
///   immediately would defeat the purpose (the base-side
///   `refresh_player_appearance` reads cached holstered state).
///
/// Bug shape this catches: a refactor that drops the
/// `Item_Unequip` dispatch or the Phase 2 scheduling regresses
/// to "weapon vanishes instantly with no holster animation".
#[tokio::test]
async fn sync_bandolier_items_active_slot_lost_weapon_fires_unequip_and_schedules_phase2() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.archetype_id = Some(1);
        e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
        e.weapon_holstered = false; // weapon was drawn (post-equip grace)
        e.active_bandolier_slot = 0;
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 55,
                clip_size: 15,
                default_ammo_type: 2,
                current_ammo: 7,
                cur_ammo_type: 2,
            },
        );
        // OOC timer armed — would fire Item_Unequip in 10s.
        e.combat_exit_at = Some(std::time::Instant::now());
        e.holster_animation_complete_at = None;
    }
    mgr.connect_entity(1);
    // Seed sequence map for the Item_Unequip lookup (archetype 1
    // → event set 804 → seq 1873).
    mgr.sequence_map
        .insert((804, crate::cell::spawner::EVENT_ITEM_UNEQUIP), 1873);

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    // Unequip: SyncBandolierItems with empty active slot.
    handle_base_message(
        BaseToCellMsg::SyncBandolierItems {
            entity_id: 1,
            active_bandolier_slot: 0,
            bandolier_items: vec![],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let e = mgr.get_entity(1).unwrap();
    assert!(
        !e.weapon_holstered,
        "unequip must NOT immediately flip weapon_holstered=true \
         — Phase 2 does that when it broadcasts. Flipping here \
         defeats the purpose (mesh removal races animation)",
    );
    assert!(
        e.combat_exit_at.is_none(),
        "unequip must disarm any pending OOC re-holster timer — \
         without this, holster_timer_tick fires Item_Unequip a \
         second time after the OOC grace expires",
    );
    assert!(
        e.holster_animation_complete_at.is_some(),
        "unequip must schedule Phase 2 via holster_animation_complete_at \
         — that's the hook holster_timer_tick uses to send \
         RefreshAppearance(holstered=true) after the animation",
    );

    let mut saw_unequip_sequence = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
            if method_index == crate::cell::client_methods::spawnable_entity::ON_SEQUENCE {
                saw_unequip_sequence = true;
            }
        }
    }
    assert!(
        saw_unequip_sequence,
        "unequip must fire ON_SEQUENCE (Item_Unequip) so the client \
         plays the holster animation while the mesh is still attached",
    );
}

/// `SyncBandolierItems` when the active slot is UNCHANGED (same
/// item_id as before) must NOT re-fire the equip-display chain.
/// This catches the "post-vendor-buy resync re-equips the weapon
/// you already had" regression — base resyncs the bandolier after
/// any inventory change, and we don't want a stash-slot grant to
/// retrigger the active weapon's equip animation.
#[tokio::test]
async fn sync_bandolier_items_active_slot_unchanged_does_not_re_animate() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.weapon_holstered = false; // already drawn — mid-fight or post-equip grace
        e.active_bandolier_slot = 0;
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 55,
                clip_size: 15,
                default_ammo_type: 2,
                current_ammo: 7,
                cur_ammo_type: 2,
            },
        );
        // Mock that combat_exit_at was stamped a while ago — we
        // want to verify the resync DOESN'T reset it (which would
        // extend the OOC grace window mid-game).
        e.combat_exit_at = Some(std::time::Instant::now() - std::time::Duration::from_secs(5));
    }
    mgr.connect_entity(1);

    let prior_combat_exit_at = mgr.get_entity(1).unwrap().combat_exit_at;

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    // Resync with same active item — picks up new stash slot.
    handle_base_message(
        BaseToCellMsg::SyncBandolierItems {
            entity_id: 1,
            active_bandolier_slot: 0,
            bandolier_items: vec![
                (
                    0,
                    BandolierItem {
                        item_id: 55,
                        clip_size: 15,
                        default_ammo_type: 2,
                        current_ammo: 7,
                        cur_ammo_type: 2,
                    },
                ),
                (
                    2,
                    BandolierItem {
                        item_id: 99,
                        clip_size: 30,
                        default_ammo_type: 1,
                        current_ammo: 30,
                        cur_ammo_type: 1,
                    },
                ),
            ],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let e = mgr.get_entity(1).unwrap();
    assert_eq!(
        e.combat_exit_at, prior_combat_exit_at,
        "resync that doesn't change the active slot's item must NOT \
         restamp combat_exit_at — that would extend the OOC grace \
         window every time the player gets ANY new stash item",
    );

    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::RefreshAppearance { .. } => {
                panic!(
                    "unchanged-active-slot resync must NOT dispatch \
                     RefreshAppearance — broadcasting on every stash \
                     update is wire spam",
                );
            }
            CellToBaseMsg::EntityMethodCall { method_index, .. }
                if method_index == crate::cell::client_methods::spawnable_entity::ON_SEQUENCE =>
            {
                panic!("unchanged-active-slot resync must NOT re-fire Item_Equip");
            }
            _ => {}
        }
    }
}
