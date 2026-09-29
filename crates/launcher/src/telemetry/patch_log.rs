//! Once-per-session summary of the client-patches DLL's own log (Black
//! Market plan §5.1, BM-06).
//!
//! The DLL writes `cimmeria-client-patches.log` next to `SGW.exe`,
//! rewritten at each launch, one line per event:
//! `[cimmeria-client-patches +<ms>ms] <message>` (see
//! `crates/client-patches/src/log.rs`). The launcher reads that file
//! during a telemetry session and records one `client.patches.boot`
//! event: the DLL version, the fingerprint result per hooked site, and
//! whether the hooks went in. With the launcher's own injection outcome
//! alongside, "no Black Market window" can be diagnosed from SigNoz: not
//! injected, a different `SGW.exe` build, or a hook that failed.
//!
//! The messages parsed here are the ones `crates/client-patches/src/boot.rs`
//! writes. Its README lists them as a contract with this parser.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use super::events::{ClientNativeEvent, TelemetryEvent};
use super::patch_counts::{self, CountsTracker};
use crate::client_patches::PatchInjection;

/// The DLL's log file name, next to `SGW.exe`.
pub const LOG_FILE_NAME: &str = "cimmeria-client-patches.log";

/// Tracing target of the summary event.
pub const EVENT_TARGET: &str = "client.patches.boot";

/// Line prefix the DLL writes, before `+<ms>ms] `.
const LINE_PREFIX: &str = "[cimmeria-client-patches +";

/// File-time slack when deciding whether the log belongs to this
/// launch: coarse file-system timestamps can round down.
const MTIME_SLACK: Duration = Duration::from_secs(2);

/// One site's fingerprint result, as the DLL logged it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiteOutcome {
    /// The expected prologue bytes.
    Stock,
    /// Already hooked by the telemetry DLL; the patch chains on top.
    Chained,
    /// Already hooked by something else; refused.
    UnknownHook,
    /// Different bytes: another `SGW.exe` build, or another patch.
    Mismatch,
    Unreadable,
}

impl SiteOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stock => "stock",
            Self::Chained => "chained",
            Self::UnknownHook => "unknown_hook",
            Self::Mismatch => "mismatch",
            Self::Unreadable => "unreadable",
        }
    }

    fn is_usable(self) -> bool {
        matches!(self, Self::Stock | Self::Chained)
    }
}

/// How the DLL's bootstrap ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Every hook went in.
    Installed,
    /// The DLL stopped before hooking (fingerprint gate, `lua51.dll`,
    /// MinHook init); the message says why.
    NothingInstalled(String),
    /// A hook failed part way; the message says which.
    HookFailed(String),
}

impl Verdict {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Installed => "installed",
            Self::NothingInstalled(_) => "nothing_installed",
            Self::HookFailed(_) => "hook_failed",
        }
    }

    fn detail(&self) -> Option<&str> {
        match self {
            Self::Installed => None,
            Self::NothingInstalled(m) | Self::HookFailed(m) => Some(m),
        }
    }
}

/// What one read of the log says.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PatchLogSummary {
    pub version: Option<String>,
    /// In log order; a site logged twice keeps its last result.
    pub sites: Vec<(String, SiteOutcome)>,
    pub verdict: Option<Verdict>,
}

/// Parse the whole log text. Lines without the DLL's prefix, and
/// messages this parser does not know, are skipped.
pub fn parse(text: &str) -> PatchLogSummary {
    let mut summary = PatchLogSummary::default();
    for message in text.lines().filter_map(message_of) {
        if let Some(rest) = message.strip_prefix("attached, version ") {
            let version = rest.split(',').next().unwrap_or(rest).trim();
            summary.version = Some(version.to_string());
        } else if let Some((site, outcome)) = parse_site(message) {
            match summary.sites.iter_mut().find(|(name, _)| *name == site) {
                Some(entry) => entry.1 = outcome,
                None => summary.sites.push((site.to_string(), outcome)),
            }
        } else if message.starts_with("Black Market installed")
            || message.starts_with("Black Market receive path installed")
        {
            // The DLL writes "Black Market installed: ..." (boot.rs); the
            // older wording is still accepted for earlier DLL builds.
            summary.verdict = Some(Verdict::Installed);
        } else if message.ends_with("nothing installed") {
            summary.verdict = Some(Verdict::NothingInstalled(message.to_string()));
        } else if message.contains("stopping, the Black Market stays off") {
            summary.verdict = Some(Verdict::HookFailed(message.to_string()));
        }
    }
    summary
}

