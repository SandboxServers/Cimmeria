//! Narrow, path-free runtime evidence from a Windows x86 process. Module loading
//! is not SDK initialization, graphics-device creation, or game readiness.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const MAX_REQUEST: usize = 8192;
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

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum LoadResult {
    Loaded,
    Unavailable { win32_error: u32 },
    ContextUnavailable,
}
#[derive(Serialize)]
pub struct Module {
    pub component: &'static str,
    pub result: LoadResult,
}
#[derive(Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub architecture: &'static str,
    pub activation_context: LoadResult,
    pub modules: Vec<Module>,
    /// Always false: loading PhysXLoader is not proof its registered engine works.
    pub physx_engine_checked: bool,
    /// Always false: a probe never starts SGW or attempts login.
    pub game_started: bool,
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
        let result = if needs_context && context != LoadResult::Loaded {
            LoadResult::ContextUnavailable
        } else {
            load(dll)
        };
        modules.push(Module { component, result });
    }
    Report {
        schema_version: 1,
        architecture: "x86",
        activation_context: context,
        modules,
        physx_engine_checked: false,
        game_started: false,
    }
}

#[cfg(all(windows, target_arch = "x86"))]
pub mod windows;

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
        assert_eq!(report.modules[0].result, LoadResult::ContextUnavailable);
        assert_eq!(
            report.modules[2].result,
            LoadResult::Unavailable { win32_error: 126 }
        );
    }
    #[test]
    fn loaded_modules_never_assert_physx_or_game_readiness() {
        let report = collect(LoadResult::Loaded, |_| LoadResult::Loaded);
        assert_eq!(report.modules.len(), 5);
        assert!(!report.physx_engine_checked && !report.game_started);
        let json = serde_json::to_string(&report).unwrap();
        assert!(!json.contains("game_binaries") && !json.contains("ready"));
    }
}
