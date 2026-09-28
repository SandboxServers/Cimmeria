//! Whether to inject the client-patches DLL, and the injection order.

use std::path::{Path, PathBuf};

use super::dll_source::{DllSource, DllUnavailable};
use crate::config::ClientPatchesSettings;

/// What the launcher will do with the DLL on this launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InjectDecision {
    Inject(DllSource),
    /// The player turned client patches off.
    OptedOut,
    /// Wanted, but there is no DLL to inject; the reason is for the
    /// status log.
    Unavailable(String),
}

/// Decide from the settings and, only when patches are on, the DLL
/// lookup. An opted-out launch never touches the disk.
pub fn decide(
    settings: &ClientPatchesSettings,
    resolve: impl FnOnce() -> Result<DllSource, DllUnavailable>,
) -> InjectDecision {
    if !settings.enabled {
        return InjectDecision::OptedOut;
    }
    match resolve() {
        Ok(src) => InjectDecision::Inject(src),
        Err(e) => InjectDecision::Unavailable(e.to_string()),
    }
}

/// How the client-patches DLL fared on one launch: the `injection`
/// field of the once-per-session telemetry event, so a missing Black
/// Market window can be told apart from "never loaded" in SigNoz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchInjection {
    Injected,
    OptedOut,
    Unavailable,
    /// The DLL was found but injection failed; the game was relaunched
    /// without it.
    InjectFailed,
}

impl PatchInjection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Injected => "injected",
            Self::OptedOut => "opted_out",
            Self::Unavailable => "unavailable",
            Self::InjectFailed => "inject_failed",
        }
    }
}

/// The DLLs to inject, in order: client patches first, then telemetry.
///
/// Both DLLs MinHook `FEngineLoop::Tick` and the drop callee
/// (`0x01590f30`). The patches DLL's fingerprint gate accepts an earlier
/// hook at those two sites only when it jumps into the telemetry DLL;
/// the telemetry DLL has no gate and MinHook relocates whatever jump it
/// finds into its trampoline. Injecting patches first means its
/// bootstrap thread, which hooks straight away, normally finds the
/// stock prologues and never depends on the chain rule. The telemetry
/// DLL hooks later (it reads its session file first) and chains on top.
/// If the two ever race anyway, the patches DLL re-reads each prologue
/// after MinHook copies it and rebuilds the hook when it changed.
/// Gameplay also comes first: if the telemetry injection fails, the
/// patches are already in.
pub fn injection_order(patches: Option<&Path>, telemetry: Option<&Path>) -> Vec<PathBuf> {
    patches
        .into_iter()
        .chain(telemetry)
        .map(Path::to_path_buf)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(enabled: bool) -> ClientPatchesSettings {
        ClientPatchesSettings {
            enabled,
            dll_override: None,
        }
    }

    #[test]
    fn enabled_with_a_dll_injects_it() {
        let src = DllSource::Bundled(PathBuf::from("p.dll"));
        let expected = src.clone();
        assert_eq!(
            decide(&settings(true), || Ok(src)),
            InjectDecision::Inject(expected)
        );
    }

    /// The opt-out must not even look for the DLL (no disk writes).
    #[test]
    fn opted_out_never_resolves() {
        let decision = decide(&settings(false), || {
            panic!("an opted-out launch must not resolve the DLL")
        });
        assert_eq!(decision, InjectDecision::OptedOut);
    }

    #[test]
    fn enabled_without_a_dll_is_unavailable_with_the_reason() {
        let decision = decide(&settings(true), || {
            Err(DllUnavailable::NotBundled(PathBuf::from("L")))
        });
        match decision {
            InjectDecision::Unavailable(why) => {
                assert!(why.contains("does not bundle"), "{why}")
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn patches_go_in_before_telemetry() {
        let order = injection_order(Some(Path::new("patches.dll")), Some(Path::new("tel.dll")));
        assert_eq!(
            order,
            vec![PathBuf::from("patches.dll"), PathBuf::from("tel.dll")]
        );
    }

    #[test]
    fn order_skips_absent_dlls() {
        assert_eq!(
            injection_order(None, Some(Path::new("tel.dll"))),
            vec![PathBuf::from("tel.dll")]
        );
        assert_eq!(
            injection_order(Some(Path::new("p.dll")), None),
            vec![PathBuf::from("p.dll")]
        );
        assert!(injection_order(None, None).is_empty());
    }

    #[test]
    fn injection_labels_are_stable() {
        // SigNoz queries filter on these strings.
        assert_eq!(PatchInjection::Injected.as_str(), "injected");
        assert_eq!(PatchInjection::OptedOut.as_str(), "opted_out");
        assert_eq!(PatchInjection::Unavailable.as_str(), "unavailable");
        assert_eq!(PatchInjection::InjectFailed.as_str(), "inject_failed");
    }
}
