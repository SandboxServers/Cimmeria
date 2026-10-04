//! Wire contract v1. Every value is a closed enum, a bounded integer or an
//! engine-minted UUID, so no path, URL, name or error text can reach a summary.
use crate::OperationKind;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

pub const SCHEMA_VERSION: u32 = 1;
/// The server types at most this many summaries per request.
pub const MAX_BATCH: usize = 32;
pub const MAX_BODY_BYTES: usize = 64 * 1024;
/// There are four timed phases; the server accepts up to this many entries.
pub const MAX_PHASES: usize = 32;
pub const SESSION_KIND: &str = "launcher_summary";

// `ALL` is generated from the variant list, so it cannot fall behind the enum.
macro_rules! closed_enum {
    ($(#[$meta:meta])* $name:ident { $($(#[$variant_meta:meta])* $variant:ident),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($(#[$variant_meta])* $variant),+ }
        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
        }
    };
}

closed_enum!(SummaryOperation {
    Install,
    PrepareRuntime,
    Repair,
    Uninstall,
    Launch,
});
closed_enum!(
    /// Where the attempt ended. `None` also covers "not observed by this process".
    SummaryPhase {
        None,
        PlatformCheck,
        CompatibilityCheck,
        CatalogFetch,
        ManifestVerify,
        DestinationCheck,
        Admission,
        Starting,
        Running,
        Download,
        Extraction,
    }
);
closed_enum!(
    /// The only phases that carry a duration.
    TimedPhase {
        Starting,
        Running,
        Download,
        Extraction,
    }
);
closed_enum!(
    /// Launch `Succeeded` means the observed game process exited with code 0.
    /// It is never login or world entry.
    SummaryOutcome {
        Succeeded,
        Failed,
        Cancelled,
        Unknown,
    }
);
closed_enum!(SummaryErrorCode {
    Unspecified,
    PlatformUnavailable,
    LauncherTooOld,
    InvalidDirectory,
    ManifestUnavailable,
    ManifestInvalid,
    SigningKeyUnavailable,
    StateInvalid,
    LocalIo,
    DestinationUnavailable,
    InstallFailed,
    ContentInvalid,
    RosettaRequired,
    RuntimeUnavailable,
    PrerequisiteFailed,
    LaunchNotStarted,
    LaunchEarlyExit,
    LaunchExitNonzero,
});
closed_enum!(SummaryOs {
    Windows,
    Macos,
    Linux,
});
closed_enum!(SummaryArch {
    #[serde(rename = "x86_64")]
    X86_64,
    Aarch64,
});
closed_enum!(
    /// One positional verdict per summary in an ingest response.
    SummaryResult {
        Accepted,
        Duplicate,
        Rejected,
    }
);

impl SummaryOperation {
    /// The wire operation for a journal kind. `None` means the kind has no
    /// value in wire schema v1, so such an operation is never summarized.
    pub(super) fn for_kind(kind: OperationKind) -> Option<Self> {
        match kind {
            OperationKind::Install => Some(Self::Install),
            OperationKind::PrepareRuntime => Some(Self::PrepareRuntime),
            OperationKind::Repair => Some(Self::Repair),
            OperationKind::Uninstall => Some(Self::Uninstall),
            OperationKind::Launch => Some(Self::Launch),
            // Adoption of an existing installation has no schema v1 value.
            OperationKind::Adopt => None,
        }
    }
}
impl From<TimedPhase> for SummaryPhase {
    fn from(phase: TimedPhase) -> Self {
        match phase {
            TimedPhase::Starting => Self::Starting,
            TimedPhase::Running => Self::Running,
            TimedPhase::Download => Self::Download,
            TimedPhase::Extraction => Self::Extraction,
        }
    }
}
// A row names the platform this build runs on, or the build does not exist: a
// target outside the wire enums fails to compile instead of borrowing a value.
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
compile_error!("launcher summaries have no `os` value for this target");
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
compile_error!("launcher summaries have no `arch` value for this target");

impl SummaryOs {
    #[cfg(target_os = "windows")]
    const CURRENT: Self = Self::Windows;
    #[cfg(target_os = "macos")]
    const CURRENT: Self = Self::Macos;
    #[cfg(target_os = "linux")]
    const CURRENT: Self = Self::Linux;

    pub fn current() -> Self {
        Self::CURRENT
    }
}
impl SummaryArch {
    #[cfg(target_arch = "x86_64")]
    const CURRENT: Self = Self::X86_64;
    #[cfg(target_arch = "aarch64")]
    const CURRENT: Self = Self::Aarch64;

    pub fn current() -> Self {
        Self::CURRENT
    }
}

/// Milliseconds, at most seven days.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct Millis(u32);
impl Millis {
    pub const MAX: Self = Self(604_800_000);
    pub fn saturating(duration: Duration) -> Self {
        Self(
            u32::try_from(duration.as_millis())
                .unwrap_or(u32::MAX)
                .min(Self::MAX.0),
        )
    }
    pub fn get(self) -> u32 {
        self.0
    }
}
impl TryFrom<u64> for Millis {
    type Error = &'static str;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        u32::try_from(value)
            .ok()
            .filter(|value| *value <= Self::MAX.0)
            .map(Self)
            .ok_or("duration out of range")
    }
}
impl From<Millis> for u64 {
    fn from(value: Millis) -> Self {
        value.0.into()
    }
}

