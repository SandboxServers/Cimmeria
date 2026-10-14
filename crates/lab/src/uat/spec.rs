//! Row specs: one TOML file per unified-UAT section under
//! `docs/guides/uat-specs/`, mirroring the guide's rows.
//!
//! Why TOML: the repo already reads TOML everywhere (Cargo, nextest,
//! `config/discord.toml`, the crate-graph groups), the `toml` crate is a
//! workspace dependency, it has comments and multi-line strings for the
//! guide's prose, and `[[row]]` / `[[row.expect]]` arrays of tables read
//! like the guide's own table. YAML would need a new, unmaintained
//! dependency and brings indentation-sensitive surprises into data that
//! non-programmers may edit.
//!
//! The format is documented for authors in
//! `docs/guides/automated-uat.md`; this module is the schema and
//! [`super::spec_validate`] the rules the types cannot express.

use serde::Deserialize;
use serde_json::Value;

use super::tier::Tier;

pub use super::spec_fixture::FixtureSpec;
pub use super::spec_validate::{validate, without_vars};

/// Current spec schema version.
pub const SCHEMA: u32 = 1;

/// One section file.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionSpec {
    pub schema: u32,
    pub section: SectionMeta,
    #[serde(default, rename = "row")]
    pub rows: Vec<RowSpec>,
}

/// Section-wide metadata and the default session every row starts from.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionMeta {
    /// Short id used on the command line and in the bundle paths
    /// (`gm-parity`, `chat`).
    pub id: String,
    /// The ledger's `System:` value (the guide's section title).
    pub system: String,
    /// Anchor in `docs/guides/unified-uat.md`.
    pub guide: String,
    /// The campaign's canonical checklist (the ledger the results go to).
    pub ledger: String,
    /// `gm` or `non-gm`: which account the rows need.
    #[serde(default = "default_account")]
    pub account: String,
    /// The character rows play: `lab` (lab-account.json's character) or
    /// `fresh` (a new one per run, see [`FreshCharacter`]).
    #[serde(default = "default_character")]
    pub character: String,
    /// Stop later rows after a prior FAIL or BLOCKED in this section.
    #[serde(default)]
    pub sequential: bool,
    #[serde(default)]
    pub fresh: Option<FreshCharacter>,
    /// Reusable fixture characters, by profile name (`[section.profiles.praxis]`).
    #[serde(default)]
    pub profiles: std::collections::BTreeMap<String, FreshCharacter>,
    // lab-golden (GD-01)
    /// The section-wide variance every row inherits (see [`GoldenSpec`]).
    #[serde(default)]
    pub golden: Option<GoldenSpec>,
    // end lab-golden
}

fn default_account() -> String {
    "gm".into()
}
fn default_character() -> String {
    "lab".into()
}

/// How to make a fresh character. The last name is generated from the run
/// id (letters only) so each run's characters are distinct and findable.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FreshCharacter {
    pub alignment: String,
    pub archetype: String,
    pub gender: String,
    #[serde(default = "default_first")]
    pub first: String,
    /// Finish the initial dialog after character creation (default true).
    #[serde(default = "default_true")]
    pub finish_intro: bool,
}

fn default_first() -> String {
    "Uat".into()
}

/// One guide row.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowSpec {
    /// The campaign's step id (`M1-1`, `U12`, `9a`).
    pub id: String,
    /// Earlier row ids this row requires; FAIL or BLOCKED stops this row.
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub title: String,
    /// The guide's "Expect" text, verbatim enough for the ledger's
    /// `Expected:` line.
    pub expected: String,
    /// The least native tier a step action may run at for a PASS.
    #[serde(default = "default_native")]
    pub required_native: Tier,
    /// `in_world` (default), `char_select`, `any` or `client_stopped`.
    #[serde(default = "default_state")]
    pub state: String,
    /// Players the row needs: 1, or 2 to drive the second lab instance
    /// (`p2`) as well. A 2-player row is BLOCKED, with the reason, when no
    /// second instance is configured; above 2 always is (matrix X1).
    #[serde(default = "default_players")]
    pub players: u32,
    /// K-numbers or step notes the guide attaches.
    #[serde(default)]
    pub known_issues: Vec<String>,
    /// A standing reason this row cannot run (owner-only files, a local
    /// server). The row is reported BLOCKED with it, nothing runs.
    #[serde(default)]
    pub blocked: Option<String>,
    /// Type `.bug uat <row id>` before the steps (default true in world).
    #[serde(default)]
    pub anchor: Option<bool>,
    /// What the ledger's `After relog:` line should say when the row
    /// checks a relog itself (`same`), else `not tried`.
    #[serde(default)]
    pub relog: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub setup: Vec<ActionSpec>,
    #[serde(default, rename = "step")]
    pub steps: Vec<ActionSpec>,
    #[serde(default)]
    pub teardown: Vec<ActionSpec>,
    #[serde(default)]
    pub expect: Vec<ExpectSpec>,
    #[serde(default)]
    pub evidence: Vec<EvidenceSpec>,
    // lab-golden (GD-01)
    /// This row's variance, added to the section's (see [`GoldenSpec`]).
    #[serde(default)]
    pub golden: Option<GoldenSpec>,
    // end lab-golden
    // lab-spec-vocab (SV-03)
    /// Interaction poses: where to stand and how to aim at a tagged target.
    #[serde(default)]
    pub pose: Vec<PoseSpec>,
    // end lab-spec-vocab
    /// The state the runner establishes before setup (lab-fixtures).
    #[serde(default)]
    pub fixture: Option<FixtureSpec>,
}

