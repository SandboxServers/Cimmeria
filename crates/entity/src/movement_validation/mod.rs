//! Server-authoritative movement validation primitives.
//!
//! The client is *client-authoritative* for its own avatar position — it
//! sends raw float32 positions in `AVATAR_UPDATE_*` packets and expects
//! the server to mirror them into the cell entity. Without server-side
//! validation, a tampered client can teleport, walk through walls, or
//! warp under the terrain. This module is the gate that proposed client
//! positions must clear before the cell entity's `position` field is
//! written.
//!
//! ## Layered design (all four layers live)
//!
//! 1. **Bounds** — proposed point must lie inside the active space's
//!    `[bmin, bmax]` AABB. Sourced from the loaded navmesh's bounds with
//!    a generous fallback for spaces whose navmesh failed to load.
//!    Catches NaN / infinity / absurd-coordinate overflows and the
//!    Z-axis floor-clip exploit. (`check_bounds`, [`bounds`] module)
//! 2. **Speed** *(warn-only)* — `|new - last| / dt` (server monotonic
//!    clock, never a client-supplied timestamp) must not exceed
//!    `top_speed × SPEED_WARN_TOLERANCE`. Over-tolerance moves are
//!    **logged + counted but still accepted**: the production tolerance
//!    is calibrated from SigNoz rejection telemetry before it is allowed
//!    to snap legitimate high-RTT players back. (`check_kinematics`)
//! 3. **Teleport detection** *(hard reject)* — a single update that is
//!    both farther than `TELEPORT_JUMP_UNITS` **and** implies a speed
//!    above `top_speed × TELEPORT_SPEED_FACTOR` is rejected with
//!    snap-back. The dual gate (distance AND implied-speed) lets a
//!    legitimately lagged client send one large-but-slow catch-up packet
//!    without a false snap, while still rejecting the
//!    100m-in-50ms teleport. (`check_kinematics`)
//! 4. **Navmesh containment** — the proposed point must lie on a
//!    walkable polygon (`SpaceManager::is_position_valid`, fail-open when
//!    no navmesh is loaded). Wired at the cell seam, not here, because it
//!    needs the per-space Detour handle.
//!
//! ## On "game tick, not wall-clock"
//!
//! The audit's warning is against trusting a **client-supplied**
//! timestamp to compute `dt` — that is spoofable (set `dt` huge → any
//! distance looks slow). The clock used here is the server's own
//! monotonic [`std::time::Instant`], sampled at the moment the packet is
//! processed. It is not client-influenced and is the same primitive the
//! cell's tick loop already uses. `dt` is injected into
//! [`MovementValidator::check_kinematics`] so the logic stays
//! deterministic under test.
//!
//! ## Correction strategy
//!
//! On any reject, the validator *does not* advance the cell entity's
//! position. The caller is expected to:
//!
//! 1. Skip the spatial-grid update (so AoI naturally rebroadcasts the
//!    last-valid position on the next 100 ms tick).
//! 2. Emit `BASEMSG_FORCED_POSITION` (via `CellToBaseMsg::TeleportPlayer`)
//!    so the offending client immediately snaps its own avatar back to
//!    the last-valid position.
//!
//! Disconnect is reserved for repeated violations beyond a per-session
//! threshold — out of scope for this layer.
//!
//! ## Module layout
//!
//! The stateless bounds AABB + finite/containment check live in
//! [`bounds`]; this module owns the reject taxonomy, the speed/teleport
//! telemetry types, and the stateful [`MovementValidator`]. Both layers'
//! public items are re-exported here so external callers keep using the
//! flat `crate::movement_validation::Item` paths.

mod bounds;

pub use bounds::{position_within_bounds, SpaceBounds};

use std::collections::HashMap;
use std::time::{Duration, Instant};

use cimmeria_common::Vector3;

/// Why a proposed position was rejected. Each variant maps to a hard
/// (snap-back) validation layer. The speed layer is **warn-only** and
/// therefore has no variant here — an over-tolerance-but-sub-teleport
/// move is accepted and surfaces as a [`SpeedSample`] instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementReject {
    /// Proposed position lay outside the active space's `[bmin, bmax]`
    /// AABB (or was non-finite). Carried by every reject so log capture
    /// pins the failure mode cleanly.
    OutOfBounds,
    /// Proposed position was off the walkable navmesh. Only emitted for
    /// spaces with a loaded navmesh — navmesh-less spaces fail open.
    OffNavmesh,
    /// Single-update jump that was both farther than
    /// [`MovementValidator::TELEPORT_JUMP_UNITS`] and faster than
    /// `top_speed × TELEPORT_SPEED_FACTOR` — i.e. a teleport/speed hack
    /// no legitimate lagged client could produce.
    Teleport,
}

