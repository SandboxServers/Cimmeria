//! Lab paths, labd.pid and the token picker. Ports the common.ps1 checks of
//! the retired tools/lab/cli/test-common.ps1 (the PowerShell check is named in each
//! comment).

mod support;

use std::path::Path;

use cimmeria_lab_cli::daemon::paths::{self, DaemonInfo};
use cimmeria_lab_cli::daemon::token;
use support::{env_lock, EnvGuard};
use tempfile::TempDir;

const HOME: &str = "CIMMERIA_LAB_HOME";
const LOCAL: &str = "LOCALAPPDATA";
const PROFILE: &str = "CIMMERIA_LAB_PROFILE_ROOT";
const UAT: &str = "CIMMERIA_LAB_UAT_DIR";

fn s(path: &Path) -> &str {
    path.to_str().expect("utf-8 path")
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, text).expect("write");
}

// 'CIMMERIA_LAB_HOME overrides the lab home'
#[test]
fn lab_home_env_override() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let _env = EnvGuard::set(&[(HOME, Some(s(&home)))]);
    assert_eq!(paths::lab_home(), home);
}

// 'labd.env lives under the lab home' (plus the pid, log and cli paths)
#[test]
fn lab_files_live_under_the_lab_home() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let _env = EnvGuard::set(&[(HOME, Some(s(&home)))]);
    assert_eq!(paths::labd_env_path(), home.join("labd.env"));
    assert_eq!(paths::labd_pid_path(), home.join("labd.pid"));
    assert_eq!(paths::labd_log_path(), home.join("labd.log"));
    assert_eq!(paths::installed_cli_dir(), home.join("cli"));
}

// 'the default lab home is %LOCALAPPDATA%\cimmeria-lab'
#[test]
fn default_lab_home_is_under_localappdata() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    let local = tmp.path().join("local");
    let _env = EnvGuard::set(&[(HOME, None), (LOCAL, Some(s(&local)))]);
    assert_eq!(paths::lab_home(), local.join("cimmeria-lab"));
}

// New: an empty override is no override (PowerShell's `if ($env:X)`).
#[test]
fn empty_lab_home_override_is_ignored() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    let local = tmp.path().join("local");
    let _env = EnvGuard::set(&[(HOME, Some("")), (LOCAL, Some(s(&local)))]);
    assert_eq!(paths::lab_home(), local.join("cimmeria-lab"));
}

// 'labd.env wins over the environment'
#[test]
fn labd_env_beats_the_environment_for_the_profile_root() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    write(
        &home.join("labd.env"),
        "CIMMERIA_LAB_PROFILE_ROOT=D:\\from-file\r\n",
    );
    let _env = EnvGuard::set(&[(HOME, Some(s(&home))), (PROFILE, Some("D:\\from-env"))]);
    assert_eq!(paths::profile_root(), Path::new("D:\\from-file"));
}

// 'the environment is used without labd.env'
#[test]
fn environment_is_used_without_labd_env() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let _env = EnvGuard::set(&[(HOME, Some(s(&home))), (PROFILE, Some("D:\\from-env"))]);
    assert_eq!(paths::profile_root(), Path::new("D:\\from-env"));
}

// 'the default profile root is under LOCALAPPDATA'
#[test]
fn default_profile_root_is_under_localappdata() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let local = tmp.path().join("local");
    let _env = EnvGuard::set(&[
        (HOME, Some(s(&home))),
        (LOCAL, Some(s(&local))),
        (PROFILE, None),
    ]);
    assert_eq!(
        paths::profile_root(),
        local.join("cimmeria-lab").join("instances")
    );
}

// New: a blank labd.env value falls through to the environment.
#[test]
fn blank_labd_env_value_falls_through_to_the_environment() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    write(&home.join("labd.env"), "CIMMERIA_LAB_PROFILE_ROOT=   \r\n");
    let _env = EnvGuard::set(&[(HOME, Some(s(&home))), (PROFILE, Some("  D:\\env  "))]);
    assert_eq!(paths::profile_root(), Path::new("D:\\env"));
}

// New (Get-UatRoot): labd.env, then the environment, then <home>\uat-runs.
#[test]
fn uat_root_prefers_labd_env() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    write(
        &home.join("labd.env"),
        "CIMMERIA_LAB_UAT_DIR=E:\\file-uat\r\n",
    );
    let _env = EnvGuard::set(&[(HOME, Some(s(&home))), (UAT, Some("E:\\env-uat"))]);
    assert_eq!(paths::uat_root(), Path::new("E:\\file-uat"));
}

#[test]
fn uat_root_uses_the_environment_without_labd_env() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let _env = EnvGuard::set(&[(HOME, Some(s(&home))), (UAT, Some("E:\\env-uat"))]);
    assert_eq!(paths::uat_root(), Path::new("E:\\env-uat"));
}

#[test]
fn uat_root_defaults_to_uat_runs_under_the_lab_home() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let _env = EnvGuard::set(&[(HOME, Some(s(&home))), (UAT, None)]);
    assert_eq!(paths::uat_root(), home.join("uat-runs"));
}

// 'Get-DaemonInfo reads labd.pid'
#[test]
fn daemon_info_reads_labd_pid() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    write(
        &home.join("labd.pid"),
        "{\"pid\":4242,\"bind\":\"127.0.0.1:9\"}",
    );
    let _env = EnvGuard::set(&[(HOME, Some(s(&home)))]);
    assert_eq!(
        paths::daemon_info(),
        DaemonInfo {
            pid: Some(4242),
            bind: "127.0.0.1:9".to_string()
        }
    );
}

