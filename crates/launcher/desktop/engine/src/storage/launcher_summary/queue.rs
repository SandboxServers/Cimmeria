//! The on-disk queue: one small file, rewritten whole. It is disposable, so any
//! problem reading it means "empty" and never reaches the launcher's own state.
use super::schema::{
    DroppedCounts, Summary, SummaryErrorCode, SummaryOperation, SummaryPhase, MAX_BATCH,
};
use crate::storage::{atomic, read, StorageError};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

pub(in crate::storage) const FILE: &str = "launcher-summaries.json";
pub(super) const MAX_ENTRIES: usize = 64;
pub(super) const TTL_S: u64 = 24 * 60 * 60;
/// Upper bound for one upload body, below the server's 64 KiB limit.
pub(super) const MAX_BATCH_BYTES: usize = 48 * 1024;

/// The attempt admitted by this or a previous process and not yet reported.
/// The local operation id only ever matches the journal; it is never exported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Tracking {
    pub local_operation_id: Uuid,
    pub attempt_id: Uuid,
    pub kind: SummaryOperation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Entry {
    pub created_unix_s: u64,
    pub pre_admission: bool,
    pub summary: Summary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Queue {
    pub schema_version: u32,
    /// Bumped whenever consent empties the queue; an upload taken under an
    /// older generation must not be applied.
    pub generation: u64,
    pub dropped: DroppedCounts,
    pub tracking: Option<Tracking>,
    /// Oldest first.
    pub entries: Vec<Entry>,
}

impl Queue {
    pub fn empty(generation: u64) -> Self {
        Self {
            schema_version: 1,
            generation,
            dropped: DroppedCounts::default(),
            tracking: None,
            entries: Vec::new(),
        }
    }

    /// Corrupt, oversized, future-schema, unreadable or not a regular file: empty.
    pub fn load(root: &Path) -> Self {
        match read::<Self>(&root.join(FILE)) {
            Ok(Some(queue)) if queue.schema_version == 1 && queue.entries.len() <= MAX_ENTRIES => {
                queue
            }
            _ => Self::empty(0),
        }
    }

    pub fn store(&self, root: &Path) -> Result<(), StorageError> {
        atomic::write(root, FILE, self)
    }

    /// Best effort. Anything that is not a regular file is left alone.
    pub fn remove(root: &Path) {
        let path = root.join(FILE);
        if std::fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.is_file()) {
            let _ = std::fs::remove_file(path);
        }
    }

    /// An admitted attempt's row always gets in; the oldest entry makes room.
    pub fn push_admitted(&mut self, entry: Entry) {
        if self.entries.len() >= MAX_ENTRIES {
            self.entries.remove(0);
            self.dropped.overflow = self.dropped.overflow.saturating_add(1);
        }
        self.entries.push(entry);
    }

    /// A repeat of a queued pre-admission failure only raises its retry count.
    /// Returns false when nothing changed.
    pub fn repeat_pre_admission(
        &mut self,
        operation: SummaryOperation,
        phase: SummaryPhase,
        code: SummaryErrorCode,
    ) -> Option<bool> {
        let summary = self
            .entries
            .iter_mut()
            .filter(|entry| entry.pre_admission)
            .map(|entry| &mut entry.summary)
            .find(|summary| {
                summary.operation == operation
                    && summary.phase == phase
                    && summary.error_code == Some(code)
            })?;
        let next = summary.retry_count.next();
        let changed = next != summary.retry_count;
        summary.retry_count = next;
        Some(changed)
    }

    /// A pre-admission row never evicts an admitted attempt's row: it replaces
    /// the oldest pre-admission row, or is itself dropped.
    pub fn push_pre_admission(&mut self, entry: Entry) {
        if self.entries.len() >= MAX_ENTRIES {
            self.dropped.overflow = self.dropped.overflow.saturating_add(1);
            let Some(oldest) = self.entries.iter().position(|entry| entry.pre_admission) else {
                return;
            };
            self.entries.remove(oldest);
        }
        self.entries.push(entry);
    }

    /// Kept at exactly 24 hours, dropped one second later. A creation time ahead
    /// of the clock counts as age zero. Returns whether anything was dropped.
    pub fn expire(&mut self, now_unix_s: u64) -> bool {
        let before = self.entries.len();
        self.entries
            .retain(|entry| now_unix_s.saturating_sub(entry.created_unix_s) <= TTL_S);
        let expired = before - self.entries.len();
        self.dropped.expired = self
            .dropped
            .expired
            .saturating_add(u16::try_from(expired).unwrap_or(u16::MAX));
        expired != 0
    }

    /// The oldest summaries that fit one upload.
    pub fn oldest_batch(&self) -> Vec<Summary> {
        let mut bytes = ENVELOPE_BYTES;
        let mut batch = Vec::new();
        for entry in self.entries.iter().take(MAX_BATCH) {
            let Ok(size) = serde_json::to_vec(&entry.summary).map(|bytes| bytes.len() + 1) else {
                break;
            };
            if bytes + size > MAX_BATCH_BYTES {
                break;
            }
            bytes += size;
            batch.push(entry.summary.clone());
        }
        batch
    }

    pub fn remove_events(&mut self, event_ids: &[Uuid]) {
        self.entries
            .retain(|entry| !event_ids.contains(&entry.summary.event_id));
    }
}

// Generous allowance for the request envelope around the summaries array.
const ENVELOPE_BYTES: usize = 256;
