//! Offline REPL over a `MinigameInstance`: drive one game from `cmd k=v` lines
//! or a C6 script, with no network. This is the per-game logic UAT (D-MG16).
//!
//! Every output line starts with `< `, the same form a C6 script uses for
//! its expected lines, so a transcript can be recorded and checked as-is.

use std::collections::HashMap;

use super::game::{GameOutput, MinigameInstance};
use super::games;
use super::protocol::SfsValue;
use super::session::MinigameSession;

/// One offline minigame, driven line by line.
pub struct Repl {
    game: Box<dyn MinigameInstance>,
    /// The terminal outcome once the game has ended: `"victory"` or `"failure"`.
    ended: Option<&'static str>,
}

impl Repl {
    /// A game named `game_name` at `difficulty`, seeded with `seed`.
    pub fn new(game_name: &str, difficulty: u32, seed: u32) -> Option<Repl> {
        let session = MinigameSession::offline(game_name, difficulty, seed);
        games::create_seeded(&session).map(|game| Repl { game, ended: None })
    }

    /// Run the game's `started` hook and render what it sends.
    pub fn start(&mut self) -> Vec<String> {
        let outputs = self.game.started();
        self.render_all(outputs)
    }

    /// Send one client command, `cmd k=v ...`, and render the replies. An
    /// unknown command gets no reply and no error. Once the game has ended,
    /// nothing more is sent to it.
    pub fn send(&mut self, line: &str) -> Result<Vec<String>, String> {
        let (cmd, params) = parse_command(line)?;
        if self.ended.is_some() {
            return Ok(Vec::new());
        }
        let outputs = self.game.message(&cmd, &params);
        Ok(self.render_all(outputs))
    }

    /// Run `n` 250 ms ticks, stopping early if the game ends.
    pub fn tick(&mut self, n: u32) -> Vec<String> {
        let mut lines = Vec::new();
        for _ in 0..n {
            if self.ended.is_some() {
                break;
            }
            let outputs = self.game.tick();
            lines.extend(self.render_all(outputs));
        }
        lines
    }

    /// Abort the game, as a disconnect or a cancel does.
    pub fn abort(&mut self) -> Vec<String> {
        let outputs = self.game.aborted();
        self.render_all(outputs)
    }

    /// Render each output in order, and remember a terminal one.
    fn render_all(&mut self, outputs: Vec<GameOutput>) -> Vec<String> {
        outputs
            .iter()
            .map(|out| {
                match out {
                    GameOutput::Victory => self.ended = Some("victory"),
                    GameOutput::Failure => self.ended = Some("failure"),
                    GameOutput::Send(_) => {}
                }
                render(out)
            })
            .collect()
    }
}

/// Render one output as a transcript line: `< victory`, `< failure`, or
/// `< cmd {k=v, ...}` with the keys sorted and the `_cmd` key taken as `cmd`.
/// A send with no other keys renders as `< cmd`.
pub fn render(out: &GameOutput) -> String {
    match out {
        GameOutput::Victory => "< victory".to_string(),
        GameOutput::Failure => "< failure".to_string(),
        GameOutput::Send(vars) => {
            let cmd = vars.get("_cmd").and_then(SfsValue::as_str).unwrap_or("");
            let mut keys: Vec<&String> = vars.keys().filter(|k| k.as_str() != "_cmd").collect();
            keys.sort();
            if keys.is_empty() {
                return format!("< {cmd}");
            }
            let body: Vec<String> = keys
                .iter()
                .map(|k| format!("{k}={}", render_value(&vars[*k])))
                .collect();
            format!("< {cmd} {{{}}}", body.join(", "))
        }
    }
}

/// Render a value: strings bare, numbers with Rust `{}` formatting, bools as
/// `true` or `false`, and objects as `{k=v, ...}` with sorted keys, recursively.
fn render_value(value: &SfsValue) -> String {
    match value {
        SfsValue::String(s) => s.clone(),
        SfsValue::Number(n) => format!("{n}"),
        SfsValue::Bool(b) => b.to_string(),
        SfsValue::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let body: Vec<String> = keys
                .iter()
                .map(|k| format!("{k}={}", render_value(&map[*k])))
                .collect();
            format!("{{{}}}", body.join(", "))
        }
    }
}

/// Split `cmd k=v ...` into the command name and its typed parameters.
fn parse_command(line: &str) -> Result<(String, HashMap<String, SfsValue>), String> {
    let mut parts = line.split_whitespace();
    let cmd = parts.next().ok_or_else(|| "empty command".to_string())?;
    let mut params = HashMap::new();
    for part in parts {
        let (key, value) = part
            .split_once('=')
            .ok_or_else(|| format!("expected k=v, got `{part}`"))?;
        if key.is_empty() {
            return Err(format!("empty key in `{part}`"));
        }
        params.insert(key.to_string(), parse_value(value));
    }
    Ok((cmd.to_string(), params))
}

