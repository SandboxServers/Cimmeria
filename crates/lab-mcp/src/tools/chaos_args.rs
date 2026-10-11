//! The chaos tools' arguments and their parse (lab-chaos CH-05).
//!
//! Each tool takes the raw JSON ([`SetCall`], [`ClearCall`]) and parses it
//! here, so a malformed call (a negative count, a fraction where an integer
//! goes, an unknown field) is a `bad chaos knob:` refusal with an audit row,
//! not an rmcp protocol error that never reaches the tool. Everything a
//! request names must be used: an unknown field, an empty `net` profile, a
//! flush knob the chosen mode ignores and a seed nothing reads are refused,
//! so a lab run never believes it set chaos it did not.

use std::borrow::Cow;

use rmcp::schemars::{self, JsonSchema};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

use cimmeria_services::base::lab_chaos::FlushChaos;
use cimmeria_services::base::lab_chaos::NetChaos;

/// The longest `label` a set takes. The label is logged on every set WARN;
/// a CH-01 header is a few dozen characters.
pub(super) const LABEL_MAX: usize = 120;

/// The longest `account` a set or clear takes: `account.account_name` is
/// `character varying(32)` (`db/sgw/Accounts/Tables/account.sql`), so a
/// longer name matches no account and is refused before the lookup.
pub(super) const ACCOUNT_NAME_MAX: usize = 32;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct SetArgs {
    /// The player entity whose session gets the chaos. Give `entity_id` or
    /// `account`, not both.
    #[serde(default)]
    pub(super) entity_id: Option<u32>,
    /// An account name, spelled as at login: arm the chaos for that
    /// account's next world entry instead (one-shot), so it reaches that
    /// login's first flush. Give `entity_id` or `account`, not both.
    #[serde(default)]
    pub(super) account: Option<String>,
    /// The net profile's knobs. Give `net`, `flush` or both.
    #[serde(default)]
    pub(super) net: Option<NetArgs>,
    /// The flush profile's knobs. Give `net`, `flush` or both.
    #[serde(default)]
    pub(super) flush: Option<FlushArgs>,
    /// Seeds the random parts of the profiles (net, flush `reorder`, shift
    /// `sweep`), so a run replays. Default 0; refused when nothing uses it.
    #[serde(default)]
    pub(super) seed: Option<u64>,
    /// Seconds until the chaos expires. Default 300; 0 or over 1800 is refused.
    #[serde(default)]
    pub(super) ttl_s: Option<u32>,
    /// The chaos spec text the server logs as `level`, such as
    /// `flush:pack seed=7`. Built from the profiles when omitted.
    #[serde(default)]
    pub(super) label: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct NetArgs {
    /// Added delay on each server send, in ms (at most 5000).
    #[serde(default)]
    pub(super) latency: Option<u32>,
    /// Random spread on top of the latency, in ms (at most 5000).
    #[serde(default)]
    pub(super) jitter: Option<u32>,
    /// Percent of packets dropped, at most one decimal (0 to 100).
    #[serde(default)]
    pub(super) loss: Option<f64>,
    /// Percent of packets sent twice, at most one decimal (0 to 100).
    #[serde(default)]
    pub(super) dup: Option<f64>,
    /// Reorder buffer size in packets (at most 1000); 0 keeps order.
    #[serde(default)]
    pub(super) reorder: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct FlushArgs {
    /// One of `delay`, `reorder`, `pack`, `shift`.
    pub(super) mode: String,
    /// For `delay` only: the hold in ms (at most 60000).
    #[serde(default)]
    pub(super) delay_ms: Option<u32>,
    /// For `pack` only: the records in the single bundle (1 to 34).
    #[serde(default)]
    pub(super) records: Option<u16>,
    /// For `shift` only: the filler's length in bytes (0 or 2 to 38), or
    /// `"sweep"`.
    #[serde(default)]
    pub(super) shift: Option<ShiftArg>,
}

/// A `shift` knob: a byte count, or the word `sweep` for a seeded length.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(untagged)]
pub(super) enum ShiftArg {
    Bytes(u16),
    Word(String),
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ClearArgs {
    /// Clear only the chaos set for this player entity. Omit both fields to
    /// clear every session and every arm.
    #[serde(default)]
    pub(super) entity_id: Option<u32>,
    /// Clear only the chaos armed for this account's next world entry (an
    /// account name, as at login). Not with `entity_id`.
    #[serde(default)]
    pub(super) account: Option<String>,
}

/// Raw JSON whose schema is `T`'s: rmcp deserializes any call into it, and
/// [`parse`] does the typed parse inside the tool.
macro_rules! raw_call {
    ($name:ident, $args:ty) => {
        #[derive(Debug)]
        pub(super) struct $name(pub(super) Value);

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                Value::deserialize(d).map(Self)
            }
        }

        impl JsonSchema for $name {
            fn schema_name() -> Cow<'static, str> {
                <$args>::schema_name()
            }

            fn schema_id() -> Cow<'static, str> {
                <$args>::schema_id()
            }

            fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
                <$args>::json_schema(generator)
            }
        }
    };
}

