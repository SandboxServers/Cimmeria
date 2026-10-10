//! `GET /status`: a read-only JSON summary of the daemon and every hosted
//! lab instance, for the `lab` CLI (docs/analysis/lab-cli/). Behind the
//! same bearer gate as `/mcp`; never carries a lease id.

use std::sync::Arc;

use axum::extract::State;
use serde_json::{json, Value};

use crate::server::LabServer;

/// When this daemon process started (set once in `build_router`).
pub struct Started {
    pub at_ms: i64,
    pub pid: u32,
}

/// One instance row (pure, unit-tested).
pub fn instance_row(
    label: &str,
    account: Option<String>,
    bridge_port: u16,
    client_pid: Option<u32>,
    lease: Value,
) -> Value {
    json!({
        "instance": label,
        "account": account,
        "bridge_port": bridge_port,
        "client_pid": client_pid,
        "lease": lease,
    })
}

/// The whole body (pure, unit-tested).
pub fn body(started: &Started, now_ms: i64, rows: Vec<Value>) -> Value {
    json!({
        "daemon": {
            "pid": started.pid,
            "version": env!("CARGO_PKG_VERSION"),
            "started_at": crate::lease::rfc3339(started.at_ms),
            "uptime_s": (now_ms - started.at_ms).max(0) / 1000,
        },
        "instances": rows,
    })
}

/// The axum handler.
pub async fn handler(
    State((server, started)): State<(LabServer, Arc<Started>)>,
) -> axum::Json<Value> {
    let mut rows = Vec::new();
    for h in server.instances().iter() {
        rows.push(instance_row(
            &h.label,
            h.supervisor.account_name(),
            h.supervisor.port(),
            h.supervisor.client_pid().await,
            h.supervisor.leases().status(),
        ));
    }
    axum::Json(body(&started, crate::supervisor::now_ms(), rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_row_has_the_contract_shape() {
        let lease = json!({ "held": false });
        let row = instance_row("p2", Some("lab2".into()), 8771, None, lease.clone());
        assert_eq!(row["instance"], "p2");
        assert_eq!(row["account"], "lab2");
        assert_eq!(row["bridge_port"], 8771);
        assert!(row["client_pid"].is_null());
        assert_eq!(row["lease"], lease);

        let row = instance_row("default", None, 8770, Some(49448), lease);
        assert!(row["account"].is_null());
        assert_eq!(row["client_pid"], 49448);
    }

    #[test]
    fn body_reports_uptime_and_version() {
        let started = Started {
            at_ms: 1_000,
            pid: 7,
        };
        let b = body(&started, 1_000 + 61_500, vec![]);
        assert_eq!(b["daemon"]["pid"], 7);
        assert_eq!(b["daemon"]["uptime_s"], 61);
        assert_eq!(b["daemon"]["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(b["daemon"]["started_at"], crate::lease::rfc3339(1_000));
        assert_eq!(b["instances"], json!([]));

        // A clock that steps backwards never reports negative uptime.
        let b = body(&started, 0, vec![]);
        assert_eq!(b["daemon"]["uptime_s"], 0);
    }
}