/// The message part of one DLL log line.
pub(super) fn message_of(line: &str) -> Option<&str> {
    let rest = line.trim_end_matches('\r').strip_prefix(LINE_PREFIX)?;
    let (_elapsed, message) = rest.split_once("ms] ")?;
    Some(message)
}

/// `<site> at 0x<hex>: <result>`.
fn parse_site(message: &str) -> Option<(&str, SiteOutcome)> {
    let (site, rest) = message.split_once(" at 0x")?;
    let (address, result) = rest.split_once(": ")?;
    if address.is_empty() || !address.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let outcome = if result == "stock" {
        SiteOutcome::Stock
    } else if result.starts_with("already hooked (jump to") {
        SiteOutcome::Chained
    } else if result.starts_with("already hooked by a jump") {
        SiteOutcome::UnknownHook
    } else if result.starts_with("expected ") {
        SiteOutcome::Mismatch
    } else if result == "unreadable" {
        SiteOutcome::Unreadable
    } else {
        return None;
    };
    Some((site, outcome))
}

/// Reads the log during one telemetry session and yields the summary
/// event exactly once.
#[derive(Debug)]
pub struct PatchLogWatcher {
    path: PathBuf,
    launched_at: SystemTime,
    injection: PatchInjection,
    emitted: bool,
    /// When to send `client.patches.counts` (see [`super::patch_counts`]).
    counts: CountsTracker,
}

impl PatchLogWatcher {
    /// `install_dir` holds `SGW.exe`; `launched_at` is taken just before
    /// the game started, so a log left over from an earlier launch is
    /// not mistaken for this one's.
    pub fn new(install_dir: &Path, launched_at: SystemTime, injection: PatchInjection) -> Self {
        Self {
            path: install_dir.join(LOG_FILE_NAME),
            launched_at,
            injection,
            emitted: false,
            counts: CountsTracker::default(),
        }
    }

    /// Called every telemetry tick. Yields a `client.patches.counts`
    /// event at most once a minute, when a count moved. Reads the log
    /// only when a report is due.
    pub fn poll_counts(&mut self, now: Instant) -> Option<TelemetryEvent> {
        if self.injection != PatchInjection::Injected || !self.counts.due(now) {
            return None;
        }
        let counts = patch_counts::parse(&self.read_text()?);
        self.counts.periodic(now, &counts)
    }

    /// Called once when the game exits: the end-of-session counts.
    pub fn finish_counts(&mut self) -> Option<TelemetryEvent> {
        if self.injection != PatchInjection::Injected {
            return None;
        }
        let counts = self.read_text().map(|t| patch_counts::parse(&t));
        self.counts.finish(counts.as_ref())
    }

    /// Called every telemetry tick. Yields the event as soon as the
    /// outcome is known: at once when the DLL was not injected, else
    /// when the log shows how the bootstrap ended.
    pub fn poll(&mut self) -> Option<TelemetryEvent> {
        if self.emitted {
            return None;
        }
        if self.injection != PatchInjection::Injected {
            return self.emit(None);
        }
        let summary = self.read()?;
        summary.verdict.as_ref()?;
        self.emit(Some(summary))
    }

    /// Called once when the game exits. Yields the event if [`poll`]
    /// never did: the DLL was injected but never logged a verdict, or
    /// never wrote its log at all.
    ///
    /// [`poll`]: Self::poll
    pub fn finish(&mut self) -> Option<TelemetryEvent> {
        if self.emitted {
            return None;
        }
        let summary = self.read();
        self.emit(summary)
    }

    /// This launch's log, if the DLL has written one.
    fn read(&self) -> Option<PatchLogSummary> {
        Some(parse(&self.read_text()?))
    }

