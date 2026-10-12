//! The ten doctor checks against `Inputs` fixtures. Ports the doctor checks
//! of tools/lab/cli/test-ops.ps1 one-to-one (the PowerShell check is named in
//! each `// ops:` comment) and pins the plain and fancy output.

use std::path::PathBuf;

use cimmeria_lab_cli::commands::doctor::checks::{
    account_check, binary_check, daemon_check, install_check, profile_root_check, run_all,
    seed_check, stray_sgw_check, task_check, token_check, version_check, BinaryFacts, Check,
    ContentCheck, Inputs, InstanceFacts, Status,
};
use cimmeria_lab_cli::commands::doctor::{exit_code, render};
use cimmeria_lab_cli::daemon::status::{DaemonRow, InstanceRow, StatusReply};
use cimmeria_lab_cli::ui::Ui;

const INSTALL: &str = "C:\\Games\\SGW";

fn status() -> StatusReply {
    StatusReply {
        daemon: DaemonRow {
            pid: 4242,
            version: "0.1.0".to_string(),
            started_at: None,
            uptime_s: Some(60),
        },
        instances: vec![InstanceRow {
            instance: "default".to_string(),
            account: None,
            bridge_port: Some(8770),
            client_pid: Some(101),
            lease: None,
            uat: None,
        }],
    }
}

fn inst(label: &str, account_file: bool, seeded: bool) -> InstanceFacts {
    InstanceFacts {
        label: label.to_string(),
        account_file,
        seeded,
    }
}

/// Everything healthy: every check passes.
fn good() -> Inputs {
    Inputs {
        task_installed: true,
        status: Some(status()),
        token_set: true,
        install_dir: Some(INSTALL.to_string()),
        sgw_exe_exists: true,
        profile_root: "C:\\Labs\\instances".to_string(),
        instances: vec![inst("default", true, true)],
        running_sgw: vec![101],
        listed_sgw: vec![101],
        binaries: BinaryFacts::Same(10),
        version_sha: Some("abc1234".to_string()),
        checkout: Some(PathBuf::from("C:/repo")),
        content: Some(ContentCheck::Same),
    }
}

fn root(i: &mut Inputs, root: &str) -> Check {
    i.profile_root = root.to_string();
    profile_root_check(i)
}

#[test]
fn the_ten_checks_run_in_the_powershell_order_and_all_pass_when_healthy() {
    let results = run_all(&good());
    let names: Vec<&str> = results.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "scheduled task CimmeriaLabDaemon exists",
            "daemon answers /status",
            "CIMMERIA_LAB_DAEMON_TOKEN is set",
            "labd.env has CIMMERIA_LAB_INSTALL_DIR and SGW.exe is under it",
            "profile root is absolute and not inside the install dir",
            "each instance's account file exists",
            "each instance's profile is seeded",
            "no SGW.exe runs that /status does not list",
            "installed cimmeria-lab.exe is the daemon's copy",
            "CLI copy matches origin/main (as of the last fetch)",
        ]
    );
    assert!(results.iter().all(|c| c.status == Status::Pass));
}

// ops: 'a profile root inside the install dir FAILs'
#[test]
fn profile_root_inside_the_install_dir_fails() {
    let c = root(&mut good(), "C:\\Games\\SGW\\instances");
    assert_eq!(c.status, Status::Fail);
    assert_eq!(
        c.plain_detail(),
        "inside the install dir: C:\\Games\\SGW\\instances"
    );
}

// ops: 'a profile root outside the install dir PASSes'
#[test]
fn profile_root_outside_the_install_dir_passes() {
    let c = root(&mut good(), "C:\\Labs\\instances");
    assert_eq!(c.status, Status::Pass);
    assert_eq!(c.plain_detail(), "C:\\Labs\\instances");
}

// ops: 'a name that only starts with the install dir is outside it'
#[test]
fn a_name_that_only_starts_with_the_install_dir_is_outside_it() {
    let c = root(&mut good(), "C:\\Games\\SGWX\\instances");
    assert_eq!(c.status, Status::Pass);
}

// ops: 'a relative profile root FAILs'
#[test]
fn a_relative_profile_root_fails() {
    let c = root(&mut good(), "instances");
    assert_eq!(c.status, Status::Fail);
    assert_eq!(c.plain_detail(), "not an absolute path: instances");
}

