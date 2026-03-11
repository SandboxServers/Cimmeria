//! World entry builders and data: map loading, world parameters, archetype stats,
//! ability trees, and the `mapLoaded()` multi-packet sequence.

use cimmeria_mercury::packet::{build_outgoing, build_fragmented_bundle, FLAG_HAS_ACKS};

use super::{
    encrypt_packet, write_wstring, append_entity_method, method_idx,
    REPLY_FLAGS, BASEMSG_CREATE_BASE_PLAYER, BASEMSG_SPACE_VIEWPORT_INFO,
    BASEMSG_CREATE_CELL_PLAYER, BASEMSG_FORCED_POSITION, SGWPLAYER_CLASS_ID,
    SKIN_TINTS,
};
use super::types::{ArchetypeStats, PlayerLoadData, WorldEntryInfo};

// ── World entry phase builders ───────────────────────────────────────────────

/// Phase 5b-A: CREATE_BASE_PLAYER + onClientMapLoad.
///
/// Sends only the base entity creation and terrain load notification.
/// The client will load terrain geometry and respond with `mapLoaded` (cell
/// method index 25, msg_id 0x99). Only after receiving that should the server
/// send Phase 5b-B (viewport + cell player + forced position + entity data).
///
/// In C++, BaseApp's `enableEntities()` sends CREATE_BASE_PLAYER then triggers
/// CellApp `sendConnectEntity`. CellApp's `connected()` callback sends
/// `onClientMapLoad`. The client loads terrain, sends `mapLoaded`, and *then*
/// CellApp responds with viewport + cell + position + the full setup sequence.
pub fn build_world_entry_phase_a(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    info: &WorldEntryInfo,
) -> Vec<u8> {
    let mut body = Vec::with_capacity(256);

    // 1. CREATE_BASE_PLAYER for SGWPlayer (WORD_LENGTH = 6)
    body.push(BASEMSG_CREATE_BASE_PLAYER);
    body.extend_from_slice(&6u16.to_le_bytes());
    body.extend_from_slice(&info.player_entity_id.to_le_bytes());
    body.push(SGWPLAYER_CLASS_ID);
    body.push(0x00); // propertyCount = 0

    // 2. onClientMapLoad — tells the client which terrain to load.
    //    Client loads geometry then sends mapLoaded (0x99) when ready.
    {
        let mut args = Vec::new();
        // areaName (WSTRING): world display name
        write_wstring(&mut args, &info.world_name);
        // mapPath (WSTRING): client terrain path (matches client_map column in worlds table)
        let client_map = client_map_for_world(&info.world_name);
        write_wstring(&mut args, client_map);
        // WorldID (INT32)
        args.extend_from_slice(&world_id_for_name(&info.world_name).to_le_bytes());
        // Location (VECTOR3)
        for &c in &info.pos {
            args.extend_from_slice(&c.to_le_bytes());
        }
        // Direction (VECTOR3) — Y/Z swapped per BigWorld convention (heading in Z)
        args.extend_from_slice(&0.0f32.to_le_bytes()); // X
        args.extend_from_slice(&0.0f32.to_le_bytes()); // Y = 0
        args.extend_from_slice(&info.rot[1].to_le_bytes()); // Z = heading
        append_entity_method(&mut body, method_idx::ON_CLIENT_MAP_LOAD, info.player_entity_id, &args);
    }

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build raw body bytes for Phase 5b-B: VIEWPORT + CELL_PLAYER + FORCED_POSITION.
///
/// Returns just the Mercury message bytes (~99 bytes) without packet framing.
/// The C++ server adds these messages to `channel_->bundle()` — the same bundle
/// that contains the mapLoaded entity methods — rather than sending them as a
/// separate packet. This function provides the raw bytes so the caller can
/// prepend them to the mapLoaded body before fragmenting into a single bundle.
pub fn build_world_entry_phase_b_body(info: &WorldEntryInfo) -> Vec<u8> {
    let mut body = Vec::with_capacity(128);

    // 1. spaceViewportInfo (0x08, CONSTANT_LENGTH = 13)
    body.push(BASEMSG_SPACE_VIEWPORT_INFO);
    body.extend_from_slice(&info.player_entity_id.to_le_bytes());
    body.extend_from_slice(&info.player_entity_id.to_le_bytes());
    body.extend_from_slice(&info.space_id.to_le_bytes());
    body.push(0x00); // viewportID = 0

    // 2. createCellPlayer (0x06, WORD_LENGTH = 32)
    body.push(BASEMSG_CREATE_CELL_PLAYER);
    body.extend_from_slice(&32u16.to_le_bytes());
    body.extend_from_slice(&info.space_id.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes()); // vehicleID = 0
    for &c in &info.pos {
        body.extend_from_slice(&c.to_le_bytes());
    }
    // C++ sends rotX, rotZ, rotY (Y and Z swapped in wire format)
    body.extend_from_slice(&info.rot[0].to_le_bytes()); // rotX
    body.extend_from_slice(&info.rot[2].to_le_bytes()); // rotZ (swapped)
    body.extend_from_slice(&info.rot[1].to_le_bytes()); // rotY (swapped)

    // 3. forcedPosition (0x31, CONSTANT_LENGTH = 49)
    body.push(BASEMSG_FORCED_POSITION);
    body.extend_from_slice(&info.player_entity_id.to_le_bytes());
    body.extend_from_slice(&info.space_id.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes()); // vehicleID = 0
    for &c in &info.pos {
        body.extend_from_slice(&c.to_le_bytes());
    }
    for &c in &[0.0f32, 0.0, 0.0] {
        body.extend_from_slice(&c.to_le_bytes());
    } // velocity = zero
    // C++ sends rotX, rotZ, rotY (swapped)
    body.extend_from_slice(&info.rot[0].to_le_bytes()); // rotX
    body.extend_from_slice(&info.rot[2].to_le_bytes()); // rotZ (swapped)
    body.extend_from_slice(&info.rot[1].to_le_bytes()); // rotY (swapped)
    body.push(0x01); // flags

    body
}

/// Phase 5b-B: VIEWPORT + CELL_PLAYER + FORCED_POSITION as a standalone packet.
///
/// Wraps [`build_world_entry_phase_b_body`] in packet framing and encryption.
/// Kept for tests/reference; the live path now embeds the body into the
/// mapLoaded fragmented bundle instead.
pub fn build_world_entry_phase_b(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    info: &WorldEntryInfo,
) -> Vec<u8> {
    let body = build_world_entry_phase_b_body(info);
    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Cell→client entity method: `onPlayerDataLoaded` (no args).
///
/// Flattened ClientMethods index 115 → msg_id = 0xF3.
/// This is the signal that tells the client "all player data has been sent,
/// transition from loading to gameplay mode."
///
/// Wire format: `[msg_id:0xF3][word_len:u16=4][entity_id:u32]`
pub fn build_on_player_data_loaded(
    key: &[u8; 32],
    seq: u32,
    acks: &[u32],
    entity_id: u32,
) -> Vec<u8> {
    let mut body = Vec::new();
    append_entity_method(&mut body, method_idx::ON_PLAYER_DATA_LOADED, entity_id, &[]);

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Cell→client entity method: `setupWorldParameters` (22 args: 5×i32 + 17×f32).
///
/// Flattened ClientMethods index 122 → extended encoding (0xBD + sub_index 61).
/// Sets world physics constants (gravity, movement speeds, etc.).
pub fn build_setup_world_parameters(
    key: &[u8; 32],
    seq: u32,
    acks: &[u32],
    entity_id: u32,
) -> Vec<u8> {
    let mut body = Vec::new();
    let args = build_world_params_args("CombatSim");
    append_entity_method(&mut body, method_idx::SETUP_WORLD_PARAMETERS, entity_id, &args);

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq), acks, None);
    encrypt_packet(&plaintext, key)
}

// ── Data lookup functions ────────────────────────────────────────────────────

/// Look up the world_id for a world name (from db/resources/Worlds/Seed/worlds.sql).
pub(crate) fn world_id_for_name(world_name: &str) -> i32 {
    match world_name {
        "CombatSim" => 1,
        "SandBox" => 2,
        "Tol-Alpha-00" => 3,
        "Tol-Alpha-01" => 4,
        "Tol-Alpha-02" => 5,
        "Ca-Alpha-00" => 6,
        "Ca-Alpha-01" => 7,
        "Castle" => 8,
        "Tol-POI-06" => 9,
        "Agnos" => 10,
        "Anima_Vitrus" => 11,
        "Castle_CellBlock" => 12,
        "Hebridan" => 13,
        "Kheb" => 14,
        "Lucia" => 15,
        "Naitac" => 16,
        "PrimHatak" => 17,
        "Omega_Site" => 18,
        "Tollana" => 19,
        "Agnos_Library" => 20,
        "Playground" => 21,
        "TestSGC1" => 22,
        "Beta_Site_Evo_1" => 23,
        "Dakara" => 24,
        "Harset" => 57,
        "SGC_W1" => 58,
        "Harset_CmdCenter" => 68,
        "Menfa_Dark" => 77,
        "Omega_Site_CmdCenter" => 80,
        "Pertho" => 83,
        "SGC" => 86,
        "SGC_W2" => 87,
        "Tollana_Curia" => 88,
        "Temple" => 89,
        "Yotunheim" => 90,
        "Holding_Area" => 91,
        "Vitrus" => 92,
        _ => {
            tracing::warn!(world = %world_name, "Unknown world_id — using 1 (CombatSim)");
            1
        }
    }
}

/// Look up the client terrain path for a world name (client_map from worlds.sql).
/// Most worlds use the same name; a few differ.
pub(crate) fn client_map_for_world(world_name: &str) -> &str {
    match world_name {
        "CombatSim" => "Combat_Terrain_Test",
        "SandBox" => "Harset_CmdCenter",
        "Tol-Alpha-00" => "Tol-Alpha_Pocket_00",
        "Tol-Alpha-01" => "Tol-Alpha_Pocket_01",
        "Tol-Alpha-02" => "Tol-Alpha_Pocket_02",
        "Ca-Alpha-00" => "Ca-Alpha_Pocket_00",
        "Ca-Alpha-01" => "Ca-Alpha_Pocket_01",
        "Tol-POI-06" => "Tol_POI_Test06",
        _ => world_name, // Most worlds: client_map == world_name
    }
}

/// Serialize setupWorldParameters argument payload (22 args from World.py defaults).
pub(crate) fn build_world_params_args(world_name: &str) -> Vec<u8> {
    let mut args = Vec::with_capacity(88);
    args.extend_from_slice(&world_id_for_name(world_name).to_le_bytes()); // worldId
    args.extend_from_slice(&0i32.to_le_bytes());       // weatherSetId
    args.extend_from_slice(&1i32.to_le_bytes());       // minToRealMinutes
    args.extend_from_slice(&1440i32.to_le_bytes());    // minutesPerDay
    args.extend_from_slice(&100000i32.to_le_bytes());  // currentTimeInSeconds
    args.extend_from_slice(&(-9.8f32).to_le_bytes());  // gravity
    args.extend_from_slice(&6.0f32.to_le_bytes());     // runSpeed
    args.extend_from_slice(&4.0f32.to_le_bytes());     // sidewaysRunSpeed
    args.extend_from_slice(&3.0f32.to_le_bytes());     // backwardsRunSpeed
    args.extend_from_slice(&2.0f32.to_le_bytes());     // walkSpeed
    args.extend_from_slice(&1.5f32.to_le_bytes());     // sidewaysWalkSpeed
    args.extend_from_slice(&1.0f32.to_le_bytes());     // backwardsWalkSpeed
    args.extend_from_slice(&3.0f32.to_le_bytes());     // crouchRunSpeed
    args.extend_from_slice(&2.0f32.to_le_bytes());     // sidewaysCrouchRunSpeed
    args.extend_from_slice(&1.5f32.to_le_bytes());     // backwardsCrouchRunSpeed
    args.extend_from_slice(&1.5f32.to_le_bytes());     // crouchWalkSpeed
    args.extend_from_slice(&1.0f32.to_le_bytes());     // sidewaysCrouchWalkSpeed
    args.extend_from_slice(&0.75f32.to_le_bytes());    // backwardsCrouchWalkSpeed
    args.extend_from_slice(&4.0f32.to_le_bytes());     // swimSpeed
    args.extend_from_slice(&2.5f32.to_le_bytes());     // sidewaysSwimSpeed
    args.extend_from_slice(&2.0f32.to_le_bytes());     // backwardsSwimSpeed
    args.extend_from_slice(&8.0f32.to_le_bytes());     // jumpSpeed
    args
}

// ── Archetype data ───────────────────────────────────────────────────────────

/// Look up archetype base stats by archetype ID.
///
/// Hardcoded from `db/resources/Archetypes/Seed/archetypes.sql`. All archetypes
/// except Commando share the same stat spread in the seed data.
pub fn archetype_stats(archetype_id: i32) -> ArchetypeStats {
    match archetype_id {
        2 => ArchetypeStats { // Commando (only one with different stats)
            coordination: 4, engagement: 4, fortitude: 2, morale: 3,
            perception: 5, intelligence: 3, health: 760, focus: 1570,
            health_per_level: 10, focus_per_level: 70,
        },
        _ => ArchetypeStats { // Soldier, Scientist, Archeologist, Asgard, Goa'uld, Shol'va, Jaffa
            coordination: 5, engagement: 4, fortitude: 3, morale: 4,
            perception: 3, intelligence: 2, health: 760, focus: 1570,
            health_per_level: 10, focus_per_level: 70,
        },
    }
}

/// Ability tree data per archetype (from `db/resources/Archetypes/Seed/archetype_ability_tree.sql`).
///
/// Only Soldier (1) and Commando (2) have tree data in seed; others get empty trees.
pub fn archetype_ability_tree(archetype_id: i32) -> cimmeria_entity::abilities::AbilityTreeData {
    use cimmeria_entity::abilities::AbilityTreeData;
    match archetype_id {
        1 => AbilityTreeData { // Soldier
            trees: [
                // Tree 0 (29 abilities)
                vec![597,603,604,610,611,616,617,621,622,623,625,627,641,
                     643,645,648,650,652,661,662,663,666,667,668,672,675,
                     677,679,680],
                // Tree 1 (28 abilities)
                vec![598,599,605,606,612,613,618,619,624,626,628,629,642,
                     644,646,647,649,651,653,654,664,665,669,670,673,674,
                     676,678],
                // Tree 2 (27 abilities)
                vec![600,601,602,607,608,609,614,615,620,630,631,655,656,
                     657,658,659,660,671,681,682,683,684,685,686,687,688,689],
            ],
        },
        2 => AbilityTreeData { // Commando
            trees: [
                // Tree 0 (28 abilities)
                vec![700,706,707,713,714,720,721,726,727,731,732,733,735,
                     736,750,751,753,755,757,765,766,770,771,774,775,778,
                     780,781],
                // Tree 1 (30 abilities)
                vec![701,702,708,709,715,716,722,723,728,729,734,737,738,
                     739,752,754,756,758,759,767,768,769,772,773,776,777,
                     779,782,783,784],
                // Tree 2 (27 abilities)
                vec![703,704,705,710,711,712,717,718,719,724,725,730,740,
                     741,760,761,762,763,764,785,786,787,788,789,790,791,792],
            ],
        },
        _ => AbilityTreeData::default(), // Other archetypes: empty trees
    }
}

/// Total XP required to reach each level (from `python/common/Constants.py`).
const LEVEL_EXP: [i32; 21] = [
    0,
    // Level 1–10
    100, 200, 300, 600, 1000, 1600, 2500, 4000, 6000, 9000,
    // Level 11–20
    14000, 18000, 25000, 40000, 60000, 90000, 120000, 180000, 250000, 400000,
];

/// Get the XP required for a given level (clamped to table bounds).
fn level_exp(level: i32) -> i32 {
    let idx = (level as usize).min(LEVEL_EXP.len() - 1);
    LEVEL_EXP[idx]
}

// ── mapLoaded multi-packet builder ───────────────────────────────────────────

/// Build and encrypt the complete `mapLoaded()` packet sequence.
///
/// Sent after Phase 5b world entry. Contains all entity method calls needed to
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
/// Build the mapLoaded entity data as a single Mercury-fragmented bundle.
///
/// Constructs ALL entity method calls (stats, appearance, inventory, etc.) into
/// one contiguous body, then uses Mercury fragmentation to split it across
/// multiple encrypted UDP packets. The client reassembles fragments into a single
/// bundle before processing, ensuring all entity data is handled atomically.
///
/// VIEWPORT + CELL + POSITION body bytes (from [`build_world_entry_phase_b_body`])
/// are prepended to this body by the caller before fragmenting, matching the
/// C++ CellApp behavior where these are in the same channel bundle.
///
/// Returns `(encrypted_packets, sequence_ids_consumed)`.
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
    tracing::info!(entity_id, body_bytes = body.len(), "mapLoaded: body assembled");
    body
}

