//! Wire types for the launcher-summary ingest, and the validation that
//! turns one JSON element into a [`Summary`].
//!
//! Everything that leaves this module is a closed enum, a bounded integer
//! or a parsed UUID. No string the client sent survives validation: the
//! enums are emitted through [`as_str`](Operation::as_str), the ids and the
//! version are re-formatted from their parsed values, and a serde error is
//! discarded where it is produced.

use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::routes::dev_session::quota::QuotaExceeded;
use crate::routes::dev_session::AuthError;

/// The only request shape this route accepts.
pub(super) const SCHEMA_VERSION: u64 = 1;
/// Most summaries one request may carry.
pub(super) const MAX_SUMMARIES: usize = 32;
/// Most entries one summary's `phases` may carry.
pub(super) const MAX_PHASES: usize = 32;
/// Longest duration a summary or a phase may report: seven days.
pub(super) const MAX_DURATION_MS: u64 = 604_800_000;
/// Highest `retry_count` a summary may report.
pub(super) const MAX_RETRY_COUNT: u64 = 100;

/// A closed wire enum: deserialized from, and emitted as, exactly the
/// listed strings. `ALL` lets the tests prove the golden request covers
/// every value.
macro_rules! wire_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
        pub(super) enum $name {
            $(#[serde(rename = $text)] $variant),+
        }

        impl $name {
            #[cfg(test)]
            pub(super) const ALL: &'static [$name] = &[$($name::$variant),+];

            pub(super) fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $text),+
                }
            }
        }
    };
}

wire_enum! {
    /// What the launcher was doing.
    Operation {
        Install => "install",
        PrepareRuntime => "prepare_runtime",
        Repair => "repair",
        Uninstall => "uninstall",
        Launch => "launch",
    }
}

wire_enum! {
    /// Where the attempt ended.
    Phase {
        None => "none",
        PlatformCheck => "platform_check",
        CompatibilityCheck => "compatibility_check",
        CatalogFetch => "catalog_fetch",
        ManifestVerify => "manifest_verify",
        DestinationCheck => "destination_check",
        Admission => "admission",
        Starting => "starting",
        Running => "running",
        Download => "download",
        Extraction => "extraction",
    }
}

wire_enum! {
    /// The phases the launcher times, and the only values `phases[]` may
    /// name. Each is also a [`Phase`], spelled the same.
    TimedPhase {
        Starting => "starting",
        Running => "running",
        Download => "download",
        Extraction => "extraction",
    }
}

wire_enum! {
    /// How the attempt ended. `Unknown` is its own outcome (the launcher
    /// lost sight of the attempt), never a success or a failure.
    Outcome {
        Succeeded => "succeeded",
        Failed => "failed",
        Cancelled => "cancelled",
        Unknown => "unknown",
    }
}

wire_enum! {
    /// Why a failed attempt failed.
    ErrorCode {
        Unspecified => "unspecified",
        PlatformUnavailable => "platform_unavailable",
        LauncherTooOld => "launcher_too_old",
        InvalidDirectory => "invalid_directory",
        ManifestUnavailable => "manifest_unavailable",
        ManifestInvalid => "manifest_invalid",
        SigningKeyUnavailable => "signing_key_unavailable",
        StateInvalid => "state_invalid",
        LocalIo => "local_io",
        DestinationUnavailable => "destination_unavailable",
        InstallFailed => "install_failed",
        ContentInvalid => "content_invalid",
        RosettaRequired => "rosetta_required",
        RuntimeUnavailable => "runtime_unavailable",
        PrerequisiteFailed => "prerequisite_failed",
        LaunchNotStarted => "launch_not_started",
        LaunchEarlyExit => "launch_early_exit",
        LaunchExitNonzero => "launch_exit_nonzero",
    }
}

wire_enum! {
    /// The launcher's host operating system.
    Os {
        Windows => "windows",
        Macos => "macos",
        Linux => "linux",
    }
}

wire_enum! {
    /// The launcher's host architecture.
    Arch {
        X86_64 => "x86_64",
        Aarch64 => "aarch64",
    }
}

/// What the launcher dropped before it could send, since its last
/// acknowledged request. `u16` is the wire range (0 to 65535).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ClientDropped {
    pub overflow: u16,
    pub expired: u16,
    pub rejected: u16,
}

/// A request that passed the envelope checks. The elements are still
/// untyped JSON: each is validated alone, so one bad element is rejected
/// without failing the others.
#[derive(Debug)]
pub(super) struct Envelope {
    pub client_dropped: ClientDropped,
    pub elements: Vec<Value>,
}

