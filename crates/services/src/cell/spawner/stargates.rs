//! Stargate destination cache.
//!
//! Maps `stargate_id → StargateEntry` for gate-travel target resolution.
//! Callers that need "which gate is on the world I'm standing on" (the DHD)
//! scan the map themselves and break ties by lowest `stargate_id`; there is
//! deliberately no second index, because two worlds carry more than one gate
//! row and the tie-break rule belongs with the caller that renders it.

use sqlx::PgPool;

/// Cached stargate destination from `resources.stargates` + `resources.worlds`.
#[derive(Debug, Clone)]
pub struct StargateEntry {
    pub world_name: String,
    /// The stargate prop's own transform. This is the prefab origin, not
    /// necessarily a standable point — see [`StargateEntry::arrival`].
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    /// Point-of-origin glyph (1–38) for the DHD dialling UI.
    ///
    /// **Not an identifier.** It repeats across rows (`address_origin = 1` on
    /// both `SGC W2` and `SGC`, `13` on both Dakara E2 and E3), so it must
    /// never be used as a key into this map — the key is `stargate_id`.
    pub address_origin: i32,
    /// Authored arrival point + facing (`arrival_x/y/z/yaw`), when pinned.
    ///
    /// `None` means "arrive on the gate row", which is what the 2009 server
    /// always did. Set on worlds where the prefab origin is not a standable
    /// point; see the column comment in
    /// `db/resources/Worlds/Tables/stargates.sql`.
    pub arrival: Option<([f32; 3], f32)>,
    /// `stargates.event_set_id` — the Kismet event set for THIS gate's
    /// prefab. Resolves `Stargate_MakeGate` (6100) / `Stargate_CrossGate`
    /// (6113) through `SpaceManager::sequence_map`. Nullable in the seed:
    /// most unbuilt worlds' gates have no prefab and carry NULL.
    pub event_set_id: Option<i32>,
}

impl StargateEntry {
    /// The position + yaw a traveller to this gate should be placed at,
    /// before navmesh validation. Prefers the authored arrival pin and falls
    /// back to the gate row — including the row's `yaw`, which is already the
    /// authored "face this way" value the 2009 `moveTo` passed.
    pub fn desired_arrival(&self) -> ([f32; 3], f32) {
        self.arrival.unwrap_or(([self.x, self.y, self.z], self.yaw))
    }
}

