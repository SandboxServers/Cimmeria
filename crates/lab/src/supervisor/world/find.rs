//! `client_entity_find`: which entities the client has, where, and what the
//! stock unit API calls them.
//!
//! One pass: walk the entity map and read each actor's pose (memory), pin
//! the nearest candidates to private unit slots and read name, level,
//! hostility and mob id with the stock `unit*` functions (one Lua call),
//! then project the matches with the game's own view (one Lua call plus a
//! frame). Nothing is driven: the result's `native_level` is `read`.

use rmcp::schemars;
use serde_json::{json, Value};

use super::geometry::{horizontal_m, Pose, Screen};
use super::io::WorldIo;
use super::lua::UnitInfo;
use super::memory::{WorldSnapshot, LAB_SLOT_BASE, LAB_SLOT_COUNT};
use super::{Steps, WorldError};

/// Default number of matches returned.
pub const DEFAULT_LIMIT: usize = 10;
/// Margin inside which a projected point counts as clickable.
pub const CLICK_MARGIN_PX: f64 = 8.0;

/// `client_entity_find` arguments.
#[derive(Debug, Clone, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct FindRequest {
    /// Only this entity id.
    #[serde(default)]
    pub entity_id: Option<u32>,
    /// Name to match (case-insensitive substring unless `exact`).
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub exact: bool,
    /// Template: the client's `unitMobId` (the NPC's mob/template id).
    #[serde(default)]
    pub mob_id: Option<i64>,
    /// `AggressionLevel` name from `unitHostilityToPlayer`, e.g. `Hostile`,
    /// `Friendly` (case-insensitive).
    #[serde(default)]
    pub hostility: Option<String>,
    /// Only entities within this horizontal distance of the player, metres.
    #[serde(default)]
    pub max_distance_m: Option<f64>,
    /// Only entities with an actor in the scene.
    #[serde(default)]
    pub rendered_only: bool,
    /// Include the player's own entity.
    #[serde(default)]
    pub include_player: bool,
    /// Max matches (nearest first; default 10).
    #[serde(default)]
    pub limit: Option<usize>,
    /// Project matches to the screen (default true).
    #[serde(default)]
    pub project: Option<bool>,
}

impl FindRequest {
    fn wants_unit_filter(&self) -> bool {
        self.name.is_some() || self.mob_id.is_some() || self.hostility.is_some()
    }

    fn info_matches(&self, info: Option<&UnitInfo>) -> bool {
        if !self.wants_unit_filter() {
            return true;
        }
        let Some(info) = info else { return false };
        if let Some(n) = &self.name {
            let ok = if self.exact {
                info.name.eq_ignore_ascii_case(n)
            } else {
                info.name.to_lowercase().contains(&n.to_lowercase())
            };
            if !ok {
                return false;
            }
        }
        if self.mob_id.is_some() && info.mob_id != self.mob_id {
            return false;
        }
        if let Some(h) = &self.hostility {
            if !info.hostility.eq_ignore_ascii_case(h) {
                return false;
            }
        }
        true
    }
}

/// One entity the find reports.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub id: u32,
    pub is_player: bool,
    pub actor: u32,
    pub pose: Option<Pose>,
    pub distance_m: Option<f64>,
    pub info: Option<UnitInfo>,
    pub screen_point: Option<(f64, f64)>,
    pub on_screen: bool,
}

impl Found {
    pub fn rendered(&self) -> bool {
        self.actor != 0
    }

    /// What a click needs: an actor on screen that the unit API knows.
    pub fn targetable(&self) -> bool {
        !self.is_player && self.rendered() && self.info.as_ref().is_some_and(|i| i.exists)
    }

    pub fn name(&self) -> Option<&str> {
        self.info
            .as_ref()
            .map(|i| i.name.as_str())
            .filter(|n| !n.is_empty())
    }

    pub fn to_json(&self) -> Value {
        let info = self.info.as_ref();
        json!({
            "id": self.id,
            "name": self.name(),
            "level": info.and_then(|i| i.level),
            "hostility": info.map(|i| i.hostility.clone()).filter(|h| !h.is_empty()),
            "is_friend": info.and_then(|i| i.is_friend),
            "mob_id": info.and_then(|i| i.mob_id),
            "unit_exists": info.map(|i| i.exists),
            "is_player": self.is_player,
            "rendered": self.rendered(),
            "targetable": self.targetable(),
            "position": self.pose.map(|p| p.to_json()),
            "distance_m": self.distance_m.map(|d| (d * 100.0).round() / 100.0),
            "screen": self.screen_point.map(|(x, y)| json!([x.round(), y.round()])),
            "on_screen": self.on_screen,
        })
    }
}

/// A find's result.
#[derive(Debug, Clone)]
pub struct FindResult {
    pub snapshot: WorldSnapshot,
    pub player: Option<Pose>,
    pub matches: Vec<Found>,
    pub scanned: usize,
    pub unscanned: usize,
    pub screen: Option<Screen>,
    pub projection_error: Option<String>,
}

