//! Live-DB guards on the world-57 population and named regions placed by
//! placement pass B (packets H14 and H15).
//!
//! Every row these guards cover is a *reconstructed* coordinate — a guess with
//! a recorded evidence class, per
//! `docs/analysis/harset-rebuild/placements/METHOD.md` — so the guards are
//! deliberately about the things a guess can still get objectively wrong:
//!
//! - the row exists at all, with the byte-exact `tag` a merged mission chain
//!   already dispatches on (a typo here is silent: `fire_interact_tag` simply
//!   matches nothing and the mission stalls);
//! - the tag set is derived from `content_triggers`, not from a list written by
//!   the same hand that wrote the seed, so a transposition is catchable;
//! - nothing placed in the shared hub is hostile (D-H03);
//! - the coordinate's relationship to `data/spaces/harset.nav` is what the
//!   ledger says it is, in both directions.
//!
//! **Why the navmesh guard is not a plain "every spawn is on-mesh" assertion.**
//! H14's acceptance line asks for `is_point_valid` on every world-57 spawn. That
//! cannot be satisfied and is not the right bar: the shipped `harset.nav` does
//! not cover the hub's upper quarters at the floor height the *geometry* has. A
//! ring probe around every named Jaffa Zone landmark found no on-mesh point
//! within 12 m at y = -41.3, and the two AUTHORED rows in that quarter — Petbe
//! (spawn 223) and `FirstBug` (spawn 224) — are off-mesh for the same reason.
//! World 57 runs `navmesh_mode = 'advisory'` (H53), so off-mesh costs NPC
//! pathing, not the player's session. So the guard is a biconditional over an
//! explicit exception table: a row the ledger calls on-mesh must be on-mesh,
//! and a row the ledger calls off-mesh must still be off-mesh. The second half
//! is the one that earns its keep — when GH1 rebuilds the mesh and a quarter
//! becomes walkable, this fails and forces the ledger row (and the
//! `is_stationary` decision that rests on it) to be revisited rather than
//! silently going stale.

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::NavMesh;

use super::*;
use crate::cell::spawner::regions::{is_point_in_region, load_regions_from_db};

/// World id of the Harset hub exterior.
const HARSET: i32 = 57;

/// `resources.worlds.world` for [`HARSET`] — the name `load_regions_from_db`
/// returns, since it joins through `resources.worlds`.
const HARSET_WORLD_NAME: &str = "Harset";

/// The two shared-hub worlds D-H03 forbids hostile NPCs in: 57 `Harset` and 68
/// `Harset_CmdCenter`.
const SHARED_HUB_WORLDS: &[i32] = &[57, 68];

/// Spawn-id block the placement ledger reserves for this pass.
const SPAWN_BLOCK: (i32, i32) = (300, 399);

/// Point-set id block the placement ledger reserves for this pass.
const REGION_BLOCK: (i32, i32) = (2100, 2149);

/// Is the coordinate expected to satisfy `is_point_valid` against the shipped
/// `harset.nav`?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mesh {
    /// On the mesh. Every one of these is on the hub component (187) or on
    /// component 1441, which the ledger records per row.
    On,
    /// Off the mesh, deliberately: placed on the true geometry floor from
    /// `obj_slab` in a quarter `harset.nav` does not model.
    Off,
}

/// Every row placement pass B adds to `spawnlist` for world 57, as
/// `(spawn_id, tag, template_id, mesh)`.
///
/// The coordinates are *not* repeated here — they are read from the DB by the
/// navmesh guard, because a hardcoded literal stops describing the seed the
/// moment someone corrects the seed, which is exactly what this pass expects
/// the owner to do after a playtest.
const PLACED: [(i32, &str, i32, Mesh); 15] = [
    (300, "Harset_Hansen", 212, Mesh::On),
    (301, "Harset_Jacobs", 213, Mesh::On),
    (302, "Harset_Lorak", 201, Mesh::On),
    (303, "Harset_FormerRaJaffa", 204, Mesh::Off),
    (304, "Harset_FormerRaJaffa2", 204, Mesh::Off),
    (305, "Harset_SuspiciousJaffa", 205, Mesh::On),
    (306, "SecondBug", 164, Mesh::Off),
    (307, "ThirdBug", 164, Mesh::Off),
    (308, "Harset_ShieldTower1", 243, Mesh::Off),
    (309, "Harset_ShieldTower2", 243, Mesh::Off),
    (310, "Harset_ShieldTower3", 243, Mesh::Off),
    (311, "Harset_ShieldControls", 248, Mesh::Off),
    (312, "Harset_BankAnchor", 248, Mesh::On),
    (313, "Harset_PetbeQuarters", 244, Mesh::Off),
    (314, "Harset_StorageLotaur", 219, Mesh::On),
];

