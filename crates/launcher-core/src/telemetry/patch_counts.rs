//! Claimed / delivered / dropped counts from the client-patches DLL's
//! log, as a `client.patches.counts` event.
//!
//! [`super::patch_log`] reports how the DLL booted. This reports what it
//! did afterwards: how many Black Market calls it claimed off the wire,
//! handed to the Lua overlay, or dropped, and why. On 2026-09-29 the only
//! way to learn that `onBMOpen` was being dropped because the overlay's
//! global `CimmeriaBM` was not defined was to ask the player for the file.
//!
//! The DLL has no telemetry channel; its log is the contract. Each
//! per-call line ends in `(#n)`, the running value of one of its session
//! counters (`crates/client-patches/src/counters.rs`), and the DLL only
//! writes the 1st, 10th, 100th ... occurrence (the send path also the
//! first 100). So the highest `n` seen per counter is a **lower bound**
//! on the true count, exact up to the first power of ten. The event says
//! so (`counts_are_lower_bounds`).
//!
//! The messages matched here are the ones `crates/client-patches/src/`
//! `receive/detours.rs`, `deliver/mod.rs`, `send/mod.rs` and
//! `send/natives.rs` write. An unknown line is skipped.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use super::events::{ClientNativeEvent, TelemetryEvent};

/// Tracing target of the event.
pub const EVENT_TARGET: &str = "client.patches.counts";

/// Shortest gap between two periodic events. The final one at game exit
/// is always sent.
pub const PERIODIC_INTERVAL: Duration = Duration::from_secs(60);

/// The DLL stops writing after this many lines (`client-patches`
/// `log::MAX_LINES`); a log that long may be missing later counts.
const DLL_MAX_LINES: usize = 2_000;

/// Longest `last_reason` carried, in characters.
const MAX_REASON_CHARS: usize = 256;

/// Most method names listed per counter.
const MAX_METHODS: usize = 16;

/// The DLL's session counters, in the order the event lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Counter {
    Claimed,
    NotLocalPlayer,
    DecodeFailed,
    DroppedQueueFull,
    Delivered,
    DroppedNoOverlay,
    DroppedNoHandler,
    HandlerFailed,
    Sent,
    SendRefused,
    TechCompetencyNil,
    NativesRegistered,
    RegisterFailed,
}

impl Counter {
    pub const ALL: [Counter; 13] = [
        Self::Claimed,
        Self::NotLocalPlayer,
        Self::DecodeFailed,
        Self::DroppedQueueFull,
        Self::Delivered,
        Self::DroppedNoOverlay,
        Self::DroppedNoHandler,
        Self::HandlerFailed,
        Self::Sent,
        Self::SendRefused,
        Self::TechCompetencyNil,
        Self::NativesRegistered,
        Self::RegisterFailed,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claimed => "claimed",
            Self::NotLocalPlayer => "not_local_player",
            Self::DecodeFailed => "decode_failed",
            Self::DroppedQueueFull => "dropped_queue_full",
            Self::Delivered => "delivered",
            Self::DroppedNoOverlay => "dropped_no_overlay",
            Self::DroppedNoHandler => "dropped_no_handler",
            Self::HandlerFailed => "handler_failed",
            Self::Sent => "sent",
            Self::SendRefused => "send_refused",
            Self::TechCompetencyNil => "tech_competency_nil",
            Self::NativesRegistered => "natives_registered",
            Self::RegisterFailed => "register_failed",
        }
    }

    /// A claimed call the overlay never got.
    fn is_drop(self) -> bool {
        matches!(
            self,
            Self::DecodeFailed
                | Self::DroppedQueueFull
                | Self::DroppedNoOverlay
                | Self::DroppedNoHandler
                | Self::HandlerFailed
        )
    }

    /// Worth a warning on its own.
    fn is_problem(self) -> bool {
        self.is_drop() || matches!(self, Self::SendRefused | Self::RegisterFailed)
    }
}

