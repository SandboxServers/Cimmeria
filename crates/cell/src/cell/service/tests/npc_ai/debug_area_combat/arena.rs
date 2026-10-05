//! Z6, the NPC-vs-NPC arena (D-DA8) on the real pit floor: fight 1 (faction 3
//! against faction 10, a player can join) and fight 2 (faction 27 against 29,
//! spectators only). Every assertion runs the production Idle scan (#1009):
//! the grid query, the faction table, the 4 u band, the aggro radius and the
//! occluder line of sight.

use std::collections::HashSet;

use super::*;

const PRAXIS: &str = "DebugArea_Arena_Praxis";
const NID: &str = "DebugArea_Arena_NID";
const GREEN: &str = "DebugArea_Arena_Green";
const YELLOW: &str = "DebugArea_Arena_Yellow";

/// Where spectators watch from, all inside the 150 u AoI of the squads. The
/// first four are on the crater floor 26-40 u above the pit (occluder
/// heights); the rest are on the pit's own level or its slope, inside the
/// 4 u band but 40+ u from the nearest NID guard: the far east slope, the
/// south-west ramp, and right beside each side of fight 2 (where a tester
/// stands to watch it or loot a corpse). Measured with `occluder_extract
/// probe` on ihpet_crater_light.occ and `nav_inspect` on the .nav.
const SPECTATOR_SPOTS: [[f32; 3]; 8] = [
    [185.0, -6.8, -725.0],
    [250.0, -0.4, -670.0],
    [280.0, 2.7, -673.0],
    [250.0, -15.9, -790.0],
    [310.0, -29.0, -725.0],
    [198.0, -32.6, -755.0],
    [211.0, -33.0, -712.0],
    [204.0, -32.2, -738.0],
];

/// The pit floor's authored height (`spawnlist_debug_area_combat.sql`).
const PIT_Y: f32 = -32.4;

fn ids(rows: &[&SpawnRecord]) -> HashSet<u32> {
    rows.iter().map(|r| npc_id(r)).collect()
}

/// **Content guard.** Each arena row, watched from the crater floor, engages a
/// member of the opposing squad and nothing else: Praxis and NID guards fight
/// each other, the Green and Yellow pairs fight each other, and no row picks
/// a target outside its fight (no chaining into the other fight or another
/// zone). Fails if a squad moves out of the other's aggro radius, a template
/// loses its radius or faction, or the floor between them stops being clear.
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

/// **Content guard.** A spectator on the rim or the far slope is never pulled:
/// every arena row picks an NPC of the other side, never the player. The rim
/// spots are out of the 4 u band, the slope spots out of the 30 u radius.
/// Fails if the NID squad moves toward the ramp or its radius grows past the
/// pit floor.
#[tokio::test]
async fn spectators_on_the_rim_and_the_slope_are_never_pulled() {
    let records = world_records();
    for spot in SPECTATOR_SPOTS {
        let rim = spot[1] - PIT_Y > 4.0;
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

/// A player who walks into the pit beside the NID squad is engaged by it (he
/// joins on the Praxis side), and the Praxis squad never targets him.
#[tokio::test]
async fn a_player_in_the_pit_is_fought_by_the_nid_squad_only() {
    let records = world_records();
    let nid = da04(&records, NID);
    let near = nid[0];
    let spot = [near.x + 12.0, PIT_Y, near.z];
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

/// Fight 2 stands 40 u or more from every NID guard (review finding 1), so a
/// tester who walks up to watch it, or to check a corpse's loot cursor, is
/// outside the guards' 30 u aggro radius whatever round fight 1 is in.
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
            assert!(d >= 40.0, "{:?} is {d:.1} u from {:?}", r.tag, g.tag);
        }
    }
}
