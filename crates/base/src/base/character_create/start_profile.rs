//! The start profile a new character is created from (Class Start v6, CS-02):
//! read fresh from `resources.char_creation` for each creation, checked, and
//! refused loudly when unusable (lock L3).

use cimmeria_resources::base::start_profiles::{self, DebugKit, StartProfile};
use sqlx::PgPool;

use super::refusal_log::Reason;

/// `ERROR_CharacterCreationInvalidCharacterType` (ErrorStrings 10001): the
/// code a client gets when its char_def's start profile cannot be used.
pub(super) const ERR_PROFILE_UNUSABLE: i32 = super::fail_code::INVALID_CHARACTER_TYPE;
/// `ERROR_CharacterCreationUnspecifiedError`: the DB-error code.
pub(super) const ERR_DB: i32 = super::fail_code::UNSPECIFIED;

/// A usable start: the profile, its world id, and the debug kit to add
/// (empty unless the profile or the caller asks for it).
#[derive(Debug)]
pub(super) struct ResolvedStart {
    pub(super) profile: StartProfile,
    pub(super) world_id: i32,
    pub(super) debug_kit: Option<DebugKit>,
}

/// Why a start could not be resolved. Not logged here: the caller writes
/// the one `character_create_failed` row from it (TG-DB-08), so a
/// start-profile refusal is one row, not two.
#[derive(Debug)]
pub(super) struct StartRefusal {
    pub(super) reason: Reason,
    /// The `onCharacterCreateFailed` code to send.
    pub(super) code: i32,
    /// The database or load error, when there is one.
    pub(super) error: Option<String>,
    /// The refused profile's key and world, once a profile was found.
    pub(super) profile_id: Option<String>,
    pub(super) world: Option<String>,
    /// Every problem of the profile, for the row's `detail`.
    pub(super) problems: Option<String>,
}

impl StartRefusal {
    fn new(reason: Reason, code: i32, profile: Option<&StartProfile>) -> Self {
        Self {
            reason,
            code,
            error: None,
            profile_id: profile.map(|p| p.profile_id.clone()),
            world: profile.map(|p| p.world.clone()),
            problems: None,
        }
    }
}

/// Load and check the start profile of `char_def_id`. `force_debug_kit` is
/// the test-only override the seed-drift test uses (never access level, L2).
///
/// Refused when: the profiles can't be read; no profile row; the profile has
/// a problem; its world is not in `resources.worlds` (or the lookup fails);
/// or the cell announced no space for it. Every refusal is ERROR-level
/// ([`Reason::level`]); the caller logs it.
pub(super) async fn resolve_start(
    pool: &PgPool,
    char_def_id: i32,
    force_debug_kit: bool,
) -> Result<ResolvedStart, Box<StartRefusal>> {
    let loaded = match pool.acquire().await {
        Ok(mut conn) => start_profiles::load_all(&mut conn).await,
        Err(e) => Err(start_profiles::LoadError::Db(e)),
    };
    let profiles = match loaded {
        Ok(p) => p,
        Err(e) => {
            return Err(Box::new(StartRefusal {
                error: Some(e.to_string()),
                ..StartRefusal::new(Reason::StartProfilesLoadFailed, ERR_DB, None)
            }));
        }
    };
    let unusable = |reason: Reason, profile: Option<&StartProfile>| {
        StartRefusal::new(reason, ERR_PROFILE_UNUSABLE, profile)
    };
    let Some(profile) = profiles.by_char_def(char_def_id) else {
        return Err(Box::new(unusable(Reason::NoStartProfile, None)));
    };
    let problems = profile.problems();
    if let Some(first) = problems.first() {
        return Err(Box::new(StartRefusal {
            problems: Some(format!("{problems:?}")),
            ..unusable(Reason::StartProfileProblem(first.reason()), Some(profile))
        }));
    }
    let world_id: Option<i32> =
        match sqlx::query_scalar("SELECT world_id FROM resources.worlds WHERE world = $1")
            .bind(&profile.world)
            .fetch_optional(pool)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                return Err(Box::new(StartRefusal {
                    error: Some(e.to_string()),
                    ..StartRefusal::new(Reason::DbError, ERR_DB, Some(profile))
                }));
            }
        };
    let Some(world_id) = world_id else {
        return Err(Box::new(unusable(Reason::StartWorldUnknown, Some(profile))));
    };
    if !cimmeria_base_session::base::world_entry::space_registry::is_world_enterable(&profile.world)
    {
        return Err(Box::new(unusable(
            Reason::StartWorldNotLoaded,
            Some(profile),
        )));
    }
    let debug_kit = (profile.debug_kit || force_debug_kit).then(|| profiles.debug_kit().clone());
    Ok(ResolvedStart {
        profile: profile.clone(),
        world_id,
        debug_kit,
    })
}

/// Training points and Applied Science Points a character created at
/// `level` holds: the level-1 grant plus one of each per level above it.
pub(super) fn starting_points(level: i32) -> (i32, i32) {
    let extra = (level - 1).max(0);
    (
        cimmeria_game::player::STARTING_TRAINING_POINTS as i32
            + extra * cimmeria_game::player::TRAINING_POINTS_PER_LEVEL as i32,
        cimmeria_game::player::STARTING_APPLIED_SCIENCE_POINTS
            + extra * cimmeria_game::player::APPLIED_SCIENCE_POINTS_PER_LEVEL,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_1_starts_with_one_point_of_each() {
        assert_eq!(starting_points(1), (1, 1));
        assert_eq!(starting_points(3), (3, 3));
    }

    /// A start level is the profile's column, never a mission's seeded
    /// level (preflight finding). Fails if creation starts reading the
    /// missions table for a level.
    #[test]
    fn no_start_level_is_derived_from_missions() {
        for src in [include_str!("mod.rs"), include_str!("start_profile.rs")] {
            assert!(!src.contains(concat!("resources", ".missions")));
        }
    }
}