/// Every named region placement pass B adds, as `(set_id, name, shape)`.
///
/// `Harset.Bar` and `Harset.HoldingPens` are absent on purpose — no landmark,
/// no actor, no telemetry, no spec coordinate. Ids 2108 and 2109 are reserved
/// for them; see the `## No idea` section of the ledger.
const REGIONS: [(i32, &str, &str); 8] = [
    (2100, "Harset.JaffaZone", "BoundingBox"),
    (2101, "Harset.OpCoreZone", "BoundingBox"),
    (2102, "Harset.Bank", "BoundingBox"),
    (2103, "Harset.PetbeQuarters", "BoundingBox"),
    (2104, "Harset.ShieldControls", "Cylinder"),
    (2105, "Harset.ShieldTower1", "Cylinder"),
    (2106, "Harset.ShieldTower2", "Cylinder"),
    (2107, "Harset.ShieldTower3", "Cylinder"),
];

/// Landmark instance positions each volume was sized to enclose, as
/// `(region name, landmark label, x, y, z)`.
///
/// These are `archetype_census` facts from the cooked map
/// (`docs/analysis/harset-rebuild/placements/data/Harset_arch_positions.tsv`),
/// not values derived from the region rows, so a box that was mistyped or
/// shrunk fails here. Deliberately one or two per volume: the point is that
/// the *named thing* is inside its *named region*, not a coverage census.
const MUST_ENCLOSE: [(&str, &str, f32, f32, f32); 10] = [
    // JF-Tent03 row and GA-Barracks01, the west and far-west ends of the camp.
    ("Harset.JaffaZone", "JF-Tent03", -154.86, -41.28, 77.744),
    ("Harset.JaffaZone", "GA-Barracks01", -198.09, -41.28, 84.35),
    // EM-Bunker00 and EM-Quartermaster01, the south and north ends of the camp.
    ("Harset.OpCoreZone", "EM-Bunker00", 202.08, -41.20, -9.04),
    (
        "Harset.OpCoreZone",
        "EM-Quartermaster01",
        188.514,
        -41.221,
        156.705,
    ),
    // The on-mesh GA-Bank00 / CA-Courtyard_Str00 pair.
    ("Harset.Bank", "GA-Bank00", -187.92, -41.44, 162.396),
    (
        "Harset.Bank",
        "CA-Courtyard_Str00",
        -184.51,
        -41.44,
        162.266,
    ),
    // Both HP-Brazier00 instances, the only two in the map.
    (
        "Harset.PetbeQuarters",
        "HP-Brazier00 east",
        -159.782,
        -28.25,
        232.308,
    ),
    (
        "Harset.PetbeQuarters",
        "HP-Brazier00 west",
        -170.078,
        -28.25,
        234.457,
    ),
    // The unique Goa'uld control-panel prop, and one tower pivot.
    (
        "Harset.ShieldControls",
        "GA-Viewscreens00",
        -88.34,
        -30.70,
        213.925,
    ),
    (
        "Harset.ShieldTower1",
        "GA-TowTall01",
        -226.04,
        -41.36,
        37.72,
    ),
];

/// Control coordinate for the navmesh guard: ring region 4's pad, a point the
/// shipped seed already stands a player-reachable entity on
/// (`ring_transport_regions.sql:29`). Without it, a mesh that failed to load
/// would make every `Mesh::Off` expectation vacuously true and the whole guard
/// would assert nothing. Same literals as
/// `chain_replay_tests::harset_space`'s control — identical f32 bit patterns to
/// the seed's fuller decimal form, per clippy's `excessive_precision`.
const RING4_PAD: Vector3 = Vector3 {
    x: -25.641,
    y: -67.828,
    z: 15.249,
};

fn harset_mesh() -> NavMesh {
    let path = std::path::Path::new("../../data/spaces/harset.nav");
    NavMesh::load(path).expect("load data/spaces/harset.nav")
}

