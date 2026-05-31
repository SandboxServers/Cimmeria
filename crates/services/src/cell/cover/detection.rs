//! Player cover-detection tick.
//!
//! Runs periodically against the live player list, asking "which cover
//! sets is each player currently inside?" and computing the
//! enter/leave/duration diff against the prior tick. Emits a
//! [`CoverDetectionTick`] event bundle that the caller (the cell
//! service tick) dispatches to:
//!
//! 1. The combatant cell-method handlers (`onEnterCoverSet` /
//!    `onLeaveCoverSet`) — which would mirror the BigWorld proximity
//!    callback semantics declared on `SGWCoverSet.def`.
//! 2. The content engine via the new `OnPlayerEnteredCover` /
//!    `OnPlayerLeftCover` / `OnPlayerInCoverDuration` triggers — so
//!    chain authors can gate quest steps on cover state (e.g. Castle
//!    Cellblock med-bay drone unlock).
//!
//! Per-player state lives in this module rather than on `CellEntity` so
//! the data shape stays close to its consumer. The state is small
//! (HashSet of cover_set_ids + per-set entry timestamps) and only
//! tracks players (NPC cover use goes through `reservation` directly).

use cimmeria_common::{EntityId, Vector3};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use super::types::{Cover, CoverHeight, CoverQuality};

/// How often the detection tick should run, in milliseconds. Chosen as a
/// conservative middle ground: fast enough that the HUD doesn't lag (the
/// client polls the server CoverSpace at ~250 ms in the original), slow
/// enough not to dominate cell-service tick time.
pub const COVER_DETECTION_TICK_MS: u64 = 250;

/// Proximity radius in BigWorld meters for the "player is in this cover
/// set" test. Mirrors the SGWCoverSet proximity controller's typical
/// radius. Tuned high enough that the player gets the HUD as they
/// approach cover, low enough that drifting past doesn't spam triggers.
pub const COVER_PROXIMITY_RADIUS: f32 = 5.0;

/// `OnPlayerInCoverDuration` debounced thresholds to test against. A
/// player who stays in the same cover set continuously will fire the
/// duration trigger once per crossed milestone. Keep this list short —
/// each milestone is checked per player per tick.
pub const COVER_DURATION_MILESTONES_SECS: &[u32] = &[3, 5, 10];

/// Per-player tracking state. One entry per player currently or recently
/// near cover.
#[derive(Debug, Default, Clone)]
struct PlayerCoverState {
    /// Cover-set IDs the player is currently within proximity of.
    current_sets: HashSet<i32>,
    /// Entry timestamp per cover_set_id — used to debounce duration triggers.
    entry_times: HashMap<i32, Instant>,
    /// Per-set, which duration milestones have already fired this stay.
    /// Resets on `OnPlayerLeftCover` for that set.
    fired_milestones: HashMap<i32, HashSet<u32>>,
}

/// Mutable detection-table state. Wrapped behind a Mutex on the `Cover`
/// service handle in real use; this struct itself is plain Rust state.
#[derive(Debug, Default)]
pub struct CoverDetectionTable {
    players: HashMap<EntityId, PlayerCoverState>,
}

impl CoverDetectionTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forget a player — call on disconnect / despawn. Without this the
    /// table leaks one `PlayerCoverState` per disconnected player.
    pub fn forget_player(&mut self, entity_id: EntityId) {
        self.players.remove(&entity_id);
    }

    /// Diagnostic: how many players are currently tracked.
    pub fn tracked_player_count(&self) -> usize {
        self.players.len()
    }
}

/// A single enter/leave event payload. Carries enough context that the
/// caller can wire both the cell-method dispatch and the content-trigger
/// emit without re-querying the cover index.
#[derive(Debug, Clone)]
pub struct EnteredCoverEvent {
    pub player_id: EntityId,
    pub cover_set_id: i32,
    /// Highest cover-height encountered in the set's nodes. Surfacing
    /// this in the trigger payload lets chain authors filter on cover
    /// quality (e.g. only fire the gate if the player is at a Mid+ cover).
    pub representative_height: CoverHeight,
    pub representative_quality: CoverQuality,
}

#[derive(Debug, Clone)]
pub struct LeftCoverEvent {
    pub player_id: EntityId,
    pub cover_set_id: i32,
}

