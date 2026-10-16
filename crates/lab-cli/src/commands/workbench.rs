//! `lab workbench open`: mint a one-use local browser opener without printing it.

use std::process::Command;
use std::time::Duration;

use clap::Subcommand;
use serde::Deserialize;

use super::Ctx;
use crate::daemon::status::Endpoint;

const EXAMPLES: &str = "Examples:\n  lab workbench open\n  lab workbench open --instance p2";

/// Open the local companion window for one hosted lab instance.
#[derive(clap::Args, Debug)]
#[command(
    about = "Open the local companion workbench",
    long_about = "Uses the installed lab daemon's bearer token to mint a one-use local browser session. The opener stays out of terminal output. The browser receives only a scoped HttpOnly cookie.",
    after_help = EXAMPLES
)]
pub struct Args {
    /// The workbench action.
    #[command(subcommand)]
    action: Action,
}

#[derive(Debug, Subcommand)]
enum Action {
    /// Open the companion window for an instance.
    Open {
        /// Hosted instance label (default, p2, ...).
        #[arg(long, default_value = "default")]
        instance: String,
    },
}

#[derive(Deserialize)]
struct OpenReply {
    path: String,
    instance: String,
}

fn opener_url(endpoint: &Endpoint, instance: &str) -> Result<String, String> {
    let token = endpoint
        .token
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or("daemon token unavailable; install or configure the lab daemon")?;
    let base = reqwest::Url::parse(&endpoint.base_url)
        .map_err(|_| "daemon endpoint is invalid".to_owned())?;
    if base.scheme() != "http"
        || !matches!(
            base.host_str(),
            Some("127.0.0.1" | "localhost" | "[::1]" | "::1")
        )
        || base.port().is_none()
        || !base.username().is_empty()
        || base.password().is_some()
        || base.path() != "/"
        || base.query().is_some()
        || base.fragment().is_some()
    {
        return Err("daemon endpoint must be a plain loopback HTTP authority".into());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "could not create local daemon client".to_owned())?;
    let response = client
        .post(base.join("workbench/session/open").expect("fixed path"))
        .bearer_auth(token)
        .json(&serde_json::json!({ "instance": instance }))
        .send()
        .map_err(|_| "local lab daemon did not answer".to_owned())?;
    if !response.status().is_success() {
        return Err(format!(
            "workbench opener refused (HTTP {})",
            response.status()
        ));
    }
    let reply: OpenReply = response
        .json()
        .map_err(|_| "daemon returned an invalid opener reply".to_owned())?;
    let nonce = reply
        .path
        .strip_prefix("/workbench/session/redeem/")
        .ok_or("daemon returned an invalid opener path")?;
    if nonce.len() != 64
        || !nonce.bytes().all(|byte| byte.is_ascii_hexdigit())
        || !reply.instance.eq_ignore_ascii_case(instance)
    {
        return Err("daemon returned an invalid opener reply".into());
    }
    Ok(base
        .join(&reply.path)
        .expect("validated local path")
        .to_string())
}

fn launch_browser(url: &str) -> Result<(), String> {
    #[cfg(windows)]
    let program = "explorer.exe";
    #[cfg(not(windows))]
    let program = "xdg-open";
    Command::new(program)
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|_| "could not open the system browser".to_owned())
}

/// Run the CLI action. The one-use URL is never written to stdout or stderr.
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let Action::Open { instance } = &args.action;
    let endpoint = Endpoint::discover();
    match open_with(&endpoint, instance, launch_browser) {
        Ok(()) => {
            anstream::println!("Workbench opened for {instance}");
            0
        }
        Err(error) => {
            ctx.ui.error(&error, Some("check: lab status"));
            1
        }
    }
}

fn open_with(
    endpoint: &Endpoint,
    instance: &str,
    browser: impl FnOnce(&str) -> Result<(), String>,
) -> Result<(), String> {
    let url = opener_url(endpoint, instance)?;
    browser(&url)
}

#[cfg(test)]
#[path = "workbench_tests.rs"]
mod tests;
