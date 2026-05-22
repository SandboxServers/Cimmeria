use super::*;

#[tokio::test]
async fn update_bandolier_item_inserts_slot_and_sets_active() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    let item = BandolierItem {
        item_id: 42,
        clip_size: 25,
        default_ammo_type: 2,
        current_ammo: 25,
        cur_ammo_type: 2,
    };

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::UpdateBandolierItem {
            entity_id: 1,
            slot_id: 2,
            item,
            make_active: true,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let entity = mgr.get_entity(1).unwrap();
    assert_eq!(
        entity.active_bandolier_slot, 2,
        "active_bandolier_slot must update"
    );
    assert!(
        entity.bandolier_items.contains_key(&2),
        "bandolier_items must contain the new slot"
    );
}

/// `UpdateBandolierItem` with `make_active=true` on an OOC player
/// must draw the weapon, stamp the OOC re-holster timer, and
/// dispatch a `RefreshAppearance` (mesh attach for the equip
/// animation) + an `ON_SEQUENCE` (Item_Equip animation).
///
/// Bug shape this catches: a refactor that drops the draw-on-equip
/// hook leaves the player picking up a weapon that stays invisible
/// until the next combat enter — the symptom that drove this fix
/// (the player's mental model is "I just equipped this, I should
/// see it").
#[tokio::test]
async fn update_bandolier_item_draws_weapon_and_arms_holster_timer_when_active() {
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
        e.weapon_holstered = true; // OOC + holstered (post-grace)
        e.combat_exit_at = None;
    }
    mgr.connect_entity(1);
    // Seed the Item_Equip sequence so `fire_item_sequence` can resolve.
    // Soldier (archetype 1) → event set 804 → seq 1872 (Item_Equip).
    mgr.sequence_map
        .insert((804, crate::cell::spawner::EVENT_ITEM_EQUIP), 1872);

    let item = BandolierItem {
        item_id: 42,
        clip_size: 25,
        default_ammo_type: 2,
        current_ammo: 25,
        cur_ammo_type: 2,
    };

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::UpdateBandolierItem {
            entity_id: 1,
            slot_id: 0,
            item,
            make_active: true,
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
        "equip on OOC player must draw the weapon so it's visible",
    );
    assert!(
        e.combat_exit_at.is_some(),
        "OOC holster timer must be armed so the weapon re-holsters \
         after OOC_HOLSTER_DELAY (matches the post-combat behavior — \
         player sees the weapon for a few seconds then it goes away)",
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
        "equip must dispatch exactly one RefreshAppearance(holstered=false) so the \
         client attaches the weapon mesh before the equip animation",
    );
    assert_eq!(
        sequence_count, 1,
        "equip must fire exactly one Item_Equip onSequence so the client plays the \
         equip animation. Without it, the weapon just teleports into the \
         hand with no animation.",
    );
}

/// `UpdateBandolierItem` with `make_active=false` but a slot that
/// MATCHES the entity's already-active slot must STILL draw the
/// weapon. This is the initial-grant case: base's
/// `handle_grant_item` SQL only flips `bandolier_slot` when the
/// previous selection points at a vacant slot — but the INSERT
/// happens BEFORE the UPDATE, so for an initial grant where
/// `next_slot == p.bandolier_slot == 0`, the NOT EXISTS check
/// finds the just-inserted row and the UPDATE is skipped,
/// leaving `bandolier_became_active=false`. The new weapon IS
/// the active one even though base couldn't say so, and the
/// equip animation MUST fire — otherwise the player's first
/// weapon never appears.
///
/// Bug shape this catches: a refactor that ties the equip-display
/// gate strictly to `make_active=true` regresses initial weapon
/// grants back to invisible-on-equip.
#[tokio::test]
async fn update_bandolier_item_make_active_false_but_slot_matches_active_still_draws() {
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
        e.weapon_holstered = true;
        // Player's default active slot is 0 (fresh character).
        e.active_bandolier_slot = 0;
    }
    mgr.connect_entity(1);
    mgr.sequence_map
        .insert((804, crate::cell::spawner::EVENT_ITEM_EQUIP), 1872);

    let item = BandolierItem {
        item_id: 42,
        clip_size: 25,
        default_ammo_type: 2,
        current_ammo: 25,
        cur_ammo_type: 2,
    };

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    // Initial weapon grant where base's bandolier_slot UPDATE
    // skipped (next_slot == p.bandolier_slot == 0, NOT EXISTS
    // finds the new INSERT). So `make_active=false`, even though
    // this IS the active slot from the entity's perspective.
    handle_base_message(
        BaseToCellMsg::UpdateBandolierItem {
            entity_id: 1,
            slot_id: 0,
            item,
            make_active: false,
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
        "initial grant into the already-active slot must draw the weapon — \
         without this the first weapon a player ever receives stays \
         invisible (the bug from playtest)",
    );
    assert!(
        e.combat_exit_at.is_some(),
        "OOC holster timer must arm so the weapon re-holsters after \
         OOC_HOLSTER_DELAY, just like the `make_active=true` path",
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
        "initial grant must dispatch exactly one RefreshAppearance — the base's \
         refresh_player_appearance is skipped on container_id=3 to \
         avoid racing this cell-side broadcast, so this IS the broadcast \
         that attaches the weapon mesh",
    );
    assert_eq!(
        sequence_count, 1,
        "initial grant must fire exactly one Item_Equip — same animation as a \
         slot-swap into the active slot",
    );
}

