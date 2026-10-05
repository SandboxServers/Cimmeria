//! Z6, the NPC-vs-NPC arena (D-DA8) on the real east shelf (DA-F2 moved it off
//! the pit's water plane): fight 1 (faction 3 against faction 10, a player can
//! join) and fight 2 (faction 27 against 29, spectators only). Every assertion runs the production Idle scan (#1009):
//! the grid query, the faction table, the 4 u band, the aggro radius and the
//! occluder line of sight.

use std::collections::HashSet;

use super::*;

const PRAXIS: &str = "DebugArea_Arena_Praxis";
const NID: &str = "DebugArea_Arena_NID";
const GREEN: &str = "DebugArea_Arena_Green";
const YELLOW: &str = "DebugArea_Arena_Yellow";

/// Where spectators watch from, all inside the 150 u AoI of the squads
/// (occluder terrain heights where the navmesh agrees, DA-F2 survey). The
/// first three are on the terrain north of the shelf, 17-20 u above it; the
/// fourth is the east approach, 4.5 u above it and 69 u from the nearest NID
/// guard; the fifth is the slope below the shelf's south edge, 7 u below it;
/// the last two stand on the shelf right beside each side of fight 2 (where a
/// tester watches it or loot-checks a corpse), 49 u or more from every NID
/// guard.
const SPECTATOR_SPOTS: [[f32; 3]; 7] = [
    [354.0, 8.52, -654.0],
    [330.0, 6.5, -662.0],
    [306.0, 5.98, -658.0],
    [410.0, -6.64, -698.0],
    [350.0, -18.19, -766.0],
    [340.0, -11.12, -690.0],
    [366.0, -11.12, -690.0],
];

/// The east shelf's terrain height (`spawnlist_debug_area_combat.sql`).
const SHELF_Y: f32 = -11.12;

fn ids(rows: &[&SpawnRecord]) -> HashSet<u32> {
    rows.iter().map(|r| npc_id(r)).collect()
}

/// **Content guard.** Each arena row, watched from the crater floor, engages a
/// member of the opposing squad and nothing else: Praxis and NID guards fight
/// each other, the Green and Yellow pairs fight each other, and no row picks
/// a target outside its fight (no chaining into the other fight or another
/// zone). Fails if a squad moves out of the other's aggro radius, a template
/// loses its radius or faction, or the ground between them stops being clear.
#[tokio::test]
async fn every_arena_row_engages_the_opposing_squad_when_watched() {
    let records = world_records();
    let fights = [
        (PRAXIS, NID),
        (NID, PRAXIS),
        (GREEN, YELLOW),
        (YELLOW, GREEN),
    ];
    let Some(mut mgr) = scene(&records) else {
        return;
    };
    add_player(&mut mgr, PLAYER, SPECTATOR_SPOTS[0]);
    let arena = ids(&da04(&records, "DebugArea_Arena_"));
    for (side, enemy) in fights {
        let rows = da04(&records, side);
        let enemies = ids(&da04(&records, enemy));
        assert!(
            rows.len() >= 2 && !enemies.is_empty(),
            "fixture: {side} vs {enemy}"
        );
        for r in rows {
            reset_idle(&mut mgr, arena.iter().copied());
            assert!(
                !mgr.get_witnesses_of(npc_id(r)).is_empty(),
                "fixture: {:?} is watched",
                r.tag
            );
            let (engaged, targets) = scan(&mut mgr, npc_id(r)).await;
            assert!(
                engaged && !targets.is_empty() && targets.iter().all(|t| enemies.contains(t)),
                "{:?} must engage only {enemy}*: engaged={engaged}, targets={targets:?}",
                r.tag
            );
        }
    }
}

/// The fight is witness-gated (#1009): with nobody in AoI no arena row starts
/// anything, so an empty Debug Area costs only idle ticks.
#[tokio::test]
async fn no_arena_row_engages_while_nobody_watches() {
    let records = world_records();
    let Some(mut mgr) = scene(&records) else {
        return;
    };
    for r in da04(&records, "DebugArea_Arena_") {
        let (engaged, targets) = scan(&mut mgr, npc_id(r)).await;
        assert!(
            !engaged && targets.is_empty(),
            "{:?} engaged {targets:?} with no witness",
            r.tag
        );
    }
}

/// **Content guard.** A spectator above, below or beside the shelf is never
/// pulled: every arena row picks an NPC of the other side, never the player.
/// The spots above and below are out of the 4 u band, the ones on the shelf
/// out of the 30 u radius. Fails if the NID squad moves toward fight 2 or its
/// radius grows past the shelf's west half.
#[tokio::test]
async fn spectators_on_the_rim_and_the_slope_are_never_pulled() {
    let records = world_records();
    for spot in SPECTATOR_SPOTS {
        let rim = (spot[1] - SHELF_Y).abs() > 4.0;
        let nid_dist = da04(&records, NID)
            .iter()
            .map(|r| (r.x - spot[0]).hypot(r.z - spot[2]))
            .fold(f32::INFINITY, f32::min);
        assert!(
            rim || nid_dist > 30.0,
            "fixture: {spot:?} is a spectator spot, not inside the fight"
        );
        let Some(mut mgr) = scene(&records) else {
            return;
        };
        add_player(&mut mgr, PLAYER, spot);
        let arena = ids(&da04(&records, "DebugArea_Arena_"));
        for r in da04(&records, "DebugArea_Arena_") {
            reset_idle(&mut mgr, arena.iter().copied());
            let (_, targets) = scan(&mut mgr, npc_id(r)).await;
            assert!(
                !targets.contains(&PLAYER),
                "{:?} pulled the spectator at {spot:?}",
                r.tag
            );
        }
    }
}

