//! `server_stage_fixture`'s arguments and their parse (lab-fixtures FX-04).
//!
//! The tool takes the raw JSON ([`StageFixtureCall`]) and parses it here,
//! so a malformed call is a `stage refused:` refusal that names the field
//! (`missions[0].state`), and is audited like every other call.

use std::borrow::Cow;

use rmcp::schemars::{self, JsonSchema};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use super::Refusal;

/// The tool's arguments: the lab runner's `FixtureSpec` JSON with
/// `character` set to the fixture character's name.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StageFixtureArgs {
    /// The character: its full name or the name's last word, compared
    /// case-insensitively (the runner sends the fixture name, e.g.
    /// `Fxpraxisdefa` for `Uat Fxpraxisdefa`). Exactly one must match, and
    /// its name's last word must be a fixture name (`Fx` then letters).
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

/// Parse the call's arguments; a bad shape is a refusal naming the field.
///
/// `serde_path_to_error` gives the path of a wrong type or bad value
/// (`missions[0].state`, `items[0].count`). A missing field is reported at
/// its parent, so the one required top-level field is checked by hand
/// first, to name `character` rather than the root.
pub fn parse_args(v: Value) -> Result<StageFixtureArgs, Refusal> {
    let Some(obj) = v.as_object() else {
        return Err(Refusal::caller(
            "arguments",
            "arguments: the call is not a JSON object",
        ));
    };
    if !obj.contains_key("character") {
        return Err(Refusal::caller(
            "arguments",
            "arguments: character: missing (the fixture character's name)",
        ));
    }
    serde_path_to_error::deserialize(v).map_err(|e| {
        let path = e.path().to_string();
        let inner = e.into_inner();
        // The root path prints as "."; an error there (an unknown
        // top-level field) names the field in its own message.
        let msg = if path == "." {
            format!("arguments: {inner}")
        } else {
            format!("arguments: {path}: {inner}")
        };
        Refusal::caller("arguments", msg)
    })
}
