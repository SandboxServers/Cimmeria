//! Create-entity packet builders — Phase 1 (CREATE_ENTITY + UPDATE_AVATAR)
//! and Phase 2 (`createOnClient()` property cascade), plus the cascade's
//! NPC stat and appearance helpers.

use cimmeria_mercury::packet::{build_outgoing, FLAG_HAS_ACKS};

use crate::cell::messages::NpcAoIData;
use crate::mercury::{
    append_entity_method, encrypt_packet, method_idx, write_wstring, REPLY_FLAGS,
};

use super::{pack_angle, BASEMSG_CREATE_ENTITY, BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR};

/// `GENERICPROPERTY_DatabaseId` — maps to speaker_id for dialog-capable entities.
const GENERICPROPERTY_DATABASE_ID: i32 = 9;

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
    tracing::debug!(
        entity_id,
        class_id,
        "ON_VISIBLE(1) appended to create_entity bundle"
    );

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
