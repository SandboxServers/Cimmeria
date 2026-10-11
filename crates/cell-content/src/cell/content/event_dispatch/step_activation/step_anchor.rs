//! The `mission.step_anchor` row (TG-MIS-03).
//!
//! One INFO row per step activation, naming what the new step needs the
//! player to reach: the chains that can advance it (`advancers`) and the
//! live entities those chains' triggers wait on, resolved in the player's
//! own space. It answers "can this step finish at all?" at the moment the
//! step starts, instead of after a playtester stalls:
//!
//! - `advancers = 0`: no chain moves the step forward (Frost's Letter step
//!   4037). Expected only for a turn-in step a `complete_mission` elsewhere
//!   closes.
//! - `unresolved_anchors` non-empty: a tag or template an advancer waits on
//!   has no entity in the space (#1341, an anchor that was never spawned).
//! - `anchor_in_witness_set = false`: the entity exists but the player's
//!   client has not been told about it yet.

use cimmeria_common::EntityId;
use cimmeria_content_engine::chain::ChainEngine;

use crate::cell::space_manager::SpaceManager;

/// The hand-named target (pinned in `OTEL_FILTER`, catalogued in
/// `observability-target-catalog.md`).
pub(crate) const TARGET: &str = "mission.step_anchor";

/// How many anchors and ids a row lists; the counts are always whole.
pub(crate) const MAX_LISTED: usize = 5;

/// One live entity an advancer's trigger waits on.
struct Anchor {
    entity_id: u32,
    /// The tag or template name the advancer named.
    key: String,
    in_witness_set: bool,
    distance: f32,
}

fn join<T: ToString>(items: impl IntoIterator<Item = T>) -> String {
    items
        .into_iter()
        .take(MAX_LISTED)
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// Log the anchors of `step_id` of `mission_id`, which has just become the
/// active step for player entity `entity_id`. NPCs carry no missions; a
/// non-player entity logs nothing.
pub(crate) fn log_step_anchors(
    entity_id: u32,
    mission_id: i32,
    step_id: i32,
    engine: &ChainEngine,
    space_mgr: &SpaceManager,
) {
    let Some(player) = space_mgr.get_entity(entity_id).filter(|e| e.is_player) else {
        return;
    };
    let wanted = engine.step_anchors(mission_id, step_id);

    let mut anchors: Vec<Anchor> = Vec::new();
    let mut unresolved: Vec<&str> = Vec::new();
    let space = space_mgr
        .entity_space
        .get(&entity_id)
        .and_then(|sid| space_mgr.spaces.get(sid));
    let keys = wanted
        .tags
        .iter()
        .map(|t| (t, true))
        .chain(wanted.templates.iter().map(|t| (t, false)));
    for (key, is_tag) in keys {
        let before = anchors.len();
        for (&eid, e) in space.into_iter().flat_map(|s| s.entities.iter()) {
            if e.is_player {
                continue;
            }
            let hit = if is_tag {
                e.tag.as_deref() == Some(key.as_str())
            } else {
                e.npc_name.as_deref() == Some(key.as_str())
            };
            if hit {
                let (dx, dy, dz) = (
                    e.position.x - player.position.x,
                    e.position.y - player.position.y,
                    e.position.z - player.position.z,
                );
                anchors.push(Anchor {
                    entity_id: eid,
                    key: key.clone(),
                    in_witness_set: player.witnesses.contains(&EntityId(eid as i32)),
                    distance: (dx * dx + dy * dy + dz * dz).sqrt(),
                });
            }
        }
        if anchors.len() == before {
            unresolved.push(key);
        }
    }
    // Nearest first, so the five listed are the ones the player can reach.
    anchors.sort_by(|a, b| a.distance.total_cmp(&b.distance));

    let names = cimmeria_names::book();
    let who = space_mgr.player_identity(entity_id);
    let world = space_mgr.get_entity_world_name(entity_id);
    tracing::info!(
        target: TARGET,
        event = "step_anchor",
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        world = world.as_deref(),
        entity_id,
        entity_name = space_mgr.entity_label(entity_id),
        mission_id,
        mission_name = names.mission(mission_id),
        step_id,
        step_name = names.mission_step(step_id),
        advancers = wanted.advancers.len(),
        advancer_chain_ids = %join(&wanted.advancers),
        anchors = anchors.len(),
        anchor_ids = %join(anchors.iter().map(|a| a.entity_id)),
        anchor_names = %join(
            anchors
                .iter()
                .map(|a| space_mgr.entity_label(a.entity_id).unwrap_or("-"))
        ),
        anchor_tags = %join(anchors.iter().map(|a| a.key.as_str())),
        anchor_in_witness_set = %join(anchors.iter().map(|a| a.in_witness_set)),
        anchor_distance = %join(anchors.iter().map(|a| format!("{:.1}", a.distance))),
        unresolved_anchors = %join(unresolved),
        dialog_anchors = %join(&wanted.dialog_ids),
        "mission step activated: its advancers and anchors"
    );
}
