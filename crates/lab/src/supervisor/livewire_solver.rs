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

fn candidates(shapes: &[Shape], cut: &BTreeSet<String>, target: usize) -> Vec<Candidate> {
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
    exposed.sort_by_key(|point| (-point.clearance, point.x, point.y));
    let mut chosen: Vec<Candidate> = Vec::new();
    for point in &exposed {
        if chosen
            .iter()
            .all(|other| (point.x - other.x).abs().max((point.y - other.y).abs()) >= 15)
        {
            chosen.push(point.clone());
            if chosen.len() == 5 {
                return chosen;
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
    chosen
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
        let mut goals: Vec<(usize, Vec<Candidate>)> = shapes
            .iter()
            .enumerate()
            .filter(|(_, shape)| shape.wire.kind == "goal" && !cut.contains(&shape.wire.name))
            .map(|(index, _)| (index, candidates(&shapes, &cut, index)))
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
        goals.sort_by_key(|(index, points)| {
            (
                points.len(),
                shapes[*index].wire.depth,
                shapes[*index].wire.name.clone(),
            )
        });
        if let Some((index, points)) = goals.iter().find(|(_, points)| !points.is_empty()) {
            clicks.push(make_click(&shapes[*index], points.clone()));
            cut.insert(shapes[*index].wire.name.clone());
            continue;
        }
        let mut coverage = BTreeMap::<usize, usize>::new();
        for (goal, _) in &goals {
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
            .filter(|(index, _)| !candidates(&shapes, &cut, *index).is_empty())
            .max_by_key(|(_, cells)| *cells)
            .map(|(index, _)| index)
            .ok_or_else(|| "goal_fully_occluded".to_string())?;
        clicks.push(make_click(
            &shapes[target],
            candidates(&shapes, &cut, target),
        ));
        cut.insert(shapes[target].wire.name.clone());
    }
    Err("Livewire plan exceeded wire count".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supervisor::livewire_atlas::HitLibrary;

    fn atlas() -> LivewireAtlas {
        let mut libraries = BTreeMap::new();
        for base in [
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
        ] {
            for variant in 1..=4 {
                libraries.insert(
                    format!("{base}{variant}"),
                    HitLibrary {
                        bounds_twips: [0.0, 21_000.0, -2_000.0, 2_000.0],
                        points: (0..=180).map(|i| [i * 5, (i % 5 - 2) * 5]).collect(),
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
        LivewireAtlas {
            version: 1,
            step: 5,
            movie_sha256: "a".repeat(64),
            libraries,
        }
    }

    #[test]
    fn seeded_boards_plan_only_exposed_topmost_points_at_all_difficulties() {
        let atlas = atlas();
        let mut seed = 0x5eed_u64;
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            (seed >> 32) as usize
        };
        for difficulty in 1..=4 {
            for _ in 0..25 {
                let goals = if difficulty == 1 {
                    2
                } else if difficulty == 4 {
                    6
                } else {
                    4
                };
                let mut wires = Vec::new();
                for index in 0..(goals + 8) {
                    let (name, lib, kind) = if index < goals {
                        (
                            format!("g{index}"),
                            format!("gRedSilver{}", next() % 4 + 1),
                            "goal",
                        )
                    } else if index < goals + 3 {
                        (
                            format!("o{index}"),
                            format!("oBlack{}", next() % 4 + 1),
                            "obstacle",
                        )
                    } else {
                        (
                            format!("p{index}"),
                            format!("pGray{}", next() % 4 + 1),
                            "playfield",
                        )
                    };
                    let (depth, y) = if index < goals {
                        (20 + index, 220.0 + index as f64 * 100.0)
                    } else if index < goals + 3 {
                        (60 + index, 220.0 + (next() % goals) as f64 * 100.0)
                    } else {
                        (index - goals, 220.0 + (next() % goals) as f64 * 100.0)
                    };
                    wires.push(serde_json::json!({"name": name, "lib": lib, "kind": kind,
                        "depth": depth, "x": 275.6, "y": y, "cut": false}));
                }
                let board =
                    serde_json::json!({"state":"running", "goal_total": goals, "wires": wires});
                let result = plan(&board, &atlas).unwrap();
                let parsed: Board = serde_json::from_value(board).unwrap();
                let mut cut = BTreeSet::new();
                let shapes: Vec<_> = parsed
                    .wires
                    .iter()
                    .map(|wire| Shape {
                        wire: wire.clone(),
                        cells: atlas
                            .library(&wire.lib)
                            .unwrap()
                            .points
                            .iter()
                            .map(|[x, y]| {
                                (
                                    grid(wire.x + f64::from(*x)).unwrap(),
                                    grid(wire.y + f64::from(*y)).unwrap(),
                                )
                            })
                            .collect(),
                    })
                    .collect();
                for click in &result.clicks {
                    let target = shapes
                        .iter()
                        .position(|shape| shape.wire.name == click.name)
                        .unwrap();
                    assert!(!click.candidates.is_empty());
                    for point in &click.candidates {
                        assert!(!blocked((point.x, point.y)));
                        assert_eq!(top(&shapes, &cut, (point.x, point.y)), Some(target));
                    }
                    cut.insert(click.name.clone());
                }
                assert_eq!(
                    result
                        .clicks
                        .iter()
                        .filter(|click| click.kind == "goal")
                        .count(),
                    goals
                );
            }
        }
    }
}
