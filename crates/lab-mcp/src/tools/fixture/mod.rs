//! `server_stage_fixture` tool logic (lab-fixtures FX-04).
//!
//! Stages a row fixture (missions, items, tutorials, world and position)
//! for an offline lab fixture character on a GM account. The base takes it
//! at that character's next Play and writes it before world entry
//! (`cimmeria_base_session::base::world_entry::lab_fixture`, FX-02). This
//! tool writes nothing to the database: it reads the character, its
//! account and the seed, and puts the fixture in the in-memory store.
//!
//! The store trusts its caller for three things (its `stage_fixture` doc),
//! and this module owns them: the account is a GM's, the character is not
//! online, and `account_id` is the one on the character's row. It adds a
//! fourth: the character is a lab fixture character (its name's last word
//! is `Fx` then letters, the runner's `fixture_character_name`), so a
//! token holder cannot stage onto a real GM's main. Everything else the
//! base apply would refuse later, it refuses here first where the check is
//! cheap, so a lab run learns about a bad fixture at stage time instead of
//! from a silent "not applied" ERROR after Play.
//!
//! Every refusal is a [`Refusal`] whose message starts [`REFUSED`] and
//! names the field, logged once as WARN "lab fixture stage refused" with a
//! stable `reason`. A database failure is not the caller's fault: it logs
//! ERROR with `reason = "lookup_failed"` and the adapter returns an
//! internal error instead of `invalid_params`.

use std::collections::HashSet;
use std::future::Future;

use serde_json::{json, Value};
use sqlx::PgPool;

use cimmeria_services::base::lab_fixture::{
    stage_fixture as store_fixture, StagedFixture, StagedItem, StagedMission, StagedMissionState,
    FIXTURE_MAX_ITEM_COUNT, GM_ACCESS_LEVEL,
};
use cimmeria_services::base::OnlinePlayer;

use crate::state::LabState;

mod args;
mod seed;
pub use args::{parse_args, StageFixtureArgs, StageFixtureCall};
use args::{ItemArg, MissionArg, MissionStateArg};
use seed::check_seed;

/// The prefix of every refusal (the FX contract).
pub const REFUSED: &str = "stage refused: ";

/// Why a stage was refused. `reason` is the stable log field (Rule 6);
/// `message` is what the caller sees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub reason: &'static str,
    pub message: String,
    /// A database failure, not a bad call: logged ERROR, returned as an
    /// internal error.
    pub internal: bool,
    /// The resolved character, once known, for the log.
    pub player: Option<(i32, String)>,
}

impl Refusal {
    pub(crate) fn caller(reason: &'static str, msg: impl std::fmt::Display) -> Self {
        Self {
            reason,
            message: format!("{REFUSED}{msg}"),
            internal: false,
            player: None,
        }
    }

    fn lookup_failed(what: impl std::fmt::Display, e: sqlx::Error) -> Self {
        Self {
            reason: "lookup_failed",
            message: format!("{REFUSED}{what} lookup failed: {e}"),
            internal: true,
            player: None,
        }
    }

    fn for_player(mut self, ch: &Character) -> Self {
        self.player = Some((ch.player_id, ch.player_name.clone()));
        self
    }

    /// The one log line per refusal.
    fn log(&self) {
        let player_id = self.player.as_ref().map(|p| p.0);
        let player_name = self.player.as_ref().map(|p| p.1.as_str());
        if self.internal {
            tracing::error!(
                reason = self.reason,
                player_id,
                player_name,
                error = %self.message,
                "lab fixture stage refused"
            );
        } else {
            tracing::warn!(
                reason = self.reason,
                player_id,
                player_name,
                error = %self.message,
                "lab fixture stage refused"
            );
        }
    }
}

/// Where the tool gets the database and the online roster. Production
/// passes [`LabState`]; the tests pass their own, so the path from the
/// roster to the online gate is the one the tool runs.
pub(crate) trait StageContext: Sync {
    fn pool(&self) -> impl Future<Output = Option<PgPool>> + Send;
    fn roster(&self) -> impl Future<Output = Vec<OnlinePlayer>> + Send;
}

