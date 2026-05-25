//! Tests for the `world_data` module — kept separate from `mod.rs` glue
//! to keep that file readable.

#![cfg(test)]

use super::*;

mod bandolier;
mod entity_encoding;
mod map_loaded;
mod player_creation;
mod stargates;
mod stats;

const TEST_KEY: [u8; 32] = [0x42u8; 32];

fn sample_player_load_data() -> PlayerLoadData {
    PlayerLoadData {
        player_id: 1,
        level: 5,
        player_name: "RoundTrip".into(),
        extra_name: String::new(),
        alignment: 1,
        archetype: 2,
        gender: 1,
        bodyset: "BS_HumanMale.BS_HumanMale".into(),
        components: vec![],
        weapon_visual: None,
        exp: 0,
        naquadah: 0,
        known_stargates: vec![],
        abilities: vec![],
        training_points: 0,
        applied_science_points: 0,
        blueprint_ids: vec![],
        first_login: 0,
        access_level: 0,
        skin_color_id: 0,
        ability_tree: archetype_ability_tree(2),
        items: vec![],
        active_bandolier_slot: 0,
        bandolier_items: vec![],
        auto_reload: true,
        reload_on_activate: false,
    }
}

fn sample_world_entry() -> WorldEntryInfo {
    WorldEntryInfo {
        player_entity_id: 100,
        space_id: 65552,
        pos: [0.0; 3],
        rot: [0.0; 3],
        world_name: "CombatSim".into(),
        class_id: 0x02,
        world_stargates: vec![],
    }
}

/// Walk an entity-method body and return the (method_index, byte_offset)
/// of each record in encounter order. Stops at the first non-record byte.
///
/// Used by both the fragmentation tests in `map_loaded` and the player
/// creation/appearance ordering tests in `player_creation`.
pub(super) fn walk_entity_method_records(body: &[u8]) -> Vec<(u16, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < body.len() {
        let b = body[i];
        if !(0x80..=0xBD).contains(&b) {
            break;
        }
        if i + 3 > body.len() {
            break;
        }
        let word_len = u16::from_le_bytes([body[i + 1], body[i + 2]]) as usize;
        let record_end = i + 3 + word_len;
        if record_end > body.len() {
            break;
        }
        let method_index = if b == 0xBD {
            if i + 7 >= body.len() {
                break;
            }
            61 + body[i + 7] as u16
        } else {
            (b & 0x7F) as u16
        };
        out.push((method_index, i));
        i = record_end;
    }
    out
}

/// Locate a `setupStargateInfo` (method index 65, extended encoding) call for
/// `entity_id` inside an unfragmented mapLoaded body and return its raw args
/// (the bytes after the sub-index byte, up to the encoded payload length).
///
/// Wire layout for an extended entity-method call:
///   `0xBD | payload_len:u16 LE | entity_id:u32 LE | sub_index:u8 | args...`
/// where payload_len = 4 + 1 + args.len() and sub_index = method_index - 61.
fn find_setup_stargate_info_args(body: &[u8], entity_id: u32) -> &[u8] {
    let sub_index = (method_idx::SETUP_STARGATE_INFO - 61) as u8;
    let eid = entity_id.to_le_bytes();
    let mut i = 0;
    while i < body.len() {
        if body[i] == 0xBD && i + 8 <= body.len() {
            let payload_len = u16::from_le_bytes([body[i + 1], body[i + 2]]) as usize;
            // A 0xBD byte that appears inside another method's args is not a
            // record start. Reject any apparent header whose payload_len is
            // too small to cover (eid + sub_index = 5) or that runs past the
            // body's end, and advance one byte instead of trusting a bogus
            // length to skip past the (non-)record.
            if payload_len >= 5 && i + 3 + payload_len <= body.len() {
                let header_end = i + 3 + 4 + 1; // marker + len + eid + sub_index
                if body[i + 3..i + 7] == eid && body[i + 7] == sub_index {
                    let args_len = payload_len - 5;
                    return &body[header_end..header_end + args_len];
                }
                // Real entity-method record, just not ours — skip past it.
                i += 3 + payload_len;
                continue;
            }
        }
        i += 1;
    }
    panic!("setupStargateInfo call not found in mapLoaded body for entity_id={entity_id}");
}
