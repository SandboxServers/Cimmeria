//! The arena's pull map (DA-F2, #1244 review finding 2): every navmesh
//! point around the east shelf where a NID guard engages a lone player, run
//! through the production Idle scan on the real mesh and occluder.
//!
//! Fixed spectator spots miss pull zones that leak through gaps in the
//! shelf's ruin walls: the first DA-F2 layout pulled a tester in the fight-2
//! room's south-west corner past the long wall's west end, and every
//! fixed-point guard passed. Technique:
//! `.claude/agent-memory/testing-validation-engineer/technique_pull_map_probe.md`.

use cimmeria_common::Vector3;

use super::*;

/// The fight-1 strip a player walks into to join: between the long
/// east-west wall (z -736) and the south edge (z -749, with its lip), on the
/// shelf's level.
fn in_join_strip(p: &Vector3) -> bool {
    (318.0..=400.0).contains(&p.x)
        && (-752.0..=-735.0).contains(&p.z)
        && (p.y - SHELF_Y).abs() <= 1.5
}

const SHELF_Y: f32 = -11.12;

/// Height hints for the sweep: the slope below the south edge, the shelf,
/// the east approach and the terrain north of the shelf.
const HINTS: [f32; 4] = [-18.0, -11.1, -6.5, 7.0];

/// **Content guard (DA-F2).** A lone player stepped over a 2 u navmesh grid
/// around the shelf (x 300-420, z -790..-660, four height hints) is engaged
/// by a NID guard only inside the fight-1 strip. Anywhere else a spectator
/// can stand (the fight-2 room, the terrain above, the slope below, the east
/// approach and its ring pad) pulls nothing. The Praxis rows are removed
/// first: the scan picks the closest hostile, so an NPC enemy would hide a
/// player pull. Fails if a guard's radius or position lets its scan reach
/// past the walls, as the first layout's NID3 did at x 328-336,
/// z -730..-734.
#[tokio::test]
async fn the_nid_squad_pulls_a_player_only_inside_the_fight_strip() {
    let records: Vec<SpawnRecord> = world_records()
        .into_iter()
        .filter(|r| {
            !r.tag
                .as_deref()
                .is_some_and(|t| t.starts_with("DebugArea_Arena_Praxis"))
        })
        .collect();
    let Some(mut mgr) = scene(&records) else {
        return;
    };
    let mesh = navmesh().expect("checked by scene");
    let nid: Vec<u32> = da04(&records, "DebugArea_Arena_NID")
        .iter()
        .map(|r| npc_id(r))
        .collect();
    assert_eq!(nid.len(), 3, "fixture: three NID guards");
    add_player(&mut mgr, PLAYER, [400.0, -6.5, -700.0]);

    let mut in_strip = 0;
    let mut leaks = Vec::new();
    let mut probed = 0;
    let mut x = 300.0f32;
    while x <= 420.0 {
        let mut z = -790.0f32;
        while z <= -660.0 {
            let mut seen: Vec<f32> = Vec::new();
            for hint in HINTS {
                let Some(y) = mesh.get_height_near(x, hint, z) else {
                    continue;
                };
                if seen.iter().any(|s| (s - y).abs() < 0.5) {
                    continue;
                }
                seen.push(y);
                probed += 1;
                mgr.update_position_preserving_facing(PLAYER, [x, y, z], [0.0; 3]);
                let _ = mgr.compute_aoi_changes();
                let p = Vector3::new(x, y, z);
                for &g in &nid {
                    reset_idle(&mut mgr, nid.iter().copied());
                    let (_, targets) = scan(&mut mgr, g).await;
                    if targets.contains(&PLAYER) {
                        if in_join_strip(&p) {
                            in_strip += 1;
                        } else {
                            let gp = mgr.get_entity(g).unwrap().position;
                            leaks.push(format!(
                                "guard {g} pulls the player at ({x}, {y:.2}, {z}), {:.1} u",
                                (gp.x - x).hypot(gp.z - z)
                            ));
                        }
                    }
                }
            }
            z += 2.0;
        }
        x += 2.0;
    }
    assert!(
        probed > 2000,
        "fixture: the sweep found the mesh ({probed} points)"
    );
    assert!(in_strip > 0, "control: the strip still pulls (joinable)");
    assert!(
        leaks.is_empty(),
        "{} pulls outside the strip: {leaks:#?}",
        leaks.len()
    );
}
