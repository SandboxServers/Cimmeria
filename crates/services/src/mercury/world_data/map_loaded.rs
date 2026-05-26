//! `mapLoaded()` multi-packet builder: assembles all entity method calls for
//! client game state initialization, then fragments into encrypted Mercury packets.

use cimmeria_mercury::packet::build_fragmented_bundle;

use super::stats::{archetype_stats, level_exp};
use super::{
    append_entity_method, build_world_params_args, encrypt_packet, method_idx, write_wstring,
    PlayerLoadData, WorldEntryInfo, REPLY_FLAGS, SKIN_TINTS,
};

// ── mapLoaded multi-packet builder ───────────────────────────────────────────

/// Build and encrypt the complete `mapLoaded()` packet sequence.
///
/// Sent after the enter-world step. Contains all entity method calls needed to
/// initialize the client's game state: world parameters, stats, appearance,
/// abilities, inventory stubs, experience, and the final `onPlayerDataLoaded`
/// signal that transitions the client from loading screen to gameplay.
///
/// Returns multiple encrypted packets to stay within Mercury's 1411-byte
/// plaintext body limit (`MAX_BODY_LENGTH`). Each packet is independently
/// encrypted and gets its own seq ID (base_seq, base_seq+1, ...). Acks are
/// only included on the first packet.
///
/// Mirrors `python/cell/SGWPlayer.py:464-546`.
/// Constructs ALL entity method calls (stats, appearance, inventory, etc.) into
/// one contiguous body, then uses Mercury fragmentation to split it across
/// multiple encrypted UDP packets. The client reassembles fragments into a single
/// bundle before processing, ensuring all entity data is handled atomically.
///
/// VIEWPORT + CELL + POSITION body bytes (from [`super::phases::build_enter_world_body`])
/// are prepended to this body by the caller before fragmenting, matching the
/// C++ CellApp behavior where these are in the same channel bundle.
pub fn build_map_loaded(
    key: &[u8; 32],
    base_seq: u32,
    acks: &[u32],
    entity_id: u32,
    data: &PlayerLoadData,
    world_entry: &WorldEntryInfo,
) -> (Vec<Vec<u8>>, u32) {
    let body = build_map_loaded_body(entity_id, data, world_entry);
    fragment_map_loaded(key, base_seq, acks, &body)
}

/// Build just the raw body bytes for the `mapLoaded()` entity method sequence.
///
/// Returns the unencrypted, unfragmented body that the caller can measure
/// (to reserve sequence numbers atomically) before fragmenting + encrypting.
pub fn build_map_loaded_body(
    entity_id: u32,
    data: &PlayerLoadData,
    world_entry: &WorldEntryInfo,
) -> Vec<u8> {
    let stats = archetype_stats(data.archetype);
    let mut body = Vec::with_capacity(8192);
    build_map_loaded_body_inner(&mut body, entity_id, data, world_entry, &stats);
    tracing::info!(
        entity_id,
        body_bytes = body.len(),
        "mapLoaded: body assembled"
    );
    body
}

/// Calculate how many Mercury fragment packets a body of `body_len` bytes requires.
pub fn fragment_count(body_len: usize) -> u32 {
    use cimmeria_mercury::packet::FRAGMENT_BODY_SIZE;
    if body_len <= FRAGMENT_BODY_SIZE {
        1
    } else {
        body_len.div_ceil(FRAGMENT_BODY_SIZE) as u32
    }
}

/// Fragment and encrypt a pre-built body into Mercury packets.
///
/// The caller must have already reserved `fragment_count(body.len())` sequence
/// numbers starting at `base_seq` to avoid races with tick-sync.
pub fn fragment_map_loaded(
    key: &[u8; 32],
    base_seq: u32,
    acks: &[u32],
    body: &[u8],
) -> (Vec<Vec<u8>>, u32) {
    let key_copy = *key;
    build_fragmented_bundle(REPLY_FLAGS, body, base_seq, acks, |plaintext| {
        encrypt_packet(plaintext, &key_copy)
    })
}

