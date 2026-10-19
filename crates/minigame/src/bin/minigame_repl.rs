//! Minigame REPL: drive one offline game from stdin, check a C6 script, or
//! print the transcript of a script so it can be saved as a golden.
//!
//! Usage:
//! - `minigame_repl --game <name> [--difficulty N] [--seed N]` reads C6
//!   commands from stdin and prints the outputs;
//! - `minigame_repl --script <file>` checks a script and exits 1 on a mismatch;
//! - `minigame_repl --record <file>` prints the transcript of a script.

use std::process::ExitCode;

use cimmeria_minigame::minigame::repl;

/// The parsed command line.
struct Args {
    game: Option<String>,
    difficulty: u32,
    seed: u32,
    script: Option<String>,
    record: Option<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(message) => {
            eprintln!("minigame_repl: {message}");
            ExitCode::from(2)
        }
    }
}

/// Run the mode the arguments select. A usage error is an `Err`, and a script
/// failure is `Ok` with exit code 1.
fn run() -> Result<ExitCode, String> {
    let args = parse_args(std::env::args().skip(1))?;
    if let Some(path) = args.script {
        let text = read_file(&path)?;
        return Ok(match repl::check_script(&text) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{path}: {message}");
                ExitCode::from(1)
            }
        });
    }
    if let Some(path) = args.record {
        let text = read_file(&path)?;
        return Ok(match repl::run_script(&text) {
            Ok(transcript) => {
                print!("{transcript}");
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{path}: {message}");
                ExitCode::from(1)
            }
        });
    }
    let Some(game) = args.game else {
        return Err("expected --game, --script or --record".to_string());
    };
    let stdin =
        std::io::read_to_string(std::io::stdin()).map_err(|e| format!("reading stdin: {e}"))?;
    let script = format!(
        "game {game} difficulty {} seed {}\n{stdin}",
        args.difficulty, args.seed
    );
    Ok(match repl::run_script(&script) {
        Ok(transcript) => {
            for line in transcript.lines().filter(|l| l.starts_with('<')) {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    })
}

/// Parse the flags by hand, so the binary needs no argument-parsing crate.
fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut parsed = Args {
        game: None,
        difficulty: 1,
        seed: 1,
        script: None,
        record: None,
    };
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--game" => parsed.game = Some(value(&mut args, &flag)?),
            "--difficulty" => parsed.difficulty = number(&mut args, &flag)?,
            "--seed" => parsed.seed = number(&mut args, &flag)?,
            "--script" => parsed.script = Some(value(&mut args, &flag)?),
            "--record" => parsed.record = Some(value(&mut args, &flag)?),
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    Ok(parsed)
}

/// The value after `flag`.
fn value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next().ok_or_else(|| format!("{flag} needs a value"))
}

/// The number after `flag`.
fn number(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<u32, String> {
    let text = value(args, flag)?;
    text.parse()
        .map_err(|_| format!("{flag} needs a number, got `{text}`"))
}

/// Read a script file.
fn read_file(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))
}
