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
//! `docs/guides/automated-uat.md`; this module is the schema.

use std::collections::HashSet;

use serde::Deserialize;
use serde_json::Value;

use super::tier::Tier;

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
    #[serde(default)]
    pub fresh: Option<FreshCharacter>,
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
    /// Players the row needs. Above 1 the row is BLOCKED until a second
    /// lab instance or a puppet can join it (matrix X1).
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
}

impl ActionSpec {
    /// Which of the four kinds this is, or why it is malformed.
    pub fn kind(&self) -> Result<ActionKind, String> {
        let set = [
            self.tool.is_some(),
            self.chat.is_some(),
            self.wait_ms.is_some(),
            self.capture.is_some(),
        ];
        match set.iter().filter(|b| **b).count() {
            1 => {}
            0 => return Err("an action needs one of tool, chat, wait_ms, capture".into()),
            _ => return Err("an action has more than one of tool, chat, wait_ms, capture".into()),
        }
        Ok(if self.tool.is_some() {
            ActionKind::Tool
        } else if self.chat.is_some() {
            ActionKind::Chat
        } else if self.wait_ms.is_some() {
            ActionKind::Wait
        } else {
            ActionKind::Capture
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    Tool,
    Chat,
    Wait,
    Capture,
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
    Server,
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
    /// Chat: only lines after the action with this label.
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
    /// Chat: store regex group 1 of the first match into this var.
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
}

/// Parse and validate one section file.
pub fn parse(text: &str) -> Result<SectionSpec, String> {
    let mut spec: SectionSpec = toml::from_str(text).map_err(|e| format!("spec TOML: {e}"))?;
    // `@capability` names become tool names first (super::tools).
    super::tools::resolve_section(&mut spec)?;
    validate(&spec)?;
    Ok(spec)
}

/// Structural checks the type system cannot express. Every problem is
/// reported with its row id so an author can find it.
pub fn validate(spec: &SectionSpec) -> Result<(), String> {
    let mut errs = Vec::new();
    if spec.schema != SCHEMA {
        errs.push(format!(
            "schema {} (this runner reads {SCHEMA})",
            spec.schema
        ));
    }
    if spec.section.character == "fresh" && spec.section.fresh.is_none() {
        errs.push("character = \"fresh\" needs a [section.fresh] table".into());
    }
    if !matches!(spec.section.character.as_str(), "lab" | "fresh") {
        errs.push(format!(
            "character {:?}: lab or fresh",
            spec.section.character
        ));
    }
    if !matches!(spec.section.account.as_str(), "gm" | "non-gm") {
        errs.push(format!("account {:?}: gm or non-gm", spec.section.account));
    }
    let mut ids = HashSet::new();
    for row in &spec.rows {
        let r = &row.id;
        if !ids.insert(r.clone()) {
            errs.push(format!("{r}: duplicate row id"));
        }
        if !matches!(
            row.state.as_str(),
            "in_world" | "char_select" | "any" | "client_stopped"
        ) {
            errs.push(format!("{r}: state {:?}", row.state));
        }
        let mut labels = HashSet::new();
        for a in row.setup.iter().chain(&row.steps).chain(&row.teardown) {
            check_action(r, a, &mut errs);
            if let Some(l) = &a.label {
                if !labels.insert(l.clone()) {
                    errs.push(format!("{r}: duplicate action label {l:?}"));
                }
            }
        }
        if row.steps.is_empty() && row.blocked.is_none() {
            errs.push(format!("{r}: no steps (add `blocked` if it cannot run)"));
        }
        if row.expect.is_empty() && row.blocked.is_none() {
            errs.push(format!("{r}: no expected clauses"));
        }
        let mut cids = HashSet::new();
        for c in &row.expect {
            if !cids.insert(c.id.clone()) {
                errs.push(format!("{r}/{}: duplicate clause id", c.id));
            }
            for l in [&c.at, &c.since, &c.action].into_iter().flatten() {
                if !labels.contains(l) {
                    errs.push(format!("{r}/{}: no action labelled {l:?}", c.id));
                }
            }
            if let Err(e) = check_clause(c) {
                errs.push(format!("{r}/{}: {e}", c.id));
            }
        }
        for e in &row.evidence {
            if let Some(l) = &e.at {
                if !labels.contains(l) {
                    errs.push(format!("{r}/evidence {}: no action labelled {l:?}", e.name));
                }
            }
        }
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs.join("; "))
    }
}

fn check_action(row: &str, a: &ActionSpec, errs: &mut Vec<String>) {
    match a.kind() {
        Err(e) => errs.push(format!("{row}: {e}")),
        Ok(ActionKind::Capture) => {
            if a.regex.is_none() || a.var.is_none() {
                errs.push(format!("{row}: capture needs regex and var"));
            }
            if a.capture.as_deref() != Some("chat") {
                errs.push(format!("{row}: capture source must be \"chat\""));
            }
        }
        Ok(_) => {}
    }
    if let Some(re) = &a.regex {
        if let Err(e) = regex::Regex::new(&without_vars(re)) {
            errs.push(format!("{row}: bad regex {re:?}: {e}"));
        }
    }
    for f in &a.fallback {
        if f.tool.is_none() && f.chat.is_none() {
            errs.push(format!("{row}: a fallback must be a tool or chat action"));
        }
        check_action(row, f, errs);
    }
}

/// A pattern with its `${var}` placeholders replaced by a literal, so it
/// can be compiled before the variables are known.
pub fn without_vars(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find("${") {
        out.push_str(&rest[..i]);
        match rest[i..].find('}') {
            Some(j) => {
                out.push('X');
                rest = &rest[i + j + 1..];
            }
            None => {
                out.push_str(&rest[i..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

fn check_clause(c: &ExpectSpec) -> Result<(), String> {
    let need = |ok: bool, what: &str| if ok { Ok(()) } else { Err(what.to_string()) };
    match c.source {
        Source::Chat => need(
            c.contains.is_some() || c.matches.is_some(),
            "a chat clause needs contains or matches",
        )?,
        Source::Tool | Source::Server => need(c.tool.is_some(), "needs tool")?,
        Source::Lua => need(c.chunk.is_some(), "a lua clause needs chunk")?,
        Source::Wait => need(
            c.lua_condition.is_some(),
            "a wait clause needs lua_condition",
        )?,
        Source::Timing => need(
            c.action.is_some() && c.max_ms.is_some(),
            "a timing clause needs action and max_ms",
        )?,
        Source::Signoz => need(c.filter.is_some(), "a signoz clause needs filter")?,
        Source::Packet => {
            need(c.message.is_some(), "a packet clause needs message")?;
            need(
                matches!(c.direction.as_deref(), Some("to_client" | "to_server")),
                "a packet clause needs direction = \"to_client\" or \"to_server\"",
            )?;
            // One tap covers the row and is read once, at teardown.
            need(
                c.at.is_none() && c.since.is_none(),
                "a packet clause is graded over the whole row: no at or since",
            )?;
            // Without a field, op/value would be silently ignored and any
            // matching message would pass.
            need(
                c.field.is_some() || (c.op.is_none() && c.value.is_none() && c.tolerance.is_none()),
                "a packet clause's op, value and tolerance need a field",
            )?;
        }
        Source::Human => need(c.question.is_some(), "a human clause needs question")?,
    }
    if matches!(c.source, Source::Tool | Source::Server | Source::Lua)
        && c.op.is_none()
        && c.value.is_none()
    {
        return Err("needs op and/or value".into());
    }
    if c.field.is_some() && c.op.is_none() {
        return Err("field needs op".into());
    }
    if c.op == Some(Op::Approx)
        && (c.tolerance.is_none_or(|t| !t.is_finite() || t < 0.0)
            || !c.value.as_ref().is_some_and(Value::is_number))
    {
        return Err("op approx needs a numeric value and a finite tolerance >= 0".into());
    }
    if let Some(re) = &c.matches {
        regex::Regex::new(&without_vars(re)).map_err(|e| format!("bad regex {re:?}: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINI: &str = r#"
schema = 1
[section]
id = "gm-parity"
system = "GM console command parity"
guide = "unified-uat.md#gm-console-command-parity"
ledger = "legacy-command-parity/README.md"

[[row]]
id = "M1-1"
title = "help answers"
expected = "Each answers in chat."
step = [{ chat = ".help", label = "help" }]

[[row.expect]]
id = "help"
text = ".help lists commands"
source = "chat"
since = "help"
contains = "help"
"#;

    #[test]
    fn a_minimal_section_parses_with_defaults() {
        let s = parse(MINI).unwrap();
        let row = &s.rows[0];
        assert_eq!(row.required_native, Tier::N1);
        assert_eq!(row.state, "in_world");
        assert_eq!(row.players, 1);
        assert_eq!(s.section.character, "lab");
        assert_eq!(row.steps[0].kind().unwrap(), ActionKind::Chat);
    }

    #[test]
    fn a_dangling_label_and_a_bare_clause_are_rejected() {
        let bad = MINI.replace("since = \"help\"", "since = \"nope\"");
        let e = parse(&bad).unwrap_err();
        assert!(e.contains("M1-1/help: no action labelled \"nope\""), "{e}");
        let bad = MINI.replace("contains = \"help\"", "");
        assert!(parse(&bad).unwrap_err().contains("contains or matches"));
    }

    #[test]
    fn an_action_with_two_kinds_is_rejected() {
        let bad = MINI.replace(
            "{ chat = \".help\", label = \"help\" }",
            "{ chat = \".help\", wait_ms = 5, label = \"help\" }",
        );
        assert!(parse(&bad).unwrap_err().contains("more than one"));
    }

    #[test]
    fn unknown_fields_are_errors_not_silently_ignored() {
        let bad = MINI.replace("title = \"help answers\"", "title = \"x\"\ntypo = 1");
        assert!(parse(&bad).is_err());
    }

    const PACKET: &str = r#"
[[row.expect]]
id = "timer"
text = "a 15 s timer"
source = "packet"
message = "onTimerUpdate"
direction = "to_client"
entity = "${player_entity_id}"
field = "complete_in_s"
op = "approx"
value = 15
tolerance = 1
"#;

    #[test]
    fn a_packet_clause_parses_and_its_rules_hold() {
        let s = parse(&format!("{MINI}{PACKET}")).unwrap();
        let c = &s.rows[0].expect[1];
        assert_eq!(c.source, Source::Packet);
        assert_eq!(c.message.as_deref(), Some("onTimerUpdate"));
        assert_eq!(c.op, Some(Op::Approx));
        assert_eq!(c.tolerance, Some(1.0));

        let bad = format!("{MINI}{}", PACKET.replace("to_client", "outbound"));
        assert!(parse(&bad).unwrap_err().contains("to_client"));
        let bad = format!(
            "{MINI}{}",
            PACKET.replace("message = \"onTimerUpdate\"\n", "")
        );
        assert!(parse(&bad).unwrap_err().contains("needs message"));
        // One tap per row, read at teardown: a step-anchored clause is wrong.
        let bad = format!(
            "{MINI}{}",
            PACKET.replace("source = \"packet\"", "source = \"packet\"\nat = \"help\"")
        );
        assert!(parse(&bad).unwrap_err().contains("no at or since"));
        let bad = format!("{MINI}{}", PACKET.replace("tolerance = 1\n", ""));
        assert!(parse(&bad).unwrap_err().contains("tolerance"));
        // Non-finite and negative tolerances would pass anything or nothing.
        for t in ["inf", "+inf", "nan", "-1"] {
            let bad = format!(
                "{MINI}{}",
                PACKET.replace("tolerance = 1", &format!("tolerance = {t}"))
            );
            assert!(parse(&bad).unwrap_err().contains("finite tolerance"), "{t}");
        }
        // op/value without field would pass on any matching message.
        let bad = format!(
            "{MINI}{}",
            PACKET.replace("field = \"complete_in_s\"\n", "")
        );
        assert!(parse(&bad).unwrap_err().contains("need a field"));
    }
}
