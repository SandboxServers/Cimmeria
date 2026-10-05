//! Path-free module and experimental SDK evidence from a Windows x86 process.
//! Neither establishes graphics-device creation or game readiness.
pub mod game_launch;
pub mod physx;
pub mod prerequisite;
use physx::SdkResult;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const MAX_REQUEST: usize = 8192;
pub const MAX_REPORT: usize = 8192;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    pub game_binaries: PathBuf,
}
pub fn decode(bytes: &[u8]) -> Result<Request, &'static str> {
    if bytes.len() > MAX_REQUEST {
        return Err("request_too_large");
    }
    let request: Request = serde_json::from_slice(bytes).map_err(|_| "invalid_request")?;
    if request.schema_version != 1 {
        return Err("unsupported_schema");
    }
    if !request.game_binaries.is_absolute()
        || request
            .game_binaries
            .components()
            .any(|p| matches!(p, std::path::Component::ParentDir))
    {
        return Err("invalid_directory");
    }
    Ok(request)
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum LoadResult {
    Loaded {},
    Unavailable { win32_error: u32 },
    ContextUnavailable {},
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Module {
    pub component: String,
    pub result: LoadResult,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema_version: u32,
    pub architecture: String,
    pub activation_context: LoadResult,
    pub modules: Vec<Module>,
    /// Module load and SDK initialization are deliberately separate observations.
    pub physx_sdk: SdkResult,
    /// Always false: a probe never starts SGW or attempts login.
    pub game_started: bool,
}

/// Validate the complete one-shot report before a host uses any component result.
/// Success still means module evidence only, never permission to launch a game.
pub fn decode_report(bytes: &[u8]) -> Result<Report, &'static str> {
    if bytes.len() > MAX_REPORT {
        return Err("report_too_large");
    }
    let report: Report = serde_json::from_slice(bytes).map_err(|_| "invalid_report")?;
    if report.schema_version != 2
        || report.architecture != "x86"
        || report.game_started
        || report.activation_context == (LoadResult::ContextUnavailable {})
    {
        return Err("invalid_report");
    }
    let components = [
        "vc80_crt",
        "vc80_cpp",
        "d3dx9_40",
        "xinput_1_3",
        "physx_loader",
    ];
    if report.modules.len() != components.len() {
        return Err("invalid_report");
    }
    for (index, (module, expected)) in report.modules.iter().zip(components).enumerate() {
        let skipped = index < 2 && report.activation_context != (LoadResult::Loaded {});
        if module.component != expected
            || (module.result == (LoadResult::ContextUnavailable {})) != skipped
        {
            return Err("invalid_report");
        }
    }
    if matches!(
        report.physx_sdk,
        SdkResult::CreateFailed { .. } | SdkResult::InitializedAndReleased {}
    ) && report.modules[4].result != (LoadResult::Loaded {})
    {
        return Err("invalid_report");
    }
    Ok(report)
}

pub fn collect(context: LoadResult, mut load: impl FnMut(&str) -> LoadResult) -> Report {
    let mut modules = Vec::new();
    for (component, dll, needs_context) in [
        ("vc80_crt", "msvcr80.dll", true),
        ("vc80_cpp", "msvcp80.dll", true),
        ("d3dx9_40", "d3dx9_40.dll", false),
        ("xinput_1_3", "xinput1_3.dll", false),
        ("physx_loader", "PhysXLoader.dll", false),
    ] {
        let result = if needs_context && context != (LoadResult::Loaded {}) {
            LoadResult::ContextUnavailable {}
        } else {
            load(dll)
        };
        modules.push(Module {
            component: component.into(),
            result,
        });
    }
    Report {
        schema_version: 2,
        architecture: "x86".into(),
        activation_context: context,
        modules,
        physx_sdk: SdkResult::NotChecked {},
        game_started: false,
    }
}

