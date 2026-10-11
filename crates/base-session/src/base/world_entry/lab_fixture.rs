//! The staged lab fixture store (lab-fixtures FX-02).
//!
//! The lab endpoint stages a fixture (missions, items, tutorials, world and
//! position) for an offline GM character; `handle_play_character` takes it
//! at that character's next Play and writes it before any world-entry read,
//! so the login and the cell's `player_loaded` chains see the fixture state
//! as if the player had reached it. A process-wide map rather than a session
//! field: the stage and the take run on different paths (the lab endpoint
//! holds no session; the character is offline when it is staged), the same
//! shape as the GM-only redirect lines in `gm_only_worlds`.
//!
//! A fixture is taken once. Gate travel and respawn rerun `onClientReady`,
//! never `playCharacter`, so they never see it. One left unclaimed expires
//! after [`STAGED_FIXTURE_TTL`], so a lab run that died before Play cannot
//! rewrite a character hours later.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// The level a session needs for its staged fixture to be applied; the
/// endpoint stages only for characters on accounts at this level.
pub use super::gm_only_worlds::GM_ACCESS_LEVEL;

/// A staged fixture expires this long after staging (D-FX2).
pub const STAGED_FIXTURE_TTL: Duration = Duration::from_secs(600);
/// The most of one design id a fixture may ask a character to carry (D-FX7).
pub const FIXTURE_MAX_ITEM_COUNT: u32 = 10;
/// Containers the fixture counts as carried: main through crafting.
/// 16 to 20 (buyback, bank, auction, team and command banks) are never
/// touched (`cimmeria_entity::inventory`'s container ids).
pub const FIXTURE_CARRIED_CONTAINERS: std::ops::RangeInclusive<i32> = 1..=15;

/// What a fixture sets one mission to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StagedMissionState {
    /// Active on `step_id`, every earlier step completed.
    Active { step_id: i32 },
    /// Never taken: the mission's row is removed.
    NotActive,
    /// Completed once.
    Completed,
}

/// One mission of a fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedMission {
    pub mission_id: i32,
    pub state: StagedMissionState,
}

/// One exact carried count of a fixture (FX-03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedItem {
    pub type_id: i32,
    pub count: u32,
    pub container_id: i32,
}

/// A fixture waiting for its character's next Play.
#[derive(Debug, Clone, PartialEq)]
pub struct StagedFixture {
    pub player_id: i32,
    /// For logs (Rule 6).
    pub player_name: String,
    /// A `sgw_player.world_location` value (a world name).
    pub world: Option<String>,
    pub position: Option<[f32; 3]>,
    pub missions: Vec<StagedMission>,
    pub items: Vec<StagedItem>,
    pub tutorials_unseen: Vec<i32>,
}

type Store = Mutex<HashMap<i32, (Instant, StagedFixture)>>;

fn store() -> &'static Store {
    static STORE: OnceLock<Store> = OnceLock::new();
    STORE.get_or_init(Default::default)
}

/// Lock the store. A panic while it was held leaves a map that is still
/// whole (every write is one insert or remove), so a poisoned lock is
/// recovered rather than turning every later Play into a panic.
fn lock() -> std::sync::MutexGuard<'static, HashMap<i32, (Instant, StagedFixture)>> {
    store()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Whether a fixture staged at `staged_at` has expired by `now`.
fn is_expired(staged_at: Instant, now: Instant) -> bool {
    now.saturating_duration_since(staged_at) > STAGED_FIXTURE_TTL
}

/// Stage `f` for its player, replacing any earlier one. Returns true when
/// one was replaced.
pub fn stage_fixture(f: StagedFixture) -> bool {
    stage_fixture_at(f, Instant::now())
}

fn stage_fixture_at(f: StagedFixture, now: Instant) -> bool {
    lock().insert(f.player_id, (now, f)).is_some()
}

/// Remove and return the player's fixture; None when none is staged or it
/// is older than [`STAGED_FIXTURE_TTL`] (an expired one is dropped with a
/// WARN).
pub fn take_fixture(player_id: i32) -> Option<StagedFixture> {
    take_fixture_at(player_id, Instant::now())
}

fn take_fixture_at(player_id: i32, now: Instant) -> Option<StagedFixture> {
    let (staged_at, f) = lock().remove(&player_id)?;
    if is_expired(staged_at, now) {
        tracing::warn!(
            player_id,
            player_name = %f.player_name,
            age_secs = now.saturating_duration_since(staged_at).as_secs(),
            ttl_secs = STAGED_FIXTURE_TTL.as_secs(),
            reason = "expired",
            "lab fixture dropped: staged too long before the character's Play"
        );
        return None;
    }
    Some(f)
}

/// True while one is staged and not expired (the tool's reply, tests).
pub fn fixture_pending(player_id: i32) -> bool {
    fixture_pending_at(player_id, Instant::now())
}

fn fixture_pending_at(player_id: i32, now: Instant) -> bool {
    lock()
        .get(&player_id)
        .is_some_and(|(staged_at, _)| !is_expired(*staged_at, now))
}

#[cfg(test)]
mod tests {
    use super::*;

    // In-memory ids only, but the sentinel lint covers those too.
    const TAKEN_ONCE: i32 = 0x7046_0201;
    const REPLACED: i32 = 0x7046_0202;
    const EXPIRED: i32 = 0x7046_0203;

    fn fixture(player_id: i32, world: &str) -> StagedFixture {
        StagedFixture {
            player_id,
            player_name: format!("fx02-store-{player_id}"),
            world: Some(world.to_string()),
            position: None,
            missions: vec![StagedMission {
                mission_id: 622,
                state: StagedMissionState::Active { step_id: 80623 },
            }],
            items: Vec::new(),
            tutorials_unseen: vec![5882],
        }
    }

    /// Take removes: a second Play (or a reconnect) must not apply the
    /// fixture again over what the player has since done.
    #[test]
    fn a_fixture_is_taken_once() {
        let f = fixture(TAKEN_ONCE, "Castle_CellBlock");
        assert!(!stage_fixture(f.clone()));
        assert!(fixture_pending(TAKEN_ONCE));
        assert_eq!(take_fixture(TAKEN_ONCE), Some(f));
        assert!(!fixture_pending(TAKEN_ONCE));
        assert_eq!(take_fixture(TAKEN_ONCE), None);
    }

    #[test]
    fn staging_again_replaces_and_reports_it() {
        assert!(!stage_fixture(fixture(REPLACED, "Castle_CellBlock")));
        assert!(stage_fixture(fixture(REPLACED, "Castle")));
        let taken = take_fixture(REPLACED).expect("the second stage is pending");
        assert_eq!(taken.world.as_deref(), Some("Castle"));
        assert_eq!(take_fixture(REPLACED), None, "only one fixture per player");
    }

    /// The TTL is checked against the take's clock, not the stage's: an
    /// abandoned lab run must not rewrite the character later.
    #[test]
    fn an_expired_fixture_is_not_returned() {
        let staged = Instant::now();
        stage_fixture_at(fixture(EXPIRED, "Castle"), staged);
        let just_inside = staged + STAGED_FIXTURE_TTL;
        let past = staged + STAGED_FIXTURE_TTL + Duration::from_secs(1);
        assert!(fixture_pending_at(EXPIRED, just_inside));
        assert!(!fixture_pending_at(EXPIRED, past));
        assert_eq!(take_fixture_at(EXPIRED, past), None);
        assert!(
            !fixture_pending_at(EXPIRED, staged),
            "an expired fixture is dropped by the take, not left behind"
        );
        assert!(!is_expired(staged, just_inside));
        assert!(is_expired(staged, past));
    }
}