#[derive(Debug, Clone)]
pub struct DurationCoverEvent {
    pub player_id: EntityId,
    pub cover_set_id: i32,
    /// Which milestone (in seconds) was crossed by this tick.
    pub seconds: u32,
}

/// Event bundle produced by one detection-tick run. The caller dispatches
/// each event to the cell-method handler + content engine.
#[derive(Debug, Default, Clone)]
pub struct CoverDetectionTick {
    pub entered: Vec<EnteredCoverEvent>,
    pub left: Vec<LeftCoverEvent>,
    pub duration_milestones: Vec<DurationCoverEvent>,
}

impl CoverDetectionTick {
    /// True if any kind of event fired this tick. Caller can skip the
    /// dispatch hot-loop entirely when this is false (saves a few
    /// allocations on quiet ticks).
    pub fn is_empty(&self) -> bool {
        self.entered.is_empty() && self.left.is_empty() && self.duration_milestones.is_empty()
    }
}

/// Run one detection tick. `players` is the list of (player_id, position)
/// pairs currently in the cell — caller computes from the entity table.
///
/// `now` is passed in for deterministic testing; production callers pass
/// `Instant::now()`.
pub fn run_detection_tick(
    cover: &Cover,
    players: &[(EntityId, Vector3)],
    table: &mut CoverDetectionTable,
    now: Instant,
    proximity_radius: f32,
    duration_milestones_secs: &[u32],
) -> CoverDetectionTick {
    let mut tick = CoverDetectionTick::default();

    // Forget players that aren't in the current player list (they
    // disconnected or got despawned without an explicit `forget_player`
    // call). Compute the active set up front.
    let active_players: HashSet<EntityId> = players.iter().map(|(eid, _)| *eid).collect();
    table.players.retain(|eid, _| active_players.contains(eid));

    for (player_id, player_pos) in players {
        // Query nearby cover nodes; fold to set of cover_set_ids (chunk_ids).
        let node_indices = cover.index.nearby(player_pos, proximity_radius, Some(5.0));

        // For each chunk represented in the hits, find the highest-quality
        // node to use as the "representative" for the trigger payload. A
        // chunk has one cover set; multiple nodes share the chunk_id.
        let mut chunk_reps: HashMap<i32, (CoverHeight, CoverQuality)> = HashMap::new();
        for idx in node_indices {
            let Some(n) = cover.index.node(idx) else {
                continue;
            };
            chunk_reps
                .entry(n.chunk_id)
                .and_modify(|rep| {
                    // Prefer higher height; on tie prefer higher quality.
                    if (n.height as u8) > (rep.0 as u8)
                        || ((n.height as u8) == (rep.0 as u8) && (n.quality as u8) > (rep.1 as u8))
                    {
                        *rep = (n.height, n.quality);
                    }
                })
                .or_insert((n.height, n.quality));
        }
        let current_sets: HashSet<i32> = chunk_reps.keys().copied().collect();

        // Diff against the player's prior state.
        let state = table.players.entry(*player_id).or_default();

        // Entered: sets that are current but weren't previously tracked.
        for &cover_set_id in current_sets.difference(&state.current_sets) {
            let (h, q) = chunk_reps
                .get(&cover_set_id)
                .copied()
                .unwrap_or((CoverHeight::Low, CoverQuality::None_));
            tick.entered.push(EnteredCoverEvent {
                player_id: *player_id,
                cover_set_id,
                representative_height: h,
                representative_quality: q,
            });
            state.entry_times.insert(cover_set_id, now);
            state.fired_milestones.insert(cover_set_id, HashSet::new());
        }

        // Left: sets that were tracked previously but aren't now.
        let left_now: Vec<i32> = state
            .current_sets
            .difference(&current_sets)
            .copied()
            .collect();
        for cover_set_id in left_now {
            tick.left.push(LeftCoverEvent {
                player_id: *player_id,
                cover_set_id,
            });
            state.entry_times.remove(&cover_set_id);
            state.fired_milestones.remove(&cover_set_id);
        }

        // Duration milestones: for every still-occupied set, check which
        // thresholds have been crossed since entry that haven't fired yet.
        for &cover_set_id in current_sets.intersection(&state.current_sets.clone()) {
            let Some(entry_time) = state.entry_times.get(&cover_set_id).copied() else {
                continue;
            };
            let secs_in = now.duration_since(entry_time).as_secs() as u32;
            let fired_set = state.fired_milestones.entry(cover_set_id).or_default();
            for &threshold in duration_milestones_secs {
                if secs_in >= threshold && !fired_set.contains(&threshold) {
                    tick.duration_milestones.push(DurationCoverEvent {
                        player_id: *player_id,
                        cover_set_id,
                        seconds: threshold,
                    });
                    fired_set.insert(threshold);
                }
            }
        }

        // Commit the current set as the new baseline for this player.
        state.current_sets = current_sets;
    }

    tick
}

