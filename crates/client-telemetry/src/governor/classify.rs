//! Which events matter: the one table the governor reads.
//!
//! Every event the DLL emits is put in one [`Class`] before anything else
//! happens to it. The decision is made in three steps, in this order, and
//! the first one that applies wins:
//!
//! 1. **Level.** `warn` and `error` are always [`Class::MustKeep`]. A hook
//!    that reports a problem at `warn` never has it summarized away.
//! 2. **Outcome.** An event whose fields say something went wrong is
//!    [`Class::MustKeep`], whatever its level: `ok: false`,
//!    `success: false`, `failed: true`, a non-null `error`, or an `outcome`
//!    / `result` string in [`NON_HAPPY_OUTCOMES`].
//! 3. **Target.** The first row of [`RULES`] whose pattern matches the
//!    target decides. No row matching means [`Class::Budgeted`].
//!
//! Nothing else in the governor branches on target names. A new hook that
//! needs special handling gets a row here and a test in
//! `governor/tests/classify.rs`, not a condition somewhere else.

use std::collections::BTreeMap;

use serde_json::Value;

/// How the governor treats one event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Forwarded as is: never throttled, collapsed or summarized.
    MustKeep(KeepReason),
    /// The first `first_k` events per (target, `entity_id`, key) are
    /// forwarded; the rest go into the target's rollup. An event with no
    /// `entity_id` field is treated as [`Class::Budgeted`].
    PerEntity {
        /// Events forwarded per (target, entity, key) before the rollup
        /// takes over.
        first_k: u32,
    },
    /// A high-rate repetitive stream: never forwarded one by one, always
    /// summarized in the periodic rollup.
    Hot,
    /// Forwarded while the target is under its rate budget; identical
    /// consecutive events collapse, and whatever is over budget goes into
    /// the rollup.
    Budgeted,
}

impl Class {
    /// Short name for health counters and tests.
    pub fn as_str(self) -> &'static str {
        match self {
            Class::MustKeep(_) => "must_keep",
            Class::PerEntity { .. } => "per_entity",
            Class::Hot => "hot",
            Class::Budgeted => "budgeted",
        }
    }
}

/// Why an event is [`Class::MustKeep`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepReason {
    /// `warn` or `error` level.
    Level,
    /// The event's own fields report a failure (see [`NON_HAPPY_OUTCOMES`]).
    NonHappyOutcome,
    /// Entity create / enter / entered_world / leave / destroyed /
    /// appearance_request / queue_replay.
    EntityLifecycle,
    /// Mercury receive-path anomalies: fragments, bundles, errors, dropped
    /// methods.
    MercuryAnomaly,
    /// Hook install results, fingerprint and capabilities.
    HookInstall,
    /// DLL attach, session and catalog events: once per boot.
    SessionBoot,
    /// Crash, exception, assert and load-failure reports.
    Failure,
    /// The governor's own rollup, repeat and health events.
    SelfReport,
}

impl KeepReason {
    /// Short name for health counters and tests.
    pub fn as_str(self) -> &'static str {
        match self {
            KeepReason::Level => "level",
            KeepReason::NonHappyOutcome => "non_happy_outcome",
            KeepReason::EntityLifecycle => "entity_lifecycle",
            KeepReason::MercuryAnomaly => "mercury_anomaly",
            KeepReason::HookInstall => "hook_install",
            KeepReason::SessionBoot => "session_boot",
            KeepReason::Failure => "failure",
            KeepReason::SelfReport => "self_report",
        }
    }
}

/// How a [`Rule`] matches a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pattern {
    /// The whole target.
    Exact(&'static str),
    /// The start of the target.
    Prefix(&'static str),
}

impl Pattern {
    fn matches(self, target: &str) -> bool {
        match self {
            Pattern::Exact(p) => target == p,
            Pattern::Prefix(p) => target.starts_with(p),
        }
    }
}

/// One row of the classification table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule {
    /// Which targets the row covers.
    pub pattern: Pattern,
    /// What happens to them.
    pub class: Class,
    /// The field a rollup counts distinct values of (top-N keys). `None`
    /// uses the first of [`FALLBACK_KEY_FIELDS`] the event carries.
    pub key: Option<&'static str>,
}

const fn keep(pattern: Pattern, reason: KeepReason) -> Rule {
    Rule {
        pattern,
        class: Class::MustKeep(reason),
        key: None,
    }
}

const fn hot(pattern: Pattern, key: Option<&'static str>) -> Rule {
    Rule {
        pattern,
        class: Class::Hot,
        key,
    }
}

const fn per_entity(pattern: Pattern, key: &'static str) -> Rule {
    Rule {
        pattern,
        class: Class::PerEntity {
            first_k: DEFAULT_FIRST_K,
        },
        key: Some(key),
    }
}

/// Events forwarded per (target, entity, key) before the rollup takes
/// over, for [`Class::PerEntity`] rows. World entry for one entity raises
/// a handful of each CME event and method; 16 keeps all of them and caps
/// a stuck entity that repeats one forever.
pub const DEFAULT_FIRST_K: u32 = 16;

use Pattern::{Exact, Prefix};

