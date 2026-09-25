//! Per-player post-crossing cinematic hold (NA35, 2026-09-25) — the state
//! that defers a stargate crossing's world transition
//! (`CellToBaseMsg::GateTravel` / `RESET_ENTITIES`) until
//! `CROSSING_CINEMATIC_HOLD` has elapsed.
//!
//! Split out from the sibling [`super::gate_dial_state`] (dial timer)
//! once both existed: the two are independent state machines with no
//! calls into each other — a dial is consumed before a hold is ever
//! armed — kept in separate files per the project's line-count soft cap
//! rather than one growing file mixing two concerns.
//!
//! Ground truth for the hold's existence and duration is the client
//! binary, not `deprecated/python` — see
//! `docs/reverse-engineering/findings/stargate-dial-and-travel-sequences.md`
//! and D-CA20 in `docs/analysis/castle-rebuild/README.md`. The deadline is
//! drained by the existing 100 ms cell tick for the same reason the dial
//! timer is: a spawned `tokio` task has no route to `&mut SpaceManager`,
//! owned exclusively by the single-threaded cell message loop. Draining
//! on a tick that already runs inherits `destroy_entity` /
//! `disconnect_entity` cleanup for free.

use std::time::{Duration, Instant};

use super::SpaceManager;

/// How long a crossing waits, after `Stargate_CrossGate` / `onStargatePassage`
/// are sent, before the deferred world transition (`RESET_ENTITIES`) runs.
///
/// **Provisional, not client-verified (NA35, 2026-09-25).** Before this
/// existed, the server sent the crossing notifications and issued
/// `CellToBaseMsg::GateTravel` back to back in the same async call, with no
/// scheduled gap at all — the client had no guaranteed opportunity to
/// render even one frame of whatever Kismet sequence
/// `Stargate_CrossGate` (6113) resolves to before its view was torn down.
/// `FUN_00e2c810` / `FUN_00d2de90` (`ghidra://SGW.exe@0x00e2c810`) confirm
/// CrossGate is matched per-instance against a specific `USeqEvent_Stargate`
/// node (by `SourceAddressId`/`TargetAddressId`), so it is a real
/// world-space Kismet trigger, not UI chrome — but this pass did not
/// extract that node's Matinee track length from the cooked map packages
/// (same `crates/upk-objects` gap as `GATE_DIAL_DURATION` in the sibling
/// dial-state file). This value closes the confirmed race with a
/// conservative placeholder; retime it once a live capture or a
/// Kismet/Matinee read gives a real number. The traveller's movement is
/// locked (`BSF_MovementLock`, see
/// `cell::gate_travel::sequences::set_crossing_movement_lock`) for the
/// duration so they cannot wander off mid-cinematic while still resident
/// in the old world; the lock is released if the deferred travel fails
/// (see `cell::gate_travel::tick::crossing_tick`).
pub(crate) const CROSSING_CINEMATIC_HOLD: Duration = Duration::from_millis(1_500);

/// A crossing whose `Stargate_CrossGate` / `onStargatePassage` notifications
/// have been sent, waiting out `CROSSING_CINEMATIC_HOLD` before the deferred
/// world transition runs.
#[derive(Debug, Clone)]
pub(crate) struct PendingCrossing {
    /// Deadline at which `cell::gate_travel::tick::crossing_tick` runs the
    /// deferred `perform_gate_travel`. Mirrors `PendingGateDial::open_at`.
    pub(crate) travel_at: Instant,
    /// The destination stargate id, snapshotted at crossing time. The dial
    /// itself is cancelled (`cancel_gate_dial`) before the hold begins, so
    /// this is the only place the destination survives the wait.
    pub(crate) target_address_id: i32,
}

impl SpaceManager {
    /// Arm the post-crossing hold for `entity_id`. Called once, right
    /// after `Stargate_CrossGate` / `onStargatePassage` are sent and the
    /// dial is cancelled — see `cell::gate_travel::on_stargate_passage`.
    ///
    /// Replaces any hold already in flight rather than panicking. There
    /// should never be two: a second crossing needs a second dial, and the
    /// entity is torn down (or the hold explicitly cancelled) before a new
    /// one could be armed. Kept consistent with `begin_gate_dial`'s
    /// re-dial handling rather than asserting an invariant a future bug
    /// could silently violate.
    pub(crate) fn begin_crossing_hold(&mut self, entity_id: u32, target_address_id: i32) {
        let replaced = self.pending_crossings.insert(
            entity_id,
            PendingCrossing {
                travel_at: Instant::now() + CROSSING_CINEMATIC_HOLD,
                target_address_id,
            },
        );
        if replaced.is_some() {
            tracing::warn!(
                entity_id,
                target_address_id,
                reason = "crossing_hold_replaced",
                "gate crossing: a second hold was armed before the first \
                 one ran — this should not happen; the earlier hold's \
                 travel is now dropped"
            );
        }
    }

