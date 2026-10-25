//! The `ui_event` command (C13): a frontend error that was swallowed reaches the
//! launcher's telemetry as one `launcher.ui` record. The input is closed: an area
//! from `UI_AREAS` and snake_case codes, never free text. At most 60 events are
//! accepted per minute; the rest are counted into the next accepted one.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

/// The screens and subsystems the frontend may name (C13).
pub(crate) const UI_AREAS: [&str; 17] = [
    "install",
    "adoption",
    "update",
    "game_update",
    "launch",
    "repair",
    "settings",
    "seed_archive",
    "server_list",
    "log_upload",
    "physx",
    "sign_in",
    "migration",
    "patch_notes",
    "client_patches",
    "client_maintenance",
    "telemetry",
];
/// Events accepted in one window.
const LIMIT: u64 = 60;
/// The window length in milliseconds.
const WINDOW_MS: u64 = 60_000;
static THROTTLE: Throttle = Throttle::new();

/// The throttle state: when the window opened, how many events it admitted, and how
/// many were turned away since the last admitted one.
struct Throttle {
    start_ms: AtomicU64,
    count: AtomicU64,
    suppressed: AtomicU64,
}

impl Throttle {
    const fn new() -> Self {
        Self {
            start_ms: AtomicU64::new(0),
            count: AtomicU64::new(0),
            suppressed: AtomicU64::new(0),
        }
    }

    /// Admits one event at `now_ms`. Returns the events turned away since the last
    /// admitted one, or None when this event is over the window's limit.
    fn admit(&self, now_ms: u64) -> Option<u64> {
        if now_ms.saturating_sub(self.start_ms.load(Ordering::Relaxed)) >= WINDOW_MS {
            self.start_ms.store(now_ms, Ordering::Relaxed);
            self.count.store(0, Ordering::Relaxed);
        }
        if self.count.fetch_add(1, Ordering::Relaxed) < LIMIT {
            return Some(self.suppressed.swap(0, Ordering::Relaxed));
        }
        self.suppressed.fetch_add(1, Ordering::Relaxed);
        None
    }
}

/// True when `text` matches `^[a-z][a-z0-9_]{0,39}$`, checked by hand.
fn is_code(text: &str) -> bool {
    let mut chars = text.chars();
    text.len() <= 40
        && matches!(chars.next(), Some('a'..='z'))
        && chars.all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_'))
}

/// Milliseconds since the first call in this process.
fn now_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    let start = START.get_or_init(Instant::now);
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Reports one frontend event. Invalid input is refused with `invalid ui event` and
/// a local debug record; valid input over the limit is counted and returns Ok.
#[tauri::command]
pub(crate) fn ui_event(
    area: String,
    event: String,
    code: String,
    level: String,
) -> Result<(), String> {
    report(&THROTTLE, now_ms(), &area, &event, &code, &level)
}

/// The validated, throttled core of `ui_event`, taking its state and clock so tests
/// can drive a window.
fn report(
    throttle: &Throttle,
    now_ms: u64,
    area: &str,
    event: &str,
    code: &str,
    level: &str,
) -> Result<(), String> {
    let valid_level = matches!(level, "info" | "warn");
    if !UI_AREAS.contains(&area) || !is_code(event) || !is_code(code) || !valid_level {
        tracing::debug!(
            target: "launcher.ui",
            reason = "invalid_input",
            "launcher UI event refused: the area, event, code or level is not allowed"
        );
        return Err("invalid ui event".to_owned());
    }
    let Some(suppressed) = throttle.admit(now_ms) else {
        return Ok(());
    };
    if level == "info" {
        tracing::info!(
            target: "launcher.ui",
            event,
            area,
            code,
            suppressed,
            "launcher UI: {area} reported {code}"
        );
    } else {
        tracing::warn!(
            target: "launcher.ui",
            event,
            area,
            code,
            suppressed,
            "launcher UI: {area} reported {code}"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_capture::{capture, Captured};
    use tracing::Level;

    /// The `launcher.ui` records that were not debug, in order.
    fn forwarded(captured: &[Captured]) -> Vec<&Captured> {
        captured
            .iter()
            .filter(|c| c.target == "launcher.ui" && c.level != Level::DEBUG)
            .collect()
    }

    #[test]
    fn valid_input_gives_one_event() {
        let throttle = Throttle::new();
        let (result, captured) =
            capture(|| report(&throttle, 0, "install", "step_failed", "disk_full", "warn"));
        assert_eq!(result, Ok(()));
        let events = forwarded(&captured);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].level, Level::WARN);
        assert_eq!(events[0].field("event"), Some("step_failed"));
        assert_eq!(events[0].field("area"), Some("install"));
        assert_eq!(events[0].field("code"), Some("disk_full"));
        assert_eq!(events[0].field("suppressed"), Some("0"));

        let (_, captured) = capture(|| report(&throttle, 1, "telemetry", "ping", "ok", "info"));
        let events = forwarded(&captured);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].level, Level::INFO);
    }

    #[test]
    fn sign_in_areas_are_accepted() {
        // The sign-in and account panels both report under `sign_in` (frontend UiArea);
        // without it in UI_AREAS the native side refuses their telemetry.
        assert!(UI_AREAS.contains(&"sign_in"));
        let throttle = Throttle::new();
        let (result, captured) = capture(|| {
            report(
                &throttle,
                0,
                "sign_in",
                "sign_in_failed",
                "rate_limited",
                "warn",
            )
        });
        assert_eq!(result, Ok(()));
        assert_eq!(forwarded(&captured).len(), 1);
    }

    #[test]
    fn bad_input_gives_err_and_no_event() {
        let long = "a".repeat(41);
        let cases = [
            ("nowhere", "step_failed", "disk_full", "warn"),
            ("install", "Step", "disk_full", "warn"),
            ("install", "step_failed", "", "warn"),
            ("install", "step_failed", "disk-full", "warn"),
            ("install", "step_failed", "disk_full", "error"),
            ("install", "1st", "disk_full", "warn"),
            ("install", long.as_str(), "disk_full", "warn"),
        ];
        for (area, event, code, level) in cases {
            let throttle = Throttle::new();
            let (result, captured) = capture(|| report(&throttle, 0, area, event, code, level));
            assert_eq!(result, Err("invalid ui event".to_owned()), "{area} {event}");
            assert!(forwarded(&captured).is_empty(), "{area} {event}");
            assert!(captured
                .iter()
                .any(|c| c.level == Level::DEBUG && c.target == "launcher.ui"));
        }
    }

    #[test]
    fn the_61st_call_is_suppressed_and_the_next_window_counts_it() {
        let throttle = Throttle::new();
        let (results, captured) = capture(|| {
            let mut results = Vec::new();
            for _ in 0..61 {
                results.push(report(
                    &throttle,
                    0,
                    "launch",
                    "play_failed",
                    "timeout",
                    "warn",
                ));
            }
            results.push(report(
                &throttle,
                WINDOW_MS,
                "launch",
                "play_failed",
                "timeout",
                "warn",
            ));
            results
        });
        assert!(results.iter().all(|result| result == &Ok(())));
        let events = forwarded(&captured);
        assert_eq!(events.len(), 61);
        assert!(events[..60]
            .iter()
            .all(|e| e.field("suppressed") == Some("0")));
        assert_eq!(events[60].field("suppressed"), Some("1"));
    }
}