    /// This launch's log text, if the DLL has written one.
    fn read_text(&self) -> Option<String> {
        let modified = std::fs::metadata(&self.path).ok()?.modified().ok()?;
        if modified + MTIME_SLACK < self.launched_at {
            return None;
        }
        let bytes = std::fs::read(&self.path).ok()?;
        Some(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn emit(&mut self, summary: Option<PatchLogSummary>) -> Option<TelemetryEvent> {
        self.emitted = true;
        Some(summary_event(self.injection, summary.as_ref()))
    }
}

/// The `client.patches.boot` event. `summary` is `None` when there was
/// no log from this launch to read.
pub fn summary_event(
    injection: PatchInjection,
    summary: Option<&PatchLogSummary>,
) -> TelemetryEvent {
    use serde_json::Value;

    let mut fields = serde_json::Map::new();
    fields.insert("injection".into(), injection.as_str().into());
    fields.insert("log_found".into(), summary.is_some().into());
    let mut healthy = injection == PatchInjection::Injected;
    if let Some(s) = summary {
        if let Some(v) = &s.version {
            fields.insert("dll_version".into(), v.as_str().into());
        }
        let fingerprint_ok = !s.sites.is_empty() && s.sites.iter().all(|(_, o)| o.is_usable());
        fields.insert("fingerprint_ok".into(), fingerprint_ok.into());
        for (site, outcome) in &s.sites {
            fields.insert(format!("fingerprint.{site}"), outcome.as_str().into());
        }
        let verdict = s.verdict.as_ref().map_or("none", Verdict::as_str);
        fields.insert("verdict".into(), verdict.into());
        if let Some(detail) = s.verdict.as_ref().and_then(Verdict::detail) {
            fields.insert("verdict_detail".into(), Value::String(detail.to_string()));
        }
        healthy &= s.verdict == Some(Verdict::Installed);
    } else {
        healthy = false;
    }
    TelemetryEvent::ClientNative(ClientNativeEvent {
        ts_ms: 0,
        seq: 0,
        target: EVENT_TARGET.into(),
        level: if healthy { "info" } else { "warn" }.into(),
        fields,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A successful boot, as `boot.rs` logs it.
    const INSTALLED_LOG: &str = "\
[cimmeria-client-patches +0ms] attached, version 0.1.0, host C:\\SGW\\Working\\Binaries\\SGW.exe\r\n\
[cimmeria-client-patches +3ms] lua51.dll exports resolved\r\n\
[cimmeria-client-patches +4ms] Client_NetIn_EntityMethodDispatch at 0x00c6f8f0: stock\r\n\
[cimmeria-client-patches +4ms] EntityDescription_GetExposedClientMethodByIndex at 0x01590f30: already hooked (jump to 0x10002000), chaining\r\n\
[cimmeria-client-patches +4ms] FEngineLoop::Tick at 0x00416ec0: stock\r\n\
[cimmeria-client-patches +4ms] ServerConnection::startEntityMessage at 0x00dd6a60: stock\r\n\
[cimmeria-client-patches +5ms] hooked FEngineLoop::Tick\r\n\
[cimmeria-client-patches +5ms] hooked Client_NetIn_EntityMethodDispatch\r\n\
[cimmeria-client-patches +5ms] hooked EntityDescription_GetExposedClientMethodByIndex\r\n\
[cimmeria-client-patches +5ms] Black Market receive path installed; calls go to the Lua table CimmeriaBM\r\n";

    const MISMATCH_LOG: &str = "\
[cimmeria-client-patches +0ms] attached, version 0.2.0, host SGW.exe\r\n\
[cimmeria-client-patches +1ms] cimmeria-client-telemetry.dll loaded at 0x10000000..0x10080000; its hooks may be chained\r\n\
[cimmeria-client-patches +2ms] FEngineLoop::Tick at 0x00416ec0: expected 64 a1 00 00, found 55 8b ec 83\r\n\
[cimmeria-client-patches +2ms] Client_NetIn_EntityMethodDispatch at 0x00c6f8f0: already hooked by a jump to 0x7ff00000, outside every known hook owner; not chaining\r\n\
[cimmeria-client-patches +2ms] a prologue does not match: a different SGW.exe build, or another patch at that address; nothing installed\r\n";

    /// The verdict line exactly as the shipped DLL writes it (copied from
    /// a player's log, 2026-09-29). The parser used to look only for an
    /// older wording, so a successful install never reported a verdict.
    #[test]
    fn parses_the_installed_line_the_dll_writes() {
        let log = concat!(
            "[cimmeria-client-patches +0ms] attached, version 0.1.0, host SGW.exe
",
            "[cimmeria-client-patches +257ms] Black Market installed: received calls go to ",
            "the Lua table CimmeriaBM, and CimmeriaBMNative is registered for sending once ",
            "the UI Lua is up
",
        );
        assert_eq!(parse(log).verdict, Some(Verdict::Installed));
    }

    #[test]
    fn parses_version_sites_and_installed_verdict() {
        let s = parse(INSTALLED_LOG);
        assert_eq!(s.version.as_deref(), Some("0.1.0"));
        assert_eq!(
            s.sites,
            vec![
                (
                    "Client_NetIn_EntityMethodDispatch".to_string(),
                    SiteOutcome::Stock
                ),
                (
                    "EntityDescription_GetExposedClientMethodByIndex".to_string(),
                    SiteOutcome::Chained
                ),
                ("FEngineLoop::Tick".to_string(), SiteOutcome::Stock),
                (
                    "ServerConnection::startEntityMessage".to_string(),
                    SiteOutcome::Stock
                ),
            ]
        );
        assert_eq!(s.verdict, Some(Verdict::Installed));
    }

    #[test]
    fn parses_mismatch_unknown_hook_and_nothing_installed() {
        let s = parse(MISMATCH_LOG);
        assert_eq!(s.version.as_deref(), Some("0.2.0"));
        assert_eq!(
            s.sites,
            vec![
                ("FEngineLoop::Tick".to_string(), SiteOutcome::Mismatch),
                (
                    "Client_NetIn_EntityMethodDispatch".to_string(),
                    SiteOutcome::UnknownHook
                ),
            ],
            "the telemetry-DLL range line must not parse as a site"
        );
        assert!(
            matches!(s.verdict, Some(Verdict::NothingInstalled(m)) if m.contains("different SGW.exe build"))
        );
    }

    #[test]
    fn hook_failure_and_unreadable_site() {
        let s = parse(
            "[cimmeria-client-patches +1ms] FEngineLoop::Tick at 0x00416ec0: unreadable\n\
             [cimmeria-client-patches +2ms] MH_CreateHook failed; stopping, the Black Market stays off\n",
        );
        assert_eq!(
            s.sites,
            vec![("FEngineLoop::Tick".to_string(), SiteOutcome::Unreadable)]
        );
        assert!(matches!(s.verdict, Some(Verdict::HookFailed(_))));
    }

    /// Other programs' lines, and a hostile message that only looks like
    /// a site line, are ignored.
    #[test]
    fn foreign_and_malformed_lines_are_skipped() {
        let s = parse(
            "FEngineLoop::Tick at 0x00416ec0: stock\n\
             [cimmeria-client-patches +1ms] x at 0xZZ: stock\n\
             [cimmeria-client-patches +1ms] x at 0x00: something new\n\
             [cimmeria-client-patches garbage\n",
        );
        assert_eq!(s, PatchLogSummary::default());
    }

    #[test]
    fn a_partial_log_has_no_verdict() {
        let s = parse("[cimmeria-client-patches +0ms] attached, version 0.1.0, host SGW.exe\r\n");
        assert_eq!(s.version.as_deref(), Some("0.1.0"));
        assert_eq!(s.verdict, None);
    }

    fn fields(ev: &TelemetryEvent) -> (&str, &serde_json::Map<String, serde_json::Value>) {
        match ev {
            TelemetryEvent::ClientNative(e) => {
                assert_eq!(e.target, EVENT_TARGET);
                (e.level.as_str(), &e.fields)
            }
            other => panic!("expected ClientNative, got {other:?}"),
        }
    }

    #[test]
    fn installed_event_carries_version_and_fingerprint_at_info() {
        let ev = summary_event(PatchInjection::Injected, Some(&parse(INSTALLED_LOG)));
        let (level, f) = fields(&ev);
        assert_eq!(level, "info");
        assert_eq!(f["injection"], "injected");
        assert_eq!(f["dll_version"], "0.1.0");
        assert_eq!(f["verdict"], "installed");
        assert_eq!(f["fingerprint_ok"], true);
        assert_eq!(f["fingerprint.FEngineLoop::Tick"], "stock");
        assert_eq!(
            f["fingerprint.EntityDescription_GetExposedClientMethodByIndex"],
            "chained"
        );
        assert!(!f.contains_key("verdict_detail"));
    }

    #[test]
    fn mismatch_event_is_a_warning_with_the_reason() {
        let ev = summary_event(PatchInjection::Injected, Some(&parse(MISMATCH_LOG)));
        let (level, f) = fields(&ev);
        assert_eq!(level, "warn");
        assert_eq!(f["fingerprint_ok"], false);
        assert_eq!(f["verdict"], "nothing_installed");
        assert!(f["verdict_detail"]
            .as_str()
            .unwrap()
            .contains("nothing installed"));
    }

    #[test]
    fn opted_out_event_has_no_log_fields() {
        let ev = summary_event(PatchInjection::OptedOut, None);
        let (level, f) = fields(&ev);
        assert_eq!(level, "warn");
        assert_eq!(f["injection"], "opted_out");
        assert_eq!(f["log_found"], false);
        assert!(!f.contains_key("verdict"));
    }

    #[test]
    fn watcher_emits_once_when_the_verdict_appears() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join(LOG_FILE_NAME);
        let launched = SystemTime::now() - Duration::from_secs(1);
        let mut w = PatchLogWatcher::new(dir.path(), launched, PatchInjection::Injected);
        assert!(w.poll().is_none(), "no log yet");
        std::fs::write(
            &log,
            "[cimmeria-client-patches +0ms] attached, version 0.1.0, host x\r\n",
        )
        .unwrap();
        assert!(w.poll().is_none(), "no verdict yet");
        std::fs::write(&log, INSTALLED_LOG).unwrap();
        let ev = w.poll().expect("verdict logged");
        assert_eq!(fields(&ev).1["verdict"], "installed");
        assert!(w.poll().is_none(), "once per session");
        assert!(w.finish().is_none(), "once per session");
    }

    /// A log left over from an earlier launch describes that launch, not
    /// this one.
    #[test]
    fn watcher_ignores_a_stale_log() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(LOG_FILE_NAME), INSTALLED_LOG).unwrap();
        let launched = SystemTime::now() + Duration::from_secs(60);
        let mut w = PatchLogWatcher::new(dir.path(), launched, PatchInjection::Injected);
        assert!(w.poll().is_none());
        let ev = w.finish().expect("finish always reports");
        let (level, f) = fields(&ev);
        assert_eq!(level, "warn");
        assert_eq!(f["log_found"], false);
    }

    #[test]
    fn watcher_reports_a_skipped_injection_on_the_first_poll() {
        let dir = tempfile::tempdir().unwrap();
        let mut w =
            PatchLogWatcher::new(dir.path(), SystemTime::now(), PatchInjection::Unavailable);
        let ev = w.poll().expect("nothing to wait for");
        assert_eq!(fields(&ev).1["injection"], "unavailable");
        assert!(w.finish().is_none());
    }

    #[test]
    fn watcher_finish_reports_an_injected_dll_that_never_logged_a_verdict() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(LOG_FILE_NAME),
            "[cimmeria-client-patches +0ms] attached, version 0.1.0, host x\r\n",
        )
        .unwrap();
        let mut w = PatchLogWatcher::new(
            dir.path(),
            SystemTime::now() - Duration::from_secs(1),
            PatchInjection::Injected,
        );
        assert!(w.poll().is_none());
        let ev = w.finish().unwrap();
        let (level, f) = fields(&ev);
        assert_eq!(level, "warn");
        assert_eq!(f["verdict"], "none");
        assert_eq!(f["dll_version"], "0.1.0");
    }

    /// The counts event comes from this launch's log, periodically and
    /// once at exit, and never for a DLL that was not injected.
    #[test]
    fn watcher_reports_counts_periodically_and_at_exit() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(LOG_FILE_NAME),
            "[cimmeria-client-patches +9ms] claimed onBMOpen (#1 received)\r\n\
             [cimmeria-client-patches +9ms] onBMOpen dropped: the global CimmeriaBM is not defined, so the UI overlay is not installed (#1)\r\n",
        )
        .unwrap();
        let launched = SystemTime::now() - Duration::from_secs(1);
        let mut w = PatchLogWatcher::new(dir.path(), launched, PatchInjection::Injected);
        let now = Instant::now();
        let TelemetryEvent::ClientNative(e) = w.poll_counts(now).expect("counts moved") else {
            panic!()
        };
        assert_eq!(e.target, patch_counts::EVENT_TARGET);
        assert_eq!(e.fields["dropped_no_overlay"], 1);
        assert!(w.poll_counts(now).is_none(), "not due again yet");
        assert!(w.finish_counts().is_some());
        assert!(w.finish_counts().is_none());

        let mut off = PatchLogWatcher::new(dir.path(), launched, PatchInjection::OptedOut);
        assert!(off.poll_counts(now).is_none());
        assert!(off.finish_counts().is_none());
    }
}
