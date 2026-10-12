//! `lab env`: show, get, set or unset labd.env entries (secrets masked).
//!
//! The command logic is [`exec`], which returns the text and exit code, so
//! tests need no process. [`run`] prints that text. Plain mode prints what
//! the removed PowerShell env command printed, line for line; fancy mode adds the D-LX5 style.

use std::io::ErrorKind;
use std::path::Path;

use crate::commands::Ctx;
use crate::daemon::env_file::{self, EnvLine};
use crate::daemon::paths;
use crate::ui::{Mark, Tone, Ui};

const USAGE: &str =
    "usage: lab env [get KEY | set KEY VALUE | unset KEY]  (a VALUE starting with '-': -Value:<value>)";
const KEY_RULE: &str = "KEY must match ^[A-Z][A-Z0-9_]*$";
const RESTART_PLAIN: &str = "restart the daemon for this to apply: lab restart";

const LONG: &str = concat!(
    "lab env                  every labd.env line, values masked\nlab env get KEY          one value, masked\nlab env set KEY VALUE    set KEY in place, or append it; comments and order are kept\nlab env unset KEY        remove KEY's lines",
    "\n\n",
    "A key whose name contains TOKEN, SECRET, PASSWORD, KEY, AUTH or CREDENTIAL prints as <redacted>. A URL's user, password and host print as <host>, and a line that is not KEY=VALUE or a comment prints as <unparsed line>. All of it is display only: set and unset write the values back unchanged. Keys match ignoring case, as the daemon reads them. Before a set or unset, labd.env is copied to labd.env.bak-<yyyyMMdd-HHmmss>, and the new file replaces the old one in a single move. The daemon reads labd.env only at start, so a change applies after lab restart.",
    "\n\n",
    "A VALUE that starts with '-' must be written --value=<value>. A VALUE with leading or trailing spaces is refused, because labd.env lines are trimmed when read.",
);

const EXAMPLES: &str = "Examples:\n  lab env\n  lab env set CIMMERIA_LAB_INSTANCES default,p2\n  lab env set SOME_FLAGS --value=-Xmx512m";

// Arguments of `lab env`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Show, get, set or unset labd.env entries (secrets masked)",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {
    /// get, set or unset; none lists every entry
    pub verb: Option<String>,
    /// The labd.env key
    pub key: Option<String>,
    /// The value for set
    pub value: Option<String>,
    /// The value for set, for a value that starts with '-' (--value=-x)
    #[arg(
        long = "value",
        value_name = "VALUE",
        allow_hyphen_values = true,
        conflicts_with = "value"
    )]
    pub value_flag: Option<String>,
}

/// What a `lab env` call produced: the text for each stream and the exit code.
#[derive(Debug)]
pub(crate) struct Outcome {
    pub code: u8,
    pub out: String,
    pub err: String,
}

impl Outcome {
    fn ok(out: String) -> Self {
        Self {
            code: 0,
            out,
            err: String::new(),
        }
    }

    /// An error. Plain mode prints the message only (what the PowerShell
    /// printed); fancy mode adds the `error:` label and the dim hint.
    fn fail(ui: &Ui, code: u8, message: &str, hint: Option<&str>) -> Self {
        let mut err = if ui.fancy {
            format!("{} {message}\n", ui.paint(Tone::Fail, "error:"))
        } else {
            format!("{message}\n")
        };
        if let (true, Some(hint)) = (ui.fancy, hint) {
            err.push_str(&format!(
                "  {}\n",
                ui.paint(Tone::Dim, &format!("hint: {hint}"))
            ));
        }
        Self {
            code,
            out: String::new(),
            err,
        }
    }
}

/// A usage error (exit 2): the message, then the usage line.
fn usage(ui: &Ui, message: &str) -> Outcome {
    if ui.fancy {
        Outcome::fail(ui, 2, message, Some(USAGE))
    } else {
        Outcome {
            code: 2,
            out: String::new(),
            err: format!("{message}\n{USAGE}\n"),
        }
    }
}