/// Run a find inside a tool call (click and move resolve their target with it).
pub async fn find_entities<W: WorldIo>(
    io: &mut W,
    req: &FindRequest,
    steps: &mut Steps,
) -> Result<FindResult, WorldError> {
    let t0 = io.now_ms();
    let snap = io
        .snapshot()
        .await
        .map_err(|e| steps.fail(io.now_ms(), "read_entities", e))?;
    let player = snap.player().and_then(|p| p.pose);
    let mut cands: Vec<Found> = snap
        .entities
        .iter()
        .filter(|e| req.include_player || e.id != snap.player_id)
        .filter(|e| req.entity_id.is_none_or(|id| id == e.id))
        .filter(|e| !req.rendered_only || e.rendered())
        .map(|e| Found {
            id: e.id,
            is_player: e.id == snap.player_id,
            actor: e.actor,
            pose: e.pose,
            distance_m: match (player, e.pose) {
                (Some(p), Some(q)) => Some(horizontal_m(p.pos, q.pos)),
                _ => None,
            },
            info: None,
            screen_point: None,
            on_screen: false,
        })
        .filter(|f| match (req.max_distance_m, f.distance_m) {
            (Some(max), Some(d)) => d <= max,
            (Some(_), None) => false,
            (None, _) => true,
        })
        .collect();
    cands.sort_by(|a, b| {
        a.distance_m
            .unwrap_or(f64::INFINITY)
            .total_cmp(&b.distance_m.unwrap_or(f64::INFINITY))
            .then(a.id.cmp(&b.id))
    });
    steps.record(
        "read_entities",
        t0,
        io.now_ms(),
        json!({ "entities": snap.entities.len(), "candidates": cands.len() }),
    );

    // Name the nearest candidates through private unit slots.
    let t1 = io.now_ms();
    let scan = cands.len().min(LAB_SLOT_COUNT);
    let mut slots = Vec::with_capacity(scan);
    for (i, c) in cands.iter().take(scan).enumerate() {
        let slot = LAB_SLOT_BASE + i as i32;
        io.pin(slot, c.id).await.map_err(|e| {
            steps.fail(
                io.now_ms(),
                "pin_unit_slot",
                format!("entity {}: {e}", c.id),
            )
        })?;
        slots.push(slot);
    }
    let infos = io
        .unit_info(&slots)
        .await
        .map_err(|e| steps.fail(io.now_ms(), "unit_info", e))?;
    for (i, c) in cands.iter_mut().take(scan).enumerate() {
        c.info = infos
            .iter()
            .find(|u| u.slot == LAB_SLOT_BASE + i as i32)
            .cloned();
    }
    let unscanned = cands.len() - scan;
    steps.record(
        "unit_info",
        t1,
        io.now_ms(),
        json!({ "pinned": scan, "unscanned": unscanned }),
    );

    let limit = req.limit.unwrap_or(DEFAULT_LIMIT).max(1);
    let mut matches: Vec<Found> = cands
        .into_iter()
        .filter(|c| req.info_matches(c.info.as_ref()))
        .take(limit)
        .collect();

    let mut screen = None;
    let mut projection_error = None;
    if req.project.unwrap_or(true) {
        let t2 = io.now_ms();
        let points: Vec<_> = matches
            .iter()
            .filter_map(|m| m.pose.map(|p| p.pos))
            .collect();
        if !points.is_empty() {
            match io.project(&points).await {
                Ok(pr) => {
                    let mut it = pr.points.into_iter();
                    for m in matches.iter_mut().filter(|m| m.pose.is_some()) {
                        m.screen_point = it.next().flatten();
                        m.on_screen = m
                            .screen_point
                            .is_some_and(|p| pr.screen.contains(p, CLICK_MARGIN_PX));
                    }
                    screen = Some(pr.screen);
                }
                Err(e) => projection_error = Some(e),
            }
            steps.record(
                "project",
                t2,
                io.now_ms(),
                json!({ "points": points.len(), "error": projection_error }),
            );
        }
    }
    Ok(FindResult {
        snapshot: snap,
        player,
        matches,
        scanned: scan,
        unscanned,
        screen,
        projection_error,
    })
}

/// `client_entity_find`.
pub async fn run<W: WorldIo>(io: &mut W, req: FindRequest) -> Result<Value, WorldError> {
    let mut steps = Steps::new("client_entity_find", io.now_ms());
    let r = find_entities(io, &req, &mut steps).await?;
    let out = json!({
        "player": {
            "id": r.snapshot.player_id,
            "position": r.player.map(|p| p.to_json()),
        },
        "matches": r.matches.iter().map(Found::to_json).collect::<Vec<_>>(),
        "count": r.matches.len(),
        "entities_in_client": r.snapshot.entities.len(),
        "entity_map_truncated": r.snapshot.truncated,
        "named": r.scanned,
        "unnamed_beyond_scan": r.unscanned,
        "screen": r.screen.map(|s| json!([s.w, s.h])),
        "projection_error": r.projection_error,
        "note": "Positions come from each entity's actor (UE3 AActor::Location); `server` is the same point in BigWorld metres. `targetable` means it has an actor and the unit API knows it.",
    });
    Ok(steps.finish(io.now_ms(), out))
}
