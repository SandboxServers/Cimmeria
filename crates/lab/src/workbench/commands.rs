//! Typed GM workbench action contract and authority checks.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Version shared with the dot-console registry's initial workbench metadata.
pub const COMMAND_METADATA_VERSION: u32 = 1;

/// The two command transports have different parsers and evidence requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandChannel {
    Dot,
    Slash,
}

/// Whether a command may be prepared as a typed workbench action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Runnable,
    Unsupported,
    Hidden,
}

/// Supported argument value types; names alone do not prove a parser contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArgKind {
    Text,
    Integer,
    Float,
    Boolean,
    Enum,
    Entity,
    Template,
    Mission,
    Item,
    World,
    Position,
}

/// Bounds on a numeric argument.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArgRange {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// Source-backed typed argument specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArgDescriptor {
    pub name: String,
    pub kind: ArgKind,
    pub required: bool,
    pub default: Option<Value>,
    pub range: Option<ArgRange>,
    pub choices: Vec<Value>,
    pub help: String,
    pub sensitive: bool,
}

/// Stable target types shared by dot and slash descriptors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    None,
    /// Any live entity kind, including props; the caller's space is rechecked by the handler.
    AnyEntity,
    Player,
    Mob,
    MobOrId,
    Being,
    Spawnable,
}

/// How a descriptor obtains its target at execution time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionRule {
    None,
    Required,
    SelectedOrArgument,
    ArgumentRequired,
    ArgumentRequiredOther,
    ArgumentRequiredSelfZero,
}

/// Target type and selection rule, checked against a fresh actor snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetPolicy {
    pub kind: TargetKind,
    pub selection_rule: SelectionRule,
    /// Name of the source-backed entity-id argument when the rule uses one.
    pub argument_name: Option<String>,
}

/// Persistence and reversibility category of a command's effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactKind {
    ReadOnly,
    Transient,
    Persistent,
    Authoring,
    Irreversible,
}

/// Human impact summary and explicit prerequisites.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Impact {
    pub kind: ImpactKind,
    pub summary: String,
    pub preconditions: Vec<String>,
}

/// Readback plan; an empty plan cannot produce a verified receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyRecipe {
    pub read_tools: Vec<String>,
    pub comparison_fields: Vec<String>,
}

/// Documented recovery, never inferred from an impact label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Compensation {
    None,
    Inverse(String),
    Manual(String),
}

/// Stable metadata for one dot or native slash command. Pending evidence is
/// represented by absent fields and can never pass the execution gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandDescriptor {
    pub id: String,
    pub channel: CommandChannel,
    pub category: String,
    pub aliases: Vec<String>,
    pub summary: String,
    pub example: Option<String>,
    pub availability: Availability,
    pub disposition_reason: Option<String>,
    pub manual_pending: bool,
    pub args: Vec<ArgDescriptor>,
    pub target: TargetPolicy,
    pub impact: Option<Impact>,
    pub verify: Option<VerifyRecipe>,
    pub compensation: Option<Compensation>,
    pub metadata_version: u32,
}

/// A user-requested action. The dispatcher must supply a fresh actor snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandAction {
    pub descriptor_id: String,
    pub instance: String,
    pub actor_entity_id: u32,
    pub selected_entity_id: Option<u32>,
    pub typed_args: BTreeMap<String, Value>,
    pub idempotency_key: String,
}

/// Current server-side identity and selection facts, never supplied by the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorSnapshot {
    pub instance: String,
    pub entity_id: u32,
    pub is_gm: bool,
    pub selected_entity_id: Option<u32>,
    pub selected_target_kind: Option<TargetKind>,
    pub argument_target_entity_id: Option<u32>,
    pub argument_target_kind: Option<TargetKind>,
}

/// Action whose identity, target and typed values passed the current gate.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedAction {
    pub action: CommandAction,
    pub resolved_args: BTreeMap<String, Value>,
}

/// Stable refusal reasons returned before dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    WrongDescriptor,
    Unsupported,
    StaleInstance,
    StaleActor,
    NotGm,
    StaleTarget,
    WrongTargetKind,
    MissingArgument,
    UnknownArgument,
    MalformedArgument,
    OutOfRange,
    InvalidChoice,
    MissingIdempotencyKey,
}

/// Independent submission, server acceptance and readback facts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionReceipt {
    pub descriptor_id: String,
    pub submitted: bool,
    pub server_accepted: Option<bool>,
    pub verified: Option<bool>,
    pub feedback: Vec<String>,
    pub before_reads: Vec<Value>,
    pub after_reads: Vec<Value>,
    pub audit_id: Option<String>,
    pub evidence_paths: Vec<String>,
}

