//! How long a stored target (`CellEntity::current_target_id`) lives (#844).
//!
//! `setTargetID` and `gmSetTarget` are the only writers. Before #844 nothing
//! ever cleared the field, so a target that walked out of view, despawned or
//! respawned under the same id stayed selected server-side for as long as the
//! session lasted, and every reader acted on it: GM target resolution,
//! `setAutoCycle`, pet stance, and the `onTargetUpdate` replayed to new
//! observers. One colo `.summon` dropped a player on an NPC's spawn point
//! 216 m away off a 19-minute-old target.
//!
//! The server now drops a stored target when:
//!
//! - the target leaves the holder's witness set (the AoI diff in `aoi.rs`);
//! - the target is destroyed (`destroy_entity`, which every teardown path —
//!   despawn, disconnect, gate travel, instance teardown — runs through);
//! - the target respawns (`npc_respawn`): same id, new life, new place;
//! - the target dies, for the killer only, beside the `onTargetUpdate(0)`
//!   reticle drop the death burst already sent (`abilities::death`).
//!
//! Only AoI leave and destroy go without a client update: the client
//! destroys the entity on `LeftAoI` and loses its selection with it. The
//! respawn and death clears are paired with `onTargetUpdate(0)` by their
//! callers, so the client and the server agree.
//!
//! Death does not clear anyone else's target: a player looting a corpse
//! still has it selected.

use cimmeria_common::EntityId;
use cimmeria_entity::cell_entity::CellEntity;

use super::{EntityNames, SpaceManager};

/// Drop `holder`'s stored target if it is `gone`. Returns whether it did.
///
/// One debug line per clear, with `reason=` naming the event, so "why did my
/// target vanish?" is one SigNoz query. Default module target: it keeps the
/// line in `aoi.log` and in the `cimmeria_cell_world` OTLP directive.
pub(super) fn drop_target_if(
    holder_id: u32,
    holder: &mut CellEntity,
    gone: u32,
    gone_name: Option<&str>,
    reason: &'static str,
) -> bool {
    let Ok(gone_i32) = i32::try_from(gone) else {
        return false;
    };
    if holder.current_target_id != Some(gone_i32) {
        return false;
    }
    holder.current_target_id = None;
    tracing::debug!(
        holder_id,
        holder_name = EntityNames::of(holder).entity_name,
        target_id = gone,
        target_name = gone_name,
        reason,
        "stored target cleared"
    );
    true
}

impl SpaceManager {
    /// Clear every stored target in `gone`'s space that points at `gone`,
    /// and return the holders whose target was cleared.
    ///
    /// Call while `gone` is still resident: the space is found through it.
    pub fn clear_targets_on(&mut self, gone: u32, reason: &'static str) -> Vec<u32> {
        let Some(&space_id) = self.entity_space.get(&gone) else {
            return Vec::new();
        };
        let Some(space) = self.spaces.get_mut(&space_id) else {
            return Vec::new();
        };
        // Named only when the clear line is on: this runs on every destroy.
        let gone_name = if tracing::enabled!(tracing::Level::DEBUG) {
            space
                .entities
                .get(&gone)
                .and_then(|e| EntityNames::of(e).entity_name)
        } else {
            None
        };
        let mut cleared = Vec::new();
        for (&holder_id, holder) in space.entities.iter_mut() {
            if drop_target_if(holder_id, holder, gone, gone_name, reason) {
                cleared.push(holder_id);
            }
        }
        cleared
    }

    /// Clear `holder`'s stored target if it is `target`. Returns whether it
    /// did. Used by the death burst for the killer alone.
    pub fn clear_target_of(&mut self, holder: u32, target: u32, reason: &'static str) -> bool {
        let target_name = self.entity_names(target).entity_name;
        self.get_entity_mut(holder)
            .is_some_and(|h| drop_target_if(holder, h, target, target_name, reason))
    }

    /// Is `target` something `viewer` can legitimately have selected: itself,
    /// or an entity in its witness set (its AoI)?
    ///
    /// The check GM target resolution runs before acting on a stored target,
    /// so a target that somehow outlived its AoI still cannot be acted on.
    pub fn target_in_view(&self, viewer: u32, target: u32) -> bool {
        if viewer == target {
            return true;
        }
        let Ok(t) = i32::try_from(target) else {
            return false;
        };
        self.get_entity(viewer)
            .is_some_and(|v| v.witnesses.contains(&EntityId(t)))
    }
}
