//! `server_stage_fixture` tool logic (lab-fixtures FX-04).
//!
//! Stages a row fixture (missions, items, tutorials, world and position)
//! for an offline character on a GM account. The base takes it at that
//! character's next Play and writes it before world entry
//! (`cimmeria_base_session::base::world_entry::lab_fixture`, FX-02). This
//! tool writes nothing to the database: it reads the character, its
//! account and the seed, and puts the fixture in the in-memory store.
//!
//! The store trusts its caller for three things (its `stage_fixture` doc),
//! and this module owns them: the account is a GM's, the character is not
//! online, and `account_id` is the one on the character's row. Everything
//! else the base apply would refuse later, it refuses here first where the
//! check is cheap, so a lab run learns about a bad fixture at stage time
//! instead of from a silent "not applied" ERROR after Play.
//!
//! Every refusal is a message starting [`REFUSED`] that names the field;
//! the adapter turns it into an `invalid_params` error.

use std::borrow::Cow;
use std::collections::HashSet;

use rmcp::schemars::{self, JsonSchema};
use serde::{Deserialize, Deserializer};
use serde_json::{json, Value};
use sqlx::PgPool;

use cimmeria_services::base::lab_fixture::{
    stage_fixture as store_fixture, StagedFixture, StagedItem, StagedMission, StagedMissionState,
    FIXTURE_MAX_ITEM_COUNT, GM_ACCESS_LEVEL,
};
use cimmeria_services::base::OnlinePlayer;

use crate::state::LabState;

/// The prefix of every refusal (the FX contract).
pub const REFUSED: &str = "stage refused: ";

fn refused(msg: impl std::fmt::Display) -> String {
    format!("{REFUSED}{msg}")
}

/// The tool's arguments: the lab runner's `FixtureSpec` JSON with
/// `character` set to the fixture character's name.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StageFixtureArgs {
    /// The character: its full name or the name's last word, compared
    /// case-insensitively (the runner sends the fixture name, e.g.
    /// `Fxpraxisdefa` for `Uat Fxpraxisdefa`). Exactly one must match.
    pub character: String,
    /// The world to put the character in (a `resources.worlds` name).
    #[serde(default)]
    pub world: Option<String>,
    /// Server metres `[x, y, z]`, as `/gmgotoxyz` takes them.
    #[serde(default)]
    pub position: Option<[f32; 3]>,
    /// Missions to set: `step` makes one active on that step; otherwise
    /// `state` is `not_active` or `completed`.
    #[serde(default)]
    pub missions: Vec<MissionArg>,
    /// Exact carried counts, 0 to 10, per item design id.
    #[serde(default)]
    pub items: Vec<ItemArg>,
    /// Tutorial ids to mark unseen again.
    #[serde(default)]
    pub tutorials_unseen: Vec<i32>,
}

/// One mission of the fixture.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MissionArg {
    pub id: i32,
    /// Active on this step (a `resources.mission_steps` row of this mission).
    #[serde(default)]
    pub step: Option<i32>,
    #[serde(default)]
    pub state: Option<MissionStateArg>,
}

/// The spec's `FixtureMissionState`, as the runner serializes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MissionStateArg {
    Active,
    NotActive,
    Completed,
}

/// One exact carried count of the fixture.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ItemArg {
    /// Item design id (`resources.items.item_id`).
    pub id: i32,
    /// Carried count after the fixture, 0 to 10.
    pub count: u32,
    /// `main` (the default) or `mission`.
    #[serde(default)]
    pub container: Option<String>,
}

/// The raw call: any JSON, parsed by [`parse_args`]. rmcp would otherwise
/// reject a malformed call before the tool runs, with its own message and
/// no audit row; this way that call is a `stage refused:` refusal too and
/// is audited like every other. Its schema is [`StageFixtureArgs`]'s.
#[derive(Debug)]
pub struct StageFixtureCall(pub Value);

