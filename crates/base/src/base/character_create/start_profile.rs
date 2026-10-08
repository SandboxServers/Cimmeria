//! The start profile a new character is created from (Class Start v6, CS-02):
//! read fresh from `resources.char_creation` for each creation, checked, and
//! refused loudly when unusable (lock L3).

use std::net::SocketAddr;

use cimmeria_resources::base::start_profiles::{self, DebugKit, StartProfile};
use sqlx::PgPool;

/// `ERROR_CharacterCreationInvalidCharacterType` (error_texts 10001): the
/// code a client gets when its char_def's start profile cannot be used.
pub(super) const ERR_PROFILE_UNUSABLE: i32 = 10001;
/// The DB-error code.
pub(super) const ERR_DB: i32 = 3;

/// A usable start: the profile, its world id, and the debug kit to add
/// (empty unless the profile or the caller asks for it).
#[derive(Debug)]
pub(super) struct ResolvedStart {
    pub(super) profile: StartProfile,
    pub(super) world_id: i32,
    pub(super) debug_kit: Option<DebugKit>,
}

/// Load and check the start profile of `char_def_id`. `force_debug_kit` is
/// the test-only override the seed-drift test uses (never access level, L2).
///
/// Refused, with an ERROR naming the reason, when: no profile row; the
/// profile has a problem; its world is not in `resources.worlds`; or the cell
/// announced no space for it. Returns the client error code on refusal.
pub(super) async fn resolve_start(
    pool: &PgPool,
    addr: SocketAddr,
    char_def_id: i32,
    force_debug_kit: bool,
) -> Result<ResolvedStart, i32> {
    let loaded = match pool.acquire().await {
        Ok(mut conn) => start_profiles::load_all(&mut conn).await,
        Err(e) => Err(start_profiles::LoadError::Db(e)),
    };
    let profiles = match loaded {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(
                event = "character_create_failed",
                reason = "start_profiles_load_failed",
                %addr,
                char_def_id, // nt:id-only char_def rows carry no name column
                error = %e,
                "character_create: could not read the start profiles"
            );
            return Err(ERR_DB);
        }
    };
    let refuse = |reason: &'static str, profile: Option<&StartProfile>, detail: String| {
        tracing::error!(
            event = "character_create_failed",
            reason,
            %addr,
            char_def_id, // nt:id-only char_def rows carry the profile id below
            profile_id = profile.map(|p| p.profile_id.as_str()), // nt:id-only profile key has no display name
            world = profile.map(|p| p.world.as_str()),
            detail = %detail,
            "character_create: start profile unusable; creation refused"
        );
        ERR_PROFILE_UNUSABLE
    };
    let Some(profile) = profiles.by_char_def(char_def_id) else {
        return Err(refuse("no_start_profile", None, String::new()));
    };
    let problems = profile.problems();
    if let Some(first) = problems.first() {
        return Err(refuse(
            first.reason(),
            Some(profile),
            format!("{problems:?}"),
        ));
    }
    let world_id: Option<i32> =
        match sqlx::query_scalar("SELECT world_id FROM resources.worlds WHERE world = $1")
            .bind(&profile.world)
            .fetch_optional(pool)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(
                    event = "character_create_failed",
                    reason = "db_error",
                    %addr,
                    world = %profile.world,
                    error = %e,
                    "character_create: world_id lookup failed"
                );
                return Err(ERR_DB);
            }
        };
    let Some(world_id) = world_id else {
        return Err(refuse("start_world_unknown", Some(profile), String::new()));
    };
    if !cimmeria_base_session::base::world_entry::space_registry::is_world_enterable(&profile.world)
    {
        return Err(refuse(
            "start_world_not_loaded",
            Some(profile),
            String::new(),
        ));
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