/// Validate a typed action against the descriptor and fresh server identity.
/// The dispatching cell or lab endpoint must repeat this check at execution.
pub fn validate_command(
    action: &CommandAction,
    descriptor: &CommandDescriptor,
    actor: &ActorSnapshot,
) -> Result<ValidatedAction, Refusal> {
    if action.descriptor_id != descriptor.id {
        return Err(Refusal::WrongDescriptor);
    }
    if !matches!(
        (descriptor.channel, descriptor.id.chars().next()),
        (CommandChannel::Dot, Some('.')) | (CommandChannel::Slash, Some('/'))
    ) {
        return Err(Refusal::WrongDescriptor);
    }
    if descriptor.availability != Availability::Runnable || descriptor.manual_pending {
        return Err(Refusal::Unsupported);
    }
    if descriptor.impact.is_none()
        || descriptor.verify.is_none()
        || descriptor.compensation.is_none()
    {
        return Err(Refusal::Unsupported);
    }
    if action.instance != actor.instance {
        return Err(Refusal::StaleInstance);
    }
    if action.actor_entity_id != actor.entity_id {
        return Err(Refusal::StaleActor);
    }
    if !actor.is_gm {
        return Err(Refusal::NotGm);
    }
    if action.idempotency_key.is_empty() || action.idempotency_key.len() > 128 {
        return Err(Refusal::MissingIdempotencyKey);
    }
    if action.selected_entity_id != actor.selected_entity_id {
        return Err(Refusal::StaleTarget);
    }
    let target_kind = match descriptor.target.selection_rule {
        SelectionRule::None => None,
        SelectionRule::Required => {
            actor.selected_entity_id.ok_or(Refusal::StaleTarget)?;
            Some(actor.selected_target_kind.ok_or(Refusal::StaleTarget)?)
        }
        SelectionRule::SelectedOrArgument
            if descriptor
                .target
                .argument_name
                .as_ref()
                .is_some_and(|name| action.typed_args.contains_key(name)) =>
        {
            let requested = descriptor
                .target
                .argument_name
                .as_ref()
                .and_then(|name| action.typed_args.get(name))
                .and_then(Value::as_u64);
            if requested.is_none() || requested != actor.argument_target_entity_id.map(u64::from) {
                return Err(Refusal::StaleTarget);
            }
            Some(actor.argument_target_kind.ok_or(Refusal::StaleTarget)?)
        }
        SelectionRule::SelectedOrArgument => {
            actor.selected_entity_id.ok_or(Refusal::StaleTarget)?;
            Some(actor.selected_target_kind.ok_or(Refusal::StaleTarget)?)
        }
        SelectionRule::ArgumentRequired
        | SelectionRule::ArgumentRequiredOther
        | SelectionRule::ArgumentRequiredSelfZero => {
            let name = descriptor
                .target
                .argument_name
                .as_ref()
                .ok_or(Refusal::WrongDescriptor)?;
            let value = action
                .typed_args
                .get(name)
                .ok_or(Refusal::MissingArgument)?;
            let requested = value.as_u64().or_else(|| {
                value.as_str().and_then(|text| {
                    (!text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
                        .then(|| text.parse::<u64>().ok())
                        .flatten()
                })
            });
            let requested = requested.ok_or(Refusal::MalformedArgument)?;
            let resolved = if requested == 0
                && descriptor.target.selection_rule == SelectionRule::ArgumentRequiredSelfZero
            {
                u64::from(actor.entity_id)
            } else {
                requested
            };
            if resolved == 0 || Some(resolved) != actor.argument_target_entity_id.map(u64::from) {
                return Err(Refusal::StaleTarget);
            }
            if descriptor.target.selection_rule == SelectionRule::ArgumentRequiredOther
                && resolved == u64::from(actor.entity_id)
            {
                return Err(Refusal::StaleTarget);
            }
            Some(actor.argument_target_kind.ok_or(Refusal::StaleTarget)?)
        }
    };
    if target_kind.is_some_and(|kind| !target_matches(descriptor.target.kind, kind)) {
        return Err(Refusal::WrongTargetKind);
    }
    let mut resolved_args = BTreeMap::new();
    for arg in &descriptor.args {
        let value = action.typed_args.get(&arg.name).or(arg.default.as_ref());
        let Some(value) = value else {
            if arg.required {
                return Err(Refusal::MissingArgument);
            }
            continue;
        };
        validate_value(value, arg)?;
        validate_source_text(descriptor, arg, value)?;
        resolved_args.insert(arg.name.clone(), value.clone());
    }
    if action
        .typed_args
        .keys()
        .any(|name| !descriptor.args.iter().any(|arg| arg.name == *name))
    {
        return Err(Refusal::UnknownArgument);
    }
    Ok(ValidatedAction {
        action: action.clone(),
        resolved_args,
    })
}

fn target_matches(required: TargetKind, actual: TargetKind) -> bool {
    match required {
        TargetKind::None | TargetKind::Spawnable => true,
        TargetKind::AnyEntity => matches!(
            actual,
            TargetKind::Player | TargetKind::Mob | TargetKind::Being | TargetKind::Spawnable
        ),
        TargetKind::Being => matches!(
            actual,
            TargetKind::Player | TargetKind::Mob | TargetKind::Being
        ),
        TargetKind::MobOrId | TargetKind::Mob => actual == TargetKind::Mob,
        TargetKind::Player => actual == TargetKind::Player,
    }
}

fn validate_source_text(
    descriptor: &CommandDescriptor,
    arg: &ArgDescriptor,
    value: &Value,
) -> Result<(), Refusal> {
    match (
        descriptor.channel,
        descriptor.id.as_str(),
        arg.name.as_str(),
    ) {
        (CommandChannel::Slash, "/gmsettarget", "NameOrID") => {
            let text = value.as_str().ok_or(Refusal::MalformedArgument)?.trim();
            let id = text
                .parse::<i64>()
                .map_err(|_| Refusal::MalformedArgument)?;
            if !(0..=i64::from(i32::MAX)).contains(&id) {
                return Err(Refusal::OutOfRange);
            }
        }
        (CommandChannel::Slash, "/gmgotolocation", "WorldName")
            if value.as_str().is_none_or(|text| text.trim().is_empty()) =>
        {
            return Err(Refusal::MalformedArgument);
        }
        _ => {}
    }
    Ok(())
}

fn validate_value(value: &Value, arg: &ArgDescriptor) -> Result<(), Refusal> {
    // ArgRange stores f64 bounds. Larger integers cannot be compared to those
    // bounds without rounding a distinct JSON integer onto a permitted value.
    const MAX_EXACT_F64_INTEGER: i128 = 1_i128 << 53;
    let number = match arg.kind {
        ArgKind::Text | ArgKind::Enum | ArgKind::World => {
            if !value.is_string() {
                return Err(Refusal::MalformedArgument);
            }
            None
        }
        ArgKind::Boolean => {
            if !value.is_boolean() {
                return Err(Refusal::MalformedArgument);
            }
            None
        }
        ArgKind::Integer => {
            let integer = value
                .as_i64()
                .map(i128::from)
                .or_else(|| value.as_u64().map(i128::from));
            if integer
                .is_some_and(|v| !(-MAX_EXACT_F64_INTEGER..=MAX_EXACT_F64_INTEGER).contains(&v))
            {
                return Err(Refusal::OutOfRange);
            }
            integer.map(|v| v as f64)
        }
        ArgKind::Entity | ArgKind::Template | ArgKind::Mission | ArgKind::Item => value
            .as_u64()
            .filter(|v| *v > 0 && *v <= u32::MAX as u64)
            .map(|v| v as f64),
        ArgKind::Float => value.as_f64().filter(|v| v.is_finite()),
        ArgKind::Position => {
            let Some(coords) = value.as_array() else {
                return Err(Refusal::MalformedArgument);
            };
            if coords.len() != 3
                || coords
                    .iter()
                    .any(|v| !v.as_f64().is_some_and(f64::is_finite))
            {
                return Err(Refusal::MalformedArgument);
            }
            None
        }
    };
    if matches!(
        arg.kind,
        ArgKind::Integer
            | ArgKind::Entity
            | ArgKind::Template
            | ArgKind::Mission
            | ArgKind::Item
            | ArgKind::Float
    ) && number.is_none()
    {
        return Err(Refusal::MalformedArgument);
    }
    if let (Some(number), Some(range)) = (number, &arg.range) {
        if range.min.is_some_and(|min| number < min) || range.max.is_some_and(|max| number > max) {
            return Err(Refusal::OutOfRange);
        }
    }
    if !arg.choices.is_empty() && !arg.choices.contains(value) {
        return Err(Refusal::InvalidChoice);
    }
    Ok(())
}
