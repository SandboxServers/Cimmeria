//! Per-entity tracing: the shared state and field builders behind the
//! `client.entity.*`, `client.mercury.entity_*` and entity-tagged
//! `client.cme.event` / `client.dispatch.method_dropped` events.
//!
//! The client learns about an entity in three steps, on the Mercury
//! network thread: `enterAoI` (the server says it is in range),
//! `createEntity` (the server sends its state), and `EntityManager::
//! enterWorld` (the client makes it a live entity and asks for its
//! appearance). An entity that is created but never entered stays in the
//! manager's cache map, and every method the server sends it is queued
//! instead of dispatched (issue #838). These hooks report each step with
//! the entity id, and the manager's own view of where the entity is before
//! and after ([`map::Snapshot`]), so a missing step is visible per entity.
//!
//! Everything here is portable and unit-tested off the DLL target. The
//! detours that drive it live in `hooks::inline_hooks::entity_lifecycle`,
//! `entity_messages` and `net_out`.

pub(crate) mod map;

use std::cell::Cell;
use std::sync::Mutex;

use serde_json::{json, Value};

use super::name_throttle::{Decision, NameThrottle};
use map::{Snapshot, Where};

/// Fields of one event, in emit order.
pub(crate) type Fields = Vec<(&'static str, Value)>;

/// Tracked (event, entity) pairs before the table restarts. One entry is
/// about 100 bytes, so this bounds the table near a megabyte.
pub(crate) const MAX_TRACKED: usize = 8192;

// ---------------------------------------------------------------------
// Per-thread context

/// Which inbound entity method the current thread is dispatching, so an
/// event created inside the dispatch can carry the entity it is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Ctx {
    /// Target entity id.
    pub entity_id: i32,
    /// Entity type id, when known.
    pub type_id: Option<u16>,
    /// Message id of the method (the wire byte, not the decoded index).
    pub msg_id: u32,
}

thread_local! {
    static CTX: Cell<Option<Ctx>> = const { Cell::new(None) };
    static PHASE: Cell<&'static str> = const { Cell::new("other") };
    static ENTERED_WORLD: Cell<bool> = const { Cell::new(false) };
    static DESTROYED: Cell<bool> = const { Cell::new(false) };
    static SCHEDULED: Cell<bool> = const { Cell::new(false) };
}

/// Sets the dispatch context for its lifetime and restores the previous one
/// when dropped, including on a C++ exception unwinding through it.
pub(crate) struct CtxScope {
    prev: Option<Ctx>,
}

impl CtxScope {
    /// Make `ctx` the current thread's dispatch context.
    pub(crate) fn enter(ctx: Ctx) -> Self {
        Self {
            prev: CTX.try_with(|c| c.replace(Some(ctx))).unwrap_or(None),
        }
    }
}

impl Drop for CtxScope {
    fn drop(&mut self) {
        let _ = CTX.try_with(|c| c.set(self.prev));
    }
}

/// The entity the current thread is dispatching a method for, if any.
pub(crate) fn current_ctx() -> Option<Ctx> {
    CTX.try_with(Cell::get).unwrap_or(None)
}

/// One lifecycle operation (`enter`, `create`, `leave`, `replay`): tracks
/// whether the entity entered the world or was destroyed while it ran, and
/// names the operation for events raised from inside it.
pub(crate) struct OpScope {
    prev_phase: &'static str,
    prev_entered: bool,
    prev_destroyed: bool,
}

impl OpScope {
    /// Start operation `phase` on this thread.
    pub(crate) fn begin(phase: &'static str) -> Self {
        let prev_phase = PHASE.try_with(|c| c.replace(phase)).unwrap_or("other");
        let prev_entered = ENTERED_WORLD
            .try_with(|c| c.replace(false))
            .unwrap_or(false);
        let prev_destroyed = DESTROYED.try_with(|c| c.replace(false)).unwrap_or(false);
        Self {
            prev_phase,
            prev_entered,
            prev_destroyed,
        }
    }

