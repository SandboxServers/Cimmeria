//! `lab local-setup`: one-shot setup for a lab against a local server, by
//! running `cli\local-setup.ps1` with the same arguments (LX-18).

use crate::commands::Ctx;
use crate::fallback;

const LONG: &str = concat!(
    "Sets a lab up against a local server: i686 target, tokens, lab account file, login row. Idempotent: every step keeps what is already there and works, and no secret value is ever printed.",
    "\n\n",
    "1. Adds the i686-pc-windows-msvc Rust target to the checkout's pinned toolchain when it is missing (tools/lab/install.ps1 builds the client DLLs for it).\n2. Sets user environment variables, keeping any that already work: CIMMERIA_LAB_MCP_TOKEN (32 random bytes, hex, when unset or short), CIMMERIA_TELEMETRY_HMAC_SECRET (64 random bytes, hex, when unset or short), CIMMERIA_LAB_MCP_BIND (127.0.0.1:8451), CIMMERIA_LAB_MCP_URL (http://127.0.0.1:8451/mcp), CIMMERIA_LAB_INSTALL_DIR (the install found). A server started from a NEW terminal inherits them, which turns on its lab endpoint and the lab's telemetry mint; .mcp.json's lab-server entry expands the URL and token. Restart Claude Code (or Codex) and open a new terminal after the first run.\n3. Writes <install>\\Binaries\\sessions\\lab-account.json when it is absent: login row Local, the seeded account lab / test, character Labone. Labone is not seeded: create it once with lab_create_character, or lab_ensure_in_world with a create block.\n4. Checks the client's LoginInternal.lua for a Local row pointing at http://127.0.0.1:8081. With --patch-login-internal it adds the row (after a backup); without it, it prints the line to add.\n5. Prints what is left to do.",
    "\n\n",
    "--install-dir, passed explicitly, is also recorded as the user variable CIMMERIA_LAB_INSTALL_DIR, replacing a different value there (the row names both paths). Found any other way, a different stored value is kept with a WARN row. A missing git, rustup or a failed rustup target add becomes a to-do item; the later steps still run.",
);

const EXAMPLES: &str =
    "Examples:\n  lab local-setup --install-dir 'D:\\Games\\Stargate Worlds'\n  lab local-setup --patch-login-internal";

/// Arguments of `lab local-setup`.
#[derive(clap::Args, Debug)]
#[command(
    about = "One-shot setup for a lab against a local server",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {
    /// The SGW install (the folder holding Binaries\SGW.exe); default CIMMERIA_LAB_INSTALL_DIR, else labd.env's
    #[arg(long)]
    pub install_dir: Option<String>,
    /// The checkout whose pinned toolchain gets the i686 target (default: the Cimmeria checkout you are in)
    #[arg(long)]
    pub worktree: Option<String>,
    /// Add the Local row to LoginInternal.lua, keeping a .bak copy
    #[arg(long)]
    pub patch_login_internal: bool,
    /// Where the variables go: User, or Process (changes nothing outside this run; for testing)
    #[arg(long, ignore_case = true, default_value = "User", value_parser = ["User", "Process"])]
    pub env_scope: String,
}

/// Runs `lab local-setup`: `cli\local-setup.ps1` with the same arguments.
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let _ = args;
    fallback::run_script(ctx, "cli/local-setup.ps1", &[])
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, try_parse, Command};

    fn local_setup(tokens: &[&str]) -> super::Args {
        let Command::LocalSetup(args) = parse_ok("local-setup", tokens) else {
            panic!("not local-setup")
        };
        args
    }

    /// The two EXAMPLES forms parse, with the right values.
    #[test]
    fn examples_parse() {
        let a = local_setup(&["-InstallDir", "D:\\Games\\Stargate Worlds"]);
        assert_eq!(a.install_dir.as_deref(), Some("D:\\Games\\Stargate Worlds"));
        assert_eq!(a.env_scope, "User");

        assert!(local_setup(&["-PatchLoginInternal"]).patch_login_internal);
    }

    #[test]
    fn env_scope_values() {
        assert_eq!(local_setup(&["-EnvScope", "Process"]).env_scope, "Process");
        assert!(try_parse("local-setup", &["-EnvScope", "process"]).is_ok());
        assert!(try_parse("local-setup", &["-EnvScope", "Machine"]).is_err());
    }

    #[test]
    fn worktree_flag() {
        let a = local_setup(&["-Worktree", "C:\\wt"]);
        assert_eq!(a.worktree.as_deref(), Some("C:\\wt"));
    }
}
