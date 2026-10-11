//! The labd.env file: its entries, read and written in place.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;

/// One line of labd.env.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvLine {
    /// `KEY=VALUE`: the key is trimmed, the value is everything after the
    /// first `=` of the trimmed line (it may itself contain `=`).
    Pair { key: String, value: String },
    /// A line whose first non-blank character is `#`, kept as written.
    Comment(String),
    /// An empty or whitespace-only line.
    Blank,
    /// Anything else (no `=` after the first character), kept as written.
    Unparsed(String),
}

/// Parses one line by the rules of `Read-LabdEnvFile`: trim, drop a leading
/// BOM, then blank, `#` comment, `KEY=VALUE` (the `=` at index 1 or later),
/// else unparsed.
pub fn parse_line(raw: &str) -> EnvLine {
    let trimmed = raw.trim().trim_start_matches('\u{feff}');
    if trimmed.is_empty() {
        return EnvLine::Blank;
    }
    if trimmed.starts_with('#') {
        return EnvLine::Comment(raw.to_string());
    }
    match trimmed.find('=') {
        Some(index) if index >= 1 => EnvLine::Pair {
            key: trimmed[..index].trim().to_string(),
            value: trimmed[index + 1..].to_string(),
        },
        _ => EnvLine::Unparsed(raw.to_string()),
    }
}

/// Splits `text` on `\r\n`, `\n` or `\r` (no empty last element for a
/// trailing newline, like `ReadAllLines`) and parses each line.
pub fn parse(text: &str) -> Vec<EnvLine> {
    let text = text.trim_start_matches('\u{feff}');
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .lines()
        .map(parse_line)
        .collect()
}

/// Reads labd.env. A missing file is `Err` with `ErrorKind::NotFound`.
pub fn read(path: &Path) -> io::Result<Vec<EnvLine>> {
    let bytes = std::fs::read(path)?;
    Ok(parse(&String::from_utf8_lossy(&bytes)))
}

/// Reads labd.env; a missing or unreadable file is no lines (what
/// `Read-LabdEnvFile` returns for a missing file).
pub fn read_or_empty(path: &Path) -> Vec<EnvLine> {
    read(path).unwrap_or_default()
}

/// One line as it is written to the file.
pub fn render_line(line: &EnvLine) -> String {
    match line {
        EnvLine::Pair { key, value } => format!("{key}={value}"),
        EnvLine::Comment(text) | EnvLine::Unparsed(text) => text.clone(),
        EnvLine::Blank => String::new(),
    }
}

/// The file text: every line followed by CRLF, no BOM.
pub fn render(lines: &[EnvLine]) -> String {
    let mut text = String::new();
    for line in lines {
        text.push_str(&render_line(line));
        text.push_str("\r\n");
    }
    text
}

/// The value of `key` (matched ignoring ASCII case). The last line that sets
/// it wins, as in `Read-LabdEnvFile`'s map.
pub fn get<'a>(lines: &'a [EnvLine], key: &str) -> Option<&'a str> {
    lines.iter().rev().find_map(|line| match line {
        EnvLine::Pair { key: k, value } if k.eq_ignore_ascii_case(key) => Some(value.as_str()),
        _ => None,
    })
}

/// Sets `key`: the last line that sets it (ignoring case) is replaced in
/// place (with the new key spelling), else a new line is appended.
pub fn set(lines: &mut Vec<EnvLine>, key: &str, value: &str) {
    let new = EnvLine::Pair {
        key: key.to_string(),
        value: value.to_string(),
    };
    let last = lines.iter().rposition(
        |line| matches!(line, EnvLine::Pair { key: k, .. } if k.eq_ignore_ascii_case(key)),
    );
    match last {
        Some(index) => lines[index] = new,
        None => lines.push(new),
    }
}

/// Removes every line that sets `key` (ignoring case); comments and other
/// keys stay. Returns whether anything was removed.
pub fn unset(lines: &mut Vec<EnvLine>, key: &str) -> bool {
    let before = lines.len();
    lines.retain(
        |line| !matches!(line, EnvLine::Pair { key: k, .. } if k.eq_ignore_ascii_case(key)),
    );
    lines.len() != before
}