/// A number if the text parses as one, `true` or `false` as a bool, and a
/// string otherwise.
fn parse_value(text: &str) -> SfsValue {
    // f64::from_str also accepts "inf"/"nan" in any case; those are strings (C6).
    if let Some(n) = text.parse::<f64>().ok().filter(|n| n.is_finite()) {
        SfsValue::Number(n)
    } else if text == "true" {
        SfsValue::Bool(true)
    } else if text == "false" {
        SfsValue::Bool(false)
    } else {
        SfsValue::String(text.to_string())
    }
}

/// One line of a script run. `output` marks a rendered game output; the rest
/// are echoed input. `line` is the 1-based script line that produced it.
struct Emit {
    line: usize,
    text: String,
    output: bool,
}

/// Run a C6 script. Input lines are echoed, `<` expectation lines are skipped,
/// and each output follows the command that produced it.
fn execute(text: &str) -> Result<Vec<Emit>, String> {
    let mut emits = Vec::new();
    let mut repl: Option<Repl> = None;
    for (index, raw) in text.lines().enumerate() {
        let no = index + 1;
        let line = raw.trim();
        if line.starts_with('<') {
            continue;
        }
        emits.push(Emit {
            line: no,
            text: line.to_string(),
            output: false,
        });
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let outputs = match repl.as_mut() {
            Some(game) => dispatch(game, line).map_err(|e| format!("line {no}: {e}"))?,
            None => {
                repl = Some(open_game(line).map_err(|e| format!("line {no}: {e}"))?);
                continue;
            }
        };
        emits.extend(outputs.into_iter().map(|text| Emit {
            line: no,
            text,
            output: true,
        }));
    }
    if repl.is_none() {
        return Err("script has no `game` line".to_string());
    }
    Ok(emits)
}

/// Parse the `game <name> difficulty <n> seed <n>` header.
fn open_game(line: &str) -> Result<Repl, String> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let ["game", name, "difficulty", difficulty, "seed", seed] = fields.as_slice() else {
        return Err(format!(
            "expected `game <name> difficulty <n> seed <n>`, got `{line}`"
        ));
    };
    let difficulty = difficulty
        .parse::<u32>()
        .map_err(|_| format!("bad difficulty `{difficulty}`"))?;
    let seed = seed
        .parse::<u32>()
        .map_err(|_| format!("bad seed `{seed}`"))?;
    Repl::new(name, difficulty, seed).ok_or_else(|| format!("no game named `{name}`"))
}

/// Run one script command: `> cmd k=v`, `start`, `tick N` or `abort`.
fn dispatch(repl: &mut Repl, line: &str) -> Result<Vec<String>, String> {
    if let Some(command) = line.strip_prefix('>') {
        return repl.send(command.trim());
    }
    match line.split_whitespace().collect::<Vec<_>>().as_slice() {
        ["start"] => Ok(repl.start()),
        ["abort"] => Ok(repl.abort()),
        ["tick", count] => {
            let count = count
                .parse::<u32>()
                .map_err(|_| format!("bad tick count `{count}`"))?;
            Ok(repl.tick(count))
        }
        _ => Err(format!("unknown line `{line}`")),
    }
}

/// Run a script and return its transcript: every input line, with each output
/// line after the command that produced it.
pub fn run_script(text: &str) -> Result<String, String> {
    let mut transcript = String::new();
    for emit in execute(text)? {
        transcript.push_str(&emit.text);
        transcript.push('\n');
    }
    Ok(transcript)
}

/// Run a script and compare its outputs with the `<` lines in order. The error
/// names the 1-based line of the first mismatch.
pub fn check_script(text: &str) -> Result<(), String> {
    let emits = execute(text)?;
    let actual: Vec<&Emit> = emits.iter().filter(|e| e.output).collect();
    let expected: Vec<(usize, &str)> = text
        .lines()
        .enumerate()
        .filter_map(|(index, raw)| {
            let line = raw.trim();
            line.starts_with('<').then_some((index + 1, line))
        })
        .collect();
    for index in 0..expected.len().max(actual.len()) {
        match (expected.get(index), actual.get(index)) {
            (Some((_, want)), Some(got)) if *want == got.text => {}
            (Some((no, want)), Some(got)) => {
                return Err(format!("line {no}: expected `{want}`, got `{}`", got.text));
            }
            (Some((no, want)), None) => {
                return Err(format!("line {no}: expected `{want}`, got no output"));
            }
            (None, Some(got)) => {
                return Err(format!(
                    "line {}: unexpected output `{}`",
                    got.line, got.text
                ));
            }
            (None, None) => break,
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "repl_tests.rs"]
mod tests;
