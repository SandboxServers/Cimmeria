//! AoI (Area of Interest) packet builders: entity creation, leave, avatar updates,
//! and entity method packets for ghost entities.

use cimmeria_mercury::packet::{build_outgoing, FLAG_HAS_ACKS};

use super::{
    append_entity_method, encrypt_packet, method_idx, write_wstring, BASEMSG_FORCED_POSITION,
    REPLY_FLAGS,
};
use crate::cell::messages::NpcAoIData;

/// `GENERICPROPERTY_DatabaseId` — maps to speaker_id for dialog-capable entities.
const GENERICPROPERTY_DATABASE_ID: i32 = 9;

/// `BASEMSG_CREATE_ENTITY` — create a ghost (non-player) entity on the client (0x09).
/// Sent when an entity enters a player's Area of Interest.
/// Wire: `[msg_id:0x09][wordLen:u16=8][entityId:u32][idAlias:0xFF][classId:u8][0x00][0x00]`
pub(crate) const BASEMSG_CREATE_ENTITY: u8 = 0x09;
/// `BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL` — position update
/// for ghost entities (0x10, CONSTANT_LENGTH = 25).
pub(crate) const BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR: u8 = 0x10;
/// `BASEMSG_ENTITY_INVISIBLE` — mark entity invisible before removal (0x0B, CONSTANT_LENGTH = 5).
pub(crate) const BASEMSG_ENTITY_INVISIBLE: u8 = 0x0B;
/// `BASEMSG_LEAVE_AOI` — remove entity from client's AoI (0x0C, WORD_LENGTH).
pub(crate) const BASEMSG_LEAVE_AOI: u8 = 0x0C;

