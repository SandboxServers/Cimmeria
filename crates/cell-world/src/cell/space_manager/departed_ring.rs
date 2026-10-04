//! The departed-entity rings: who held an entity ID slot, and when.
//!
//! Entity IDs are recycled runtime slots (instrumentation-discipline Rule 5),
//! so a log row that arrives late, such as a client telemetry row replayed by
//! the admin-api ingest (NT-40), can't be named from the live entity: the
//! slot may hold someone else by then. Each space keeps a short history of
//! the entities destroyed in it, with their lifetimes, and
//! [`SpaceManager::entity_label_at`](super::SpaceManager::entity_label_at)
//! answers from it.
//!
//! Players and NPCs are kept in separate rings. Player slots are the ones
//! that get recycled; NPC ids come from a monotonic counter. A busy space
//! despawns NPCs by the thousand, and in a shared ring those rows would push
//! a departed player out long before its 10 minutes were up.
//!
//! The rings live on the `SpaceManager`, not on the space, because an
//! instanced space is destroyed when its last player leaves, and that
//! player's late rows are exactly the ones that need naming.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, SystemTime};

/// How long a departed entity stays nameable.
pub const DEPARTED_RETENTION: Duration = Duration::from_secs(10 * 60);
/// Most departed entities of one kind (players, NPCs) kept per space; the
/// oldest go first.
pub const DEPARTED_CAP: usize = 4_096;
/// How often a push also ages every other space's rings, so a destroyed
/// instance's rings (which never see another push) are dropped.
const SWEEP_INTERVAL: Duration = Duration::from_secs(60);

/// One entity that left a space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DepartedEntity {
    pub entity_id: u32,
    /// True for a player entity; picks the ring.
    pub player: bool,
    /// Character name for a player, `name_id` text for an NPC; `None` when
    /// it had none.
    pub label: Option<&'static str>,
    /// NPC template, `None` for players.
    pub template_id: Option<i32>,
    pub created_at: SystemTime,
    pub destroyed_at: SystemTime,
}

impl DepartedEntity {
    /// True when `at` falls inside this entity's lifetime, half-open:
    /// `[created_at, destroyed_at)`. At the instant of a destroy the slot
    /// already belongs to whoever is created next.
    fn was_alive_at(&self, at: SystemTime) -> bool {
        self.created_at <= at && at < self.destroyed_at
    }
}

/// One space's departures of one kind: the rows in departure order (for
/// eviction), and the same rows grouped by entity ID, so a lookup costs one
/// hash probe plus the few lifetimes that slot had, never a scan of the
/// whole ring. The telemetry ingest asks about entity IDs a client chose
/// (NT-40), so a miss has to be cheap.
#[derive(Debug, Default)]
struct SpaceRing {
    rows: VecDeque<DepartedEntity>,
    by_entity: HashMap<u32, VecDeque<DepartedEntity>>,
}

impl SpaceRing {
    fn push(&mut self, row: DepartedEntity) {
        self.rows.push_back(row);
        self.by_entity
            .entry(row.entity_id)
            .or_default()
            .push_back(row);
    }

    /// Drop the oldest row from both views. Rows enter both in the same
    /// order, so it is also the front of its entity's list.
    fn pop_front(&mut self) {
        let Some(row) = self.rows.pop_front() else {
            return;
        };
        if let Some(list) = self.by_entity.get_mut(&row.entity_id) {
            list.pop_front();
            if list.is_empty() {
                self.by_entity.remove(&row.entity_id);
            }
        }
    }

    /// Drop the rows that are past retention at `now`.
    fn age(&mut self, now: SystemTime) {
        while self
            .rows
            .front()
            .is_some_and(|d| aged_out(d.destroyed_at, now))
        {
            self.pop_front();
        }
    }
}

/// One kind's rings, keyed by space.
#[derive(Debug, Default)]
struct Rings(HashMap<u32, SpaceRing>);

impl Rings {
    fn push(&mut self, space_id: u32, row: DepartedEntity) {
        let now = row.destroyed_at;
        let ring = self.0.entry(space_id).or_default();
        ring.push(row);
        while ring.rows.len() > DEPARTED_CAP {
            ring.pop_front();
        }
        ring.age(now);
    }

    fn sweep(&mut self, now: SystemTime) {
        self.0.retain(|_, ring| {
            ring.age(now);
            !ring.rows.is_empty()
        });
    }

    fn alive_at(&self, space_id: u32, entity_id: u32, at: SystemTime) -> Option<&DepartedEntity> {
        // Newest first, in case a clock step made two lifetimes overlap.
        self.0
            .get(&space_id)?
            .by_entity
            .get(&entity_id)?
            .iter()
            .rev()
            .find(|d| d.was_alive_at(at))
    }

    fn len(&self, space_id: u32) -> usize {
        self.0.get(&space_id).map_or(0, |r| r.rows.len())
    }
}

/// Per-space rings of [`DepartedEntity`], one for players and one for NPCs,
/// each bounded at [`DEPARTED_RETENTION`] or [`DEPARTED_CAP`] rows per space,
/// whichever is smaller.
#[derive(Debug, Default)]
pub struct DepartedEntities {
    players: Rings,
    npcs: Rings,
    /// When every ring was last aged.
    last_sweep: Option<SystemTime>,
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