raw_call!(SetCall, SetArgs);
raw_call!(ClearCall, ClearArgs);

/// The typed parse of a raw call. A bad shape is a refusal naming the field
/// (`net.loss: invalid type: ...`); absent arguments read as `{}`.
pub(super) fn parse<T: DeserializeOwned>(raw: Value) -> Result<T, String> {
    let raw = match raw {
        Value::Null => Value::Object(Map::new()),
        other => other,
    };
    serde_path_to_error::deserialize(raw).map_err(|e| {
        let path = e.path().to_string();
        let inner = e.into_inner();
        // The root prints as "."; an error there names its field itself.
        if path == "." {
            knob_refusal(inner)
        } else {
            knob_refusal(format!("{path}: {inner}"))
        }
    })
}

/// The arguments as the audit row records them. A `label` or an `account`
/// the tool would refuse (too long, or with a control character) is
/// replaced by its length, so the raw text never reaches the log.
pub(super) fn audit_args(raw: &Value) -> Value {
    let mut audit = raw.clone();
    if let Some(Value::String(label)) = audit.get("label") {
        if check_label(label).is_err() {
            let n = label.chars().count();
            audit["label"] = Value::String(format!("<label: {n} chars, refused>"));
        }
    }
    if let Some(Value::String(account)) = audit.get("account") {
        if !account.is_empty() && check_account(account).is_err() {
            let n = account.chars().count();
            audit["account"] = Value::String(format!("<account: {n} chars, refused>"));
        }
    }
    audit
}

/// Whom a set or a clear names: a player entity in the world, or an account
/// whose next world entry takes the chaos (D-CH10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Target {
    Entity(u32),
    Account(String),
}

/// A set names exactly one of `entity_id` and `account`.
pub(super) fn set_target(args: &SetArgs) -> Result<Target, String> {
    match target(args.entity_id, args.account.as_deref())? {
        Some(target) => Ok(target),
        None => Err(knob_refusal("set needs entity_id or account")),
    }
}

/// A clear names at most one of `entity_id` and `account`; `None` clears
/// everything.
pub(super) fn clear_target(args: &ClearArgs) -> Result<Option<Target>, String> {
    target(args.entity_id, args.account.as_deref())
}

fn target(entity_id: Option<u32>, account: Option<&str>) -> Result<Option<Target>, String> {
    match (entity_id, account) {
        (Some(_), Some(_)) => Err(knob_refusal("give entity_id or account, not both")),
        (Some(id), None) => Ok(Some(Target::Entity(id))),
        (None, Some("")) => Err(knob_refusal("account is empty")),
        (None, Some(name)) => {
            check_account(name)?;
            Ok(Some(Target::Account(name.to_string())))
        }
        (None, None) => Ok(None),
    }
}

/// An account name no account can have is refused before the lookup: over
/// the column's [`ACCOUNT_NAME_MAX`] characters, or with a control
/// character. The refusal does not echo the name.
fn check_account(name: &str) -> Result<(), String> {
    if name.chars().count() > ACCOUNT_NAME_MAX {
        return Err(knob_refusal(format!(
            "account is over {ACCOUNT_NAME_MAX} characters"
        )));
    }
    if name.chars().any(char::is_control) {
        return Err(knob_refusal("account has a control character"));
    }
    Ok(())
}

/// A label is logged as text on the set WARN: short, one line, printable.
pub(super) fn check_label(label: &str) -> Result<(), String> {
    if label.chars().count() > LABEL_MAX {
        return Err(knob_refusal(format!(
            "label is over {LABEL_MAX} characters"
        )));
    }
    if label.chars().any(char::is_control) {
        return Err(knob_refusal("label has a control character"));
    }
    Ok(())
}

/// The net profile. One that sets no knob would arm nothing, so it is refused.
pub(super) fn net_from(args: &NetArgs, seed: Option<u64>) -> Result<NetChaos, String> {
    let net = NetChaos {
        latency_ms: args.latency.unwrap_or(0),
        jitter_ms: args.jitter.unwrap_or(0),
        loss_per_thousand: per_thousand(args.loss, "loss")?,
        dup_per_thousand: per_thousand(args.dup, "dup")?,
        reorder: args.reorder.unwrap_or(0),
        seed: seed.unwrap_or(0),
    };
    let knobs = [
        net.latency_ms,
        net.jitter_ms,
        net.loss_per_thousand,
        net.dup_per_thousand,
        net.reorder,
    ];
    if knobs.iter().all(|&k| k == 0) {
        return Err(knob_refusal("net sets no knob"));
    }
    Ok(net)
}

