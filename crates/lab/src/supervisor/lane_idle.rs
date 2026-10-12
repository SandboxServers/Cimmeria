//! Wait for the build lane to go idle before a client launch.
//!
//! The client's startup reads many GB of images and PAKs. With concurrent
//! cargo jobs and sccache saturating the drive it sat in page-in waits
//! until the watchdog killed it (2026-10-12, three times out of three
//! between 01:57Z and 02:28Z), and started in about 2 s once the builds
//! drained. The lane's slot protocol is `tools/build-lane/lane-slots.ps1`:
//! a held slot is the directory `slot.N` under `<LANE_ROOT>/lane`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::time::Instant;

use super::process;

/// How long a launch waits for the lane to go idle.
pub const LANE_WAIT: Duration = Duration::from_secs(90);

/// Overrides [`LANE_WAIT`], in whole seconds; `0` turns the wait off.
pub const LANE_WAIT_ENV: &str = "CIMMERIA_LAB_LANE_WAIT_SECS";

/// The longest wait [`LANE_WAIT_ENV`] can set.
pub const MAX_LANE_WAIT: Duration = Duration::from_secs(600);

/// How often the wait looks at the lane.
const POLL: Duration = Duration::from_secs(5);

/// The wait from the raw [`LANE_WAIT_ENV`] value: unset, empty or
/// unparseable is [`LANE_WAIT`]; clamped to [`MAX_LANE_WAIT`].
pub fn lane_wait_from(raw: Option<&str>) -> Duration {
    raw.map(str::trim)
        .and_then(|s| s.parse::<u64>().ok())
        .map(|secs| Duration::from_secs(secs.min(MAX_LANE_WAIT.as_secs())))
        .unwrap_or(LANE_WAIT)
}

/// `<lane root>/lane`, the way `lane.ps1` derives it: `LANE_ROOT`, else
/// `%LOCALAPPDATA%\cimmeria-build`.
pub fn lane_dir(lane_root: Option<&str>, local_app_data: Option<&str>) -> Option<PathBuf> {
    let root = lane_root
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            local_app_data
                .filter(|s| !s.is_empty())
                .map(|d| Path::new(d).join("cimmeria-build"))
        })?;
    Some(root.join("lane"))
}

/// The held slots of the lane at `dir`, as `slot.N [what]`, sorted. A slot
/// whose `winpid` is dead (`is_alive` false) is a leftover and is skipped;
/// one with only an MSYS `pid` counts as held.
pub fn busy_slots(dir: &Path, is_alive: impl Fn(u32) -> bool) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut busy = Vec::new();
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !name.starts_with("slot.") || !e.path().is_dir() {
            continue;
        }
        let read = |f: &str| {
            std::fs::read_to_string(e.path().join(f))
                .unwrap_or_default()
                .trim()
                .to_string()
        };
        if read("winpid").parse::<u32>().is_ok_and(|p| !is_alive(p)) {
            continue;
        }
        busy.push(format!("{name} [{}]", read("what")));
    }
    busy.sort();
    busy
}

/// What a wait found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdleWait {
    pub waited: Duration,
    /// The slots still held when the wait ended; empty means idle.
    pub busy: Vec<String>,
}

/// Poll the lane at `dir` every `poll` until no slot is held or `max`
/// has passed.
pub async fn wait_for_idle(
    dir: &Path,
    max: Duration,
    poll: Duration,
    is_alive: impl Fn(u32) -> bool,
) -> IdleWait {
    let start = Instant::now();
    loop {
        let busy = busy_slots(dir, &is_alive);
        let waited = start.elapsed();
        if busy.is_empty() || waited >= max {
            return IdleWait { waited, busy };
        }
        tokio::time::sleep(poll).await;
    }
}