/// Runs `lab env` against the labd.env at `path`.
pub(crate) fn exec(args: &Args, ui: &Ui, path: &Path) -> Outcome {
    // An empty verb lists too: the PowerShell tested `-not $Verb`.
    let Some(verb_text) = args.verb.as_deref().filter(|verb| !verb.is_empty()) else {
        return list(ui, path);
    };
    let verb = verb_text.to_ascii_lowercase();
    if !matches!(verb.as_str(), "get" | "set" | "unset") {
        return usage(ui, &format!("unknown verb '{verb_text}'"));
    }
    let key = args.key.as_deref().unwrap_or("");
    if !env_file::is_valid_key(key) {
        return usage(ui, KEY_RULE);
    }
    match verb.as_str() {
        "get" => get_value(ui, path, key),
        "set" => set_value(args, ui, path, key),
        _ => unset_value(ui, path, key),
    }
}

/// The lines of labd.env; a missing file is no lines, any other read error
/// is an outcome to return.
fn load(ui: &Ui, path: &Path) -> Result<Vec<EnvLine>, Outcome> {
    match env_file::read(path) {
        Ok(lines) => Ok(lines),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(Outcome::fail(
            ui,
            1,
            &format!("cannot read {}: {e}", path.display()),
            None,
        )),
    }
}

/// One line for the listing. In plain mode this equals `env_file::mask_line`.
fn list_line(ui: &Ui, line: &EnvLine) -> String {
    match line {
        EnvLine::Pair { key, value, .. } => format!(
            "{}={}",
            ui.paint(Tone::Cmd, key),
            ui.paint(Tone::Dim, &env_file::mask(key, value))
        ),
        EnvLine::Comment(_) | EnvLine::Unparsed(_) => {
            ui.paint(Tone::Dim, &env_file::mask_line(line))
        }
        EnvLine::Blank(_) => String::new(),
    }
}

/// `lab env`: every line, masked.
fn list(ui: &Ui, path: &Path) -> Outcome {
    let lines = match env_file::read(path) {
        Ok(lines) => lines,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Outcome::fail(
                ui,
                1,
                &format!("no labd.env at {}", path.display()),
                Some("lab env set KEY VALUE creates it"),
            );
        }
        Err(e) => {
            return Outcome::fail(ui, 1, &format!("cannot read {}: {e}", path.display()), None);
        }
    };
    let mut out = String::new();
    for line in &lines {
        out.push_str(&list_line(ui, line));
        out.push('\n');
    }
    Outcome::ok(out)
}

/// `lab env get KEY`.
fn get_value(ui: &Ui, path: &Path, key: &str) -> Outcome {
    let lines = match load(ui, path) {
        Ok(lines) => lines,
        Err(outcome) => return outcome,
    };
    match env_file::get(&lines, key) {
        Some(value) => Outcome::ok(format!(
            "{}\n",
            ui.paint(Tone::Dim, &env_file::mask(key, value))
        )),
        None => Outcome::fail(ui, 1, &format!("{key} is not set"), None),
    }
}

/// `lab env set KEY VALUE`.
fn set_value(args: &Args, ui: &Ui, path: &Path, key: &str) -> Outcome {
    let Some(value) = args.value.as_deref().or(args.value_flag.as_deref()) else {
        return usage(ui, "set needs KEY and VALUE");
    };
    if value.contains('\r') || value.contains('\n') {
        return Outcome::fail(ui, 2, "VALUE must be one line", None);
    }
    if value != value.trim() {
        return Outcome::fail(
            ui,
            2,
            "VALUE must not start or end with spaces (labd.env lines are trimmed when read)",
            None,
        );
    }
    let mut lines = match load(ui, path) {
        Ok(lines) => lines,
        Err(outcome) => return outcome,
    };
    env_file::set(&mut lines, key, value);
    save(ui, path, &lines, &format!("set {key}"))
}

/// `lab env unset KEY`.
fn unset_value(ui: &Ui, path: &Path, key: &str) -> Outcome {
    let mut lines = match load(ui, path) {
        Ok(lines) => lines,
        Err(outcome) => return outcome,
    };
    if !env_file::unset(&mut lines, key) {
        let text = format!("{key} is not set; labd.env unchanged");
        let line = if ui.fancy {
            ui.paint(Tone::Warn, &format!("{} {text}", ui.mark(Mark::Warn)))
        } else {
            text
        };
        return Outcome::ok(format!("{line}\n"));
    }
    save(ui, path, &lines, &format!("unset {key}"))
}

