//! The failures the lab already knows, tagged by signature. One table for
//! `lab watch` and `lab uat` (LX-15 to LX-17).

use std::sync::OnceLock;

use regex::Regex;

/// One known failure: the first row and reason that match both patterns get
/// its tag.
#[derive(Debug, Clone, Copy)]
pub struct KnownIssue {
    /// The issue tag, e.g. `#1341`.
    pub issue: &'static str,
    /// Regex for the failing row id.
    pub rows: &'static str,
    /// Regex for the failing reason.
    pub reason: &'static str,
    /// What the failure is.
    pub note: &'static str,
}

/// The table, in match order.
pub const KNOWN_ISSUES: &[KnownIssue] = &[
    KnownIssue {
        issue: "#1341",
        rows: "^FS-[PS]2$",
        reason: "movie-over",
        note: "client dropped the first-login entity batch (iterator residue)",
    },
    KnownIssue {
        issue: "#1343",
        rows: ".",
        reason: "Login_PasswordEdit holds 0 characters",
        note: "login password box read back empty on a slow-booting client",
    },
];

fn compiled() -> &'static [(Regex, Regex)] {
    static TABLE: OnceLock<Vec<(Regex, Regex)>> = OnceLock::new();
    TABLE.get_or_init(|| {
        KNOWN_ISSUES
            .iter()
            .map(|k| {
                (
                    Regex::new(&format!("(?i){}", k.rows)).expect("known issue row pattern"),
                    Regex::new(&format!("(?i){}", k.reason)).expect("known issue reason pattern"),
                )
            })
            .collect()
    })
}

/// The issue tag for a failing row and reason: the first entry whose row and
/// reason patterns both match, else `None`.
pub fn tag(row: &str, reason: &str) -> Option<&'static str> {
    KNOWN_ISSUES
        .iter()
        .zip(compiled())
        .find(|(_, (rows, why))| rows.is_match(row) && why.is_match(reason))
        .map(|(known, _)| known.issue)
}

#[cfg(test)]
mod tests {
    use super::tag;

    #[test]
    fn movie_over_on_fs_p2_and_fs_s2_is_1341() {
        assert_eq!(
            tag(
                "FS-P2",
                "clause movie-over failed: client_entity_find/count gte 1"
            ),
            Some("#1341")
        );
        assert_eq!(tag("FS-S2", "movie-over"), Some("#1341"));
    }

    #[test]
    fn the_row_pattern_is_anchored() {
        assert_eq!(tag("FS-P3", "movie-over"), None);
        assert_eq!(tag("XFS-P2", "movie-over"), None);
    }

    #[test]
    fn an_empty_password_box_on_any_row_is_1343() {
        assert_eq!(
            tag("FS-01", "setup failed: lab_login: Login_PasswordEdit holds 0 characters after typing, expected 4"),
            Some("#1343")
        );
    }

    #[test]
    fn matching_ignores_case_and_an_empty_row_matches_nothing() {
        assert_eq!(tag("fs-p2", "MOVIE-OVER"), Some("#1341"));
        assert_eq!(tag("", "Login_PasswordEdit holds 0 characters"), None);
        assert_eq!(tag("FS-01", "something else"), None);
    }
}