/// Repeats of one identical pre-admission failure, at most 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct RetryCount(u8);
impl RetryCount {
    pub const ZERO: Self = Self(0);
    pub const MAX: Self = Self(100);
    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1).min(Self::MAX.0))
    }
    pub fn get(self) -> u8 {
        self.0
    }
}
impl TryFrom<u64> for RetryCount {
    type Error = &'static str;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        u8::try_from(value)
            .ok()
            .filter(|value| *value <= Self::MAX.0)
            .map(Self)
            .ok_or("retry count out of range")
    }
}
impl From<RetryCount> for u64 {
    fn from(value: RetryCount) -> Self {
        value.0.into()
    }
}

/// "a.b.c", each component 0 to 999. It is emitted from the parsed integers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct LauncherVersion(u16, u16, u16);
impl LauncherVersion {
    /// The product version is native input; clamp it so the wire value stays valid.
    pub fn new((major, minor, patch): (u16, u16, u16)) -> Self {
        Self(major.min(999), minor.min(999), patch.min(999))
    }
}
impl TryFrom<String> for LauncherVersion {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut parts = value.split('.').map(|part| {
            ((1..=3).contains(&part.len()) && part.bytes().all(|byte| byte.is_ascii_digit()))
                .then(|| part.parse::<u16>().ok())
                .flatten()
        });
        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some(Some(major)), Some(Some(minor)), Some(Some(patch)), None) => {
                Ok(Self(major, minor, patch))
            }
            _ => Err("launcher version is not three short integers"),
        }
    }
}
impl From<LauncherVersion> for String {
    fn from(value: LauncherVersion) -> Self {
        format!("{}.{}.{}", value.0, value.1, value.2)
    }
}

/// Deltas since the last acknowledged upload.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DroppedCounts {
    pub overflow: u16,
    pub expired: u16,
    pub rejected: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhaseDuration {
    pub phase: TimedPhase,
    pub duration_ms: Millis,
}

/// One terminal row per attempt. `phases` holds what this process timed itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawSummary")]
pub struct Summary {
    pub event_id: Uuid,
    pub attempt_id: Uuid,
    pub operation: SummaryOperation,
    pub phase: SummaryPhase,
    pub outcome: SummaryOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<SummaryErrorCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<Millis>,
    pub retry_count: RetryCount,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phases: Option<Vec<PhaseDuration>>,
    pub launcher_version: LauncherVersion,
    pub os: SummaryOs,
    pub arch: SummaryArch,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSummary {
    event_id: Uuid,
    attempt_id: Uuid,
    operation: SummaryOperation,
    phase: SummaryPhase,
    outcome: SummaryOutcome,
    #[serde(default)]
    error_code: Option<SummaryErrorCode>,
    #[serde(default)]
    duration_ms: Option<Millis>,
    retry_count: RetryCount,
    #[serde(default)]
    phases: Option<Vec<PhaseDuration>>,
    launcher_version: LauncherVersion,
    os: SummaryOs,
    arch: SummaryArch,
}
impl TryFrom<RawSummary> for Summary {
    type Error = &'static str;
    fn try_from(raw: RawSummary) -> Result<Self, Self::Error> {
        if raw.event_id.is_nil() || raw.attempt_id.is_nil() {
            return Err("nil id");
        }
        if raw.error_code.is_some() != (raw.outcome == SummaryOutcome::Failed) {
            return Err("error code must accompany exactly a failed outcome");
        }
        if let Some(phases) = &raw.phases {
            let repeated = phases
                .iter()
                .enumerate()
                .any(|(index, entry)| phases[..index].iter().any(|seen| seen.phase == entry.phase));
            if phases.len() > MAX_PHASES || repeated {
                return Err("too many phases, or one listed twice");
            }
        }
        Ok(Self {
            event_id: raw.event_id,
            attempt_id: raw.attempt_id,
            operation: raw.operation,
            phase: raw.phase,
            outcome: raw.outcome,
            error_code: raw.error_code,
            duration_ms: raw.duration_ms,
            retry_count: raw.retry_count,
            phases: raw.phases,
            launcher_version: raw.launcher_version,
            os: raw.os,
            arch: raw.arch,
        })
    }
}

/// The ingest body: `POST {base}/telemetry/launcher-summary`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawRequest")]
pub struct SummaryRequest {
    pub schema_version: u32,
    pub client_dropped: DroppedCounts,
    pub summaries: Vec<Summary>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRequest {
    schema_version: u32,
    client_dropped: DroppedCounts,
    summaries: Vec<Summary>,
}
impl TryFrom<RawRequest> for SummaryRequest {
    type Error = &'static str;
    fn try_from(raw: RawRequest) -> Result<Self, Self::Error> {
        if raw.schema_version != SCHEMA_VERSION {
            return Err("unsupported schema version");
        }
        if !(1..=MAX_BATCH).contains(&raw.summaries.len()) {
            return Err("summary count out of range");
        }
        Ok(Self {
            schema_version: raw.schema_version,
            client_dropped: raw.client_dropped,
            summaries: raw.summaries,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SummaryResponse {
    pub results: Vec<SummaryResult>,
}

/// The mint body: `POST {base}/auth/dev-session`. `install_id` is a fresh random
/// value for each mint and is never stored; the identity fields are always empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MintRequest {
    install_id: Uuid,
    machine_id: &'static str,
    branch: &'static str,
    git_sha: &'static str,
    launcher_version: LauncherVersion,
    tags: [&'static str; 0],
    session_kind: &'static str,
}
impl MintRequest {
    pub fn new(install_id: Uuid, launcher_version: LauncherVersion) -> Self {
        Self {
            install_id,
            machine_id: "",
            branch: "",
            git_sha: "",
            launcher_version,
            tags: [],
            session_kind: SESSION_KIND,
        }
    }
}
