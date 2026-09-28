//! Outbound (server → client) method decoders — Phase 1 priority set.
//!
//! Each `decode_*` function reads the wire schema for one method and
//! returns a [`serde_json::Value`] (typically an Object) with named
//! fields. Failures return `None` and the wire-log capture falls back
//! to `args_hex` only.
//!
//! Schemas come from `docs/protocol/client-method-dispatch-table.md`
//! and the canonical `.def` files under `entities/defs/`.

use serde_json::{json, Value};

use super::primitives::Cursor;

pub(super) fn decode(method_index: u16, args: &[u8]) -> Option<Value> {
    match method_index {
        // SGWSpawnableEntity
        1 => decode_on_sequence(args),
        // SGWBeing interface
        12 => decode_on_timer_update(args),
        14 => decode_on_effect_results(args),
        19 => decode_on_state_field_update(args),
        // SGWCombatant interface
        20 => decode_on_stat_update(args),
        21 => decode_on_stat_base_update(args),
        // SGWBeing own
        26 => decode_being_appearance(args),
        // SGWPlayer own
        105 => decode_on_dialog_display(args),
        _ => None,
    }
}

/// Method 1: `onSequence` — the wire packet that plays a kismet
/// sequence client-side. This is the fire-animation broadcast (and
/// every other weapon/spell visual).
///
/// Wire: `INT32 KismetEventSetSeqID, INT32 SourceID, INT32 TargetID,
/// INT8 PrimaryTarget, FLOAT ImpactTime, ARRAY<NameValuePair>
/// NameValuePairs, INT8 ViewType, INT32 InstanceId`
///
/// We decode the fixed prefix + the ARRAY count; per-pair NVPs are
/// captured as `nvp_count` only (they're rarely diagnostic and have
/// their own variable-length payloads).
fn decode_on_sequence(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let seq_id = c.i32_le()?;
    let source_id = c.i32_le()?;
    let target_id = c.i32_le()?;
    let primary_target = c.i8()?;
    let impact_time = c.f32_le()?;
    let nvp_count = c.u32_le()?;
    // Skip past the NVP payload (each pair is var-length WSTRINGs); we
    // don't decode the pairs themselves, but we DO want ViewType
    // (the byte right after the NVP array) so SigNoz can filter onSequence
    // packets by camera-control mode. EKismetViewType values per
    // `docs/gameplay/cinematic-system.md`:
    //   0 = Witness, 1 = NonWitness, 2 = Finish,
    //   3 = EventInvoker (ring transport, gate),
    //   4 = EventWitness.
    // Because NVPs are var-length we can only reliably read ViewType
    // when nvp_count == 0 (the common case). Otherwise we omit it.
    let view_type = if nvp_count == 0 { c.u8() } else { None };
    Some(json!({
        "kismet_event_set_seq_id": seq_id,
        "source_id": source_id,
        "target_id": target_id,
        "primary_target": primary_target,
        "impact_time": impact_time,
        "nvp_count": nvp_count,
        "view_type": view_type,
    }))
}

/// Method 12: `onTimerUpdate` — cooldown / channel timer broadcasts.
/// Wire: `INT32 ID, INT8 Type, INT32 SourceID, INT32 SecondaryId,
/// FLOAT TotalTime, FLOAT BigWorldTimeComplete` (21 bytes;
/// `interfaces/SGWBeing.def`).
fn decode_on_timer_update(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    Some(json!({
        "id": c.i32_le()?,
        "timer_type": c.i8()?,
        "source_id": c.i32_le()?,
        "secondary_id": c.i32_le()?,
        "total_time": c.f32_le()?,
        "completion_time": c.f32_le()?,
    }))
}

