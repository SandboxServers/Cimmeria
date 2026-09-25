//! Per-player stargate dial state — originally modelled on `SGWPlayer`'s
//! `dialedAddress` / `dialingStargate` / `gatePassable` / `gateDialTimer`
//! quartet (`deprecated/python/cell/SGWPlayer.py:52-55, 2078-2129`).
//!
//! **The deprecated Python is not a behavioural reference.** The owner
//! confirmed (2026-09-25, NA35) that the legacy server never had working
//! gate travel end to end, so "the 2009 *server* did X" carries no
//! authority — it only ever proved what one unfinished reference
//! implementation happened to do, not what the 2009 *client* expects.
//! Ground truth for this file's timing is the client binary and its
//! Kismet rigs; see
//! `docs/reverse-engineering/findings/stargate-dial-and-travel-sequences.md`
//! for the evidence and `docs/analysis/castle-rebuild/README.md`'s D-CA20
//! for the decision record. D-CA10 predates that correction and is left
//! unedited; D-CA20 supersedes its framing.
//!
//! The dial deadline is drained by the existing 100 ms cell tick rather
//! than a spawned `tokio::time::sleep` task — same reasoning as C08a's
//! `deferred_content_actions`: a spawned task has no route to
//! `&mut SpaceManager` (owned exclusively by the single-threaded cell
//! message loop), so it would need a new channel and message variant just
//! to re-enter. A map keyed by entity id, drained on a tick that is
//! already running, needs none of that — and it inherits the
//! `destroy_entity` / `disconnect_entity` cleanup choke points for free,
//! which is exactly the "cancel the pending open if the dialer leaves the
//! space" requirement.
//!
//! The sibling post-crossing hold (`CROSSING_CINEMATIC_HOLD` /
//! `PendingCrossing`) lives in [`super::crossing_hold_state`] — a distinct
//! state machine with no calls into this one, split out once both existed
//! (NA35) to keep each file under the project's line-count soft cap.

use std::time::{Duration, Instant};

use super::SpaceManager;

/// How long the gate takes to open after a successful dial.
///
/// **Corrected 2026-09-25 (NA35).** Previously `Duration::from_secs(4)`,
/// sourced only from the disavowed `deprecated/python` reference
/// (`SGWPlayer.beginDialing`: `Atrea.addTimer(now + 4.0, ...)`) — evidence
/// for what one never-finished server did, not for what the client
/// expects. The client binary has no matching constant. Its DHD dialling
/// UI collects every glyph and calls `Event_World_DialStargateAddress`
/// exactly once, only after the full address is entered
/// (`FUN_005682d0` case `'d'` "dialStargateAddress",
/// `ghidra://SGW.exe@0x005682d0`), and the DHD window closes on the
/// client's own timeline at that point, independent of any server round
/// trip. By the time a real `onDialGate` even reaches the server, the
/// player has already left the DHD screen — matching tester Lomiada's
/// report that "the dialing is quite fast/done when I leave the DHD."
/// Standing them in front of a visibly inert gate for four more seconds
/// was the bug, not a deliberate pace.
///
/// There is no confirmed client-side duration to replace it with either —
/// extracting the `Stargate_MakeGate` Kismet rig's own vortex-formation
/// Matinee length would need a live client capture or a cooked-package
/// Kismet/Matinee parse; `crates/upk-objects` has no Matinee/`SeqAct_Interp`
/// reader today (only model/terrain/texture2d), so this pass did not
/// extract one. This constant is therefore the minimum the tick-drain
/// architecture can express: the gate opens on the next 100 ms cell tick
/// after a successful dial, not after an invented multi-second wait.
/// Retime it once a real duration is measured.
pub(crate) const GATE_DIAL_DURATION: Duration = Duration::from_millis(100);