fn build_map_loaded_body_inner(
    body: &mut Vec<u8>,
    entity_id: u32,
    data: &PlayerLoadData,
    world_entry: &WorldEntryInfo,
    stats: &super::ArchetypeStats,
) {
    // Helper: append an entity method call to the body.
    macro_rules! append_method {
        ($method_idx:expr, $args:expr) => {{
            append_entity_method(body, $method_idx, entity_id, $args);
        }};
    }

    // 0. setupWorldParameters (22 args: 5xi32 + 17xf32)
    append_method!(
        method_idx::SETUP_WORLD_PARAMETERS,
        &build_world_params_args(&world_entry.world_name)
    );

    // 1. BeingAppearance + 2. onEntityTint — promoted to the front of the
    //    mapLoaded body so they land in the first fragment alongside
    //    `createCellPlayer`, instead of fragments 4-5. Without this the
    //    player entity exists on the client with no body model for the
    //    50-200 ms it takes the rest of the bundle to arrive. (The
    //    `build_create_player` path also pre-warms these now; keeping them
    //    here too lets a UDP-dropped pre-warm still self-heal once the
    //    mapLoaded bundle lands.)
    {
        // Player spawns weapon-holstered. The wire `ComponentList` omits
        // the active bandolier weapon visual; the client's appearance
        // compositor (`ghidra://SGW.exe@0x00ec0840`) selects the
        // unarmed-stance pose when no weapon-shaped entry is present.
        // See `PlayerLoadData::appearance_components` for the helper.
        let wire_components = data.appearance_components(true);
        tracing::info!(
            bodyset = %data.bodyset,
            bodyset_len = data.bodyset.len(),
            component_count = wire_components.len(),
            components = ?wire_components,
            holstered_weapon = ?data.weapon_visual,
            skin_color_id = data.skin_color_id,
            "mapLoaded: BeingAppearance + onEntityTint data (spawn-holstered)"
        );
        let mut args = Vec::new();
        write_wstring(&mut args, &data.bodyset);
        args.extend_from_slice(&(wire_components.len() as u32).to_le_bytes());
        for comp in &wire_components {
            write_wstring(&mut args, comp);
        }
        append_method!(method_idx::BEING_APPEARANCE, &args);
    }
    {
        let skin_tint = if (data.skin_color_id as usize) < SKIN_TINTS.len() {
            SKIN_TINTS[data.skin_color_id as usize]
        } else {
            SKIN_TINTS[0]
        };
        let mut args = Vec::with_capacity(12);
        args.extend_from_slice(&0u32.to_le_bytes()); // primaryColorId
        args.extend_from_slice(&0u32.to_le_bytes()); // secondaryColorId
        args.extend_from_slice(&skin_tint.to_le_bytes());
        append_method!(method_idx::ON_ENTITY_TINT, &args);
    }

    // 3. setupStargateInfo (3xARRAY<INT32>: world, known, hidden)
    {
        let mut args = Vec::new();
        // worldStargateIds: stargates physically present in the destination
        // world (queried by query_world_stargates and stored in WorldEntryInfo).
        // Cap at u32::MAX entries and only serialize that many — using the same
        // count for both the length prefix and the loop ensures a corrupt
        // input (>2^32 entries) can't desync the encoded length from the
        // actual payload count.
        let world_count = world_entry.world_stargates.len().min(u32::MAX as usize);
        args.extend_from_slice(&(world_count as u32).to_le_bytes());
        for &sg in world_entry.world_stargates.iter().take(world_count) {
            args.extend_from_slice(&sg.to_le_bytes());
        }
        // knownStargateIds: address-book entries the player has unlocked.
        let known_count = data.known_stargates.len().min(u32::MAX as usize);
        args.extend_from_slice(&(known_count as u32).to_le_bytes());
        for &sg in data.known_stargates.iter().take(known_count) {
            args.extend_from_slice(&sg.to_le_bytes());
        }
        args.extend_from_slice(&0u32.to_le_bytes()); // hiddenStargates: empty
        append_method!(method_idx::SETUP_STARGATE_INFO, &args);
    }

    // 2. clearClientHintedGenericRegions (no args)
    append_method!(method_idx::CLEAR_HINTED_REGIONS, &[]);

    // 3. onTimeofDay(FLOAT32, FLOAT32, UINT8) — hardcoded sun position
    {
        let mut args = Vec::with_capacity(9);
        args.extend_from_slice(&0.129059f32.to_le_bytes());
        args.extend_from_slice(&1.0f32.to_le_bytes());
        args.push(1u8);
        append_method!(method_idx::ON_TIME_OF_DAY, &args);
    }

    // 4. onLevelUpdate(INT32)
    append_method!(method_idx::ON_LEVEL_UPDATE, &data.level.to_le_bytes());

    // 5. onBeingNameUpdate(UNICODE_STRING)
    {
        let mut args = Vec::new();
        write_wstring(&mut args, &data.player_name);
        append_method!(method_idx::ON_BEING_NAME_UPDATE, &args);
    }

    // 6. onStateFieldUpdate(UINT32) — default 0
    append_method!(method_idx::ON_STATE_FIELD_UPDATE, &0u32.to_le_bytes());

    // 7. onKismetEventSetUpdate(INT32) — default 1025
    append_method!(
        method_idx::ON_KISMET_EVENT_SET_UPDATE,
        &1025i32.to_le_bytes()
    );

    // 8. sendStats: onStatUpdate + onStatBaseUpdate
    //    Uses StatList from entity crate — sends ALL stats, matching Python's
    //    `self.sendStats(self.client, False, False)` which sends everything.
    {
        use cimmeria_entity::stats::{ArchetypeStatValues, StatList, AMMO_SLOT_1};
        let mut stat_list = StatList::new();
        stat_list.apply_archetype(&ArchetypeStatValues {
            coordination: stats.coordination,
            engagement: stats.engagement,
            fortitude: stats.fortitude,
            morale: stats.morale,
            perception: stats.perception,
            intelligence: stats.intelligence,
            health: stats.health,
            focus: stats.focus,
            health_per_level: stats.health_per_level,
            focus_per_level: stats.focus_per_level,
        });
        // Seed AmmoSlot{N} stats from persisted bandolier ammo so the UI
        // shows the correct value at world entry. Without this seed every
        // re-login sends the default (0, 0, 0) tuple — the cell-side
        // InitPlayerState seeding (service.rs) sets the stats correctly on
        // the entity but happens after this packet is already on the wire.
        for (slot_id, item) in &data.bandolier_items {
            let stat_id = AMMO_SLOT_1 + slot_id;
            if let Some(stat) = stat_list.get_mut(stat_id) {
                stat.update(0, item.current_ammo, item.clip_size);
            }
        }
        // onStatUpdate: dynamic values (min, current, max)
        let stat_args = stat_list.serialize_all();
        append_method!(method_idx::ON_STAT_UPDATE, &stat_args);
        // onStatBaseUpdate: base values (same for fresh characters)
        let base_args = stat_list.serialize_all_base();
        append_method!(method_idx::ON_STAT_BASE_UPDATE, &base_args);
    }

    // 9. onArchetypeUpdate(INT32)
    append_method!(
        method_idx::ON_ARCHETYPE_UPDATE,
        &data.archetype.to_le_bytes()
    );

    // 10. onAlignmentUpdate(INT8)
    append_method!(method_idx::ON_ALIGNMENT_UPDATE, &[data.alignment as u8]);

    // 11. onFactionUpdate(INT8) — hardcoded 3 (from setupPlayer)
    append_method!(method_idx::ON_FACTION_UPDATE, &[3u8]);

    // 12. onAbilityTreeInfo(ARRAY<ARRAY<INT32>>) — 3 ability tree branches
    //     Extended encoding (method_index 141 >= 128)
    {
        let tree_args = data.ability_tree.serialize();
        append_method!(method_idx::ON_ABILITY_TREE_INFO, &tree_args);
    }

    // 13. onKnownAbilitiesUpdate(ARRAY<INT32>)
    {
        let mut args = Vec::with_capacity(4 + data.abilities.len() * 4);
        args.extend_from_slice(&(data.abilities.len() as u32).to_le_bytes());
        for &id in &data.abilities {
            args.extend_from_slice(&id.to_le_bytes());
        }
        tracing::info!(
            player_id = data.player_id,
            ability_count = data.abilities.len(),
            abilities = ?data.abilities,
            "mapLoaded: sending onKnownAbilitiesUpdate"
        );
        append_method!(method_idx::ON_KNOWN_ABILITIES_UPDATE, &args);
    }

    // 14. onResetMapInfo (no args)
    append_method!(method_idx::ON_RESET_MAP_INFO, &[]);

    // BeingAppearance + onEntityTint moved to the top of the mapLoaded body
    // (steps 1-2) so they land in the first fragment.

    // 17. onExtraNameUpdate(UNICODE_STRING) — extended encoding
    {
        let mut args = Vec::new();
        write_wstring(&mut args, &data.extra_name);
        append_method!(method_idx::ON_EXTRA_NAME_UPDATE, &args);
    }

    // 18. onExpUpdate(INT32) — extended encoding
    append_method!(method_idx::ON_EXP_UPDATE, &data.exp.to_le_bytes());

    // 19. onMaxExpUpdate(INT32) — extended encoding
    append_method!(
        method_idx::ON_MAX_EXP_UPDATE,
        &level_exp(data.level).to_le_bytes()
    );

    // 20. onEntityProperty x6 (INT32 propId, INT32 value)
    //     GENERICPROPERTY IDs from Atrea.enums
    //
    // Stage C: AmmoTypeId is sourced directly from the active bandolier item.
    // No shadow `active_ammo_type` field on PlayerLoadData anymore — if no
    // item is in the active slot we fall back to 0 (matches legacy "no weapon
    // equipped" behavior).
    let active_ammo_type = data
        .bandolier_items
        .iter()
        .find(|(slot, _)| *slot == data.active_bandolier_slot)
        .map_or(0, |(_, item)| item.cur_ammo_type);
    for &(prop_id, value) in &[
        (2i32, data.applied_science_points), // AppliedSciencePoints
        (1, data.training_points),           // TrainingPoints
        (7, data.access_level),              // AccessLevel
        (8, data.gender),                    // Gender
        (4, 0),                              // PvPFlag
        (3, active_ammo_type),               // AmmoTypeId
    ] {
        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&prop_id.to_le_bytes());
        args.extend_from_slice(&value.to_le_bytes());
        append_method!(method_idx::ON_ENTITY_PROPERTY, &args);
    }

    // 21. Inventory: onBagInfo(ARRAY<BagInfo>) — single call with all bags
    {
        use cimmeria_entity::inventory::Inventory;
        let inv = Inventory::new(data.naquadah);
        let bag_info = inv.serialize_bag_info();
        append_method!(method_idx::ON_BAG_INFO, &bag_info);
    }

    // 21a. onActiveSlotUpdate(bagId, slotId) — initialise the client's
    // bandolier slot indicator from the persisted `bandolier_slot`. Without
    // this seed the client UI defaults to slot 1 (1-indexed) regardless of
    // what the DB says, and the LUA `getActiveSlotForContainer(...)`
    // comparator silently drops keypresses for whatever the client thinks
    // is already active. Wire format matches `Bag.py:369` —
    // `(bagId, activeSlotId + 1)`.
    {
        const CONTAINER_BANDOLIER: i32 = 3;
        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&CONTAINER_BANDOLIER.to_le_bytes());
        args.extend_from_slice(&(data.active_bandolier_slot + 1).to_le_bytes());
        append_method!(method_idx::ON_ACTIVE_SLOT_UPDATE, &args);
    }

    // 21b. onUpdateItem(ARRAY<InvItem>) — all inventory items
    //      Reference: python/cell/Inventory.py flushUpdates() step 3
    if !data.items.is_empty() {
        let mut args = Vec::with_capacity(4 + data.items.len() * 48);
        args.extend_from_slice(&(data.items.len() as u32).to_le_bytes());
        for item in &data.items {
            item.serialize(&mut args);
        }
        append_method!(method_idx::ON_UPDATE_ITEM, &args);
    }

    // 22. onCashChanged(INT32) — naquadah
    append_method!(method_idx::ON_CASH_CHANGED, &data.naquadah.to_le_bytes());

    // 23. onUpdateKnownCrafts(ARRAY<INT32>) — extended encoding
    {
        let mut args = Vec::with_capacity(4 + data.blueprint_ids.len() * 4);
        args.extend_from_slice(&(data.blueprint_ids.len() as u32).to_le_bytes());
        for &bp in &data.blueprint_ids {
            args.extend_from_slice(&bp.to_le_bytes());
        }
        append_method!(method_idx::ON_UPDATE_KNOWN_CRAFTS, &args);
    }

    // NOTE: `onChatJoined` × 8 channels and `onPlayerCommunication` (welcome
    // message) used to live here but were moved to `handle_on_client_ready`
    // — that's where the original `python/base/SGWPlayer.py onClientReady
    // -> ChannelManager.playerLoggedIn` flow runs them. Keeping them in the
    // mapLoaded bundle padded ~311 B onto the worst-case fragment burst, and
    // the one-RTT window where the player is in the world without registered
    // chat channels is identical to the original game's behavior.

    // NOTE: onPlayMovie (first-login cinematic) is intentionally NOT in this
    // bundle. It is deferred to `handle_on_client_ready` and fired AFTER the
    // BeingAppearance / onEntityTint resends so the cinematic plays against a
    // pawn that's already possessed AND has its model bound. Firing it here
    // (in the mapLoaded bundle, before onClientReady) lets the cinematic-exit
    // CollectGarbage reclaim the in-flight appearance asset, producing a
    // one-frame "dev cube" flash on the first post-cinematic render. See
    // issue #288 and python/cell/SGWPlayer.py:558-565 (the original flow
    // gated mapLoaded() ITSELF on onClientReady, which had the same effect).

    // 24. onPlayerDataLoaded (no args) — client transitions to gameplay
    append_method!(method_idx::ON_PLAYER_DATA_LOADED, &[]);

    // 25. onTargetUpdate(INT32) — default 0 (no target)
    append_method!(method_idx::ON_TARGET_UPDATE, &0i32.to_le_bytes());
}