/// Called before every client launch: wait (bounded) for the build lane to
/// go idle, then launch regardless. Never fails.
pub async fn wait_before_launch(cause: &'static str) {
    let max = lane_wait_from(std::env::var(LANE_WAIT_ENV).ok().as_deref());
    if max.is_zero() {
        return;
    }
    let Some(dir) = lane_dir(
        std::env::var("LANE_ROOT").ok().as_deref(),
        std::env::var("LOCALAPPDATA").ok().as_deref(),
    ) else {
        return;
    };
    let w = wait_for_idle(&dir, max, POLL, process::is_alive).await;
    if w.busy.is_empty() {
        if !w.waited.is_zero() {
            tracing::info!(
                target: "lab.client",
                event = "lane_idle",
                cause,
                waited_ms = w.waited.as_millis() as u64,
                "build lane idle; launching"
            );
        }
    } else {
        tracing::warn!(
            target: "lab.client",
            event = "lane_busy_launch",
            cause,
            waited_ms = w.waited.as_millis() as u64,
            busy = %w.busy.join(" "),
            "build lane still busy after the wait; launching anyway (a slow start may be killed by the watchdog)"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(dir: &Path, name: &str, files: &[(&str, &str)]) {
        let d = dir.join(name);
        std::fs::create_dir_all(&d).unwrap();
        for (f, c) in files {
            std::fs::write(d.join(f), c).unwrap();
        }
    }

    #[test]
    fn only_live_slots_count_as_busy() {
        let t = tempfile::tempdir().unwrap();
        slot(
            t.path(),
            "slot.1",
            &[("winpid", "1111"), ("what", "10:00 wt :: cargo check")],
        );
        slot(
            t.path(),
            "slot.2",
            &[("winpid", "2222"), ("what", "dead job")],
        );
        slot(t.path(), "slot.3", &[("pid", "77"), ("what", "bash lane")]);
        slot(t.path(), ".stage-9-abc", &[("what", "staging")]);
        std::fs::write(t.path().join("SLOTS"), "4").unwrap();
        let busy = busy_slots(t.path(), |p| p == 1111);
        assert_eq!(
            busy,
            vec!["slot.1 [10:00 wt :: cargo check]", "slot.3 [bash lane]"]
        );
        assert!(busy_slots(&t.path().join("missing"), |_| true).is_empty());
    }

    #[test]
    fn the_lane_dir_follows_lane_ps1() {
        assert_eq!(
            lane_dir(Some("D:/x"), Some("C:/L")),
            Some(PathBuf::from("D:/x").join("lane"))
        );
        assert_eq!(
            lane_dir(None, Some("C:/L")),
            Some(PathBuf::from("C:/L").join("cimmeria-build").join("lane"))
        );
        assert_eq!(lane_dir(Some(""), None), None);
    }

    #[test]
    fn the_env_override_parses_seconds() {
        assert_eq!(lane_wait_from(None), LANE_WAIT);
        assert_eq!(lane_wait_from(Some("x")), LANE_WAIT);
        assert_eq!(lane_wait_from(Some("0")), Duration::ZERO);
        assert_eq!(lane_wait_from(Some("9999")), MAX_LANE_WAIT);
    }

    /// A held lane is waited out to the cap and reported busy; one that
    /// frees mid-wait returns idle at the poll that sees it.
    #[tokio::test(start_paused = true)]
    async fn the_wait_ends_when_the_lane_frees_or_the_cap_passes() {
        let t = tempfile::tempdir().unwrap();
        slot(t.path(), "slot.1", &[("winpid", "1"), ("what", "job")]);
        let held = wait_for_idle(
            t.path(),
            Duration::from_secs(20),
            Duration::from_secs(5),
            |_| true,
        )
        .await;
        assert_eq!(held.busy, vec!["slot.1 [job]"]);
        assert!(held.waited >= Duration::from_secs(20));

        let dir = t.path().to_path_buf();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(12)).await;
            std::fs::remove_dir_all(dir.join("slot.1")).unwrap();
        });
        let freed = wait_for_idle(
            t.path(),
            Duration::from_secs(60),
            Duration::from_secs(5),
            |_| true,
        )
        .await;
        assert!(freed.busy.is_empty());
        assert!(freed.waited >= Duration::from_secs(12) && freed.waited < Duration::from_secs(20));
    }
}
