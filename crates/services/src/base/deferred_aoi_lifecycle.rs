//! Entity-lifecycle ordering for a drained deferred-AoI buffer.
//!
//! The flush in `cell_dispatch::deferred_flush` bundles every `EnteredAoI`
//! into two leading packets and replays the rest afterwards. That is only
//! correct while no entity's lifecycle *crosses* that reordering, and a long
//! hold makes crossings routine: over the 16-second first-login cinematic
//! hold a patrolling NPC can enter, leave and re-enter a witness's AoI.
//!
//! Two shapes break the naive "introductions first" order:
//!
//! - `Entered(X) … Left(X)` — sending the create and then the leave is merely
//!   wasteful, but `Entered(X) … Left(X) … Entered(X)` sends both creates
//!   first and the leave last, so the client finishes **without** X.
//! - `Left(X) … Entered(X)` for an entity the client already had — same
//!   ending: create first, leave last.
//!
//! [`lifecycle_segments`] removes the first shape by cancelling an enter
//! against the leave that undoes it (the client never needs to hear about an
//! entity that came and went while it was not listening), and handles the
//! second by cutting the buffer into segments that are each safe to dispatch
//! introductions-first.

use std::collections::{HashMap, HashSet};

use super::deferred_aoi::DeferredAoiMsg;

/// The entity a message is *about*, for the entity-scoped variants. Player-self
/// `EntityMethodCall`s are not scoped to an AoI entity and return `None`.
fn scoped_entity(msg: &DeferredAoiMsg) -> Option<u32> {
    match msg {
        DeferredAoiMsg::EnteredAoI { entity_id, .. }
        | DeferredAoiMsg::LeftAoI { entity_id }
        | DeferredAoiMsg::WitnessEntityMethod { entity_id, .. }
        | DeferredAoiMsg::EntityInvisible { entity_id } => Some(*entity_id),
        DeferredAoiMsg::EntityMethodCall { .. } => None,
    }
}

/// Cancel each `EnteredAoI(X)` against a later `LeftAoI(X)`, together with
/// everything said about X in between. Order of the survivors is unchanged.
fn cancel_enter_leave_pairs(buffered: Vec<DeferredAoiMsg>) -> Vec<DeferredAoiMsg> {
    let mut out: Vec<Option<DeferredAoiMsg>> = Vec::with_capacity(buffered.len());
    // entity_id → index in `out` of its still-open EnteredAoI.
    let mut open_enter: HashMap<u32, usize> = HashMap::new();

    for msg in buffered {
        match &msg {
            DeferredAoiMsg::EnteredAoI { entity_id, .. } => {
                open_enter.insert(*entity_id, out.len());
                out.push(Some(msg));
            }
            DeferredAoiMsg::LeftAoI { entity_id } => match open_enter.remove(entity_id) {
                Some(start) => {
                    for slot in &mut out[start..] {
                        if slot.as_ref().and_then(scoped_entity) == Some(*entity_id) {
                            *slot = None;
                        }
                    }
                    // The leave is dropped too: the client never had X.
                }
                None => out.push(Some(msg)),
            },
            _ => out.push(Some(msg)),
        }
    }
    out.into_iter().flatten().collect()
}

