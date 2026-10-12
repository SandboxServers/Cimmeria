//! Which instances `lab clients stop` stops, refuses or skips, and the lines
//! it prints. Pure: no process is touched and nothing is printed here. Port of
//! `Select-StopTargets`, `Get-LeaseHeld`, `Get-LeaseWho` and the messages of
//! the retired tools/lab/cli/clients.ps1.

use serde_json::Value;

use crate::daemon::status::{InstanceRow, StatusReply};
use crate::ui::{Mark, Tone, Ui};

/// Printed (exit 2) for a missing or unknown verb.
pub const USAGE: &str = "usage: lab clients stop [<instance>|all] [--force]";
/// Printed (exit 2) for `--force` with no instance or `all`.
pub const FORCE_NEEDS_NAME: &str = "--force needs a named instance";
/// Printed (exit 1) when the daemon gives no status.
pub const DAEMON_DOWN: &str =
    "daemon not running (or status endpoint unavailable); nothing stopped";

/// Whether `which` means every instance: absent, empty or `all` (any case).
pub fn is_all(which: Option<&str>) -> bool {
    match which {
        None => true,
        Some(name) => name.is_empty() || name.eq_ignore_ascii_case("all"),
    }
}

/// Instance names compare without regard to case, as PowerShell's `-eq` did.
pub fn same_name(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// Whether the image path names `SGW.exe`: the last path component, any case,
/// the whole name (`SGW.exe.bak` and `notSGW.exe` are not it).
#[cfg_attr(not(windows), allow(dead_code))]
pub fn is_sgw_image(path: &str) -> bool {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
    name.eq_ignore_ascii_case("SGW.exe")
}

/// Whether a session holds the instance's lease (`lease.held` is JSON `true`).
pub fn lease_held(inst: &InstanceRow) -> bool {
    matches!(
        inst.lease.as_ref().and_then(|lease| lease.get("held")),
        Some(Value::Bool(true))
    )
}

/// A JSON field as text: empty for a missing or null field.
fn text(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// `owner (purpose)` of the instance's lease, `-` for a part the status
/// leaves out. Never a lease id.
pub fn lease_who(inst: &InstanceRow) -> String {
    let held = inst.lease.as_ref().and_then(|lease| lease.get("lease"));
    let mut owner = text(held.and_then(|h| h.get("owner")));
    let mut purpose = text(held.and_then(|h| h.get("purpose")));
    if owner.is_empty() {
        owner = "-".to_string();
    }
    if purpose.is_empty() {
        purpose = "-".to_string();
    }
    format!("{owner} ({purpose})")
}

/// The instances a stop names, sorted into buckets.
#[derive(Debug, Default)]
pub struct StopTargets<'a> {
    /// To stop (includes `forced`).
    pub stop: Vec<&'a InstanceRow>,
    /// The leased ones stopped because of `--force`.
    pub forced: Vec<&'a InstanceRow>,
    /// Leased and not stopped.
    pub refuse: Vec<&'a InstanceRow>,
    /// No client.
    pub none: Vec<&'a InstanceRow>,
}

/// Sorts the instances a stop names. `--force` applies only to a named
/// instance, never to all. Port of `Select-StopTargets`.
pub fn select_stop_targets<'a>(
    status: &'a StatusReply,
    which: Option<&str>,
    force: bool,
) -> StopTargets<'a> {
    let all = is_all(which);
    let mut targets = StopTargets::default();
    for inst in &status.instances {
        if !all && !which.is_some_and(|name| same_name(&inst.instance, name)) {
            continue;
        }
        if inst.client_pid.is_none() {
            targets.none.push(inst);
            continue;
        }
        if !lease_held(inst) {
            targets.stop.push(inst);
            continue;
        }
        if force && !all {
            targets.forced.push(inst);
            targets.stop.push(inst);
        } else {
            targets.refuse.push(inst);
        }
    }
    targets
}

/// A line led by a glyph in fancy mode, the bare text in plain mode.
fn marked(ui: &Ui, mark: Mark, tone: Tone, plain: &str, fancy: &str) -> String {
    if ui.fancy {
        format!("{} {fancy}", ui.paint(tone, ui.mark(mark)))
    } else {
        plain.to_string()
    }
}

/// Stderr lines for an error: the bare message in plain mode; `error: ...`
/// and an indented dim `hint: ...` in fancy mode (the `Ui::error` look).
pub fn error_lines(ui: &Ui, message: &str, hint: Option<&str>) -> Vec<String> {
    if !ui.fancy {
        return vec![message.to_string()];
    }
    let mut lines = vec![format!("{} {message}", ui.paint(Tone::Fail, "error:"))];
    if let Some(hint) = hint {
        lines.push(format!(
            "  {}",
            ui.paint(Tone::Dim, &format!("hint: {hint}"))
        ));
    }
    lines
}

