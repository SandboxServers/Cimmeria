//! Record walked routes and load named waypoint legs for UAT.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use super::geometry::{client_to_server, Vec3};
use super::io::{LiveWorld, WorldIo};
use crate::supervisor::Supervisor;

const CATALOG: &str =
    include_str!("../../../../../docs/guides/uat-specs/routes/castle-cellblock.toml");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    schema: u32,
    #[serde(default, rename = "route")]
    routes: Vec<Route>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Route {
    name: String,
    space: String,
    points: Vec<[f64; 3]>,
}

/// Return the named leg's arguments for `client_move_to`.
pub fn move_args(name: &str) -> Result<Value, String> {
    move_args_from(CATALOG, name)
}

fn move_args_from(text: &str, name: &str) -> Result<Value, String> {
    let catalog: Catalog = toml::from_str(text).map_err(|e| format!("route catalog: {e}"))?;
    if catalog.schema != 1 {
        return Err(format!(
            "route catalog schema {} is unsupported",
            catalog.schema
        ));
    }
    let mut names = BTreeMap::new();
    for route in catalog.routes {
        if !valid_name(&route.name) || route.space != "Castle_CellBlock" || route.points.len() < 2 {
            return Err(format!("invalid route {} in catalog", route.name));
        }
        if route.points.iter().flatten().any(|v| !v.is_finite()) {
            return Err(format!("route {} has a non-finite coordinate", route.name));
        }
        if names.insert(route.name.clone(), route.points).is_some() {
            return Err(format!("duplicate route {} in catalog", route.name));
        }
    }
    let points = names
        .get(name)
        .ok_or_else(|| format!("unknown route {name:?}"))?;
    let waypoint = |p: &[f64; 3]| json!({"x":p[0], "y":p[1], "z":p[2]});
    Ok(json!({
        "waypoints": points[..points.len()-1].iter().map(waypoint).collect::<Vec<_>>(),
        "point": waypoint(points.last().expect("validated route has points")),
    }))
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 80
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

struct Recording {
    name: String,
    samples: Arc<Mutex<Vec<Vec3>>>,
    task: tokio::task::JoinHandle<()>,
}

/// One active recording per lab server instance.
#[derive(Default)]
pub struct RouteRecorder {
    active: Option<Recording>,
}

impl RouteRecorder {
    /// Start sampling the player position every 500 ms.
    pub async fn start(
        &mut self,
        name: String,
        supervisor: Arc<Supervisor>,
    ) -> Result<Value, String> {
        if !valid_name(&name) {
            return Err("route name must be lowercase letters, digits or hyphens (1..80)".into());
        }
        if self.active.is_some() {
            return Err("a route is already recording; call lab_route_stop first".into());
        }
        let mut io = LiveWorld::new(&supervisor);
        let first = player_position(&mut io).await?;
        let samples = Arc::new(Mutex::new(vec![first]));
        let recorded = samples.clone();
        let task = tokio::spawn(async move {
            let mut io = LiveWorld::new(&supervisor);
            let mut tick = tokio::time::interval(Duration::from_millis(500));
            tick.tick().await;
            loop {
                tick.tick().await;
                match player_position(&mut io).await {
                    Ok(point) => recorded.lock().await.push(point),
                    Err(error) => tracing::warn!(%error, "route sample skipped"),
                }
            }
        });
        self.active = Some(Recording {
            name: name.clone(),
            samples,
            task,
        });
        Ok(json!({"recording":true,"name":name,"sample_hz":2,"native_level":"read"}))
    }

    /// Stop sampling and return a TOML candidate for human review.
    pub async fn stop(&mut self) -> Result<Value, String> {
        let recording = self.active.take().ok_or("no active route recording")?;
        recording.task.abort();
        let samples = recording.samples.lock().await;
        let waypoints = simplify(&samples);
        let toml = format!(
            "[[route]]\nname = {:?}\nspace = \"Castle_CellBlock\"\npoints = [\n{}]\n",
            recording.name,
            waypoints
                .iter()
                .map(|p| format!("  [{:.2}, {:.2}, {:.2}],\n", p.x, p.y, p.z))
                .collect::<String>()
        );
        Ok(
            json!({"name":recording.name,"samples":samples.len(),"waypoints":waypoints.iter().map(|p| p.to_json()).collect::<Vec<_>>(),"toml":toml,"native_level":"read"}),
        )
    }
}

async fn player_position<W: WorldIo>(io: &mut W) -> Result<Vec3, String> {
    let snapshot = io.snapshot().await?;
    let player = snapshot.player().ok_or("player not in entity map")?;
    let pose = player.pose.ok_or("player actor pose unavailable")?;
    Ok(client_to_server(pose.pos))
}

/// Keep the start and end, and a point at each 20-degree turn or 8 m span.
pub fn simplify(samples: &[Vec3]) -> Vec<Vec3> {
    let Some(&first) = samples.first() else {
        return vec![];
    };
    let mut out = vec![first];
    for pair in samples.windows(3) {
        let (before, point, after) = (pair[0], pair[1], pair[2]);
        let last = *out.last().expect("start is present");
        let distance = ((point.x - last.x).powi(2) + (point.z - last.z).powi(2)).sqrt();
        let a = (point.z - before.z).atan2(point.x - before.x);
        let b = (after.z - point.z).atan2(after.x - point.x);
        let angle = (b - a).sin().atan2((b - a).cos()).abs().to_degrees();
        if distance >= 8.0 || (distance >= 0.5 && angle > 20.0) {
            out.push(point);
        }
    }
    if let Some(&end) = samples.last() {
        if end != *out.last().expect("start is present") {
            out.push(end);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simplification_keeps_turn_distance_and_end() {
        let points = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
            Vec3::new(9.0, 0.0, 0.0),
            Vec3::new(12.0, 0.0, 0.0),
            Vec3::new(12.0, 0.0, 4.0),
            Vec3::new(12.0, 0.0, 6.0),
        ];
        let result = simplify(&points);
        assert_eq!(result, vec![points[0], points[2], points[3], points[5]]);
    }

    #[test]
    fn catalog_loads_and_rejects_unknown_name() {
        let sample = "schema = 1\n[[route]]\nname = 'stasis-exit'\nspace = 'Castle_CellBlock'\npoints = [[1.0,2.0,3.0],[4.0,5.0,6.0]]\n";
        let args = move_args_from(sample, "stasis-exit").unwrap();
        assert_eq!(args["waypoints"][0]["x"], 1.0);
        assert_eq!(args["point"]["z"], 6.0);
        assert!(move_args_from(sample, "absent")
            .unwrap_err()
            .contains("unknown route"));
    }

    #[test]
    fn uat_spec_rejects_an_unknown_route_before_running() {
        let text = r#"
schema = 1
[section]
id = "route-test"
system = "Cellblock"
guide = "unified-uat.md#cellblock"
ledger = "cellblock/README.md"
[[row]]
id = "R1"
title = "walk"
expected = "arrives"
step = [{ tool = "@move_to", route = "missing-leg", label = "walk" }]
[[row.expect]]
id = "arrived"
text = "the avatar arrives"
source = "chat"
since = "walk"
contains = "arrived"
"#;
        let error = crate::uat::spec::parse(text).unwrap_err();
        assert!(error.contains("unknown route \"missing-leg\""), "{error}");
    }
}