    /// Drop any armed crossing hold for `entity_id` without running the
    /// deferred travel. Used when the entity leaves its space (disconnect,
    /// destroy, or any other teardown) during the hold, mirroring
    /// `cancel_gate_dial`.
    pub(crate) fn cancel_crossing_hold(&mut self, entity_id: u32) -> Option<PendingCrossing> {
        self.pending_crossings.remove(&entity_id)
    }

    /// Read-only peek at the armed crossing hold, if any. Test-only:
    /// production code drains via `take_ready_crossings` and never needs to
    /// peek first (unlike `gate_dial`, which `handle_stargate_region_entered`
    /// reads directly).
    #[cfg(test)]
    pub(crate) fn crossing_hold(&self, entity_id: u32) -> Option<&PendingCrossing> {
        self.pending_crossings.get(&entity_id)
    }

    /// Take every crossing hold whose `CROSSING_CINEMATIC_HOLD` deadline
    /// has passed, removing them so the deferred travel runs exactly once
    /// per crossing.
    pub(crate) fn take_ready_crossings(&mut self, now: Instant) -> Vec<(u32, PendingCrossing)> {
        let ready_ids: Vec<u32> = self
            .pending_crossings
            .iter()
            .filter(|(_, crossing)| crossing.travel_at <= now)
            .map(|(&entity_id, _)| entity_id)
            .collect();
        ready_ids
            .into_iter()
            .filter_map(|entity_id| {
                self.pending_crossings
                    .remove(&entity_id)
                    .map(|crossing| (entity_id, crossing))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mgr() -> SpaceManager {
        SpaceManager::new(1)
    }

    #[test]
    fn a_fresh_crossing_hold_does_not_travel_immediately() {
        let mut m = mgr();
        m.begin_crossing_hold(1, 3);

        let hold = m.crossing_hold(1).expect("hold must be armed");
        assert_eq!(hold.target_address_id, 3);

        assert!(
            m.take_ready_crossings(Instant::now()).is_empty(),
            "the hold must not release before its own deadline"
        );
        assert!(m.crossing_hold(1).is_some(), "the hold must still be armed");
    }

    #[test]
    fn crossing_hold_releases_once_the_deadline_passes_and_only_once() {
        let mut m = mgr();
        m.begin_crossing_hold(1, 3);

        let after = Instant::now() + CROSSING_CINEMATIC_HOLD + Duration::from_millis(1);
        let ready = m.take_ready_crossings(after);
        assert_eq!(ready.len(), 1, "the elapsed hold must release");
        assert_eq!(ready[0].0, 1);
        assert_eq!(ready[0].1.target_address_id, 3);

        assert!(
            m.crossing_hold(1).is_none(),
            "a released hold is removed, not left passable — the deferred \
             travel runs exactly once"
        );
        assert!(
            m.take_ready_crossings(after).is_empty(),
            "a second drain must not re-release the same crossing"
        );
    }

    #[test]
    fn cancel_crossing_hold_drops_it_without_travelling() {
        let mut m = mgr();
        m.begin_crossing_hold(1, 3);

        let cancelled = m.cancel_crossing_hold(1).expect("cancel returns the hold");
        assert_eq!(cancelled.target_address_id, 3);
        assert!(m.crossing_hold(1).is_none());
        assert!(m
            .take_ready_crossings(
                Instant::now() + CROSSING_CINEMATIC_HOLD + Duration::from_millis(1)
            )
            .is_empty());
    }

    #[test]
    fn destroy_entity_drops_the_pending_crossing_hold() {
        let mut m = mgr();
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        m.parse_spaces_xml(xml).unwrap();
        m.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        m.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();

        m.begin_crossing_hold(1, 3);
        m.destroy_entity(1);

        assert!(
            m.crossing_hold(1).is_none(),
            "leaving the space must cancel a pending crossing hold — \
             otherwise the tick would run a deferred travel for an entity \
             that is no longer there (and whose id may be reused)"
        );
    }

    #[tokio::test]
    async fn disconnect_entity_drops_the_pending_crossing_hold() {
        let mut m = mgr();
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        m.parse_spaces_xml(xml).unwrap();
        m.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        m.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        m.connect_entity(1);

        m.begin_crossing_hold(1, 3);
        let (tx, _rx) = tokio::sync::mpsc::channel(8);
        m.disconnect_entity(1, &tx).await;

        assert!(m.crossing_hold(1).is_none());
    }
}