/// An instance with no client.
pub fn none_line(ui: &Ui, label: &str) -> String {
    let text = format!("{label}: no client");
    marked(ui, Mark::Stopped, Tone::Dim, &text, &text)
}

/// A leased instance that is not stopped.
pub fn refuse_line(ui: &Ui, label: &str, who: &str) -> String {
    let plain = format!("{label}: leased to {who}; not stopped (use --force to override)");
    let fancy = format!(
        "refused {label}: leased by {who}  {}",
        ui.paint(Tone::Dim, "use --force to override")
    );
    marked(ui, Mark::Fail, Tone::Fail, &plain, &fancy)
}

/// The warning before a leased client is stopped anyway.
pub fn forced_line(ui: &Ui, label: &str, who: &str) -> String {
    let plain = format!(
        "{label}: leased to {who}; stopping anyway (--force). The holder loses the client and the watchdog may relaunch it."
    );
    let fancy = format!(
        "{label}: leased by {who}; stopping anyway (--force). The holder loses the client and the watchdog may relaunch it."
    );
    marked(ui, Mark::Warn, Tone::Warn, &plain, &fancy)
}

/// A client that was stopped.
pub fn stopped_line(ui: &Ui, label: &str, pid: u32) -> String {
    marked(
        ui,
        Mark::Pass,
        Tone::Ok,
        &format!("{label}: stopped pid {pid}"),
        &format!("stopped {label} (pid {pid})"),
    )
}

/// A pid that is not a running `SGW.exe`: nothing was touched.
pub fn not_sgw_line(ui: &Ui, label: &str, pid: u32) -> String {
    let text = format!("{label}: no client (pid {pid} is not a running SGW)");
    marked(ui, Mark::Stopped, Tone::Dim, &text, &text)
}