fn default_native() -> Tier {
    Tier::N1
}
fn default_state() -> String {
    "in_world".into()
}
fn default_players() -> u32 {
    1
}

/// One action. Exactly one of `tool`, `chat`, `wait_ms`, `capture`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionSpec {
    /// A tool name (data: the runner looks it up in the live router, and
    /// a missing one BLOCKs the row naming it).
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub args: Option<Value>,
    /// Named leg from the committed route catalog, for `client_move_to`.
    #[serde(default)]
    pub route: Option<String>,
    /// A line typed into chat (Enter, type, Enter), or sent through
    /// `client_chat_send` when that tool exists.
    #[serde(default)]
    pub chat: Option<String>,
    /// Sleep this long (for server round trips the runner cannot observe).
    #[serde(default)]
    pub wait_ms: Option<u64>,
    /// Read a source (`chat`) and store `regex` group 1 into `var`.
    #[serde(default)]
    pub capture: Option<String>,
    #[serde(default)]
    pub regex: Option<String>,
    #[serde(default)]
    pub var: Option<String>,
    // lab-spec-vocab (SV-03)
    /// Wait for a server event (a log line) instead of sleeping.
    #[serde(default)]
    pub wait_server: Option<ServerWait>,
    // end lab-spec-vocab
    /// Declared tier (see [`super::tier`] for what a spec may claim).
    #[serde(default)]
    pub tier: Option<Tier>,
    /// Name for `at =` / `since =` / timing references.
    #[serde(default)]
    pub label: Option<String>,
    /// Tried in order when the primary tool is not routed. Their tier is
    /// what the row records, so an N3 fallback costs the row its PASS.
    #[serde(default)]
    pub fallback: Vec<ActionSpec>,
    /// An error here is recorded but does not fail or block the row.
    #[serde(default)]
    pub optional: bool,
    /// Which lab client runs it: `p1` (default, the lab character) or
    /// `p2` (the second instance; `players = 2` rows only).
    #[serde(default)]
    pub client: Option<String>,
}

impl ActionSpec {
    /// Resolve a route to ordinary `client_move_to` arguments.
    pub fn route_args(&self) -> Result<Option<Value>, String> {
        let Some(name) = &self.route else {
            return Ok(None);
        };
        if !matches!(self.tool.as_deref(), Some("client_move_to" | "@move_to")) {
            return Err("route requires client_move_to or @move_to".into());
        }
        let mut resolved = crate::supervisor::world::routes::move_args(name)?;
        if let Some(args) = &self.args {
            let extra = args
                .as_object()
                .ok_or("route action args must be a table")?;
            if extra
                .keys()
                .any(|k| matches!(k.as_str(), "point" | "waypoints" | "entity_id" | "name"))
            {
                return Err("route cannot be combined with another move target".into());
            }
            resolved
                .as_object_mut()
                .expect("route arguments are an object")
                .extend(extra.clone());
        }
        Ok(Some(resolved))
    }