    /// Record a departure from `space_id`. Ages the ring it lands in, and at
    /// most once per [`SWEEP_INTERVAL`] every other ring too.
    /// `departed.destroyed_at` is the "now" the eviction measures from.
    pub fn push(&mut self, space_id: u32, departed: DepartedEntity) {
        let now = departed.destroyed_at;
        if departed.player {
            self.players.push(space_id, departed);
        } else {
            self.npcs.push(space_id, departed);
        }
        let due = self
            .last_sweep
            .is_none_or(|last| now.duration_since(last).is_ok_and(|d| d >= SWEEP_INTERVAL));
        if due {
            self.players.sweep(now);
            self.npcs.sweep(now);
            self.last_sweep = Some(now);
        }
    }

    /// The departed entity that held `entity_id` in `space_id` at `at`.
    pub fn alive_at(
        &self,
        space_id: u32,
        entity_id: u32,
        at: SystemTime,
    ) -> Option<&DepartedEntity> {
        self.players
            .alive_at(space_id, entity_id, at)
            .or_else(|| self.npcs.alive_at(space_id, entity_id, at))
    }

    /// Rows held for `space_id`, players and NPCs together.
    pub fn len(&self, space_id: u32) -> usize {
        self.players.len(space_id) + self.npcs.len(space_id)
    }

    /// True when no space holds a departed row.
    pub fn is_empty(&self) -> bool {
        self.players.0.is_empty() && self.npcs.0.is_empty()
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
            player: true,
            label: Some(label),
            template_id: None,
            created_at: t(created),
            destroyed_at: t(destroyed),
        }
    }

    fn npc(entity_id: u32, created: u64, destroyed: u64) -> DepartedEntity {
        DepartedEntity {
            player: false,
            label: Some("Jaffa Guard"),
            ..row(entity_id, "", created, destroyed)
        }
    }

    fn label(ring: &DepartedEntities, space: u32, id: u32, at: SystemTime) -> Option<&'static str> {
        ring.alive_at(space, id, at).and_then(|d| d.label)
    }

    #[test]
    fn a_slot_held_twice_names_each_occupant_in_its_own_lifetime() {
        let mut ring = DepartedEntities::default();
        ring.push(1, row(7, "Old", 0, 100));
        ring.push(1, row(7, "New", 100, 200));
        assert_eq!(label(&ring, 1, 7, t(50)), Some("Old"));
        assert_eq!(
            label(&ring, 1, 7, t(100)),
            Some("New"),
            "lifetimes are half-open: at the hand-over instant the slot is the new occupant's"
        );
        assert_eq!(label(&ring, 1, 7, t(150)), Some("New"));
        assert_eq!(label(&ring, 1, 7, t(200)), None, "at the last departure");
        assert_eq!(label(&ring, 2, 7, t(50)), None, "another space");
    }

    #[test]
    fn rows_older_than_retention_are_evicted() {
        let mut ring = DepartedEntities::default();
        ring.push(1, row(7, "Old", 0, 10));
        let later = 10 + DEPARTED_RETENTION.as_secs() + 1;
        ring.push(1, row(8, "Fresh", later - 5, later));
        assert_eq!(
            label(&ring, 1, 7, t(5)),
            None,
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
    fn npc_departures_never_evict_a_player() {
        let mut ring = DepartedEntities::default();
        ring.push(1, row(7, "Daniel", 0, 10));
        for i in 0..(2 * DEPARTED_CAP as u32) {
            ring.push(1, npc(100_000 + i, 10, 20));
        }
        assert_eq!(label(&ring, 1, 7, t(5)), Some("Daniel"));
        assert_eq!(
            label(&ring, 1, 100_000 + 2 * DEPARTED_CAP as u32 - 1, t(15)),
            Some("Jaffa Guard")
        );
    }

    #[test]
    fn a_backwards_clock_step_keeps_history_and_names_each_row() {
        let mut ring = DepartedEntities::default();
        ring.push(1, row(7, "Old", 0, 1_000));
        ring.push(1, row(8, "Stepped", 0, 500));
        assert_eq!(label(&ring, 1, 7, t(10)), Some("Old"));
        assert_eq!(label(&ring, 1, 8, t(10)), Some("Stepped"));
    }

    /// The by-entity index follows the ring: an evicted row leaves it, so a
    /// lookup sees exactly the rows the ring holds, and an entity whose rows
    /// are all gone leaves no key behind.
    #[test]
    fn the_entity_index_follows_eviction() {
        let mut ring = DepartedEntities::default();
        ring.push(1, row(7, "Old", 0, 10));
        for i in 0..DEPARTED_CAP as u32 {
            ring.push(1, row(1_000 + i, "x", 10, 20));
        }
        let space = &ring.players.0[&1];
        assert_eq!(space.rows.len(), DEPARTED_CAP);
        assert!(
            !space.by_entity.contains_key(&7),
            "evicted row left its key"
        );
        assert_eq!(space.by_entity.len(), DEPARTED_CAP);
        assert_eq!(label(&ring, 1, 7, t(5)), None);
        assert_eq!(label(&ring, 1, 1_000, t(15)), Some("x"));
    }
}