/// Build and encrypt `CREATE_ENTITY (0x09)` + `UPDATE_AVATAR (0x10)` — phase 1.
///
/// In the C++ server, CREATE_ENTITY + UPDATE_AVATAR are sent by the BaseApp
/// immediately (`cached_entity.cpp:199`), while the property cascade arrives
/// later from the CellApp after a round trip (`base_client.cpp:448`).
/// Splitting into separate packets matches that timing so the client creates
/// the entity object before entity methods try to configure it.
pub fn build_create_entity_base(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    entity_id: u32,
    class_id: u8,
    position: [f32; 3],
    direction: [f32; 3],
) -> Vec<u8> {
    let mut body = Vec::with_capacity(48);

    // CREATE_ENTITY (0x09, WORD_LENGTH)
    body.push(BASEMSG_CREATE_ENTITY);
    body.extend_from_slice(&8u16.to_le_bytes()); // wordLength = 8
    body.extend_from_slice(&entity_id.to_le_bytes());
    body.push(0xFF); // idAlias = no alias
    body.push(class_id);
    body.push(0x00); // unknown1
    body.push(0x00); // unknown2

    // UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL (0x10, CONSTANT_LENGTH = 25)
    body.push(BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR);
    body.extend_from_slice(&entity_id.to_le_bytes());
    for &c in &position {
        body.extend_from_slice(&c.to_le_bytes());
    }
    body.extend_from_slice(&[0u8; 5]); // velocity = zero
    body.push(0x01); // physics mode
    body.push(pack_angle(direction[1])); // yaw
    body.push(pack_angle(direction[0])); // pitch
    body.push(pack_angle(direction[2])); // roll

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt the `createOnClient()` property cascade — phase 2.
///
/// Sent in a separate packet after [`build_create_entity_base`] so the client
/// has processed CREATE_ENTITY first. Mirrors the CellApp's `createOnClient()`
/// then `SGWBeing.createOnClient()` Python cascade that arrives after the
/// BaseApp→CellApp `sendRequestEntityUpdate` round trip.
pub fn build_create_entity_cascade(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    entity_id: u32,
    class_id: u8,
    level: u32,
    npc_data: Option<&NpcAoIData>,
) -> Vec<u8> {
    let mut body = Vec::with_capacity(128);

    // Per-entity values from template data (or defaults for players)
    let entity_flags = npc_data.map_or(0u64, |d| d.entity_flags);
    let align = npc_data.map_or(0u8, |d| d.alignment);
    let fac = npc_data.map_or(0u8, |d| d.faction);

    // ── SGWSpawnableEntity.createOnClient ──

    // 1. onEntityProperty(GENERICPROPERTY_DatabaseId, speakerId)
    if let Some(d) = npc_data {
        if let Some(speaker_id) = d.speaker_id {
            let mut args = Vec::with_capacity(8);
            args.extend_from_slice(&GENERICPROPERTY_DATABASE_ID.to_le_bytes());
            args.extend_from_slice(&speaker_id.to_le_bytes());
            append_entity_method(&mut body, method_idx::ON_ENTITY_PROPERTY, entity_id, &args);
        }
    }

    // 2. onKismetEventSetUpdate(eventSetId)
    if let Some(d) = npc_data {
        if let Some(event_set_id) = d.event_set_id {
            if event_set_id != 0 {
                append_entity_method(
                    &mut body,
                    method_idx::ON_KISMET_EVENT_SET_UPDATE,
                    entity_id,
                    &event_set_id.to_le_bytes(),
                );
            }
        }
    }

    // 3. createAppearanceOnClient — BeingAppearance (humanoid) OR onStaticMeshNameUpdate (prop)
    if let Some(d) = npc_data {
        append_appearance(&mut body, entity_id, d);
    }

    // 4. InteractionType(interactionType) — base flags (dynamic merged flags sent separately)
    if let Some(d) = npc_data {
        append_entity_method(
            &mut body,
            method_idx::INTERACTION_TYPE,
            entity_id,
            &(d.interaction_type as u64).to_le_bytes(),
        );
    }

    // 5. onBeingNameIDUpdate(nameId)
    if let Some(d) = npc_data {
        if let Some(name_id) = d.name_id {
            if name_id != 0 {
                append_entity_method(
                    &mut body,
                    method_idx::ON_BEING_NAME_ID_UPDATE,
                    entity_id,
                    &name_id.to_le_bytes(),
                );
            }
        }
    }

    // 6. onEntityFlags
    append_entity_method(
        &mut body,
        method_idx::ON_ENTITY_FLAGS,
        entity_id,
        &entity_flags.to_le_bytes(),
    );

    // 7. onVisible(1) — CRITICAL: registers entity with the client's viewport
    append_entity_method(&mut body, method_idx::ON_VISIBLE, entity_id, &[1u8]);

    // ── SGWBeing.createOnClient ──
    if class_id != 0x00 {
        // 8. onLevelUpdate(level)
        append_entity_method(
            &mut body,
            method_idx::ON_LEVEL_UPDATE,
            entity_id,
            &(level as i32).to_le_bytes(),
        );
        // 9. onTargetUpdate(0) — no current target
        // C++ sends this; missing it may leave the entity partially uninitialized.
        append_entity_method(
            &mut body,
            method_idx::ON_TARGET_UPDATE,
            entity_id,
            &0i32.to_le_bytes(),
        );
        // 10. onAlignmentUpdate
        append_entity_method(
            &mut body,
            method_idx::ON_ALIGNMENT_UPDATE,
            entity_id,
            &[align],
        );
        // 11. onFactionUpdate
        append_entity_method(&mut body, method_idx::ON_FACTION_UPDATE, entity_id, &[fac]);
        // 12. onStateFieldUpdate(0) — alive state
        append_entity_method(
            &mut body,
            method_idx::ON_STATE_FIELD_UPDATE,
            entity_id,
            &0u32.to_le_bytes(),
        );

        // 13-14. onStatBaseUpdate + onStatUpdate — NPC stat data
        // C++ sends 180 bytes each (4-byte count + 11×16-byte stats = 180).
        // Without populated stats, the client doesn't consider the entity
        // "ready" for interaction (right-click blocked).
        let stat_data = build_default_npc_stats();
        append_entity_method(
            &mut body,
            method_idx::ON_STAT_BASE_UPDATE,
            entity_id,
            &stat_data,
        );
        append_entity_method(&mut body, method_idx::ON_STAT_UPDATE, entity_id, &stat_data);
    }

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt `ENTITY_INVISIBLE (0x0B)` *alone* — temporary visual
/// hide that keeps the entity in the client's AoI bookkeeping. Used for the
/// ring-transport teleport-out fade. Matches C++ `ClientHandler::leaveAoI(id,
/// deleteEntity=false)` from `client_handler.cpp:516-528`.
///
/// To re-show, send `onVisible(1)` (entity method index 8 with arg 0x01) —
/// see `client_handler.cpp::enterAoI`.
pub fn build_entity_invisible(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    entity_id: u32,
) -> Vec<u8> {
    let mut body = Vec::with_capacity(8);
    body.push(BASEMSG_ENTITY_INVISIBLE);
    body.extend_from_slice(&entity_id.to_le_bytes());
    body.push(0xFF); // idAlias = no alias

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt `ENTITY_INVISIBLE (0x0B)` + `LEAVE_AOI (0x0C)` for when
/// an entity leaves a witness's Area of Interest.
///
/// Matches C++ `ClientHandler::leaveAoI(id, deleteEntity=true)` from
/// `client_handler.cpp:516-539`.
pub fn build_entity_leave(key: &[u8; 32], seq_id: u32, acks: &[u32], entity_id: u32) -> Vec<u8> {
    let mut body = Vec::with_capacity(24);

    // ENTITY_INVISIBLE (0x0B, CONSTANT_LENGTH = 5)
    body.push(BASEMSG_ENTITY_INVISIBLE);
    body.extend_from_slice(&entity_id.to_le_bytes());
    body.push(0xFF); // idAlias = no alias

    // LEAVE_AOI (0x0C, WORD_LENGTH)
    body.push(BASEMSG_LEAVE_AOI);
    body.extend_from_slice(&8u16.to_le_bytes()); // wordLength = 8
    body.extend_from_slice(&entity_id.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes()); // cacheStamp = 0

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt `UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL (0x10)` for
/// relaying position updates to AoI witnesses.
///
/// Matches C++ `ClientHandler::moveEntity()` from `client_handler.cpp:542-556`.
pub fn build_avatar_update(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    entity_id: u32,
    position: [f32; 3],
    velocity: [f32; 3],
    direction: [f32; 3],
) -> Vec<u8> {
    let mut body = Vec::with_capacity(32);

    body.push(BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR);
    body.extend_from_slice(&entity_id.to_le_bytes());
    for &c in &position {
        body.extend_from_slice(&c.to_le_bytes());
    }
    // Pack velocity using the C++ packXYZ algorithm
    let packed = pack_velocity_xyz(velocity);
    body.extend_from_slice(&packed);
    // Physics mode flags = 0x01
    body.push(0x01);
    // Direction: yaw, pitch, roll
    body.push(pack_angle(direction[1])); // yaw
    body.push(pack_angle(direction[0])); // pitch
    body.push(pack_angle(direction[2])); // roll

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build `FORCED_POSITION (0x31)` — authoritative position snap for the
/// player's own avatar.
///
/// This is the engine-level message that `BigWorld::ClientHandler` consumes
/// before user code; it bypasses prediction/interpolation and snaps the pawn
/// to `position`. `onPlayerTeleport` (method 116) only flags a streaming-load
/// waiting state — it does not move the avatar. See SGWPlayer.def's comment
/// on `onPlayerTeleport` and docs/protocol/position-updates.md.
///
/// Wire layout matches `build_enter_world_body`:
/// `[entityID:u32][spaceID:u32][vehicleID:u32=0][pos:3×f32][vel:3×f32=0]
///  [rot:3×f32][flags:u8=0x01]`.
pub fn build_forced_position(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    entity_id: u32,
    space_id: u32,
    position: [f32; 3],
) -> Vec<u8> {
    let mut body = Vec::with_capacity(50);
    body.push(BASEMSG_FORCED_POSITION);
    body.extend_from_slice(&entity_id.to_le_bytes());
    body.extend_from_slice(&space_id.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes()); // vehicleID = 0
    for &c in &position {
        body.extend_from_slice(&c.to_le_bytes());
    }
    body.extend_from_slice(&[0u8; 12]); // velocity = 0,0,0
    body.extend_from_slice(&[0u8; 12]); // rotation = 0,0,0 (yaw/pitch/roll)
    body.push(0x01); // flags

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt a single server→client entity method call.
///
/// Used by CellService to send entity method calls (e.g., `onTimerUpdate`,
/// `onEffectResults`) to a specific client.
pub fn build_entity_method_packet(
    key: &[u8; 32],
    seq: u32,
    acks: &[u32],
    entity_id: u32,
    method_index: u16,
    args: &[u8],
) -> Vec<u8> {
    let mut body = Vec::with_capacity(32 + args.len());
    append_entity_method(&mut body, method_index, entity_id, args);
    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build default NPC stat data matching `SGWBeing.statsTemplate`.
///
/// Wire format: `ARRAY<StatUpdate>` = `[count: u32 LE][StatUpdate, ...]`
/// where `StatUpdate = { StatId: i32, Min: i32, Current: i32, Max: i32 }` (16 bytes each).
/// 11 stats × 16 bytes + 4 byte count = 180 bytes total.
fn build_default_npc_stats() -> Vec<u8> {
    use cimmeria_entity::stats::*;
    // (stat_id, min, current, max) — from SGWBeing.statsTemplate defaults
    let stats: &[(i32, i32, i32, i32)] = &[
        (HEALTH, 0, 100, 100),
        (FOCUS, 0, 0, 0),
        (COORDINATION, 0, 1, 1),
        (ENGAGEMENT, 0, 1, 1),
        (FORTITUDE, 0, 1, 1),
        (MORALE, 0, 1, 1),
        (PERCEPTION, 0, 1, 1),
        (INTELLIGENCE, 0, 1, 1),
        (ACCURACY, -1000, 0, 1000),
        (MOVEMENT_SPEED_MOD, 0, 100, 500),
        (DEFENSE, 0, 0, 0),
    ];
    let mut buf = Vec::with_capacity(4 + stats.len() * 16);
    buf.extend_from_slice(&(stats.len() as u32).to_le_bytes());
    for &(id, min, cur, max) in stats {
        buf.extend_from_slice(&id.to_le_bytes());
        buf.extend_from_slice(&min.to_le_bytes());
        buf.extend_from_slice(&cur.to_le_bytes());
        buf.extend_from_slice(&max.to_le_bytes());
    }
    buf
}

/// Append appearance data for an NPC entity (BeingAppearance or onStaticMeshNameUpdate).
///
/// Mirrors `SGWBeing.createAppearanceOnClient()` / `SGWSpawnableEntity.createAppearanceOnClient()`:
/// - If bodySet + components (humanoid): `BeingAppearance(bodySet, componentList)` + `onEntityTint(0,0,0)`
/// - Else if staticMesh + bodySet: `onStaticMeshNameUpdate(staticMesh, bodySet)`
fn append_appearance(body: &mut Vec<u8>, entity_id: u32, d: &NpcAoIData) {
    if let Some(ref body_set) = d.body_set {
        if !body_set.is_empty() && !d.components.is_empty() {
            // Humanoid: BeingAppearance(bodySet: WSTRING, componentList: ARRAY<WSTRING>)
            let mut args = Vec::with_capacity(128);
            write_wstring(&mut args, body_set);
            // ARRAY<WSTRING>: [count: u32 LE][WSTRING, WSTRING, ...]
            args.extend_from_slice(&(d.components.len() as u32).to_le_bytes());
            for comp in &d.components {
                write_wstring(&mut args, comp);
            }
            append_entity_method(body, method_idx::BEING_APPEARANCE, entity_id, &args);

            // onEntityTint(primaryColorId=0, secondaryColorId=0, skinTint=0)
            let mut tint_args = Vec::with_capacity(12);
            tint_args.extend_from_slice(&0u32.to_le_bytes());
            tint_args.extend_from_slice(&0u32.to_le_bytes());
            tint_args.extend_from_slice(&0u32.to_le_bytes());
            append_entity_method(body, method_idx::ON_ENTITY_TINT, entity_id, &tint_args);
            return;
        }
    }

    // Non-humanoid: onStaticMeshNameUpdate(staticMeshName: WSTRING, bodySet: WSTRING)
    if let Some(ref static_mesh) = d.static_mesh {
        if !static_mesh.is_empty() {
            let body_set_str = d.body_set.as_deref().unwrap_or("");
            let mut args = Vec::with_capacity(64);
            write_wstring(&mut args, static_mesh);
            write_wstring(&mut args, body_set_str);
            // onStaticMeshNameUpdate is method index 0
            append_entity_method(body, 0, entity_id, &args);
        }
    }
}

/// Pack a float angle (radians) into a single byte (256 steps per circle).
///
/// Matches C++ `(uint8_t)(angle / 0.024543693f)`.
pub(crate) fn pack_angle(radians: f32) -> u8 {
    const SCALE: f32 = 0.024543693;
    (radians / SCALE) as u8
}

/// Pack a velocity Vec3 into 5 bytes using the C++ `packXYZ` format.
///
/// Exact port of `ClientHandler::packXYZ()` from `client_handler.cpp:647-687`.
pub(crate) fn pack_velocity_xyz(v: [f32; 3]) -> [u8; 5] {
    let mut packed1: u32 = 0;
    let mut packed2: u8 = 0;

    // X component
    let x = if v[0] < 0.0 {
        packed1 |= 0x00800000;
        -v[0]
    } else {
        v[0]
    };
    let x_biased = x + 2.0;
    let x_bits = x_biased.to_bits();
    packed1 |= (x_bits >> 3) & 0x007FF000;

    // Z component
    let z = if v[2] < 0.0 {
        packed1 |= 0x00000800;
        -v[2]
    } else {
        v[2]
    };
    let z_biased = z + 2.0;
    let z_bits = z_biased.to_bits();
    packed1 |= (z_bits >> 15) & 0x000007FF;

    // Y component
    let y = if v[1] < 0.0 {
        packed2 |= 0x80;
        -v[1]
    } else {
        v[1]
    };
    let y_biased = y + 2.0;
    let y_bits = y_biased.to_bits();
    let y_delta = (y_bits >> 12) & 0x00007FFF;
    packed1 |= (y_delta & 0xFF) << 24;
    packed2 |= ((y_delta & 0x7F00) >> 8) as u8;

    let p1 = packed1.to_le_bytes();
    [p1[0], p1[1], p1[2], p1[3], packed2]
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_mercury::encryption::MercuryEncryption;

    const TEST_KEY: [u8; 32] = [0x42u8; 32];

    /// `build_forced_position` must produce the exact byte layout the BigWorld
    /// engine expects on the client side. The layout matches `build_enter_world_body`
    /// in world_data/phases.rs — same message id, same field order, same flags
    /// byte. Mismatch here is the difference between the avatar snapping and
    /// staying put.
    #[test]
    fn forced_position_wire_layout() {
        let pkt = build_forced_position(
            &TEST_KEY,
            1,
            &[],
            0x12345678,
            0x0001_0010,
            [10.0, 20.0, 30.0],
        );
        let enc = MercuryEncryption::from_session_key(TEST_KEY);
        let pt = enc.decrypt(&pkt).unwrap();

        // body starts at offset 1 (offset 0 is flags).
        assert_eq!(pt[1], super::super::BASEMSG_FORCED_POSITION, "msg id");
        // entity_id LE
        assert_eq!(&pt[2..6], &0x12345678u32.to_le_bytes());
        // space_id LE
        assert_eq!(&pt[6..10], &0x00010010u32.to_le_bytes());
        // vehicleID = 0
        assert_eq!(&pt[10..14], &0u32.to_le_bytes());
        // pos x/y/z
        assert_eq!(&pt[14..18], &10.0f32.to_le_bytes());
        assert_eq!(&pt[18..22], &20.0f32.to_le_bytes());
        assert_eq!(&pt[22..26], &30.0f32.to_le_bytes());
        // velocity = 0,0,0 (12 zero bytes)
        assert_eq!(&pt[26..38], &[0u8; 12]);
        // rotation = 0,0,0 (12 zero bytes)
        assert_eq!(&pt[38..50], &[0u8; 12]);
        // flags = 0x01
        assert_eq!(pt[50], 0x01, "flags");
    }

    /// `build_create_entity_base` emits CREATE_ENTITY (0x09) immediately
    /// followed by UPDATE_AVATAR (0x10) in a single packet — the client
    /// expects the create + initial pose to arrive together so the entity
    /// is fully positioned before the cascade configures it.
    #[test]
    fn create_entity_base_wire_layout() {
        let pkt = build_create_entity_base(
            &TEST_KEY,
            1,
            &[],
            0xDEADBEEF,
            0x42,
            [10.0, 20.0, 30.0],
            [0.0, 0.0, 0.0],
        );
        let enc = MercuryEncryption::from_session_key(TEST_KEY);
        let pt = enc.decrypt(&pkt).unwrap();

        // Body starts at offset 1 (offset 0 is flags).
        // CREATE_ENTITY: [0x09][word_len=8 LE][entity_id u32][0xFF][class_id][0x00][0x00]
        assert_eq!(pt[1], BASEMSG_CREATE_ENTITY);
        assert_eq!(
            u16::from_le_bytes([pt[2], pt[3]]),
            8,
            "CREATE_ENTITY wordLength"
        );
        assert_eq!(&pt[4..8], &0xDEADBEEFu32.to_le_bytes());
        assert_eq!(pt[8], 0xFF, "idAlias = no alias");
        assert_eq!(pt[9], 0x42, "class_id");
        assert_eq!(&pt[10..12], &[0x00, 0x00], "two trailing zero bytes");

        // UPDATE_AVATAR begins at offset 12: [0x10][entity_id u32][pos 3×f32]
        // [vel 5 zero bytes][0x01 physics mode][yaw][pitch][roll]
        assert_eq!(pt[12], BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR);
        assert_eq!(&pt[13..17], &0xDEADBEEFu32.to_le_bytes());
        assert_eq!(&pt[17..21], &10.0f32.to_le_bytes());
        assert_eq!(&pt[21..25], &20.0f32.to_le_bytes());
        assert_eq!(&pt[25..29], &30.0f32.to_le_bytes());
        assert_eq!(
            &pt[29..34],
            &[0u8; 5],
            "velocity bytes (zero on initial pose)"
        );
        assert_eq!(pt[34], 0x01, "physics mode");
        // yaw/pitch/roll are direction[1]/[0]/[2] — all zero direction packs to 0
        assert_eq!(
            &pt[35..38],
            &[0u8; 3],
            "yaw/pitch/roll = 0 for zero direction"
        );

        // Body length = 11 (CREATE_ENTITY) + 26 (UPDATE_AVATAR) = 37 bytes,
        // occupying pt[1..38]. With FLAG_HAS_SEQUENCE the seq_id (we passed
        // 1) is appended as a u32 footer immediately after the body, so
        // pt[38..42] must equal the seq_id LE bytes. Asserting that pins
        // "the body is exactly 37 bytes" — a layout drift would shift the
        // footer and fail this check.
        assert_eq!(
            u32::from_le_bytes([pt[38], pt[39], pt[40], pt[41]]),
            1,
            "seq_id footer must start at pt[38] (proves body length = 37)",
        );
    }

    /// `build_entity_invisible` is the smallest AoI builder — 6-byte body.
    /// Used for ring-transport teleport-out fades where the entity should
    /// vanish but not be deleted from the client's entity table.
    #[test]
    fn entity_invisible_wire_layout() {
        let pkt = build_entity_invisible(&TEST_KEY, 1, &[], 0x12345678);
        let enc = MercuryEncryption::from_session_key(TEST_KEY);
        let pt = enc.decrypt(&pkt).unwrap();

        // [flags][0x0B][entity_id u32][0xFF]
        assert_eq!(pt[1], BASEMSG_ENTITY_INVISIBLE);
        assert_eq!(&pt[2..6], &0x12345678u32.to_le_bytes());
        assert_eq!(pt[6], 0xFF, "idAlias = no alias");
    }

    /// `build_entity_leave` emits ENTITY_INVISIBLE (0x0B) then LEAVE_AOI (0x0C)
    /// in a single packet — invisible-first prevents a one-frame "ghost"
    /// flicker as the client tears down the entity. Both records must reach
    /// the wire in this order.
    #[test]
    fn entity_leave_emits_invisible_then_leave_aoi() {
        let pkt = build_entity_leave(&TEST_KEY, 1, &[], 0xCAFEF00D);
        let enc = MercuryEncryption::from_session_key(TEST_KEY);
        let pt = enc.decrypt(&pkt).unwrap();

        // ENTITY_INVISIBLE first: [0x0B][entity_id u32][0xFF]
        assert_eq!(pt[1], BASEMSG_ENTITY_INVISIBLE);
        assert_eq!(&pt[2..6], &0xCAFEF00Du32.to_le_bytes());
        assert_eq!(pt[6], 0xFF);

        // LEAVE_AOI next at offset 7:
        //   [0x0C][word_len=8 LE][entity_id u32][cacheStamp=0 u32]
        assert_eq!(pt[7], BASEMSG_LEAVE_AOI);
        assert_eq!(
            u16::from_le_bytes([pt[8], pt[9]]),
            8,
            "LEAVE_AOI wordLength"
        );
        assert_eq!(&pt[10..14], &0xCAFEF00Du32.to_le_bytes());
        assert_eq!(&pt[14..18], &0u32.to_le_bytes(), "cacheStamp = 0");
    }

    /// `build_avatar_update` is the per-tick AoI position relay for ghost
    /// entities. Wire layout is fixed-size 26 bytes after the msg id, so any
    /// drift in field order desyncs all subsequent updates in the same packet.
    #[test]
    fn avatar_update_wire_layout() {
        let pkt = build_avatar_update(
            &TEST_KEY,
            1,
            &[],
            0x00ABCDEF,
            [100.0, 200.0, 300.0],
            [0.0, 0.0, 0.0], // zero velocity — packed to 5 zero bytes
            [0.0, 0.0, 0.0], // zero direction — yaw/pitch/roll all 0
        );
        let enc = MercuryEncryption::from_session_key(TEST_KEY);
        let pt = enc.decrypt(&pkt).unwrap();

        // [flags][0x10][entity_id u32][pos 3×f32][vel 5 bytes][0x01][yaw][pitch][roll]
        assert_eq!(pt[1], BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR);
        assert_eq!(&pt[2..6], &0x00ABCDEFu32.to_le_bytes());
        assert_eq!(&pt[6..10], &100.0f32.to_le_bytes());
        assert_eq!(&pt[10..14], &200.0f32.to_le_bytes());
        assert_eq!(&pt[14..18], &300.0f32.to_le_bytes());
        // Zero velocity packed via pack_velocity_xyz — exact bytes are an
        // implementation detail of the packer, but a zero input MUST round-
        // trip to all zeros so a missing-update doesn't drift the ghost.
        assert_eq!(
            &pt[18..23],
            &[0u8; 5],
            "zero velocity must pack to 5 zero bytes"
        );
        assert_eq!(pt[23], 0x01, "physics mode");
        assert_eq!(&pt[24..27], &[0u8; 3], "yaw/pitch/roll for zero direction");
    }
}
