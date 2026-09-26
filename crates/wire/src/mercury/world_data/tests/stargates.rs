//! Tests covering the `setupStargateInfo` payload — both the per-world
//! gate list and the player's known-stargates roundtrip — embedded inside
//! the `mapLoaded` body.

use super::super::*;
use super::find_setup_stargate_info_args;
use super::sample_player_load_data;
use super::sample_world_entry;

/// `world_stargates: Vec<i32>` on `WorldEntryInfo` must round-trip through
/// `setupStargateInfo` exactly — count prefix + i32 LE entries — and must
/// come *before* the `knownStargates` array.
#[test]
fn world_stargates_round_trips_through_setup_stargate_info() {
    let mut data = sample_player_load_data();
    // Distinctive non-overlapping values so the assertion can't accidentally
    // match `known_stargates` data later in the same args buffer.
    data.known_stargates = vec![777, 888];
    let mut entry = sample_world_entry();
    entry.world_stargates = vec![1234, 5678, 9012];

    let body = build_map_loaded_body(entry.player_entity_id, &data, &entry);
    let args = find_setup_stargate_info_args(&body, entry.player_entity_id);

    // worldStargateIds: u32 count + count * i32 LE
    let world_count = u32::from_le_bytes([args[0], args[1], args[2], args[3]]);
    assert_eq!(world_count, 3, "world_stargates count prefix mismatch");
    assert_eq!(
        i32::from_le_bytes([args[4], args[5], args[6], args[7]]),
        1234,
    );
    assert_eq!(
        i32::from_le_bytes([args[8], args[9], args[10], args[11]]),
        5678,
    );
    assert_eq!(
        i32::from_le_bytes([args[12], args[13], args[14], args[15]]),
        9012,
    );

    // Immediately after the world_stargates payload comes the knownStargateIds
    // array. Verify it parses cleanly — proves the world_stargates section
    // didn't desync the buffer.
    let known_off = 4 + 3 * 4;
    let known_count = u32::from_le_bytes([
        args[known_off],
        args[known_off + 1],
        args[known_off + 2],
        args[known_off + 3],
    ]);
    assert_eq!(
        known_count, 2,
        "knownStargates count mismatch (alignment drift?)"
    );
    assert_eq!(
        i32::from_le_bytes([
            args[known_off + 4],
            args[known_off + 5],
            args[known_off + 6],
            args[known_off + 7],
        ]),
        777,
    );
}

/// An empty `world_stargates` must still emit a u32(0) count prefix. Eliding
/// the empty array entirely makes the client read the next field's bytes as
/// the world count and corrupts the gate-travel address book.
#[test]
fn empty_world_stargates_still_emits_zero_count_prefix() {
    let mut data = sample_player_load_data();
    data.known_stargates = vec![42];
    let entry = sample_world_entry(); // world_stargates is empty by default

    let body = build_map_loaded_body(entry.player_entity_id, &data, &entry);
    let args = find_setup_stargate_info_args(&body, entry.player_entity_id);

    // Empty world_stargates: count = 0, no entries.
    assert_eq!(
        u32::from_le_bytes([args[0], args[1], args[2], args[3]]),
        0,
        "empty world_stargates must still emit a u32(0) count prefix",
    );
    // knownStargates count + value follow immediately.
    assert_eq!(
        u32::from_le_bytes([args[4], args[5], args[6], args[7]]),
        1,
        "knownStargates count read garbage — empty world_stargates desynced the buffer",
    );
    assert_eq!(
        i32::from_le_bytes([args[8], args[9], args[10], args[11]]),
        42,
    );
}