/// Cut a drained buffer into segments, each safe to dispatch with its
/// introductions bundled ahead of everything else in the segment.
///
/// A new segment starts at an `EnteredAoI(X)` whenever the current segment
/// already carries a `LeftAoI(X)`, so that leave reaches the client before
/// the re-introduction instead of after it. The common case — no entity
/// both leaves and enters — is a single segment, which keeps the 28-NPC
/// world-entry burst at its two-bundle packet budget.
pub(crate) fn lifecycle_segments(buffered: Vec<DeferredAoiMsg>) -> Vec<Vec<DeferredAoiMsg>> {
    let mut segments: Vec<Vec<DeferredAoiMsg>> = Vec::new();
    let mut current: Vec<DeferredAoiMsg> = Vec::new();
    let mut left_in_current: HashSet<u32> = HashSet::new();

    for msg in cancel_enter_leave_pairs(buffered) {
        match &msg {
            DeferredAoiMsg::EnteredAoI { entity_id, .. } if left_in_current.contains(entity_id) => {
                segments.push(std::mem::take(&mut current));
                left_in_current.clear();
            }
            DeferredAoiMsg::LeftAoI { entity_id } => {
                left_in_current.insert(*entity_id);
            }
            _ => {}
        }
        current.push(msg);
    }
    if !current.is_empty() {
        segments.push(current);
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entered(entity_id: u32) -> DeferredAoiMsg {
        DeferredAoiMsg::EnteredAoI {
            entity_id,
            class_id: 1,
            position: [0.0; 3],
            direction: [0.0; 3],
            level: 1,
            npc_data: None,
            player_data: None,
        }
    }

    fn left(entity_id: u32) -> DeferredAoiMsg {
        DeferredAoiMsg::LeftAoI { entity_id }
    }

    fn witness(entity_id: u32) -> DeferredAoiMsg {
        DeferredAoiMsg::WitnessEntityMethod {
            entity_id,
            method_index: 3,
            args: Vec::new(),
            entity_is_player: false,
        }
    }

    fn self_call(method_index: u16) -> DeferredAoiMsg {
        DeferredAoiMsg::EntityMethodCall {
            entity_id: 2,
            method_index,
            args: Vec::new(),
        }
    }

    /// Compact form for assertions: `E7` / `L7` / `W7` / `I7` / `S`.
    fn shape(segments: &[Vec<DeferredAoiMsg>]) -> Vec<Vec<String>> {
        segments
            .iter()
            .map(|seg| {
                seg.iter()
                    .map(|m| match m {
                        DeferredAoiMsg::EnteredAoI { entity_id, .. } => format!("E{entity_id}"),
                        DeferredAoiMsg::LeftAoI { entity_id } => format!("L{entity_id}"),
                        DeferredAoiMsg::WitnessEntityMethod { entity_id, .. } => {
                            format!("W{entity_id}")
                        }
                        DeferredAoiMsg::EntityInvisible { entity_id } => format!("I{entity_id}"),
                        DeferredAoiMsg::EntityMethodCall { .. } => "S".to_string(),
                    })
                    .collect()
            })
            .collect()
    }

    /// The reviewer's case. An NPC walks out of and back into range during
    /// the hold. Dispatching introductions-first over the raw buffer sends
    /// `E7 E7 … L7` and the client ends without entity 7; the pair must
    /// cancel so only the final introduction survives.
    #[test]
    fn enter_leave_reenter_keeps_only_the_final_introduction() {
        let segments = lifecycle_segments(vec![entered(7), witness(7), left(7), entered(7)]);
        assert_eq!(shape(&segments), vec![vec!["E7"]]);
    }

    /// An entity that came and went while the client was not listening is
    /// never mentioned at all — including what was said about it in between.
    #[test]
    fn enter_then_leave_cancels_along_with_traffic_about_that_entity() {
        let segments = lifecycle_segments(vec![
            entered(7),
            entered(8),
            witness(7),
            witness(8),
            self_call(1),
            left(7),
        ]);
        assert_eq!(shape(&segments), vec![vec!["E8", "W8", "S"]]);
    }

    /// A leave for an entity the client already had, then its return: the
    /// leave must reach the client first, so the re-introduction starts a new
    /// segment instead of being bundled ahead of it.
    #[test]
    fn leave_then_reenter_of_a_known_entity_splits_into_two_segments() {
        let segments = lifecycle_segments(vec![entered(9), left(7), entered(7), witness(7)]);
        assert_eq!(shape(&segments), vec![vec!["E9", "L7"], vec!["E7", "W7"]]);
    }

    /// The world-entry burst — introductions only — must stay one segment, or
    /// the two-bundle packet budget the flush is built around is lost.
    #[test]
    fn introductions_only_stay_a_single_segment() {
        let segments = lifecycle_segments((100..128).map(entered).collect());
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].len(), 28);
    }

    #[test]
    fn empty_buffer_yields_no_segments() {
        assert!(lifecycle_segments(Vec::new()).is_empty());
    }
}
