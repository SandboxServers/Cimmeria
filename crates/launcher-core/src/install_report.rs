//! What one install run did to each patch, for the install-result
//! telemetry event ([`crate::telemetry::install_result`]).
//!
//! [`crate::install::install_all`] fills an [`InstallReport`] as it goes:
//! the seed step, then one [`PatchOutcome`] per manifest patch in
//! declared order, then how the run ended. The report is plain data, so
//! the event is built and tested without a network or an install dir.

use crate::install::InstallError;
use crate::unpack::UnpackError;

/// What happened to one patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchOutcomeKind {
    /// Downloaded, verified and unpacked this run.
    Applied,
    /// Recorded as applied by an earlier run; nothing done.
    Already,
    /// Tried and failed; the reason says why.
    Failed,
    /// Not tried: its `after` names a patch that did not apply this run.
    SkippedDependency,
}

impl PatchOutcomeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::Already => "already",
            Self::Failed => "failed",
            Self::SkippedDependency => "skipped_dependency",
        }
    }
}

/// A hash check that failed, with the file and both hashes: the one
/// piece of evidence a player otherwise has to send by hand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashEvidence {
    /// `source_mismatch` (a stock file a patch set rebuilds from is not
    /// stock), `result_mismatch` (a rebuilt file came out wrong) or
    /// `download` (the downloaded blob failed its manifest sha256).
    pub kind: &'static str,
    /// The file (patch set) or the blob label (download).
    pub path: String,
    pub expected_sha256: String,
    pub actual_sha256: String,
}

impl HashEvidence {
    /// The hash evidence an install error carries, if any.
    pub fn of(e: &InstallError) -> Option<Self> {
        match e {
            InstallError::Unpack(UnpackError::PatchsetHash {
                kind,
                path,
                expected,
                actual,
                ..
            }) => Some(Self {
                kind,
                path: path.clone(),
                expected_sha256: expected.clone(),
                actual_sha256: actual.clone(),
            }),
            InstallError::HashMismatch {
                what,
                expected,
                actual,
            } => Some(Self {
                kind: "download",
                path: what.clone(),
                expected_sha256: expected.clone(),
                actual_sha256: actual.clone(),
            }),
            _ => None,
        }
    }
}

/// One patch's outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchOutcome {
    pub id: String,
    pub outcome: PatchOutcomeKind,
    /// Why it failed or was skipped; `None` when it applied.
    pub reason: Option<String>,
    pub evidence: Option<HashEvidence>,
}

/// How the whole run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunResult {
    /// Every patch applied (or already was).
    #[default]
    Ok,
    /// Some patches failed or were skipped; the others applied.
    PatchesFailed,
    /// The player cancelled.
    Cancelled,
    /// The run stopped early (seed download, disk, state file).
    Error,
}

impl RunResult {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::PatchesFailed => "patches_failed",
            Self::Cancelled => "cancelled",
            Self::Error => "error",
        }
    }
}

/// Everything one [`crate::install::install_all`] run did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InstallReport {
    /// `true` when this run downloaded and unpacked the seed.
    pub seed_applied: bool,
    pub patches: Vec<PatchOutcome>,
    pub result: RunResult,
    /// The error text when `result` is `Error`.
    pub error: Option<String>,
    /// Hash evidence for a run that stopped early (a seed hash
    /// mismatch); per-patch evidence lives on each [`PatchOutcome`].
    pub error_evidence: Option<HashEvidence>,
}

impl InstallReport {
    pub fn push(&mut self, id: &str, outcome: PatchOutcomeKind, reason: Option<String>) {
        self.patches.push(PatchOutcome {
            id: id.to_string(),
            outcome,
            reason,
            evidence: None,
        });
    }

    pub fn push_failure(&mut self, id: &str, e: &InstallError) {
        self.patches.push(PatchOutcome {
            id: id.to_string(),
            outcome: PatchOutcomeKind::Failed,
            reason: Some(e.to_string()),
            evidence: HashEvidence::of(e),
        });
    }

    /// Record how the run ended.
    pub fn finish(&mut self, result: &Result<(), InstallError>) {
        self.result = match result {
            Ok(()) => RunResult::Ok,
            Err(InstallError::PatchesFailed(_)) => RunResult::PatchesFailed,
            Err(InstallError::Cancelled) => RunResult::Cancelled,
            Err(e) => {
                self.error = Some(e.to_string());
                self.error_evidence = HashEvidence::of(e);
                RunResult::Error
            }
        };
    }

    /// How many patches ended with `kind`.
    pub fn count(&self, kind: PatchOutcomeKind) -> usize {
        self.patches.iter().filter(|p| p.outcome == kind).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_comes_from_a_patchset_mismatch_and_a_download_mismatch() {
        let e = InstallError::Unpack(UnpackError::PatchsetHash {
            kind: "source_mismatch",
            path: "a.upk".into(),
            expected: "11".into(),
            actual: "22".into(),
            message: "m".into(),
        });
        assert_eq!(
            HashEvidence::of(&e),
            Some(HashEvidence {
                kind: "source_mismatch",
                path: "a.upk".into(),
                expected_sha256: "11".into(),
                actual_sha256: "22".into(),
            })
        );
        let d = InstallError::HashMismatch {
            what: "patch 001".into(),
            expected: "aa".into(),
            actual: "bb".into(),
        };
        assert_eq!(HashEvidence::of(&d).unwrap().kind, "download");
        assert_eq!(HashEvidence::of(&InstallError::Cancelled), None);
    }

    #[test]
    fn finish_classifies_the_run() {
        let mut r = InstallReport::default();
        r.finish(&Err(InstallError::PatchesFailed(Vec::new())));
        assert_eq!(r.result, RunResult::PatchesFailed);
        assert_eq!(r.error, None);
        r.finish(&Err(InstallError::InvalidSha256("zz".into())));
        assert_eq!(r.result, RunResult::Error);
        assert!(r.error.as_deref().unwrap().contains("zz"));
        r.finish(&Err(InstallError::Cancelled));
        assert_eq!(r.result, RunResult::Cancelled);
    }
}