#[cfg(test)]
mod detection_tests {
    use super::*;
    use crate::cell::cover::types::{CoverHeight, CoverNode, CoverQuality};
    use std::time::Duration;

    fn node(chunk_id: i32, node_id: i32, x: f32, z: f32) -> CoverNode {
        CoverNode {
            chunk_id,
            node_id,
            pos: Vector3::new(x, 0.0, z),
            orient: 0.0,
            height: CoverHeight::Mid,
            quality: CoverQuality::Best,
            tail: [0; 4],
        }
    }

    fn cover_with(nodes: Vec<CoverNode>) -> Cover {
        Cover::from_loaded(Vec::new(), nodes)
    }

    #[test]
    fn entering_proximity_fires_entered_event() {
        let cover = cover_with(vec![node(7, 0, 2.0, 0.0)]);
        let mut table = CoverDetectionTable::new();
        let now = Instant::now();
        let players = vec![(EntityId(10), Vector3::new(0.0, 0.0, 0.0))];

        let tick = run_detection_tick(&cover, &players, &mut table, now, 5.0, &[]);

        assert_eq!(tick.entered.len(), 1);
        assert_eq!(tick.entered[0].cover_set_id, 7);
        assert_eq!(tick.entered[0].player_id, EntityId(10));
        assert!(tick.left.is_empty());
    }

    #[test]
    fn re_entering_same_set_does_not_re_fire() {
        let cover = cover_with(vec![node(7, 0, 2.0, 0.0)]);
        let mut table = CoverDetectionTable::new();
        let now = Instant::now();
        let players = vec![(EntityId(10), Vector3::new(0.0, 0.0, 0.0))];

        let first = run_detection_tick(&cover, &players, &mut table, now, 5.0, &[]);
        assert_eq!(first.entered.len(), 1);

        // Second tick — same position, same state — must not re-fire.
        let second = run_detection_tick(&cover, &players, &mut table, now, 5.0, &[]);
        assert!(second.entered.is_empty(), "no re-fire while still in set");
        assert!(second.left.is_empty());
    }

    #[test]
    fn leaving_proximity_fires_left_event() {
        let cover = cover_with(vec![node(7, 0, 2.0, 0.0)]);
        let mut table = CoverDetectionTable::new();
        let now = Instant::now();

        // Enter.
        run_detection_tick(
            &cover,
            &[(EntityId(10), Vector3::new(0.0, 0.0, 0.0))],
            &mut table,
            now,
            5.0,
            &[],
        );

        // Leave (player teleports far away).
        let tick = run_detection_tick(
            &cover,
            &[(EntityId(10), Vector3::new(100.0, 0.0, 100.0))],
            &mut table,
            now,
            5.0,
            &[],
        );
        assert_eq!(tick.left.len(), 1);
        assert_eq!(tick.left[0].cover_set_id, 7);
        assert!(tick.entered.is_empty());
    }

    #[test]
    fn duration_milestones_fire_once_per_threshold() {
        let cover = cover_with(vec![node(7, 0, 2.0, 0.0)]);
        let mut table = CoverDetectionTable::new();
        let mut now = Instant::now();
        let players = vec![(EntityId(10), Vector3::new(0.0, 0.0, 0.0))];

        // Tick 1: enter; no duration yet.
        run_detection_tick(&cover, &players, &mut table, now, 5.0, &[3, 5]);

        // Tick 2 at +3 s: 3-second threshold crosses.
        now += Duration::from_secs(3);
        let t2 = run_detection_tick(&cover, &players, &mut table, now, 5.0, &[3, 5]);
        assert_eq!(t2.duration_milestones.len(), 1);
        assert_eq!(t2.duration_milestones[0].seconds, 3);

        // Tick 3 at +3 s again (still in cover, total 6 s): 5 crosses, 3 already fired.
        now += Duration::from_secs(3);
        let t3 = run_detection_tick(&cover, &players, &mut table, now, 5.0, &[3, 5]);
        assert_eq!(t3.duration_milestones.len(), 1, "only 5-s threshold fires");
        assert_eq!(t3.duration_milestones[0].seconds, 5);

        // Tick 4 same time: no more thresholds left.
        let t4 = run_detection_tick(&cover, &players, &mut table, now, 5.0, &[3, 5]);
        assert!(
            t4.duration_milestones.is_empty(),
            "milestones don't re-fire once crossed"
        );
    }