impl<'de> Deserialize<'de> for StageFixtureCall {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Value::deserialize(d).map(Self)
    }
}

impl JsonSchema for StageFixtureCall {
    fn schema_name() -> Cow<'static, str> {
        StageFixtureArgs::schema_name()
    }

    fn schema_id() -> Cow<'static, str> {
        StageFixtureArgs::schema_id()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        StageFixtureArgs::json_schema(generator)
    }
}

/// Parse the call's arguments; a bad shape is a refusal.
pub fn parse_args(v: Value) -> Result<StageFixtureArgs, String> {
    serde_json::from_value(v).map_err(|e| refused(format!("arguments: {e}")))
}

/// The tool: parse, gate, check and stage. See the module docs.
pub async fn stage_fixture(state: &LabState, call: StageFixtureCall) -> Result<Value, String> {
    let args = parse_args(call.0)?;
    let Some(pool) = state.db_pool().await else {
        return Err(refused("no database"));
    };
    let online = state.online_players().await;
    stage_with(&pool, &online, args).await
}

/// The character a fixture is staged for, with its account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Character {
    pub(crate) player_id: i32,
    pub(crate) player_name: String,
    pub(crate) account_id: i32,
    pub(crate) account_name: String,
    pub(crate) accesslevel: i32,
}

/// [`stage_fixture`] past the pool and the online snapshot, so the live-DB
/// tests run the same path with their own roster.
pub(crate) async fn stage_with(
    pool: &PgPool,
    online: &[OnlinePlayer],
    args: StageFixtureArgs,
) -> Result<Value, String> {
    let ch = resolve_character(pool, &args.character).await?;
    if i64::from(ch.accesslevel) < i64::from(GM_ACCESS_LEVEL) {
        return Err(refused(format!(
            "character {} is not on a GM account",
            args.character
        )));
    }
    refuse_if_online(&args.character, &ch, online)?;
    let (missions, items) = check_shape(&args)?;
    check_seed(pool, &args, &missions).await?;

    let (mission_count, item_count, tutorial_count) =
        (missions.len(), items.len(), args.tutorials_unseen.len());
    let replaced = store_fixture(StagedFixture {
        player_id: ch.player_id,
        // The store hands the fixture only to a session of this account; a
        // wrong value would leave it staged at the owner's own Play.
        account_id: ch.account_id,
        player_name: ch.player_name.clone(),
        world: args.world.clone(),
        position: args.position,
        missions,
        items,
        tutorials_unseen: args.tutorials_unseen,
    });
    tracing::info!(
        player_id = ch.player_id,
        player_name = %ch.player_name,
        account_id = ch.account_id,
        account_name = %ch.account_name,
        world = args.world.as_deref(),
        missions = mission_count,
        items = item_count,
        tutorials_unseen = tutorial_count,
        replaced,
        "lab fixture staged"
    );
    Ok(json!({
        "staged": true,
        "player_id": ch.player_id,
        "player_name": ch.player_name,
        "replaced": replaced,
    }))
}

/// The one character whose name, or whose name's last space-separated
/// word, equals `name` case-insensitively.
async fn resolve_character(pool: &PgPool, name: &str) -> Result<Character, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(refused("character is empty"));
    }
    let rows: Vec<(i32, String, i32, String, i32)> = sqlx::query_as(
        "SELECT p.player_id, p.player_name, a.account_id, a.account_name, a.accesslevel \
         FROM sgw_player p JOIN account a ON a.account_id = p.account_id \
         WHERE lower(p.player_name) = lower($1) \
            OR lower(substring(p.player_name from '[^ ]*$')) = lower($1) \
         ORDER BY p.player_id",
    )
    .bind(name)
    .fetch_all(pool)
    .await
    .map_err(|e| refused(format!("character {name} lookup failed: {e}")))?;
    match rows.as_slice() {
        [(player_id, player_name, account_id, account_name, accesslevel)] => Ok(Character {
            player_id: *player_id,
            player_name: player_name.clone(),
            account_id: *account_id,
            account_name: account_name.clone(),
            accesslevel: *accesslevel,
        }),
        [] => Err(refused(format!("character {name} matches no character"))),
        many => Err(refused(format!(
            "character {name} matches {} characters",
            many.len()
        ))),
    }
}