/// One armed dial. Exists only between `onDialGate` and either the
/// crossing, a re-dial, a cancel, or the entity leaving its space.
#[derive(Debug, Clone)]
pub(crate) struct PendingGateDial {
    /// Deadline at which `Stargate_MakeGate` fires and the gate becomes
    /// passable. Mirrors the Python `gateDialTimer`.
    pub(crate) open_at: Instant,
    /// `SGWPlayer.gatePassable` — false until the dial timer expires.
    /// `stargatePassed()` is a no-op while this is false.
    pub(crate) passable: bool,
    /// `SGWPlayer.dialedAddress` — the destination gate's `stargate_id`.
    pub(crate) target_address_id: i32,
    /// Destination world name, snapshotted at dial time. Carried so the
    /// content-engine `stargate_crossed` trigger can key on it without a
    /// second cache lookup at crossing time.
    pub(crate) target_world_name: String,
    /// `SGWPlayer.dialingStargate.eventSet` — the event set of the gate
    /// the player is standing at (the ORIGIN world's gate), NOT the
    /// destination's. `SGWPlayer.py:2073-2074` sets `dialingStargate =
    /// world.stargates[0]` from the player's current space.
    pub(crate) origin_event_set_id: Option<i32>,
}

impl SpaceManager {
    /// Arm a dial for `entity_id`, replacing any dial already in flight.
    ///
    /// Replacement is the `beginDialing` behaviour: it opens with
    /// `if self.dialedAddress is not None: self.cancelDialing()`, so a
    /// second dial silently drops the first gate's pending open.
    pub(crate) fn begin_gate_dial(
        &mut self,
        entity_id: u32,
        target_address_id: i32,
        target_world_name: String,
        origin_event_set_id: Option<i32>,
    ) {
        let replaced = self.pending_gate_dials.insert(
            entity_id,
            PendingGateDial {
                open_at: Instant::now() + GATE_DIAL_DURATION,
                passable: false,
                target_address_id,
                target_world_name,
                origin_event_set_id,
            },
        );
        if let Some(prev) = replaced {
            tracing::debug!(
                entity_id,
                previous_target = prev.target_address_id,
                new_target = target_address_id,
                "gate dial: re-dial cancelled the in-flight dial"
            );
        }
    }

    /// Drop any armed dial for `entity_id`. `SGWPlayer.cancelDialing`.
    ///
    /// Returns the dropped dial so callers can log what was cancelled.
    pub(crate) fn cancel_gate_dial(&mut self, entity_id: u32) -> Option<PendingGateDial> {
        self.pending_gate_dials.remove(&entity_id)
    }

    /// Read-only peek at the armed dial, if any.
    pub(crate) fn gate_dial(&self, entity_id: u32) -> Option<&PendingGateDial> {
        self.pending_gate_dials.get(&entity_id)
    }