/// Every placed row exists in world 57 with its exact tag and template, is
/// stationary, and carries a respawn delay.
///
/// `is_stationary` is not cosmetic here: the Jaffa Zone, the towers and the
/// palace terrace have no navmesh under them, so a mobile NPC there would
/// either stand still anyway or chase into a hole. D-H17 wants `respawn_secs`
/// on every row, including the props (where it can never fire).
#[tokio::test]
async fn world57_placement_rows_are_seeded_with_their_tags_and_templates() {
    let pool = require_db_or_skip!();

    for (spawn_id, tag, template_id, _) in PLACED {
        let row: Option<(String, i32, i32, bool, Option<i32>)> = sqlx::query_as(
            "SELECT tag, template_id, world_id, is_stationary, respawn_secs \
             FROM resources.spawnlist WHERE spawn_id = $1 AND tag IS NOT NULL",
        )
        .bind(spawn_id)
        .fetch_optional(&pool)
        .await
        .expect("query must succeed");

        let (db_tag, db_template, db_world, stationary, respawn) = row.unwrap_or_else(|| {
            panic!(
                "spawn {spawn_id} ({tag}) is missing from resources.spawnlist — placement \
                 pass B's block in db/resources/Worlds/Seed/spawnlist.sql was removed or \
                 renumbered"
            )
        });

        assert_eq!(db_tag, tag, "spawn {spawn_id} carries the wrong tag");
        assert_eq!(
            db_template, template_id,
            "spawn {spawn_id} ({tag}) points at the wrong entity template"
        );
        assert_eq!(
            db_world, HARSET,
            "spawn {spawn_id} ({tag}) is not in world 57"
        );
        assert!(
            stationary,
            "spawn {spawn_id} ({tag}) must be is_stationary: world 57's upper quarters have \
             no navmesh under them, so a mobile NPC there chases into a hole (GH1)"
        );
        assert!(
            respawn.is_some(),
            "spawn {spawn_id} ({tag}) must set respawn_secs (D-H17)"
        );
        let (lo, hi) = SPAWN_BLOCK;
        assert!(
            (lo..=hi).contains(&spawn_id),
            "spawn {spawn_id} is outside the {lo}-{hi} block the ledger reserves"
        );
    }
}

/// No spawnlist row in either shared-hub world stands a hostile template
/// (D-H03).
///
/// Deliberately zone-wide and deliberately joined through the DB rather than
/// read off [`PLACED`]. The mistake this exists to catch is a *placement*
/// mistake — reaching for the hostile twin of a talk-to NPC (templates
/// 221/222/223 exist precisely because `faction` is immutable at runtime) and
/// standing it in the shared hub. A version of this guard that looked up the
/// faction of the template id in [`PLACED`] instead of the template id in the
/// row would pass while the seed stood Grogan's hostile twin in the plaza,
/// which is exactly what a revert check of an earlier draft showed it doing.
///
/// `faction` is nullable and the DHD row (spawn 37, template 1) has it NULL, so
/// NULL is treated as "not hostile" rather than as a failure — the combat path
/// reads a missing faction as non-hostile too.
#[tokio::test]
async fn no_shared_hub_spawn_stands_a_hostile_template() {
    let pool = require_db_or_skip!();

    let offenders: Vec<(i32, i32, Option<String>, String)> = sqlx::query_as(
        "SELECT s.spawn_id, s.template_id, s.tag, t.template_name \
         FROM resources.spawnlist s \
         JOIN resources.entity_templates t ON t.template_id = s.template_id \
         WHERE s.world_id = ANY($1) AND t.faction = $2 \
         ORDER BY s.spawn_id",
    )
    .bind(SHARED_HUB_WORLDS)
    .bind(i32::from(HOSTILE_FACTION))
    .fetch_all(&pool)
    .await
    .expect("query must succeed");

    assert!(
        offenders.is_empty(),
        "spawn rows in the shared hub worlds {SHARED_HUB_WORLDS:?} stand templates with \
         faction {HOSTILE_FACTION}: {offenders:?}. D-H03 forbids hostiles in worlds 57 and \
         68; `faction` cannot be changed at runtime, so the fix is to point the row at the \
         non-hostile twin, not to flip a flag."
    );

    // Control: the hostile twins exist, so the query above is not silently
    // matching nothing because `HOSTILE_FACTION` drifted away from the seed.
    let hostile_templates: i64 =
        sqlx::query_scalar("SELECT count(*) FROM resources.entity_templates WHERE faction = $1")
            .bind(i32::from(HOSTILE_FACTION))
            .fetch_one(&pool)
            .await
            .expect("query must succeed");
    assert!(
        hostile_templates > 0,
        "no template in the seed has faction {HOSTILE_FACTION} — either the constant no \
         longer matches the data or the templates did not load, and the assertion above \
         would pass for the wrong reason"
    );
}

