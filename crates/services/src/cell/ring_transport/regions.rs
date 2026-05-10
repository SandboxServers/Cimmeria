//! Ring-transport region definitions loaded from the database.
//!
//! Mirrors `python/common/defs/RingTransporterRegion.py`. One row per ring pad
//! across all worlds — the table is global because rings can teleport across
//! worlds, so the destination metadata must be resolvable from any cell.

use std::collections::HashMap;

use sqlx::PgPool;

/// One ring transporter region — a teleporter pad keyed by `region_id`
/// (cross-world unique).
#[derive(Debug, Clone)]
pub struct RingRegion {
    pub region_id: i32,
    pub world_id: i32,
    pub world_name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub tag: String,
    pub height: f32,
    pub radius: f32,
    pub event_set_id: i32,
    pub display_name_id: i32,
    /// Region IDs this pad can teleport to. Already filtered:
    /// self-references and unknown IDs are removed (matches Python `postLoad`).
    pub destination_ids: Vec<i32>,
    /// `point_sets.set_id` for the trigger volume around this pad.
    pub point_set_id: i32,
    /// When set, `handle_interact` and `handle_select_destination` reject
    /// the player unless `mission_<id>_status` is `completed`. NULL (None)
    /// means "no gating" — the historical default for all rings.
    pub required_mission_id: Option<i32>,
}

/// Load every `ring_transport_regions` row from the DB and return them keyed
/// by `region_id`.
///
/// The Python `postLoad` step (drop self-references and dangling destination
/// IDs) is performed in a second pass once the full table is in memory.
pub async fn load_ring_regions(pool: &PgPool) -> Result<HashMap<i32, RingRegion>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT r.region_id, r.world_id, r.x, r.y, r.z, r.tag, \
                r.height, r.radius, r.event_set_id, r.display_name_id, \
                r.destination_region_ids, r.point_set_id, \
                r.required_mission_id, w.world AS world_name \
           FROM resources.ring_transport_regions r \
           JOIN resources.worlds w ON r.world_id = w.world_id \
          ORDER BY r.region_id",
    )
    .fetch_all(pool)
    .await?;

    let mut regions: HashMap<i32, RingRegion> = HashMap::with_capacity(rows.len());
    for r in &rows {
        let region_id: i32 = r.get("region_id");
        // SQL NULL → empty list (intentional — region with no destinations is
        // valid). Real decode errors (wrong column type, malformed array)
        // bubble up so startup fails loudly instead of silently dropping
        // routing data.
        let dests: Option<Vec<i32>> = r.try_get("destination_region_ids")?;
        let dests = dests.unwrap_or_default();
        regions.insert(
            region_id,
            RingRegion {
                region_id,
                world_id: r.get("world_id"),
                world_name: r.get("world_name"),
                x: r.get("x"),
                y: r.get("y"),
                z: r.get("z"),
                tag: r.get("tag"),
                // height/radius are NOT NULL in the schema — a decode error here
                // means real corruption (wrong column type, malformed numeric).
                // Bubble up so startup fails loudly instead of producing a
                // zero-radius pad the trigger volume can't see players entering.
                height: r.try_get::<f32, _>("height")?,
                radius: r.try_get::<f32, _>("radius")?,
                event_set_id: r.get("event_set_id"),
                display_name_id: r.get("display_name_id"),
                destination_ids: dests,
                point_set_id: r.get("point_set_id"),
                required_mission_id: r.try_get("required_mission_id")?,
            },
        );
    }

    // postLoad: filter self-references and dangling destinations.
    let known: std::collections::HashSet<i32> = regions.keys().copied().collect();
    for (id, region) in regions.iter_mut() {
        let mut filtered = Vec::with_capacity(region.destination_ids.len());
        for &dst in &region.destination_ids {
            if dst == *id {
                tracing::warn!(
                    region_id = id,
                    "ring region lists itself as destination — dropping"
                );
                continue;
            }
            if !known.contains(&dst) {
                tracing::warn!(
                    region_id = id,
                    dst,
                    "ring region destination not in table — dropping"
                );
                continue;
            }
            filtered.push(dst);
        }
        region.destination_ids = filtered;
    }

    tracing::info!(count = regions.len(), "Loaded ring transport regions");
    Ok(regions)
}

#[cfg(test)]
mod live_db_tests {
    //! Live-DB regression guards on the seeded `ring_transport_regions`
    //! rows used by the cross-world armory ring. The chain replay tests
    //! pin the chains; these tests pin the runtime-loaded ring metadata
    //! they depend on. Drift in either direction (column type change,
    //! seed row removal, sequence value drift) surfaces here near the
    //! seed it touches.

    use super::*;
    use crate::test_support::require_db_or_skip;

    /// Region 33 (Cellblock-side armory ring) must load from the seeded
    /// `ring_transport_regions` row with `required_mission_id = Some(688)`,
    /// `world_name = "Castle_CellBlock"`, and destination 34 in the list.
    /// A regression here would either remove the gate (NULL column) or
    /// re-point the destination, both of which would let a player ring
    /// out of the armory before completing the gating mission.
    #[tokio::test]
    async fn region_33_loads_with_mission_gate_and_destination_34() {
        let pool = require_db_or_skip!();
        let regions = load_ring_regions(&pool)
            .await
            .expect("load_ring_regions must succeed against seeded DB");
        let r33 = regions
            .get(&33)
            .expect("region 33 (Cellblock_ArmoryRingSwitch) must be present in the seed");
        assert_eq!(
            r33.tag, "Cellblock_ArmoryRingSwitch",
            "region 33 tag drifted",
        );
        assert_eq!(
            r33.world_name, "Castle_CellBlock",
            "region 33 must live on world Castle_CellBlock",
        );
        assert_eq!(
            r33.required_mission_id,
            Some(688),
            "region 33 must gate on mission 688",
        );
        assert!(
            r33.destination_ids.contains(&34),
            "region 33 must list region 34 (Castle drop zone) as a destination; got {:?}",
            r33.destination_ids,
        );
    }

