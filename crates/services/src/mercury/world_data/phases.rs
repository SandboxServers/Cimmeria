//! World entry builders: create player, enter world, and standalone entity method packets.

use cimmeria_mercury::packet::{build_outgoing, FLAG_HAS_ACKS};

use super::{
    append_entity_method, build_world_params_args, client_map_for_world, encrypt_packet,
    method_idx, world_id_for_name, write_wstring, PlayerLoadData, WorldEntryInfo,
    BASEMSG_CREATE_BASE_PLAYER, BASEMSG_CREATE_CELL_PLAYER, BASEMSG_FORCED_POSITION,
    BASEMSG_SPACE_VIEWPORT_INFO, REPLY_FLAGS, SKIN_TINTS,
};

// ── World entry builders ─────────────────────────────────────────────────────

/// Create player step: CREATE_BASE_PLAYER + (optional appearance pre-warm) + onClientMapLoad.
///
/// Sends the base entity creation and terrain load notification. When
/// `load` is provided, also appends `BeingAppearance` + `onEntityTint`
/// between the entity create and the terrain-load trigger — this kicks the
/// client into loading the player's body-model assets in parallel with the
/// terrain load instead of waiting until after `mapLoaded` returns. Without
/// this pre-warm, the client briefly renders the dev-cube placeholder
/// (the default visual for an entity with no bodyset) between
/// `createCellPlayer` and the eventual `BeingAppearance` arrival.
///
/// The client will load terrain geometry and respond with `mapLoaded` (cell
/// method index 25, msg_id 0x99). Only after receiving that should the server
/// send the enter-world packet (viewport + cell player + forced position + entity data).
///
/// In C++, BaseApp's `enableEntities()` sends CREATE_BASE_PLAYER then triggers
/// CellApp `sendConnectEntity`. CellApp's `connected()` callback sends
/// `onClientMapLoad`. The client loads terrain, sends `mapLoaded`, and *then*
/// CellApp responds with viewport + cell + position + the full setup sequence.
pub fn build_create_player(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    info: &WorldEntryInfo,
    load: Option<&PlayerLoadData>,
) -> Vec<u8> {
    let mut body = Vec::with_capacity(256);

    // 1. CREATE_BASE_PLAYER (WORD_LENGTH = 6)
    // class_id: SGWPlayer (0x02) or SGWGmPlayer (0x03) based on access_level.
    body.push(BASEMSG_CREATE_BASE_PLAYER);
    body.extend_from_slice(&6u16.to_le_bytes());
    body.extend_from_slice(&info.player_entity_id.to_le_bytes());
    body.push(info.class_id);
    body.push(0x00); // propertyCount = 0

    // 1b. Pre-warm body model + tint so the client's async asset load runs
    //     in parallel with the terrain load triggered by onClientMapLoad.
    //     The entity already exists from CREATE_BASE_PLAYER above; these
    //     entity-method calls just configure its visual properties. Without
    //     this, the entity arrives via `createCellPlayer` with no bodyset
    //     and the renderer shows the dev-cube placeholder until
    //     `BeingAppearance` lands later in the mapLoaded body.
    if let Some(load) = load {
        // Spawn weapon-holstered — `appearance_components(true)` drops the
        // active bandolier weapon from the ComponentList so the client's
        // appearance compositor (`ghidra://SGW.exe@0x00ec0840`) selects
        // the unarmed-stance animation pose.
        let wire_components = load.appearance_components(true);
        let mut appearance_args = Vec::new();
        write_wstring(&mut appearance_args, &load.bodyset);
        appearance_args.extend_from_slice(&(wire_components.len() as u32).to_le_bytes());
        for comp in &wire_components {
            write_wstring(&mut appearance_args, comp);
        }
        append_entity_method(
            &mut body,
            method_idx::BEING_APPEARANCE,
            info.player_entity_id,
            &appearance_args,
        );

        let skin_tint = if (load.skin_color_id as usize) < SKIN_TINTS.len() {
            SKIN_TINTS[load.skin_color_id as usize]
        } else {
            SKIN_TINTS[0]
        };
        let mut tint_args = Vec::with_capacity(12);
        tint_args.extend_from_slice(&0u32.to_le_bytes()); // primaryColorId
        tint_args.extend_from_slice(&0u32.to_le_bytes()); // secondaryColorId
        tint_args.extend_from_slice(&skin_tint.to_le_bytes());
        append_entity_method(
            &mut body,
            method_idx::ON_ENTITY_TINT,
            info.player_entity_id,
            &tint_args,
        );
    }

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
        append_entity_method(
            &mut body,
            method_idx::ON_CLIENT_MAP_LOAD,
            info.player_entity_id,
            &args,
        );
    }

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build raw body bytes for the enter-world step:
/// VIEWPORT + (optional BeingAppearance + onEntityTint) + CELL_PLAYER + FORCED_POSITION.
///
/// Returns just the Mercury message bytes without packet framing.
/// The C++ server adds these messages to `channel_->bundle()` — the same bundle
/// that contains the mapLoaded entity methods — rather than sending them as a
/// separate packet. This function provides the raw bytes so the caller can
/// prepend them to the mapLoaded body before fragmenting into a single bundle.
///
/// When `load` is provided, `BeingAppearance` and `onEntityTint` are emitted
/// **between** `spaceViewportInfo` and `createCellPlayer`. The client's
/// `createCellPlayer` handler calls into the appearance gate internally — by
/// having the bodyset already applied to the entity at that moment, the gate's
/// model-asset-load job is queued against the just-completed bodyset, so the
/// renderable entity comes up with the correct model on its first render frame
/// instead of flashing the dev-cube placeholder. Live-debug evidence:
/// `logs/x64dbg-issue288-run3-analysis.txt`.
pub fn build_enter_world_body(info: &WorldEntryInfo, load: Option<&PlayerLoadData>) -> Vec<u8> {
    let mut body = Vec::with_capacity(128);

    // 1. spaceViewportInfo (0x08, CONSTANT_LENGTH = 13)
    body.push(BASEMSG_SPACE_VIEWPORT_INFO);
    body.extend_from_slice(&info.player_entity_id.to_le_bytes());
    body.extend_from_slice(&info.player_entity_id.to_le_bytes());
    body.extend_from_slice(&info.space_id.to_le_bytes());
    body.push(0x00); // viewportID = 0

    // 1b. BeingAppearance + onEntityTint — must precede createCellPlayer so
    //     the client's cell-entity creation handler picks up the freshly-set
    //     bodyset during its internal appearance evaluation. Without this,
    //     the renderable entity is created against the default cube
    //     placeholder and the asset bind only completes on a later render
    //     frame.
    if let Some(load) = load {
        // Spawn weapon-holstered — `appearance_components(true)` drops the
        // active bandolier weapon from the ComponentList so the client's
        // appearance compositor (`ghidra://SGW.exe@0x00ec0840`) selects
        // the unarmed-stance animation pose.
        let wire_components = load.appearance_components(true);
        let mut appearance_args = Vec::new();
        write_wstring(&mut appearance_args, &load.bodyset);
        appearance_args.extend_from_slice(&(wire_components.len() as u32).to_le_bytes());
        for comp in &wire_components {
            write_wstring(&mut appearance_args, comp);
        }
        append_entity_method(
            &mut body,
            method_idx::BEING_APPEARANCE,
            info.player_entity_id,
            &appearance_args,
        );

        let skin_tint = if (load.skin_color_id as usize) < SKIN_TINTS.len() {
            SKIN_TINTS[load.skin_color_id as usize]
        } else {
            SKIN_TINTS[0]
        };
        let mut tint_args = Vec::with_capacity(12);
        tint_args.extend_from_slice(&0u32.to_le_bytes()); // primaryColorId
        tint_args.extend_from_slice(&0u32.to_le_bytes()); // secondaryColorId
        tint_args.extend_from_slice(&skin_tint.to_le_bytes());
        append_entity_method(
            &mut body,
            method_idx::ON_ENTITY_TINT,
            info.player_entity_id,
            &tint_args,
        );
    }

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
    // Previous-position reference vector (NOT velocity). Client's
    // `ProcessForcedEntityPosition` (Ghidra 0x00dd9ee0) copies these 12 bytes
    // verbatim into `pPrevPos`; the camera-attach path then interpolates from
    // pPrevPos to current_pos on the first frame. Emitting zeros at world entry
    // makes the camera slide from world origin (0,0,0) through the floor to the
    // spawn point — visible as a "camera-through-floor" glitch. Set the prev-pos
    // reference equal to the spawn position so the interpolation distance is
    // zero. See spec.protocol.mercury §1.10.6 + §1.16 Q3 closure for the
    // Ghidra-confirmed semantic; canonical layout in
    // spec.protocol.position-updates §1.4.2.
    let prev_pos = info.pos;
    for &c in &prev_pos {
        body.extend_from_slice(&c.to_le_bytes());
    }
    // C++ sends rotX, rotZ, rotY (swapped)
    body.extend_from_slice(&info.rot[0].to_le_bytes()); // rotX
    body.extend_from_slice(&info.rot[2].to_le_bytes()); // rotZ (swapped)
    body.extend_from_slice(&info.rot[1].to_le_bytes()); // rotY (swapped)
    body.push(0x01); // flags

    body
}