// New: GetFullPath rules (case, `/`, trailing `\`, `..`) decide "inside".
#[test]
fn profile_root_is_compared_after_normalising() {
    let c = root(&mut good(), "C:/games/sgw/instances/");
    assert_eq!(c.status, Status::Fail);
    assert_eq!(
        c.plain_detail(),
        "inside the install dir: C:\\games\\sgw\\instances"
    );
    let c = root(&mut good(), "C:\\Games\\SGW\\..\\Labs");
    assert_eq!(c.status, Status::Pass);
    assert_eq!(c.plain_detail(), "C:\\Games\\Labs");
    let c = root(&mut good(), "C:\\Games\\SGW");
    assert_eq!(
        c.status,
        Status::Fail,
        "the root equal to the install dir is inside it"
    );
}

// New: with no install dir only the absolute rule applies; an empty root fails.
#[test]
fn profile_root_without_an_install_dir_only_needs_to_be_absolute() {
    let mut i = good();
    i.install_dir = None;
    assert_eq!(
        root(&mut i, "C:\\Games\\SGW\\instances").status,
        Status::Pass
    );
    assert_eq!(root(&mut i, "").plain_detail(), "not an absolute path: ");
}

// ops: 'a missing scheduled task FAILs' / 'an installed scheduled task PASSes'
#[test]
fn the_scheduled_task_check() {
    let mut i = good();
    assert_eq!(task_check(&i).status, Status::Pass);
    assert_eq!(task_check(&i).plain_detail(), "installed");
    i.task_installed = false;
    let c = task_check(&i);
    assert_eq!(c.status, Status::Fail);
    assert_eq!(c.plain_detail(), "not installed; run lab install");
}

// ops: 'no /status FAILs the daemon check' / 'a /status PASSes the daemon check'
#[test]
fn the_daemon_check() {
    let mut i = good();
    let c = daemon_check(&i);
    assert_eq!(c.status, Status::Pass);
    assert_eq!(c.plain_detail(), "pid 4242, version 0.1.0");
    i.status = None;
    let c = daemon_check(&i);
    assert_eq!(c.status, Status::Fail);
    assert_eq!(
        c.plain_detail(),
        "not running, or the status endpoint is down; run lab start"
    );
}

// ops: 'an unset token FAILs' / 'a set token PASSes'
#[test]
fn the_token_check_reports_set_or_not_set_only() {
    let mut i = good();
    let c = token_check(&i);
    assert_eq!(
        (c.status, c.plain_detail()),
        (Status::Pass, "set".to_string())
    );
    i.token_set = false;
    let c = token_check(&i);
    assert_eq!(c.status, Status::Fail);
    assert_eq!(c.plain_detail(), "not set; run lab install");
}

// ops: 'no install dir FAILs'
#[test]
fn no_install_dir_fails() {
    let mut i = good();
    i.install_dir = None;
    let c = install_check(&i);
    assert_eq!(c.status, Status::Fail);
    assert_eq!(
        c.plain_detail(),
        "CIMMERIA_LAB_INSTALL_DIR is not set in labd.env"
    );
}

// ops: 'an install dir without SGW.exe FAILs' / '... with SGW.exe PASSes'
#[test]
fn the_install_dir_needs_sgw_exe() {
    let mut i = good();
    i.sgw_exe_exists = false;
    let c = install_check(&i);
    assert_eq!(c.status, Status::Fail);
    assert_eq!(c.plain_detail(), "no C:\\Games\\SGW\\Binaries\\SGW.exe");
    i.sgw_exe_exists = true;
    let c = install_check(&i);
    assert_eq!(c.status, Status::Pass);
    assert_eq!(c.plain_detail(), "C:\\Games\\SGW\\Binaries\\SGW.exe");
}

// ops: 'a missing p2 account file WARNs, naming p2' / 'the default account alone PASSes'
#[test]
fn the_account_check_names_the_missing_instance() {
    let mut i = good();
    i.instances = vec![inst("default", true, true), inst("p2", false, true)];
    let c = account_check(&i);
    assert_eq!(c.status, Status::Warn);
    assert_eq!(c.plain_detail(), "missing: p2; run lab instances init");
    i.instances = vec![inst("default", true, true)];
    let c = account_check(&i);
    assert_eq!(
        (c.status, c.plain_detail()),
        (Status::Pass, "1 present".to_string())
    );
}

