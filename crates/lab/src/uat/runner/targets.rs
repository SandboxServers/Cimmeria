//! Spec targets (SV-07): `pose`, `tag` and `face_tag` arguments become the
//! plain `entity_id` / `point` arguments a client tool takes, and `@stand`
//! teleports to a stand point and waits until the server has the player
//! there.
//!
//! D-SV1: the client has no tags, the server does, so the runner resolves
//! them: the player's entity id from the client, its space from
//! `server_entity_get`, then `server_entity_query { space_id, tag }`, with
//! the reply filtered by tag again (a server older than SV-01 ignores the
//! argument and answers with the whole space). Every read goes into the
//! action's `calls`, never into an action record of its own, so grading
//! and action indices do not move.
//!
//! A `pose` given together with `name`, `entity_id` or `point` (or the
//! `face_*` ones on the camera) is refused: the spec validator says so, and
//! the runner again for arguments built from `${vars}`. Any other argument
//! given next to a pose (`zoom`, `pitch_deg`, `rotate_camera`, `aim`, ...)
//! wins over the pose's value.

use std::time::Duration;

use serde_json::{json, Map, Value};

use super::actions::fail;
use super::players::Who;
use super::{RowCtx, Runner};
use crate::uat::evidence::ActionRecord;
use crate::uat::invoke::{ServerInvoker, ToolInvoker};
use crate::uat::spec::{AimMode, PoseSpec};
use crate::uat::spec_validate::pose_clashes;
use crate::uat::tools::STAND_TOOL;

/// Why a tag cannot be resolved without the server lab MCP.
pub(crate) const NEEDS_SERVER: &str = "needs the server lab MCP (CIMMERIA_LAB_MCP_URL/_TOKEN)";
/// The arguments that name a target through the server.
pub(crate) const TAG_ARGS: [&str; 3] = ["pose", "tag", "face_tag"];
/// Removed before a client tool sees the arguments.
const RUNNER_ARGS: [&str; 5] = ["pose", "tag", "face_tag", "aim", "aim_dy_m"];
/// The client's own entity and position.
pub(crate) const FIND_TOOL: &str = "client_entity_find";
/// D-SV11: how often `@stand` asks where the player is ...
pub(crate) const STAND_POLL: Duration = Duration::from_millis(250);
/// ... for how long ...
pub(crate) const STAND_TIMEOUT: Duration = Duration::from_secs(5);
/// ... and how close (horizontally, server metres) counts as there.
pub(crate) const STAND_TOLERANCE_M: f64 = 0.5;

/// The one entity carrying a tag in the lab player's space.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedTag {
    pub tag: String,
    /// The lab player's own entity, looked up on the way.
    pub player: u32,
    pub entity_id: u32,
    pub space_id: u32,
    /// Live server position, metres.
    pub position: [f64; 3],
}

/// Whether an action's arguments name a target through the server.
pub(crate) fn names_a_tag(args: Option<&Value>) -> bool {
    args.and_then(Value::as_object)
        .is_some_and(|m| TAG_ARGS.iter().any(|k| m.contains_key(*k)))
}

fn find_args() -> Value {
    json!({ "include_player": true, "limit": 1, "project": false })
}

/// A stand poll: the player by id, so the client scans no other entity.
fn poll_args(player: u32) -> Value {
    json!({ "entity_id": player, "include_player": true, "limit": 1, "project": false })
}

/// A `pose` next to an argument it replaces, refused (SV-07).
pub(super) fn refuse_pose_clash(tool: &str, m: &Map<String, Value>) -> Result<(), String> {
    if !m.contains_key("pose") {
        return Ok(());
    }
    match pose_clashes(tool).iter().find(|c| m.contains_key(**c)) {
        Some(c) => Err(format!("pose with {c}: give one")),
        None => Ok(()),
    }
}

fn as_u32(v: &Value) -> Option<u32> {
    v.as_u64().and_then(|n| u32::try_from(n).ok())
}

fn as_xyz(v: &Value) -> Option<[f64; 3]> {
    match v {
        Value::Array(a) if a.len() == 3 => Some([a[0].as_f64()?, a[1].as_f64()?, a[2].as_f64()?]),
        Value::Object(o) => Some([
            o.get("x")?.as_f64()?,
            o.get("y")?.as_f64()?,
            o.get("z")?.as_f64()?,
        ]),
        _ => None,
    }
}

fn point_json(p: [f64; 3]) -> Value {
    json!({ "x": p[0], "y": p[1], "z": p[2] })
}

fn fmt_point(p: [f64; 3]) -> String {
    format!("({:.2}, {:.2}, {:.2})", p[0], p[1], p[2])
}

/// x/z distance: the stand height is the server's to settle.
fn horizontal(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).hypot(a[2] - b[2])
}