// New: no pidfile, bad JSON, or no bind all give the default bind.
#[test]
fn daemon_info_defaults_when_the_pidfile_is_absent() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    let _env = EnvGuard::set(&[(HOME, Some(s(&tmp.path().join("none"))))]);
    assert_eq!(
        paths::daemon_info(),
        DaemonInfo {
            pid: None,
            bind: "127.0.0.1:8779".to_string()
        }
    );
}

#[test]
fn daemon_info_defaults_when_the_pidfile_is_not_json() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    write(&home.join("labd.pid"), "not json {");
    let _env = EnvGuard::set(&[(HOME, Some(s(&home)))]);
    assert_eq!(paths::daemon_info().bind, "127.0.0.1:8779");
    assert_eq!(paths::daemon_info().pid, None);
}

#[test]
fn daemon_info_uses_the_default_bind_when_the_pidfile_names_none() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    write(&home.join("labd.pid"), "{\"pid\":7,\"bind\":\"  \"}");
    let _env = EnvGuard::set(&[(HOME, Some(s(&home)))]);
    assert_eq!(
        paths::daemon_info(),
        DaemonInfo {
            pid: Some(7),
            bind: "127.0.0.1:8779".to_string()
        }
    );
}

// New (Get-DefaultSpecsDir): the checkout's spec folder, nearest ancestor.
#[test]
fn default_specs_dir_from_finds_the_checkout_above_the_start() {
    let tmp = TempDir::new().unwrap();
    let specs = tmp.path().join("docs").join("guides").join("uat-specs");
    std::fs::create_dir_all(&specs).unwrap();
    let start = tmp.path().join("target").join("release");
    std::fs::create_dir_all(&start).unwrap();
    assert_eq!(paths::default_specs_dir_from(&start), Some(specs));
}

#[test]
fn default_specs_dir_from_is_none_outside_a_checkout() {
    let tmp = TempDir::new().unwrap();
    let start = tmp.path().join("bin");
    std::fs::create_dir_all(&start).unwrap();
    assert_eq!(paths::default_specs_dir_from(&start), None);
}

// New (Get-LabToken): the user value wins, then the process value, empty is none.
#[test]
fn pick_token_prefers_the_user_value() {
    assert_eq!(
        token::pick_token(Some("user".into()), Some("process".into())),
        Some("user".to_string())
    );
}

#[test]
fn pick_token_falls_back_to_the_process_value() {
    assert_eq!(
        token::pick_token(None, Some("process".into())),
        Some("process".to_string())
    );
    assert_eq!(
        token::pick_token(Some(String::new()), Some("process".into())),
        Some("process".to_string())
    );
}

#[test]
fn pick_token_treats_empty_and_missing_as_none() {
    assert_eq!(token::pick_token(None, None), None);
    assert_eq!(
        token::pick_token(Some(String::new()), Some(String::new())),
        None
    );
}

// A lab home override is a test run: the registry (the real daemon's token)
// must not be read, whatever it holds.
#[test]
fn token_with_a_home_override_never_reads_the_user_environment() {
    let mut read = false;
    let got = token::token_with(
        true,
        || {
            read = true;
            Some("real-token-from-registry".into())
        },
        Some("process".into()),
    );
    assert!(!read, "the user environment was read");
    assert_eq!(got, Some("process".to_string()));
    assert_eq!(token::token_with(true, || None, None), None);
}

#[test]
fn token_without_a_home_override_reads_the_user_environment_first() {
    let got = token::token_with(false, || Some("user".into()), Some("process".into()));
    assert_eq!(got, Some("user".to_string()));
}

#[test]
fn lab_home_overridden_follows_the_variable() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    {
        let _env = EnvGuard::set(&[(HOME, Some(s(tmp.path())))]);
        assert!(paths::lab_home_overridden());
    }
    {
        let _env = EnvGuard::set(&[(HOME, Some(""))]);
        assert!(!paths::lab_home_overridden());
    }
    let _env = EnvGuard::set(&[(HOME, None)]);
    assert!(!paths::lab_home_overridden());
}

// With the override set, daemon_token is the process variable alone, so a
// machine's real registry token can never reach a test.
#[test]
fn daemon_token_with_a_home_override_is_the_process_variable() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    {
        let _env = EnvGuard::set(&[
            (HOME, Some(s(tmp.path()))),
            (token::TOKEN_VAR, Some("test-token-not-real")),
        ]);
        assert_eq!(
            token::daemon_token(),
            Some("test-token-not-real".to_string())
        );
    }
    // Unset in the process: None even where the registry holds a real token.
    let _env = EnvGuard::set(&[(HOME, Some(s(tmp.path()))), (token::TOKEN_VAR, None)]);
    assert_eq!(token::daemon_token(), None);
}

// Off Windows there is no registry, so the process environment decides.
#[cfg(not(windows))]
#[test]
fn daemon_token_reads_the_process_environment() {
    let _lock = env_lock();
    {
        let _env = EnvGuard::set(&[(token::TOKEN_VAR, Some("test-token-not-real"))]);
        assert_eq!(
            token::daemon_token(),
            Some("test-token-not-real".to_string())
        );
    }
    let _env = EnvGuard::set(&[(token::TOKEN_VAR, Some(""))]);
    assert_eq!(token::daemon_token(), None);
}