/// What one counter's lines say.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CounterSeen {
    /// Highest `(#n)` seen: a lower bound on the DLL's count.
    pub at_least: u64,
    /// Method (or Lua handler / native) names the lines named.
    pub methods: BTreeSet<String>,
    /// The last line's reason, for counters whose lines carry one.
    pub last_reason: Option<String>,
}

/// Counts from one read of the log.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PatchCounts {
    pub counters: std::collections::BTreeMap<Counter, CounterSeen>,
    /// DLL log lines read.
    pub lines: usize,
}

impl PatchCounts {
    pub fn get(&self, c: Counter) -> u64 {
        self.counters.get(&c).map_or(0, |s| s.at_least)
    }

    fn dropped_total(&self) -> u64 {
        Counter::ALL
            .iter()
            .filter(|c| c.is_drop())
            .map(|c| self.get(*c))
            .sum()
    }

    fn has_problem(&self) -> bool {
        Counter::ALL
            .iter()
            .any(|c| c.is_problem() && self.get(*c) > 0)
    }

    /// The numbers only, for deciding whether anything changed.
    fn totals(&self) -> Vec<u64> {
        Counter::ALL.iter().map(|c| self.get(*c)).collect()
    }
}

/// Parse the whole log text.
pub fn parse(text: &str) -> PatchCounts {
    let mut counts = PatchCounts::default();
    for message in text.lines().filter_map(super::patch_log::message_of) {
        counts.lines += 1;
        let Some((counter, method, reason, n)) = classify(message) else {
            continue;
        };
        let seen = counts.counters.entry(counter).or_default();
        seen.at_least = seen.at_least.max(n);
        if !method.is_empty() && seen.methods.len() < MAX_METHODS {
            seen.methods.insert(method.to_string());
        }
        if let Some(r) = reason {
            seen.last_reason = Some(r.chars().take(MAX_REASON_CHARS).collect());
        }
    }
    counts
}

/// `(counter, method, reason, n)` for one DLL message, or `None`.
fn classify(message: &str) -> Option<(Counter, &str, Option<&str>, u64)> {
    let (body, n) = split_count(message)?;
    if let Some(m) = body.strip_prefix("claimed ") {
        return Some((Counter::Claimed, m, None, n));
    }
    if let Some(rest) = body.strip_prefix("delivered ") {
        let m = rest.split(" to ").next().unwrap_or(rest);
        return Some((Counter::Delivered, m, None, n));
    }
    if let Some(rest) = body.strip_prefix("sent ") {
        let m = rest.split(" (").next().unwrap_or(rest);
        return Some((Counter::Sent, m, None, n));
    }
    if let Some(rest) = body.strip_prefix("registered ") {
        let m = rest.split(' ').next().unwrap_or(rest);
        return Some((Counter::NativesRegistered, m, None, n));
    }
    if let Some(rest) = body.strip_prefix("registering ") {
        let (m, why) = rest.split_once(' ').unwrap_or((rest, ""));
        return Some((Counter::RegisterFailed, m, Some(why), n));
    }
    if let Some(m) = body.strip_suffix(" for another entity left to the client") {
        return Some((Counter::NotLocalPlayer, m, None, n));
    }
    if let Some((m, why)) = body.split_once(" dropped: ") {
        let counter = if why.starts_with("the global ") {
            Counter::DroppedNoOverlay
        } else if why.ends_with(" is not a function") {
            Counter::DroppedNoHandler
        } else {
            Counter::HandlerFailed
        };
        return Some((counter, m, Some(why), n));
    }
    if let Some((m, why)) = body.split_once(": dropped, ") {
        let counter = if why == "the queue is full" {
            Counter::DroppedQueueFull
        } else {
            Counter::DecodeFailed
        };
        return Some((counter, m, Some(why), n));
    }
    if let Some(m) = body.strip_suffix(": could not open the argument stream") {
        return Some((
            Counter::DecodeFailed,
            m,
            Some("could not open the argument stream"),
            n,
        ));
    }
    if let Some((m, why)) = body.split_once(" refused: ") {
        return Some((Counter::SendRefused, m, Some(why), n));
    }
    if let Some(m) = body.strip_suffix(" returns nil: no verified native getter in this build") {
        return Some((Counter::TechCompetencyNil, m, None, n));
    }
    if let Some((handler, why)) = body.split_once(" raised an error ") {
        return Some((Counter::HandlerFailed, handler, Some(why), n));
    }
    None
}