    /// Region 34 (Castle-side drop zone) must load with no
    /// `required_mission_id` — the destination ring is reached via
    /// teleport, never directly interacted with, so gating it would
    /// only mask future regressions.
    #[tokio::test]
    async fn region_34_loads_with_no_mission_gate_on_castle() {
        let pool = require_db_or_skip!();
        let regions = load_ring_regions(&pool)
            .await
            .expect("load_ring_regions must succeed against seeded DB");
        let r34 = regions
            .get(&34)
            .expect("region 34 (Castle_ArmoryRingDropZone) must be present in the seed");
        assert_eq!(
            r34.tag, "Castle_ArmoryRingDropZone",
            "region 34 tag drifted",
        );
        assert_eq!(
            r34.world_name, "Castle",
            "region 34 must live on world Castle (world_id=8)",
        );
        assert_eq!(
            r34.required_mission_id, None,
            "region 34 must NOT have a mission gate (destination ring, never directly interacted with)",
        );
    }

    /// Pre-existing rings (region_id 1, 2, 3 — Cellblock 1/2/3) must
    /// continue to load with `required_mission_id = None`. A failure
    /// here means the seed migration accidentally added a gate to
    /// historical rings, which would break the same-world Cellblock
    /// flow that mission 640 / HackTheRings depends on.
    #[tokio::test]
    async fn historical_cellblock_rings_have_no_mission_gate() {
        let pool = require_db_or_skip!();
        let regions = load_ring_regions(&pool)
            .await
            .expect("load_ring_regions must succeed against seeded DB");
        for id in [1i32, 2, 3] {
            let r = regions
                .get(&id)
                .unwrap_or_else(|| panic!("region {id} must remain in the seed"));
            assert_eq!(
                r.required_mission_id, None,
                "region {id} (CellblockRing{id}) must remain ungated",
            );
        }
    }

    /// The chain replay tests reference spawn tags
    /// (`Cellblock_TerminalX`, `Cellblock_ArmoryRingSwitch`,
    /// `Cellblock_ArmoryGuard1`) by name — but those tests inject the
    /// tag into the chain context directly and don't read from the
    /// `spawnlist` seed. This test pins the seed-side mapping so a
    /// rename or accidental NULL would surface even if the chains
    /// themselves are correct. spawn_id 27 was previously untagged
    /// and 79 was `Cellblock_FakeRingSwitch`; both were re-tagged
    /// when the armory mission was wired up.
    #[tokio::test]
    async fn mission_688_spawn_retags_are_present() {
        use sqlx::Row;

        let pool = require_db_or_skip!();
        let rows = sqlx::query(
            "SELECT spawn_id, tag FROM resources.spawnlist \
             WHERE spawn_id IN (27, 34, 79) ORDER BY spawn_id",
        )
        .fetch_all(&pool)
        .await
        .expect("query spawnlist must succeed");
        let mut by_id: std::collections::HashMap<i32, String> = std::collections::HashMap::new();
        for r in rows {
            let id: i32 = r.get("spawn_id");
            let tag: Option<String> = r.try_get("tag").ok();
            by_id.insert(id, tag.unwrap_or_default());
        }
        assert_eq!(
            by_id.get(&27).map(String::as_str),
            Some("Cellblock_ArmoryGuard1"),
            "spawn 27 must be tagged Cellblock_ArmoryGuard1 (was previously NULL)",
        );
        assert_eq!(
            by_id.get(&34).map(String::as_str),
            Some("Cellblock_TerminalX"),
            "spawn 34 (the armory terminal) must remain tagged Cellblock_TerminalX",
        );
        assert_eq!(
            by_id.get(&79).map(String::as_str),
            Some("Cellblock_ArmoryRingSwitch"),
            "spawn 79 must be re-tagged Cellblock_ArmoryRingSwitch (was Cellblock_FakeRingSwitch)",
        );
    }

    /// The new ring rows reference `point_sets` 2080/2081 by id —
    /// without those point_sets the trigger volume around the pad
    /// doesn't exist and the FSM can't observe the player stepping
    /// onto the ring. Pin both sides of the foreign-key relationship.
    #[tokio::test]
    async fn mission_688_ring_point_sets_are_present() {
        use sqlx::Row;

        let pool = require_db_or_skip!();
        let rows = sqlx::query(
            "SELECT set_id, world_id, shape FROM resources.point_sets \
             WHERE set_id IN (2080, 2081) ORDER BY set_id",
        )
        .fetch_all(&pool)
        .await
        .expect("query point_sets must succeed");
        assert_eq!(
            rows.len(),
            2,
            "expected point_sets 2080 and 2081 (mission 688 rings) — got {} rows",
            rows.len(),
        );
        let cellblock = &rows[0];
        let castle = &rows[1];
        assert_eq!(cellblock.get::<i32, _>("set_id"), 2080);
        assert_eq!(
            cellblock.get::<i32, _>("world_id"),
            12,
            "point_set 2080 must live on world 12 (Castle_CellBlock)",
        );
        assert_eq!(cellblock.get::<String, _>("shape"), "Cylinder");
        assert_eq!(castle.get::<i32, _>("set_id"), 2081);
        assert_eq!(
            castle.get::<i32, _>("world_id"),
            8,
            "point_set 2081 must live on world 8 (Castle)",
        );
        assert_eq!(castle.get::<String, _>("shape"), "Cylinder");
    }
}