/// `UpdateBandolierItem` with `make_active=false` AND a slot
/// different from the entity's active slot must NOT draw the
/// weapon. The player still has their existing active weapon —
/// this is a non-active stash slot grant.
///
/// Bug shape this catches: a refactor that drops the `slot_is_active`
/// gate and always fires equip-display would yank the visible
/// weapon mid-fight when a quest reward lands in a stash slot.
#[tokio::test]
async fn update_bandolier_item_make_active_false_into_non_active_slot_does_not_draw() {
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
        e.weapon_holstered = true;
        // Player's active slot is 0 (their existing pistol).
        e.active_bandolier_slot = 0;
    }
    mgr.connect_entity(1);

    let item = BandolierItem {
        item_id: 99,
        clip_size: 30,
        default_ammo_type: 1,
        current_ammo: 30,
        cur_ammo_type: 1,
    };

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    // Quest reward lands in slot 2, not the active slot 0.
    handle_base_message(
        BaseToCellMsg::UpdateBandolierItem {
            entity_id: 1,
            slot_id: 2,
            item,
            make_active: false,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let e = mgr.get_entity(1).unwrap();
    assert_eq!(
        e.active_bandolier_slot, 0,
        "stash-slot grant must NOT touch active_bandolier_slot",
    );
    assert!(
        e.weapon_holstered,
        "stash-slot grant must NOT draw the weapon — player still has \
         their existing active weapon (which is holstered here)",
    );
    assert!(
        e.combat_exit_at.is_none(),
        "stash-slot grant must NOT arm the OOC holster timer",
    );

    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::RefreshAppearance { .. } => {
                panic!(
                    "stash-slot grant must NOT dispatch RefreshAppearance — \
                     the active slot's appearance is unchanged",
                );
            }
            CellToBaseMsg::EntityMethodCall { method_index, .. }
                if method_index == crate::cell::client_methods::spawnable_entity::ON_SEQUENCE =>
            {
                panic!("stash-slot grant must NOT fire Item_Equip");
            }
            _ => {}
        }
    }
}

/// `UpdateBandolierItem` while in combat must NOT arm the OOC
/// holster timer — `combat_exit_at` is supposed to be stamped by
/// `exit_player_combat`, and stamping it here would let the holster
/// fire mid-combat (the holster scan doesn't gate on
/// `threatened_mobs.is_empty()` today).
#[tokio::test]
async fn update_bandolier_item_in_combat_does_not_arm_holster_timer() {
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
        e.weapon_holstered = false; // in combat, already drawn
        e.combat_exit_at = None;
        e.threatened_mobs.insert(999); // pretend there's an aggro
    }
    mgr.connect_entity(1);

    let item = BandolierItem {
        item_id: 42,
        clip_size: 25,
        default_ammo_type: 2,
        current_ammo: 25,
        cur_ammo_type: 2,
    };

    let (tx, _rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::UpdateBandolierItem {
            entity_id: 1,
            slot_id: 0,
            item,
            make_active: true,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.combat_exit_at.is_none(),
        "in-combat equip must NOT stamp combat_exit_at — the timer \
         would otherwise fire mid-combat and holster the weapon while \
         the player is still fighting. `exit_player_combat` stamps \
         this on combat exit naturally.",
    );
}
