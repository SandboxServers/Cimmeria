//! Plan rollover-verified Livewire clicks from a server board and the installed hit atlas.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::livewire_atlas::LivewireAtlas;

type Point = (i32, i32);

#[derive(Clone, Debug, Deserialize)]
struct Board {
    state: String,
    goal_total: u32,
    wires: Vec<Wire>,
}

#[derive(Clone, Debug, Deserialize)]
struct Wire {
    name: String,
    lib: String,
    kind: String,
    depth: u32,
    x: f64,
    y: f64,
    cut: bool,
}

#[derive(Debug, Serialize)]
pub struct ClickPlan {
    pub status: &'static str,
    pub goal_total: u32,
    pub clicks: Vec<PlannedClick>,
    pub unattached: Vec<String>,
    pub atlas_sha256: String,
}

#[derive(Debug, Serialize)]
pub struct PlannedClick {
    pub name: String,
    pub kind: String,
    pub lib: String,
    pub depth: u32,
    pub candidates: Vec<Candidate>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Candidate {
    pub x: i32,
    pub y: i32,
    pub clearance: i32,
}

#[derive(Debug)]
struct Shape {
    wire: Wire,
    cells: BTreeSet<Point>,
}

fn grid(value: f64) -> Result<i32, String> {
    if !value.is_finite() || value.abs() > 10_000.0 {
        return Err("Livewire board has an invalid wire coordinate".into());
    }
    Ok((value / 5.0).round() as i32 * 5)
}

fn blocked((x, y): Point) -> bool {
    !(445..=1270).contains(&x) || !(110..=890).contains(&y)
}

fn top(shapes: &[Shape], cut: &BTreeSet<String>, point: Point) -> Option<usize> {
    shapes
        .iter()
        .enumerate()
        .filter(|(_, shape)| !cut.contains(&shape.wire.name) && shape.cells.contains(&point))
        .max_by_key(|(_, shape)| shape.wire.depth)
        .map(|(index, _)| index)
}

fn clearance(shapes: &[Shape], cut: &BTreeSet<String>, target: usize, point: Point) -> i32 {
    let mut radius = 0;
    for next in [5, 10, 15, 20] {
        let square_clear = (-next..=next).step_by(5).all(|dx| {
            (-next..=next).step_by(5).all(|dy| {
                let probe = (point.0 + dx, point.1 + dy);
                !blocked(probe)
                    && shapes[target].cells.contains(&probe)
                    && top(shapes, cut, probe) == Some(target)
            })
        });
        if !square_clear {
            break;
        }
        radius = next;
    }
    radius
}

fn candidates(shapes: &[Shape], cut: &BTreeSet<String>, target: usize) -> (usize, Vec<Candidate>) {
    let mut exposed: Vec<Candidate> = shapes[target]
        .cells
        .iter()
        .copied()
        .filter(|&point| !blocked(point) && top(shapes, cut, point) == Some(target))
        .map(|point| Candidate {
            x: point.0,
            y: point.1,
            clearance: clearance(shapes, cut, target, point),
        })
        .collect();
    let exposed_count = exposed.len();
    exposed.sort_by_key(|point| (-point.clearance, point.x, point.y));
    let mut chosen: Vec<Candidate> = Vec::new();
    for point in &exposed {
        if chosen
            .iter()
            .all(|other| (point.x - other.x).abs().max((point.y - other.y).abs()) >= 15)
        {
            chosen.push(point.clone());
            if chosen.len() == 5 {
                return (exposed_count, chosen);
            }
        }
    }
    for point in exposed {
        if chosen.len() == 5 {
            break;
        }
        if !chosen
            .iter()
            .any(|other| other.x == point.x && other.y == point.y)
        {
            chosen.push(point);
        }
    }
    (exposed_count, chosen)
}

fn make_click(shape: &Shape, candidates: Vec<Candidate>) -> PlannedClick {
    PlannedClick {
        name: shape.wire.name.clone(),
        kind: shape.wire.kind.clone(),
        lib: shape.wire.lib.clone(),
        depth: shape.wire.depth,
        candidates,
    }
}

/// Plan each remaining goal from the server's current board. Every candidate
/// is topmost at the point where its step executes; the caller must confirm
/// that claim with a SmartFox `processover` before clicking.
pub fn plan(board: &Value, atlas: &LivewireAtlas) -> Result<ClickPlan, String> {
    let board: Board = serde_json::from_value(board.clone())
        .map_err(|error| format!("invalid Livewire board: {error}"))?;
    if board.state != "ready" && board.state != "running" {
        return Err(format!("Livewire board is not active: {}", board.state));
    }
    if board.goal_total == 0 || board.wires.len() > 128 {
        return Err("Livewire board has invalid wire counts".into());
    }
    let mut unattached = Vec::new();
    let mut shapes = Vec::new();
    let mut seen = BTreeSet::new();
    for wire in board.wires {
        if !seen.insert(wire.name.clone()) {
            return Err(format!("duplicate Livewire wire: {}", wire.name));
        }
        let Some(library) = atlas.library(&wire.lib) else {
            if wire.kind == "goal" && !wire.cut {
                return Err(format!(
                    "goal {} has no hit atlas library {}",
                    wire.name, wire.lib
                ));
            }
            unattached.push(wire.name);
            continue;
        };
        let cells = library
            .points
            .iter()
            .map(|[x, y]| Ok((grid(wire.x + f64::from(*x))?, grid(wire.y + f64::from(*y))?)))
            .collect::<Result<BTreeSet<_>, String>>()?;
        shapes.push(Shape { wire, cells });
    }
    let goal_count = shapes
        .iter()
        .filter(|shape| shape.wire.kind == "goal")
        .count();
    if goal_count != board.goal_total as usize {
        return Err("Livewire board goal count differs from its wires".into());
    }
    let mut cut: BTreeSet<String> = shapes
        .iter()
        .filter(|shape| shape.wire.cut)
        .map(|shape| shape.wire.name.clone())
        .collect();
    let mut clicks = Vec::new();
    for _ in 0..shapes.len() {
        let mut goals: Vec<(usize, usize, Vec<Candidate>)> = shapes
            .iter()
            .enumerate()
            .filter(|(_, shape)| shape.wire.kind == "goal" && !cut.contains(&shape.wire.name))
            .map(|(index, shape)| {
                let (exposed, points) = candidates(&shapes, &cut, index);
                let visible_band = shape.cells.iter().filter(|&&point| !blocked(point)).count();
                (index, visible_band - exposed, points)
            })
            .collect();
        if goals.is_empty() {
            return Ok(ClickPlan {
                status: "planned",
                goal_total: board.goal_total,
                clicks,
                unattached,
                atlas_sha256: atlas.movie_sha256.clone(),
            });
        }
        goals.sort_by_key(|(index, occluded, _)| {
            (
                *occluded,
                shapes[*index].wire.depth,
                shapes[*index].wire.name.clone(),
            )
        });
        if let Some((index, _, points)) = goals.iter().find(|(_, _, points)| !points.is_empty()) {
            clicks.push(make_click(&shapes[*index], points.clone()));
            cut.insert(shapes[*index].wire.name.clone());
            continue;
        }
        let mut coverage = BTreeMap::<usize, usize>::new();
        for (goal, _, _) in &goals {
            for &point in &shapes[*goal].cells {
                if blocked(point) {
                    continue;
                }
                if let Some(index) = top(&shapes, &cut, point) {
                    if matches!(shapes[index].wire.kind.as_str(), "obstacle" | "moving") {
                        *coverage.entry(index).or_default() += 1;
                    }
                }
            }
        }
        let target = coverage
            .into_iter()
            .filter(|(index, _)| !candidates(&shapes, &cut, *index).1.is_empty())
            .max_by_key(|(_, cells)| *cells)
            .map(|(index, _)| index)
            .ok_or_else(|| "goal_fully_occluded".to_string())?;
        clicks.push(make_click(
            &shapes[target],
            candidates(&shapes, &cut, target).1,
        ));
        cut.insert(shapes[target].wire.name.clone());
    }
    Err("Livewire plan exceeded wire count".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supervisor::livewire_atlas::HitLibrary;
    use cimmeria_minigame::minigame::game::create_game;
    use cimmeria_minigame::minigame::session::MinigameSession;

    fn atlas() -> LivewireAtlas {
        let mut libraries = BTreeMap::new();
        for (base_index, base) in [
            "gBlackGreen",
            "gGreenBlack",
            "gRedSilver",
            "oBlack",
            "oGreen",
            "oRed",
            "mBlackStripe",
            "mOrangeStripe",
            "mPurpleStripe",
            "mYellowStripe",
            "pBronze",
            "pGray",
            "pGreen",
            "pRust",
        ]
        .into_iter()
        .enumerate()
        {
            for variant in 1..=4 {
                // A 900-stage-unit winding stroke with thickness and crossing
                // variants, matching the atlas scale without shipping client art.
                // Each of the 56 libraries gets its own silhouette: a phase in
                // 0..41, with the sawtooth mirrored past 41. Two libraries with
                // one shape would let an uncuttable playfield wire hide a goal
                // in its lane exactly, which real client art never does.
                let shape = base_index as i32 * 4 + variant - 1;
                let (phase, slope) = (shape % 41, if shape < 41 { 1 } else { -1 });
                let points = (0..=180)
                    .flat_map(|x| {
                        let wave = slope * ((x + phase) % 41 - 20) * 5;
                        [-5, 0, 5].map(move |dy| [x * 5, wave + dy])
                    })
                    .collect();
                libraries.insert(
                    format!("{base}{variant}"),
                    HitLibrary {
                        bounds_twips: [0.0, 18_000.0, -2_200.0, 2_200.0],
                        points,
                    },
                );
            }
        }
        for name in [
            "mGear",
            "mMicroChip1",
            "mMicroChip2",
            "pLEDBlue",
            "pLEDGreen",
            "pLEDRed",
        ] {
            libraries.insert(
                name.into(),
                HitLibrary {
                    bounds_twips: [0.0, 1_000.0, 0.0, 1_000.0],
                    points: (0..=10)
                        .flat_map(|x| (0..=10).map(move |y| [x * 5, y * 5]))
                        .collect(),
                },
            );
        }
        let atlas = LivewireAtlas {
            version: 1,
            step: 5,
            movie_sha256: "a".repeat(64),
            libraries,
        };
        atlas.validate().unwrap();
        atlas
    }

    fn hit(wire: &Wire, point: &Candidate, atlas: &LivewireAtlas) -> bool {
        atlas.library(&wire.lib).is_some_and(|library| {
            library.points.iter().any(|[local_x, local_y]| {
                let x = ((wire.x + f64::from(*local_x)) / 5.0).round() as i32 * 5;
                let y = ((wire.y + f64::from(*local_y)) / 5.0).round() as i32 * 5;
                point.x == x && point.y == y
            })
        })
    }

    #[test]
    fn seeded_livewire_game_boards_plan_exposed_topmost_points_at_all_difficulties() {
        let atlas = atlas();
        for difficulty in 1..=4 {
            for seed in 1..=12 {
                let session = MinigameSession {
                    session_instance_id: 1,
                    terminal_reported: false,
                    entity_id: 42,
                    player_id: 17,
                    game_name: "Livewire".into(),
                    difficulty,
                    tech_competency: 1,
                    seed,
                    abilities_mask: 0,
                    intelligence: 0,
                    player_level: 1,
                    ticket: "test".into(),
                    on_victory_chains: vec![],
                    on_failure_chains: vec![],
                    player_name: None,
                    client_ip: None,
                    created_at: tokio::time::Instant::now(),
                    connected: true,
                    cancel: tokio::sync::watch::channel(false).0,
                };
                let mut game = create_game(&session).unwrap();
                game.started();
                let board = serde_json::to_value(game.board_snapshot().unwrap()).unwrap();
                let result = plan(&board, &atlas)
                    .unwrap_or_else(|error| panic!("difficulty={difficulty} seed={seed}: {error}"));
                let parsed: Board = serde_json::from_value(board).unwrap();
                let mut cut = BTreeSet::new();
                for click in &result.clicks {
                    let target = parsed
                        .wires
                        .iter()
                        .find(|wire| wire.name == click.name)
                        .unwrap();
                    assert!(
                        !click.candidates.is_empty(),
                        "difficulty={difficulty} seed={seed}"
                    );
                    for point in &click.candidates {
                        assert!((445..=1270).contains(&point.x));
                        assert!((110..=890).contains(&point.y));
                        assert!(hit(target, point, &atlas), "target miss: {click:?}");
                        assert!(
                            !parsed.wires.iter().any(|wire| {
                                wire.depth > target.depth
                                    && !wire.cut
                                    && !cut.contains(&wire.name)
                                    && hit(wire, point, &atlas)
                            }),
                            "higher live wire covers {click:?}"
                        );
                    }
                    assert!(cut.insert(click.name.clone()), "duplicate click: {click:?}");
                }
                assert_eq!(
                    result
                        .clicks
                        .iter()
                        .filter(|click| click.kind == "goal")
                        .count(),
                    parsed.goal_total as usize,
                    "difficulty={difficulty} seed={seed}"
                );
            }
        }
    }

    #[test]
    fn goal_with_fewer_occluded_cells_is_planned_first() {
        let mut atlas = atlas();
        atlas.libraries.get_mut("gRedSilver1").unwrap().points = vec![[200, 0], [205, 0]];
        atlas.libraries.get_mut("gBlackGreen1").unwrap().points = vec![[200, 0], [205, 0]];
        atlas.libraries.get_mut("oBlack1").unwrap().points = vec![[200, 0]];
        let board = serde_json::json!({"state":"ready", "goal_total":2, "wires":[
            {"name":"g1","lib":"gRedSilver1","kind":"goal","depth":1,"x":275.6,"y":391.0,"cut":false},
            {"name":"g2","lib":"gBlackGreen1","kind":"goal","depth":2,"x":275.6,"y":497.0,"cut":false},
            {"name":"o1","lib":"oBlack1","kind":"obstacle","depth":3,"x":275.6,"y":391.0,"cut":false}
        ]});
        let result = plan(&board, &atlas).unwrap();
        assert_eq!(result.clicks[0].name, "g2");
    }
}