/// Writes `lines` with a backup and builds the success text: the backup path
/// (when a file existed) and the restart hint.
fn save(ui: &Ui, path: &Path, lines: &[EnvLine], done: &str) -> Outcome {
    let backup = match env_file::write_with_backup(path, lines) {
        Ok(backup) => backup,
        Err(e) => {
            return Outcome::fail(
                ui,
                1,
                &format!("cannot write {}: {e}", path.display()),
                None,
            );
        }
    };
    let pad = if ui.fancy { "  " } else { "" };
    let mut out = String::new();
    if ui.fancy {
        out.push_str(&ui.paint(Tone::Ok, &format!("{} {done}", ui.mark(Mark::Pass))));
        out.push('\n');
    }
    if let Some(backup) = backup {
        out.push_str(pad);
        out.push_str(&ui.paint(Tone::Dim, &format!("backup: {}", backup.display())));
        out.push('\n');
    }
    out.push_str(pad);
    if ui.fancy {
        out.push_str(&ui.paint(Tone::Dim, "hint: run: lab restart"));
    } else {
        out.push_str(RESTART_PLAIN);
    }
    out.push('\n');
    Outcome::ok(out)
}

/// Runs `lab env` and returns its exit code.
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let outcome = exec(args, &ctx.ui, &paths::labd_env_path());
    anstream::print!("{}", outcome.out);
    anstream::eprint!("{}", outcome.err);
    outcome.code
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, Command};

    fn env_args(command: Command) -> crate::commands::env::Args {
        match command {
            Command::Env(args) => args,
            other => panic!("not env: {other:?}"),
        }
    }

    /// The `.EXAMPLE` lines of the removed PowerShell env command parse, with the right values.
    #[test]
    fn examples_parse() {
        let a = env_args(parse_ok("env", &[]));
        assert_eq!(a.verb, None);
        let a = env_args(parse_ok(
            "env",
            &["set", "CIMMERIA_LAB_INSTANCES", "default,p2"],
        ));
        assert_eq!(a.verb.as_deref(), Some("set"));
        assert_eq!(a.key.as_deref(), Some("CIMMERIA_LAB_INSTANCES"));
        assert_eq!(a.value.as_deref(), Some("default,p2"));
        assert_eq!(a.value_flag, None);
    }

    /// A value that starts with '-' comes through the `--value=` form, and
    /// the hyphen is not read as a flag.
    #[test]
    fn hyphen_value_uses_the_value_flag() {
        let a = env_args(parse_ok("env", &["set", "SOME_FLAGS", "-Value:-Xmx512m"]));
        assert_eq!(a.verb.as_deref(), Some("set"));
        assert_eq!(a.key.as_deref(), Some("SOME_FLAGS"));
        assert_eq!(a.value, None);
        assert_eq!(a.value_flag.as_deref(), Some("-Xmx512m"));
    }
}

#[cfg(test)]
mod exec_tests {
    use super::*;
    use crate::ui::strip_ansi;

    fn fancy() -> Ui {
        Ui {
            color: true,
            fancy: true,
            width: None,
        }
    }

    /// New (ties the listing to the masking rules LX-04 ported): in plain mode
    /// every line prints as `env_file::mask_line` says.
    #[test]
    fn plain_listing_equals_the_masked_lines() {
        let lines = env_file::parse(
            "# c TOKEN=abc\nFOO=1\n\nA_TOKEN=abc\nstray\nU=postgres://u:pw@db:5432/d\n",
        );
        assert_eq!(lines.len(), 6);
        for line in &lines {
            assert_eq!(list_line(&Ui::plain(), line), env_file::mask_line(line));
        }
    }

