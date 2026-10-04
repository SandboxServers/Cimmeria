//! Consent transitions around the preferences write. Withdrawal closes the gate
//! before the write and keeps it closed if the write fails; granting consent
//! starts from a queue that is known to be empty.
use super::{
    queue::Queue,
    tracker::{lock, Tracker},
};
use crate::storage::{DesktopState, StorageError};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::storage) enum ConsentChange {
    Unchanged,
    OptOut,
    OptIn,
}

impl DesktopState {
    /// Call once the request is otherwise valid, before preferences are written.
    pub(in crate::storage) fn summary_consent_requested(
        &mut self,
        requested: bool,
    ) -> Result<ConsentChange, StorageError> {
        match (self.preferences.launcher_summary_consent, requested) {
            (true, false) => {
                let closed = self.summary_guarded(|state| lock(&state.summaries).begin_opt_out());
                if closed.is_none() {
                    lock(&self.summaries).export_blocked = true;
                }
                Ok(ConsentChange::OptOut)
            }
            (false, true) => {
                match self.summary_guarded(|state| lock(&state.summaries).prepare_opt_in()) {
                    Some(true) => Ok(ConsentChange::OptIn),
                    // Consent stays off when the old queue cannot be emptied first.
                    _ => Err(StorageError::Io),
                }
            }
            _ => Ok(ConsentChange::Unchanged),
        }
    }

    /// Call with the result of the preferences write, after `self.preferences`
    /// reflects it.
    pub(in crate::storage) fn summary_consent_written(
        &mut self,
        change: ConsentChange,
        written: Result<(), StorageError>,
    ) {
        self.summary_guarded(|state| {
            let consent = state.preferences.launcher_summary_consent;
            let reopen = state.requires_reopen();
            let mut tracker = lock(&state.summaries);
            tracker.consent = consent;
            tracker.reopen = reopen;
            match (change, written) {
                // The replacement may have reached the disk, so the queue goes too.
                (ConsentChange::OptOut, Ok(()) | Err(StorageError::PersistenceUncertain)) => {
                    tracker.purge();
                }
                (ConsentChange::OptIn, Ok(())) => tracker.export_blocked = false,
                // A failed opt-out leaves `export_blocked` set for this process run.
                _ => (),
            }
        });
    }
}

impl Tracker {
    fn begin_opt_out(&mut self) {
        self.export_blocked = true;
        self.test_panic();
        self.cancel.cancel();
        self.cancel = CancellationToken::new();
        self.forget_attempt();
        if self.queue.tracking.take().is_some() && self.active.is_some() {
            self.store();
        }
    }

    fn prepare_opt_in(&mut self) -> bool {
        self.test_panic();
        if self.active.is_none() {
            // Inert: there is no queue to empty, but a file an earlier run left
            // must not be picked up by a later one. Best effort, so a build
            // without an endpoint can never fail a preferences save.
            Queue::remove(&self.root);
            return true;
        }
        self.forget_attempt();
        self.queue = Queue::empty(self.queue.generation.saturating_add(1));
        self.store_checked()
    }

    fn purge(&mut self) {
        self.queue = Queue::empty(self.queue.generation.saturating_add(1));
        if self.active.is_some() {
            self.store();
        } else {
            // Rows from an earlier configured run must not outlive a withdrawal.
            Queue::remove(&self.root);
        }
    }
}
