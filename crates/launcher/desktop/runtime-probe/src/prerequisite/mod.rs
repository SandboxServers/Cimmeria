//! One-shot prerequisite worker contract. The parent must persist admission and
//! host identity before dispatch, own the prefix, and enforce deadline/recovery.
//! The worker does not infer guest quiescence, accept UI commands or start SGW.
pub mod package;
#[cfg(all(windows, target_arch = "x86"))]
pub mod windows;
use crate::{decode_report, Report, MAX_REQUEST};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

pub const MAX_RESULT: usize = 16_384;
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrepareRequest {
    pub schema_version: u32,
    pub operation_id: Uuid,
    pub prefix_generation: Uuid,
    pub game_binaries: PathBuf,
    pub package: PathBuf,
    /// Fresh, native-owned work directory. Never overlay an existing attempt.
    pub scratch: PathBuf,
}
pub fn decode_request(bytes: &[u8]) -> Result<PrepareRequest, &'static str> {
    if bytes.len() > MAX_REQUEST {
        return Err("request_too_large");
    }
    let request: PrepareRequest = serde_json::from_slice(bytes).map_err(|_| "invalid_request")?;
    if request.schema_version != 1
        || request.operation_id.is_nil()
        || request.prefix_generation.is_nil()
        || [&request.game_binaries, &request.package, &request.scratch]
            .iter()
            .any(|p| !absolute(p))
        || request.scratch.file_name().is_none()
    {
        return Err("invalid_request");
    }
    Ok(request)
}
fn absolute(path: &Path) -> bool {
    path.is_absolute()
        && !path.components().any(|c| matches!(c, Component::ParentDir))
        && !path.as_os_str().to_string_lossy().contains('\0')
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    InvalidInput,
    PackageIdentity,
    ScratchUnavailable,
    Io,
    Probe,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResultKind {
    Failed {
        reason: Failure,
    },
    /// Includes reboot statuses; only zero permits the following functional probe.
    InstallerFailed {
        installer_code: u32,
    },
    /// Probe completion is evidence, never a claim of prerequisite/game readiness.
    Probed {
        report: Report,
    },
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrepareResult {
    pub schema_version: u32,
    pub operation_id: Uuid,
    pub prefix_generation: Uuid,
    pub result: ResultKind,
}
/// Preserve an installer failure instead of laundering it through a successful
/// probe. Reboot-required results are intentionally not accepted as completion.
pub fn after_install(code: u32, probe: impl FnOnce() -> Result<Report, Failure>) -> ResultKind {
    if code != 0 {
        return ResultKind::InstallerFailed {
            installer_code: code,
        };
    }
    match probe() {
        Ok(report) => ResultKind::Probed { report },
        Err(reason) => ResultKind::Failed { reason },
    }
}
pub fn decode_result(
    bytes: &[u8],
    operation: Uuid,
    generation: Uuid,
) -> Result<PrepareResult, &'static str> {
    if bytes.len() > MAX_RESULT {
        return Err("result_too_large");
    }
    let result: PrepareResult = serde_json::from_slice(bytes).map_err(|_| "invalid_result")?;
    if result.schema_version != 1
        || operation.is_nil()
        || generation.is_nil()
        || result.operation_id != operation
        || result.prefix_generation != generation
    {
        return Err("invalid_result");
    }
    match &result.result {
        ResultKind::Probed { report } => {
            if matches!(report.physx_sdk, crate::physx::SdkResult::NotChecked {}) {
                return Err("invalid_result");
            }
            let report = serde_json::to_vec(report).map_err(|_| "invalid_result")?;
            decode_report(&report)?;
        }
        ResultKind::InstallerFailed { installer_code: 0 } => return Err("invalid_result"),
        _ => {}
    }
    Ok(result)
}
#[cfg(test)]
mod tests;