/// The classification table. First match wins; order matters only where
/// a prefix row would shadow a more specific one.
pub const RULES: &[Rule] = &[
    // The governor's own output. Generated after classification, but
    // listed so that nothing can ever re-summarize a rollup.
    keep(Prefix("client.telemetry."), KeepReason::SelfReport),
    // Boot, session and install: rare, and the first thing anyone reads.
    keep(Prefix("client.dll."), KeepReason::SessionBoot),
    keep(Prefix("client.session."), KeepReason::SessionBoot),
    keep(Prefix("client.cme.catalog"), KeepReason::SessionBoot),
    keep(Prefix("client.hooks."), KeepReason::HookInstall),
    // Entity lifecycle, every variant (`appearance_request.scheduled` too).
    keep(Prefix("client.entity."), KeepReason::EntityLifecycle),
    // Mercury receive-path anomalies (#1088) and the drop oracle.
    keep(Exact("client.mercury.error"), KeepReason::MercuryAnomaly),
    keep(
        Prefix("client.mercury.fragment"),
        KeepReason::MercuryAnomaly,
    ),
    keep(Prefix("client.mercury.bundle"), KeepReason::MercuryAnomaly),
    keep(
        Exact("client.dispatch.method_dropped"),
        KeepReason::MercuryAnomaly,
    ),
    // Failures reported by the engine-layer sinks (#1084) and older hooks,
    // whatever level the hook chose.
    keep(Exact("client.lua.error"), KeepReason::Failure),
    keep(Exact("client.os.exception"), KeepReason::Failure),
    keep(Prefix("client.ue3.assert"), KeepReason::Failure),
    keep(Prefix("client.ue3.fatal"), KeepReason::Failure),
    keep(Prefix("client.physx.error"), KeepReason::Failure),
    keep(Prefix("client.physx.assert"), KeepReason::Failure),
    keep(Exact("client.io.open_failed"), KeepReason::Failure),
    keep(Exact("client.engine.load_failed"), KeepReason::Failure),
    keep(Exact("client.engine.hitch"), KeepReason::Failure),
    keep(
        Exact("client.engine.level_stream_slow"),
        KeepReason::Failure,
    ),
    // Per-entity streams: the first K per entity survive world entry.
    per_entity(Exact("client.cme.event"), "name"),
    per_entity(Exact("client.mercury.entity_method"), "msg_id"),
    per_entity(Exact("client.mercury.entity_property"), "msg_id"),
    per_entity(Exact("client.net.out"), "method"),
    // High-rate repetitive streams: summarized only.
    hot(Exact("client.engine.sequence_tick"), None),
    hot(Exact("client.engine.actor_tick"), Some("tick_type")),
    hot(Exact("client.engine.tick"), None),
    hot(Exact("client.engine.bink_tick"), None),
    hot(Exact("client.frame_tick"), None),
    hot(Exact("client.lua.pcall"), None),
    hot(Exact("client.lua.call"), None),
    hot(Exact("client.engine.async_archive_serialize"), None),
    hot(
        Exact("client.engine.static_load_object"),
        Some("package_name"),
    ),
    hot(Exact("client.engine.update_level_streaming"), None),
    hot(Exact("client.os.get_foreground_window"), None),
];

/// `outcome` / `result` strings that mark a failure. Compared lower-case.
pub const NON_HAPPY_OUTCOMES: &[&str] = &[
    "error",
    "failed",
    "failure",
    "rejected",
    "refused",
    "dropped",
    "timeout",
    "timed_out",
    "missing",
    "invalid",
    "no_entity",
];

/// Fields tried, in order, for a rollup's distinct-key count when the rule
/// names none.
pub const FALLBACK_KEY_FIELDS: &[&str] = &["name", "package_name", "method", "msg_id"];

/// The outcome of [`classify`]: the class and the rollup key field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verdict {
    /// What happens to the event.
    pub class: Class,
    /// The rule's key field, if it names one.
    pub key: Option<&'static str>,
}

/// The table row for `target`, if any.
pub fn rule_for(target: &str) -> Option<&'static Rule> {
    RULES.iter().find(|r| r.pattern.matches(target))
}

/// Classify one event. Pure: the same inputs always give the same answer.
pub fn classify(target: &str, level: &str, fields: &BTreeMap<String, Value>) -> Verdict {
    let rule = rule_for(target);
    let key = rule.and_then(|r| r.key);
    if level.eq_ignore_ascii_case("warn") || level.eq_ignore_ascii_case("error") {
        return Verdict {
            class: Class::MustKeep(KeepReason::Level),
            key,
        };
    }
    if is_non_happy(fields) {
        return Verdict {
            class: Class::MustKeep(KeepReason::NonHappyOutcome),
            key,
        };
    }
    Verdict {
        class: rule.map_or(Class::Budgeted, |r| r.class),
        key,
    }
}

/// Whether the fields report a failure.
pub fn is_non_happy(fields: &BTreeMap<String, Value>) -> bool {
    let is = |k: &str, v: bool| fields.get(k).and_then(Value::as_bool) == Some(v);
    if is("ok", false) || is("success", false) || is("failed", true) {
        return true;
    }
    if fields.get("error").is_some_and(|v| !v.is_null()) {
        return true;
    }
    ["outcome", "result"].iter().any(|k| {
        fields
            .get(*k)
            .and_then(Value::as_str)
            .is_some_and(|s| NON_HAPPY_OUTCOMES.iter().any(|n| s.eq_ignore_ascii_case(n)))
    })
}
