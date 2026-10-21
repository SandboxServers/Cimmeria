//! The launcher's panic hook (LT-06, C4 and C5). A panic emits one error on
//! `launcher.crash` with a closed location and leaves a marker the next start
//! reads. The panic message goes to the local log (`launcher.pipeline`) only.
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// The marker file the state directory holds between a panic and the next start.
const MARKER: &str = "last-panic.json";

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Marker {
    ts_ms: u64,
    location: String,
}

/// Chains the existing hook, so the default panic output still happens. Call once,
/// after the subscriber is installed; a test never sets the global hook.
pub(crate) fn install(state_dir: PathBuf) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(target: "launcher.pipeline", "launcher panic: {info}");
        let location = info
            .location()
            .map(|at| (basename(at.file()).to_owned(), at.line()));
        let thread = std::thread::current();
        record_panic(location, thread.name(), &state_dir);
        previous(info);
    }));
}

/// Emits `event=panic` on `launcher.crash` and writes the marker. The location
/// is `<file basename>:<line>`, never a path.
pub(crate) fn record_panic(
    location: Option<(String, u32)>,
    thread: Option<&str>,
    state_dir: &Path,
) {
    let location = match location {
        Some((file, line)) => format!("{file}:{line}"),
        None => "unknown".to_owned(),
    };
    let marker = Marker {
        ts_ms: now_ms(),
        location: location.clone(),
    };
    write_marker(state_dir, &marker);
    tracing::error!(
        target: "launcher.crash",
        event = "panic",
        location = location.as_str(),
        thread = thread.unwrap_or("unknown"),
        "launcher panic: a thread panicked; expected no panic, the next start reports it"
    );
}

/// Reads and deletes the marker a previous run left, returning its location.
/// A marker that cannot be parsed still counts as a panic, located "unknown".
pub(crate) fn take_previous_panic(state_dir: &Path) -> Option<String> {
    let path = state_dir.join(MARKER);
    let bytes = std::fs::read(&path).ok()?;
    // A failed delete only repeats the report at the next start.
    let _ = std::fs::remove_file(&path);
    let location = serde_json::from_slice::<Marker>(&bytes)
        .map_or_else(|_| "unknown".to_owned(), |marker| marker.location);
    Some(location)
}

/// One warn on `launcher.crash` for a panic the previous run recorded.
pub(crate) fn report_previous_panic(location: &str) {
    tracing::warn!(
        target: "launcher.crash",
        event = "previous_run_panicked",
        location = location,
        "launcher crash: the previous run panicked; expected a clean exit, its details stay in the local log"
    );
}

fn write_marker(state_dir: &Path, marker: &Marker) {
    let result = serde_json::to_vec(marker)
        .map_err(io::Error::other)
        .and_then(|json| {
            std::fs::create_dir_all(state_dir)?;
            std::fs::write(state_dir.join(MARKER), json)
        });
    if let Err(error) = result {
        tracing::warn!(target: "launcher.pipeline", %error, "launcher panic: the crash marker was not written");
    }
}

/// The last component of a source path, split on either separator.
fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_capture::capture;
    use tracing::Level;

    #[test]
    fn record_panic_writes_the_marker_and_emits_one_crash_error() {
        let root = tempfile::tempdir().unwrap();
        let ((), events) = capture(|| {
            record_panic(Some(("main.rs".to_owned(), 42)), Some("main"), root.path());
        });
        assert!(events.iter().all(|event| event.target == "launcher.crash"));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].level, Level::ERROR);
        assert_eq!(events[0].field("event"), Some("panic"));
        assert_eq!(events[0].field("location"), Some("main.rs:42"));
        assert_eq!(events[0].field("thread"), Some("main"));
        let bytes = std::fs::read(root.path().join(MARKER)).unwrap();
        let marker: Marker = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(marker.location, "main.rs:42");
        assert!(marker.ts_ms > 0);
    }

    #[test]
    fn take_previous_panic_round_trips_and_deletes_the_marker() {
        let root = tempfile::tempdir().unwrap();
        let ((), _) = capture(|| record_panic(Some(("lib.rs".to_owned(), 7)), None, root.path()));
        assert_eq!(
            take_previous_panic(root.path()),
            Some("lib.rs:7".to_owned())
        );
        assert!(!root.path().join(MARKER).exists());
        assert_eq!(take_previous_panic(root.path()), None);
    }

    #[test]
    fn an_unreadable_marker_is_still_a_previous_panic() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join(MARKER), b"not json").unwrap();
        assert_eq!(take_previous_panic(root.path()), Some("unknown".to_owned()));
    }

    #[test]
    fn a_previous_panic_warns_on_the_crash_target_with_its_location() {
        let ((), events) = capture(|| report_previous_panic("lib.rs:7"));
        assert_eq!(events[0].target, "launcher.crash");
        assert_eq!(events[0].level, Level::WARN);
        assert_eq!(events[0].field("event"), Some("previous_run_panicked"));
        assert_eq!(events[0].field("location"), Some("lib.rs:7"));
    }

    #[test]
    fn basename_drops_both_kinds_of_separator() {
        assert_eq!(basename("src/cell/lib.rs"), "lib.rs");
        assert_eq!(basename("src\\cell\\main.rs"), "main.rs");
        assert_eq!(basename("main.rs"), "main.rs");
    }
}
