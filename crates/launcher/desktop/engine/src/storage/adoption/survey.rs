//! Explains a refused scan: which entries of the folder cannot be adopted safely, and why (launcher-adopt-ux D-AU4). Read only and bounded; it runs only after the scan refused the folder, and never descends into an offender.
use super::*;
use std::collections::BTreeMap;

pub(super) const MAX_UNSAFE_SHOWN: usize = 5;
const MAX_UNSAFE_LOGGED: u32 = 100;
const MAX_PATH_SHOWN: usize = 200;

struct Survey {
    report: UnsafeReport,
    seen: usize,
    folded: BTreeMap<String, String>,
}
impl Survey {
    fn note(&mut self, reason: UnsafeReason, path: Option<String>) {
        if self.report.total < MAX_UNSAFE_LOGGED {
            tracing::warn!(target: "adoption", ?reason,
                path = %path.as_deref().unwrap_or("(the folder's location)"),
                "adoption refused: an entry cannot be adopted safely");
        }
        self.report.total = self.report.total.saturating_add(1);
        if self.report.entries.len() < MAX_UNSAFE_SHOWN {
            self.report.entries.push(UnsafeEntry { reason, path });
        }
    }
}
/// The entries of `root` that cannot be adopted safely. Read only and bounded.
pub(super) fn survey(root: &Path) -> UnsafeReport {
    let mut survey = Survey {
        report: UnsafeReport {
            entries: Vec::new(),
            total: 0,
        },
        seen: 0,
        folded: BTreeMap::new(),
    };
    match inventory::unsafe_reason(root) {
        Some(reason) => survey.note(reason, Some(".".to_owned())),
        None => walk(root, root, 0, &mut survey),
    }
    if survey.report.total > MAX_UNSAFE_LOGGED {
        tracing::warn!(target: "adoption", total = survey.report.total,
            "adoption refused: more unsafe entries were not logged");
    }
    survey.report
}
fn walk(root: &Path, dir: &Path, depth: usize, survey: &mut Survey) {
    if depth > inventory::MAX_DEPTH {
        return;
    }
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut children: Vec<PathBuf> = read
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    children.sort();
    for child in children {
        survey.seen += 1;
        if survey.seen > inventory::MAX_FILES {
            return;
        }
        let Ok(relative) = child.strip_prefix(root) else {
            continue;
        };
        let shown_path = Some(shown(relative));
        if let Some(reason) = inventory::unsafe_reason(&child) {
            survey.note(reason, shown_path);
            continue;
        }
        // Index names use `/` on every host, as the scan does.
        let name = relative
            .components()
            .map(|part| match part {
                std::path::Component::Normal(part) => part.to_str().map(str::to_owned),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
            .map(|parts| parts.join("/"))
            .filter(|name| inventory::index_name_allowed(name));
        let Some(name) = name else {
            survey.note(UnsafeReason::Name, shown_path);
            continue;
        };
        if survey
            .folded
            .insert(name.to_ascii_lowercase(), name.clone())
            .is_some()
        {
            survey.note(UnsafeReason::CaseCollision, shown_path);
            continue;
        }
        if std::fs::symlink_metadata(&child).is_ok_and(|metadata| metadata.is_dir()) {
            walk(root, &child, depth + 1, survey);
        }
    }
}
/// `relative`, `/`-joined, lossy, cut to MAX_PATH_SHOWN chars plus `…`.
pub(super) fn shown(relative: &Path) -> String {
    let joined = relative
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");
    if joined.chars().count() > MAX_PATH_SHOWN {
        let mut cut: String = joined.chars().take(MAX_PATH_SHOWN).collect();
        cut.push('…');
        cut
    } else {
        joined
    }
}
/// The error for a scan of `root` that failed with a bare `UnsafeFile`:
/// `Error::Unsafe(report)`, or the bare `UnsafeFile` again when the survey finds nothing.
pub(super) fn explain(root: &Path) -> Error {
    let report = survey(root);
    if report.total == 0 {
        tracing::warn!(target: "adoption",
            "adoption refused: an entry changed or could not be checked");
        return StorageError::UnsafeFile.into();
    }
    Error::Unsafe(Box::new(report))
}
/// `result` with a bare `Err(Error::Storage(StorageError::UnsafeFile))` replaced by `explain(root)`.
pub(super) fn explained<T>(root: &Path, result: Result<T, Error>) -> Result<T, Error> {
    match result {
        Err(Error::Storage(StorageError::UnsafeFile)) => Err(explain(root)),
        other => other,
    }
}