/// Every `interact_tag` key a merged world-57 chain dispatches on has a
/// spawnlist row in world 57 carrying exactly that tag.
///
/// This is the linkage guard, and the reason it queries `content_triggers`
/// instead of comparing two constants is that a hand-written expectation list
/// cannot catch the error it is meant to catch. `fire_interact_tag` matches the
/// tag byte-exactly; a chain on `Harset_FormerRaJaffa2` and a spawn row on
/// `Harset_FormerRaJaffa_2` load cleanly, dispatch nothing, and the mission
/// simply never advances.
///
/// Scoped to chains that carry a `world eq 57` condition so it does not demand
/// world-68 rows (packet H12) or instance-only `spawn_entity` tags (H03), and
/// tolerant of tags owned by other packets: an unplaced tag from a chain this
/// pass does not own is reported with the chain id so the coordinator can route
/// it, rather than failing a packet that could not have fixed it.
#[tokio::test]
async fn world57_interact_tags_from_merged_chains_all_have_a_spawn_row() {
    let pool = require_db_or_skip!();

    let expected: Vec<(i32, String)> = sqlx::query_as(
        "SELECT DISTINCT t.chain_id, t.event_key \
         FROM resources.content_triggers t \
         JOIN resources.content_conditions c ON c.chain_id = t.chain_id \
         WHERE t.event_type = 'interact_tag' \
           AND c.condition_type = 'world' AND c.target_id = $1 AND c.operator = 'eq' \
         ORDER BY t.chain_id, t.event_key",
    )
    .bind(HARSET)
    .fetch_all(&pool)
    .await
    .expect("query must succeed");

    assert!(
        !expected.is_empty(),
        "no world-57 `interact_tag` chain found at all — the harset chain seeds did not \
         load, and this guard would otherwise pass vacuously"
    );

    let placed_tags: Vec<String> = sqlx::query_scalar(
        "SELECT tag FROM resources.spawnlist WHERE world_id = $1 AND tag IS NOT NULL",
    )
    .bind(HARSET)
    .fetch_all(&pool)
    .await
    .expect("query must succeed");

    let ours: Vec<&str> = PLACED.iter().map(|(_, tag, ..)| *tag).collect();
    let mut missing_ours = Vec::new();
    let mut missing_elsewhere = Vec::new();
    for (chain_id, key) in &expected {
        if placed_tags.iter().any(|t| t == key) {
            continue;
        }
        if ours.contains(&key.as_str()) {
            missing_ours.push(format!("{key} (chain {chain_id})"));
        } else {
            missing_elsewhere.push(format!("{key} (chain {chain_id})"));
        }
    }

    assert!(
        missing_ours.is_empty(),
        "world-57 chains dispatch on tags this pass claims to place, but no spawnlist row \
         in world 57 carries them: {missing_ours:?}. `fire_interact_tag` matches byte-exactly, \
         so this is a silent dead mission, not a warning."
    );
    assert!(
        missing_elsewhere.is_empty(),
        "world-57 chains dispatch on `interact_tag` keys with no spawn row, and they are not \
         placement pass B's: {missing_elsewhere:?}. Route them to the owning packet (see \
         docs/analysis/harset-rebuild/worknotes/harset-tags.md) — the chain is dead until a \
         row carries the tag."
    );
}

/// Each placed coordinate's relationship to `harset.nav` is the one the ledger
/// records, in both directions.
///
/// See the module docs for why this is an exception table rather than a blanket
/// on-mesh assertion. The coordinates come from the DB so that correcting a
/// seed row is a one-file change; only the on/off verdict lives in code, and
/// that is the thing the ledger is asserting.
#[tokio::test]
async fn world57_placements_match_their_recorded_navmesh_verdict() {
    let pool = require_db_or_skip!();
    let mesh = harset_mesh();

    assert!(
        mesh.is_point_valid(&RING4_PAD),
        "ring region 4's pad must be on-mesh — if this fails the mesh did not load and every \
         Mesh::Off expectation below would pass for the wrong reason"
    );

    for (spawn_id, tag, _, expected) in PLACED {
        let (x, y, z): (f32, f32, f32) =
            sqlx::query_as("SELECT x, y, z FROM resources.spawnlist WHERE spawn_id = $1")
                .bind(spawn_id)
                .fetch_one(&pool)
                .await
                .expect("the placed row must exist — see the presence guard");

        let pos = Vector3::new(x, y, z);
        let on_mesh = mesh.is_point_valid(&pos);

        match expected {
            Mesh::On => assert!(
                on_mesh,
                "spawn {spawn_id} ({tag}) at {pos:?} is recorded as ON the harset.nav mesh \
                 but `is_point_valid` refuses it. Either the coordinate moved off the hub \
                 component or the mesh changed; re-run the METHOD.md checks and update the \
                 ledger row."
            ),
            Mesh::Off => assert!(
                !on_mesh,
                "spawn {spawn_id} ({tag}) at {pos:?} is recorded as OFF the harset.nav mesh \
                 and is now ON it. That is good news, not a regression — but it invalidates \
                 the reason this row is `is_stationary` and the reason the ledger calls it \
                 low-confidence. Update the ledger row and this table, and reconsider \
                 `is_stationary` now that NPC pathing can reach it."
            ),
        }
    }
}

