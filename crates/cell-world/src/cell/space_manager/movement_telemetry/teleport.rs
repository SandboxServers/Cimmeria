//! The `movement.teleport` row: one line per **server-authoritative** move.
//!
//! Every path that writes an entity's position on the server's own say-so
//! (GM travel and summon, content-engine teleports, ring transport, respawn,
//! gate arrival, snap-back recovery) ends in
//! [`SpaceManager::note_authorized_teleport`], after the position is written.
//! That call is the choke point, so the row is emitted from there and from
//! nowhere else. A per-caller copy would drift the first time a new teleport
//! path forgot it.
//!
//! The row is event-rate, never tick-rate: nothing calls
//! `note_authorized_teleport` from a per-tick loop.

use cimmeria_common::Vector3;

use super::super::SpaceManager;
use super::UNKNOWN_WORLD;

/// `navmesh_mode` value for a space with no resident mesh. `enforce` and
/// `advisory` come from [`super::super::NavmeshMode::as_db_str`].
const NAVMESH_MODE_NONE: &str = "none";

impl SpaceManager {
    /// Report one server-authoritative move of `entity_id` to the position
    /// it now holds.
    ///
    /// DEBUG `movement.teleport` `event = "authorized_teleport"` always. A
    /// **player** moved off the walkable mesh in a space that enforces
    /// navmesh containment also gets a WARN `reason = "teleport_dest_off_mesh"`
    /// and a `movement_teleport_dest_off_mesh_total{world}` increment: the
    /// validator will refuse that player's very next client packet, so this
    /// is the row that names the teleport that put them there, instead of the
    /// snap-back loop that follows.
    pub(crate) fn log_authorized_teleport(&self, entity_id: u32) {
        let Some(entity) = self.get_entity(entity_id) else {
            return;
        };
        let pos = entity.position;
        let space_id = self.get_entity_space_id(entity_id);
        let is_player = entity.is_player
            || space_id
                .and_then(|sid| self.spaces.get(&sid))
                .is_some_and(|s| s.players.contains(&entity_id));
        let world = space_id
            .and_then(|sid| self.world_name_for_space(sid))
            .unwrap_or(UNKNOWN_WORLD);
        let has_mesh = space_id
            .and_then(|sid| self.spaces.get(&sid))
            .is_some_and(|s| s.navmesh.is_some());
        let navmesh_mode = if has_mesh {
            self.navmesh_mode(world).as_db_str()
        } else {
            NAVMESH_MODE_NONE
        };
        // `None` for a meshless space, which leaves the field out.
        let dest_on_mesh = self
            .diagnose_point(entity_id, &Vector3::new(pos.x, pos.y, pos.z))
            .map(|v| v.valid);
        let enforced = space_id.is_some_and(|sid| self.enforces_navmesh_containment(sid));
        let identity = self.player_identity(entity_id);
        let entity_name = self.entity_label(entity_id);

        tracing::debug!(
            target: "movement.teleport",
            event = "authorized_teleport",
            entity_id,
            entity_name,
            account_id = identity.account_id,
            account_name = identity.account_name,
            player_id = identity.player_id,
            player_name = identity.player_name,
            space_id,
            world,
            x = pos.x,
            y = pos.y,
            z = pos.z,
            navmesh_mode,
            dest_on_mesh,
            is_player,
            "movement.teleport: server-authoritative move written"
        );

        if is_player && enforced && dest_on_mesh == Some(false) {
            cimmeria_observability::counter!(
                "movement_teleport_dest_off_mesh_total",
                "world" => world.to_string(),
            );
            tracing::warn!(
                target: "movement.teleport",
                event = "authorized_teleport",
                reason = "teleport_dest_off_mesh",
                entity_id,
                entity_name,
                account_id = identity.account_id,
                account_name = identity.account_name,
                player_id = identity.player_id,
                player_name = identity.player_name,
                space_id,
                world,
                x = pos.x,
                y = pos.y,
                z = pos.z,
                navmesh_mode,
                "movement.teleport: player moved off the walkable mesh in an \
                 enforce world; the validator will reject its next position"
            );
        }
    }
}