/// A percent with at most one decimal, as tenths of a percent: 1.2 is 12 per
/// thousand. A finer value (0.05) is refused, not rounded, and so is a
/// negative one (the cast to the registry's unsigned knob would read it as
/// 0). A value too large for `u32` saturates and the registry refuses it as
/// over 1000.
fn per_thousand(percent: Option<f64>, knob: &str) -> Result<u32, String> {
    let percent = percent.unwrap_or(0.0);
    if percent.is_nan() || percent < 0.0 {
        return Err(knob_refusal(format!("net {knob} is negative")));
    }
    let tenths = percent * 10.0;
    if (tenths - tenths.round()).abs() > 1e-9 {
        return Err(knob_refusal(format!(
            "net {knob} {percent} has more than one decimal"
        )));
    }
    Ok(tenths.round() as u32)
}

/// The flush profile. A knob the mode does not read is refused.
pub(super) fn flush_from(args: &FlushArgs, seed: Option<u64>) -> Result<FlushChaos, String> {
    let seed = seed.unwrap_or(0);
    let mode = args.mode.as_str();
    let unused = [
        ("delay_ms", args.delay_ms.is_some() && mode != "delay"),
        ("records", args.records.is_some() && mode != "pack"),
        ("shift", args.shift.is_some() && mode != "shift"),
    ];
    // An unknown mode is refused by name below, not for its knobs.
    let known = matches!(mode, "delay" | "reorder" | "pack" | "shift");
    if let Some((knob, _)) = unused.iter().find(|(_, set)| known && *set) {
        return Err(knob_refusal(format!("flush {mode} does not use {knob}")));
    }
    match mode {
        "delay" => args
            .delay_ms
            .map(|ms| FlushChaos::Delay { ms })
            .ok_or_else(|| knob_refusal("flush delay needs delay_ms")),
        "reorder" => Ok(FlushChaos::Reorder { seed }),
        "pack" => args
            .records
            .map(|records| FlushChaos::Pack { records })
            .ok_or_else(|| knob_refusal("flush pack needs records")),
        "shift" => match &args.shift {
            Some(ShiftArg::Bytes(len)) => Ok(FlushChaos::Shift { len: *len }),
            Some(ShiftArg::Word(word)) if word == "sweep" => Ok(FlushChaos::ShiftSweep { seed }),
            _ => Err(knob_refusal("flush shift needs a byte count or \"sweep\"")),
        },
        other => Err(knob_refusal(format!("unknown flush mode {other:?}"))),
    }
}

/// A named seed must seed something: the net profile, or a flush `reorder`
/// or shift `sweep`. Otherwise the run would not replay the way its label
/// says.
pub(super) fn check_seed_used(
    seed: Option<u64>,
    net: Option<&NetChaos>,
    flush: Option<&FlushChaos>,
) -> Result<(), String> {
    let seeded_flush = matches!(
        flush,
        Some(FlushChaos::Reorder { .. } | FlushChaos::ShiftSweep { .. })
    );
    if seed.is_some() && net.is_none() && !seeded_flush {
        return Err(knob_refusal("seed is not used by this profile"));
    }
    Ok(())
}

/// The label logged as `level` when the request gives none, in the CH-01
/// header's shape: `net seed=7`, `flush:pack`, `flush:reorder seed=7`.
pub(super) fn built_label(net: Option<&NetChaos>, flush: Option<&FlushChaos>) -> String {
    let mut parts = Vec::new();
    if let Some(net) = net {
        parts.push(format!("net seed={}", net.seed));
    }
    if let Some(flush) = flush {
        parts.push(match flush {
            FlushChaos::Delay { .. } => "flush:delay".to_string(),
            FlushChaos::Reorder { seed } => format!("flush:reorder seed={seed}"),
            FlushChaos::Pack { .. } => "flush:pack".to_string(),
            FlushChaos::Shift { .. } => "flush:shift".to_string(),
            FlushChaos::ShiftSweep { seed } => format!("flush:shift seed={seed}"),
        });
    }
    parts.join(" ")
}

/// Shaped like the registry's `BadKnob` text, so a reader sees one form.
pub(super) fn knob_refusal(reason: impl std::fmt::Display) -> String {
    format!("bad chaos knob: {reason}")
}