/// Refuse a character that has a session: the base reads the fixture's
/// rows only at Play, so staging for a character in the world would
/// rewrite it under a later login instead. A session is the character's
/// when its player id matches, or, before `playCharacter` set one, when it
/// carries the character's name.
pub(crate) fn refuse_if_online(
    requested: &str,
    ch: &Character,
    online: &[OnlinePlayer],
) -> Result<(), String> {
    let is_this = |p: &OnlinePlayer| match p.player_id {
        Some(id) => id == ch.player_id,
        None => !p.name.is_empty() && p.name.eq_ignore_ascii_case(&ch.player_name),
    };
    if online.iter().any(is_this) {
        return Err(refused(format!("character {requested} is online")));
    }
    Ok(())
}

/// The checks that need no database, and the staged missions and items.
pub(crate) fn check_shape(
    args: &StageFixtureArgs,
) -> Result<(Vec<StagedMission>, Vec<StagedItem>), String> {
    if let Some(p) = args.position {
        // A JSON number beyond f32 range arrives as infinity.
        if !p.iter().all(|c| c.is_finite()) {
            return Err(refused(format!("position {p:?} is not finite")));
        }
    }
    if let Some(world) = args.world.as_deref() {
        if world.trim().is_empty() {
            return Err(refused("world is empty"));
        }
    }
    let mut seen = HashSet::new();
    let mut missions = Vec::with_capacity(args.missions.len());
    for (i, m) in args.missions.iter().enumerate() {
        if !seen.insert(m.id) {
            return Err(refused(format!(
                "missions[{i}]: mission {} is listed twice",
                m.id
            )));
        }
        missions.push(StagedMission {
            mission_id: m.id,
            state: mission_state(i, m)?,
        });
    }
    let mut seen = HashSet::new();
    let mut items = Vec::with_capacity(args.items.len());
    for (i, it) in args.items.iter().enumerate() {
        if it.count > FIXTURE_MAX_ITEM_COUNT {
            return Err(refused(format!(
                "items[{i}]: count {} of item {} is over {FIXTURE_MAX_ITEM_COUNT}",
                it.count, it.id
            )));
        }
        let container_id = match it.container.as_deref() {
            None | Some("main") => 1,
            Some("mission") => 2,
            Some(other) => {
                return Err(refused(format!(
                    "items[{i}]: container {other:?} of item {} is not main or mission",
                    it.id
                )))
            }
        };
        // The base sets a design's count across every carried container,
        // so a second entry for the same design would undo the first.
        if !seen.insert(it.id) {
            return Err(refused(format!(
                "items[{i}]: item {} is listed twice",
                it.id
            )));
        }
        items.push(StagedItem {
            type_id: it.id,
            count: it.count,
            container_id,
        });
    }
    if let Some(t) = args.tutorials_unseen.iter().find(|&&t| t <= 0) {
        return Err(refused(format!(
            "tutorials_unseen: {t} is not a tutorial id"
        )));
    }
    Ok((missions, items))
}

/// The spec's rule (`FixtureMission::resolved_state`): a step means
/// active; active needs a step; one of the two must be there.
fn mission_state(i: usize, m: &MissionArg) -> Result<StagedMissionState, String> {
    match (m.step, m.state) {
        (Some(step_id), None | Some(MissionStateArg::Active)) => {
            Ok(StagedMissionState::Active { step_id })
        }
        (Some(_), Some(s)) => Err(refused(format!(
            "missions[{i}]: mission {} has a step, so it is active, not {s:?}",
            m.id
        ))),
        (None, Some(MissionStateArg::Active)) => Err(refused(format!(
            "missions[{i}]: mission {} is active but has no step",
            m.id
        ))),
        (None, Some(MissionStateArg::NotActive)) => Ok(StagedMissionState::NotActive),
        (None, Some(MissionStateArg::Completed)) => Ok(StagedMissionState::Completed),
        (None, None) => Err(refused(format!(
            "missions[{i}]: mission {} needs a step or a state",
            m.id
        ))),
    }
}

