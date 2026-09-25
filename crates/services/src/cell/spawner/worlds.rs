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
//!   [`crate::cell::space_manager::NavmeshMode`]).
//!
//! Every other cell-side loader JOINs that table to recover the name and
//! throws the rest away, so this is the one place the mapping is kept. One
//! query for both columns: a second round trip for the mode could fail on
//! its own and leave the two halves of a world's definition disagreeing.
//!
//! Consumed by [`SpaceManager::stamp_world_rows`](crate::cell::space_manager::SpaceManager::stamp_world_rows),
//! which writes both onto the matching `WorldDef`.

use std::collections::HashMap;

use sqlx::PgPool;

use crate::cell::space_manager::NavmeshMode;

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
/// [`mode_from_db_value`](crate::cell::space_manager::NavmeshMode) for why
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
            let navmesh_mode = crate::cell::space_manager::mode_from_db_value(&world, &mode);
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
                "Dakara_E1",
                "Dakara_E1_StoryRm",
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
