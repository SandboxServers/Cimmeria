//! Shell-originated adoption refusals: logged like engine rejections (C14), never stored.
use super::{events, AdoptionError};
use cimmeria_launcher_engine::adoption;

/// Logs `adoption_rejected` with `check` and the closed phrase `found`, and returns `error`.
pub(crate) fn refusal(
    error: AdoptionError,
    check: &'static str,
    found: &'static str,
) -> AdoptionError {
    adoption::refuse(
        &events::code(error),
        &adoption::Rejection::new(check).found(found),
    );
    error
}

#[cfg(test)]
mod tests {
    use super::super::fixture::*;
    use super::*;
    use crate::telemetry::{ForwardLayer, LauncherRecord};
    use tracing::subscriber::with_default;
    use tracing_subscriber::{layer::SubscriberExt, Registry};

    #[test]
    fn refusal_logs_the_check_and_returns_the_error() {
        let (tx, rx) = std::sync::mpsc::sync_channel(16);
        let returned = with_default(Registry::default().with(ForwardLayer::new(tx)), || {
            refusal(
                AdoptionError::ImportRequired,
                "import_required",
                "no_saved_import",
            )
        });
        assert_eq!(returned, AdoptionError::ImportRequired);
        let records: Vec<LauncherRecord> = rx.try_iter().collect();
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.level, "warn");
        assert_eq!(record.fields["event"], "adoption_rejected");
        assert_eq!(record.fields["check"], "import_required");
        assert_eq!(record.fields["code"], "import_required");
        assert_eq!(record.fields["found"], "no_saved_import");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_shell_refusal_logs_its_check() {
        let mut f = Fixture::new(serde_json::json!({}));
        f.serve_from("http://127.0.0.1:9/manifest.json".into());
        std::fs::create_dir_all(f.location().join(super::super::COPY_FOLDER)).unwrap();
        let (tx, rx) = std::sync::mpsc::sync_channel(16);
        let result = with_default(Registry::default().with(ForwardLayer::new(tx)), || {
            f.begin()
        });
        assert_eq!(result.unwrap_err(), AdoptionError::InvalidDirectory);
        let records: Vec<LauncherRecord> = rx
            .try_iter()
            .filter(|record| record.fields["event"] == "adoption_rejected")
            .collect();
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.fields["check"], "invalid_directory");
        assert_eq!(record.fields["code"], "invalid_directory");
        assert_eq!(record.fields["found"], "copy_folder_exists");
    }
}
