//! World-name → per-world cell settings startup cache.
//!
//! `entities/spaces.xml` carries world *names* only. Two things the cell
//! needs live exclusively in `resources.worlds`:
//!
//! * the numeric `world_id` that `spawnlist`, `stargates`,
//!   `ring_transport_regions` and content-engine `world` condition rows all
//!   reference, and
//! * `navmesh_mode`, which decides whether that world's navmesh is a
//!   containment gate on player movement or information only (see
//!   [`NavmeshMode`]).
//!
//! Every other cell-side loader JOINs that table to recover the name and
//! throws the rest away, so this is the one place the mapping is kept. One
//! query for both columns: a second round trip for the mode could fail on
//! its own and leave the two halves of a world's definition disagreeing.
//!
//! Consumed by `SpaceManager::stamp_world_rows` (`cell::space_manager` in
//! `cimmeria-services`), which writes both onto the matching `WorldDef`.

use std::collections::HashMap;

use sqlx::PgPool;

use super::navmesh_mode::{mode_from_db_value, NavmeshMode};

/// The `resources.worlds` columns the cell stamps onto a `WorldDef`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldRow {
    pub world_id: i32,
    pub navmesh_mode: NavmeshMode,
}

impl WorldRow {
    /// A row with containment enforced — the shape every world but Harset
    /// has, and the one test fixtures want.
    pub fn enforcing(world_id: i32) -> Self {
        Self {
            world_id,
            navmesh_mode: NavmeshMode::Enforce,
        }
    }
}

/// Load `world name → {world_id, navmesh_mode}` from `resources.worlds`.
///
/// The name column is `world`, not `world_name` — every sibling loader
/// aliases it the same way (`spawner/stargates.rs`, `spawner/respawners.rs`).
///
/// An unrecognised `navmesh_mode` string is logged and read as
/// [`NavmeshMode::Enforce`]; see
/// [`mode_from_db_value`] for why
/// the fallback is the strict side.
pub async fn load_world_rows(pool: &PgPool) -> Result<HashMap<String, WorldRow>, sqlx::Error> {
    let rows: Vec<(i32, String, String)> = sqlx::query_as(
        "SELECT world_id, world, navmesh_mode FROM resources.worlds ORDER BY world_id",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(world_id, world, mode)| {
            let navmesh_mode = mode_from_db_value(&world, &mode);
            (
                world,
                WorldRow {
                    world_id,
                    navmesh_mode,
                },
            )
        })
        .collect())
}

#[cfg(test)]
mod live_db_tests {
    //! Live-DB guards on the seeded `navmesh_mode` column. The column is a
    //! movement gate: a world that loses its `'advisory'` row becomes
    //! unwalkable for ordinary players, and a world that gains one silently
    //! loses containment. Both directions are asserted against the real
    //! seed rather than a fixture, because the defect shape is "the seed
    //! says something different from what the code assumes".

    use super::*;
    use crate::test_support::require_db_or_skip;

    /// Harset (57) is seeded advisory, and Castle_CellBlock (12) — the one
    /// world whose mesh players have walked under containment — is not.
    ///
    /// Asserted as a pair on purpose. "Harset is advisory" alone passes
    /// just as well if the loader hardcoded advisory for everything, which
    /// would drop containment server-wide with one green test.
    #[tokio::test]
    async fn harset_loads_advisory_and_a_meshed_neighbour_loads_enforce() {
        let pool = require_db_or_skip!();
        let rows = load_world_rows(&pool).await.expect("load_world_rows");

        let harset = rows.get("Harset").expect(
            "resources.worlds must carry world 57 'Harset' — \
             db/resources/Worlds/Seed/worlds.sql is unseeded or not \\ir'd",
        );
        assert_eq!(harset.world_id, 57);
        assert_eq!(
            harset.navmesh_mode,
            NavmeshMode::Advisory,
            "Harset must load 'advisory': harset.nav has a 30-unit hole across \
             the only walk to the Command Center door, and enforcing it snaps \
             every non-GM player back at the boundary (H53)",
        );

        let cellblock = rows
            .get("Castle_CellBlock")
            .expect("resources.worlds must carry world 12 'Castle_CellBlock'");
        assert_eq!(cellblock.world_id, 12);
        assert_eq!(
            cellblock.navmesh_mode,
            NavmeshMode::Enforce,
            "a world nobody demoted must load 'enforce' — advisory is opt-in \
             per world, never the default",
        );
    }