/// Method 14: `onEffectResults` — the per-hit damage broadcast.
/// Wire: `INT32 SourceID, INT32 AbilityID, INT32 EffectID, INT32
/// TargetID, UINT8 ResultCode, ClientEffectResultList`
///
/// The ClientEffectResultList tail is variable-length; we capture
/// the fixed prefix + the tail's first byte (typically the result
/// count) for at-a-glance diagnostics.
fn decode_on_effect_results(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let source_id = c.i32_le()?;
    let ability_id = c.i32_le()?;
    let effect_id = c.i32_le()?;
    let target_id = c.i32_le()?;
    let result_code = c.u8()?;
    let result_list_byte = c.u8();
    Some(json!({
        "source_id": source_id,
        "ability_id": ability_id,
        "effect_id": effect_id,
        "target_id": target_id,
        "result_code": result_code,
        "result_list_first_byte": result_list_byte,
    }))
}

/// Method 19: `onStateFieldUpdate` — the BSF flags broadcast (in-combat,
/// dead, auto-cycling, etc.). The bug-relevant bits are `BSF_DEAD`
/// (bit 0), `BSF_IN_COMBAT` (bit 3), `BSF_AUTO_CYCLING` (bit 4).
/// Wire: `INT32 bStateField`
fn decode_on_state_field_update(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let state_field = c.i32_le()?;
    Some(json!({
        "state_field": state_field,
        "state_field_hex": format!("0x{state_field:08x}"),
        "dead": (state_field & 0x01) != 0,
        "in_combat": (state_field & 0x08) != 0,
        "auto_cycling": (state_field & 0x10) != 0,
    }))
}

/// How many `StatUpdate` entries a stat decoder samples per event.
const STAT_SAMPLE_MAX: u32 = 8;

/// Method 20: `onStatUpdate` — stat dirty-flush broadcasts.
fn decode_on_stat_update(args: &[u8]) -> Option<Value> {
    decode_stat_update_list(args)
}

/// Method 21: `onStatBaseUpdate` — the same `StatUpdateList`, carrying base
/// (unbuffed) values instead of the live ones.
fn decode_on_stat_base_update(args: &[u8]) -> Option<Value> {
    decode_stat_update_list(args)
}

/// `StatUpdateList` (`entities/defs/alias.xml`): `UINT32 count`, then
/// `count` 16-byte `StatUpdate` entries of `INT32 StatId, INT32 Min,
/// INT32 Current, INT32 Max` — the layout `StatList::serialize_entries`
/// writes. Before #843 this walked 8-byte `(stat_id, value)` pairs, so every
/// entry after the first was misaligned. Decodes the count and the first
/// [`STAT_SAMPLE_MAX`] entries.
fn decode_stat_update_list(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let count = c.u32_le()?;
    let mut sample = Vec::new();
    for _ in 0..count.min(STAT_SAMPLE_MAX) {
        let stat_id = c.i32_le()?;
        let min = c.i32_le()?;
        let cur = c.i32_le()?;
        let max = c.i32_le()?;
        sample.push(json!({ "stat_id": stat_id, "min": min, "cur": cur, "max": max }));
    }
    Some(json!({
        "count": count,
        "sample": sample,
        "sample_truncated": count > STAT_SAMPLE_MAX,
    }))
}

/// Method 26: `BeingAppearance` — the visual model + component list.
/// **This is the broadcast that controls weapon visibility** — the
/// drone-aggro-breaks-weapon-display bug lives here.
///
/// Wire: `WSTRING BodySet, ARRAY<WSTRING> Components`
///
/// `body_set` names the body mesh (e.g., `"BodySet_HumanMale"`).
/// `components` is the ARRAY of visual attachments — weapon, armor,
/// helmet. If the weapon is missing from this list, the client won't
/// render it regardless of bandolier state.
fn decode_being_appearance(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    let body_set = c.wstring()?;
    let component_count = c.u32_le()?;
    let mut components = Vec::with_capacity(component_count.min(32) as usize);
    for _ in 0..component_count.min(32) {
        components.push(c.wstring()?);
    }
    Some(json!({
        "body_set": body_set,
        "component_count": component_count,
        "components": components,
        "components_truncated": component_count > 32,
    }))
}