impl StageContext for LabState {
    fn pool(&self) -> impl Future<Output = Option<PgPool>> + Send {
        LabState::db_pool(self)
    }

    fn roster(&self) -> impl Future<Output = Vec<OnlinePlayer>> + Send {
        LabState::online_players(self)
    }
}

/// The tool: parse, gate, check and stage. See the module docs.
pub async fn stage_fixture(state: &LabState, call: StageFixtureCall) -> Result<Value, Refusal> {
    stage_from(state, call.0).await
}

/// [`stage_fixture`] over any [`StageContext`]; logs every refusal once.
pub(crate) async fn stage_from(ctx: &impl StageContext, call: Value) -> Result<Value, Refusal> {
    let result = async {
        let args = parse_args(call)?;
        let Some(pool) = ctx.pool().await else {
            return Err(Refusal::caller("no_database", "no database"));
        };
        let online = ctx.roster().await;
        stage_with(&pool, &online, args).await
    }
    .await;
    if let Err(r) = &result {
        r.log();
    }
    result
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

/// The gates and checks past the pool and the online snapshot.
async fn stage_with(
    pool: &PgPool,
    online: &[OnlinePlayer],
    args: StageFixtureArgs,
) -> Result<Value, Refusal> {
    let ch = resolve_character(pool, &args.character).await?;
    let (missions, items) = async {
        gate_character(&args.character, &ch, online)?;
        let staged = check_shape(&args)?;
        check_seed(pool, &args, &staged.0).await?;
        Ok::<_, Refusal>(staged)
    }
    .await
    .map_err(|e| e.for_player(&ch))?;

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
async fn resolve_character(pool: &PgPool, name: &str) -> Result<Character, Refusal> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Refusal::caller("no_character", "character is empty"));
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
    .map_err(|e| Refusal::lookup_failed(format_args!("character {name}"), e))?;
    match rows.as_slice() {
        [(player_id, player_name, account_id, account_name, accesslevel)] => Ok(Character {
            player_id: *player_id,
            player_name: player_name.clone(),
            account_id: *account_id,
            account_name: account_name.clone(),
            accesslevel: *accesslevel,
        }),
        [] => Err(Refusal::caller(
            "no_character",
            format!("character {name} matches no character"),
        )),
        many => Err(Refusal::caller(
            "ambiguous",
            format!("character {name} matches {} characters", many.len()),
        )),
    }
}

/// The character gates, in order: a lab fixture character, on a GM
/// account, not online.
pub(crate) fn gate_character(
    requested: &str,
    ch: &Character,
    online: &[OnlinePlayer],
) -> Result<(), Refusal> {
    // First, so a probe with a real character's name learns nothing about
    // its account.
    if !is_fixture_name(&ch.player_name) {
        return Err(Refusal::caller(
            "not_fixture_character",
            format!(
                "character {} is not a lab fixture character (last name must be Fx...)",
                ch.player_name
            ),
        ));
    }
    if i64::from(ch.accesslevel) < i64::from(GM_ACCESS_LEVEL) {
        return Err(Refusal::caller(
            "not_gm",
            format!("character {requested} is not on a GM account"),
        ));
    }
    refuse_if_online(requested, ch, online)
}

/// `true` when the last space-separated word of `player_name` matches
/// `^Fx[A-Za-z]+$`: the shape of the runner's `fixture_character_name`
/// (`crates/lab/src/uat/fixture.rs`). Case-sensitive, like the regex.
pub(crate) fn is_fixture_name(player_name: &str) -> bool {
    let last = player_name.rsplit(' ').next().unwrap_or("");
    last.strip_prefix("Fx")
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_alphabetic()))
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
) -> Result<(), Refusal> {
    let is_this = |p: &OnlinePlayer| match p.player_id {
        Some(id) => id == ch.player_id,
        None => !p.name.is_empty() && p.name.eq_ignore_ascii_case(&ch.player_name),
    };
    if online.iter().any(is_this) {
        return Err(Refusal::caller(
            "online",
            format!("character {requested} is online"),
        ));
    }
    Ok(())
}