#[cfg(all(windows, target_arch = "x86"))]
pub mod windows;
#[cfg(all(windows, target_arch = "x86"))]
mod windows_physx;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_schema_and_unknown_inputs_are_rejected() {
        assert!(decode(&vec![b' '; MAX_REQUEST + 1]).is_err());
        assert!(decode(br#"{"schema_version":2,"game_binaries":"relative"}"#).is_err());
        assert!(decode(br#"{"schema_version":1,"game_binaries":"relative"}"#).is_err());
        assert!(
            decode(br#"{"schema_version":1,"game_binaries":"/game","dll":"evil.dll"}"#).is_err()
        );
    }
    #[test]
    fn failed_context_does_not_misclassify_vc80_or_skip_other_checks() {
        let mut calls = Vec::new();
        let report = collect(LoadResult::Unavailable { win32_error: 14001 }, |dll| {
            calls.push(dll.to_owned());
            LoadResult::Unavailable { win32_error: 126 }
        });
        assert_eq!(calls, ["d3dx9_40.dll", "xinput1_3.dll", "PhysXLoader.dll"]);
        assert_eq!(report.modules[0].result, LoadResult::ContextUnavailable {});
        assert_eq!(
            report.modules[2].result,
            LoadResult::Unavailable { win32_error: 126 }
        );
    }
    #[test]
    fn loaded_modules_never_assert_physx_or_game_readiness() {
        let report = collect(LoadResult::Loaded {}, |_| LoadResult::Loaded {});
        assert_eq!(report.modules.len(), 5);
        assert!(report.physx_sdk == (SdkResult::NotChecked {}) && !report.game_started);
        let json = serde_json::to_string(&report).unwrap();
        assert!(!json.contains("game_binaries") && !json.contains("ready"));
    }
}

#[cfg(test)]
mod report_tests {
    use super::*;
    fn report() -> serde_json::Value {
        serde_json::to_value(collect(LoadResult::Loaded {}, |_| LoadResult::Loaded {})).unwrap()
    }
    fn valid(value: &serde_json::Value) -> bool {
        decode_report(&serde_json::to_vec(value).unwrap()).is_ok()
    }
    #[test]
    fn host_rejects_overstated_mixed_or_incomplete_evidence() {
        let baseline = report();
        assert!(valid(&baseline));
        let mut changed = baseline.clone();
        changed["game_started"] = true.into();
        assert!(!valid(&changed));
        let mut changed = baseline.clone();
        changed["modules"][0]["result"] = serde_json::json!({"state":"context_unavailable"});
        assert!(!valid(&changed));
        let mut changed = baseline.clone();
        changed["activation_context"] =
            serde_json::json!({"state":"unavailable","win32_error":14001});
        assert!(
            !valid(&changed),
            "loaded CRT cannot come from failed context"
        );
        let mut changed = baseline.clone();
        changed["modules"].as_array_mut().unwrap().pop();
        assert!(!valid(&changed));
        let mut changed = baseline.clone();
        changed["modules"][0]["component"] = "physx_loader".into();
        assert!(
            !valid(&changed),
            "duplicate or reordered components are invalid"
        );
        let mut changed = baseline;
        changed["modules"][0]["result"]["path"] = "private".into();
        assert!(!valid(&changed));
        assert!(decode_report(&vec![b' '; MAX_REPORT + 1]).is_err());
    }
    #[test]
    fn sdk_evidence_requires_loaded_module_and_current_report_schema() {
        let mut evidence = report();
        evidence["schema_version"] = 1.into();
        assert!(!valid(&evidence));
        evidence["schema_version"] = 2.into();
        evidence["physx_sdk"] = serde_json::json!({"state":"initialized_and_released"});
        assert!(valid(&evidence));
        evidence["modules"][4]["result"] =
            serde_json::json!({"state":"unavailable","win32_error":126});
        assert!(!valid(&evidence));
        evidence["physx_sdk"] = serde_json::json!({"state":"not_checked","path":"secret"});
        assert!(!valid(&evidence));
    }
    #[test]
    fn failed_activation_report_is_valid_evidence_not_a_protocol_failure() {
        let report = collect(LoadResult::Unavailable { win32_error: 14001 }, |_| {
            LoadResult::Unavailable { win32_error: 126 }
        });
        let decoded = decode_report(&serde_json::to_vec(&report).unwrap()).unwrap();
        assert_eq!(decoded.modules[0].result, LoadResult::ContextUnavailable {});
        assert_eq!(
            decoded.modules[4].result,
            LoadResult::Unavailable { win32_error: 126 }
        );
    }
}
