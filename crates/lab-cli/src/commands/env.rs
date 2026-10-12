//! `lab env`: shows, gets, sets or unsets labd.env entries, secrets masked.
//! LX-02 gives it its clap arguments and help page; the body is still the
//! PowerShell fallback.

use crate::commands::Ctx;
use crate::fallback;

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

/// Runs `lab env` (still the PowerShell fallback).
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let _ = args;
    fallback::run(ctx)
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

    /// The `.EXAMPLE` lines of env.ps1 parse, with the right values.
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
