//! Reading the start profiles from the `resources` schema.

use sqlx::PgConnection;

use super::{DebugKit, KitAbility, KitItem, KitSource, StartProfile, StartProfiles, StartState};

/// Why the profiles could not be read.
#[derive(Debug)]
pub enum LoadError {
    Db(sqlx::Error),
    /// A row the `CHECK` constraints should have refused (an unknown
    /// `start_state` or `source_kind`), or a child row for a char_def with
    /// no profile. Never silently skipped.
    BadRow(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Db(e) => write!(f, "database error: {e}"),
            Self::BadRow(s) => write!(f, "bad start-profile row: {s}"),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<sqlx::Error> for LoadError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(e)
    }
}

#[derive(sqlx::FromRow)]
struct ProfileRow {
    char_def_id: i32,
    profile_id: String,
    alignment: i32,
    archetype: i32,
    starting_world: String,
    starting_x: f32,
    starting_y: f32,
    starting_z: f32,
    start_level: i32,
    debug_kit: bool,
    start_state: String,
}

/// Every start profile and the debug kit, read on one connection (a pool
/// connection, or a transaction's so a caller holding a row lock does not
/// take a second one). A concrete `&mut PgConnection` rather than a generic
/// `Acquire`: the generic future is not `Send` for every lifetime, which
/// breaks the base's spawned cell-message task.
///
/// Alignment and archetype are the enum ordinals `sgw_player` stores
/// (`array_position - 1`), the reading `gm_ability_bulk` and `player_load`
/// already use for `EArchetype`.
pub async fn load_all(conn: &mut PgConnection) -> Result<StartProfiles, LoadError> {
    let rows = sqlx::query_as::<_, ProfileRow>(
        "SELECT cc.char_def_id, cc.profile_id::text AS profile_id, \
                (array_position(enum_range(NULL::resources.\"EAlignment\"), cc.alignment) - 1)::int \
                    AS alignment, \
                (array_position(enum_range(NULL::resources.\"EArchetype\"), cc.archetype) - 1)::int \
                    AS archetype, \
                cc.starting_world::text AS starting_world, \
                cc.starting_x, cc.starting_y, cc.starting_z, \
                cc.start_level, cc.debug_kit, cc.start_state::text AS start_state \
           FROM resources.char_creation cc \
          ORDER BY cc.char_def_id",
    )
    .fetch_all(&mut *conn)
    .await?;
    let abilities = sqlx::query_as::<_, (i32, i32, String)>(
        "SELECT char_def_id, ability_id, source_kind::text \
           FROM resources.char_creation_abilities ORDER BY char_def_id, ability_id",
    )
    .fetch_all(&mut *conn)
    .await?;
    let items = sqlx::query_as::<_, (i32, i32, i32)>(
        "SELECT char_def_id, item_id, stack_size \
           FROM resources.char_creation_items ORDER BY char_def_id, item_id",
    )
    .fetch_all(&mut *conn)
    .await?;
    let debug_abilities = sqlx::query_scalar::<_, i32>(
        "SELECT ability_id FROM resources.char_creation_debug_kit_abilities ORDER BY ability_id",
    )
    .fetch_all(&mut *conn)
    .await?;
    let debug_items = sqlx::query_as::<_, (i32, i32)>(
        "SELECT item_id, stack_size FROM resources.char_creation_debug_kit_items ORDER BY item_id",
    )
    .fetch_all(&mut *conn)
    .await?;

    let mut profiles = Vec::with_capacity(rows.len());
    for r in rows {
        let start_state = StartState::try_from(r.start_state.as_str())
            .map_err(|e| LoadError::BadRow(format!("char_def {}: {e}", r.char_def_id)))?;
        profiles.push(StartProfile {
            char_def_id: r.char_def_id,
            profile_id: r.profile_id,
            alignment: r.alignment,
            archetype: r.archetype,
            world: r.starting_world,
            position: [r.starting_x, r.starting_y, r.starting_z],
            start_level: r.start_level,
            debug_kit: r.debug_kit,
            start_state,
            abilities: Vec::new(),
            items: Vec::new(),
        });
    }
    for (char_def_id, ability_id, kind) in abilities {
        let source = KitSource::try_from(kind.as_str())
            .map_err(|e| LoadError::BadRow(format!("char_def {char_def_id}: {e}")))?;
        profile_mut(&mut profiles, char_def_id, "char_creation_abilities")?
            .abilities
            .push(KitAbility { ability_id, source });
    }
    for (char_def_id, item_id, stack_size) in items {
        profile_mut(&mut profiles, char_def_id, "char_creation_items")?
            .items
            .push(KitItem {
                item_id,
                stack_size,
            });
    }
    Ok(StartProfiles::new(
        profiles,
        DebugKit {
            abilities: debug_abilities,
            items: debug_items
                .into_iter()
                .map(|(item_id, stack_size)| KitItem {
                    item_id,
                    stack_size,
                })
                .collect(),
        },
    ))
}

fn profile_mut<'p>(
    profiles: &'p mut [StartProfile],
    char_def_id: i32,
    table: &str,
) -> Result<&'p mut StartProfile, LoadError> {
    profiles
        .iter_mut()
        .find(|p| p.char_def_id == char_def_id)
        .ok_or_else(|| {
            LoadError::BadRow(format!(
                "{table} row for char_def {char_def_id} with no profile"
            ))
        })
}

/// Load the profiles into the process registry, once per service start.
/// The base and the cell both call it (they share the process), like the
/// name book; the second call reloads the same rows.
///
/// A profile with a problem is logged at ERROR and kept: character creation
/// refuses it on its own read, and the sync consumers only use its world
/// and point. A failed read installs nothing, also at ERROR: every consumer
/// then fails closed (no new-character start is known).
pub async fn load_at_boot(pool: &sqlx::PgPool) {
    let loaded = match pool.acquire().await {
        Ok(mut conn) => load_all(&mut conn).await,
        Err(e) => Err(LoadError::Db(e)),
    };
    match loaded {
        Ok(profiles) => {
            for p in profiles.profiles() {
                for problem in p.problems() {
                    tracing::error!(
                        event = "start_profile_invalid",
                        reason = problem.reason(),
                        char_def_id = p.char_def_id, // nt:id-only char_def rows carry the profile id below
                        profile_id = %p.profile_id, // nt:id-only profile key has no display name
                        world = %p.world,
                        detail = ?problem,
                        "start profile has a problem; character creation refuses it"
                    );
                }
            }
            tracing::info!(
                event = "start_profiles_loaded",
                count = profiles.profiles().len(),
                worlds = ?profiles.start_worlds(),
                debug_kit_abilities = profiles.debug_kit().abilities.len(),
                debug_kit_items = profiles.debug_kit().items.len(),
                "Loaded start profiles"
            );
            super::install(profiles);
        }
        Err(e) => tracing::error!(
            event = "start_profiles_load_failed",
            error = %e,
            "could not read the start profiles; new-character starts are unknown and \
             every consumer fails closed"
        ),
    }
}
