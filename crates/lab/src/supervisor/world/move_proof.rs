//! Judging `client_move_to` (C1) and explaining its stops (C8). Pure: plain
//! values in, checks and reports out, so every branch is a unit test.

use serde_json::json;

use super::geometry::{client_to_server, horizontal_m, server_to_client, Vec3};
use crate::proof::stop::{StopKind, StopPlayer, StopReport};
use crate::proof::{Check, Source, Why};

/// The server may place the player this much further from the goal than the
/// client does: the two positions are read a tick apart and rounded differently.
const SERVER_SLACK_M: f64 = 1.0;
/// A client walk shorter than this proves nothing about a rubber band.
const RUBBER_MIN_CLIENT_M: f64 = 1.0;
/// A server end this close to its start means the server never moved the player.
const RUBBER_SERVER_M: f64 = 0.5;

/// The checks for a finished walk. All positions are client space (UE3 units)
/// except the two server ones, which are server metres.
///
/// `server_arrived` is required (D-AX11): a client that walked into a wall
/// while the server held the player elsewhere is not an arrival.
pub(super) fn arrival_checks(
    goal_client: Vec3,
    start_client: Vec3,
    end_client: Vec3,
    arrival_m: f64,
    start_server: Option<Vec3>,
    end_server: Option<Vec3>,
) -> Vec<Check> {
    let mut checks = vec![Check::new(
        "client_arrived",
        Source::Client,
        Some(horizontal_m(end_client, goal_client) <= arrival_m),
        true,
    )
    .with_before(client_to_server(goal_client).to_json())
    .with_after(client_to_server(end_client).to_json())
    .with_why(Why::NoEffect)];

    let server_ok = end_server
        .map(|p| horizontal_m(server_to_client(p), goal_client) <= arrival_m + SERVER_SLACK_M);
    let mut server = Check::new("server_arrived", Source::Server, server_ok, true)
        .with_before(client_to_server(goal_client).to_json())
        .with_why(if end_server.is_some() {
            Why::ServerDisagrees
        } else {
            Why::ServerUnreachable
        });
    if let Some(p) = end_server {
        server = server.with_after(p.to_json());
    }
    checks.push(server);

    if let (Some(s0), Some(s1)) = (start_server, end_server) {
        if horizontal_m(start_client, end_client) >= RUBBER_MIN_CLIENT_M {
            let moved = horizontal_m(server_to_client(s0), server_to_client(s1));
            checks.push(
                Check::new(
                    "no_rubber_band",
                    Source::Server,
                    Some(moved > RUBBER_SERVER_M),
                    true,
                )
                .with_before(s0.to_json())
                .with_after(s1.to_json())
                .with_why(Why::ServerDisagrees),
            );
        }
    }
    checks
}

/// A stop report (C8). Both positions are client space; the report is in
/// server metres.
pub(super) fn stop_report(
    kind: StopKind,
    step: &str,
    seconds: f64,
    tried: Vec<String>,
    at_client: Vec3,
    now_client: Vec3,
) -> StopReport {
    let arr = |v: Vec3| {
        let s = client_to_server(v);
        [s.x, s.y, s.z]
    };
    let suggest: &[&str] = match kind {
        StopKind::Stuck => &["unstick", "release_input"],
        StopKind::IgnoredInput => &["release_input", "close_dialogs"],
        StopKind::Timeout | StopKind::Stalled => &["unstick"],
        StopKind::BlockedUi => &["close_dialogs"],
        StopKind::Ledge => &[],
    };
    StopReport {
        kind,
        step: step.to_string(),
        seconds,
        tried,
        player: Some(StopPlayer::new(arr(at_client), arr(now_client))),
        suggest: suggest.iter().map(|s| s.to_string()).collect(),
    }
}

/// The names of what the walk tried: the leg's unsticks, plus the input probe.
pub(super) fn tried_list(unsticks: &[&str], probed: bool) -> Vec<String> {
    let mut v: Vec<String> = unsticks.iter().map(|s| s.to_string()).collect();
    if probed {
        v.push("input_probe".to_string());
    }
    v
}

/// The `server_end` output field.
pub(super) fn server_end_json(end: Option<Vec3>) -> serde_json::Value {
    end.map_or(json!(null), |p| p.to_json())
}

#[cfg(test)]
#[path = "move_to_proof_tests.rs"]
mod sim_tests;
#[cfg(test)]
#[path = "move_proof_tests.rs"]
mod tests;
