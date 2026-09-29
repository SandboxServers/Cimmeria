//! What this launcher binary knows about its own release.
//!
//! The release workflow (`.github/workflows/launcher-release.yml`) stamps
//! the tag and the build time BEFORE it builds the launcher, and passes them
//! in as `CIMMERIA_LAUNCHER_TAG` (`launcher-YYYYMMDD-<short sha>`) and
//! `CIMMERIA_LAUNCHER_BUILD_EPOCH` (Unix seconds, UTC). Local, dev and PR
//! builds carry neither, so they are development builds and never update
//! themselves.

/// Prefix every launcher release tag carries. Server releases (`v2026-…`)
/// and the `content-current` prerelease share the repository, so release
/// discovery filters on it.
pub const LAUNCHER_TAG_PREFIX: &str = "launcher-";

/// The running build's release identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherBuild {
    /// `launcher-YYYYMMDD-<sha>`, or `None` for a development build.
    pub tag: Option<String>,
    /// When the release workflow started this build, Unix seconds UTC.
    pub built_at: Option<i64>,
}

impl LauncherBuild {
    /// The identity compiled into this binary.
    pub fn current() -> Self {
        Self::from_parts(
            option_env!("CIMMERIA_LAUNCHER_TAG"),
            option_env!("CIMMERIA_LAUNCHER_BUILD_EPOCH"),
        )
    }

    /// Build the identity from the two compile-time strings. A value that
    /// is missing, blank (GitHub Actions expands an unset input to "") or
    /// malformed leaves the build a development build: an update decision
    /// made on a half-parsed identity could downgrade.
    pub fn from_parts(tag: Option<&str>, epoch: Option<&str>) -> Self {
        let tag = tag
            .map(str::trim)
            .filter(|t| tag_date(t).is_some())
            .map(str::to_string);
        let built_at = epoch
            .map(str::trim)
            .and_then(|e| e.parse::<i64>().ok())
            .filter(|e| *e > 0);
        if tag.is_none() || built_at.is_none() {
            return Self::dev();
        }
        Self { tag, built_at }
    }

    pub fn dev() -> Self {
        Self {
            tag: None,
            built_at: None,
        }
    }

    /// True for a build the release workflow stamped. Only those update.
    pub fn is_release(&self) -> bool {
        self.tag.is_some() && self.built_at.is_some()
    }

    /// The version as the window shows it.
    pub fn display(&self) -> String {
        match &self.tag {
            Some(t) => t.clone(),
            None => "development build".into(),
        }
    }

    /// The tag's `YYYYMMDD`, for ordering.
    pub fn date(&self) -> Option<u32> {
        self.tag.as_deref().and_then(tag_date)
    }
}

/// The `YYYYMMDD` of a `launcher-YYYYMMDD-<sha>` tag, or `None` when the
/// string is not a launcher tag.
pub fn tag_date(tag: &str) -> Option<u32> {
    let rest = tag.strip_prefix(LAUNCHER_TAG_PREFIX)?;
    let (date, sha) = rest.split_once('-')?;
    if date.len() != 8 || !date.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if sha.is_empty() || !sha.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    date.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stamped_build_is_a_release() {
        let b = LauncherBuild::from_parts(Some("launcher-20260929-f518b57"), Some("1790000000"));
        assert!(b.is_release());
        assert_eq!(b.date(), Some(20260929));
        assert_eq!(b.display(), "launcher-20260929-f518b57");
    }

    // Bug shape: a build without the stamp (local, PR, or a release job
    // whose stamp step was skipped) must never be treated as a release, or
    // it would offer to "update" a developer's own build.
    #[test]
    fn missing_blank_or_malformed_stamps_make_a_dev_build() {
        for (tag, epoch) in [
            (None, None),
            (Some(""), Some("")),
            (Some("launcher-20260929-f518b57"), None),
            (Some("launcher-20260929-f518b57"), Some("")),
            (Some("launcher-20260929-f518b57"), Some("abc")),
            (None, Some("1790000000")),
            (Some("v2026-09-29"), Some("1790000000")),
            (Some("launcher-2026929-f518b57"), Some("1790000000")),
            (Some("launcher-20260929-"), Some("1790000000")),
            (Some("launcher-20260929-../x"), Some("1790000000")),
        ] {
            let b = LauncherBuild::from_parts(tag, epoch);
            assert!(!b.is_release(), "{tag:?} {epoch:?} must be a dev build");
            assert_eq!(b.display(), "development build");
        }
    }

    #[test]
    fn tag_date_rejects_other_release_families() {
        assert_eq!(tag_date("content-current"), None);
        assert_eq!(tag_date("v2026-09-29-abcdef0"), None);
        assert_eq!(tag_date("launcher-20261001-abc1234"), Some(20261001));
    }
}
