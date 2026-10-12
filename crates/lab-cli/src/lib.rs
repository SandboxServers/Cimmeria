//! `lab.exe`: the Cimmeria Live Research Lab command line.
//!
//! Every command is a subcommand in [`commands`]; the few that still run PowerShell do it through [`fallback::run_script`].

pub mod argv;
pub mod batch;
pub mod clip;
pub mod commands;
pub mod daemon;
pub mod fallback;
pub mod help;
pub mod known_issues;
pub mod log_filter;
pub mod processes;
pub mod ui;

use std::ffi::OsString;
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
    let cli = match Cli::try_parse_from(once(OsString::from("lab")).chain(rewritten)) {
        Ok(cli) => cli,
        Err(err) => return clap_error(err, &ui, &name),
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

/// Prints a clap error (or help page) and returns its exit code: 0 for help,
/// 2 for a usage error. An unknown subcommand gets the `lab` unknown-command
/// message instead of clap's.
fn clap_error(err: clap::Error, ui: &Ui, name: &str) -> ExitCode {
    if err.kind() == clap::error::ErrorKind::InvalidSubcommand {
        return unknown_command(ui, name);
    }
    let code = u8::try_from(err.exit_code()).unwrap_or(2);
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
