//! `lab.exe`: the Cimmeria Live Research Lab command line.
//!
//! LX-01 is the skeleton: argument rewriting, the UI kit, the root help, the
//! PowerShell fallback and `version`. Each later packet moves one command
//! from `tools/lab/lab.ps1` into [`commands`].

pub mod argv;
pub mod commands;
pub mod daemon;
pub mod fallback;
pub mod help;
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

/// `lab help [name]`: the root help, or the full help of one command.
fn help_for(ui: &Ui, target: Option<&OsString>) -> ExitCode {
    let Some(target) = target else {
        anstream::println!("{}", help::root_help(ui));
        return ExitCode::SUCCESS;
    };
    let name = target.to_string_lossy().into_owned();
    match commands::find(&name) {
        // `version` has no PowerShell half to ask for its help.
        Some(info) if name == "version" => {
            anstream::println!("lab {} - {}", info.name, info.summary);
            ExitCode::SUCCESS
        }
        Some(_) => ExitCode::from(fallback::run(&Ctx {
            ui: *ui,
            command: name,
            raw: vec![OsString::from("--help")],
        })),
        None => unknown_command(ui, &name),
    }
}

/// Runs one command: `version` through clap, any other known name through
/// the PowerShell fallback with its original tokens.
fn dispatch(ui: Ui, raw: Vec<OsString>, rewritten: Vec<OsString>) -> ExitCode {
    let name = raw
        .first()
        .map(|t| t.to_string_lossy().into_owned())
        .unwrap_or_default();
    if name == "version" {
        return match Cli::try_parse_from(once(OsString::from("lab")).chain(rewritten)) {
            Ok(_) => commands::version::run(),
            Err(err) => {
                let code = u8::try_from(err.exit_code()).unwrap_or(2);
                let _ = err.print();
                ExitCode::from(code)
            }
        };
    }
    if commands::find(&name).is_none() {
        return unknown_command(&ui, &name);
    }
    let tail = raw.into_iter().skip(1).collect();
    ExitCode::from(fallback::run(&Ctx {
        ui,
        command: name,
        raw: tail,
    }))
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
