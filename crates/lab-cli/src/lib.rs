//! `lab.exe`: the Cimmeria Live Research Lab command line.
//!
//! Every command is a subcommand in [`commands`]; the few that still run PowerShell do it through [`fallback::run_script`].

pub mod argv;
pub mod ascii_json;
pub mod batch;
pub mod clip;
pub mod commands;
pub mod daemon;
pub mod fallback;
pub mod help;
pub mod human;
pub mod known_issues;
pub mod log_filter;
pub mod processes;
pub mod ui;

use std::ffi::OsString;
use std::io::Write;
use std::iter::once;
use std::process::ExitCode;

use clap::Parser;

use commands::{Cli, Ctx};
use ui::Ui;

/// Runs the `lab` command line over `args`, the program name first.
pub fn main_with(args: impl IntoIterator<Item = OsString>) -> ExitCode {
    let raw: Vec<OsString> = args.into_iter().skip(1).collect();
    let ui = Ui::detect();
    let rewritten = argv::rewrite(raw.clone());
    let head = rewritten.first().map(|t| t.to_string_lossy().into_owned());
    match head.as_deref() {
        None | Some("--help" | "-h") => {
            anstream::println!("{}", help::root_help(&ui));
            ExitCode::SUCCESS
        }
        Some("help") => help_for(&ui, raw.get(1)),
        Some(_) => dispatch(ui, raw, rewritten),
    }
}

/// `lab help [name]`: the root help, or the full help of one command (clap's
/// `--help` page).
fn help_for(ui: &Ui, target: Option<&OsString>) -> ExitCode {
    let Some(target) = target else {
        anstream::println!("{}", help::root_help(ui));
        return ExitCode::SUCCESS;
    };
    let name = target.to_string_lossy().into_owned();
    if commands::find(&name).is_none() {
        return unknown_command(ui, &name);
    }
    let tokens = vec![OsString::from(name), OsString::from("--help")];
    dispatch(*ui, tokens.clone(), tokens)
}

/// Runs one command: clap parses it.
fn dispatch(ui: Ui, raw: Vec<OsString>, rewritten: Vec<OsString>) -> ExitCode {
    let name = raw
        .first()
        .map(|t| t.to_string_lossy().into_owned())
        .unwrap_or_default();
    if commands::find(&name).is_none() {
        return unknown_command(&ui, &name);
    }
    let cli =
        match Cli::try_parse_from(once(OsString::from("lab")).chain(rewritten.iter().cloned())) {
            Ok(cli) => cli,
            Err(err) => return clap_error(err, &ui, &name, &rewritten),
        };
    let Some(command) = cli.command else {
        return ExitCode::SUCCESS;
    };
    let ctx = Ctx {
        ui,
        command: name,
        raw: raw.into_iter().skip(1).collect(),
    };
    ExitCode::from(commands::dispatch(&command, &ctx))
}

/// The commands whose `--json` output is one object, a usage error included.
const JSON_COMMANDS: [&str; 5] = ["uat", "golden", "calibrate", "record", "watch"];

/// Whether the (rewritten) arguments ask for `--json`: the flag before any bare
/// `--`, in either spelling.
fn asks_for_json(name: &str, tokens: &[OsString]) -> bool {
    JSON_COMMANDS.contains(&name)
        && tokens
            .iter()
            .take_while(|t| *t != "--")
            .any(|t| t.to_string_lossy().eq_ignore_ascii_case("--json"))
}

/// The `--json` body of a usage error: `{"ok":false,"exit":2,"error":"..."}`,
/// the error being clap's first line without its `error: ` label.
pub fn usage_error_json(clap_text: &str) -> String {
    let first = clap_text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    let first = first.strip_prefix("error: ").unwrap_or(first);
    ascii_json::to_ascii_json(&serde_json::json!({"ok": false, "exit": 2, "error": first}))
}

/// Prints a clap error (or help page) and returns its exit code: 0 for help,
/// 2 for a usage error. An unknown subcommand gets the `lab` unknown-command
/// message instead of clap's. A usage error of a command that was given
/// `--json` is the one JSON object on stdout instead (`uat`, `golden`,
/// `calibrate`, `record`, `watch`), so an agent never has to parse clap's text.
fn clap_error(err: clap::Error, ui: &Ui, name: &str, tokens: &[OsString]) -> ExitCode {
    if err.kind() == clap::error::ErrorKind::InvalidSubcommand {
        return unknown_command(ui, name);
    }
    let code = u8::try_from(err.exit_code()).unwrap_or(2);
    if code != 0 && asks_for_json(name, tokens) {
        let _ = writeln!(std::io::stdout(), "{}", usage_error_json(&err.to_string()));
        return ExitCode::from(2);
    }
    let _ = err.print();
    ExitCode::from(code)
}

/// Reports an unknown command, with the nearest names when there are any.
fn unknown_command(ui: &Ui, name: &str) -> ExitCode {
    let suggestions = help::suggest(name);
    let hint = if suggestions.is_empty() {
        "run: lab help".to_string()
    } else {
        format!("did you mean: {}?  run: lab help", suggestions.join(", "))
    };
    ui.error(&format!("unknown command '{name}'"), Some(&hint));
    ExitCode::from(2)
}
