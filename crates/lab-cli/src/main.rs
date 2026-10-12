//! `lab.exe` entry point. The command line lives in the library.

use std::process::ExitCode;

fn main() -> ExitCode {
    cimmeria_lab_cli::main_with(std::env::args_os())
}
