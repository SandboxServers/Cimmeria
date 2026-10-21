//! The client-authoritative position path: validation, snap-back, and
//! off-mesh recovery.
//!
//! Split out of `entities.rs` (which keeps entity lifecycle) because this
//! is a distinct concern with its own state — the movement validator, the
//! snap-back correction budget, and the terminal-fallback resolver.
//!
//! # The correction contract
//!
//! A rejected client position is never written. The client is told to snap
//! back via `BASEMSG_FORCED_POSITION` (the caller turns
//! [`ClientMoveOutcome::Rejected`] into `CellToBaseMsg::TeleportPlayer`),
//! and witnesses keep seeing the unchanged cell-entity position on the next
//! AoI tick.
//!
//! That only terminates if the snap target is a position the validator
//! would itself accept. When it is not — the entity's authoritative
//! position is off-navmesh or outside the space AABB, which a
//! server-authoritative write such as a GM `.gotoxyz`, a content teleport,
//! or a stale persisted position can produce — the client snaps to the bad
//! point, re-reports it, gets rejected again, and rubber-bands at its own
//! update rate until the player gives up. [`SpaceManager::reject_outcome`]
//! is the gate that stops that: it checks the snap target before promising
//! it, relocates the entity to a real safe point when the target is
//! unusable, and stops emitting corrections entirely once the budget in
//! [`MovementValidator::MAX_SNAP_BACK_CORRECTIONS`] is spent.

use std::time::Instant;

use cimmeria_commands::permissions::AccessLevel;
use cimmeria_common::Vector3;
use cimmeria_entity::movement_validation::{
    position_within_bounds, MovementReject, MovementValidator, SpaceBounds, SPEED_WINDOW,
};

use super::movement_lock;
use super::SpaceManager;
use crate::cell::dispatch::access_level_from_u32;

/// How far a resolved recovery point must actually be from where the entity
/// already is, in world units, before relocating it counts as progress.
///
/// The navmesh branch of [`SpaceManager::resolve_recovery_position`] answers
/// through Detour's nearest-poly search and detail-mesh height interpolation,
/// so a point that is already on the mesh comes back *nearly* — but almost
/// never exactly — where it started. An exact-inequality test therefore reads
/// float noise as a successful relocation. Well below anything a player can
/// perceive, and far below the agent-radius gap that makes a point read as
/// off-mesh in the first place, so a genuine recovery always clears it.
const RECOVERY_MIN_DISPLACEMENT: f32 = 0.05;

/// Outcome of `SpaceManager::apply_client_position_update`.
#[derive(Debug)]
pub enum ClientMoveOutcome {
    /// Position passed validation and was written. Carries the new
    /// position so callers that want to log it can do so without
    /// re-querying the entity.
    Accepted { position: [f32; 3] },
    /// Position failed validation and the cell entity is unchanged. The
    /// caller must emit `CellToBaseMsg::TeleportPlayer` so the offending
    /// client snaps back to `last_valid` — which has been checked to be a
    /// position this validator would itself accept.
    Rejected {
        reason: MovementReject,
        last_valid: [f32; 3],
        space_id: u32,
        /// The bounds the proposed position was tested against. Carried
        /// out so the caller's structured log can include `bounds_min`
        /// and `bounds_max` per the negative-logging convention.
        bounds: SpaceBounds,
    },
    /// Position failed validation **and** the entity's own authoritative
    /// position was unusable as a snap target. The entity has already been
    /// relocated to `recovered_to`, a terminal safe point; the caller snaps
    /// the client there instead of re-issuing the correction that was
    /// looping.
    ///
    /// Unusable is the *only* trigger. An entity standing somewhere the
    /// validator would itself accept is never relocated, however many
    /// strikes it has accrued — an exhausted budget on a sound position is
    /// [`Self::CorrectionSuppressed`]. Relocating there instead would be a
    /// server-initiated move of a player who never left a legal point, and
    /// (because recovery clears the budget) would make the budget
    /// unenforceable on any navmesh-backed world.
    Recovered {
        reason: MovementReject,
        /// The unusable position the entity was stuck at.
        from: [f32; 3],
        /// Where it now is — written to the cell entity before returning.
        recovered_to: [f32; 3],
        space_id: u32,
    },
    /// Position failed validation, the snap target is unusable, and no
    /// safe point could be resolved. The caller must **not** emit another
    /// `BASEMSG_FORCED_POSITION`: doing so is what produced the rubber-band
    /// loop. The cell entity is left alone and the next accepted position
    /// clears the state.
    CorrectionSuppressed {
        reason: MovementReject,
        from: [f32; 3],
        space_id: u32,
        /// Consecutive rejects for this entity, for the operator log.
        strikes: u32,
    },
    /// The entity is not currently in any space — likely a stale
    /// inbound packet that arrived after destroy / disconnect. The
    /// caller can safely no-op; the original `update_entity_position`
    /// silently dropped the same shape.
    EntityMissing,
}