/// A pose built from bare arguments: `tag` (or `face_tag`), and for a
/// stand `radius_m`, `bearing_deg` and `dy_m`.
fn pose_of_args(m: &Map<String, Value>, stand: bool) -> Result<PoseSpec, String> {
    let tag = m
        .get("tag")
        .or_else(|| m.get("face_tag"))
        .and_then(Value::as_str)
        .ok_or("tag must be a string")?
        .to_string();
    let num = |k: &str| m.get(k).and_then(Value::as_f64);
    let (radius_m, bearing_deg) = if stand {
        match (num("radius_m"), num("bearing_deg")) {
            (Some(r), Some(b)) => (r, b),
            _ => {
                return Err(format!(
                    "@stand by tag {tag} needs radius_m and bearing_deg"
                ))
            }
        }
    } else {
        (0.0, 0.0)
    };
    Ok(PoseSpec {
        id: String::new(),
        tag,
        radius_m,
        bearing_deg,
        dy_m: num("dy_m").unwrap_or(0.1),
        pitch_deg: None,
        yaw_offset_deg: None,
        zoom: None,
        aim: AimMode::Entity,
        aim_dy_m: None,
        calibrated: None,
    })
}

/// The row's pose named by `pose`, when the arguments name one.
fn named_pose(m: &Map<String, Value>, ctx: &RowCtx) -> Result<Option<PoseSpec>, String> {
    let Some(p) = m.get("pose") else {
        return Ok(None);
    };
    let id = p.as_str().ok_or("pose must be a string")?;
    ctx.poses
        .iter()
        .find(|x| x.id == id)
        .cloned()
        .map(Some)
        .ok_or_else(|| format!("pose {id} is not a pose of this row"))
}

/// `aim` / `aim_dy_m` given as arguments override the pose's.
fn apply_aim_args(pose: &mut PoseSpec, m: &Map<String, Value>) -> Result<(), String> {
    match m.get("aim").map(|v| v.as_str()) {
        None => {}
        Some(Some("entity")) => pose.aim = AimMode::Entity,
        Some(Some("point")) => pose.aim = AimMode::Point,
        Some(other) => return Err(format!("aim {other:?}: give \"entity\" or \"point\"")),
    }
    if let Some(d) = m.get("aim_dy_m") {
        if pose.aim == AimMode::Entity {
            return Err("aim_dy_m needs aim = \"point\"".into());
        }
        pose.aim_dy_m = Some(d.as_f64().ok_or("aim_dy_m must be a number")?);
    }
    if pose.aim == AimMode::Point && pose.aim_dy_m.is_none() {
        return Err("aim = \"point\" needs aim_dy_m".into());
    }
    Ok(())
}

