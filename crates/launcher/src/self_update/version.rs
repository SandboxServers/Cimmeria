//! Version ordering: when is a launcher release newer than this build, and
//! is this build too old for the manifest's `min_launcher`?
//!
//! # The ordering rule
//!
//! A tag's date (`launcher-YYYYMMDD-<sha>`) is not enough on its own: two
//! releases can be cut on one day, and the short sha carries no order. The
//! release workflow stamps the build time into the binary before it builds
//! (`CIMMERIA_LAUNCHER_BUILD_EPOCH`), and GitHub records when each release
//! was published (`published_at`). A release is newer than the running
//! build when all of these hold:
//!
//! 1. the running build is a stamped release (development builds never
//!    update);
//! 2. the tags differ (the same tag is up to date);
//! 3. the release's tag date is not earlier than the running build's (a
//!    re-published old release never downgrades);
//! 4. the release was published after the running build started building.
//!
//! Rule 4 orders same-day releases. It is sound because the release
//! workflow runs one job at a time (`concurrency: launcher-release`,
//! `cancel-in-progress: false`) and publishes at the end of the job it
//! stamped at the start: every later release starts building, and so is
//! published, after this one was published, and every earlier release was
//! published before this one's build began.

use super::build_info::{tag_date, LauncherBuild};
use super::releases::LauncherRelease;

/// What the update check concluded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateDecision {
    /// Unstamped build: updates are off.
    DevBuild,
    /// No usable launcher release was listed.
    NoRelease,
    /// The newest release is this build, or older than it.
    UpToDate {
        newest: String,
    },
    Available(LauncherRelease),
}

/// True when `release` should replace the running `build`. See the module
/// docs for the rule.
pub fn is_newer(build: &LauncherBuild, release: &LauncherRelease) -> bool {
    let (Some(own_tag), Some(own_date), Some(built_at)) =
        (build.tag.as_deref(), build.date(), build.built_at)
    else {
        return false;
    };
    if release.tag == own_tag {
        return false;
    }
    let Some(release_date) = tag_date(&release.tag) else {
        return false;
    };
    release_date >= own_date && release.published_at > built_at
}

/// Decide from the candidates, newest first (as
/// [`super::releases::launcher_releases`] returns them).
pub fn decide(build: &LauncherBuild, candidates: &[LauncherRelease]) -> UpdateDecision {
    if !build.is_release() {
        return UpdateDecision::DevBuild;
    }
    let Some(newest) = candidates.first() else {
        return UpdateDecision::NoRelease;
    };
    if is_newer(build, newest) {
        UpdateDecision::Available(newest.clone())
    } else {
        UpdateDecision::UpToDate {
            newest: newest.tag.clone(),
        }
    }
}

/// The manifest's `min_launcher` gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MinLauncherGate {
    /// No minimum, or this build meets it.
    Satisfied,
    /// Development builds are never gated, so a developer can always run
    /// against any manifest.
    DevBuildExempt,
    /// The value is not a launcher tag. The launcher does not lock players
    /// out over an operator typo; the check logs a WARN instead.
    Malformed(String),
    /// This build is older than the minimum: installing and launching are
    /// off until the launcher updates.
    TooOld { required: String },
}

impl MinLauncherGate {
    pub fn blocks(&self) -> bool {
        matches!(self, MinLauncherGate::TooOld { .. })
    }
}