/// The checks against the seed, the same ones the base precheck runs
/// before its first write (`base-world-entry`'s `lab_fixture::precheck`
/// and `plan_missions`), plus the item design ids FX-03 checks.
async fn check_seed(
    pool: &PgPool,
    args: &StageFixtureArgs,
    missions: &[StagedMission],
) -> Result<(), String> {
    if let Some(world) = args.world.as_deref() {
        // The base precheck's lookup: `sgw_player.world_location` is a
        // foreign key into `resources.worlds`. (`.gotolocation` resolves
        // through the cell's in-memory world catalog, loaded from the same
        // table; this endpoint has no cell round trip for it.)
        if !exists(
            pool,
            "SELECT EXISTS (SELECT 1 FROM resources.worlds WHERE world = $1)",
            world,
        )
        .await?
        {
            return Err(refused(format!("world {world} is not in resources.worlds")));
        }
    }
    for (i, m) in missions.iter().enumerate() {
        let known = exists_id(
            pool,
            "SELECT EXISTS (SELECT 1 FROM resources.missions WHERE mission_id = $1)",
            m.mission_id,
        )
        .await?;
        if !known {
            return Err(refused(format!(
                "missions[{i}]: mission {} is not in resources.missions",
                m.mission_id
            )));
        }
        match m.state {
            StagedMissionState::Active { step_id } => {
                let owner: Option<i32> = sqlx::query_scalar(
                    "SELECT mission_id FROM resources.mission_steps WHERE step_id = $1",
                )
                .bind(step_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| {
                    refused(format!("missions[{i}]: step {step_id} lookup failed: {e}"))
                })?;
                match owner {
                    None => {
                        return Err(refused(format!(
                            "missions[{i}].step: step {step_id} is not in resources.mission_steps"
                        )))
                    }
                    Some(owner) if owner != m.mission_id => {
                        return Err(refused(format!(
                            "missions[{i}].step: step {step_id} belongs to mission {owner}, not mission {}",
                            m.mission_id
                        )))
                    }
                    Some(_) => {}
                }
            }
            StagedMissionState::Completed => {
                let has_steps = exists_id(
                    pool,
                    "SELECT EXISTS (SELECT 1 FROM resources.mission_steps WHERE mission_id = $1)",
                    m.mission_id,
                )
                .await?;
                if !has_steps {
                    return Err(refused(format!(
                        "missions[{i}]: mission {} has no steps to complete",
                        m.mission_id
                    )));
                }
            }
            StagedMissionState::NotActive => {}
        }
    }
    for (i, it) in args.items.iter().enumerate() {
        let known = exists_id(
            pool,
            "SELECT EXISTS (SELECT 1 FROM resources.items WHERE item_id = $1)",
            it.id,
        )
        .await?;
        if !known {
            return Err(refused(format!(
                "items[{i}]: item {} is not in resources.items",
                it.id
            )));
        }
    }
    Ok(())
}

async fn exists(pool: &PgPool, sql: &'static str, key: &str) -> Result<bool, String> {
    sqlx::query_scalar(sql)
        .bind(key)
        .fetch_one(pool)
        .await
        .map_err(|e| refused(format!("seed lookup failed: {e}")))
}

async fn exists_id(pool: &PgPool, sql: &'static str, id: i32) -> Result<bool, String> {
    sqlx::query_scalar(sql)
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| refused(format!("seed lookup failed: {e}")))
}

#[cfg(test)]
#[path = "fixture_tests.rs"]
mod tests;