/// The checks that need no database, and the staged missions and items.
pub(crate) fn check_shape(
    args: &StageFixtureArgs,
) -> Result<(Vec<StagedMission>, Vec<StagedItem>), Refusal> {
    if let Some(p) = args.position {
        // A JSON number beyond f32 range arrives as infinity.
        if !p.iter().all(|c| c.is_finite()) {
            return Err(Refusal::caller(
                "position",
                format!("position {p:?} is not finite"),
            ));
        }
    }
    if let Some(world) = args.world.as_deref() {
        if world.trim().is_empty() {
            return Err(Refusal::caller("world", "world is empty"));
        }
    }
    let mut seen = HashSet::new();
    let mut missions = Vec::with_capacity(args.missions.len());
    for (i, m) in args.missions.iter().enumerate() {
        if !seen.insert(m.id) {
            return Err(Refusal::caller(
                "mission",
                format!("missions[{i}]: mission {} is listed twice", m.id),
            ));
        }
        missions.push(StagedMission {
            mission_id: m.id,
            state: mission_state(i, m)?,
        });
    }
    let mut seen = HashSet::new();
    let mut items = Vec::with_capacity(args.items.len());
    for (i, it) in args.items.iter().enumerate() {
        items.push(staged_item(i, it)?);
        // The base sets a design's count across every carried container,
        // so a second entry for the same design would undo the first.
        if !seen.insert(it.id) {
            return Err(Refusal::caller(
                "item",
                format!("items[{i}]: item {} is listed twice", it.id),
            ));
        }
    }
    if let Some(t) = args.tutorials_unseen.iter().find(|&&t| t <= 0) {
        return Err(Refusal::caller(
            "tutorial",
            format!("tutorials_unseen: {t} is not a tutorial id"),
        ));
    }
    Ok((missions, items))
}

fn staged_item(i: usize, it: &ItemArg) -> Result<StagedItem, Refusal> {
    if it.count > FIXTURE_MAX_ITEM_COUNT {
        return Err(Refusal::caller(
            "item",
            format!(
                "items[{i}]: count {} of item {} is over {FIXTURE_MAX_ITEM_COUNT}",
                it.count, it.id
            ),
        ));
    }
    let container_id = match it.container.as_deref() {
        None | Some("main") => 1,
        Some("mission") => 2,
        Some(other) => {
            return Err(Refusal::caller(
                "item",
                format!(
                    "items[{i}]: container {other:?} of item {} is not main or mission",
                    it.id
                ),
            ))
        }
    };
    Ok(StagedItem {
        type_id: it.id,
        count: it.count,
        container_id,
    })
}

/// The spec's rule (`FixtureMission::resolved_state`): a step means
/// active; active needs a step; one of the two must be there.
fn mission_state(i: usize, m: &MissionArg) -> Result<StagedMissionState, Refusal> {
    use self::MissionStateArg as S;
    let bad = |msg: String| Err(Refusal::caller("mission", format!("missions[{i}]: {msg}")));
    match (m.step, m.state) {
        (Some(step_id), None | Some(S::Active)) => Ok(StagedMissionState::Active { step_id }),
        (Some(_), Some(s)) => bad(format!(
            "mission {} has a step, so it is active, not {s:?}",
            m.id
        )),
        (None, Some(S::Active)) => bad(format!("mission {} is active but has no step", m.id)),
        (None, Some(S::NotActive)) => Ok(StagedMissionState::NotActive),
        (None, Some(S::Completed)) => Ok(StagedMissionState::Completed),
        (None, None) => bad(format!("mission {} needs a step or a state", m.id)),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod live_db_tests;
