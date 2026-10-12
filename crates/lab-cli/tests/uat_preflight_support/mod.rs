//! Shared builders of the `lab uat` pre-flight tests: run `execute` against a
//! stub `/status` server and a stub MCP call, and collect what it printed.
#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs;
use std::time::Duration;

use cimmeria_lab_cli::commands::uat::{execute, Args, Deps, Outcome};
use cimmeria_lab_cli::commands::{Cli, Command};
use cimmeria_lab_cli::daemon::mcp::McpError;
use cimmeria_lab_cli::daemon::status::Endpoint;
use cimmeria_lab_cli::ui::Ui;
use clap::Parser;
use serde_json::{json, Value};
use tempfile::TempDir;

use crate::support::StubServer;
pub struct Ran {
    pub outcome: Outcome,
    pub out: String,
    pub err: String,
    pub calls: Vec<(String, Value)>,
}

impl Ran {
    pub fn code(&self) -> Option<u8> {
        match self.outcome {
            Outcome::Done(c) => Some(c),
            Outcome::Drive(_) => None,
        }
    }
}

pub fn args(tokens: &[&str]) -> Args {
    let mut all = vec!["lab", "uat"];
    all.extend_from_slice(tokens);
    match Cli::try_parse_from(all).expect("parses").command {
        Some(Command::Uat(a)) => a,
        _ => panic!("not uat"),
    }
}

/// `[(instance, leased, client_pid)]` as the daemon's /status body.
pub fn status_body(rows: &[(&str, bool, Option<u32>)]) -> String {
    let instances: Vec<Value> = rows
        .iter()
        .map(|(name, held, pid)| json!({"instance": name, "client_pid": pid, "lease": {"held": held}}))
        .collect();
    json!({"daemon": {"pid": 1, "version": "x"}, "instances": instances}).to_string()
}

pub fn one_free() -> String {
    status_body(&[("default", false, None)])
}

pub fn ready(rows: &[&str]) -> Value {
    let rows: Vec<Value> = rows
        .iter()
        .map(|r| json!({"row": r, "result": "SKIPPED", "reasons": []}))
        .collect();
    json!({ "rows": rows })
}

pub fn with_chaos(mut reply: Value, headers: &[&str]) -> Value {
    reply["chaos"] = json!(headers);
    reply
}

pub const FANCY: Ui = Ui {
    color: false,
    fancy: true,
    width: None,
};

pub fn run_with(
    tokens: &[&str],
    ep: &Endpoint,
    specs: Option<&str>,
    max_clients: u32,
    sgw: Option<Vec<u32>>,
    ui: Ui,
    replies: Vec<Result<Value, McpError>>,
) -> Ran {
    let queue = RefCell::new(VecDeque::from(replies));
    let calls = RefCell::new(Vec::<(String, Value)>::new());
    let mut call = |name: &str, a: Value, _t: Duration| -> Result<Value, McpError> {
        calls.borrow_mut().push((name.to_string(), a));
        queue
            .borrow_mut()
            .pop_front()
            .expect("an unexpected MCP call")
    };
    let sgw_fn = move || sgw.clone().expect("SGW.exe must not be scanned here");
    let mut deps = Deps {
        ep,
        specs: specs.map(str::to_string),
        max_clients,
        sgw_pids: &sgw_fn,
        call: &mut call,
    };
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let outcome = execute(&args(tokens), &ui, &mut deps, &mut out, &mut err);
    Ran {
        outcome,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
        calls: calls.into_inner(),
    }
}

/// The common case: one free instance, no local spec folder, no foreign client.
pub fn run(tokens: &[&str], replies: Vec<Result<Value, McpError>>) -> Ran {
    let body = one_free();
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    run_with(tokens, &ep, None, 0, Some(vec![]), Ui::plain(), replies)
}

/// A spec folder with a golden for `first-session`, or without.
pub fn specs_dir(with_golden: bool) -> TempDir {
    let dir = TempDir::new().unwrap();
    if with_golden {
        fs::create_dir_all(dir.path().join("golden")).unwrap();
        fs::write(
            dir.path().join("golden").join("first-session.json"),
            r#"{"schema":1,"section":"first-session","fingerprint":{}}"#,
        )
        .unwrap();
    }
    dir
}