// New (PowerShell branch no test covered): no install dir warns, files not checked.
#[test]
fn the_account_check_without_an_install_dir_warns() {
    let mut i = good();
    i.install_dir = None;
    let c = account_check(&i);
    assert_eq!(c.status, Status::Warn);
    assert_eq!(
        c.plain_detail(),
        "no install dir; account files not checked"
    );
}

// ops: 'an unseeded p2 WARNs, naming p2' / 'a seeded default PASSes'
#[test]
fn the_seed_check_names_the_unseeded_instance() {
    let mut i = good();
    i.instances = vec![inst("default", true, true), inst("p2", true, false)];
    let c = seed_check(&i);
    assert_eq!(c.status, Status::Warn);
    assert_eq!(c.plain_detail(), "not seeded: p2");
    i.instances = vec![inst("default", true, true)];
    let c = seed_check(&i);
    assert_eq!(
        (c.status, c.plain_detail()),
        (Status::Pass, "1 seeded".to_string())
    );
}

// ops: 'an SGW.exe pid that /status does not list WARNs, naming it'
#[test]
fn an_unlisted_sgw_pid_warns_naming_it() {
    let mut i = good();
    i.running_sgw = vec![10, 20];
    i.listed_sgw = vec![10];
    let c = stray_sgw_check(&i);
    assert_eq!(c.status, Status::Warn);
    assert_eq!(c.plain_detail(), "not listed by /status: pid 20");
}

// ops: 'a listed SGW.exe pid PASSes' and 'no SGW.exe PASSes'
#[test]
fn listed_or_no_sgw_passes() {
    let mut i = good();
    i.running_sgw = vec![10];
    i.listed_sgw = vec![10];
    assert_eq!(stray_sgw_check(&i).status, Status::Pass);
    i.running_sgw = vec![];
    i.listed_sgw = vec![];
    let c = stray_sgw_check(&i);
    assert_eq!(
        (c.status, c.plain_detail()),
        (Status::Pass, "none".to_string())
    );
}

// ops: 'with the daemon down, a running SGW.exe WARNs as unmatched'
#[test]
fn with_the_daemon_down_a_running_sgw_warns_as_unmatched() {
    let mut i = good();
    i.status = None;
    i.running_sgw = vec![30];
    i.listed_sgw = vec![];
    let c = stray_sgw_check(&i);
    assert_eq!(c.status, Status::Warn);
    assert_eq!(
        c.plain_detail(),
        "daemon down; running SGW pid 30 cannot be matched to an instance"
    );
}

// New: several stray pids are listed in order, comma separated.
#[test]
fn several_stray_pids_are_listed_in_order() {
    let mut i = good();
    i.running_sgw = vec![30, 10, 20];
    i.listed_sgw = vec![10];
    assert_eq!(
        stray_sgw_check(&i).plain_detail(),
        "not listed by /status: pid 30, 20"
    );
}

// ops: 'differing bin and labd copies WARN'
#[test]
fn differing_binaries_warn() {
    let mut i = good();
    i.binaries = BinaryFacts::Differ;
    let c = binary_check(&i);
    assert_eq!(c.status, Status::Warn);
    assert_eq!(
        c.plain_detail(),
        "bin and labd copies differ; run lab restart"
    );
}

// ops: 'identical bin and labd copies PASS'
#[test]
fn identical_binaries_pass() {
    let mut i = good();
    i.binaries = BinaryFacts::Same(1234);
    let c = binary_check(&i);
    assert_eq!(c.status, Status::Pass);
    assert_eq!(c.plain_detail(), "same bytes (1234 bytes)");
}

// ops: 'a missing copy WARNs'
#[test]
fn a_missing_binary_warns() {
    let mut i = good();
    i.binaries = BinaryFacts::Missing(PathBuf::from("C:\\lab\\labd\\cimmeria-lab.exe"));
    let c = binary_check(&i);
    assert_eq!(c.status, Status::Warn);
    assert_eq!(
        c.plain_detail(),
        "missing: C:\\lab\\labd\\cimmeria-lab.exe; run lab install"
    );
}

