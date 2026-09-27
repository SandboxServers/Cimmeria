//! Offered-dialog set — which dialog ids are currently live for a player.
//!
//! The server-authority precondition behind `dialogButtonChoice`
//! (CAT-J-01 / #479): a choice is only honoured for a dialog id this
//! player was actually shown. This module owns the bookkeeping; the
//! cell-side handlers own the wire I/O and the logging.
//!
//! `CellEntity::offered_dialog_ids` is pushed when `onDialogDisplay` is
//! sent (`cell::interactions::send_dialog_display`, the single choke
//! point all display paths route through) and removed on the matching
//! `DIALOG_BUTTON_CHOICE`. Without it, a forged choice packet for any
//! discovered `dialog_id` drives the bound chain's actions (GrantXP,
//! GrantItem, AcceptMission, Teleport, …) with no precondition at all.
//!
//! The field is private by deliberate exception to `CellEntity`'s
//! all-`pub` convention, because the four accessors below are what keep
//! the [`MAX_OFFERED_DIALOGS`] bound and the one-shot guarantee. A direct
//! `push_back` could not forge past the gate, but it could break the
//! bound or duplicate an id into a double-fire.
//!
//! **Why a set and not a single pin.** The client's `DialogController`
//! keeps two active slots (one non-tutorial, one tutorial) and evicts the
//! previous occupant through its discard path when a *different* id
//! arrives. Discard closes the old window and, when that dialog has zero
//! cooked buttons, sends `dialogButtonChoice(oldId, -1)` — which reaches
//! the server AFTER the server has already recorded the new dialog. A
//! single pin rejects that close, so the evicted dialog's `dialog_choice`
//! chain never fires and the player silently loses the progression step
//! that closing the window was supposed to grant. Legacy python kept a
//! dict for exactly this reason (`deprecated/python/cell/SGWPlayer.py`,
//! `displayedDialogs`). The same rejection hits a lure-queued dialog
//! opened minutes later, because opening a lure is purely local and tells
//! the server nothing.
//!
//! **What the set does NOT weaken.** An id that was never offered is
//! still rejected, and a valid choice still removes its id, so a replay
//! is rejected too. The window widens from "the one dialog open right
//! now" to "one of the last few dialogs this player was shown", each
//! still single-use. Every id in the set is one the player could have
//! acted on legitimately at the moment it was sent.

use super::CellEntity;

/// Hard cap on simultaneously-offered dialog ids per player.
///
/// The client can hold at most two active dialogs (non-tutorial +
/// tutorial) plus whatever sits in its lure queues, so eight leaves
/// headroom without letting a player who ignores every dialog accumulate
/// an unbounded list. Overflow evicts the oldest entry; the caller warns.
pub const MAX_OFFERED_DIALOGS: usize = 8;

impl CellEntity {
    /// Record `dialog_id` as offered to this player.
    ///
    /// Returns the id evicted to stay inside [`MAX_OFFERED_DIALOGS`], if
    /// any. Callers log that eviction (negative-logging convention): it
    /// means a dialog this player was shown can no longer be answered.
    pub fn offer_dialog(&mut self, dialog_id: i32) -> Option<i32> {
        // Re-displaying the SAME id is idempotent on the client — the
        // display core installs it into the slot it already occupies and
        // does not run discard — so it must not create a second entry.
        // Removing first and pushing to the back also makes the
        // just-displayed dialog the last one evicted, which is what a
        // recency-ordered bound should do.
        self.offered_dialog_ids.retain(|&id| id != dialog_id);
        let evicted = if self.offered_dialog_ids.len() >= MAX_OFFERED_DIALOGS {
            self.offered_dialog_ids.pop_front()
        } else {
            None
        };
        self.offered_dialog_ids.push_back(dialog_id);
        evicted
    }

    /// Is `dialog_id` currently offered to this player?
    pub fn dialog_is_offered(&self, dialog_id: i32) -> bool {
        self.offered_dialog_ids.contains(&dialog_id)
    }