/// Enter world step: VIEWPORT + CELL_PLAYER + FORCED_POSITION as a standalone packet.
///
/// Wraps [`build_enter_world_body`] in packet framing and encryption.
/// Kept for tests/reference; the live path now embeds the body into the
/// mapLoaded fragmented bundle instead.
pub fn build_enter_world(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    info: &WorldEntryInfo,
    load: Option<&PlayerLoadData>,
) -> Vec<u8> {
    let body = build_enter_world_body(info, load);
    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Cell->client entity method: `onPlayerDataLoaded` (no args).
///
/// Flattened ClientMethods index 115 -> msg_id = 0xF3.
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

/// Cell->client entity method: `setupWorldParameters` (22 args: 5xi32 + 17xf32).
///
/// Flattened ClientMethods index 122 -> extended encoding (0xBD + sub_index 61).
/// Sets world physics constants (gravity, movement speeds, etc.).
pub fn build_setup_world_parameters(
    key: &[u8; 32],
    seq: u32,
    acks: &[u32],
    entity_id: u32,
) -> Vec<u8> {
    let mut body = Vec::new();
    let args = build_world_params_args("CombatSim");
    append_entity_method(
        &mut body,
        method_idx::SETUP_WORLD_PARAMETERS,
        entity_id,
        &args,
    );

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq), acks, None);
    encrypt_packet(&plaintext, key)
}