/// A player who walks onto the shelf beside the NID squad is engaged by it (he
/// joins on the Praxis side), and the Praxis squad never targets him.
#[tokio::test]
async fn a_player_on_the_shelf_is_fought_by_the_nid_squad_only() {
    let records = world_records();
    let nid = da04(&records, NID);
    let near = nid[0];
    let spot = [near.x + 12.0, SHELF_Y, near.z];
    let Some(mut mgr) = scene(&records) else {
        return;
    };
    add_player(&mut mgr, PLAYER, spot);
    let arena = ids(&da04(&records, "DebugArea_Arena_"));
    for r in da04(&records, "DebugArea_Arena_") {
        reset_idle(&mut mgr, arena.iter().copied());
        let (_, targets) = scan(&mut mgr, npc_id(r)).await;
        let is_nid = r.tag.as_deref().is_some_and(|t| t.starts_with(NID));
        if r.spawn_id == near.spawn_id {
            assert_eq!(
                targets,
                vec![PLAYER],
                "{:?} engages the player 12 u away",
                r.tag
            );
        } else if !is_nid {
            assert!(
                !targets.contains(&PLAYER),
                "{:?} (not hostile to players) targeted the player",
                r.tag
            );
        }
    }
}

/// Fight 2 stands 39 u or more from every NID guard (DA-04 review finding 1;
/// 39 on the shelf, DA-F2), so a tester who walks up to watch it, or to check a
/// corpse's loot cursor, is outside the guards' 30 u aggro radius whatever
/// round fight 1 is in.
#[test]
fn the_spectator_fight_stands_clear_of_the_nid_squad() {
    let records = world_records();
    let nid = da04(&records, NID);
    for r in da04(&records, GREEN)
        .into_iter()
        .chain(da04(&records, YELLOW))
    {
        for g in &nid {
            let d = (r.x - g.x).hypot(r.z - g.z);
            assert!(d >= 39.0, "{:?} is {d:.1} u from {:?}", r.tag, g.tag);
        }
    }
}

/// **Regression guard (DA-F2, live client 2026-10-05).** Every arena row stands
/// where a player can stand: its grounded position has the occluder's terrain
/// within 0.5 u under it. The pit floor the arena first used is a water
/// collision plane (geometry at y -33.28) over a lakebed 10-25 u lower; the
/// navmesh lies on the water, so the squads stood on it while a player who
/// walked in sank to y -52 and saw them floating 19 m overhead. Fails if a row
/// goes back onto the water plane, or anywhere the navmesh floats off the
/// terrain.
#[tokio::test]
async fn every_arena_row_stands_on_terrain_a_player_can_reach() {
    let records = world_records();
    let Some(mgr) = scene(&records) else {
        return;
    };
    let occ = cimmeria_occluder::PagedOccluder::load(&repo("data/spaces/ihpet_crater_light.occ"))
        .expect("load ihpet_crater_light.occ");
    let mut bad = Vec::new();
    for r in da04(&records, "DebugArea_Arena_") {
        let p = mgr.get_entity(npc_id(r)).unwrap().position;
        let col = occ.column(p.x, p.z);
        let on_terrain = col.iter().any(|(kind, _, top)| {
            matches!(kind, cimmeria_occluder::LayerKind::Terrain) && (top - p.y).abs() <= 0.5
        });
        if !on_terrain {
            bad.push(format!("{:?} at {p:?}: column {col:?}", r.tag));
        }
    }
    assert!(bad.is_empty(), "arena rows off solid terrain: {bad:#?}");
}

/// The shelf is reachable on foot, so fight 1 stays joinable and fight 2
/// watchable from up close: the navmesh routes from the Z1 arrival in the
/// south compound, and from the east approach the UAT row starts at, to the
/// spot between the squads where the NID guards take a player
/// (`a_player_on_the_shelf_is_fought_by_the_nid_squad_only`) and to the
/// spectator spot beside fight 2. A partial route means the shelf
/// is an island a tester cannot walk onto.
#[test]
fn the_shelf_is_reachable_on_foot() {
    let Some(mesh) = navmesh() else {
        return;
    };
    let at = |x: f32, y: f32, z: f32| {
        let h = mesh.get_height_near(x, y, z).expect("on the navmesh");
        Vector3::new(x, h, z)
    };
    let starts = [at(251.0, 7.11, -962.0), at(420.0, -6.0, -741.0)];
    let ends = [at(366.0, SHELF_Y, -738.0), at(340.0, SHELF_Y, -690.0)];
    for from in starts {
        for to in ends {
            let outcome = mesh.find_path(&from, &to);
            let status = outcome.status.label();
            let end = outcome
                .into_waypoints()
                .and_then(|w| w.last().map(|e| e.distance_to(&to)));
            assert!(
                status == "ok" && end.is_some_and(|d| d < 2.0),
                "{from:?} -> {to:?}: {status}, ends {end:?} short"
            );
        }
    }
}
