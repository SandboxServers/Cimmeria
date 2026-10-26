//! Whether a cache copy is current, needs writing, or is ahead of the source.
//!
//! The rules mirror the client's startup copy: the cache version is the
//! higher of its own version and the source's, and a copy is written only when
//! that effective version is not the one the server serves. Versions compare
//! as unsigned integers (D-LP15).

/// What the presync does with one category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// The effective cache version is the served version. Nothing to write.
    UpToDate,
    /// The cache copy must be composed from the server's body.
    Write,
    /// The source copy is newer than the served version. Play is blocked.
    SourceNewer,
}

/// The version the client will hold after its startup copy.
///
/// With no cache copy, the source's version. Otherwise the source's version
/// when it is higher than the cache's, else the cache's.
pub fn effective_version(cache: Option<u32>, source: Option<u32>) -> Option<u32> {
    match cache {
        None => source,
        Some(cache) => match source {
            Some(source) if source > cache => Some(source),
            _ => Some(cache),
        },
    }
}

/// Decides the action for a category the server serves at `served`.
pub fn decide(cache: Option<u32>, source: Option<u32>, served: u32) -> Action {
    if effective_version(cache, source) == Some(served) {
        Action::UpToDate
    } else if source.is_some_and(|source| source > served) {
        Action::SourceNewer
    } else {
        Action::Write
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decide_table() {
        let cases: &[(Option<u32>, Option<u32>, u32, Action)] = &[
            (None, None, 5, Action::Write),
            (Some(5), None, 5, Action::UpToDate),
            (Some(4), Some(5), 5, Action::UpToDate),
            (Some(5), Some(6), 5, Action::SourceNewer),
            (Some(6), Some(4), 5, Action::Write),
            (Some(!5), Some(4), 5, Action::Write),
            (Some(u32::MAX), None, 1, Action::Write),
        ];
        for &(cache, source, served, expected) in cases {
            assert_eq!(
                decide(cache, source, served),
                expected,
                "cache {cache:?}, source {source:?}, served {served}"
            );
        }
    }

    #[test]
    fn effective_version_takes_the_higher_source_or_the_cache() {
        assert_eq!(effective_version(None, Some(3)), Some(3));
        assert_eq!(effective_version(None, None), None);
        assert_eq!(effective_version(Some(4), Some(5)), Some(5));
        assert_eq!(effective_version(Some(6), Some(5)), Some(6));
        assert_eq!(effective_version(Some(6), None), Some(6));
    }
}
