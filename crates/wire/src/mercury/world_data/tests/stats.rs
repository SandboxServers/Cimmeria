//! Tests for archetype stat tables and level-experience curves.

use super::super::*;

/// The `onMaxExpUpdate(INT32)` argument the world-entry `mapLoaded` body
/// carries for a player at `level`.
fn map_loaded_max_exp(level: i32) -> i32 {
    use super::{sample_player_load_data, sample_world_entry, walk_entity_method_records};
    let mut data = sample_player_load_data();
    data.level = level;
    let body = build_map_loaded_body(42, &data, &sample_world_entry());
    let records = walk_entity_method_records(&body);
    let &(_, start) = records
        .iter()
        .find(|(idx, _)| *idx == crate::mercury::method_idx::ON_MAX_EXP_UPDATE)
        .expect("mapLoaded must carry onMaxExpUpdate");
    // Record = [0xBD][u16 word_len][...]; the INT32 arg is its last 4 bytes.
    let word_len = u16::from_le_bytes([body[start + 1], body[start + 2]]) as usize;
    let end = start + 3 + word_len;
    i32::from_le_bytes(body[end - 4..end].try_into().unwrap())
}

/// World entry reads the `game` crate's single 50-level XP table. Before
/// AT-07 `mercury::world_data::stats` carried its own 21-entry copy that
/// clamped at 400,000, so a level-21+ player saw the level-20 threshold.
/// Levels 21 and 50 below fail against that copy.
#[test]
fn map_loaded_max_exp_reads_the_50_level_table() {
    assert_eq!(map_loaded_max_exp(-3), 0, "a negative DB level reads as 0");
    assert_eq!(map_loaded_max_exp(1), 100);
    assert_eq!(map_loaded_max_exp(20), 400_000);
    assert_eq!(map_loaded_max_exp(21), 460_000);
    assert_eq!(map_loaded_max_exp(49), 23_065_000);
    assert_eq!(
        map_loaded_max_exp(50),
        26_525_000,
        "at the cap the client is shown the level-50 display sentinel"
    );
    assert_eq!(
        map_loaded_max_exp(99),
        26_525_000,
        "clamped to the sentinel"
    );
}

#[test]
fn archetype_stats_commando_differs() {
    let soldier = archetype_stats(1);
    let commando = archetype_stats(2);
    assert_eq!(soldier.coordination, 5);
    assert_eq!(commando.coordination, 4);
    assert_eq!(commando.perception, 5);
}
