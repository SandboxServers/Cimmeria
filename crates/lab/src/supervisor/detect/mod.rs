//! The C12 detectors: rules over what the daemon already holds (the
//! `client_watch` samples, the heartbeat history, the held keys and the
//! recent `lab.call` rows) that name a fault in a run. Pure: the caller
//! passes the inputs and the clock, so the rules are tested without a client.
//! `lab_detectors_read` ([`crate::server`]) is the only caller.

pub mod calls;
pub mod input;
pub mod load;
pub mod motion;
pub mod timing;

use serde_json::Value;

use crate::supervisor::watch_store::WatchSample;
use crate::timeline::client_events::HeartbeatSample;
use calls::CallRow;

/// One detector firing: its C12 name, when (daemon wall ms), the client frame
/// if the samples carry one, and the numbers behind it.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Fired {
    pub name: &'static str,
    pub at_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<u64>,
    pub detail: Value,
}

/// Everything the detectors read. `holds` are key name and hold start (daemon
/// wall ms); `now_ms` is the window's end.
pub struct Inputs<'a> {
    pub samples: &'a [WatchSample],
    pub heartbeats: &'a [HeartbeatSample],
    pub holds: &'a [(String, i64)],
    pub calls: &'a [CallRow],
    pub now_ms: i64,
}

/// Run the detectors in C12 table order and sort the firings by time. With no
/// watch samples only the heartbeat hitch and the call-error rule run.
pub fn run_all(i: &Inputs) -> Vec<Fired> {
    let mut out = Vec::new();
    if i.samples.is_empty() {
        out.extend(timing::hitch_heartbeat(i.heartbeats));
    } else {
        out.extend(timing::hitch_watch(i.samples));
        out.extend(motion::teleport(i.samples));
        out.extend(motion::long_fall(i.samples));
        out.extend(motion::out_of_world(i.samples));
        out.extend(motion::nan(i.samples));
        out.extend(input::stuck_input(i.samples, i.holds, i.now_ms));
        out.extend(load::actor_growth(i.samples));
    }
    out.extend(calls::log_error(i.calls, i.now_ms));
    out.sort_by_key(|f| f.at_ms);
    out
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