    /// Whether `enterWorld` ran since [`OpScope::begin`].
    pub(crate) fn entered_world(&self) -> bool {
        ENTERED_WORLD.try_with(Cell::get).unwrap_or(false)
    }

    /// Whether the entity was destroyed since [`OpScope::begin`].
    pub(crate) fn destroyed(&self) -> bool {
        DESTROYED.try_with(Cell::get).unwrap_or(false)
    }
}

impl Drop for OpScope {
    fn drop(&mut self) {
        let _ = PHASE.try_with(|c| c.set(self.prev_phase));
        let _ = ENTERED_WORLD.try_with(|c| c.set(self.prev_entered));
        let _ = DESTROYED.try_with(|c| c.set(self.prev_destroyed));
    }
}

/// `enterWorld` ran: returns the operation that caused it.
pub(crate) fn note_entered_world() -> &'static str {
    let _ = ENTERED_WORLD.try_with(|c| c.set(true));
    PHASE.try_with(Cell::get).unwrap_or("other")
}

/// The entity destroy ran.
pub(crate) fn note_destroyed() {
    let _ = DESTROYED.try_with(|c| c.set(true));
}

/// One `GameEntity` appearance request (`0x00e69150`): tracks whether it
/// reached the job scheduler (`0x00e998e0`).
pub(crate) struct AppearanceScope {
    prev: bool,
}

impl AppearanceScope {
    /// Start an appearance request on this thread.
    pub(crate) fn begin() -> Self {
        Self {
            prev: SCHEDULED.try_with(|c| c.replace(false)).unwrap_or(false),
        }
    }

    /// Whether the scheduler ran since [`AppearanceScope::begin`].
    pub(crate) fn scheduled(&self) -> bool {
        SCHEDULED.try_with(Cell::get).unwrap_or(false)
    }
}

impl Drop for AppearanceScope {
    fn drop(&mut self) {
        let _ = SCHEDULED.try_with(|c| c.set(self.prev));
    }
}

/// The appearance job scheduler ran.
pub(crate) fn note_scheduled() {
    let _ = SCHEDULED.try_with(|c| c.set(true));
}

// ---------------------------------------------------------------------
// Per-(event, entity) throttle

/// Token-bucket throttle keyed by event *and* entity, so one entity's flood
/// of a method cannot hide another entity's first one: an entity that never
/// spoke before always gets through.
pub(crate) struct EntityThrottle {
    table: NameThrottle,
    cap: usize,
}

impl EntityThrottle {
    /// A throttle that restarts its table after `cap` tracked pairs.
    pub(crate) fn new(cap: usize) -> Self {
        Self {
            table: NameThrottle::with_max_names(cap.saturating_add(1)),
            cap,
        }
    }

    /// Decide for one event `name` about `entity` at monotonic `now_ms`.
    pub(crate) fn check(&mut self, name: &str, entity: i32, now_ms: u64) -> Decision {
        if self.table.len() >= self.cap {
            // Restarting hands every pair a fresh burst: a few duplicate
            // events, never a lost first sighting.
            self.table.clear();
        }
        self.table.check(&format!("{name}#{entity}"), now_ms)
    }
}

static THROTTLE: Mutex<Option<EntityThrottle>> = Mutex::new(None);
static EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// Run the shared per-(event, entity) throttle. Poisoning is ignored.
pub(crate) fn throttle(name: &str, entity: i32) -> Decision {
    let now_ms = EPOCH
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_millis() as u64;
    let mut g = THROTTLE.lock().unwrap_or_else(|e| e.into_inner());
    g.get_or_insert_with(|| EntityThrottle::new(MAX_TRACKED))
        .check(name, entity, now_ms)
}

/// Add the suppressed count to `fields` when the throttle reports one.
pub(crate) fn with_suppressed(mut fields: Fields, decision: Decision) -> Option<Fields> {
    let Decision::Emit { suppressed } = decision else {
        return None;
    };
    if suppressed > 0 {
        fields.push(("suppressed", json!(suppressed)));
    }
    Some(fields)
}