impl SpaceManager {
    /// Apply a client-authoritative position update through the
    /// movement validator.
    ///
    /// This is the **only** seam that should be called from the inbound
    /// `BaseToCellMsg::EntityMove` handler — every other position
    /// mutation in the cell is server-authoritative (ring transport,
    /// respawn, content-engine teleport, NPC movement) and goes through
    /// the unchecked [`SpaceManager::update_entity_position`] directly.
    ///
    /// All four validation layers run here, in cheapest-first order:
    /// bounds (catches NaN / infinity / absurd coordinates and the
    /// Z-floor-clip), navmesh containment (off-walkable-polygon), then
    /// the stateful speed/teleport kinematics. Bounds and teleport hard-
    /// reject; speed is warn-only (logged + counted, still accepted);
    /// navmesh hard-rejects for ordinary players and is warn-only for a
    /// GM (see [`SpaceManager::apply_client_position_update_at`]). The
    /// `spaceId` cross-check lives in the `EntityMove` handler, where the
    /// client-claimed space id is in hand.
    ///
    /// Production callers use this 4-arg form (server `Instant::now()`);
    /// the time-injected [`SpaceManager::apply_client_position_update_at`]
    /// backs it so the speed/teleport layer is deterministic under test.
    pub fn apply_client_position_update(
        &mut self,
        entity_id: u32,
        position: [f32; 3],
        direction: [i8; 3],
        velocity: [f32; 3],
    ) -> ClientMoveOutcome {
        self.apply_client_position_update_at(
            Instant::now(),
            entity_id,
            position,
            direction,
            velocity,
        )
    }

