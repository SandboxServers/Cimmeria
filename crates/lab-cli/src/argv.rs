//! PowerShell-style flag rewriting (D-LX4). `-Rows` becomes `--rows`, so a
//! flag reads the same whether it is typed to `lab.ps1` or to `lab.exe`.

use std::ffi::OsString;

/// Rewrites the PowerShell-style tokens in `args` (the arguments after the
/// program name). Tokens after a bare `--` and tokens that are not UTF-8 pass
/// through untouched.
pub fn rewrite(args: Vec<OsString>) -> Vec<OsString> {
    let mut after_separator = false;
    args.into_iter()
        .map(|arg| {
            if after_separator {
                return arg;
            }
            let Some(text) = arg.to_str() else {
                return arg;
            };
            if text == "--" {
                after_separator = true;
                return arg;
            }
            match rewrite_token(text) {
                Some(rewritten) => OsString::from(rewritten),
                None => arg,
            }
        })
        .collect()
}

/// The rewritten form of one token, or `None` when it stays as it is.
fn rewrite_token(text: &str) -> Option<String> {
    if text == "-?" || text == "/?" {
        return Some("--help".to_string());
    }
    let body = text.strip_prefix('-')?;
    if let Some((name, value)) = body.split_once(':') {
        return is_flag_name(name).then(|| format!("--{}={value}", kebab(name)));
    }
    is_flag_name(body).then(|| format!("--{}", kebab(body)))
}

/// `[A-Za-z][A-Za-z0-9]+`: a PowerShell parameter name, two characters or more.
fn is_flag_name(name: &str) -> bool {
    name.len() >= 2
        && name.starts_with(|c: char| c.is_ascii_alphabetic())
        && name.chars().all(|c| c.is_ascii_alphanumeric())
}

/// PascalCase to kebab-case: a dash goes before an upper-case letter that
/// follows a lower-case letter or a digit, and everything is lower-cased.
fn kebab(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    let mut after_lower_or_digit = false;
    for c in name.chars() {
        if c.is_ascii_uppercase() && after_lower_or_digit {
            out.push('-');
        }
        out.push(c.to_ascii_lowercase());
        after_lower_or_digit = c.is_ascii_lowercase() || c.is_ascii_digit();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rw(args: &[&str]) -> Vec<String> {
        rewrite(args.iter().map(OsString::from).collect())
            .into_iter()
            .map(|t| t.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn pascal_case_flags_become_kebab_long_flags() {
        assert_eq!(rw(&["-Rows"]), ["--rows"]);
        assert_eq!(rw(&["-PlanOnly"]), ["--plan-only"]);
        assert_eq!(rw(&["-RunsPerLease"]), ["--runs-per-lease"]);
        assert_eq!(rw(&["-DiffGolden"]), ["--diff-golden"]);
        assert_eq!(rw(&["-Json"]), ["--json"]);
    }

    #[test]
    fn colon_form_becomes_equals_form() {
        assert_eq!(rw(&["-Value:-x"]), ["--value=-x"]);
    }

    #[test]
    fn question_marks_become_help() {
        assert_eq!(rw(&["-?"]), ["--help"]);
        assert_eq!(rw(&["/?"]), ["--help"]);
    }

    #[test]
    fn everything_else_is_untouched() {
        assert_eq!(rw(&["-h"]), ["-h"]);
        assert_eq!(rw(&["--json"]), ["--json"]);
        assert_eq!(rw(&["FS-01,FS-02"]), ["FS-01,FS-02"]);
        assert_eq!(rw(&["-5"]), ["-5"]);
        assert_eq!(rw(&["C:\\path"]), ["C:\\path"]);
    }

    #[test]
    fn tokens_after_separator_are_untouched() {
        assert_eq!(
            rw(&["uat", "--", "-Rows", "-h"]),
            ["uat", "--", "-Rows", "-h"]
        );
        assert_eq!(rw(&["-Rows", "--", "-Json"]), ["--rows", "--", "-Json"]);
    }

    #[test]
    fn mixed_command_line() {
        assert_eq!(
            rw(&["uat", "first-session", "-Rows", "FS-01,FS-02", "-PlanOnly"]),
            [
                "uat",
                "first-session",
                "--rows",
                "FS-01,FS-02",
                "--plan-only"
            ]
        );
    }

    #[cfg(windows)]
    #[test]
    fn non_utf8_tokens_pass_through() {
        use std::os::windows::ffi::OsStringExt;
        let bad = OsString::from_wide(&[0xD800, 0x0041]);
        let out = rewrite(vec![bad.clone()]);
        assert_eq!(out, vec![bad]);
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_tokens_pass_through() {
        use std::os::unix::ffi::OsStringExt;
        let bad = OsString::from_vec(vec![0xff, 0x41]);
        let out = rewrite(vec![bad.clone()]);
        assert_eq!(out, vec![bad]);
    }
}