// ops: 'no VERSION inside a checkout WARNs'
#[test]
fn no_version_file_inside_a_checkout_warns() {
    let mut i = good();
    i.version_sha = None;
    i.content = None;
    let c = version_check(&i);
    assert_eq!(c.status, Status::Warn);
    assert_eq!(c.plain_detail(), "no cli\\cli\\VERSION; run lab setup");
}

// ops: 'outside a checkout the VERSION check is skipped (PASS)'
#[test]
fn outside_a_checkout_the_version_check_is_skipped() {
    let mut i = good();
    i.checkout = None;
    i.content = None;
    let c = version_check(&i);
    assert_eq!(c.status, Status::Pass);
    assert_eq!(c.plain_detail(), "not in a Cimmeria checkout; skipped");
}

// ops: "origin/main's own sha PASSes"
#[test]
fn a_sha_with_origin_mains_tools_lab_passes() {
    let c = version_check(&good());
    assert_eq!(c.status, Status::Pass);
    assert_eq!(c.plain_detail(), "abc1234 has origin/main's tools/lab");
}

// ops: 'a sha whose tools/lab differs from origin/main WARNs'
#[test]
fn a_sha_whose_tools_lab_differs_warns() {
    let mut i = good();
    i.content = Some(ContentCheck::Differs);
    let c = version_check(&i);
    assert_eq!(c.status, Status::Warn);
    assert_eq!(
        c.plain_detail(),
        "abc1234 differs from origin/main in tools/lab; run lab setup"
    );
}

// ops: 'an unknown sha WARNs as unknown'
#[test]
fn an_unknown_sha_warns_as_unknown() {
    let mut i = good();
    i.version_sha = Some("deadbee".to_string());
    i.content = Some(ContentCheck::Unknown);
    let c = version_check(&i);
    assert_eq!(c.status, Status::Warn);
    assert_eq!(
        c.plain_detail(),
        "deadbee is unknown in C:/repo; git fetch, or run lab setup"
    );
}

fn fixture_results() -> Vec<Check> {
    let mut i = good();
    i.token_set = false;
    i.instances = vec![inst("default", true, true), inst("p3", true, false)];
    vec![task_check(&i), seed_check(&i), token_check(&i)]
}

// Plain mode is the PowerShell text: STATUS, two spaces, check, two spaces,
// detail (hint after "; "), no count line.
#[test]
fn plain_output_is_one_powershell_line_per_check() {
    assert_eq!(
        render(&Ui::plain(), &fixture_results()),
        "PASS  scheduled task CimmeriaLabDaemon exists  installed\n\
         WARN  each instance's profile is seeded  not seeded: p3\n\
         FAIL  CIMMERIA_LAB_DAEMON_TOKEN is set  not set; run lab install\n"
    );
}

// Fancy mode (glyphs, no colour codes here): glyph + name, detail and hint
// indented under a warning or failure, then the count line.
#[test]
fn fancy_output_has_glyphs_hints_and_a_count_line() {
    let ui = Ui {
        color: false,
        fancy: true,
        width: None,
    };
    assert_eq!(
        render(&ui, &fixture_results()),
        "✓ scheduled task CimmeriaLabDaemon exists\n\
         ! each instance's profile is seeded\n\
         \x20   not seeded: p3\n\
         ✗ CIMMERIA_LAB_DAEMON_TOKEN is set\n\
         \x20   not set\n\
         \x20   hint: run lab install\n\
         3 checks · 1 passed · 1 warning · 1 failed\n"
    );
}

#[test]
fn the_count_line_pluralises_warnings() {
    let ui = Ui {
        color: false,
        fancy: true,
        width: None,
    };
    let text = render(&ui, &[task_check(&good())]);
    assert_eq!(
        text,
        "✓ scheduled task CimmeriaLabDaemon exists\n1 checks · 1 passed · 0 warnings · 0 failed\n"
    );
}

// The exit code: 1 for any FAIL, 0 for warnings alone.
#[test]
fn exit_code_is_1_only_when_a_check_fails() {
    let results = fixture_results();
    assert_eq!(exit_code(&results), 1);
    assert_eq!(exit_code(&results[..2]), 0, "a warning does not fail");
    assert_eq!(exit_code(&run_all(&good())), 0);
}
