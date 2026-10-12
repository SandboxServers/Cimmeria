//! Fixtures for the `lab record` tests: a stubbed daemon probe and tool call.
#![allow(dead_code)]

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Duration;

use cimmeria_lab_cli::commands::record::action::{run_action, Env, Outcome};
use cimmeria_lab_cli::commands::record::Args;
use cimmeria_lab_cli::daemon::mcp::McpError;
use cimmeria_lab_cli::ui::Ui;
use serde_json::Value;

/// The recording id the tests use.
pub const ID: &str = "20261012-143000-p2";
/// The spec folder the context offers.
pub const SPECS: &str = r"C:\repo\docs\guides\uat-specs";
/// The UAT root the context offers.
pub const UAT: &str = r"C:\lab\uat-runs";

/// What a stubbed tool call answers: a reply, or an error.
pub type Reply = Result<Value, McpError>;

/// One call the code made.
#[derive(Debug, Clone)]
pub struct Recorded {
    pub name: String,
    pub args: Value,
}

/// `Args` for `action` and an optional positional target; set other fields on the result.
pub fn args(action: &str, target: Option<&str>) -> Args {
    Args {
        action: Some(action.to_owned()),
        target: target.map(str::to_owned),
        ..Args::default()
    }
}

/// A tool answer.
pub fn ok(value: Value) -> Reply {
    Ok(value)
}

/// A tool error with this text.
pub fn tool_err(text: &str) -> Reply {
    Err(McpError::Tool(text.to_owned()))
}

/// The context: `home` is the lab home and the working folder.
pub fn env(home: &Path) -> Env {
    Env {
        lab_home: home.to_path_buf(),
        uat_root: PathBuf::from(UAT),
        specs_dir: Some(PathBuf::from(SPECS)),
        cwd: home.to_path_buf(),
    }
}

/// Joins path parts the way the code under test does.
pub fn joined(base: &str, parts: &[&str]) -> String {
    let mut path = PathBuf::from(base);
    for part in parts {
        path.push(part);
    }
    path.display().to_string()
}

/// A styled terminal.
pub fn fancy_ui() -> Ui {
    Ui {
        color: true,
        fancy: true,
        width: None,
    }
}

/// Runs the action in plain mode; the daemon answers `up`; each call takes the next reply.
pub fn run(home: &Path, a: &Args, replies: Vec<Reply>, up: bool) -> (Outcome, Vec<Recorded>) {
    run_with(&Ui::plain(), home, a, replies, up)
}

/// [`run`] with a chosen `Ui`.
pub fn run_with(
    ui: &Ui,
    home: &Path,
    a: &Args,
    replies: Vec<Reply>,
    up: bool,
) -> (Outcome, Vec<Recorded>) {
    let mut queue: VecDeque<Reply> = replies.into();
    let mut calls: Vec<Recorded> = Vec::new();
    let outcome = run_action(
        a,
        &env(home),
        ui,
        || up,
        |name: &str, arguments: Value, _timeout: Duration| {
            calls.push(Recorded {
                name: name.to_owned(),
                args: arguments,
            });
            queue.pop_front().expect("a reply was queued for this call")
        },
    );
    (outcome, calls)
}