/// Check `min_launcher` (a launcher tag, e.g. `launcher-20261002-bbbbbbb`)
/// against the running build.
///
/// A different day decides by date. On the same day the minimum's own
/// release must be in `known` (the last releases list) to order the two
/// by rule 4 above; when it is not (offline, rate limited, not published
/// yet) the build is let through rather than locked out on a guess.
pub fn check_min_launcher(
    build: &LauncherBuild,
    min_launcher: Option<&str>,
    known: &[LauncherRelease],
) -> MinLauncherGate {
    let Some(min) = min_launcher.map(str::trim).filter(|m| !m.is_empty()) else {
        return MinLauncherGate::Satisfied;
    };
    let (Some(own_tag), Some(own_date), Some(built_at)) =
        (build.tag.as_deref(), build.date(), build.built_at)
    else {
        return MinLauncherGate::DevBuildExempt;
    };
    let Some(min_date) = tag_date(min) else {
        return MinLauncherGate::Malformed(min.to_string());
    };
    let too_old = MinLauncherGate::TooOld {
        required: min.to_string(),
    };
    if own_tag == min || own_date > min_date {
        return MinLauncherGate::Satisfied;
    }
    if own_date < min_date {
        return too_old;
    }
    match known.iter().find(|r| r.tag == min) {
        Some(r) if r.published_at > built_at => too_old,
        _ => MinLauncherGate::Satisfied,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_update::releases::ReleaseAsset;

    fn asset(n: &str) -> ReleaseAsset {
        ReleaseAsset {
            name: n.into(),
            url: format!("https://github.com/x/{n}"),
            size: 1,
        }
    }

    fn rel(tag: &str, published_at: i64) -> LauncherRelease {
        LauncherRelease {
            tag: tag.into(),
            published_at,
            page_url: String::new(),
            exe: asset("a.exe"),
            sha256: asset("a.exe.sha256"),
        }
    }

    fn build(tag: &str, built_at: i64) -> LauncherBuild {
        LauncherBuild::from_parts(Some(tag), Some(&built_at.to_string()))
    }

    const A: &str = "launcher-20261002-aaaaaaa";
    const B: &str = "launcher-20261002-bbbbbbb";

    // Two releases on one day: A built at 1000, published at 1600; B built
    // at 2000, published at 2600.
    #[test]
    fn same_day_releases_order_by_publish_time_against_build_time() {
        let a = build(A, 1000);
        let b = build(B, 2000);
        assert!(is_newer(&a, &rel(B, 2600)), "B is newer than A");
        assert!(!is_newer(&b, &rel(A, 1600)), "A must never replace B");
    }

    #[test]
    fn the_same_tag_is_up_to_date() {
        let a = build(A, 1000);
        assert!(!is_newer(&a, &rel(A, 1600)));
        assert_eq!(
            decide(&a, &[rel(A, 1600)]),
            UpdateDecision::UpToDate { newest: A.into() }
        );
    }

    // Bug shape: an old release re-published (fresh published_at) must not
    // be offered to a newer build.
    #[test]
    fn an_older_tag_date_never_downgrades_even_if_published_later() {
        let b = build("launcher-20261005-bbbbbbb", 5000);
        assert!(!is_newer(&b, &rel("launcher-20261001-aaaaaaa", 9000)));
    }

    #[test]
    fn a_later_day_is_newer() {
        let a = build(A, 1000);
        assert!(is_newer(&a, &rel("launcher-20261003-ccccccc", 90_000)));
    }

    #[test]
    fn dev_builds_never_offer_updates() {
        let dev = LauncherBuild::dev();
        assert_eq!(decide(&dev, &[rel(B, 2600)]), UpdateDecision::DevBuild);
        assert!(!is_newer(&dev, &rel(B, 2600)));
    }

    #[test]
    fn decide_offers_the_newest_candidate() {
        let a = build(A, 1000);
        assert_eq!(
            decide(&a, &[rel(B, 2600), rel(A, 1600)]),
            UpdateDecision::Available(rel(B, 2600))
        );
        assert_eq!(decide(&a, &[]), UpdateDecision::NoRelease);
    }

    #[test]
    fn min_launcher_absent_or_met_lets_the_build_through() {
        let b = build(B, 2000);
        assert_eq!(
            check_min_launcher(&b, None, &[]),
            MinLauncherGate::Satisfied
        );
        assert_eq!(
            check_min_launcher(&b, Some(""), &[]),
            MinLauncherGate::Satisfied
        );
        assert_eq!(
            check_min_launcher(&b, Some(B), &[]),
            MinLauncherGate::Satisfied
        );
        assert_eq!(
            check_min_launcher(&b, Some("launcher-20260929-f518b57"), &[]),
            MinLauncherGate::Satisfied
        );
        assert_eq!(
            check_min_launcher(&b, Some(A), &[rel(A, 1600)]),
            MinLauncherGate::Satisfied,
            "A was published before B was built"
        );
    }

    #[test]
    fn min_launcher_blocks_an_older_build() {
        let a = build(A, 1000);
        let gate = check_min_launcher(&a, Some("launcher-20261003-ccccccc"), &[]);
        assert!(gate.blocks(), "{gate:?}");
        let gate = check_min_launcher(&a, Some(B), &[rel(B, 2600)]);
        assert!(
            gate.blocks(),
            "same day, B published after A was built: {gate:?}"
        );
    }

    #[test]
    fn min_launcher_same_day_unknown_release_does_not_lock_out() {
        let a = build(A, 1000);
        assert_eq!(
            check_min_launcher(&a, Some(B), &[]),
            MinLauncherGate::Satisfied
        );
    }

    #[test]
    fn min_launcher_never_gates_a_dev_build_and_tolerates_typos() {
        assert_eq!(
            check_min_launcher(&LauncherBuild::dev(), Some(B), &[]),
            MinLauncherGate::DevBuildExempt
        );
        let gate = check_min_launcher(&build(A, 1000), Some("2026-10-02"), &[]);
        assert_eq!(gate, MinLauncherGate::Malformed("2026-10-02".into()));
        assert!(!gate.blocks());
    }
}