    /// Time-injected core of [`SpaceManager::apply_client_position_update`].
    /// `now` is the server's monotonic processing time, threaded in so unit
    /// tests can drive the speed/teleport layer with controlled deltas.
    pub fn apply_client_position_update_at(
        &mut self,
        now: Instant,
        entity_id: u32,
        position: [f32; 3],
        direction: [i8; 3],
        velocity: [f32; 3],
    ) -> ClientMoveOutcome {
        let space_id = match self.entity_space.get(&entity_id) {
            Some(&id) => id,
            None => return ClientMoveOutcome::EntityMissing,
        };

        // Source bounds from the active space's navmesh if present; fall
        // back to the generous default for spaces without a loaded
        // navmesh (most non-Castle zones today). The fallback is wider
        // than any legitimate world by an order of magnitude — see
        // `SpaceBounds::FALLBACK`.
        let (bounds, last_valid, movement_unrestricted, is_gm, top_speed, held) = {
            let space = match self.spaces.get(&space_id) {
                Some(s) => s,
                None => return ClientMoveOutcome::EntityMissing,
            };
            let bounds = match &space.navmesh {
                Some(nav) => SpaceBounds::new(nav.bmin, nav.bmax),
                None => SpaceBounds::FALLBACK,
            };
            let entity = match space.entities.get(&entity_id) {
                Some(e) => e,
                None => return ClientMoveOutcome::EntityMissing,
            };
            let last_valid = [entity.position.x, entity.position.y, entity.position.z];
            (
                bounds,
                last_valid,
                entity.movement_unrestricted,
                // Through the canonical clamped conversion, not a raw
                // discriminant comparison: `access_level_from_u32` is the one
                // mapping every other GM gate in the cell uses, and a `>=` on
                // the raw column silently diverges from it the moment
                // `AccessLevel`'s discriminants are reordered or a level is
                // inserted.
                access_level_from_u32(entity.access_level).can_execute(AccessLevel::GameMaster),
                // Scale the class baseline by this entity's own
                // `movementSpeedMod`, the same stat the NPC path-stepping tick
                // scales by. A GM `.speed 300` (or any future haste/snare
                // effect) raises the client's own prediction, so measuring its
                // packets against the unscaled constant would warn on movement
                // the server itself authorised — and would hard-reject it once
                // the speed layer is promoted past warn-only.
                MovementValidator::DEFAULT_TOP_SPEED * entity.stats.movement_speed_scale(),
                movement_lock::movement_held(entity),
            )
        };

        let proposed = Vector3::new(position[0], position[1], position[2]);

        // Advance the per-entity processing clock up front and recover the
        // previous sample, *before* any layer can short-circuit. This is
        // what stops an attacker from spamming cheaply-rejected
        // (out-of-bounds / off-navmesh) packets to inflate `dt`, then
        // slipping one large jump past the teleport gate at an
        // artificially low implied speed. Every processed packet advances
        // the clock by exactly one tick regardless of which layer rejects.
        // Kept running even on the GM bypass path below so the clock stays
        // fresh for when physics is re-enabled (a stale clock would either
        // produce a bogus dt or, worse, get skipped entirely and leave the
        // next real check comparing against a multi-minute-old sample).
        let prev_sample = self.movement_validator.touch_clock(entity_id, now);

        // Non-finite coordinates are rejected unconditionally, even under
        // the GM physics bypass below — this is deliberately NOT folded
        // into the `movement_unrestricted` branch. `update_entity_position`
        // does no sanitization of its own; if a NaN/Infinity slipped
        // through while unrestricted, it would get written straight into
        // `cell_entity.position`. That doesn't corrupt the spatial grid
        // (float->int cell indexing saturates), but it poisons this
        // entity's own kinematics state: `check_kinematics`'s
        // `distance_to` against a NaN last-position is NaN, and every NaN
        // comparison (including `distance > TELEPORT_JUMP_UNITS`) is
        // `false` under IEEE754 — so the hard teleport-reject would
        // silently and permanently stop firing for this entity the moment
        // physics was restored. No legitimate fly/ghost movement needs a
        // non-finite coordinate.
        if !proposed.x.is_finite() || !proposed.y.is_finite() || !proposed.z.is_finite() {
            return self.reject_outcome(
                entity_id,
                MovementReject::OutOfBounds,
                last_valid,
                space_id,
                bounds,
                is_gm,
            );
        }

        // GM movement-validator bypass (`onPhysics` / `/gmsetfly` /
        // `/gmsetghost` — see `console::gm::physics`). The client is
        // already authoritative for its own position while flying/
        // ghosting; skip straight past the remaining rejection layers
        // below but still advance the entity's tracked position (spatial
        // grid, AoI source-of-truth) so witnesses see the GM move and so
        // validation resumes cleanly (no stale `last_valid`) once physics
        // is restored.
        if movement_unrestricted {
            self.movement_validator.clear_speed_window(entity_id);
            return self.accept(now, entity_id, position, direction, velocity);
        }

        // Server hold (minigame, prison boot): the slack is measured from the
        // anchor taken when the hold was first seen, not from `last_valid`, which
        // every accepted packet moves; per-packet slack would let a modified
        // client creep. Game masters are exempt (no anchor is kept for them).
        if let Some(anchor) =
            self.movement_validator
                .hold_anchor(entity_id, held && !is_gm, last_valid)
        {
            if let Some(reason) = movement_lock::check(position, anchor) {
                return self.reject_outcome(entity_id, reason, last_valid, space_id, bounds, is_gm);
            }
        }

        // Layer 1 — bounds (also the Z-axis floor-clip / NaN / infinity gate).
        if let Err(reason) = self.movement_validator.check_bounds(proposed, &bounds) {
            return self.reject_outcome(entity_id, reason, last_valid, space_id, bounds, is_gm);
        }

        // Layer 4 — navmesh containment. Gated on
        // `enforces_navmesh_containment`, which is false both for a space
        // with no loaded navmesh and for an `advisory` world, so this is a
        // no-op in either and the bounds AABB remains the only spatial
        // gate. Checked before kinematics so an off-mesh point is reported
        // as `OffNavmesh` regardless of how far it is from the last
        // position.
        //
        // The predicate is consulted *first* so an advisory world does not
        // pay for a Detour query per inbound packet for an answer nothing
        // would act on.
        if self.enforces_navmesh_containment(space_id)
            && !self.is_position_valid(entity_id, &proposed)
        {
            // A GM is allowed off the walkable mesh: standing inside
            // geometry, on a rooftop, or hovering over a gap is how you
            // inspect a broken spawn or an unreachable region. Downgraded
            // to warn-only rather than routed through `movement_unrestricted`
            // so it needs no in-game toggle and cannot be self-asserted —
            // `access_level` is read from `account.accesslevel` at login and
            // never from a client byte. The bounds and teleport layers still
            // hard-reject, so a GM still cannot write a NaN or an absurd
            // coordinate into the spatial grid.
            if is_gm {
                // Who the GM is, not just which slot they occupy: this line is
                // the audit trail for a privileged player standing somewhere an
                // ordinary player is snapped back from.
                let id = self.player_identity(entity_id);
                // The counter sees every packet; only the row is throttled.
                cimmeria_observability::counter!(
                    "movement_validation_warns_total",
                    "reason" => "navmesh_gm_bypass",
                );
                // INFO, not WARN: a GM standing off-mesh is intended, so the
                // row is an audit trail. One per GM per window; `suppressed`
                // counts the packets in between.
                if let Some(suppressed) = self.movement_telemetry.gm_bypass_log.admit(
                    entity_id,
                    "navmesh_gm_bypass",
                    now,
                    super::movement_telemetry::GM_BYPASS_LOG_INTERVAL,
                ) {
                    // `world`, like every other `movement.validation` row
                    // whose space id resolves: a dashboard filtered by world
                    // must not silently drop the GM allowance rows, which
                    // are the audit trail for a privileged player standing
                    // where an ordinary one is snapped back from.
                    let world = self.world_name_for_space(space_id);
                    tracing::info!(
                        target: "movement.validation",
                        entity_id,
                        entity_name = id.player_name,
                        account_id = id.account_id,
                        account_name = id.account_name,
                        player_id = id.player_id,
                        player_name = id.player_name,
                        space_id,
                        world,
                        client_x = position[0],
                        client_y = position[1],
                        client_z = position[2],
                        reason = "navmesh_gm_bypass",
                        suppressed,
                        "movement.navmesh_gm_bypass: off-navmesh position accepted for a \
                         GM caller (ordinary players are snapped back)"
                    );
                }
            } else {
                return self.reject_outcome(
                    entity_id,
                    MovementReject::OffNavmesh,
                    last_valid,
                    space_id,
                    bounds,
                    is_gm,
                );
            }
        } else if !self.enforces_navmesh_containment(space_id)
            // TRACE, and level-gated before the query: in an advisory world
            // this would otherwise run a Detour lookup on every inbound
            // position packet for every player to produce a line nobody is
            // collecting. With the level on, it is the only record of *which*
            // parts of a world's mesh real players walk through — the input a
            // mesh rebake (GH1) actually needs, as opposed to a static probe.
            && tracing::enabled!(target: "movement.navmesh", tracing::Level::TRACE)
            && self.spaces.get(&space_id).is_some_and(|s| s.navmesh.is_some())
            && !self.is_position_valid(entity_id, &proposed)
        {
            // NA25 made this row reachable (the `cimmeria-trace` index
            // enables `movement.navmesh` at TRACE), so it now fires on every
            // off-mesh packet. Throttled per player; `suppressed` keeps the
            // packet count.
            if let Some(suppressed) = self.movement_telemetry.advisory_off_mesh_log.admit(
                entity_id,
                "advisory_off_mesh",
                now,
                super::movement_telemetry::ADVISORY_OFF_MESH_LOG_INTERVAL,
            ) {
                let world = self.world_name_for_space(space_id);
                tracing::trace!(
                    target: "movement.navmesh",
                    entity_id,
                    entity_name = self.entity_label(entity_id),
                    space_id,
                    world,
                    client_x = position[0],
                    client_y = position[1],
                    client_z = position[2],
                    reason = "advisory_off_mesh_accepted",
                    suppressed,
                    "movement.navmesh: position is off this world's navmesh and was \
                     accepted anyway -- the world is 'advisory', so the mesh is not a \
                     containment gate here"
                );
            }
        }

        // Layers 2+3 — speed (warn-only) + teleport (hard reject). Measured
        // against the entity's current authoritative position.
        let last_pos = Vector3::new(last_valid[0], last_valid[1], last_valid[2]);
        let kin = self.movement_validator.check_kinematics(
            now,
            prev_sample,
            last_pos,
            proposed,
            top_speed,
        );
        if let Some(reason) = kin.reject {
            return self.reject_outcome(entity_id, reason, last_valid, space_id, bounds, is_gm);
        }
        // Speed is judged over a per-entity window (#1249), not per packet:
        // client send jitter (25 ms then 175 ms) would otherwise read as 4x
        // top speed. Needs a time baseline; a rejected move never gets here.
        let sample = prev_sample.and_then(|prev| {
            let dt = now.saturating_duration_since(prev).as_secs_f32();
            self.movement_validator.observe_speed(
                entity_id,
                proposed.distance_to(&last_pos),
                dt,
                top_speed,
            )
        });
        if let Some(sample) = sample {
            // Warn-only: the move is accepted. The fields are window sums, so
            // the SigNoz tolerance-calibration pipeline sees what the verdict
            // was made on before the layer is ever promoted to snap-back.
            let id = self.player_identity(entity_id);
            let world = self.world_name_for_space(space_id);
            tracing::warn!(
                target: "movement.validation",
                entity_id,
                entity_name = id.player_name,
                account_id = id.account_id,
                account_name = id.account_name,
                player_id = id.player_id,
                player_name = id.player_name,
                space_id,
                world,
                client_x = position[0],
                client_y = position[1],
                client_z = position[2],
                distance = sample.distance,
                dt_secs = sample.dt_secs,
                implied_speed = sample.implied_speed,
                window_secs = sample.dt_secs.max(SPEED_WINDOW.as_secs_f32()),
                top_speed = sample.top_speed,
                ratio = sample.implied_speed / sample.top_speed,
                reason = "speed",
                "movement.speed_warning: client move exceeded speed tolerance \
                 (warn-only — accepted; calibrate before enforcing)"
            );
            cimmeria_observability::counter!(
                "movement_validation_warns_total",
                "reason" => "speed",
            );
        }

        self.accept(now, entity_id, position, direction, velocity)
    }

