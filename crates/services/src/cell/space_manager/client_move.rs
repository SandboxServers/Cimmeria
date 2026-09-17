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
    position_within_bounds, MovementReject, MovementValidator, SpaceBounds,
};

use super::SpaceManager;

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
        let (bounds, last_valid, movement_unrestricted, is_gm, top_speed) = {
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
                entity.access_level >= AccessLevel::GameMaster as u32,
                // Scale the class baseline by this entity's own
                // `movementSpeedMod`, the same stat the NPC path-stepping tick
                // scales by. A GM `.speed 300` (or any future haste/snare
                // effect) raises the client's own prediction, so measuring its
                // packets against the unscaled constant would warn on movement
                // the server itself authorised — and would hard-reject it once
                // the speed layer is promoted past warn-only.
                MovementValidator::DEFAULT_TOP_SPEED * entity.stats.movement_speed_scale(),
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
        // `/gmsetghost` — see `cell_methods::gm::physics`). The client is
        // already authoritative for its own position while flying/
        // ghosting; skip straight past the remaining rejection layers
        // below but still advance the entity's tracked position (spatial
        // grid, AoI source-of-truth) so witnesses see the GM move and so
        // validation resumes cleanly (no stale `last_valid`) once physics
        // is restored.
        if movement_unrestricted {
            return self.accept(entity_id, position, direction, velocity);
        }

        // Layer 1 — bounds (also the Z-axis floor-clip / NaN / infinity gate).
        if let Err(reason) = self.movement_validator.check_bounds(proposed, &bounds) {
            return self.reject_outcome(entity_id, reason, last_valid, space_id, bounds, is_gm);
        }

        // Layer 4 — navmesh containment. `is_position_valid` fails open
        // for spaces with no loaded navmesh, so this is a no-op there and
        // the bounds AABB remains the only spatial gate. Checked before
        // kinematics so an off-mesh point is reported as `OffNavmesh`
        // regardless of how far it is from the last position.
        if !self.is_position_valid(entity_id, &proposed) {
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
                tracing::warn!(
                    target: "movement.validation",
                    entity_id,
                    account_id = id.account_id,
                    player_id = id.player_id,
                    space_id,
                    client_x = position[0],
                    client_y = position[1],
                    client_z = position[2],
                    reason = "navmesh_gm_bypass",
                    "movement.navmesh_gm_bypass: off-navmesh position accepted for a \
                     GM caller (warn-only — ordinary players are snapped back)"
                );
                cimmeria_observability::counter!(
                    "movement_validation_warns_total",
                    "reason" => "navmesh_gm_bypass",
                );
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
        if let Some(sample) = kin.speed_warn {
            // Warn-only: the move is accepted. Surface the full
            // (distance, dt, implied_speed) triple so the SigNoz
            // tolerance-calibration pipeline can compute the legitimate
            // p99.9 before the speed layer is ever promoted to snap-back.
            let id = self.player_identity(entity_id);
            tracing::warn!(
                target: "movement.validation",
                entity_id,
                account_id = id.account_id,
                player_id = id.player_id,
                space_id,
                client_x = position[0],
                client_y = position[1],
                client_z = position[2],
                distance = sample.distance,
                dt_secs = sample.dt_secs,
                implied_speed = sample.implied_speed,
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

        self.accept(entity_id, position, direction, velocity)
    }

    /// Write an accepted client position and clear the correction budget.
    fn accept(
        &mut self,
        entity_id: u32,
        position: [f32; 3],
        direction: [i8; 3],
        velocity: [f32; 3],
    ) -> ClientMoveOutcome {
        self.update_entity_position(entity_id, position, direction, velocity);
        self.movement_validator.clear_rejects(entity_id);
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
        let target_is_sound = position_within_bounds(last_pos, &bounds)
            && (is_gm || self.is_position_valid(entity_id, &last_pos));

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
                // `update_entity_position` overwrites `direction` from its
                // `[i8; 3]` parameter, so the zero below would silently
                // re-face the entity north on every recovery. Same
                // workaround the GM travel commands use.
                let facing = self.get_entity(entity_id).map(|e| e.direction);
                self.update_entity_position(entity_id, safe, [0, 0, 0], [0.0; 3]);
                if let Some(f) = facing {
                    if let Some(e) = self.get_entity_mut(entity_id) {
                        e.direction = f;
                    }
                }
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

    /// Resolve a position an entity stuck at `from` can be safely placed
    /// at. `None` means there is nothing better than where it already is.
    ///
    /// Ordered by how little it disturbs the player:
    ///
    /// 1. The nearest walkable navmesh point. This is the Z-clamp answer —
    ///    a player who ended up a metre inside the floor comes back out on
    ///    the surface they were standing on, not across the map.
    /// 2. The world's nearest authored respawn point. Already the
    ///    server's answer to "where is it safe to put this player", so
    ///    reusing it needs no new content.
    /// 3. The space AABB, clamped — the last resort when neither of the
    ///    above answers.
    ///
    /// **Every candidate is tested against the same layers that reject a
    /// client position before it is returned.** That is what makes the
    /// recovery terminal: the outcome is written through and clears the
    /// correction budget, so handing back a point the validator would itself
    /// reject just restarts the rubber-band loop one position over, with
    /// nothing left to spend. `None` — and the resulting
    /// [`ClientMoveOutcome::CorrectionSuppressed`] — is the correct answer
    /// when nothing passes.
    fn resolve_recovery_position(
        &self,
        space_id: u32,
        from: Vector3,
        bounds: &SpaceBounds,
    ) -> Option<[f32; 3]> {
        let space = self.spaces.get(&space_id)?;

        if let Some(nav) = &space.navmesh {
            let projected = nav.get_nearest_point(&from);
            if nav.is_point_valid(&projected) && position_within_bounds(projected, bounds) {
                return Some([projected.x, projected.y, projected.z]);
            }
        }

        let world = space.world_name.as_str();
        // `load_respawners` copies the DB columns in with no validation, so an
        // authored-bad respawn point is exactly as unusable a snap target as
        // the position being recovered *from* — and relocating there would
        // still `note_authorized_teleport`, clearing the correction budget and
        // leaving the next reject with nothing left to spend. Same soundness
        // test the navmesh branch above applies, so the fallback can only ever
        // be a step towards a position the validator accepts.
        let nearest_respawner = self
            .respawners
            .iter()
            .filter(|r| r.world_name == world)
            .map(|r| Vector3::new(r.pos[0], r.pos[1], r.pos[2]))
            .filter(|p| {
                position_within_bounds(*p, bounds)
                    && space
                        .navmesh
                        .as_ref()
                        .is_none_or(|nav| nav.is_point_valid(p))
            })
            .min_by(|a, b| a.distance_to(&from).total_cmp(&b.distance_to(&from)));
        if let Some(p) = nearest_respawner {
            return Some([p.x, p.y, p.z]);
        }

        let clamped = [
            from.x.clamp(bounds.min[0], bounds.max[0]),
            from.y.clamp(bounds.min[1], bounds.max[1]),
            from.z.clamp(bounds.min[2], bounds.max[2]),
        ];
        if clamped == [from.x, from.y, from.z] {
            return None;
        }
        // A clamp answers the bounds layer by construction and says nothing
        // about walkability. On a navmesh-less space that is the whole test;
        // on a navmesh-backed one it is reachable whenever the reprojection
        // above failed (a point too far outside the mesh for Detour's
        // nearest-poly search), and pulling the entity to the AABB face lands
        // it inside whatever geometry happens to be there.
        let clamped_pos = Vector3::new(clamped[0], clamped[1], clamped[2]);
        space
            .navmesh
            .as_ref()
            .is_none_or(|nav| nav.is_point_valid(&clamped_pos))
            .then_some(clamped)
    }

    /// Reseed an entity's movement-validator clock after a server-
    /// authoritative position write (ring transport, respawn, gate
    /// arrival, content-engine teleport, GM travel). Suppresses a
    /// spurious speed warn on the first post-teleport client packet; see
    /// [`MovementValidator::note_authorized_teleport`].
    pub fn note_authorized_teleport(&mut self, entity_id: u32) {
        self.movement_validator
            .note_authorized_teleport(entity_id, Instant::now());
    }
}