/// Speed/teleport telemetry for a single update, returned by
/// [`MovementValidator::check_kinematics`]. Carries the raw inputs so
/// the cell seam's negative log and the SigNoz tolerance-calibration
/// pipeline have the full `(distance, dt, implied_speed)` triple.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeedSample {
    /// World-unit distance from the last accepted position.
    pub distance: f32,
    /// Server-clock seconds since the last accepted update.
    pub dt_secs: f32,
    /// `distance / dt_secs`, in world units per second.
    pub implied_speed: f32,
    /// The class top speed the sample was compared against.
    pub top_speed: f32,
}

/// Outcome of the stateful speed/teleport layer.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct KinematicsOutcome {
    /// `Some(MovementReject::Teleport)` when the move must be hard-
    /// rejected; `None` when it is accepted (possibly with a warn).
    pub reject: Option<MovementReject>,
    /// `Some(sample)` when the move exceeded the warn tolerance but was
    /// still accepted under the warn-only speed policy. The caller logs
    /// + counts it but does **not** snap the client.
    pub speed_warn: Option<SpeedSample>,
}

/// Server-authoritative movement validator.
///
/// Holds the per-entity server-clock sample the speed/teleport layer
/// needs (`move_clock`). The bounds and navmesh layers are stateless;
/// they read everything from their arguments. State is keyed by
/// `entity_id` and must be released via [`MovementValidator::forget`]
/// when the entity is destroyed.
#[derive(Debug, Default)]
pub struct MovementValidator {
    /// Server-monotonic timestamp of each entity's last *processed*
    /// client position update (accepted or hard-rejected). `dt` for the
    /// next update is `now − this`. Absence means "no prior sample" — the
    /// first update for an entity seeds the clock and skips the
    /// speed/teleport check (no baseline to measure against).
    move_clock: HashMap<u32, Instant>,
}

impl MovementValidator {
    /// Class top speed (world units / second) used as the speed/teleport
    /// baseline. Sourced from `db/resources/Worlds/Seed/worlds.sql`
    /// `run_speed`, which is `8.125` for every populated world. A
    /// per-world source (and reconciliation of the `runSpeed = 6.0`
    /// drift in `mercury/world_data`) is a documented follow-up; the
    /// warn-only speed policy and the generous tolerances below make the
    /// single constant safe in the interim (it is the *higher* of the
    /// two candidate numbers, so it cannot cause false warns relative to
    /// what the client is told).
    pub const DEFAULT_TOP_SPEED: f32 = 8.125;

    /// Warn when `implied_speed > top_speed × this`. Warn-only — no
    /// snap-back. 1.5× absorbs sprint bursts, slope/stair geometry, and
    /// RTT jitter; the real tolerance is calibrated from telemetry later.
    pub const SPEED_WARN_TOLERANCE: f32 = 1.5;

    /// Distance gate for the teleport hard-reject. A single update must
    /// move farther than this (world units) to be teleport-eligible. At
    /// `DEFAULT_TOP_SPEED` this is ~6 s of continuous running in one
    /// packet — implausible for legitimate traffic, even lagged.
    pub const TELEPORT_JUMP_UNITS: f32 = 50.0;

    /// Speed gate for the teleport hard-reject, as a multiple of
    /// `top_speed`. Combined with the distance gate via AND: a far-but-
    /// slow catch-up packet from a lagged client (large distance, small
    /// implied speed) is **not** a teleport, but a far-and-fast jump is.
    pub const TELEPORT_SPEED_FACTOR: f32 = 10.0;

    /// Construct a fresh validator with no per-entity state.
    pub fn new() -> Self {
        Self {
            move_clock: HashMap::new(),
        }
    }

    /// Bounds layer. Returns `Ok(())` on accept,
    /// `Err(MovementReject::OutOfBounds)` on reject.
    ///
    /// `bounds` is sourced by the caller: prefer the active space's
    /// navmesh `bmin`/`bmax`, fall back to `SpaceBounds::FALLBACK` when
    /// the navmesh is unloaded. This split keeps the validator pure
    /// (no I/O, no navmesh handle plumbing) — the caller already has
    /// the space in hand and can decide which source to use.
    pub fn check_bounds(&self, pos: Vector3, bounds: &SpaceBounds) -> Result<(), MovementReject> {
        if position_within_bounds(pos, bounds) {
            Ok(())
        } else {
            Err(MovementReject::OutOfBounds)
        }
    }

