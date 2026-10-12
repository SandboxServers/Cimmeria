//! `labd.pid` can only point the CLI at a loopback address: the bearer token is
//! sent there, so a bind that names another host is ignored.

mod support;

use cimmeria_lab_cli::daemon::paths::{self, loopback_bind};
use cimmeria_lab_cli::daemon::status::Endpoint;
use support::{env_lock, EnvGuard};
use tempfile::TempDir;

const DEFAULT: &str = "127.0.0.1:8779";

#[test]
fn loopback_addresses_are_accepted() {
    assert_eq!(
        loopback_bind("127.0.0.1:9000").as_deref(),
        Some("127.0.0.1:9000")
    );
    assert_eq!(
        loopback_bind(" 127.0.0.1:9 ").as_deref(),
        Some("127.0.0.1:9")
    );
    assert_eq!(loopback_bind("[::1]:9000").as_deref(), Some("[::1]:9000"));
    assert_eq!(
        loopback_bind("localhost:9000").as_deref(),
        Some("localhost:9000")
    );
    assert_eq!(
        loopback_bind("LocalHost:9000").as_deref(),
        Some("localhost:9000")
    );
}

#[test]
fn everything_that_is_not_loopback_is_refused() {
    for bad in [
        "10.0.0.5:8779",
        "192.168.1.2:8779",
        "0.0.0.0:8779",
        "[::]:8779",
        "evil.example:8779",
        "127.0.0.1:8779@evil.example",
        "evil.example@127.0.0.1:8779",
        "user:pass@127.0.0.1:8779",
        "localhost.evil.example:8779",
        "localhost@evil.example:8779",
        "localhost:8779@evil.example",
        "localhost:0",
        "127.0.0.1:0",
        "127.0.0.1",
        "localhost",
        "http://127.0.0.1:8779",
        "127.0.0.1:8779/x",
        "",
        "::1",
    ] {
        assert_eq!(loopback_bind(bad), None, "{bad:?}");
    }
}

fn with_bind(bind: &str) -> (TempDir, EnvGuard) {
    let tmp = TempDir::new().unwrap();
    let text = serde_json::json!({"pid": 4242, "bind": bind}).to_string();
    std::fs::write(tmp.path().join("labd.pid"), text).unwrap();
    let guard = EnvGuard::set(&[("CIMMERIA_LAB_HOME", Some(tmp.path().to_str().unwrap()))]);
    (tmp, guard)
}

#[test]
fn a_remote_bind_in_labd_pid_falls_back_to_the_default() {
    let _lock = env_lock();
    for bad in [
        "evil.example:8779",
        "127.0.0.1:8779@evil.example",
        "0.0.0.0:8779",
        "10.1.2.3:8779",
    ] {
        let (_tmp, _env) = with_bind(bad);
        let info = paths::daemon_info();
        assert_eq!(info.bind, DEFAULT, "{bad}");
        assert_eq!(info.pid, Some(4242), "the pid is still reported: {bad}");
        assert_eq!(
            Endpoint::discover().base_url,
            format!("http://{DEFAULT}"),
            "{bad}"
        );
    }
}

#[test]
fn a_loopback_bind_in_labd_pid_is_used() {
    let _lock = env_lock();
    for (bind, url) in [
        ("127.0.0.1:9123", "http://127.0.0.1:9123"),
        ("localhost:9123", "http://localhost:9123"),
        ("[::1]:9123", "http://[::1]:9123"),
    ] {
        let (_tmp, _env) = with_bind(bind);
        assert_eq!(paths::daemon_info().bind, bind);
        assert_eq!(Endpoint::discover().base_url, url);
    }
}