    /// Consume `dialog_id`: remove it and report whether it was offered.
    ///
    /// `false` is the rejection signal for `dialogButtonChoice` — a
    /// forged, replayed or stale id. One-shot by construction: the second
    /// call for the same id returns `false`.
    pub fn take_offered_dialog(&mut self, dialog_id: i32) -> bool {
        match self
            .offered_dialog_ids
            .iter()
            .position(|&id| id == dialog_id)
        {
            Some(idx) => {
                self.offered_dialog_ids.remove(idx);
                true
            }
            None => false,
        }
    }

    /// Snapshot of the offered ids, oldest first. Diagnostics only.
    pub fn offered_dialogs(&self) -> Vec<i32> {
        self.offered_dialog_ids.iter().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player() -> CellEntity {
        CellEntity::new(
            cimmeria_common::EntityId(1),
            cimmeria_common::SpaceId(1),
            cimmeria_common::Vector3::zero(),
        )
    }

    #[test]
    fn a_fresh_entity_offers_nothing() {
        let e = player();
        assert!(e.offered_dialogs().is_empty());
        assert!(!e.dialog_is_offered(2576));
    }

    #[test]
    fn offer_then_take_is_one_shot() {
        let mut e = player();
        assert_eq!(e.offer_dialog(2576), None, "no eviction on the first offer");
        assert!(e.dialog_is_offered(2576));
        assert!(e.take_offered_dialog(2576), "the offered id is accepted");
        assert!(
            !e.take_offered_dialog(2576),
            "a replay of the same id must be rejected — take is one-shot"
        );
        assert!(e.offered_dialogs().is_empty());
    }

    #[test]
    fn an_unoffered_id_is_never_taken() {
        let mut e = player();
        e.offer_dialog(2576);
        assert!(
            !e.take_offered_dialog(9999),
            "a forged id that was never offered must be rejected"
        );
        assert!(
            e.dialog_is_offered(2576),
            "a rejected take must not disturb the real offers"
        );
    }

    /// F13: display A, display B, then answer A. Both stay live, and
    /// answering A does not consume B.
    #[test]
    fn two_offers_are_independently_answerable() {
        let mut e = player();
        e.offer_dialog(2574);
        e.offer_dialog(2576);
        assert!(
            e.take_offered_dialog(2574),
            "the evicted dialog is still answerable"
        );
        assert!(
            e.dialog_is_offered(2576),
            "answering the evicted dialog must leave the current one offered"
        );
        assert!(e.take_offered_dialog(2576));
    }

    /// A re-display of the same id is idempotent on the client, so it
    /// must not consume a second slot here either.
    #[test]
    fn re_offering_the_same_id_does_not_duplicate() {
        let mut e = player();
        e.offer_dialog(2576);
        assert_eq!(e.offer_dialog(2576), None);
        assert_eq!(e.offered_dialogs(), vec![2576]);
        assert!(e.take_offered_dialog(2576));
        assert!(
            !e.dialog_is_offered(2576),
            "one take must clear a re-displayed id — a duplicate entry would \
             let the same chain fire twice"
        );
    }

    /// Re-offering moves the id to the back, so it outlives ids that were
    /// displayed before it when the bound is reached.
    #[test]
    fn re_offering_refreshes_recency() {
        let mut e = player();
        e.offer_dialog(1);
        e.offer_dialog(2);
        e.offer_dialog(1);
        assert_eq!(e.offered_dialogs(), vec![2, 1]);
    }

    #[test]
    fn the_set_is_bounded_and_evicts_the_oldest() {
        let mut e = player();
        for id in 1..=MAX_OFFERED_DIALOGS as i32 {
            assert_eq!(e.offer_dialog(id), None, "no eviction below the bound");
        }
        assert_eq!(e.offered_dialogs().len(), MAX_OFFERED_DIALOGS);

        let evicted = e.offer_dialog(99);
        assert_eq!(evicted, Some(1), "overflow evicts the OLDEST offer");
        assert_eq!(
            e.offered_dialogs().len(),
            MAX_OFFERED_DIALOGS,
            "the set must stay bounded — an unbounded list is the memory \
             leak this cap exists to prevent"
        );
        assert!(
            !e.dialog_is_offered(1),
            "an evicted id must no longer be answerable"
        );
        assert!(e.dialog_is_offered(99));
    }
}