/// Calculate how many Mercury fragment packets a body of `body_len` bytes requires.
pub fn fragment_count(body_len: usize) -> u32 {
    use cimmeria_mercury::packet::FRAGMENT_BODY_SIZE;
    if body_len <= FRAGMENT_BODY_SIZE {
        1
    } else {
        ((body_len + FRAGMENT_BODY_SIZE - 1) / FRAGMENT_BODY_SIZE) as u32
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
    build_fragmented_bundle(
        REPLY_FLAGS,
        body,
        base_seq,
        acks,
        |plaintext| encrypt_packet(plaintext, &key_copy),
    )
}

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

fn build_map_loaded_body_inner(
    body: &mut Vec<u8>,
    entity_id: u32,
    data: &PlayerLoadData,
    world_entry: &WorldEntryInfo,
    stats: &ArchetypeStats,
) {
    // Helper: append an entity method call to the body.
    macro_rules! append_method {
        ($method_idx:expr, $args:expr) => {{
            append_entity_method(body, $method_idx, entity_id, $args);
        }};
    }

    // 0. setupWorldParameters (22 args: 5×i32 + 17×f32)
    append_method!(method_idx::SETUP_WORLD_PARAMETERS, &build_world_params_args(&world_entry.world_name));

    // 1. setupStargateInfo (3×ARRAY<INT32>: world, known, hidden)
    {
        let mut args = Vec::new();
        args.extend_from_slice(&0u32.to_le_bytes()); // worldStargateIds: empty
        args.extend_from_slice(&(data.known_stargates.len() as u32).to_le_bytes());
        for &sg in &data.known_stargates {
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
    append_method!(method_idx::ON_KISMET_EVENT_SET_UPDATE, &1025i32.to_le_bytes());

    // 8. sendStats: onStatUpdate + onStatBaseUpdate
    //    Uses StatList from entity crate — sends ALL stats, matching Python's
    //    `self.sendStats(self.client, False, False)` which sends everything.
    {
        use cimmeria_entity::stats::{StatList, ArchetypeStatValues};
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
        // onStatUpdate: dynamic values (min, current, max)
        let stat_args = stat_list.serialize_all();
        append_method!(method_idx::ON_STAT_UPDATE, &stat_args);
        // onStatBaseUpdate: base values (same for fresh characters)
        let base_args = stat_list.serialize_all_base();
        append_method!(method_idx::ON_STAT_BASE_UPDATE, &base_args);
    }

    // 9. onArchetypeUpdate(INT32)
    append_method!(method_idx::ON_ARCHETYPE_UPDATE, &data.archetype.to_le_bytes());

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
        append_method!(method_idx::ON_KNOWN_ABILITIES_UPDATE, &args);
    }

    // 14. onResetMapInfo (no args)
    append_method!(method_idx::ON_RESET_MAP_INFO, &[]);

    // 15. BeingAppearance(UNICODE_STRING bodySet, ARRAY<UNICODE_STRING> components)
    {
        tracing::info!(
            bodyset = %data.bodyset,
            bodyset_len = data.bodyset.len(),
            component_count = data.components.len(),
            components = ?data.components,
            skin_color_id = data.skin_color_id,
            "mapLoaded: BeingAppearance + onEntityTint data"
        );
        let mut args = Vec::new();
        write_wstring(&mut args, &data.bodyset);
        args.extend_from_slice(&(data.components.len() as u32).to_le_bytes());
        for comp in &data.components {
            write_wstring(&mut args, comp);
        }
        append_method!(method_idx::BEING_APPEARANCE, &args);
    }

    // 16. onEntityTint(UINT32, UINT32, UINT32) — primary, secondary, skin
    {
        let skin_tint = if (data.skin_color_id as usize) < SKIN_TINTS.len() {
            SKIN_TINTS[data.skin_color_id as usize]
        } else {
            SKIN_TINTS[0]
        };
        let mut args = Vec::with_capacity(12);
        args.extend_from_slice(&0u32.to_le_bytes());        // primaryColorId (default 0, matches C++)
        args.extend_from_slice(&0u32.to_le_bytes());        // secondaryColorId (default 0, matches C++)
        args.extend_from_slice(&skin_tint.to_le_bytes());
        append_method!(method_idx::ON_ENTITY_TINT, &args);
    }

    // 17. onExtraNameUpdate(UNICODE_STRING) — extended encoding
    {
        let mut args = Vec::new();
        write_wstring(&mut args, &data.extra_name);
        append_method!(method_idx::ON_EXTRA_NAME_UPDATE, &args);
    }

    // 18. onExpUpdate(INT32) — extended encoding
    append_method!(method_idx::ON_EXP_UPDATE, &data.exp.to_le_bytes());

    // 19. onMaxExpUpdate(INT32) — extended encoding
    append_method!(method_idx::ON_MAX_EXP_UPDATE, &level_exp(data.level).to_le_bytes());

    // 20. onEntityProperty x6 (INT32 propId, INT32 value)
    //     GENERICPROPERTY IDs from Atrea.enums
    for &(prop_id, value) in &[
        (2i32, data.applied_science_points), // AppliedSciencePoints
        (1,    data.training_points),        // TrainingPoints
        (7,    data.access_level),           // AccessLevel
        (8,    data.gender),                 // Gender
        (4,    0),                           // PvPFlag
        (3,    0),                           // AmmoTypeId
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

    // 24. onChatJoined — notify client about default channels
    //     Reference: python/base/SGWPlayer.py onClientReady → ChannelManager.playerLoggedIn
    for &(channel_name, channel_id) in &[
        ("say", 0u8), ("emote", 1), ("yell", 2), ("team", 3),
        ("squad", 4), ("command", 5), ("server", 7), ("tell", 9),
    ] {
        let mut args = Vec::new();
        write_wstring(&mut args, channel_name);
        args.push(channel_id);
        append_method!(method_idx::ON_CHAT_JOINED, &args);
    }

    // 25. onPlayMovie — fullscreen SGW logo cinematic on first-ever login
    //     Reference: python/cell/SGWPlayer.py:535-537
    if data.first_login != 0 {
        let mut args = Vec::new();
        write_wstring(&mut args, "Cine-SGWLogo.SGWLogo");
        args.push(1u8); // FullScreen = true
        append_method!(method_idx::ON_PLAY_MOVIE, &args);
    }

    // 26. onPlayerDataLoaded (no args) — client transitions to gameplay
    append_method!(method_idx::ON_PLAYER_DATA_LOADED, &[]);

    // 26. onTargetUpdate(INT32) — default 0 (no target)
    append_method!(method_idx::ON_TARGET_UPDATE, &0i32.to_le_bytes());

    // 27. onPlayerCommunication(WSTRING speaker, UINT8 flags, UINT8 channel, WSTRING text)
    //     Welcome message on the feedback channel.
    {
        let mut args = Vec::new();
        write_wstring(&mut args, &data.player_name); // Speaker
        args.push(0u8); // SpeakerFlags
        args.push(9u8); // Channel = CHAN_TELL (matches C++ SGWPlayer.py:541)
        let welcome = format!(
            "Welcome to Stargate Worlds. Your player id is: {}.",
            entity_id
        );
        write_wstring(&mut args, &welcome); // Text
        append_method!(method_idx::ON_PLAYER_COMMUNICATION, &args);
    }

}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_mercury::encryption::MercuryEncryption;

    const TEST_KEY: [u8; 32] = [0x42u8; 32];

    #[test]
    fn build_on_player_data_loaded_uses_correct_msg_id() {
        let out = build_on_player_data_loaded(&TEST_KEY, 1, &[], 42);
        let enc = MercuryEncryption::from_session_key(TEST_KEY);
        let pt = enc.decrypt(&out).unwrap();
        // body starts at offset 1, method_index=115 >= 61, so extended: 0xBD
        assert_eq!(pt[1], 0xBD);
    }

    #[test]
    fn build_setup_world_parameters_uses_correct_msg_id() {
        let out = build_setup_world_parameters(&TEST_KEY, 1, &[], 42);
        let enc = MercuryEncryption::from_session_key(TEST_KEY);
        let pt = enc.decrypt(&out).unwrap();
        // method_index=122 >= 61, so extended: 0xBD
        assert_eq!(pt[1], 0xBD);
    }

    #[test]
    fn build_map_loaded_produces_multiple_packets() {
        let data = PlayerLoadData {
            player_id: 1,
            level: 1,
            player_name: "Test".into(),
            extra_name: String::new(),
            alignment: 1,
            archetype: 1,
            gender: 1,
            bodyset: "BS_HumanMale.BS_HumanMale".into(),
            components: vec!["head_test".into()],
            exp: 0,
            naquadah: 0,
            known_stargates: vec![],
            abilities: vec![],
            training_points: 0,
            applied_science_points: 0,
            blueprint_ids: vec![],
            first_login: 1,
            access_level: 0,
            skin_color_id: 0,
            ability_tree: Default::default(),
            items: vec![],
        };
        let entry = WorldEntryInfo { player_entity_id: 42, space_id: 65552, pos: [0.0; 3], rot: [0.0; 3], world_name: "CombatSim".into() };
        let (packets, seqs) = build_map_loaded(&TEST_KEY, 5, &[], 42, &data, &entry);
        assert!(!packets.is_empty(), "mapLoaded should produce at least one packet");
        assert_eq!(seqs as usize, packets.len(), "seqs_consumed should match packet count");
        for (i, pkt) in packets.iter().enumerate() {
            assert!(!pkt.is_empty(), "packet {} should not be empty", i);
        }
    }

    #[test]
    fn build_map_loaded_each_packet_decrypts_within_limit() {
        let data = PlayerLoadData {
            player_id: 1,
            level: 5,
            player_name: "Warrior".into(),
            extra_name: "The Brave".into(),
            alignment: 2,
            archetype: 2,
            gender: 1,
            bodyset: "BS_HumanMale.BS_HumanMale".into(),
            components: vec![],
            exp: 500,
            naquadah: 100,
            known_stargates: vec![1, 2],
            abilities: vec![10, 20],
            training_points: 3,
            applied_science_points: 1,
            blueprint_ids: vec![],
            first_login: 0,
            access_level: 0,
            skin_color_id: 5,
            ability_tree: archetype_ability_tree(2),
            items: vec![],
        };
        let entry = WorldEntryInfo { player_entity_id: 100, space_id: 65552, pos: [0.0; 3], rot: [0.0; 3], world_name: "CombatSim".into() };
        let (packets, _seqs) = build_map_loaded(&TEST_KEY, 5, &[], 100, &data, &entry);
        let enc = MercuryEncryption::from_session_key(TEST_KEY);
        // Mercury MAX_BODY_LENGTH is 1411 bytes
        const MAX_PLAINTEXT: usize = 1411;
        for (i, pkt) in packets.iter().enumerate() {
            let pt = enc.decrypt(pkt).unwrap();
            assert!(pt.len() <= MAX_PLAINTEXT,
                "packet {} plaintext {} bytes exceeds {} limit", i, pt.len(), MAX_PLAINTEXT);
        }
    }

    #[test]
    fn build_map_loaded_contains_setup_world_params_and_player_data_loaded() {
        let data = PlayerLoadData {
            player_id: 1,
            level: 5,
            player_name: "Warrior".into(),
            extra_name: "The Brave".into(),
            alignment: 2,
            archetype: 2,
            gender: 1,
            bodyset: "BS_HumanMale.BS_HumanMale".into(),
            components: vec![],
            exp: 500,
            naquadah: 100,
            known_stargates: vec![1, 2],
            abilities: vec![10, 20],
            training_points: 3,
            applied_science_points: 1,
            blueprint_ids: vec![],
            first_login: 0,
            access_level: 0,
            skin_color_id: 5,
            ability_tree: archetype_ability_tree(2),
            items: vec![],
        };
        let entry = WorldEntryInfo { player_entity_id: 100, space_id: 65552, pos: [0.0; 3], rot: [0.0; 3], world_name: "CombatSim".into() };
        let (packets, _seqs) = build_map_loaded(&TEST_KEY, 5, &[], 100, &data, &entry);
        let enc = MercuryEncryption::from_session_key(TEST_KEY);

        // Collect all decrypted plaintext across all packets
        let mut all_bytes = Vec::new();
        for pkt in &packets {
            let pt = enc.decrypt(pkt).unwrap();
            // Include everything after the flags byte (body + footers).
            // We're just checking for presence of specific marker bytes.
            all_bytes.extend_from_slice(&pt[1..]);
        }
        // setupWorldParameters = 0xFA should be present
        assert!(all_bytes.contains(&0xFA), "should contain setupWorldParameters (0xFA)");
        // onPlayerDataLoaded = 0xF3 should be present
        assert!(all_bytes.contains(&0xF3), "should contain onPlayerDataLoaded (0xF3)");
        // onAbilityTreeInfo uses extended 0xBD
        assert!(all_bytes.contains(&0xBD), "should contain extended encoding marker (0xBD)");
    }

    #[test]
    fn build_map_loaded_uses_mercury_fragmentation() {
        use cimmeria_mercury::packet::{FLAG_FRAGMENTED, FLAG_HAS_SEQUENCE};

        let data = PlayerLoadData {
            player_id: 1,
            level: 5,
            player_name: "Warrior".into(),
            extra_name: "The Brave".into(),
            alignment: 2,
            archetype: 2,
            gender: 1,
            bodyset: "BS_HumanMale.BS_HumanMale".into(),
            components: vec!["head_test".into(), "torso_test".into()],
            exp: 500,
            naquadah: 100,
            known_stargates: vec![1, 2],
            abilities: vec![10, 20, 30],
            training_points: 3,
            applied_science_points: 1,
            blueprint_ids: vec![],
            first_login: 0,
            access_level: 0,
            skin_color_id: 5,
            ability_tree: archetype_ability_tree(2),
            items: vec![],
        };
        let entry = WorldEntryInfo {
            player_entity_id: 100, space_id: 65552,
            pos: [0.0; 3], rot: [0.0; 3], world_name: "CombatSim".into(),
        };
        let (packets, seqs) = build_map_loaded(&TEST_KEY, 10, &[], 100, &data, &entry);
        let enc = MercuryEncryption::from_session_key(TEST_KEY);

        // Must produce multiple packets (body > 1300 bytes)
        assert!(packets.len() > 1,
            "Expected multiple fragments, got {} packet(s)", packets.len());
        assert_eq!(seqs as usize, packets.len());

        for (i, pkt) in packets.iter().enumerate() {
            let pt = enc.decrypt(pkt).unwrap();
            let flags = pt[0];

            // Every fragment must have FLAG_FRAGMENTED (0x20) and FLAG_HAS_SEQUENCE (0x40)
            assert!(flags & FLAG_FRAGMENTED != 0,
                "Packet {} flags=0x{:02X} missing FLAG_FRAGMENTED (0x20)", i, flags);
            assert!(flags & FLAG_HAS_SEQUENCE != 0,
                "Packet {} flags=0x{:02X} missing FLAG_HAS_SEQUENCE (0x40)", i, flags);

            // Parse footers from the end: seq_id (4), frag_end (4), frag_begin (4)
            let len = pt.len();
            let seq_id = u32::from_le_bytes(pt[len-4..len].try_into().unwrap());
            let frag_end = u32::from_le_bytes(pt[len-8..len-4].try_into().unwrap());
            let frag_begin = u32::from_le_bytes(pt[len-12..len-8].try_into().unwrap());

            // frag_begin should be base_seq (10)
            assert_eq!(frag_begin, 10,
                "Packet {} frag_begin={} expected 10", i, frag_begin);
            // frag_end should be base_seq + num_frags - 1
            let expected_frag_end = 10 + packets.len() as u32 - 1;
            assert_eq!(frag_end, expected_frag_end,
                "Packet {} frag_end={} expected {}", i, frag_end, expected_frag_end);
            // seq_id should be base_seq + i
            assert_eq!(seq_id, 10 + i as u32,
                "Packet {} seq_id={} expected {}", i, seq_id, 10 + i as u32);

            // Body chunk size: for non-last fragments, should be FRAGMENT_BODY_SIZE (1300)
            let body_len = len - 1 - 12; // flags(1) + footers(12) subtracted
            if i < packets.len() - 1 {
                assert_eq!(body_len, 1300,
                    "Packet {} body_len={} expected 1300 (FRAGMENT_BODY_SIZE)", i, body_len);
            }

            eprintln!("Fragment {}: plaintext_len={} flags=0x{:02X} body={} seq={} frag=[{}-{}]",
                i, pt.len(), flags, body_len, seq_id, frag_begin, frag_end);
        }
    }

    #[test]
    fn level_exp_boundaries() {
        assert_eq!(level_exp(0), 0);
        assert_eq!(level_exp(1), 100);
        assert_eq!(level_exp(10), 9000);
        assert_eq!(level_exp(20), 400000);
        assert_eq!(level_exp(99), 400000); // clamped
    }

    #[test]
    fn archetype_stats_commando_differs() {
        let soldier = archetype_stats(1);
        let commando = archetype_stats(2);
        assert_eq!(soldier.coordination, 5);
        assert_eq!(commando.coordination, 4);
        assert_eq!(commando.perception, 5);
    }

    /// Verify BeingAppearance wire encoding matches C++ reference:
    ///   msg_id=0x9A (index 26, direct: 26|0x80)
    ///   word_len=u16 LE (entity_id + bodyset WSTRING + array WSTRING)
    ///   entity_id=u32 LE
    ///   bodyset: [u32 char_count][UTF-16LE data]
    ///   components: [u32 element_count][WSTRING element]*
    #[test]
    fn being_appearance_wire_encoding() {
        use super::*;

        let entity_id: u32 = 42;
        let bodyset = "BS_HumanMale.BS_HumanMale";
        let components = vec![
            "BS_HumanMale.BS_HM_Head_00".to_string(),
            "BS_HumanMale.BS_HM_Torso_00".to_string(),
            "BS_HumanMale.BS_HM_Legs_00".to_string(),
            "AR_Global.Prisoner_Torso".to_string(),
        ];

        let mut args = Vec::new();
        write_wstring(&mut args, bodyset);
        args.extend_from_slice(&(components.len() as u32).to_le_bytes());
        for comp in &components {
            write_wstring(&mut args, comp);
        }

        let mut body = Vec::new();
        append_entity_method(&mut body, method_idx::BEING_APPEARANCE, entity_id, &args);

        // method_index=26 < 61 → direct: msg_id = 26 | 0x80 = 0x9A
        assert_eq!(body[0], 0x9A, "msg_id should be 0x9A for BeingAppearance (index 26)");

        // word_len (u16 LE) at offset 1-2
        let word_len = u16::from_le_bytes([body[1], body[2]]);
        let expected_payload = 4 + args.len(); // entity_id + args
        assert_eq!(word_len as usize, expected_payload, "word_len mismatch");

        // entity_id (u32 LE) at offset 3-6
        let eid = u32::from_le_bytes([body[3], body[4], body[5], body[6]]);
        assert_eq!(eid, entity_id, "entity_id mismatch");

        // bodyset WSTRING at offset 7
        let off = 7;
        let bs_char_count = u32::from_le_bytes([body[off], body[off+1], body[off+2], body[off+3]]);
        assert_eq!(bs_char_count as usize, bodyset.len(), "bodyset char_count should match");

        // Verify bodyset UTF-16LE chars
        let bs_data_start = off + 4;
        for (i, ch) in bodyset.encode_utf16().enumerate() {
            let wire_ch = u16::from_le_bytes([
                body[bs_data_start + i * 2],
                body[bs_data_start + i * 2 + 1],
            ]);
            assert_eq!(wire_ch, ch, "bodyset char {i} mismatch");
        }

        // Component array count
        let comp_off = bs_data_start + bodyset.len() * 2;
        let comp_count = u32::from_le_bytes([
            body[comp_off], body[comp_off+1], body[comp_off+2], body[comp_off+3],
        ]);
        assert_eq!(comp_count, components.len() as u32, "component count mismatch");

        // Verify each component WSTRING
        let mut cursor = comp_off + 4;
        for (idx, comp) in components.iter().enumerate() {
            let cc = u32::from_le_bytes([
                body[cursor], body[cursor+1], body[cursor+2], body[cursor+3],
            ]);
            assert_eq!(cc as usize, comp.len(), "component {idx} char_count mismatch");
            cursor += 4;
            for (i, ch) in comp.encode_utf16().enumerate() {
                let wire_ch = u16::from_le_bytes([
                    body[cursor + i * 2],
                    body[cursor + i * 2 + 1],
                ]);
                assert_eq!(wire_ch, ch, "component {idx} char {i} mismatch");
            }
            cursor += comp.len() * 2;
        }

        // Verify total length
        assert_eq!(cursor, body.len(), "total body length should match parsed position");
    }

    /// Verify onEntityTint wire encoding: 3x u32 LE
    #[test]
    fn entity_tint_wire_encoding() {
        use super::*;

        let entity_id: u32 = 42;
        let primary: u32 = 0;
        let secondary: u32 = 0;
        let skin: u32 = SKIN_TINTS[5]; // skin_color_id = 5

        let mut args = Vec::with_capacity(12);
        args.extend_from_slice(&primary.to_le_bytes());
        args.extend_from_slice(&secondary.to_le_bytes());
        args.extend_from_slice(&skin.to_le_bytes());

        let mut body = Vec::new();
        append_entity_method(&mut body, method_idx::ON_ENTITY_TINT, entity_id, &args);

        // method_index=10 < 61 → direct: msg_id = 10 | 0x80 = 0x8A
        assert_eq!(body[0], 0x8A, "msg_id should be 0x8A for onEntityTint (index 10)");

        let word_len = u16::from_le_bytes([body[1], body[2]]);
        assert_eq!(word_len, 16, "word_len should be 4 (entity_id) + 12 (3x u32)");

        let eid = u32::from_le_bytes([body[3], body[4], body[5], body[6]]);
        assert_eq!(eid, entity_id);

        let p = u32::from_le_bytes([body[7], body[8], body[9], body[10]]);
        assert_eq!(p, 0, "primaryColorId should be 0");

        let s = u32::from_le_bytes([body[11], body[12], body[13], body[14]]);
        assert_eq!(s, 0, "secondaryColorId should be 0");

        let sk = u32::from_le_bytes([body[15], body[16], body[17], body[18]]);
        assert_eq!(sk, SKIN_TINTS[5], "skinTint should match SKIN_TINTS[5]");
    }
}
