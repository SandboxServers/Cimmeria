//! The departed-entity ring: who held an entity ID slot, and when.
//!
//! Entity IDs are recycled runtime slots (instrumentation-discipline Rule 5),
//! so a log row that arrives late, such as a client telemetry row replayed by
//! the admin-api ingest (NT-40), can't be named from the live entity: the
//! slot may hold someone else by then. Each space keeps a short history of
//! the entities destroyed in it, with their lifetimes, and
//! [`SpaceManager::entity_label_at`](super::SpaceManager::entity_label_at)
//! answers from it.
//!
//! The rings live on the `SpaceManager`, not on the space, because an
//! instanced space is destroyed when its last player leaves, and that
//! player's late rows are exactly the ones that need naming.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, SystemTime};

/// How long a departed entity stays nameable.
pub const DEPARTED_RETENTION: Duration = Duration::from_secs(10 * 60);
/// Most departed entities kept per space; the oldest go first.
pub const DEPARTED_CAP: usize = 4_096;

/// One entity that left a space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DepartedEntity {
    pub entity_id: u32,
    /// Character name for a player, `name_id` text for an NPC; `None` when
    /// it had none.
    pub label: Option<&'static str>,
    /// NPC template, `None` for players.
    pub template_id: Option<i32>,
    pub created_at: SystemTime,
    pub destroyed_at: SystemTime,
}

impl DepartedEntity {
    /// True when `at` falls inside this entity's lifetime (both ends
    /// inclusive).
    fn was_alive_at(&self, at: SystemTime) -> bool {
        self.created_at <= at && at <= self.destroyed_at
    }
}

/// Per-space rings of [`DepartedEntity`], bounded at [`DEPARTED_RETENTION`]
/// or [`DEPARTED_CAP`] rows per space, whichever is smaller.
#[derive(Debug, Default)]
pub struct DepartedEntities {
    rings: HashMap<u32, VecDeque<DepartedEntity>>,
    /// A fixed clock for tests; `None` reads the wall clock.
    now_override: Option<SystemTime>,
}

impl DepartedEntities {
    /// The time a departure is stamped with: the wall clock, or the test
    /// clock when one is set.
    pub fn now(&self) -> SystemTime {
        self.now_override.unwrap_or_else(SystemTime::now)
    }

    /// Fix the clock departures are stamped with (tests only).
    #[cfg(any(test, feature = "test-support"))]
    pub fn set_now(&mut self, now: SystemTime) {
        self.now_override = Some(now);
    }

    /// Record a departure from `space_id`, then evict what has aged out.
    /// `departed.destroyed_at` is the "now" the eviction measures from.
    pub fn push(&mut self, space_id: u32, departed: DepartedEntity) {
        let now = departed.destroyed_at;
        let ring = self.rings.entry(space_id).or_default();
        ring.push_back(departed);
        while ring.len() > DEPARTED_CAP {
            ring.pop_front();
        }
        // Every ring is aged here, not only this one: a destroyed instance's
        // ring never receives another push, and would otherwise stay forever.
        self.rings.retain(|_, ring| {
            while ring.front().is_some_and(|d| aged_out(d.destroyed_at, now)) {
                ring.pop_front();
            }
            !ring.is_empty()
        });
    }

    /// The departed entity that held `entity_id` in `space_id` at `at`.
    /// Lifetimes of one slot never overlap, so at most one row matches;
    /// the newest is taken if a clock step made two.
    pub fn alive_at(
        &self,
        space_id: u32,
        entity_id: u32,
        at: SystemTime,
    ) -> Option<&DepartedEntity> {
        self.rings
            .get(&space_id)?
            .iter()
            .rev()
            .find(|d| d.entity_id == entity_id && d.was_alive_at(at))
    }

    /// Rows held for `space_id`.
    pub fn len(&self, space_id: u32) -> usize {
        self.rings.get(&space_id).map_or(0, VecDeque::len)
    }

    /// True when no space holds a departed row.
    pub fn is_empty(&self) -> bool {
        self.rings.is_empty()
    }
}

/// True when a row destroyed at `destroyed_at` is past retention at `now`.
/// A wall clock stepped backwards makes `now` earlier than the row, which
/// keeps it rather than evicting the newest history.
fn aged_out(destroyed_at: SystemTime, now: SystemTime) -> bool {
    now.duration_since(destroyed_at)
        .is_ok_and(|age| age > DEPARTED_RETENTION)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000 + secs)
    }

    fn row(entity_id: u32, label: &'static str, created: u64, destroyed: u64) -> DepartedEntity {
        DepartedEntity {
            entity_id,
            label: Some(label),
            template_id: None,
            created_at: t(created),
            destroyed_at: t(destroyed),
        }
    }

    #[test]
    fn a_slot_held_twice_names_each_occupant_in_its_own_lifetime() {
        let mut ring = DepartedEntities::default();
        ring.push(1, row(7, "Old", 0, 100));
        ring.push(1, row(7, "New", 100, 200));
        assert_eq!(
            ring.alive_at(1, 7, t(50)).and_then(|d| d.label),
            Some("Old")
        );
        assert_eq!(
            ring.alive_at(1, 7, t(150)).and_then(|d| d.label),
            Some("New")
        );
        assert!(
            ring.alive_at(1, 7, t(250)).is_none(),
            "after the last departure"
        );
        assert!(ring.alive_at(2, 7, t(50)).is_none(), "another space");
    }

    #[test]
    fn rows_older_than_retention_are_evicted() {
        let mut ring = DepartedEntities::default();
        ring.push(1, row(7, "Old", 0, 10));
        let later = 10 + DEPARTED_RETENTION.as_secs() + 1;
        ring.push(1, row(8, "Fresh", later - 5, later));
        assert!(
            ring.alive_at(1, 7, t(5)).is_none(),
            "a row past retention must be gone, not answered from"
        );
        assert_eq!(ring.len(1), 1);
    }

    #[test]
    fn a_destroyed_spaces_ring_ages_out_on_another_spaces_push() {
        let mut ring = DepartedEntities::default();
        ring.push(1, row(7, "Gone", 0, 10));
        let later = 10 + DEPARTED_RETENTION.as_secs() + 1;
        ring.push(2, row(8, "Fresh", later - 5, later));
        assert_eq!(ring.len(1), 0, "space 1's ring must not outlive retention");
    }

    #[test]
    fn the_ring_is_capped_and_drops_the_oldest() {
        let mut ring = DepartedEntities::default();
        for i in 0..=DEPARTED_CAP as u32 {
            ring.push(1, row(i, "x", 0, 1));
        }
        assert_eq!(ring.len(1), DEPARTED_CAP);
        assert!(
            ring.alive_at(1, 0, t(0)).is_none(),
            "the oldest row went first"
        );
        assert!(ring.alive_at(1, DEPARTED_CAP as u32, t(0)).is_some());
    }

    #[test]
    fn a_backwards_clock_step_keeps_history() {
        let mut ring = DepartedEntities::default();
        ring.push(1, row(7, "Old", 0, 1_000));
        ring.push(1, row(8, "Stepped", 0, 500));
        assert!(ring.alive_at(1, 7, t(10)).is_some());
    }
}