impl<I: ToolInvoker> Runner<'_, I> {
    /// The client's own player entity id (`client_entity_find` `/player/id`).
    pub(crate) async fn player_entity(
        &self,
        who: Who,
        rec: &mut ActionRecord,
    ) -> Result<u32, String> {
        let args = find_args();
        let out = self.on(who).call(FIND_TOOL, args.clone()).await;
        rec.calls
            .push(json!({ "tool": FIND_TOOL, "args": args, "ok": out.ok }));
        if !out.ok {
            return Err(format!("{FIND_TOOL}: {}", out.error.unwrap_or_default()));
        }
        out.json
            .pointer("/player/id")
            .and_then(as_u32)
            .ok_or_else(|| format!("{FIND_TOOL} reported no /player/id"))
    }

    /// One server read, recorded in the action's `calls`.
    async fn server_read(
        server: &dyn ServerInvoker,
        tool: &str,
        args: Value,
        rec: &mut ActionRecord,
    ) -> Result<Value, String> {
        let out = server.call(tool, args.clone()).await;
        rec.calls
            .push(json!({ "tool": tool, "args": args, "ok": out.ok }));
        if out.ok {
            Ok(out.json)
        } else {
            Err(format!("{tool}: {}", out.error.unwrap_or_default()))
        }
    }

    /// D-SV1: the one entity with `tag` in the player's space.
    pub(crate) async fn resolve_tag(
        &self,
        who: Who,
        tag: &str,
        rec: &mut ActionRecord,
    ) -> Result<ResolvedTag, String> {
        let server = self
            .server
            .ok_or_else(|| format!("tag {tag} {NEEDS_SERVER}"))?;
        let player = self.player_entity(who, rec).await?;
        let got = Self::server_read(
            server,
            "server_entity_get",
            json!({ "entity_id": player }),
            rec,
        )
        .await?;
        let space_id = got
            .pointer("/entity/space_id")
            .and_then(as_u32)
            .ok_or_else(|| format!("server_entity_get: no space for player entity {player}"))?;
        let reply = Self::server_read(
            server,
            "server_entity_query",
            json!({ "space_id": space_id, "tag": tag }),
            rec,
        )
        .await?;
        // Filter again: an old server ignores `tag` and returns the space.
        let hits: Vec<&Value> = reply
            .get("entities")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|e| e.get("tag").and_then(Value::as_str) == Some(tag))
            .filter(|e| {
                e.get("space_id")
                    .and_then(as_u32)
                    .is_none_or(|s| s == space_id)
            })
            .collect();
        let one = match hits.as_slice() {
            [one] => *one,
            [] => {
                // A capped reply from an old server may have cut the entity off.
                let capped = if reply.get("capped") == Some(&json!(true)) {
                    " (the reply was capped)"
                } else {
                    ""
                };
                return Err(format!(
                    "no entity with tag {tag} in space {space_id}{capped}"
                ));
            }
            many => {
                let ids: Vec<String> = many
                    .iter()
                    .map(|e| e.get("entity_id").map_or("?".into(), Value::to_string))
                    .collect();
                return Err(format!(
                    "tag {tag} matches entities {} in space {space_id}",
                    ids.join(", ")
                ));
            }
        };
        let entity_id = one
            .get("entity_id")
            .and_then(as_u32)
            .ok_or_else(|| format!("tag {tag}: the matching entity has no entity_id"))?;
        let position = one
            .get("position")
            .and_then(as_xyz)
            .ok_or_else(|| format!("tag {tag}: entity {entity_id} has no position"))?;
        rec.calls.push(json!({ "resolve": {
            "tag": tag, "entity_id": entity_id, "space_id": space_id,
        }}));
        Ok(ResolvedTag {
            tag: tag.to_string(),
            player,
            entity_id,
            space_id,
            position,
        })
    }

    /// [`Self::resolve_tag`] for a pose: its id joins the `resolve` record
    /// (SV-11's heal hint reads it there).
    async fn resolve_pose(
        &self,
        who: Who,
        pose: &PoseSpec,
        rec: &mut ActionRecord,
    ) -> Result<ResolvedTag, String> {
        let t = self.resolve_tag(who, &pose.tag, rec).await?;
        if !pose.id.is_empty() {
            if let Some(r) = rec.calls.last_mut().and_then(|c| c.get_mut("resolve")) {
                r["pose"] = json!(pose.id);
            }
        }
        Ok(t)
    }

    /// `pose` / `tag` / `face_tag` arguments rewritten to plain ones.
    pub(crate) async fn expand_target_args(
        &self,
        who: Who,
        tool: &str,
        args: Value,
        ctx: &RowCtx,
        rec: &mut ActionRecord,
    ) -> Result<Value, String> {
        let Value::Object(mut m) = args else {
            return Ok(args);
        };
        if !TAG_ARGS.iter().any(|k| m.contains_key(*k)) {
            return Ok(Value::Object(m));
        }
        refuse_pose_clash(tool, &m)?;
        let named = named_pose(&m, ctx)?;
        let from_pose = named.is_some();
        let mut pose = match named {
            Some(p) => p,
            None => pose_of_args(&m, false)?,
        };
        apply_aim_args(&mut pose, &m)?;
        for k in RUNNER_ARGS {
            m.remove(k);
        }
        let target = self.resolve_pose(who, &pose, rec).await?;
        let aim = pose.aim_point(target.position);
        match tool {
            "client_world_click" | "client_target" => {
                for k in ["name", "entity_id", "point"] {
                    m.remove(k);
                }
                match aim {
                    Some(p) => m.insert("point".into(), point_json(p)),
                    None => m.insert("entity_id".into(), json!(target.entity_id)),
                };
                if from_pose {
                    // The pose's stand and camera set the view: don't turn it.
                    m.entry("rotate_camera").or_insert(json!(false));
                }
            }
            "client_camera" => {
                for k in ["face_name", "face_entity_id", "face_point"] {
                    m.remove(k);
                }
                match aim {
                    Some(p) => m.insert("face_point".into(), point_json(p)),
                    None => m.insert("face_entity_id".into(), json!(target.entity_id)),
                };
                for (k, v) in [
                    ("pitch_deg", pose.pitch_deg),
                    ("yaw_offset_deg", pose.yaw_offset_deg),
                ] {
                    if let Some(v) = v {
                        m.entry(k).or_insert(json!(v));
                    }
                }
                // `zoom_to` is the same setting; the tool refuses both.
                if let (Some(z), false) = (pose.zoom, m.contains_key("zoom_to")) {
                    m.entry("zoom").or_insert(json!(z));
                }
            }
            "client_entity_find" => {
                m.remove("name");
                m.insert("entity_id".into(), json!(target.entity_id));
            }
            other => return Err(format!("{other} does not take pose, tag or face_tag")),
        }
        Ok(Value::Object(m))
    }

    /// D-SV11: `/gmgotoxyz`, then wait until `player` (the lab player's
    /// entity, from [`Self::player_entity`]) is there. `lab-fixtures` reuses
    /// it: the caller's earlier `rec.ok` does not decide the outcome.
    pub(crate) async fn stand_at(
        &self,
        who: Who,
        player: u32,
        point: [f64; 3],
        rec: &mut ActionRecord,
    ) -> Result<(), String> {
        let line = format!("/gmgotoxyz {:.2} {:.2} {:.2}", point[0], point[1], point[2]);
        // `send_chat` reports through `rec` and the macro path only ever
        // fails it, so start clean.
        rec.ok = true;
        rec.error = None;
        self.send_chat(who, &line, rec).await;
        if !rec.ok {
            return Err(rec
                .error
                .clone()
                .unwrap_or_else(|| format!("{line}: not sent")));
        }
        let deadline = tokio::time::Instant::now() + STAND_TIMEOUT;
        // Only the last poll is kept: 20 identical reads say nothing more.
        let mut dist: Option<f64> = None;
        let mut why = String::new();
        let mut last = Value::Null;
        loop {
            // The deadline bounds the whole wait: the last sleep is cut
            // short, and none starts past it.
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            if left.is_zero() {
                break;
            }
            tokio::time::sleep(STAND_POLL.min(left)).await;
            let args = poll_args(player);
            let out = self.on(who).call(FIND_TOOL, args.clone()).await;
            // Server metres against server metres: the stand point is.
            let at = out
                .ok
                .then(|| out.json.pointer("/player/position/server").and_then(as_xyz))
                .flatten();
            last = json!({ "tool": FIND_TOOL, "args": args, "ok": out.ok, "position": at.map(point_json) });
            match at {
                Some(p) => {
                    let d = horizontal(p, point);
                    dist = Some(d);
                    if d <= STAND_TOLERANCE_M {
                        rec.calls.push(last);
                        return Ok(());
                    }
                }
                None => {
                    why = out
                        .error
                        .unwrap_or_else(|| "no /player/position/server".into());
                }
            }
        }
        rec.calls.push(last);
        let secs = STAND_TIMEOUT.as_secs();
        Err(match dist {
            Some(d) => format!(
                "stand: still {d:.2} m from {} after {secs} s",
                fmt_point(point)
            ),
            None => format!("stand: no player position after {secs} s ({FIND_TOOL}: {why})"),
        })
    }

    /// `@stand`: `pose` stands at the pose's point from the target's live
    /// position (D-SV3); `tag` + `radius_m` + `bearing_deg` (+ `dy_m`) the
    /// same with a pose built on the spot; `point` as it is.
    pub(crate) async fn exec_stand(
        &self,
        who: Who,
        args: &Value,
        ctx: &RowCtx,
        rec: &mut ActionRecord,
    ) {
        rec.tool = Some(STAND_TOOL.into());
        let (point, player) = match self.stand_point_of(who, args, ctx, rec).await {
            Ok(p) => p,
            Err(e) => return fail(rec, e),
        };
        // A point stand has not looked the player up yet.
        let player = match player {
            Some(id) => id,
            None => match self.player_entity(who, rec).await {
                Ok(id) => id,
                Err(e) => return fail(rec, e),
            },
        };
        let res = self.stand_at(who, player, point, rec).await;
        // `send_chat` names the sender it used; the record is the stand.
        rec.tool = Some(STAND_TOOL.into());
        rec.result = json!({ "stand_point": point_json(point) });
        match res {
            Ok(()) => {
                rec.ok = true;
                rec.error = None;
            }
            Err(e) => fail(rec, e),
        }
    }

    /// The stand point, and the player's entity when resolving a tag
    /// already looked it up.
    async fn stand_point_of(
        &self,
        who: Who,
        args: &Value,
        ctx: &RowCtx,
        rec: &mut ActionRecord,
    ) -> Result<([f64; 3], Option<u32>), String> {
        let empty = Map::new();
        let m = args.as_object().unwrap_or(&empty);
        refuse_pose_clash(STAND_TOOL, m)?;
        let pose = match named_pose(m, ctx)? {
            Some(p) => p,
            None if m.contains_key("tag") => pose_of_args(m, true)?,
            None => {
                return m
                    .get("point")
                    .and_then(as_xyz)
                    .map(|p| (p, None))
                    .ok_or_else(|| "@stand needs pose, tag or point = { x, y, z }".into());
            }
        };
        let target = self.resolve_pose(who, &pose, rec).await?;
        Ok((pose.stand_point(target.position), Some(target.player)))
    }
}