    /// Advance an entity's processing clock to `now` and return the
    /// previous sample.
    ///
    /// Call this exactly once for **every** processed client position
    /// packet — *before* the bounds / navmesh layers can short-circuit.
    /// That is what closes the dt-inflation hole: if the clock only
    /// advanced on the packets that reach the kinematics layer, an
    /// attacker could spam cheaply-rejected (out-of-bounds / off-navmesh)
    /// packets to let `dt` grow, then send one large jump whose implied
    /// speed (`distance / dt`) looks slow enough to clear the teleport
    /// gate. Advancing up front keeps `dt` ~one tick regardless of which
    /// layer rejected the previous packet.
    pub fn touch_clock(&mut self, entity_id: u32, now: Instant) -> Option<Instant> {
        self.move_clock.insert(entity_id, now)
    }

    /// Speed + teleport layer (pure — no clock mutation; the caller
    /// advances the clock via [`touch_clock`] and passes the recovered
    /// `prev` sample in).
    ///
    /// `last_pos` is the entity's current authoritative position (the
    /// cell entity's `position` field — already advanced by any
    /// server-authoritative teleport), `proposed` is the client's new
    /// position, `now` is the server's monotonic processing time, and
    /// `prev` is the previous processing-clock sample (`None` = no time
    /// baseline yet — the first packet is gated on distance from
    /// `last_pos` alone, since no speed can be computed).
    ///
    /// Sourcing `last_pos` from the post-teleport cell entity is what
    /// makes this layer robust without an authorized-teleport allowlist:
    /// after a ring/respawn/gate/content teleport writes the new position
    /// through `update_entity_position`, the *next* client packet is
    /// measured against the destination, so a legitimate teleport never
    /// produces a self-inflicted false reject. [`note_authorized_teleport`]
    /// exists only to reseed the clock so the post-teleport packet's `dt`
    /// is measured from the teleport instant rather than a stale sample.
    ///
    /// [`touch_clock`]: Self::touch_clock
    /// [`note_authorized_teleport`]: Self::note_authorized_teleport
    pub fn check_kinematics(
        &self,
        now: Instant,
        prev: Option<Instant>,
        last_pos: Vector3,
        proposed: Vector3,
        top_speed: f32,
    ) -> KinematicsOutcome {
        let distance = proposed.distance_to(&last_pos);

        let Some(prev) = prev else {
            // No *time* baseline yet (first packet since spawn / forget),
            // so we can't compute a speed. But `last_pos` is the
            // authoritative spawn/teleport position, so gate on distance
            // alone: a first move farther than the teleport jump distance
            // is a teleport regardless of timing. This closes the "one
            // free large jump on the very first packet" bypass — the
            // dt-based speed gate below only kicks in once a baseline
            // exists.
            let reject = (distance > Self::TELEPORT_JUMP_UNITS).then_some(MovementReject::Teleport);
            return KinematicsOutcome {
                reject,
                speed_warn: None,
            };
        };

        let dt_secs = now.saturating_duration_since(prev).as_secs_f32();
        // dt can be ~0 when two packets share a clock instant; treat that
        // as infinite implied speed so the distance gate alone decides.
        let implied_speed = if dt_secs > 1e-4 {
            distance / dt_secs
        } else {
            f32::INFINITY
        };

        let is_teleport = distance > Self::TELEPORT_JUMP_UNITS
            && implied_speed > top_speed * Self::TELEPORT_SPEED_FACTOR;
        if is_teleport {
            return KinematicsOutcome {
                reject: Some(MovementReject::Teleport),
                speed_warn: None,
            };
        }

        let speed_warn =
            (implied_speed > top_speed * Self::SPEED_WARN_TOLERANCE).then_some(SpeedSample {
                distance,
                dt_secs,
                implied_speed,
                top_speed,
            });
        KinematicsOutcome {
            reject: None,
            speed_warn,
        }
    }

    /// Reseed an entity's speed/teleport clock after a server-
    /// authoritative position write (ring transport, respawn, gate
    /// travel arrival, content-engine teleport, GM travel). The next
    /// client packet's `dt` is then measured from the teleport instant,
    /// which suppresses a spurious speed warn when an authoritative move
    /// interrupts the client's update stream. It does **not** suppress
    /// hard rejects — a stale in-flight packet pointing at the old
    /// location *should* be snapped to the new one. See
    /// `.claude/agent-memory/movement-teleport-advisor/authorized-teleport-paths.md`.
    pub fn note_authorized_teleport(&mut self, entity_id: u32, now: Instant) {
        self.touch_clock(entity_id, now);
    }