    /// Write an accepted client position and clear the correction budget.
    ///
    /// `now` is the caller's server processing instant, threaded through
    /// rather than re-read: the sampler below stamps its window with it,
    /// so reading `Instant::now()` here would both record a sample at a
    /// slightly different time than the packet was processed at and put
    /// the 5 s window out of reach of the time-injected
    /// [`SpaceManager::apply_client_position_update_at`] tests.
    fn accept(
        &mut self,
        now: Instant,
        entity_id: u32,
        position: [f32; 3],
        direction: [i8; 3],
        velocity: [f32; 3],
    ) -> ClientMoveOutcome {
        self.update_entity_position(entity_id, position, direction, velocity);
        self.movement_validator.clear_rejects(entity_id);
        // Observability only, and self-throttled to ~1 row per player per
        // 5 s of actual movement (NPCs never sample). The positive-space
        // counterpart to the reject log: without it we know where players
        // are stopped and nothing about where they successfully walk,
        // which is what finds a navmesh hole before somebody falls in.
        self.sample_accepted_position_at(entity_id, position, now);
        ClientMoveOutcome::Accepted { position }
    }

    /// Decide what a hard reject actually does, given that the obvious
    /// answer — "snap the client back to where the server thinks it is" —
    /// is only correct when the server's own position is somewhere the
    /// client can legally be.
    ///
    /// Three outcomes, in order of preference:
    ///
    /// 1. The snap target is sound and the budget is intact →
    ///    [`ClientMoveOutcome::Rejected`], the ordinary correction.
    /// 2. The snap target is unusable and a safe point exists → relocate the
    ///    entity there and report [`ClientMoveOutcome::Recovered`].
    /// 3. The budget is spent, or there is nothing safe to move to →
    ///    [`ClientMoveOutcome::CorrectionSuppressed`], and the caller emits
    ///    nothing at all.
    ///
    /// `is_gm` carries the same off-navmesh allowance the accept path grants
    /// (see [`SpaceManager::apply_client_position_update_at`]) into the
    /// soundness test. Without it a GM standing legitimately off-mesh who
    /// then trips an *unrelated* hard reject — bounds or teleport, both still
    /// enforced for GMs — would have their own position judged unusable and
    /// be force-relocated onto the nearest walkable point, undoing the
    /// allowance by the back door.
    fn reject_outcome(
        &mut self,
        entity_id: u32,
        reason: MovementReject,
        last_valid: [f32; 3],
        space_id: u32,
        bounds: SpaceBounds,
        is_gm: bool,
    ) -> ClientMoveOutcome {
        let last_pos = Vector3::new(last_valid[0], last_valid[1], last_valid[2]);
        let strikes = self.movement_validator.note_reject(entity_id);
        // Same three-way allowance as the accept path: a GM may stand off the
        // mesh, and in an advisory (or meshless) world *everyone* may, so the
        // AABB is the whole soundness test there. Without the
        // `enforces_navmesh_containment` term an advisory world would still
        // judge its own players' positions unusable the moment an unrelated
        // layer (bounds, teleport) rejected a packet, and force-relocate them
        // onto the nearest mesh polygon — reintroducing the snap this packet
        // removes, by the back door and from a different code path.
        let target_is_sound = position_within_bounds(last_pos, &bounds)
            && (is_gm
                || !self.enforces_navmesh_containment(space_id)
                || self.is_position_valid(entity_id, &last_pos));

        if target_is_sound {
            // Nothing to recover *from*: the entity is already somewhere the
            // validator accepts, so either correct the client back to it or —
            // once the budget is spent — stop emitting corrections. Falling
            // through to `resolve_recovery_position` here is what made the
            // budget unenforceable on navmesh-backed worlds: reprojecting a
            // sound point through Detour returns a near-identical (but rarely
            // bit-identical) point, which read as a successful relocation,
            // cleared the budget, and let the correction stream run forever.
            return if strikes <= MovementValidator::MAX_SNAP_BACK_CORRECTIONS {
                ClientMoveOutcome::Rejected {
                    reason,
                    last_valid,
                    space_id,
                    bounds,
                }
            } else {
                ClientMoveOutcome::CorrectionSuppressed {
                    reason,
                    from: last_valid,
                    space_id,
                    strikes,
                }
            };
        }

        match self.resolve_recovery_position(space_id, last_pos, &bounds) {
            // Same reason the sound-target branch above short-circuits:
            // compare by displacement, not by float equality, so a
            // reprojection that lands back where the entity already is counts
            // as "no better place to put it" rather than as a relocation.
            Some(safe)
                if Vector3::new(safe[0], safe[1], safe[2]).distance_to(&last_pos)
                    > RECOVERY_MIN_DISPLACEMENT =>
            {
                // Position-only write: a recovery relocation moves the entity
                // out of an unusable spot, it does not re-face it. (The
                // `direction`-writing sibling would zero the facing here.)
                self.update_position_preserving_facing(entity_id, safe, [0.0; 3]);
                // The relocation is a server-authoritative teleport: reseed
                // the clock so the client's first post-recovery packet is
                // measured from now, and clear the budget because the entity
                // once again has a position the validator accepts. Both are
                // `note_authorized_teleport`'s job.
                self.note_authorized_teleport(entity_id);
                ClientMoveOutcome::Recovered {
                    reason,
                    from: last_valid,
                    recovered_to: safe,
                    space_id,
                }
            }
            _ => ClientMoveOutcome::CorrectionSuppressed {
                reason,
                from: last_valid,
                space_id,
                strikes,
            },
        }
    }

    /// Reseed an entity's movement-validator clock after a server-
    /// authoritative position write (ring transport, respawn, gate
    /// arrival, content-engine teleport, GM travel). Suppresses a
    /// spurious speed warn on the first post-teleport client packet; see
    /// [`MovementValidator::note_authorized_teleport`].
    pub fn note_authorized_teleport(&mut self, entity_id: u32) {
        self.log_authorized_teleport(entity_id);
        crate::cell::player_journal::note(
            entity_id,
            crate::cell::player_journal::kinds::TELEPORT,
            "authorized",
        );
        crate::cell::playtest_friction::leader_teleported(self, entity_id);
        self.movement_validator
            .note_authorized_teleport(entity_id, Instant::now());
    }
}