// ---------------------------------------------------------------------
// Field builders

/// The map state common to every lifecycle event.
fn state_fields(f: &mut Fields, before: &Snapshot, after: &Snapshot) {
    f.push(("place_before", json!(before.place().as_str())));
    f.push(("place_after", json!(after.place().as_str())));
    if let Some(e) = after.entity() {
        f.push(("enter_count", json!(e.enter_count)));
        f.push(("entity_flags", json!(e.flags)));
    }
    if let Some(p) = after.pending_enter_count {
        f.push(("pending_enter_count", json!(p)));
    }
    if let Some(q) = after.queued_msgs {
        f.push(("queued_msgs", json!(q)));
    }
    if after.is_local_player {
        f.push(("is_local_player", json!(true)));
    }
    if !(before.complete && after.complete) {
        f.push(("state_incomplete", json!(true)));
    }
}

/// `client.entity.enter`: `enterAoI(id, space, vehicle)`.
pub(crate) fn enter_fields(
    id: i32,
    space_id: u32,
    vehicle_id: u32,
    before: &Snapshot,
    after: &Snapshot,
    entered_world: bool,
) -> Fields {
    let mut f: Fields = vec![
        ("entity_id", json!(id)),
        ("space_id", json!(space_id)),
        ("vehicle_id", json!(vehicle_id)),
        ("entered_world", json!(entered_world)),
    ];
    state_fields(&mut f, before, after);
    f
}

/// What a `createEntity` left the entity as.
pub(crate) fn create_outcome(
    before: &Snapshot,
    after: &Snapshot,
    entered_world: bool,
) -> &'static str {
    if entered_world {
        "entered_world"
    } else {
        match after.place() {
            Where::Cache => "parked",
            Where::World if before.place() == Where::World => "already_in_world",
            Where::World => "in_world",
            Where::Pending | Where::None => "no_entity",
        }
    }
}

/// `client.entity.create`: `createEntity(id, type, space, vehicle, stream)`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn create_fields(
    id: i32,
    type_id: u32,
    space_id: u32,
    vehicle_id: u32,
    payload_len: Option<u32>,
    before: &Snapshot,
    after: &Snapshot,
    entered_world: bool,
) -> Fields {
    let mut f: Fields = vec![
        ("entity_id", json!(id)),
        ("type_id", json!(type_id & 0xffff)),
        ("space_id", json!(space_id)),
        ("vehicle_id", json!(vehicle_id)),
        (
            "outcome",
            json!(create_outcome(before, after, entered_world)),
        ),
        ("entered_world", json!(entered_world)),
    ];
    if let Some(n) = payload_len {
        f.push(("payload_len", json!(n)));
    }
    state_fields(&mut f, before, after);
    f
}

/// `client.entity.leave`: `leaveAoI(id, cache_stamp)`.
pub(crate) fn leave_fields(
    id: i32,
    cache_stamp: u32,
    before: &Snapshot,
    after: &Snapshot,
    destroyed: bool,
) -> Fields {
    let mut f: Fields = vec![
        ("entity_id", json!(id)),
        ("cache_stamp", json!(cache_stamp)),
        ("destroyed", json!(destroyed)),
    ];
    state_fields(&mut f, before, after);
    f
}

/// `client.entity.appearance_request` outcome. The function has three
/// exits (`0x00e69150`): entity not ready (return), scheduled, or held for
/// a transaction (return). Scheduling is observed; "held" and "not ready"
/// are told apart only by the hold byte at `entity+0x32`, which is set for
/// the held exit and may also be set on a not-ready entity.
pub(crate) fn appearance_outcome(scheduled: bool, hold_byte: Option<u8>) -> &'static str {
    match (scheduled, hold_byte) {
        (true, _) => "scheduled",
        (false, Some(0)) => "not_ready",
        (false, Some(_)) => "held_or_not_ready",
        (false, None) => "not_scheduled",
    }
}