/// Split `<body> (#<n>)` or `<body> (#<n> received)` into body and n.
fn split_count(message: &str) -> Option<(&str, u64)> {
    let at = message.rfind(" (#")?;
    let tail = message[at + 3..].strip_suffix(')')?;
    let digits = tail.split(' ').next()?;
    let n = digits.parse().ok()?;
    Some((&message[..at], n))
}

/// Build the event. `final_report` is true for the one sent at game exit.
pub fn counts_event(counts: &PatchCounts, final_report: bool) -> TelemetryEvent {
    let mut f = serde_json::Map::new();
    f.insert("final".into(), final_report.into());
    f.insert("counts_are_lower_bounds".into(), true.into());
    f.insert("lines".into(), counts.lines.into());
    f.insert("log_capped".into(), (counts.lines >= DLL_MAX_LINES).into());
    f.insert("dropped_total".into(), counts.dropped_total().into());
    for c in Counter::ALL {
        f.insert(c.as_str().into(), counts.get(c).into());
        let Some(seen) = counts.counters.get(&c) else {
            continue;
        };
        if !seen.methods.is_empty() {
            let list: Vec<&str> = seen.methods.iter().map(String::as_str).collect();
            f.insert(format!("{}.methods", c.as_str()), list.join(",").into());
        }
        if let Some(r) = &seen.last_reason {
            f.insert(format!("{}.last_reason", c.as_str()), r.as_str().into());
        }
    }
    TelemetryEvent::ClientNative(ClientNativeEvent {
        ts_ms: 0,
        seq: 0,
        target: EVENT_TARGET.into(),
        level: if counts.has_problem() { "warn" } else { "info" }.into(),
        fields: f,
    })
}

/// Decides when to send: at most once per [`PERIODIC_INTERVAL`], only
/// when a count moved, and once more at game exit.
#[derive(Debug, Default)]
pub struct CountsTracker {
    last_emit: Option<Instant>,
    last_totals: Option<Vec<u64>>,
    finished: bool,
}

impl CountsTracker {
    /// Whether a periodic read is due at `now`.
    pub fn due(&self, now: Instant) -> bool {
        !self.finished
            && self
                .last_emit
                .is_none_or(|t| now.duration_since(t) >= PERIODIC_INTERVAL)
    }

    /// A periodic event, if `counts` moved since the last one. An empty
    /// log (the DLL claimed nothing yet) sends nothing.
    pub fn periodic(&mut self, now: Instant, counts: &PatchCounts) -> Option<TelemetryEvent> {
        if !self.due(now) {
            return None;
        }
        let totals = counts.totals();
        if totals.iter().all(|n| *n == 0) || self.last_totals.as_ref() == Some(&totals) {
            return None;
        }
        self.last_emit = Some(now);
        self.last_totals = Some(totals);
        Some(counts_event(counts, false))
    }

