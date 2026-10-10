//! `login-probe`: log in to a running server the way a client does and fail
//! loudly if any hop is broken. The container smoke tests run it against the
//! freshly built image (`tools/container-smoke.sh`, issue #1291); it works
//! against any server.
//!
//! ```text
//! login-probe --user test --password test --shard Test \
//!             [--auth-url http://127.0.0.1:8081] \
//!             [--expect-base 127.0.0.1:32832] [--expect-account-id 2] \
//!             [--no-handshake]
//! ```
//!
//! Every option also reads `LOGIN_PROBE_<OPTION>` (e.g.
//! `LOGIN_PROBE_PASSWORD`); a flag wins. Exit 0 = every check passed,
//! 1 = a check failed, 2 = bad arguments. The checks are in
//! `cimmeria_wireclient::login_probe`.

use std::net::SocketAddr;
use std::process::ExitCode;

use cimmeria_wireclient::login_probe::{self, ProbeConfig};

const USAGE: &str = "usage: login-probe --user <name> --password <pw> --shard <name> \
[--auth-url <url>] [--expect-base <ip:port>] [--expect-account-id <id>] [--no-handshake]
Every option can come from LOGIN_PROBE_<OPTION> instead (e.g. LOGIN_PROBE_PASSWORD).";

/// A flag's value, else the `LOGIN_PROBE_<NAME>` environment variable.
fn option(argv: &[String], flag: &str) -> Option<String> {
    let from_flag = argv
        .iter()
        .position(|a| a == flag)
        .and_then(|i| argv.get(i + 1).cloned());
    from_flag.or_else(|| {
        let var = format!(
            "LOGIN_PROBE_{}",
            flag.trim_start_matches("--")
                .replace('-', "_")
                .to_uppercase()
        );
        std::env::var(var).ok().filter(|v| !v.is_empty())
    })
}

fn parse_args(argv: &[String]) -> Result<ProbeConfig, String> {
    let required = |flag: &str| option(argv, flag).ok_or_else(|| format!("{flag} is required"));
    let expect_base = option(argv, "--expect-base")
        .map(|v| {
            v.parse::<SocketAddr>()
                .map_err(|_| format!("--expect-base: not an ip:port: {v}"))
        })
        .transpose()?;
    let expect_account_id = option(argv, "--expect-account-id")
        .map(|v| {
            v.parse::<u32>()
                .map_err(|_| format!("--expect-account-id: not a number: {v}"))
        })
        .transpose()?;
    Ok(ProbeConfig {
        auth_url: option(argv, "--auth-url").unwrap_or_else(|| "http://127.0.0.1:8081".into()),
        user: required("--user")?,
        password: required("--password")?,
        shard: required("--shard")?,
        expect_base,
        expect_account_id,
        handshake: !argv.iter().any(|a| a == "--no-handshake"),
    })
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "cimmeria_wireclient=info,warn".to_string()),
        )
        .init();

    let argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let cfg = match parse_args(&argv) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("login-probe: {e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    match login_probe::run(&cfg).await {
        Ok(report) => {
            println!(
                "login-probe: PASS user={} account_id={} base={} handshake={}",
                cfg.user,
                report.account_id,
                report.base_addr,
                if report.handshake_done {
                    "ok"
                } else {
                    "skipped"
                }
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("login-probe: FAIL {e}");
            ExitCode::FAILURE
        }
    }
}
