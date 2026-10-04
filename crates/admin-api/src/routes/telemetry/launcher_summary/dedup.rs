//! Duplicate detection for launcher summaries.
//!
//! The launcher keeps a summary until the server answers for it, so a lost
//! response means the same `event_id` arrives again. This remembers the
//! last [`CAPACITY`] accepted ids, in memory, for the life of the process:
//! a server restart forgets them, and a resend after one is accepted again.
//!
//! The set cannot grow past its capacity, so a caller minting ids in a loop
//! costs a fixed amount of memory. When it is full the oldest id is
//! forgotten; a summary is never refused for lack of room.

use std::collections::{HashSet, VecDeque};
use std::sync::Mutex;

use super::dto::Verdict;

/// Ids remembered: 256 launchers' full queues (a launcher holds at most
/// 64 unsent summaries), for under 1 MiB.
pub(super) const CAPACITY: usize = 16_384;

struct Seen {
    /// Arrival order, oldest first.
    order: VecDeque<u128>,
    /// The same ids, for the membership test. `HashSet`'s default hasher is
    /// seeded per process, so a caller cannot aim ids at one bucket.
    ids: HashSet<u128>,
}

pub(super) struct Dedup {
    capacity: usize,
    seen: Mutex<Seen>,
}

impl Dedup {
    pub(super) fn new() -> Self {
        Self::with_capacity(CAPACITY)
    }

    /// A set that remembers `capacity` ids (at least one).
    pub(super) fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            capacity,
            seen: Mutex::new(Seen {
                order: VecDeque::with_capacity(capacity),
                ids: HashSet::with_capacity(capacity),
            }),
        }
    }

    /// The verdict for each element of one request, in order. `None` is an
    /// element that failed validation: it is `Rejected` and its id is never
    /// remembered. The whole request is judged under one lock acquisition,
    /// so two requests carrying the same id cannot both see it as new, and
    /// a repeat inside one request is a duplicate of its first occurrence.
    pub(super) fn judge(&self, ids: &[Option<u128>]) -> Vec<Verdict> {
        // A poisoned lock still guards a consistent set: every mutation
        // below leaves `order` and `ids` in step.
        let mut seen = self.seen.lock().unwrap_or_else(|p| p.into_inner());
        ids.iter()
            .map(|id| {
                let Some(id) = *id else {
                    return Verdict::Rejected;
                };
                if seen.ids.contains(&id) {
                    return Verdict::Duplicate;
                }
                if seen.order.len() >= self.capacity {
                    if let Some(oldest) = seen.order.pop_front() {
                        seen.ids.remove(&oldest);
                    }
                }
                seen.order.push_back(id);
                seen.ids.insert(id);
                Verdict::Accepted
            })
            .collect()
    }
}