    /// Release an entity's per-entity validator state. Call on entity
    /// destroy / disconnect so the clock map cannot leak or carry a
    /// stale sample across `entity_id` reuse.
    pub fn forget(&mut self, entity_id: u32) {
        self.move_clock.remove(&entity_id);
    }
}

/// Grace window kept for documentation parity with the design note. The
/// current design does not need it (see [`MovementValidator::check_kinematics`]),
/// but the constant pins the client's ~800 ms frame-history depth plus
/// margin should a future refactor move `last_pos` into validator state.
pub const TELEPORT_GRACE: Duration = Duration::from_millis(2000);

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_bounds() -> SpaceBounds {
        SpaceBounds::new([-100.0, -50.0, -100.0], [100.0, 200.0, 100.0])
    }

    #[test]
    fn validator_accepts_in_bounds_position() {
        let v = MovementValidator::new();
        assert_eq!(v.check_bounds(Vector3::zero(), &unit_bounds()), Ok(()));
    }

    #[test]
    fn validator_rejects_out_of_bounds_position_with_out_of_bounds_variant() {
        let v = MovementValidator::new();
        let pos = Vector3::new(1_000.0, 0.0, 0.0);
        assert_eq!(
            v.check_bounds(pos, &unit_bounds()),
            Err(MovementReject::OutOfBounds)
        );
    }

    // ── Speed / teleport layer (kinematics) ─────────────────────────────

    const TOP: f32 = MovementValidator::DEFAULT_TOP_SPEED;

    #[test]
    fn no_baseline_small_first_move_is_accepted() {
        let v = MovementValidator::new();
        let t0 = Instant::now();
        // `prev = None` (first packet since spawn). A small move from the
        // authoritative spawn is fine — no speed to measure, distance is
        // under the teleport gate.
        let out = v.check_kinematics(t0, None, Vector3::zero(), Vector3::new(2.0, 0.0, 0.0), TOP);
        assert_eq!(out, KinematicsOutcome::default());
    }

    /// First-packet bypass guard: with no time baseline we can't compute a
    /// speed, but a first move farther than the teleport jump distance
    /// from the authoritative spawn is still a teleport (distance gate).
    /// Without this gate, the very first client packet after spawn could
    /// jump anywhere in-bounds for free.
    #[test]
    fn no_baseline_large_first_move_is_teleport_rejected() {
        let v = MovementValidator::new();
        let t0 = Instant::now();
        let out = v.check_kinematics(
            t0,
            None,
            Vector3::zero(),
            Vector3::new(9_000.0, 0.0, 0.0),
            TOP,
        );
        assert_eq!(out.reject, Some(MovementReject::Teleport));
    }

    #[test]
    fn small_legitimate_move_neither_warns_nor_rejects() {
        let v = MovementValidator::new();
        let t0 = Instant::now();
        // ~0.8 u over 100 ms = top speed exactly — under the 1.5× warn.
        let out = v.check_kinematics(
            t0 + Duration::from_millis(100),
            Some(t0),
            Vector3::zero(),
            Vector3::new(0.8, 0.0, 0.0),
            TOP,
        );
        assert!(out.reject.is_none(), "legit move must not reject");
        assert!(out.speed_warn.is_none(), "legit move must not warn");
    }

    /// Canonical teleport shape: 100 units in 50 ms → 2000 u/s,
    /// ~246× top speed. Both gates trip → hard reject.
    #[test]
    fn hundred_units_in_fifty_ms_is_teleport_rejected() {
        let v = MovementValidator::new();
        let t0 = Instant::now();
        let out = v.check_kinematics(
            t0 + Duration::from_millis(50),
            Some(t0),
            Vector3::zero(),
            Vector3::new(100.0, 0.0, 0.0),
            TOP,
        );
        assert_eq!(out.reject, Some(MovementReject::Teleport));
    }

    /// A lagged client sending one large *but slow* catch-up packet
    /// (60 u over 10 s = 6 u/s) must NOT be teleport-rejected — distance
    /// gate trips but the speed gate does not. This is the false-positive
    /// the dual gate exists to avoid.
    #[test]
    fn far_but_slow_catchup_is_not_teleport() {
        let v = MovementValidator::new();
        let t0 = Instant::now();
        let out = v.check_kinematics(
            t0 + Duration::from_secs(10),
            Some(t0),
            Vector3::zero(),
            Vector3::new(60.0, 0.0, 0.0),
            TOP,
        );
        assert!(
            out.reject.is_none(),
            "far-but-slow catch-up must not be a teleport"
        );
    }

    /// Sub-teleport but over-tolerance speed (5 u in 50 ms = 100 u/s,
    /// distance under the 50 u gate) → warn, still accepted.
    #[test]
    fn over_tolerance_short_hop_warns_but_accepts() {
        let v = MovementValidator::new();
        let t0 = Instant::now();
        let out = v.check_kinematics(
            t0 + Duration::from_millis(50),
            Some(t0),
            Vector3::zero(),
            Vector3::new(5.0, 0.0, 0.0),
            TOP,
        );
        assert!(out.reject.is_none(), "short hop must not reject");
        let sample = out.speed_warn.expect("100 u/s must warn");
        assert!(sample.implied_speed > 90.0 && sample.implied_speed < 110.0);
    }

    /// `touch_clock` advances the per-entity clock and hands back the
    /// previous sample, which is what feeds `check_kinematics`'s `prev`.
    #[test]
    fn touch_clock_returns_prior_and_advances() {
        let mut v = MovementValidator::new();
        let t0 = Instant::now();
        assert_eq!(v.touch_clock(1, t0), None, "first touch has no prior");
        let t1 = t0 + Duration::from_millis(100);
        assert_eq!(v.touch_clock(1, t1), Some(t0), "second touch returns t0");
    }

    /// A spamming attacker who keeps the cell entity pinned (last_pos
    /// unchanged) must be rejected on *every* packet — touching the clock
    /// each packet keeps dt ~one tick so the implied speed never decays
    /// under the teleport gate.
    #[test]
    fn sustained_teleport_spam_keeps_rejecting() {
        let mut v = MovementValidator::new();
        let mut t = Instant::now();
        v.touch_clock(1, t); // seed
        for _ in 0..5 {
            t += Duration::from_millis(100);
            let prev = v.touch_clock(1, t);
            let out =
                v.check_kinematics(t, prev, Vector3::zero(), Vector3::new(100.0, 0.0, 0.0), TOP);
            assert_eq!(
                out.reject,
                Some(MovementReject::Teleport),
                "each spam packet must re-reject; dt must not decay the speed gate"
            );
        }
    }

    #[test]
    fn note_authorized_teleport_reseeds_clock_and_suppresses_followup_warn() {
        let mut v = MovementValidator::new();
        let t0 = Instant::now();
        // Player moving normally, last sample at t0.
        v.touch_clock(1, t0);
        // Authoritative teleport 5 s later reseeds the clock.
        let t_tp = t0 + Duration::from_secs(5);
        v.note_authorized_teleport(1, t_tp);
        // First post-teleport client packet, 100 ms after the teleport,
        // a small delta from the new (destination) last_pos: dt is
        // measured from t_tp, not t0, so the speed is sane.
        let t_pkt = t_tp + Duration::from_millis(100);
        let prev = v.touch_clock(1, t_pkt);
        assert_eq!(
            prev,
            Some(t_tp),
            "reseed must make the prior the teleport instant"
        );
        let dst = Vector3::new(500.0, 0.0, 500.0);
        let out = v.check_kinematics(t_pkt, prev, dst, Vector3::new(500.5, 0.0, 500.0), TOP);
        assert!(out.reject.is_none());
        assert!(
            out.speed_warn.is_none(),
            "reseeded clock must make the post-teleport packet look slow"
        );
    }

    #[test]
    fn forget_clears_clock_so_next_sample_has_no_baseline() {
        let mut v = MovementValidator::new();
        let t0 = Instant::now();
        v.touch_clock(1, t0); // seed
        v.forget(1);
        // After forget there is no time baseline, so the next touch
        // returns None and the move is gated on distance alone (a fresh
        // start) — a small move is accepted.
        let t1 = t0 + Duration::from_millis(50);
        let prev = v.touch_clock(1, t1);
        assert_eq!(prev, None, "forget must clear the prior sample");
        let out = v.check_kinematics(t1, prev, Vector3::zero(), Vector3::new(2.0, 0.0, 0.0), TOP);
        assert_eq!(out, KinematicsOutcome::default());
    }
}
