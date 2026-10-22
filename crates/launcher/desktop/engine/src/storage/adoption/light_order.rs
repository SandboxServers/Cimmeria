//! Which index files the light review keeps when the list is capped.

use super::model::LIGHT_LIST_CAP;
use cimmeria_launcher_core::file_index::FileClass;

/// Keeps the first `LIGHT_LIST_CAP` files, the actionable ones first.
///
/// `IndexComparison.files` is sorted by path, so on a folder with many
/// already-patched files the Changed and Missing rows could fall past the cap
/// and show only as "and N more". Changed comes first, then Missing, then the
/// other classes; each group stays sorted by path. The caller keeps the full
/// count for `index_files_total`.
pub(super) fn capped_index_files(mut files: Vec<(String, FileClass)>) -> Vec<(String, FileClass)> {
    files.sort_by(|(a_path, a_class), (b_path, b_class)| {
        rank(*a_class)
            .cmp(&rank(*b_class))
            .then_with(|| a_path.cmp(b_path))
    });
    files.truncate(LIGHT_LIST_CAP);
    files
}

fn rank(class: FileClass) -> u8 {
    match class {
        FileClass::Changed => 0,
        FileClass::Missing => 1,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// More than the cap of non-stock files, with the Changed and Missing rows
    /// sorting after every Patched and Extra path.
    #[test]
    fn changed_and_missing_survive_the_cap_ahead_of_other_classes() {
        let mut files: Vec<(String, FileClass)> = (0..LIGHT_LIST_CAP + 20)
            .map(|i| {
                let class = if i % 2 == 0 {
                    FileClass::Patched
                } else {
                    FileClass::Extra
                };
                (format!("Working/a{i:04}.upk"), class)
            })
            .collect();
        files.push(("Working/z-changed-2.upk".into(), FileClass::Changed));
        files.push(("Working/z-missing.upk".into(), FileClass::Missing));
        files.push(("Working/z-changed-1.upk".into(), FileClass::Changed));
        files.sort();
        let total = files.len();
        assert!(total > LIGHT_LIST_CAP + 2);

        let kept = capped_index_files(files);

        assert_eq!(kept.len(), LIGHT_LIST_CAP);
        let head: Vec<(&str, FileClass)> =
            kept[..3].iter().map(|(p, c)| (p.as_str(), *c)).collect();
        assert_eq!(
            head,
            [
                ("Working/z-changed-1.upk", FileClass::Changed),
                ("Working/z-changed-2.upk", FileClass::Changed),
                ("Working/z-missing.upk", FileClass::Missing),
            ]
        );
        // The rest follow by path.
        assert_eq!(kept[3].0, "Working/a0000.upk");
        assert_eq!(kept[4].0, "Working/a0001.upk");
    }
}