    #[test]
    fn leaving_and_re_entering_resets_duration_milestones() {
        let cover = cover_with(vec![node(7, 0, 2.0, 0.0)]);
        let mut table = CoverDetectionTable::new();
        let mut now = Instant::now();

        // Enter, wait 5 s, fire 3-s milestone, leave.
        run_detection_tick(
            &cover,
            &[(EntityId(10), Vector3::new(0.0, 0.0, 0.0))],
            &mut table,
            now,
            5.0,
            &[3],
        );
        now += Duration::from_secs(5);
        let _ = run_detection_tick(
            &cover,
            &[(EntityId(10), Vector3::new(0.0, 0.0, 0.0))],
            &mut table,
            now,
            5.0,
            &[3],
        );
        let _ = run_detection_tick(
            &cover,
            &[(EntityId(10), Vector3::new(100.0, 0.0, 100.0))],
            &mut table,
            now,
            5.0,
            &[3],
        );

        // Re-enter, wait 3 s — the 3-s milestone must fire again because
        // the leave reset the fired-milestones for this set.
        now += Duration::from_secs(1);
        run_detection_tick(
            &cover,
            &[(EntityId(10), Vector3::new(0.0, 0.0, 0.0))],
            &mut table,
            now,
            5.0,
            &[3],
        );
        now += Duration::from_secs(3);
        let tick = run_detection_tick(
            &cover,
            &[(EntityId(10), Vector3::new(0.0, 0.0, 0.0))],
            &mut table,
            now,
            5.0,
            &[3],
        );
        assert_eq!(tick.duration_milestones.len(), 1);
        assert_eq!(tick.duration_milestones[0].seconds, 3);
    }

    #[test]
    fn disconnected_players_get_pruned() {
        let cover = cover_with(vec![node(7, 0, 2.0, 0.0)]);
        let mut table = CoverDetectionTable::new();
        let now = Instant::now();

        // Player enters cover.
        run_detection_tick(
            &cover,
            &[(EntityId(10), Vector3::new(0.0, 0.0, 0.0))],
            &mut table,
            now,
            5.0,
            &[],
        );
        assert_eq!(table.tracked_player_count(), 1);

        // Next tick player no longer in the active list (disconnected).
        let _ = run_detection_tick(&cover, &[], &mut table, now, 5.0, &[]);
        assert_eq!(table.tracked_player_count(), 0);
    }

    #[test]
    fn highest_height_in_chunk_wins_representative() {
        let cover = cover_with(vec![
            CoverNode {
                chunk_id: 7,
                node_id: 0,
                pos: Vector3::new(2.0, 0.0, 0.0),
                orient: 0.0,
                height: CoverHeight::Low,
                quality: CoverQuality::Best,
                tail: [0; 4],
            },
            CoverNode {
                chunk_id: 7,
                node_id: 1,
                pos: Vector3::new(2.5, 0.0, 0.0),
                orient: 0.0,
                height: CoverHeight::High,
                quality: CoverQuality::Better,
                tail: [0; 4],
            },
        ]);
        let mut table = CoverDetectionTable::new();
        let now = Instant::now();
        let tick = run_detection_tick(
            &cover,
            &[(EntityId(10), Vector3::new(0.0, 0.0, 0.0))],
            &mut table,
            now,
            5.0,
            &[],
        );
        assert_eq!(tick.entered.len(), 1);
        assert_eq!(
            tick.entered[0].representative_height as u8,
            CoverHeight::High as u8,
            "the High-cover node must be picked as the chunk's representative"
        );
    }
}
