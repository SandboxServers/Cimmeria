//! Native SGW.exe selection; only the resolved installation root reaches adoption.
use crate::host::AdoptionError;
use std::path::PathBuf;
use tauri_plugin_dialog::DialogExt;

pub async fn pick(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Option<PathBuf>, AdoptionError> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_parent(&window)
            .set_title("Choose SGW.exe in your existing install")
            .set_file_name("SGW.exe")
            .add_filter("SGW.exe (game executable)", &["exe"])
            .blocking_pick_file()
            .map(|file| {
                let selected = file
                    .into_path()
                    .map_err(|_| AdoptionError::InvalidDirectory)?;
                resolve_selection(&selected)
            })
            .transpose()
    })
    .await
    .map_err(|_| AdoptionError::Io)?
}

fn resolve_selection(selected: &std::path::Path) -> Result<PathBuf, AdoptionError> {
    cimmeria_launcher_engine::adoption::resolve_executable(selected)
        .map_err(AdoptionError::from)
        .inspect_err(|error| crate::host::adoption_events::refused(*error, None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{ForwardLayer, LauncherRecord};
    use tracing_subscriber::{layer::SubscriberExt, Registry};

    fn refused_once(path: &std::path::Path) {
        let (tx, rx) = std::sync::mpsc::sync_channel(16);
        tracing::subscriber::with_default(Registry::default().with(ForwardLayer::new(tx)), || {
            assert!(resolve_selection(path).is_err());
        });
        let records: Vec<LauncherRecord> = rx.try_iter().collect();
        let refused: Vec<_> = records
            .iter()
            .filter(|r| r.fields.get("step").and_then(|v| v.as_str()) == Some("refused"))
            .collect();
        assert_eq!(refused.len(), 1);
        assert_eq!(refused[0].fields["decision_outcome"], "refused");
        assert!(!serde_json::to_string(&refused[0].fields)
            .unwrap()
            .contains(&path.to_string_lossy().to_string()));
    }

    #[test]
    fn missing_other_and_flat_executable_picks_emit_one_path_free_refusal() {
        let root = tempfile::tempdir().unwrap();
        refused_once(&root.path().join("missing/SGW.exe"));
        let other = root.path().join("other.exe");
        std::fs::write(&other, b"fixture").unwrap();
        refused_once(&other);
        let flat = root.path().join("SGW.exe");
        std::fs::write(&flat, b"fixture").unwrap();
        refused_once(&flat);
    }

    #[test]
    #[cfg(unix)]
    fn linked_executable_pick_emits_one_path_free_refusal() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("target.exe");
        std::fs::write(&target, b"fixture").unwrap();
        let link = root.path().join("SGW.exe");
        std::os::unix::fs::symlink(target, &link).unwrap();
        refused_once(&link);
    }
}
