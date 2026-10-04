//! Desktop package identity is independent of the legacy game compatibility tag.
//! This policy deliberately preserves legacy development and unknown-order exemptions.
use crate::catalog::VerifiedRelease;
use serde::Serialize;

/// Native build metadata, never renderer-supplied admission input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Identity {
    pub desktop_version: String,
    pub source_revision: Option<String>,
    pub compatibility_tag: Option<String>,
    pub built_at: Option<i64>,
}
impl Identity {
    pub fn current() -> Self {
        Self::from_parts(
            env!("CARGO_PKG_VERSION"),
            option_env!("CIMMERIA_DESKTOP_SOURCE_REVISION"),
            option_env!("CIMMERIA_LAUNCHER_TAG"),
            option_env!("CIMMERIA_LAUNCHER_BUILD_EPOCH"),
        )
    }
    /// Missing or malformed stamps have the legacy development-build semantics.
    /// Package version is display metadata, never used to order compatibility tags.
    pub fn from_parts(
        version: &str,
        source: Option<&str>,
        tag: Option<&str>,
        epoch: Option<&str>,
    ) -> Self {
        let tag = tag.map(str::trim).filter(|tag| tag_date(tag).is_some());
        let epoch = epoch
            .and_then(|value| value.trim().parse::<i64>().ok())
            .filter(|epoch| *epoch > 0);
        let release = tag.is_some() && epoch.is_some();
        Self {
            desktop_version: version.into(),
            source_revision: source.map(str::to_owned),
            compatibility_tag: if release {
                tag.map(str::to_owned)
            } else {
                None
            },
            built_at: if release { epoch } else { None },
        }
    }
}

/// Metadata from a native trusted release source. The date/hash is not SemVer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownRelease {
    pub tag: String,
    pub published_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum MinimumStatus {
    Satisfied,
    DevelopmentExempt,
    Malformed {
        required: String,
    },
    /// Allowed for parity, but explicitly distinguished from proven satisfaction.
    UnknownSameDay {
        required: String,
    },
    TooOld {
        required: String,
    },
}
impl MinimumStatus {
    pub fn blocks(&self) -> bool {
        matches!(self, Self::TooOld { .. })
    }
}

/// Immutable for the lifetime of the native state owner; not deserializable from IPC.
#[derive(Debug, Clone)]
pub struct CompatibilityPolicy {
    identity: Identity,
    known: Vec<KnownRelease>,
}
impl Default for CompatibilityPolicy {
    fn default() -> Self {
        Self::new(Identity::current(), Vec::new())
    }
}
impl CompatibilityPolicy {
    pub fn new(identity: Identity, known: Vec<KnownRelease>) -> Self {
        Self { identity, known }
    }
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
    pub fn for_release(&self, release: &VerifiedRelease) -> MinimumStatus {
        self.check(release.manifest().min_launcher.as_deref())
    }
    pub fn check(&self, minimum: Option<&str>) -> MinimumStatus {
        let Some(required) = minimum.map(str::trim).filter(|value| !value.is_empty()) else {
            return MinimumStatus::Satisfied;
        };
        let (Some(own_tag), Some(own_date), Some(built_at)) = (
            self.identity.compatibility_tag.as_deref(),
            self.identity
                .compatibility_tag
                .as_deref()
                .and_then(tag_date),
            self.identity.built_at.filter(|epoch| *epoch > 0),
        ) else {
            return MinimumStatus::DevelopmentExempt;
        };
        let Some(date) = tag_date(required) else {
            return MinimumStatus::Malformed {
                required: required.into(),
            };
        };
        if required == own_tag || date < own_date {
            return MinimumStatus::Satisfied;
        }
        if date > own_date {
            return MinimumStatus::TooOld {
                required: required.into(),
            };
        }
        match self.known.iter().find(|release| release.tag == required) {
            Some(release) if release.published_at > built_at => MinimumStatus::TooOld {
                required: required.into(),
            },
            Some(_) => MinimumStatus::Satisfied,
            None => MinimumStatus::UnknownSameDay {
                required: required.into(),
            },
        }
    }
}

// Intentionally matches the legacy parser (eight decimal digits and an
// alphanumeric suffix), including its lack of calendar-date validation.
fn tag_date(tag: &str) -> Option<u32> {
    let (date, suffix) = tag.strip_prefix("launcher-")?.split_once('-')?;
    if date.len() != 8
        || !date.bytes().all(|b| b.is_ascii_digit())
        || suffix.is_empty()
        || !suffix.bytes().all(|b| b.is_ascii_alphanumeric())
    {
        return None;
    }
    date.parse().ok()
}

#[cfg(test)]
mod tests;