/// Method 105: `onDialogDisplay` — the dialog open packet. PR #401
/// fixed the bug where this was being sent with the player's entity
/// id instead of the NPC's.
///
/// Wire: `INT32 entityId, INT32 dialogId, INT32 missionFlags, UINT8
/// isImmediate, INT32 missionId`
fn decode_on_dialog_display(args: &[u8]) -> Option<Value> {
    let mut c = Cursor::new(args);
    Some(json!({
        "entity_id": c.i32_le()?,
        "dialog_id": c.i32_le()?,
        "mission_flags": c.i32_le()?,
        "is_immediate": c.u8()?,
        "mission_id": c.i32_le()?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_timer_update_decodes_secondary_id() {
        // id=597, type=0, source=100, secondary=42, total=5.0, complete=12345.0
        let args = [
            0x55, 0x02, 0x00, 0x00, // ID
            0x00, // Type
            0x64, 0x00, 0x00, 0x00, // SourceID
            0x2A, 0x00, 0x00, 0x00, // SecondaryId
            0x00, 0x00, 0xA0, 0x40, // TotalTime = 5.0
            0x00, 0xE4, 0x40, 0x46, // BigWorldTimeComplete = 12345.0
        ];
        assert_eq!(args.len(), 21, "onTimerUpdate wire layout is 21 bytes");
        let decoded = decode_on_timer_update(&args).unwrap();
        assert_eq!(decoded["id"], 597);
        assert_eq!(decoded["timer_type"], 0);
        assert_eq!(decoded["source_id"], 100);
        assert_eq!(decoded["secondary_id"], 42);
        assert!((decoded["total_time"].as_f64().unwrap() - 5.0).abs() < f64::EPSILON);
        assert!((decoded["completion_time"].as_f64().unwrap() - 12345.0).abs() < f64::EPSILON);
    }

    #[test]
    fn on_timer_update_truncated_without_secondary_id_returns_none() {
        // Legacy 17-byte layout (missing SecondaryId) must not decode cleanly.
        let args = [
            0x55, 0x02, 0x00, 0x00, 0x00, 0x64, 0x00, 0x00, 0x00, 0x00, 0x00, 0xA0, 0x40, 0x00,
            0xE4, 0x40, 0x46,
        ];
        assert_eq!(args.len(), 17);
        assert!(decode_on_timer_update(&args).is_none());
    }

    #[test]
    fn on_state_field_update_decodes_in_combat() {
        // INT32 = 0x00000008 = BSF_IN_COMBAT alone
        let args = [0x08, 0x00, 0x00, 0x00];
        let decoded = decode_on_state_field_update(&args).unwrap();
        assert_eq!(decoded["state_field"], 8);
        assert_eq!(decoded["in_combat"], true);
        assert_eq!(decoded["dead"], false);
    }

    #[test]
    fn on_dialog_display_decodes_npc_id() {
        // entity=0x64 (100), dialog=0x2A (42), flags=0, immediate=1, mission=0
        let args = [
            0x64, 0x00, 0x00, 0x00, // entity
            0x2A, 0x00, 0x00, 0x00, // dialog
            0x00, 0x00, 0x00, 0x00, // flags
            0x01, // immediate
            0x00, 0x00, 0x00, 0x00, // mission
        ];
        let decoded = decode_on_dialog_display(&args).unwrap();
        assert_eq!(decoded["entity_id"], 100);
        assert_eq!(decoded["dialog_id"], 42);
        assert_eq!(decoded["is_immediate"], 1);
    }

    #[test]
    fn being_appearance_decodes_body_and_components() {
        let mut args = Vec::new();
        // BodySet WSTRING
        let body = "Body";
        args.extend_from_slice(&(body.len() as u32).to_le_bytes());
        for ch in body.encode_utf16() {
            args.extend_from_slice(&ch.to_le_bytes());
        }
        // 2 components
        args.extend_from_slice(&2u32.to_le_bytes());
        for comp in ["Wep", "Armor"] {
            args.extend_from_slice(&(comp.len() as u32).to_le_bytes());
            for ch in comp.encode_utf16() {
                args.extend_from_slice(&ch.to_le_bytes());
            }
        }
        let decoded = decode_being_appearance(&args).unwrap();
        assert_eq!(decoded["body_set"], "Body");
        assert_eq!(decoded["component_count"], 2);
        assert_eq!(decoded["components"][0], "Wep");
        assert_eq!(decoded["components"][1], "Armor");
    }

    /// A `StatList` with HEALTH and FOCUS dirty, serialized by the server's
    /// own encoder.
    fn two_stat_list() -> cimmeria_entity::stats::StatList {
        use cimmeria_entity::stats::{StatList, FOCUS, HEALTH};
        let mut stats = StatList::new();
        stats.clear_dirty();
        stats.get_mut(HEALTH).unwrap().update(0, 750, 1000);
        stats.get_mut(FOCUS).unwrap().update(5, 40, 120);
        stats.get_mut(HEALTH).unwrap().set_base(0, 900, 1200);
        stats.get_mut(FOCUS).unwrap().set_base(10, 60, 150);
        stats
    }

    fn assert_stat(entry: &Value, stat_id: i32, min: i32, cur: i32, max: i32) {
        assert_eq!(
            entry,
            &json!({ "stat_id": stat_id, "min": min, "cur": cur, "max": max }),
        );
    }

    /// #843: each `StatUpdate` is 16 bytes. The old 8-byte walk read
    /// HEALTH's Min as its value and FOCUS's stat id from HEALTH's Current.
    #[test]
    fn on_stat_update_decodes_the_server_encoders_16_byte_entries() {
        use cimmeria_entity::stats::{FOCUS, HEALTH};
        let args = two_stat_list().serialize_dirty();
        assert_eq!(args.len(), 4 + 2 * 16);
        let decoded = decode(20, &args).unwrap();
        assert_eq!(decoded["count"], 2);
        assert_eq!(decoded["sample_truncated"], false);
        let sample = decoded["sample"].as_array().unwrap();
        assert_eq!(sample.len(), 2);
        // `serialize_entries` sorts by stat id: HEALTH (7) before FOCUS (8).
        assert_stat(&sample[0], HEALTH, 0, 750, 1000);
        assert_stat(&sample[1], FOCUS, 5, 40, 120);
    }

    #[test]
    fn on_stat_base_update_decodes_the_same_layout() {
        use cimmeria_entity::stats::{FOCUS, HEALTH};
        let args = two_stat_list().serialize_dirty_base();
        let decoded = decode(21, &args).unwrap();
        assert_eq!(decoded["count"], 2);
        let sample = decoded["sample"].as_array().unwrap();
        assert_stat(&sample[0], HEALTH, 0, 900, 1200);
        assert_stat(&sample[1], FOCUS, 10, 60, 150);
    }

    /// A full stat list (every stat) samples the first eight and flags the
    /// truncation; a list cut mid-entry does not decode.
    #[test]
    fn stat_update_list_truncates_the_sample_and_rejects_a_partial_entry() {
        let full = cimmeria_entity::stats::StatList::new().serialize_all();
        let decoded = decode(20, &full).unwrap();
        assert!(decoded["count"].as_u64().unwrap() > 8);
        assert_eq!(decoded["sample"].as_array().unwrap().len(), 8);
        assert_eq!(decoded["sample_truncated"], true);

        let args = two_stat_list().serialize_dirty();
        assert!(decode(20, &args[..args.len() - 4]).is_none());
    }

    #[test]
    fn truncated_payload_returns_none() {
        assert!(decode_on_dialog_display(&[0x00, 0x01]).is_none());
        assert!(decode_being_appearance(&[]).is_none());
    }

    #[test]
    fn dispatch_unknown_method_returns_none() {
        assert!(decode(9999, &[0u8; 16]).is_none());
    }
}