    /// Which of the five kinds this is, or why it is malformed.
    pub fn kind(&self) -> Result<ActionKind, String> {
        let set = [
            self.tool.is_some(),
            self.chat.is_some(),
            self.wait_ms.is_some(),
            self.capture.is_some(),
            // lab-spec-vocab (SV-03)
            self.wait_server.is_some(),
            // end lab-spec-vocab
        ];
        match set.iter().filter(|b| **b).count() {
            1 => {}
            0 => {
                return Err(
                    "an action needs one of tool, chat, wait_ms, capture, wait_server".into(),
                )
            }
            _ => {
                return Err(
                    "an action has more than one of tool, chat, wait_ms, capture, wait_server"
                        .into(),
                )
            }
        }
        Ok(if self.tool.is_some() {
            ActionKind::Tool
        } else if self.chat.is_some() {
            ActionKind::Chat
        } else if self.wait_ms.is_some() {
            ActionKind::Wait
        } else if self.capture.is_some() {
            ActionKind::Capture
        } else {
            // lab-spec-vocab (SV-03)
            ActionKind::ServerWait
            // end lab-spec-vocab
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    Tool,
    Chat,
    Wait,
    Capture,
    // lab-spec-vocab (SV-03)
    ServerWait,
    // end lab-spec-vocab
}

/// Where an expected clause gets its observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// Chat lines that arrived after `since` (default: the row's steps).
    Chat,
    /// Any client-lab tool's JSON, at a JSON pointer.
    Tool,
    /// `client_lua_eval` of `chunk`; the first result.
    Lua,
    /// `client_wait_for` of `lua_condition`.
    Wait,
    /// An action's elapsed time.
    Timing,
    /// A SigNoz query: recorded PENDING until `lab_uat_attest` fills it.
    Signoz,
    /// A server lab-mcp tool over HTTP (UNVERIFIED when unreachable).
    /// `server_ability_state` without an `entity_id` reads the lab
    /// character's own entity.
    Server,
    /// The client's own telemetry events (`client.ability.*` and the rest)
    /// from the lab event store, since the row's anchor or a step label:
    /// counted and field-checked like a packet clause.
    ClientEvent,
    /// Decoded Mercury messages from the server packet tap, captured for
    /// the lab character's session from the anchor to teardown
    /// (UNVERIFIED when the endpoint is unreachable).
    Packet,
    /// A question for a person (NEEDS_HUMAN until answered).
    Human,
}

/// Comparison applied to an observed value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    Eq,
    Ne,
    Contains,
    NotContains,
    Matches,
    Gt,
    Gte,
    Lt,
    Lte,
    Exists,
    Absent,
    Truthy,
    Falsy,
    LenGte,
    /// Numeric `value` within `tolerance` either way (`15 ± 1`).
    Approx,
}

/// One expected clause. The fields used depend on `source`;
/// [`validate`] rejects a clause missing what its source needs.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectSpec {
    pub id: String,
    /// The clause in the guide's words (goes into `Saw:` / the bundle).
    pub text: String,
    pub source: Source,
    /// A required clause that is not PASS keeps the row from passing.
    #[serde(default = "default_true")]
    pub required: bool,
    /// Evaluate right after the action with this label (default: after
    /// every step action).
    #[serde(default)]
    pub at: Option<String>,
    /// Chat and client_event: only lines (events) after the action with
    /// this label started.
    #[serde(default)]
    pub since: Option<String>,
    // chat
    #[serde(default)]
    pub contains: Option<String>,
    #[serde(default)]
    pub matches: Option<String>,
    /// Chat: exact number of matching lines (e.g. "shows exactly once").
    #[serde(default)]
    pub count: Option<u32>,
    #[serde(default)]
    pub absent: bool,
    /// Chat: store regex group 1 of the first match into this var. Tool,
    /// server and lua: store the observed value (a baseline a later
    /// clause compares with `value = "${var}"`).
    #[serde(default)]
    pub capture_var: Option<String>,
    // tool / server / lua
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub args: Option<Value>,
    #[serde(default)]
    pub pointer: Option<String>,
    #[serde(default)]
    pub op: Option<Op>,
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default)]
    pub chunk: Option<String>,
    // wait
    #[serde(default)]
    pub lua_condition: Option<String>,
    /// Wait: how long to wait for the condition. Client_event: how long
    /// to wait for `min_rows` events (or, with `max_rows`, for one too
    /// many) before grading; default 5000. Tool and server clauses:
    /// re-evaluate every 250 ms until the clause passes or this time is up
    /// (at most 60000).
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    // timing
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub max_ms: Option<u64>,
    // signoz
    #[serde(default)]
    pub filter: Option<String>,
    #[serde(default)]
    pub min_rows: Option<u64>,
    #[serde(default)]
    pub max_rows: Option<u64>,
    /// SigNoz and packet: a field every matching row must satisfy with
    /// `op`/`value`.
    #[serde(default)]
    pub field: Option<String>,
    /// `op = "approx"`: how far either side of `value` still passes.
    #[serde(default)]
    pub tolerance: Option<f64>,
    // packet
    /// The message name as the tap decodes it (its `msg_name`).
    #[serde(default)]
    pub message: Option<String>,
    /// `to_client` (server sends) or `to_server` (client sends).
    #[serde(default)]
    pub direction: Option<String>,
    /// Only messages for this entity (an outbound row's
    /// `target_entity_id`); a number or a `${var}`.
    #[serde(default)]
    pub entity: Option<Value>,
    // client_event
    /// The telemetry target (`client.ability.sent`); the lab ring stores it
    /// without the `client.` prefix. A glob (`client.ability.*`) is fine.
    #[serde(default)]
    pub event: Option<String>,
    /// Client_event and packet: only events (messages) whose fields equal
    /// these: `{ method = "onEffectResults", ability_id = 597 }`. Client
    /// events glob strings; packet fields compare loosely (numbers
    /// numerically).
    #[serde(default)]
    pub match_fields: Option<serde_json::Map<String, Value>>,
    /// Chat, tool, lua, wait and client_event clauses: read this client
    /// (`p1` default, `p2` on a `players = 2` row).
    #[serde(default)]
    pub client: Option<String>,
    // human
    #[serde(default)]
    pub question: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Something to capture into the row's attachments.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceSpec {
    /// Attachment base name (`final`, `vault-window`).
    pub name: String,
    pub tool: String,
    #[serde(default)]
    pub args: Option<Value>,
    /// After the action with this label; default: after the steps.
    #[serde(default)]
    pub at: Option<String>,
    /// Capture from this client (`p1` default, `p2`).
    #[serde(default)]
    pub client: Option<String>,
}