    /// The end-of-session event: always, once, when the log had any
    /// count (a session where the DLL claimed nothing says nothing, the
    /// boot event already covers it).
    pub fn finish(&mut self, counts: Option<&PatchCounts>) -> Option<TelemetryEvent> {
        if self.finished {
            return None;
        }
        self.finished = true;
        let counts = counts?;
        if counts.totals().iter().all(|n| *n == 0) {
            return None;
        }
        Some(counts_event(counts, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lines as the DLL writes them (message formats copied from
    /// `crates/client-patches/src/{receive/detours,deliver/mod,send/mod,
    /// send/natives}.rs`).
    const LOG: &str = "\
[cimmeria-client-patches +0ms] attached, version 0.1.0, host SGW.exe\r\n\
[cimmeria-client-patches +5ms] Black Market installed: received calls go to the Lua table CimmeriaBM\r\n\
[cimmeria-client-patches +900ms] claimed onBMOpen (#1 received)\r\n\
[cimmeria-client-patches +901ms] onBMOpen dropped: the global CimmeriaBM is not defined, so the UI overlay is not installed (#1)\r\n\
[cimmeria-client-patches +950ms] claimed onBMListings (#10 received)\r\n\
[cimmeria-client-patches +951ms] onBMOpen dropped: the global CimmeriaBM is not defined, so the UI overlay is not installed (#10)\r\n\
[cimmeria-client-patches +960ms] onBMListings for another entity left to the client (#1)\r\n\
[cimmeria-client-patches +961ms] onBMListings: dropped, truncated argument stream at byte 12 (#1)\r\n\
[cimmeria-client-patches +962ms] onBMListings: dropped, the queue is full (#1)\r\n\
[cimmeria-client-patches +970ms] registered CimmeriaBMNative (version 1) in lua_State 0x0a0b0c0d (#1)\r\n\
[cimmeria-client-patches +980ms] delivered onBMOpen to CimmeriaBM.onBMOpen (#1)\r\n\
[cimmeria-client-patches +981ms] onBMBid dropped: CimmeriaBM.onBMBid is not a function (#1)\r\n\
[cimmeria-client-patches +982ms] CimmeriaBM.onBMOpen raised an error (status 2): BlackMarket.lua:12: attempt to index nil (#1)\r\n\
[cimmeria-client-patches +990ms] sent BMSearch (cell method 61, sub-index 0, 12-byte payload) (#3)\r\n\
[cimmeria-client-patches +991ms] CimmeriaBMNative.bid refused: offline (not connected to a server) (#2)\r\n\
[cimmeria-client-patches +992ms] CimmeriaBMNative.techCompetency returns nil: no verified native getter in this build (#1)\r\n\
[cimmeria-client-patches +993ms] registering CimmeriaBMNative failed: no Lua stack space (#1)\r\n\
[cimmeria-client-patches +994ms] something the parser does not know (#4)\r\n";

    #[test]
    fn parses_every_counter_line_the_dll_writes() {
        let c = parse(LOG);
        assert_eq!(c.lines, 18);
        let expect = [
            (Counter::Claimed, 10),
            (Counter::DroppedNoOverlay, 10),
            (Counter::NotLocalPlayer, 1),
            (Counter::DecodeFailed, 1),
            (Counter::DroppedQueueFull, 1),
            (Counter::NativesRegistered, 1),
            (Counter::Delivered, 1),
            (Counter::DroppedNoHandler, 1),
            (Counter::HandlerFailed, 1),
            (Counter::Sent, 3),
            (Counter::SendRefused, 2),
            (Counter::TechCompetencyNil, 1),
            (Counter::RegisterFailed, 1),
        ];
        for (counter, n) in expect {
            assert_eq!(c.get(counter), n, "{counter:?}");
        }
        let claimed = &c.counters[&Counter::Claimed];
        assert_eq!(
            claimed.methods.iter().collect::<Vec<_>>(),
            ["onBMListings", "onBMOpen"]
        );
        assert_eq!(
            c.counters[&Counter::DecodeFailed].last_reason.as_deref(),
            Some("truncated argument stream at byte 12")
        );
        assert_eq!(
            c.counters[&Counter::SendRefused].last_reason.as_deref(),
            Some("offline (not connected to a server)")
        );
        assert_eq!(
            c.counters[&Counter::HandlerFailed]
                .methods
                .iter()
                .next()
                .map(String::as_str),
            Some("CimmeriaBM.onBMOpen")
        );
    }

    #[test]
    fn a_setup_error_and_no_stack_space_are_handler_failures() {
        let c = parse(
            "[cimmeria-client-patches +1ms] onBMOpen dropped: no Lua stack space (#1)\n\
             [cimmeria-client-patches +2ms] onBMOpen dropped: setting up the Lua call raised an error (status 4): not enough memory (#10)\n",
        );
        assert_eq!(c.get(Counter::HandlerFailed), 10);
        assert!(c.counters[&Counter::HandlerFailed]
            .last_reason
            .as_deref()
            .unwrap()
            .starts_with("setting up the Lua call"));
    }

    #[test]
    fn the_event_carries_counts_reasons_and_is_a_warning_on_drops() {
        let ev = counts_event(&parse(LOG), true);
        let TelemetryEvent::ClientNative(e) = &ev else {
            panic!("expected ClientNative");
        };
        assert_eq!(e.target, EVENT_TARGET);
        assert_eq!(e.level, "warn");
        let f = &e.fields;
        assert_eq!(f["final"], true);
        assert_eq!(f["counts_are_lower_bounds"], true);
        assert_eq!(f["claimed"], 10);
        assert_eq!(f["delivered"], 1);
        assert_eq!(f["dropped_no_overlay"], 10);
        // decode 1 + queue 1 + no overlay 10 + no handler 1 + handler 1.
        assert_eq!(f["dropped_total"], 14);
        assert_eq!(f["dropped_no_overlay.methods"], "onBMOpen");
        assert!(f["dropped_no_overlay.last_reason"]
            .as_str()
            .unwrap()
            .starts_with("the global CimmeriaBM is not defined"));
        assert_eq!(f["log_capped"], false);
    }

    #[test]
    fn a_clean_session_is_info() {
        let c = parse(
            "[cimmeria-client-patches +1ms] claimed onBMOpen (#1 received)\n\
             [cimmeria-client-patches +2ms] delivered onBMOpen to CimmeriaBM.onBMOpen (#1)\n",
        );
        let TelemetryEvent::ClientNative(e) = counts_event(&c, false) else {
            panic!()
        };
        assert_eq!(e.level, "info");
        assert_eq!(e.fields["dropped_total"], 0);
    }

    #[test]
    fn tracker_sends_on_change_at_most_once_a_minute_then_once_at_exit() {
        let t0 = Instant::now();
        let mut t = CountsTracker::default();
        assert!(
            t.periodic(t0, &PatchCounts::default()).is_none(),
            "nothing yet"
        );
        let one = parse("[cimmeria-client-patches +1ms] claimed onBMOpen (#1 received)\n");
        assert!(t.periodic(t0, &one).is_some());
        assert!(!t.due(t0 + Duration::from_secs(30)));
        assert!(
            t.periodic(t0 + PERIODIC_INTERVAL, &one).is_none(),
            "unchanged"
        );
        let ten = parse("[cimmeria-client-patches +1ms] claimed onBMOpen (#10 received)\n");
        assert!(t.periodic(t0 + PERIODIC_INTERVAL, &ten).is_some());
        let fin = t.finish(Some(&ten)).expect("final always sent");
        let TelemetryEvent::ClientNative(e) = fin else {
            panic!()
        };
        assert_eq!(e.fields["final"], true);
        assert!(t.finish(Some(&ten)).is_none(), "once");
        assert!(t.periodic(t0 + PERIODIC_INTERVAL * 5, &ten).is_none());
    }

    #[test]
    fn no_final_event_when_the_dll_counted_nothing() {
        let mut t = CountsTracker::default();
        assert!(t.finish(Some(&PatchCounts::default())).is_none());
        let mut t = CountsTracker::default();
        assert!(t.finish(None).is_none());
    }
}