/// One timed phase of a validated summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PhaseTiming {
    pub phase: TimedPhase,
    pub duration_ms: u32,
}

/// One validated summary: every field is in range and none holds a string
/// the client wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Summary {
    pub event_id: Uuid,
    pub attempt_id: Uuid,
    pub operation: Operation,
    pub phase: Phase,
    pub outcome: Outcome,
    pub error_code: Option<ErrorCode>,
    pub duration_ms: Option<u32>,
    pub retry_count: u8,
    pub phases: Vec<PhaseTiming>,
    pub launcher_version: (u16, u16, u16),
    pub os: Os,
    pub arch: Arch,
}

/// The positional verdict for one element of `summaries`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Verdict {
    /// Valid and seen for the first time by this server process; handed to
    /// the log pipeline. Not a storage acknowledgement.
    Accepted,
    /// Valid, but its `event_id` was already accepted.
    Duplicate,
    /// Failed validation.
    Rejected,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(super) struct SummaryResponse {
    /// Same length and order as the request's `summaries`.
    pub results: Vec<Verdict>,
}

/// Why a request was refused as a whole. Every body is static text: a
/// refusal never repeats anything the caller sent.
#[derive(Debug)]
pub(super) enum SummaryError {
    /// `CIMMERIA_TELEMETRY_KILL_SWITCH=1`: 503 with `Retry-After`.
    Paused,
    /// The peer address has used its allowance: 429 with `Retry-After`.
    OverQuota(QuotaExceeded),
    /// `Content-Type` is not `application/json`: 415.
    UnsupportedMediaType,
    /// The body is not the schema-1 envelope: 400.
    BadRequest(&'static str),
}

impl SummaryError {
    pub(super) const CONTENT_TYPE: &'static str = "Content-Type must be application/json";
}

impl From<QuotaExceeded> for SummaryError {
    fn from(e: QuotaExceeded) -> Self {
        SummaryError::OverQuota(e)
    }
}

impl IntoResponse for SummaryError {
    fn into_response(self) -> Response {
        match self {
            // The pause and the quota answer as the mint's do (status,
            // `Retry-After`, wording), through the mint's one mapping. The
            // quota text names the server's own counter and the wait, and
            // nothing the caller wrote.
            SummaryError::Paused => AuthError::KillSwitchActive.to_response(),
            SummaryError::OverQuota(e) => AuthError::QuotaExceeded(e).to_response(),
            SummaryError::UnsupportedMediaType => {
                (StatusCode::UNSUPPORTED_MEDIA_TYPE, Self::CONTENT_TYPE).into_response()
            }
            SummaryError::BadRequest(text) => (StatusCode::BAD_REQUEST, text).into_response(),
        }
    }
}

/// True for `application/json`, alone or with a `charset` parameter, which
/// is all the launcher sends. Any other media type (`text/plain`,
/// `application/x-ndjson`, `application/gzip`, a `+json` suffix), any other
/// parameter and a value that is not visible ASCII are refused: the route
/// takes one payload and says so before it parses anything.
///
/// The charset's value is not read. The body is parsed as UTF-8 JSON
/// whatever it claims, and a body that is not UTF-8 is a 400.
pub(super) fn is_json_content_type(value: &HeaderValue) -> bool {
    let Ok(text) = value.to_str() else {
        return false;
    };
    let mut parts = text.split(';');
    let essence = parts.next().unwrap_or_default().trim();
    essence.eq_ignore_ascii_case("application/json")
        && parts.all(|parameter| {
            parameter
                .split_once('=')
                .is_some_and(|(name, _)| name.trim().eq_ignore_ascii_case("charset"))
        })
}

/// Check the request envelope: a JSON object with exactly `schema_version`
/// (the integer 1), `client_dropped` and 1 to 32 `summaries`. The element
/// count is checked before any element is typed.
pub(super) fn parse_envelope(body: &[u8]) -> Result<Envelope, SummaryError> {
    let Ok(Value::Object(mut top)) = serde_json::from_slice::<Value>(body) else {
        return Err(SummaryError::BadRequest("Body is not a JSON object"));
    };
    let (Some(schema_version), Some(client_dropped), Some(summaries)) = (
        top.remove("schema_version"),
        top.remove("client_dropped"),
        top.remove("summaries"),
    ) else {
        return Err(SummaryError::BadRequest("Missing a required key"));
    };
    if !top.is_empty() {
        return Err(SummaryError::BadRequest("Unknown top-level key"));
    }
    if schema_version.as_u64() != Some(SCHEMA_VERSION) {
        return Err(SummaryError::BadRequest("Unsupported schema_version"));
    }
    let Ok(client_dropped) = serde_json::from_value::<ClientDropped>(client_dropped) else {
        return Err(SummaryError::BadRequest("Invalid client_dropped"));
    };
    let elements = match summaries {
        Value::Array(elements) if (1..=MAX_SUMMARIES).contains(&elements.len()) => elements,
        _ => {
            return Err(SummaryError::BadRequest(
                "summaries must be an array of 1 to 32 elements",
            ))
        }
    };
    Ok(Envelope {
        client_dropped,
        elements,
    })
}

/// An optional key that, when present, must hold a value: an explicit
/// `null` is refused rather than read as "absent".
fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPhase {
    phase: TimedPhase,
    duration_ms: u64,
}

/// One element as typed by serde: closed enums and unknown keys are
/// already enforced, ranges and cross-field rules are not.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSummary {
    event_id: String,
    attempt_id: String,
    operation: Operation,
    phase: Phase,
    outcome: Outcome,
    #[serde(default, deserialize_with = "present")]
    error_code: Option<ErrorCode>,
    #[serde(default, deserialize_with = "present")]
    duration_ms: Option<u64>,
    retry_count: u64,
    #[serde(default, deserialize_with = "present")]
    phases: Option<Vec<RawPhase>>,
    launcher_version: String,
    os: Os,
    arch: Arch,
}