/// Where an inbound entity method or property went.
pub(crate) fn delivery_path(in_world: bool, is_local_player: bool) -> &'static str {
    if in_world {
        "delivered"
    } else if is_local_player {
        "local_player"
    } else {
        "queued"
    }
}

/// Where an inbound entity *property* message went. `onEntityProperty`
/// (`0x00dd29d0`) does nothing for an entity in the world: it asks the stream
/// for its remaining length and returns (the BigWorld property message is
/// unused by this client: SGW sends properties as ClientMethods). For any
/// other entity it queues the message, flagged `0x40`, and the replay
/// discards it the same way.
pub(crate) fn property_path(in_world: bool) -> &'static str {
    if in_world {
        "known_entity_ignored"
    } else {
        "queued"
    }
}

#[cfg(test)]
mod tests {
    use super::map::EntityInfo;
    use super::*;

    fn snap(place: Where, enter: i32) -> Snapshot {
        let info = EntityInfo {
            ptr: 0x9000,
            enter_count: enter,
            flags: 2,
            type_id: 9,
        };
        Snapshot {
            world: (place == Where::World).then_some(info),
            cache: (place == Where::Cache).then_some(info),
            pending_enter_count: (place == Where::Pending).then_some(1),
            queued_msgs: Some(3),
            is_local_player: false,
            complete: true,
        }
    }

