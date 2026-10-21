//! Adoption step events (C10). Each step is one tracing record with a fixed step
//! code and decision outcome. Every other value is a serde code or a relative
//! path, never free text (Rec7). `ForwardLayer` (C9) sends them to the server.
use super::AdoptionError;
use cimmeria_launcher_engine::adoption::UnsafeReport;

/// The mode a step names: `in_place` or `copy`.
fn mode(in_place: bool) -> &'static str {
    if in_place {
        "in_place"
    } else {
        "copy"
    }
}

/// The serde code of `value` (snake_case), or `"unknown"` when it has none.
fn code<T: serde::Serialize>(value: T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(text)) => text,
        _ => "unknown".to_owned(),
    }
}

/// The first five entries as `reason:path`, joined by `; `. A missing path is
/// `(location)`.
fn unsafe_entries(report: &UnsafeReport) -> String {
    report
        .entries
        .iter()
        .take(5)
        .map(|entry| {
            let path = entry.path.as_deref().unwrap_or("(location)");
            format!("{}:{path}", code(entry.reason))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// The player's mode choice passed the mode check.
pub(crate) fn mode_chosen(in_place: bool) {
    tracing::info!(
        target: "adoption",
        step = "mode_chosen",
        decision_outcome = mode(in_place),
        "adoption step"
    );
}

/// A folder was selected for the mode, or the picker was cancelled.
pub(crate) fn folder_chosen(in_place: bool, selected: bool) {
    let outcome = if selected {
        "selected"
    } else {
        "picker_cancelled"
    };
    tracing::info!(
        target: "adoption",
        step = "folder_chosen",
        decision_outcome = outcome,
        mode = mode(in_place),
        "adoption step"
    );
}

/// The review of the folder is shown.
pub(crate) fn review_ready() {
    tracing::info!(
        target: "adoption",
        step = "review_ready",
        decision_outcome = "review_shown",
        "adoption step"
    );
}

/// The folder is refused. `entries` names the unsafe entries when there are any.
pub(crate) fn refused(error: AdoptionError, entries: Option<&UnsafeReport>) {
    let (total, shown) = match entries {
        Some(report) => (u64::from(report.total), unsafe_entries(report)),
        None => (0, String::new()),
    };
    let error_code = code(error);
    tracing::warn!(
        target: "adoption",
        step = "refused",
        decision_outcome = "refused",
        error = error_code.as_str(),
        unsafe_total = total,
        unsafe_entries = shown.as_str(),
        "adoption step"
    );
}

/// The confirmation was accepted and the copy or move has started.
pub(crate) fn confirmed(in_place: bool) {
    tracing::info!(
        target: "adoption",
        step = "confirmed",
        decision_outcome = "confirm_sent",
        mode = mode(in_place),
        "adoption step"
    );
}

/// The adoption ended. It is a warning, with its error code, when it failed.
pub(crate) fn finished(in_place: bool, error: Option<AdoptionError>) {
    match error {
        Some(error) => {
            let error_code = code(error);
            tracing::warn!(
                target: "adoption",
                step = "finished",
                decision_outcome = "failed",
                mode = mode(in_place),
                error = error_code.as_str(),
                "adoption step"
            );
        }
        None => tracing::info!(
            target: "adoption",
            step = "finished",
            decision_outcome = "adopted",
            mode = mode(in_place),
            "adoption step"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{ForwardLayer, LauncherRecord};
    use cimmeria_launcher_engine::adoption::{UnsafeEntry, UnsafeReason};
    use tracing::subscriber::with_default;
    use tracing_subscriber::{layer::SubscriberExt, Registry};

    #[test]
    fn every_step_carries_its_contract_fields() {
        let (tx, rx) = std::sync::mpsc::sync_channel(16);
        let report = UnsafeReport {
            entries: vec![
                UnsafeEntry {
                    reason: UnsafeReason::HardLink,
                    path: Some("later.txt".into()),
                },
                UnsafeEntry {
                    reason: UnsafeReason::Link,
                    path: None,
                },
            ],
            total: 2,
        };
        with_default(Registry::default().with(ForwardLayer::new(tx)), || {
            mode_chosen(true);
            folder_chosen(true, false);
            review_ready();
            refused(AdoptionError::UnsafeFile, Some(&report));
            confirmed(false);
            finished(true, None);
            finished(false, Some(AdoptionError::UnsafeFile));
        });
        let records: Vec<LauncherRecord> = rx.try_iter().collect();
        assert_eq!(records.len(), 7);

        let step = |index: usize, step: &str, outcome: &str, level: &str| {
            let record = &records[index];
            assert_eq!(record.level, level, "level of {step}");
            assert_eq!(record.fields["step"], step);
            assert_eq!(record.fields["decision_outcome"], outcome);
        };
        step(0, "mode_chosen", "in_place", "info");
        step(1, "folder_chosen", "picker_cancelled", "info");
        assert_eq!(records[1].fields["mode"], "in_place");
        step(2, "review_ready", "review_shown", "info");
        step(3, "refused", "refused", "warn");
        assert_eq!(records[3].fields["error"], "unsafe_file");
        assert_eq!(records[3].fields["unsafe_total"], 2);
        assert_eq!(
            records[3].fields["unsafe_entries"],
            "hard_link:later.txt; link:(location)"
        );
        step(4, "confirmed", "confirm_sent", "info");
        assert_eq!(records[4].fields["mode"], "copy");
        step(5, "finished", "adopted", "info");
        assert_eq!(records[5].fields["mode"], "in_place");
        assert!(!records[5].fields.contains_key("error"));
        step(6, "finished", "failed", "warn");
        assert_eq!(records[6].fields["mode"], "copy");
        assert_eq!(records[6].fields["error"], "unsafe_file");
    }
}