    /// The seven historical CellBlock worlds load from the real seed under
    /// the ids the base sends in `onClientMapLoad` (the cell stamps these
    /// onto `spaces.xml` worlds; a missing row would leave a world with no
    /// id, which fails every content-engine `world` condition closed), and
    /// the stock CellBlock keeps its own id.
    #[tokio::test]
    async fn historical_cellblocks_load_under_their_wire_world_ids() {
        use cimmeria_wire::mercury::world_data::historical_cellblocks::historical_cellblocks;

        let pool = require_db_or_skip!();
        let rows = load_world_rows(&pool).await.expect("load_world_rows");
        for world in historical_cellblocks() {
            let row = rows.get(world.world).unwrap_or_else(|| {
                panic!(
                    "resources.worlds must carry {} ({})",
                    world.world, world.world_id
                )
            });
            assert_eq!(row.world_id, world.world_id, "{}", world.world);
            assert_eq!(row.navmesh_mode, NavmeshMode::Advisory, "{}", world.world);
        }
        assert_eq!(rows["Castle_CellBlock"].world_id, 12);
    }

    /// World 1300 `DebugArea` (DA-01) is seeded whole: its world row under the
    /// wire id, advisory like the map it plays on, its two respawners (130
    /// arrival, 131 the respawn test) and the cover nodes `cover_extract`
    /// generated from the Ihpet_Crater_Light map. Every piece fails here if
    /// its seed rows go missing.
    #[tokio::test]
    async fn live_db_debug_area_world_respawners_and_cover_are_seeded() {
        use cimmeria_wire::mercury::world_data::added_worlds::{
            DEBUG_AREA_WORLD, DEBUG_AREA_WORLD_ID,
        };

        let pool = require_db_or_skip!();
        let rows = load_world_rows(&pool).await.expect("load_world_rows");
        let row = rows
            .get(DEBUG_AREA_WORLD)
            .expect("resources.worlds must carry 1300 'DebugArea'");
        assert_eq!(row.world_id, DEBUG_AREA_WORLD_ID);
        assert_eq!(row.navmesh_mode, NavmeshMode::Advisory);

        let respawners = super::super::respawners::load_respawners(&pool)
            .await
            .expect("load_respawners");
        let mut debug: Vec<(i32, [f32; 3])> = respawners
            .iter()
            .filter(|r| r.world_name == DEBUG_AREA_WORLD)
            .map(|r| (r.respawner_id, r.pos))
            .collect();
        debug.sort_by_key(|(id, _)| *id);
        assert_eq!(
            debug,
            [(130, [251.0, 8.0, -962.0]), (131, [438.0, 10.4, -916.0])],
            "respawner 130 is the arrival `.gotolocation DebugArea` picks \
             (lowest id), 131 the Z9 respawn test"
        );

        let (sets, min_id, max_id): (i64, Option<i32>, Option<i32>) = sqlx::query_as(
            "SELECT count(*), min(chunk_id), max(chunk_id) \
             FROM resources.cover_sets WHERE world_id = $1",
        )
        .bind(DEBUG_AREA_WORLD_ID)
        .fetch_one(&pool)
        .await
        .expect("cover_sets query");
        let (nodes,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM resources.cover_nodes n \
             JOIN resources.cover_sets s ON s.chunk_id = n.chunk_id WHERE s.world_id = $1",
        )
        .bind(DEBUG_AREA_WORLD_ID)
        .fetch_one(&pool)
        .await
        .expect("cover_nodes query");
        assert_eq!(
            (sets, nodes),
            (DEBUG_AREA_COVER_SETS, DEBUG_AREA_COVER_NODES),
            "world 1300's cover is the cover_extract output for Ihpet_Crater_Light"
        );
        // Set ids are world_id * 100000 + n (n from 1).
        assert_eq!(min_id, Some(130_000_001));
        assert_eq!(max_id, Some(130_000_000 + DEBUG_AREA_COVER_SETS as i32));
    }