    /// Mark every dial whose timer has elapsed as passable and return
    /// them, paired with the dialing entity. Dials already marked
    /// passable are NOT returned again — `Stargate_MakeGate` fires once
    /// per dial, as in `gateDialTimerExpired` (which clears
    /// `gateDialTimer` before sending).
    pub(crate) fn take_opened_gate_dials(&mut self, now: Instant) -> Vec<(u32, PendingGateDial)> {
        let mut opened = Vec::new();
        for (&entity_id, dial) in self.pending_gate_dials.iter_mut() {
            if !dial.passable && dial.open_at <= now {
                dial.passable = true;
                opened.push((entity_id, dial.clone()));
            }
        }
        opened
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mgr() -> SpaceManager {
        SpaceManager::new(1)
    }

    #[test]
    fn a_fresh_dial_is_not_passable_and_does_not_open_early() {
        let mut m = mgr();
        m.begin_gate_dial(1, 2, "Castle".to_string(), Some(10011));

        let dial = m.gate_dial(1).expect("dial must be armed");
        assert!(!dial.passable, "the gate is shut until the timer expires");
        assert_eq!(dial.target_address_id, 2);
        assert_eq!(dial.origin_event_set_id, Some(10011));

        assert!(
            m.take_opened_gate_dials(Instant::now()).is_empty(),
            "a dial must not open before its own deadline, however short"
        );
        assert!(!m.gate_dial(1).unwrap().passable);
    }

    /// Regression guard for the 2026-09-25 (NA35) correction: the previous
    /// `GATE_DIAL_DURATION` of 4 seconds was sourced only from the
    /// disavowed `deprecated/python` reference and left players staring at
    /// an inert gate long after their own client-side DHD UI had already
    /// closed (tester Lomiada, "the dialing is quite fast/done when I
    /// leave the DHD"). This must open well under a second — fails if the
    /// constant regresses back toward multi-second territory.
    #[test]
    fn dial_opens_almost_immediately_not_after_a_multi_second_hold() {
        let mut m = mgr();
        m.begin_gate_dial(1, 2, "Castle".to_string(), Some(10011));

        assert!(
            GATE_DIAL_DURATION < Duration::from_millis(500),
            "GATE_DIAL_DURATION has no client-binary support for a \
             multi-second hold — see the doc comment on the constant"
        );

        let soon = Instant::now() + Duration::from_millis(500);
        let opened = m.take_opened_gate_dials(soon);
        assert_eq!(
            opened.len(),
            1,
            "the gate must open within half a second of a successful dial"
        );
    }

    #[test]
    fn dial_opens_once_the_deadline_passes_and_only_once() {
        let mut m = mgr();
        m.begin_gate_dial(1, 2, "Castle".to_string(), Some(10011));

        let after = Instant::now() + GATE_DIAL_DURATION + Duration::from_millis(1);
        let opened = m.take_opened_gate_dials(after);
        assert_eq!(opened.len(), 1, "the elapsed dial must open");
        assert_eq!(opened[0].0, 1);
        assert!(opened[0].1.passable);
        assert!(
            m.gate_dial(1).unwrap().passable,
            "the stored dial stays passable so the crossing can travel"
        );

        assert!(
            m.take_opened_gate_dials(after).is_empty(),
            "Stargate_MakeGate fires exactly once per dial — a second \
             drain must not re-open the same dial"
        );
    }

    /// `beginDialing` cancels an in-flight dial before arming the new
    /// one, so a re-dial must leave exactly one pending entry and it
    /// must be the new destination with a fresh (not-yet-open) timer.
    #[test]
    fn redial_replaces_the_pending_dial_and_resets_the_timer() {
        let mut m = mgr();
        m.begin_gate_dial(1, 2, "Castle".to_string(), Some(10011));
        // Force the first dial to be "already open" so a failure to
        // replace would be visible as a still-passable entry.
        let after = Instant::now() + GATE_DIAL_DURATION + Duration::from_millis(1);
        m.take_opened_gate_dials(after);
        assert!(m.gate_dial(1).unwrap().passable);

        m.begin_gate_dial(1, 3, "Harset".to_string(), Some(25));

        assert_eq!(m.pending_gate_dials.len(), 1, "one dial per player");
        let dial = m.gate_dial(1).expect("re-dial must be armed");
        assert_eq!(dial.target_address_id, 3, "the new destination wins");
        assert_eq!(dial.target_world_name, "Harset");
        assert!(
            !dial.passable,
            "a re-dial restarts the dial timer — the previously-open gate \
             must not stay crossable"
        );
    }

    #[test]
    fn cancel_drops_the_pending_dial() {
        let mut m = mgr();
        m.begin_gate_dial(1, 2, "Castle".to_string(), Some(10011));

        let cancelled = m.cancel_gate_dial(1).expect("cancel returns the dial");
        assert_eq!(cancelled.target_address_id, 2);
        assert!(m.gate_dial(1).is_none());
        assert!(m
            .take_opened_gate_dials(Instant::now() + GATE_DIAL_DURATION + Duration::from_millis(1))
            .is_empty());
    }

    #[test]
    fn destroy_entity_drops_the_pending_dial() {
        let mut m = mgr();
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        m.parse_spaces_xml(xml).unwrap();
        m.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        m.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();

        m.begin_gate_dial(1, 2, "Harset".to_string(), Some(10011));
        m.destroy_entity(1);

        assert!(
            m.gate_dial(1).is_none(),
            "leaving the space must cancel the pending Stargate_MakeGate — \
             otherwise the tick would emit a gate-open for an entity that \
             is no longer there (and whose id may be reused)"
        );
    }

    #[tokio::test]
    async fn disconnect_entity_drops_the_pending_dial() {
        let mut m = mgr();
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        m.parse_spaces_xml(xml).unwrap();
        m.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        m.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        m.connect_entity(1);

        m.begin_gate_dial(1, 2, "Harset".to_string(), Some(10011));
        let (tx, _rx) = tokio::sync::mpsc::channel(8);
        m.disconnect_entity(1, &tx).await;

        assert!(m.gate_dial(1).is_none());
    }
}