    fn get<'a>(f: &'a Fields, k: &str) -> Option<&'a Value> {
        f.iter().find(|(n, _)| *n == k).map(|(_, v)| v)
    }

    /// The Frost shape: the create parks the entity in the cache map and
    /// `enterWorld` never runs. It must read as "parked", not "entered".
    #[test]
    fn a_create_that_parks_is_reported_as_parked() {
        let before = snap(Where::None, 0);
        let after = snap(Where::Cache, 0);
        assert_eq!(create_outcome(&before, &after, false), "parked");
        let f = create_fields(42, 0x1_0007, 1, 0, Some(120), &before, &after, false);
        assert_eq!(get(&f, "outcome"), Some(&json!("parked")));
        assert_eq!(get(&f, "type_id"), Some(&json!(7)), "type id is 16 bits");
        assert_eq!(get(&f, "place_before"), Some(&json!("none")));
        assert_eq!(get(&f, "place_after"), Some(&json!("cache")));
        assert_eq!(get(&f, "queued_msgs"), Some(&json!(3)));
        assert_eq!(get(&f, "entered_world"), Some(&json!(false)));
    }

    #[test]
    fn a_create_that_enters_the_world_says_so() {
        let before = snap(Where::None, 0);
        let after = snap(Where::World, 1);
        assert_eq!(create_outcome(&before, &after, true), "entered_world");
        assert_eq!(create_outcome(&after, &after, false), "already_in_world");
        assert_eq!(create_outcome(&before, &after, false), "in_world");
        assert_eq!(
            create_outcome(&before, &snap(Where::None, 0), false),
            "no_entity"
        );
    }

    #[test]
    fn an_incomplete_snapshot_is_flagged() {
        let mut after = snap(Where::World, 1);
        after.complete = false;
        let f = enter_fields(1, 2, 0, &snap(Where::None, 0), &after, true);
        assert_eq!(get(&f, "state_incomplete"), Some(&json!(true)));
        assert_eq!(get(&f, "entered_world"), Some(&json!(true)));
    }

    #[test]
    fn leave_reports_destruction() {
        let f = leave_fields(5, 77, &snap(Where::World, 1), &snap(Where::None, 0), true);
        assert_eq!(get(&f, "destroyed"), Some(&json!(true)));
        assert_eq!(get(&f, "cache_stamp"), Some(&json!(77)));
    }

    #[test]
    fn appearance_outcomes() {
        assert_eq!(appearance_outcome(true, Some(1)), "scheduled");
        assert_eq!(appearance_outcome(false, Some(0)), "not_ready");
        assert_eq!(appearance_outcome(false, Some(1)), "held_or_not_ready");
        assert_eq!(appearance_outcome(false, None), "not_scheduled");
    }

    #[test]
    fn property_paths() {
        assert_eq!(property_path(true), "known_entity_ignored");
        assert_eq!(property_path(false), "queued");
    }

    #[test]
    fn delivery_paths() {
        assert_eq!(delivery_path(true, false), "delivered");
        assert_eq!(delivery_path(false, true), "local_player");
        assert_eq!(delivery_path(false, false), "queued");
    }

    /// The scopes nest and restore, and are per thread.
    #[test]
    fn ctx_scope_restores_and_is_thread_local() {
        assert_eq!(current_ctx(), None);
        let outer = Ctx {
            entity_id: 1,
            type_id: None,
            msg_id: 9,
        };
        {
            let _a = CtxScope::enter(outer);
            assert_eq!(current_ctx(), Some(outer));
            {
                let inner = Ctx {
                    entity_id: 2,
                    ..outer
                };
                let _b = CtxScope::enter(inner);
                assert_eq!(current_ctx(), Some(inner));
            }
            assert_eq!(current_ctx(), Some(outer));
            std::thread::spawn(|| assert_eq!(current_ctx(), None))
                .join()
                .unwrap();
        }
        assert_eq!(current_ctx(), None);
    }

    #[test]
    fn op_scope_tracks_enter_world_and_nests() {
        let outer = OpScope::begin("create");
        assert!(!outer.entered_world());
        assert_eq!(note_entered_world(), "create");
        assert!(outer.entered_world());
        {
            let inner = OpScope::begin("replay");
            assert!(!inner.entered_world(), "an inner op starts clean");
            assert_eq!(note_entered_world(), "replay");
        }
        assert!(outer.entered_world(), "the outer op's flag is restored");
        drop(outer);
        assert_eq!(note_entered_world(), "other");
        // leave the thread clean for other tests on this thread
        let _ = OpScope::begin("x");
    }

    #[test]
    fn appearance_scope_sees_the_scheduler() {
        let s = AppearanceScope::begin();
        assert!(!s.scheduled());
        note_scheduled();
        assert!(s.scheduled());
        drop(s);
        assert!(!AppearanceScope::begin().scheduled());
    }

    /// A flood of one method for one entity does not hide the first one
    /// for another; and a full table restarts instead of merging pairs.
    #[test]
    fn the_throttle_is_per_event_and_per_entity() {
        let mut t = EntityThrottle::new(64);
        let mut hot = 0;
        for i in 0..1000 {
            if matches!(t.check("Event_NetIn_x", 1, i), Decision::Emit { .. }) {
                hot += 1;
            }
        }
        assert!(hot <= 12, "{hot}");
        assert_eq!(
            t.check("Event_NetIn_x", 2, 1000),
            Decision::Emit { suppressed: 0 },
            "another entity's first event always passes"
        );
        assert_eq!(
            t.check("Event_NetIn_y", 1, 1000),
            Decision::Emit { suppressed: 0 },
            "another event for the same entity passes too"
        );
    }

    #[test]
    fn a_full_table_restarts_and_never_merges_entities() {
        let mut t = EntityThrottle::new(4);
        for e in 0..4 {
            t.check("x", e, 0);
        }
        // The fifth pair restarts the table; it still gets its own burst.
        for _ in 0..8 {
            assert!(matches!(t.check("x", 99, 0), Decision::Emit { .. }));
        }
    }

    #[test]
    fn suppressed_is_attached_only_when_nonzero() {
        let base: Fields = vec![("entity_id", json!(1))];
        let f = with_suppressed(base.clone(), Decision::Emit { suppressed: 0 }).unwrap();
        assert_eq!(f.len(), 1);
        let f = with_suppressed(base.clone(), Decision::Emit { suppressed: 4 }).unwrap();
        assert_eq!(get(&f, "suppressed"), Some(&json!(4)));
        assert!(with_suppressed(base, Decision::Suppress).is_none());
    }
}