/// The eight new named regions load as `AreaSet`s for world `Harset`, reach
/// exactly four points, and enclose the landmarks they were sized to.
///
/// Four points is not a style rule: `is_point_in_region` returns `false` for
/// any other count, so a five-corner box or a cylinder that lost its radius is
/// a volume that can never be entered — and nothing else in the stack says so.
#[tokio::test]
async fn world57_named_regions_load_and_enclose_their_landmarks() {
    let pool = require_db_or_skip!();

    let regions = load_regions_from_db(&pool)
        .await
        .expect("region load must succeed");

    for (set_id, name, shape) in REGIONS {
        let (lo, hi) = REGION_BLOCK;
        assert!(
            (lo..=hi).contains(&set_id),
            "point set {set_id} is outside the {lo}-{hi} block the ledger reserves"
        );

        let db_shape: String =
            sqlx::query_scalar("SELECT shape FROM resources.point_sets WHERE set_id = $1")
                .bind(set_id)
                .fetch_optional(&pool)
                .await
                .expect("query must succeed")
                .unwrap_or_else(|| {
                    panic!(
                        "point set {set_id} ({name}) is missing — placement pass B's block in \
                         db/resources/Events/Seed/point_sets.sql was removed"
                    )
                });
        assert_eq!(db_shape, shape, "point set {set_id} ({name}) changed shape");

        let region = regions
            .iter()
            .find(|r| r.set_id == set_id)
            .unwrap_or_else(|| {
                panic!(
                    "point set {set_id} ({name}) did not come back from \
                     `load_regions_from_db` — it must be `type = 'AreaSet'` and its world \
                     must join to `resources.worlds`"
                )
            });

        assert_eq!(region.name, name, "point set {set_id} was renamed");
        assert_eq!(
            region.world_name, HARSET_WORLD_NAME,
            "region {name} must belong to world {HARSET}"
        );
        assert_eq!(
            region.points.len(),
            4,
            "region {name} resolves {} points after the loader's cylinder workaround; \
             `is_point_in_region` fails closed on anything but 4, so this volume could \
             never be entered",
            region.points.len(),
        );
    }

    for (region_name, label, x, y, z) in MUST_ENCLOSE {
        let region = regions
            .iter()
            .find(|r| r.name == region_name)
            .expect("region asserted present above");
        assert!(
            is_point_in_region(&region.points, [x, y, z]),
            "{region_name} does not contain {label} at ({x}, {y}, {z}) — the volume was \
             sized to enclose that landmark's instance position from the cooked map"
        );
    }
}

/// Every dotted `Harset.` region in the seed belongs to world 57 and is an
/// `AreaSet`.
///
/// Cheap, and it covers the next hand that adds `Harset.Bar` or
/// `Harset.HoldingPens` on the reserved ids: the world-prefix-and-dot naming is
/// what makes the cross-file region-key linter see the row at all, and a
/// `Harset.`-prefixed set pointed at another world is a region that silently
/// never fires.
#[tokio::test]
async fn dotted_harset_regions_all_belong_to_world_57() {
    let pool = require_db_or_skip!();

    let rows: Vec<(i32, String, String, i32)> = sqlx::query_as(
        "SELECT set_id, name, type, world_id FROM resources.point_sets \
         WHERE name LIKE 'Harset.%' ORDER BY set_id",
    )
    .fetch_all(&pool)
    .await
    .expect("query must succeed");

    // Not a count assertion: presence is
    // `world57_named_regions_load_and_enclose_their_landmarks`'s job, and a
    // `>= REGIONS.len()` check here passes even with one of them deleted,
    // because `Harset.Stargate` (1001) and `Harset.CommandCenterTransition`
    // (2078) make up the difference. This only refuses to pass on an empty
    // result, which would mean the point-set seed did not load.
    assert!(
        !rows.is_empty(),
        "no `Harset.`-prefixed point set loaded at all — the seed did not load and the \
         per-row checks below would pass vacuously"
    );

    for (set_id, name, kind, world_id) in rows {
        assert_eq!(
            kind, "AreaSet",
            "point set {set_id} ({name}) is `{kind}`; `spawner/regions.rs` loads only \
             `AreaSet`, so an `enter_region` on it can never fire"
        );
        assert_eq!(
            world_id, HARSET,
            "point set {set_id} ({name}) carries the `Harset.` prefix but world {world_id}"
        );
    }
}
