//! The positive-space counterpart to [`super::reject`]: a low-rate
//! sample of positions the validator **accepted**.

use std::time::Instant;

use cimmeria_common::Vector3;

use super::super::SpaceManager;
use super::{
    PositionSample, POSITION_SAMPLE_MIN_DISTANCE, POSITION_SAMPLE_MIN_INTERVAL, UNKNOWN_WORLD,
};

impl SpaceManager {
    /// Low-rate positive sample of an **accepted** player position.
    ///
    /// Rejects tell us where players are stopped; nothing told us where
    /// they successfully walk. Without that, a navmesh hole is only
    /// visible once somebody falls into it — there is no map of the
    /// surface that actually works. One sample per player per
    /// [`POSITION_SAMPLE_MIN_INTERVAL`], and only after they have moved
    /// [`POSITION_SAMPLE_MIN_DISTANCE`], builds that map from ordinary
    /// play.
    ///
    /// `now` is the caller's server processing instant, not
    /// `Instant::now()` read here: the accept path already has one, the
    /// sample should be stamped with the time the packet was processed
    /// rather than the time the sampler happened to run, and a
    /// time-injected test cannot drive the 5 s window otherwise.
    ///
    /// **Volume.** 1 row / 5 s / moving player = 720 rows/hour/player; at
    /// 20 concurrent players, 14,400 rows/hour. For scale, the reject
    /// stream this throttles was running at ~2,000 rows/hour on its
    /// own. Emitted at DEBUG, matching the sibling `movement.player`
    /// sample — see
    /// `docs/architecture/instrumentation-discipline.md`.
    ///
    /// **Players only.** NPC positions are already covered by
    /// `movement.npc` and `npc_ai.tick`, and NPCs outnumber players by
    /// an order of magnitude in a populated zone — sampling them would
    /// swamp the signal this exists to produce.
    pub(crate) fn sample_accepted_position_at(
        &mut self,
        entity_id: u32,
        position: [f32; 3],
        now: Instant,
    ) {
        let pos = Vector3::new(position[0], position[1], position[2]);

        // Cheapest gate first: NPCs never sample, and this is the accept
        // path of every inbound position packet.
        //
        // Two independent player signals, same as the despawn path:
        // `is_player` is stamped by `connect_entity`, and space
        // membership in `players` is the other half. Checking both means
        // a position update that somehow arrives before the flag is
        // stamped still samples, rather than the player silently
        // contributing nothing to the walked-surface map for their
        // first packets.
        let space_id = self.get_entity_space_id(entity_id);
        let is_player = self.get_entity(entity_id).is_some_and(|e| e.is_player)
            || space_id
                .and_then(|sid| self.spaces.get(&sid))
                .is_some_and(|s| s.players.contains(&entity_id));
        if !is_player {
            return;
        }

        match self.movement_telemetry.position_samples.get(&entity_id) {
            Some(prev)
                if now.saturating_duration_since(prev.at) < POSITION_SAMPLE_MIN_INTERVAL
                    || prev.position.distance_to(&pos) < POSITION_SAMPLE_MIN_DISTANCE =>
            {
                return;
            }
            _ => {}
        }

        let world = space_id
            .and_then(|sid| self.world_name_for_space(sid))
            .unwrap_or(UNKNOWN_WORLD)
            .to_string();
        // `None` = meshless space (nothing to be on or off), which the
        // absent field encodes correctly. `Some(false)` = there is a
        // mesh and this accepted position is not on it — which for a
        // non-GM player should be impossible, and is therefore one of
        // the more interesting rows this sampler can produce.
        let verdict = self.diagnose_point(entity_id, &pos);
        let navmesh_hash = self.navmesh_short_hash(entity_id).map(str::to_owned);
        let identity = self.player_identity(entity_id);

        self.movement_telemetry.position_samples.insert(
            entity_id,
            PositionSample {
                at: now,
                position: pos,
            },
        );

        tracing::debug!(
            target: "movement.position_sample",
            event = "position_sample",
            entity_id,
            account_id = identity.account_id,
            player_id = identity.player_id,
            space_id,
            world = %world,
            x = position[0],
            y = position[1],
            z = position[2],
            on_navmesh = verdict.map(|v| v.valid),
            nav_dy = verdict.and_then(|v| v.dy),
            navmesh_hash = navmesh_hash.as_deref(),
            "movement.position_sample: accepted player position (sampled) — \
             the positive-space counterpart to movement.validation_reject"
        );
    }
}