    /// `cover_extract` for `--map 1300=DebugArea=<CookedPC>/Maps/Ihpet_Crater_Light`
    /// (the counts in the cover seed headers).
    const DEBUG_AREA_COVER_SETS: i64 = 706;
    const DEBUG_AREA_COVER_NODES: i64 = 6_324;

    /// The column's default does the work for every row the seed never
    /// mentions it on. The advisory rows are exactly the worlds whose
    /// `data/spaces/*.nav` nobody has walked under containment, each with
    /// its evidence in the seed comment. A seed edit that widened the
    /// demotion would show up here as a changed list.
    ///
    /// - `Harset` (57): the 2012 `harset.nav` had a 30-unit hole across the
    ///   only walk to the Command Center door (H53). NA26 rebuilt it; the
    ///   rebuild covers the route but has not been walked yet.
    /// - `Castle` (8): `castle.nav` is new (rebuilt from the client maps
    ///   2026-09-19) and its exterior and interior are still separate
    ///   regions, so it ships as pathing / line-of-sight / height
    ///   information until an in-client walk proves its coverage.
    /// - The other 21 (NA26, 2026-09-25): every world that got its first
    ///   mesh, or a rebuilt one, from the cooked client maps. A new mesh
    ///   must not start snapping players back before anyone has walked it.
    ///   `SandBox` (2) is in the list because its client map is
    ///   Harset_CmdCenter and it loads a copy of that mesh.
    /// - The seven historical CellBlock worlds (1201–1207, `CellBlock43` …
    ///   `CellBlock63`): map archaeology with no mesh at all. The stock
    ///   `castle_cellblock.nav` does not match their older geometry, so
    ///   advisory keeps a mesh dropped in later from gating anyone before it
    ///   has been walked.
    /// - `DebugArea` (1300): the GM test map on the Ihpet_Crater_Light client
    ///   map. It runs on that map's `ihpet_crater_light.nav` (D-DA5), which is
    ///   advisory for world 73 too.
    ///
    /// `Castle_CellBlock` (12) is the one meshed world left on `enforce`:
    /// its mesh was rebuilt on 2026-09-19 and has been walked since.
    #[tokio::test]
    async fn only_the_documented_worlds_are_seeded_advisory() {
        let pool = require_db_or_skip!();
        let rows = load_world_rows(&pool).await.expect("load_world_rows");

        let mut advisory: Vec<&str> = rows
            .iter()
            .filter(|(_, r)| r.navmesh_mode == NavmeshMode::Advisory)
            .map(|(name, _)| name.as_str())
            .collect();
        advisory.sort_unstable();
        assert_eq!(
            advisory,
            [
                "Agnos",
                "Agnos_Library",
                "Beta_Site_Evo_1",
                "Castle",
                "CellBlock43",
                "CellBlock55",
                "CellBlock57",
                "CellBlock58",
                "CellBlock60",
                "CellBlock62",
                "CellBlock63",
                "Dakara_E1",
                "Dakara_E1_StoryRm",
                "DebugArea",
                "Harset",
                "Harset_CmdCenter",
                "Harset_Market",
                "Harset_StorageRm",
                "Ihpet_Crater_Dark",
                "Ihpet_Crater_Light",
                "Lucia",
                "Menfa_Dark",
                "Menfa_Light",
                "Omega_Site",
                "Omega_Site_CmdCenter",
                "SGC",
                "SGC_W1",
                "SandBox",
                "Sewer_Falls",
                "Tollana",
                "Tollana_Curia",
            ],
            "advisory is a per-world escape hatch for a known-bad mesh, not a \
             default; every other world's containment gate must stay on",
        );
    }
}