/// Whether `key` is a valid env key: `^[A-Z][A-Z0-9_]*$` (case sensitive).
/// Port of `Test-LabdEnvKey`.
pub fn is_valid_key(key: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[A-Z][A-Z0-9_]*$").expect("valid regex"))
        .is_match(key)
}

/// Copies an existing `path` to `<name>.bak-<yyyyMMdd-HHmmss>` beside it
/// (a `-2`, `-3`, ... suffix when that name is taken), then replaces `path`
/// with `lines` (UTF-8, CRLF, no BOM) by writing `<path>.tmp` and renaming it
/// over `path`. Creates the parent folder. Returns the backup path, `None`
/// when `path` did not exist. Port of `Save-LabdEnv`.
pub fn write_with_backup(path: &Path, lines: &[EnvLine]) -> io::Result<Option<PathBuf>> {
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    write_with_backup_at(path, lines, &stamp)
}

/// [`write_with_backup`] with the timestamp given (so tests can force a
/// same-second collision).
pub fn write_with_backup_at(
    path: &Path,
    lines: &[EnvLine],
    stamp: &str,
) -> io::Result<Option<PathBuf>> {
    let dir = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    };
    std::fs::create_dir_all(&dir)?;
    let mut backup = None;
    if path.exists() {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "labd.env".to_string());
        let mut candidate = dir.join(format!("{name}.bak-{stamp}"));
        let mut n = 2;
        while candidate.exists() {
            candidate = dir.join(format!("{name}.bak-{stamp}-{n}"));
            n += 1;
        }
        std::fs::copy(path, &candidate)?;
        backup = Some(candidate);
    }
    let mut tmp_name = path.as_os_str().to_owned();
    tmp_name.push(".tmp");
    let tmp = PathBuf::from(tmp_name);
    std::fs::write(&tmp, render(lines).as_bytes())?;
    std::fs::rename(&tmp, path)?;
    Ok(backup)
}

fn secret_assignment() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)((?:TOKEN|SECRET|PASSWORD|KEY|AUTH|CREDENTIAL)\w*\s*=)\S*")
            .expect("valid regex")
    })
}

fn bearer() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)bearer\s+\S+").expect("valid regex"))
}

fn hex64() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\b[0-9a-f]{64}\b").expect("valid regex"))
}

fn url_host() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)([a-z][a-z0-9+.-]*://)(?:\S*@)?[^/\s:?#@]+").expect("valid regex")
    })
}

fn secret_key() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)TOKEN|SECRET|PASSWORD|KEY|AUTH|CREDENTIAL").expect("valid regex")
    })
}

/// Masks secret-looking text for display (port of `Hide-LabdEnvText`): a
/// secret-named assignment loses its value, a bearer token and a 64-hex token
/// become `<redacted>`, and a URL's userinfo and host become `<host>` (the
/// port and path stay).
pub fn hide_text(text: &str) -> String {
    let t = secret_assignment().replace_all(text, "${1}<redacted>");
    let t = bearer().replace_all(&t, "<redacted>");
    let t = hex64().replace_all(&t, "<redacted>");
    url_host().replace_all(&t, "${1}<host>").into_owned()
}

/// A value for display (port of `Get-MaskedEnvValue`): `<redacted>` when the
/// key name contains TOKEN, SECRET, PASSWORD, KEY, AUTH or CREDENTIAL
/// (ignoring case), else [`hide_text`] of the value.
pub fn mask(key: &str, value: &str) -> String {
    if secret_key().is_match(key) {
        "<redacted>".to_string()
    } else {
        hide_text(value)
    }
}

/// One line for display (port of `Get-MaskedEnvLine`): `KEY=<masked value>`;
/// a comment is masked as text; a blank line is empty; an unparsed line is
/// not shown (`<unparsed line>`).
pub fn mask_line(line: &EnvLine) -> String {
    match line {
        EnvLine::Pair { key, value } => format!("{key}={}", mask(key, value)),
        EnvLine::Comment(text) => hide_text(text),
        EnvLine::Blank => String::new(),
        EnvLine::Unparsed(_) => "<unparsed line>".to_string(),
    }
}