    /// New: fancy mode paints keys cyan and values and comments dim.
    #[test]
    fn fancy_listing_paints_keys_cyan_values_and_comments_dim() {
        let ui = fancy();
        assert_eq!(
            list_line(&ui, &env_file::parse_line("FOO=1")),
            "\x1b[36mFOO\x1b[0m=\x1b[2m1\x1b[0m"
        );
        assert_eq!(
            list_line(&ui, &env_file::parse_line("# head")),
            "\x1b[2m# head\x1b[0m"
        );
        assert_eq!(
            list_line(&ui, &env_file::parse_line("stray")),
            "\x1b[2m<unparsed line>\x1b[0m"
        );
    }

    /// New: styling never reveals a secret.
    #[test]
    fn fancy_listing_keeps_a_secret_masked() {
        let text = list_line(&fancy(), &env_file::parse_line("A_TOKEN=s3cr3t"));
        assert_eq!(strip_ansi(&text), "A_TOKEN=<redacted>");
        assert!(!text.contains("s3cr3t"));
    }

    /// New: plain usage errors are the message, then the usage line (stderr only).
    #[test]
    fn plain_usage_error_is_the_message_then_the_usage_line() {
        let outcome = usage(&Ui::plain(), "KEY must match ^[A-Z][A-Z0-9_]*$");
        assert_eq!(outcome.code, 2);
        assert_eq!(outcome.out, "");
        assert_eq!(
            outcome.err,
            format!("KEY must match ^[A-Z][A-Z0-9_]*$\n{USAGE}\n")
        );
    }

    /// New: fancy usage errors carry the `error:` label and a dim hint.
    #[test]
    fn fancy_usage_error_has_the_error_label_and_a_hint() {
        let outcome = usage(&fancy(), "set needs KEY and VALUE");
        assert_eq!(outcome.code, 2);
        assert_eq!(
            strip_ansi(&outcome.err),
            format!("error: set needs KEY and VALUE\n  hint: {USAGE}\n")
        );
        assert!(outcome.err.contains("\x1b[1;31merror:\x1b[0m"));
    }

    fn args(verb: &str, key: &str) -> Args {
        Args {
            verb: Some(verb.to_string()),
            key: Some(key.to_string()),
            value: None,
            value_flag: None,
        }
    }

    /// New: fancy unset of an absent key is a yellow `!` line and writes nothing.
    #[test]
    fn fancy_unset_of_an_absent_key_warns_and_writes_nothing() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let path = dir.path().join("labd.env");
        std::fs::write(&path, "A=1\r\n").expect("seed labd.env");
        let outcome = exec(&args("unset", "NOPE"), &fancy(), &path);
        assert_eq!(outcome.code, 0);
        assert_eq!(
            strip_ansi(&outcome.out),
            "! NOPE is not set; labd.env unchanged\n"
        );
        assert!(outcome.out.contains("\x1b[33m"), "{}", outcome.out);
        assert_eq!(std::fs::read_dir(dir.path()).expect("dir").count(), 1);
    }

    /// New: fancy listing of a missing file is `error:` plus a dim hint.
    #[test]
    fn fancy_listing_without_a_file_has_the_error_label_and_a_hint() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let path = dir.path().join("labd.env");
        let none = Args {
            verb: None,
            key: None,
            value: None,
            value_flag: None,
        };
        let outcome = exec(&none, &fancy(), &path);
        assert_eq!(outcome.code, 1);
        assert_eq!(
            strip_ansi(&outcome.err),
            format!(
                "error: no labd.env at {}\n  hint: lab env set KEY VALUE creates it\n",
                path.display()
            )
        );
    }

    /// New: fancy success shows a check, the backup path and the restart hint.
    #[test]
    fn fancy_save_shows_a_check_the_backup_and_the_hint() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let path = dir.path().join("labd.env");
        std::fs::write(&path, "A=1\r\n").expect("seed labd.env");
        let outcome = save(&fancy(), &path, &env_file::parse("A=2\n"), "set A");
        assert_eq!(outcome.code, 0);
        let text = strip_ansi(&outcome.out);
        assert!(text.starts_with("✓ set A\n  backup: "), "{text}");
        assert!(text.ends_with("\n  hint: run: lab restart\n"), "{text}");
    }
}