/// Load stargate destinations from the database.
///
/// Maps `stargate_id → StargateEntry` for gate travel lookups.
///
/// `ORDER BY stargate_id` does not affect the resulting `HashMap`; it is
/// there so the partial-arrival warns below come out in a stable order when
/// an operator is diffing two startups.
pub async fn load_stargates(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, StargateEntry>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT s.stargate_id, w.world AS world_name, \
                s.x_pos, s.y_pos, s.z_pos, s.yaw, s.address_origin, \
                s.arrival_x, s.arrival_y, s.arrival_z, s.arrival_yaw, \
                s.event_set_id \
         FROM resources.stargates s \
         JOIN resources.worlds w ON s.world_id = w.world_id \
         ORDER BY s.stargate_id",
    )
    .fetch_all(pool)
    .await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for r in &rows {
        let id: i32 = r.get("stargate_id");
        let world_name: String = r.get("world_name");

        // The CHECK constraint on the table makes a partial arrival group
        // unrepresentable, but the loader must not *depend* on it: a database
        // restored from an older dump predates the constraint. Treat any
        // partial group as "not pinned" and say so, rather than silently
        // arriving at (x, 0, z).
        let ax: Option<f64> = r.get("arrival_x");
        let ay: Option<f64> = r.get("arrival_y");
        let az: Option<f64> = r.get("arrival_z");
        let ayaw: Option<f64> = r.get("arrival_yaw");
        let arrival = match (ax, ay, az, ayaw) {
            (Some(x), Some(y), Some(z), Some(yaw)) => {
                Some(([x as f32, y as f32, z as f32], yaw as f32))
            }
            (None, None, None, None) => None,
            _ => {
                tracing::warn!(
                    stargate_id = id,
                    world_name = %world_name,
                    reason = "partial_arrival_row",
                    "load_stargates: stargates.arrival_* is partially populated — \
                     ignoring the pin and arriving on the gate row, which may be \
                     off-navmesh; re-pin all four columns or clear all four"
                );
                None
            }
        };

        map.insert(
            id,
            StargateEntry {
                world_name,
                x: r.get::<f64, _>("x_pos") as f32,
                y: r.get::<f64, _>("y_pos") as f32,
                z: r.get::<f64, _>("z_pos") as f32,
                yaw: r.get::<f64, _>("yaw") as f32,
                address_origin: r.get("address_origin"),
                arrival,
                event_set_id: r.get::<Option<i32>, _>("event_set_id"),
            },
        );
    }

    tracing::info!(count = map.len(), "Loaded stargates cache");
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(arrival: Option<([f32; 3], f32)>) -> StargateEntry {
        StargateEntry {
            world_name: "Harset".to_string(),
            x: -0.076,
            y: -67.274,
            z: 38.011,
            yaw: 2.75,
            address_origin: 6,
            arrival,
            event_set_id: None,
        }
    }

    /// Unpinned gate: the traveller arrives on the gate row, yaw included.
    /// This is the pre-H01 behaviour and must survive the change.
    #[test]
    fn desired_arrival_falls_back_to_the_gate_row_including_yaw() {
        let (pos, yaw) = entry(None).desired_arrival();
        assert_eq!(pos, [-0.076, -67.274, 38.011]);
        assert_eq!(yaw, 2.75, "an unpinned gate must keep the row's facing");
    }

    /// Pinned gate: the arrival pin wins over the gate row on all four
    /// components. Reverting `desired_arrival` to `[self.x, self.y, self.z]`
    /// fails this.
    #[test]
    fn desired_arrival_prefers_the_authored_pin() {
        let (pos, yaw) = entry(Some(([10.0, 20.0, 30.0], 1.25))).desired_arrival();
        assert_eq!(pos, [10.0, 20.0, 30.0]);
        assert_eq!(yaw, 1.25);
    }

    mod live_db {
        use super::super::load_stargates;
        use crate::test_support::require_db_or_skip;

        /// Sentinel gate row. Fits `i32`, well clear of
        /// `stargates_id_seq` (set to 100000) and of every seeded id.
        const SENTINEL_GATE: i32 = 900_301;
        /// Harset (world 57) — the gate this whole packet is about.
        const HARSET_GATE: i32 = 3;

        async fn delete_sentinel(pool: &sqlx::PgPool) {
            sqlx::query("DELETE FROM resources.stargates WHERE stargate_id = $1")
                .bind(SENTINEL_GATE)
                .execute(pool)
                .await
                .expect("sentinel cleanup must succeed");
        }

        async fn insert_sentinel(
            pool: &sqlx::PgPool,
            arrival: Option<(f64, f64, f64, f64)>,
        ) -> Result<(), sqlx::Error> {
            let (ax, ay, az, ayaw) = match arrival {
                Some((x, y, z, yaw)) => (Some(x), Some(y), Some(z), Some(yaw)),
                None => (None, None, None, None),
            };
            sqlx::query(
                "INSERT INTO resources.stargates \
                 (address1, address2, address3, address4, address5, address6, \
                  address_origin, stargate_id, name, pitch, prefab_sequence, roll, \
                  world_id, x_pos, y_pos, yaw, z_pos, \
                  arrival_x, arrival_y, arrival_z, arrival_yaw) \
                 VALUES (1,2,3,4,5,6, 7, $1, 'H01 sentinel', 0, '', 0, \
                         57, 1.0, 2.0, 0.5, 3.0, $2, $3, $4, $5)",
            )
            .bind(SENTINEL_GATE)
            .bind(ax)
            .bind(ay)
            .bind(az)
            .bind(ayaw)
            .execute(pool)
            .await
            .map(|_| ())
        }

        /// The loader must survive the four new columns and read
        /// `address_origin` and the arrival group off the row.
        ///
        /// Scoped to the one seeded row this test owns, deliberately: a
        /// global "every gate is pinned" assertion would pick up any
        /// sentinel a sibling test leaked into the shared database, and 27
        /// of the 28 rows are still unpinned.
        ///
        /// The arrival assertion was inverted by placement PL-A-01, which is
        /// the change H01's version of this test anticipated ("would break
        /// the moment milestone M0 pins Harset's arrival"). The pin is now
        /// asserted by value rather than merely present: the whole point of
        /// the four columns is *which* point they name, and
        /// `arrival.is_some()` alone would stay green if a seed edit moved
        /// the pin back onto the un-standable prefab origin. The navmesh
        /// half of that claim lives in
        /// `cell::harset_placement_tests` — this one only pins the seed
        /// round-trip through the loader.
        #[tokio::test]
        async fn load_stargates_reads_the_new_columns() {
            let pool = require_db_or_skip!();
            let map = load_stargates(&pool)
                .await
                .expect("load_stargates must succeed against the seeded DB");

            let harset = map
                .get(&HARSET_GATE)
                .expect("stargate_id 3 (Harset) must be seeded");
            assert_eq!(harset.world_name, "Harset");
            assert_eq!(
                harset.address_origin, 6,
                "address_origin must come off the row, not be defaulted"
            );
            // Exact equality: the seed literals are `double precision` that
            // round-trip exactly into `f32`, and no arithmetic touches them
            // on the way through.
            const PIN: [f32; 3] = [-5.0, -68.99, 33.0];
            assert_eq!(
                harset.arrival.map(|(pos, _)| pos),
                Some(PIN),
                "Harset's arrival pin (placement PL-A-01) did not round-trip — \
                 either the three arrival_x/y/z values in \
                 db/resources/Worlds/Seed/stargates.sql changed, or the loader \
                 dropped the group as partial"
            );
            // The pin's yaw is asserted against the ROW's yaw rather than
            // against a literal, because that identity is the authoring
            // decision: yaw is atan2(dx, dz) with 0 = +Z, the gate row's
            // 3.141 already faces -Z (away from the gate, down the plaza),
            // and PL-A-01 deliberately repeats it instead of re-deriving a
            // facing. Spelling the number here would also mean writing an
            // approximation of PI that is not PI.
            assert_eq!(
                harset.arrival.map(|(_, yaw)| yaw),
                Some(harset.yaw),
                "arrival_yaw must repeat the gate row's own yaw — a pin that \
                 diverges from it is either a re-derived facing (say so in the \
                 seed comment) or a typo"
            );
            // `desired_arrival` must prefer the pin over the row; the row is
            // the prefab origin 2 m above the plaza dais.
            assert_eq!(harset.desired_arrival(), (PIN, harset.yaw));
        }

        /// `address_origin` goes on the wire as a `UINT8` glyph, and
        /// `try_open_dhd` refuses to emit anything outside that range. A
        /// seeded row outside it would leave players on that world with a
        /// dead DHD and only a server-side warn to show for it, so pin the
        /// seed rather than relying on the runtime guard.
        #[tokio::test]
        async fn every_seeded_address_origin_fits_the_wire_glyph_range() {
            let pool = require_db_or_skip!();
            let bad: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM resources.stargates \
                 WHERE address_origin < 1 OR address_origin > 38",
            )
            .fetch_one(&pool)
            .await
            .expect("count query must succeed");
            assert_eq!(bad, 0, "address_origin is a 1-38 point-of-origin glyph");
        }

        /// A fully-populated arrival group round-trips into
        /// `StargateEntry::arrival` and wins over the gate row.
        #[tokio::test]
        async fn a_pinned_arrival_row_round_trips() {
            let pool = require_db_or_skip!();
            delete_sentinel(&pool).await;
            insert_sentinel(&pool, Some((10.5, 20.25, 30.75, 1.5)))
                .await
                .expect("a complete arrival group must be accepted");

            // Cleanup runs on the panic path too. The database is shared with
            // six other sessions; a sentinel leaked by a failing assertion
            // below would otherwise sit in `resources.stargates` until
            // somebody noticed it in an unrelated test's output.
            let _guard = SentinelCleanup(pool.clone());

            let map = load_stargates(&pool).await.expect("load must succeed");
            let gate = map.get(&SENTINEL_GATE).expect("sentinel row must load");
            assert_eq!(gate.arrival, Some(([10.5, 20.25, 30.75], 1.5)));
            assert_eq!(gate.desired_arrival(), ([10.5, 20.25, 30.75], 1.5));
        }

        /// Deletes [`SENTINEL_GATE`] on drop, including while unwinding.
        struct SentinelCleanup(sqlx::PgPool);

        impl Drop for SentinelCleanup {
            fn drop(&mut self) {
                let pool = self.0.clone();
                // `Drop` is sync and the test's runtime may already be
                // unwinding, so spin up a throwaway one-thread runtime rather
                // than trying to re-enter the ambient one.
                std::thread::spawn(move || {
                    let rt = match tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        Ok(rt) => rt,
                        Err(_) => return,
                    };
                    rt.block_on(async {
                        let _ =
                            sqlx::query("DELETE FROM resources.stargates WHERE stargate_id = $1")
                                .bind(SENTINEL_GATE)
                                .execute(&pool)
                                .await;
                    });
                })
                .join()
                .ok();
            }
        }

        /// A half-pinned row would read as a valid partial arrival and drop
        /// the traveller at (x, 0, z). The table's CHECK constraint makes it
        /// unrepresentable; dropping the constraint fails this.
        #[tokio::test]
        async fn a_partially_pinned_arrival_row_is_rejected_by_the_constraint() {
            let pool = require_db_or_skip!();
            delete_sentinel(&pool).await;

            // x/z pinned, y and yaw left NULL.
            let res = sqlx::query(
                "INSERT INTO resources.stargates \
                 (address1, address2, address3, address4, address5, address6, \
                  address_origin, stargate_id, name, pitch, prefab_sequence, roll, \
                  world_id, x_pos, y_pos, yaw, z_pos, arrival_x, arrival_z) \
                 VALUES (1,2,3,4,5,6, 7, $1, 'H01 sentinel', 0, '', 0, \
                         57, 1.0, 2.0, 0.5, 3.0, 10.5, 30.75)",
            )
            .bind(SENTINEL_GATE)
            .execute(&pool)
            .await;

            assert!(
                res.is_err(),
                "stargates_arrival_all_or_nothing must reject a half-pinned \
                 arrival group"
            );

            delete_sentinel(&pool).await;
        }
    }
}