/// A launcher version: exactly three dot-separated components of one to
/// three ASCII digits. Returns the parsed integers, and callers emit those,
/// never the string, so nothing the caller typed reaches a log line or a
/// SigNoz field.
pub(super) fn parse_version_triple(value: &str) -> Option<(u16, u16, u16)> {
    // `str::parse` alone would take a leading `+`; the byte check also
    // keeps non-ASCII digits out.
    fn component(part: &str) -> Option<u16> {
        let digits = (1..=3).contains(&part.len()) && part.bytes().all(|b| b.is_ascii_digit());
        if !digits {
            return None;
        }
        part.parse().ok()
    }
    let mut parts = value.split('.');
    let triple = (
        component(parts.next()?)?,
        component(parts.next()?)?,
        component(parts.next()?)?,
    );
    parts.next().is_none().then_some(triple)
}

/// A UUID in any spelling `Uuid::parse_str` reads (hyphenated, simple,
/// braced, URN), except nil. Callers use the parsed value, so two
/// spellings of one id are one id.
fn parse_id(text: &str) -> Option<Uuid> {
    Uuid::parse_str(text).ok().filter(|id| !id.is_nil())
}

fn bounded_duration(ms: u64) -> Option<u32> {
    (ms <= MAX_DURATION_MS)
        .then(|| u32::try_from(ms).ok())
        .flatten()
}

/// Validate one element. `None` is the `rejected` verdict; why is
/// deliberately not kept, since the only honest description would quote
/// the element.
pub(super) fn validate(element: Value) -> Option<Summary> {
    let raw: RawSummary = serde_json::from_value(element).ok()?;
    let event_id = parse_id(&raw.event_id)?;
    let attempt_id = parse_id(&raw.attempt_id)?;
    // `error_code` is required on a failure and forbidden otherwise.
    if (raw.outcome == Outcome::Failed) != raw.error_code.is_some() {
        return None;
    }
    let duration_ms = match raw.duration_ms {
        Some(ms) => Some(bounded_duration(ms)?),
        None => None,
    };
    let retry_count = u8::try_from(raw.retry_count)
        .ok()
        .filter(|n| u64::from(*n) <= MAX_RETRY_COUNT)?;
    let raw_phases = raw.phases.unwrap_or_default();
    if raw_phases.len() > MAX_PHASES {
        return None;
    }
    let mut phases: Vec<PhaseTiming> = Vec::with_capacity(raw_phases.len());
    for entry in raw_phases {
        if phases.iter().any(|seen| seen.phase == entry.phase) {
            return None;
        }
        phases.push(PhaseTiming {
            phase: entry.phase,
            duration_ms: bounded_duration(entry.duration_ms)?,
        });
    }
    Some(Summary {
        event_id,
        attempt_id,
        operation: raw.operation,
        phase: raw.phase,
        outcome: raw.outcome,
        error_code: raw.error_code,
        duration_ms,
        retry_count,
        phases,
        launcher_version: parse_version_triple(&raw.launcher_version)?,
        os: raw.os,
        arch: raw.arch,
    })
}