/// A client still alive after the stop (stderr). `reason` is the error text
/// when the stop could not even be tried (for example "Windows only").
pub fn failed_line(ui: &Ui, label: &str, pid: u32, reason: Option<&str>) -> String {
    let (plain, fancy) = match reason {
        None => (
            format!("{label}: could not stop pid {pid}"),
            format!("could not stop {label} (pid {pid})"),
        ),
        Some(reason) => (
            format!("{label}: could not stop pid {pid}: {reason}"),
            format!("could not stop {label} (pid {pid}): {reason}"),
        ),
    };
    marked(ui, Mark::Fail, Tone::Fail, &plain, &fancy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::status::parse_status;

    /// A /status-shaped reply: `default` is leased and has a client, `p2` is
    /// free with a client, `p3` is free without one.
    fn fake_status() -> StatusReply {
        let body = serde_json::json!({
            "daemon": {"pid": 4242, "version": "0.1.0", "uptime_s": 60},
            "instances": [
                {"instance": "default", "account": "lab", "bridge_port": 8770, "client_pid": 101,
                 "lease": {"held": true, "lease": {"owner": "lp07-lab", "purpose": "UAT", "remaining_s": 600}}},
                {"instance": "p2", "account": "lab2", "bridge_port": 8771, "client_pid": 202,
                 "lease": {"held": false}},
                {"instance": "p3", "account": null, "bridge_port": 8772, "client_pid": null,
                 "lease": {"held": false}}
            ]
        });
        parse_status(&body).expect("the fake status has the daemon's shape")
    }

    fn names(rows: &[&InstanceRow]) -> String {
        rows.iter()
            .map(|r| r.instance.as_str())
            .collect::<Vec<_>>()
            .join(",")
    }

    // ports: 'all, no -Force: only the free client with a client is stopped'
    #[test]
    fn all_without_force_stops_only_the_free_client() {
        let status = fake_status();
        let t = select_stop_targets(&status, Some("all"), false);
        assert_eq!(names(&t.stop), "p2");
    }

    // ports: 'all, no -Force: the leased client is refused'
    #[test]
    fn all_without_force_refuses_the_leased_client() {
        let status = fake_status();
        let t = select_stop_targets(&status, Some("all"), false);
        assert_eq!(names(&t.refuse), "default");
    }

    // ports: 'all, no -Force: the instance without a client is none'
    #[test]
    fn all_without_force_reports_the_clientless_instance_as_none() {
        let status = fake_status();
        let t = select_stop_targets(&status, Some("all"), false);
        assert_eq!(names(&t.none), "p3");
    }

    // ports: 'all, no -Force: nothing is forced'
    #[test]
    fn all_without_force_forces_nothing() {
        let status = fake_status();
        let t = select_stop_targets(&status, Some("all"), false);
        assert!(t.forced.is_empty());
    }

    // ports: 'no instance means all'
    #[test]
    fn no_instance_means_all() {
        let status = fake_status();
        let t = select_stop_targets(&status, None, false);
        assert_eq!(
            (names(&t.refuse), names(&t.stop)),
            ("default".into(), "p2".into())
        );
        let t = select_stop_targets(&status, Some(""), false);
        assert_eq!(names(&t.stop), "p2");
    }

    // ports: 'a leased instance by name, no -Force: refused'
    #[test]
    fn a_leased_instance_by_name_is_refused() {
        let status = fake_status();
        let t = select_stop_targets(&status, Some("default"), false);
        assert_eq!(names(&t.refuse), "default");
        assert!(t.stop.is_empty());
    }

    // ports: 'a leased instance by name with -Force: forced and stopped'
    #[test]
    fn a_leased_instance_by_name_with_force_is_forced_and_stopped() {
        let status = fake_status();
        let t = select_stop_targets(&status, Some("default"), true);
        assert_eq!(
            (names(&t.forced), names(&t.stop)),
            ("default".into(), "default".into())
        );
    }

    // ports: 'a leased instance by name with -Force: not refused'
    #[test]
    fn a_forced_instance_is_not_refused() {
        let status = fake_status();
        let t = select_stop_targets(&status, Some("default"), true);
        assert!(t.refuse.is_empty());
    }

    // ports: '-Force never applies to all'
    #[test]
    fn force_never_applies_to_all() {
        let status = fake_status();
        let t = select_stop_targets(&status, Some("all"), true);
        assert!(t.forced.is_empty());
        assert_eq!(names(&t.refuse), "default");
    }

    // ports: 'a named instance without a client is none'
    #[test]
    fn a_named_instance_without_a_client_is_none() {
        let status = fake_status();
        let t = select_stop_targets(&status, Some("p3"), false);
        assert_eq!(names(&t.none), "p3");
        assert!(t.stop.is_empty());
    }

    // ports: 'a /status without client_pid or lease reads as no client'
    #[test]
    fn a_status_without_client_pid_or_lease_reads_as_no_client() {
        let body = serde_json::json!({
            "daemon": {"pid": 1, "version": "x"},
            "instances": [{"instance": "p4"}]
        });
        let status = parse_status(&body).expect("shape");
        let t = select_stop_targets(&status, Some("all"), false);
        assert_eq!(names(&t.none), "p4");
    }

    // new: PowerShell's -eq ignored case
    #[test]
    fn instance_names_and_all_match_without_regard_to_case() {
        let status = fake_status();
        assert_eq!(
            names(&select_stop_targets(&status, Some("P2"), false).stop),
            "p2"
        );
        assert_eq!(
            names(&select_stop_targets(&status, Some("ALL"), false).stop),
            "p2"
        );
        assert!(is_all(Some("All")));
        assert!(!is_all(Some("p2")));
    }

    // new: Get-LeaseWho
    #[test]
    fn lease_who_names_owner_and_purpose_and_fills_gaps_with_a_dash() {
        let status = fake_status();
        assert_eq!(lease_who(&status.instances[0]), "lp07-lab (UAT)");
        assert_eq!(lease_who(&status.instances[1]), "- (-)");
        let body = serde_json::json!({
            "daemon": {"pid": 1, "version": "x"},
            "instances": [{"instance": "a", "lease": {"held": true, "lease": {"owner": "o"}}}]
        });
        let one = parse_status(&body).expect("shape");
        assert_eq!(lease_who(&one.instances[0]), "o (-)");
    }

    // new: only the whole image name SGW.exe counts
    #[test]
    fn only_an_image_named_sgw_exe_is_a_client() {
        assert!(is_sgw_image(r"C:\Game\Binaries\SGW.exe"));
        assert!(is_sgw_image(r"c:\game\sgw.EXE"));
        assert!(is_sgw_image("SGW.exe"));
        assert!(!is_sgw_image(r"C:\x\SGW.exe.bak"));
        assert!(!is_sgw_image(r"C:\x\notSGW.exe"));
        assert!(!is_sgw_image(r"C:\x\SGW"));
    }

    #[test]
    fn plain_lines_are_the_powershell_strings_with_the_flag_respelled() {
        let ui = Ui::plain();
        assert_eq!(none_line(&ui, "p3"), "p3: no client");
        assert_eq!(
            refuse_line(&ui, "default", "lp07-lab (UAT)"),
            "default: leased to lp07-lab (UAT); not stopped (use --force to override)"
        );
        assert_eq!(stopped_line(&ui, "p2", 202), "p2: stopped pid 202");
        assert_eq!(
            not_sgw_line(&ui, "p2", 202),
            "p2: no client (pid 202 is not a running SGW)"
        );
        assert_eq!(
            failed_line(&ui, "p2", 202, None),
            "p2: could not stop pid 202"
        );
        assert_eq!(error_lines(&ui, USAGE, None), vec![USAGE.to_string()]);
    }
}
