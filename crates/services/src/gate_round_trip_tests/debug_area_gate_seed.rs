//! The Debug Area gate exists twice: as `resources.stargates` row 29 (what
//! the cell dials, grants and places by) and as the cooked
//! `COOKED_STARGATE` entry the client resolves ids in
//! (`cimmeria_resources::base::stargate_overrides::DEBUG_AREA_GATE`). This
//! live-DB test holds the two together: an edit to one that misses the other
//! would hand the client a gate whose world, prefab or address disagrees
//! with the server's.

use cimmeria_resources::base::stargate_overrides::DEBUG_AREA_GATE;
use sqlx::Row;

use crate::test_support::require_db_or_skip;

#[tokio::test]
async fn live_db_the_debug_area_gate_seed_row_matches_its_cooked_entry() {
    let pool = require_db_or_skip!();
    let row = sqlx::query(
        "SELECT world_id, name::text AS name, prefab_sequence::text AS prefab, \
                x_pos, y_pos, z_pos, yaw, pitch, roll, \
                ARRAY[address1, address2, address3, address4, address5, address6] AS address, \
                address_origin, debug_dial_hub \
           FROM resources.stargates WHERE stargate_id = $1",
    )
    .bind(DEBUG_AREA_GATE.stargate_id as i32)
    .fetch_optional(&pool)
    .await
    .expect("query must succeed")
    .expect("stargate 29 (Debug Area) is seeded");

    let g = DEBUG_AREA_GATE;
    assert_eq!(row.get::<i32, _>("world_id"), g.world_id as i32, "world");
    assert_eq!(row.get::<String, _>("name"), g.name, "name");
    assert_eq!(row.get::<String, _>("prefab"), g.prefab_sequence, "prefab");
    // The column is double precision, the cooked entry f32.
    let transform: [f32; 6] =
        ["x_pos", "y_pos", "z_pos", "yaw", "pitch", "roll"].map(|c| row.get::<f64, _>(c) as f32);
    assert_eq!(
        transform,
        [g.x, g.y, g.z, g.yaw, g.pitch, g.roll],
        "transform"
    );
    assert_eq!(
        row.get::<Vec<i32>, _>("address"),
        g.address.map(i32::from).to_vec(),
        "address glyphs"
    );
    assert_eq!(
        row.get::<i32, _>("address_origin"),
        i32::from(g.address_origin),
        "point of origin"
    );
    assert!(
        row.get::<bool, _>("debug_dial_hub"),
        "the row is the dial hub"
    );
}