// lab-golden (GD-01)
/// What may differ between runs that still agree (README D-GD5).
/// `[section.golden]` applies to every row; `[row.golden]` adds to it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldenSpec {
    /// Message-name globs (`*` only) dropped before comparing.
    #[serde(default)]
    pub ignore: Vec<String>,
    /// Globs moved out of the ordered stream into a per-row sorted multiset.
    #[serde(default)]
    pub allow_unordered: Vec<String>,
    /// Globs whose consecutive identical events count once.
    #[serde(default)]
    pub collapse: Vec<String>,
    /// Window-name globs left out of the opened and closed sets.
    #[serde(default)]
    pub ignore_windows: Vec<String>,
    /// Message name -> decoded fields to keep; replaces the built-in entry.
    #[serde(default)]
    pub key_args: std::collections::BTreeMap<String, Vec<String>>,
    /// A tap bound after the row entered the world drops this many ms of
    /// messages after its first one (default 1000).
    #[serde(default)]
    pub late_bind_settle_ms: Option<u64>,
    /// Row only: leave the row out of every fingerprint (cleanup rows).
    #[serde(default)]
    pub skip: bool,
}
// end lab-golden

pub use super::spec_pose::{AimMode, PoseSpec};

// lab-spec-vocab (SV-03)
/// Wait for a server log line (over the server lab MCP) instead of sleeping.
#[derive(Debug, Clone, PartialEq, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServerWait {
    /// A substring of the log message (`server_log_tail` `message`).
    pub log: String,
    /// The tracing target, exact (`aoi.cinematic_hold`).
    #[serde(default)]
    pub target: Option<String>,
    /// Every field must equal (numbers numerically, `${var}`s substituted).
    #[serde(default)]
    pub fields: Option<serde_json::Map<String, Value>>,
    /// Default 30000; at most 120000.
    #[serde(default = "default_server_wait_ms")]
    pub timeout_ms: u64,
}

fn default_server_wait_ms() -> u64 {
    30000
}
// end lab-spec-vocab

/// Parse and validate one section file.
pub fn parse(text: &str) -> Result<SectionSpec, String> {
    let mut spec: SectionSpec = toml::from_str(text).map_err(|e| format!("spec TOML: {e}"))?;
    // `@capability` names become tool names first (super::tools).
    super::tools::resolve_section(&mut spec)?;
    validate(&spec)?;
    for row in &spec.rows {
        for action in row.setup.iter().chain(&row.steps).chain(&row.teardown) {
            action
                .route_args()
                .map_err(|e| format!("row {}: {e}", row.id))?;
            for fallback in &action.fallback {
                fallback
                    .route_args()
                    .map_err(|e| format!("row {} fallback: {e}", row.id))?;
            }
        }
    }
    Ok(spec)
}

#[cfg(test)]
#[path = "spec_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "spec_fixture_tests.rs"]
mod fixture_tests;
