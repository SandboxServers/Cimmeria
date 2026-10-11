//! Path scrubbing for labd's exported log rows (D-TG2).
//!
//! labd logs local paths freely: the install dir, the state dir, a profile
//! under `%LOCALAPPDATA%\cimmeria-lab\instances\<label>\profile`, the DLL.
//! Every one of them carries the Windows user name (`C:\Users\<name>\...`).
//! `labd.log` keeps them; the copy shipped to the colo SigNoz must not. The
//! owner's rule: a local path is reduced to the instance name it belongs to,
//! which is the only part of it a reader of SigNoz needs.
//!
//! A path with no instance in it becomes [`PATH_PLACEHOLDER`]. The user name
//! is then scrubbed from whatever is left (an `%USERNAME%` echoed in a
//! message, a relative path that slipped past the scanner).

use std::borrow::Cow;

/// What a local path with no instance in it is exported as.
pub(crate) const PATH_PLACEHOLDER: &str = "<path>";
/// What the Windows user name is exported as.
pub(crate) const USER_PLACEHOLDER: &str = "<user>";

/// The directory name the per-instance layouts put a label under:
/// `instances\<label>\profile` and `sessions\instances\<name>\`
/// (`supervisor::instance`, `supervisor::instance_profile`).
const INSTANCES_DIR: &str = "instances";

/// A user name shorter than this is not scrubbed: replacing every `ab` in a
/// row would mangle it and protect nothing.
const MIN_USER_LEN: usize = 3;

/// Reduces local paths in a string to the instance name.
#[derive(Debug, Clone, Default)]
pub(crate) struct Redactor {
    /// The instance labels this daemon hosts (`p2`, `inst-3`).
    instances: Vec<String>,
    /// The Windows user name, when long enough to scrub.
    user: Option<String>,
}

impl Redactor {
    pub(crate) fn new(instances: impl IntoIterator<Item = String>, user: Option<&str>) -> Self {
        Self {
            instances: instances.into_iter().filter(|s| !s.is_empty()).collect(),
            user: user
                .map(str::trim)
                .filter(|u| u.chars().count() >= MIN_USER_LEN)
                .map(str::to_string),
        }
    }

    /// The instance labels `CIMMERIA_LAB_INSTANCES` or `CIMMERIA_LAB_INSTANCE`
    /// name, and the user from `USERNAME` (or `USER`).
    pub(crate) fn from_env() -> Self {
        let mut labels = Vec::new();
        if let Ok(raw) = std::env::var(crate::server::INSTANCES_ENV) {
            if let Ok(entries) = crate::server::parse_list(&raw) {
                labels.extend(entries.into_iter().flatten());
            }
        }
        if let Ok(Some(name)) = crate::supervisor::instance::from_env() {
            labels.push(name);
        }
        let user = std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .ok();
        Self::new(labels, user.as_deref())
    }

    /// `s` with every local path reduced to its instance name (or
    /// [`PATH_PLACEHOLDER`]) and the user name replaced. Borrowed when there
    /// was nothing to change, which is almost every row.
    pub(crate) fn redact<'a>(&self, s: &'a str) -> Cow<'a, str> {
        let paths = self.redact_paths(s);
        match self.scrub_user(&paths) {
            Cow::Borrowed(_) => paths,
            Cow::Owned(o) => Cow::Owned(o),
        }
    }

    fn redact_paths<'a>(&self, s: &'a str) -> Cow<'a, str> {
        let Some(first) = next_path_start(s, 0) else {
            return Cow::Borrowed(s);
        };
        let mut out = String::with_capacity(s.len());
        let mut copied = 0;
        let mut at = Some(first);
        while let Some((start, body)) = at {
            let end = path_end(s, start, body);
            out.push_str(&s[copied..start]);
            out.push_str(self.reduce(&s[start..end]));
            copied = end;
            at = next_path_start(s, end);
        }
        out.push_str(&s[copied..]);
        Cow::Owned(out)
    }

    /// The instance a path belongs to, or [`PATH_PLACEHOLDER`]. The deepest
    /// match wins: `...\instances\p2\profile\...\inst-3\x` is `inst-3`.
    fn reduce<'p>(&self, path: &'p str) -> &'p str {
        let mut found: Option<&'p str> = None;
        let mut prev_is_instances = false;
        for comp in path.split(is_sep).filter(|c| !c.is_empty()) {
            let stem = comp.split('.').next().unwrap_or(comp);
            for label in &self.instances {
                // `...\inst-3\SGW.exe`, or a per-instance file name such as
                // `cimmeria-client-native-p2.log`.
                if comp.eq_ignore_ascii_case(label) {
                    found = Some(comp);
                } else if let Some(cut) = stem.len().checked_sub(label.len()) {
                    if cut > 0
                        && stem.is_char_boundary(cut)
                        && stem[cut..].eq_ignore_ascii_case(label)
                        && stem[..cut].ends_with('-')
                    {
                        found = Some(&stem[cut..]);
                    }
                }
            }
            // An instance this daemon does not host still sits under
            // `instances\<label>`; its label is as safe to export.
            if prev_is_instances && is_instance_name(comp) {
                found = Some(comp);
            }
            prev_is_instances = comp.eq_ignore_ascii_case(INSTANCES_DIR);
        }
        found.unwrap_or(PATH_PLACEHOLDER)
    }

    fn scrub_user<'a>(&self, s: &'a str) -> Cow<'a, str> {
        let Some(user) = self.user.as_deref() else {
            return Cow::Borrowed(s);
        };
        let hay = s.as_bytes();
        let needle = user.as_bytes();
        let mut out: Option<String> = None;
        let mut copied = 0;
        let mut i = 0;
        while i + needle.len() <= hay.len() {
            if hay[i..i + needle.len()].eq_ignore_ascii_case(needle)
                && s.is_char_boundary(i)
                && s.is_char_boundary(i + needle.len())
            {
                let o = out.get_or_insert_with(|| String::with_capacity(s.len()));
                o.push_str(&s[copied..i]);
                o.push_str(USER_PLACEHOLDER);
                i += needle.len();
                copied = i;
            } else {
                i += 1;
            }
        }
        match out {
            Some(mut o) => {
                o.push_str(&s[copied..]);
                Cow::Owned(o)
            }
            None => Cow::Borrowed(s),
        }
    }
}

/// The shape `supervisor::instance::validate_name` accepts (plus `default`):
/// 1 to 16 ASCII letters, digits, `-` or `_`.
fn is_instance_name(s: &str) -> bool {
    (1..=16).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn is_sep(c: char) -> bool {
    c == '\\' || c == '/'
}

/// Characters that end a path: quotes (a `Debug` string), list and field
/// punctuation, and the characters Windows forbids in a file name.
fn is_terminator(c: char) -> bool {
    matches!(
        c,
        '"' | '\'' | '`' | ',' | ';' | '|' | '<' | '>' | '*' | '?' | ':' | '\n' | '\r' | '\t'
    )
}

/// The next path at or after `from`: `(start, body)`, where `start` is the
/// first byte to replace and `body` the first byte after the root
/// (`C:\`, `\\?\C:\`, `\\server\`).
fn next_path_start(s: &str, from: usize) -> Option<(usize, usize)> {
    let b = s.as_bytes();
    let mut i = from;
    while i < b.len() {
        if let Some(body) = path_root_len(s, i).map(|n| i + n) {
            // A `\\?\` verbatim prefix (or its `Debug` form) belongs to the path.
            let mut start = i;
            while start > from && matches!(b[start - 1], b'\\' | b'?') {
                start -= 1;
            }
            return Some((start, body));
        }
        i += 1;
    }
    None
}

/// Length of a path root beginning at byte `i`, if one does: a drive
/// (`C:\`, `C:/`, `C:\\` in `Debug` form) not glued to a word (so the `p:`
/// in `http://` is not one), or a UNC `\\server\`.
fn path_root_len(s: &str, i: usize) -> Option<usize> {
    let b = s.as_bytes();
    let boundary = i == 0 || !b[i - 1].is_ascii_alphanumeric();
    if boundary
        && b[i].is_ascii_alphabetic()
        && b.get(i + 1) == Some(&b':')
        && b.get(i + 2).is_some_and(|c| *c == b'\\' || *c == b'/')
    {
        let mut n = 3;
        while b.get(i + n).is_some_and(|c| *c == b'\\' || *c == b'/') {
            n += 1;
        }
        return Some(n);
    }
    // UNC: `\\server\` (also `\\\\server\\` in Debug form). Needs the
    // trailing separator so a regex escape like `\\d+` is left alone.
    if boundary && b[i] == b'\\' && b.get(i + 1) == Some(&b'\\') {
        let mut j = i + 2;
        while b.get(j) == Some(&b'\\') {
            j += 1;
        }
        let host = j;
        while b
            .get(j)
            .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'_'))
        {
            j += 1;
        }
        if j > host && b.get(j) == Some(&b'\\') {
            return Some(j + 1 - i);
        }
    }
    None
}

/// Where the path that starts at `start` (root ends at `body`) ends.
///
/// Paths contain spaces (`Program Files (x86)`), so a space does not end
/// one; but the words after the last separator's file name are the rest of
/// the message, not the path. So: scan to a terminator or the next path,
/// then cut after the first word following the last separator.
fn path_end(s: &str, start: usize, body: usize) -> usize {
    let mut j = body;
    for (off, c) in s[body..].char_indices() {
        let at = body + off;
        if is_terminator(c) || (at > body && path_root_len(s, at).is_some()) {
            break;
        }
        j = at + c.len_utf8();
    }
    let candidate = &s[start..j];
    let root = body - start;
    let tail_from = candidate[root..]
        .rfind(is_sep)
        .map_or(root, |p| root + p + 1);
    let mut end = start
        + tail_from
        + candidate[tail_from..]
            .find(char::is_whitespace)
            .unwrap_or(candidate.len() - tail_from);
    // `(C:\x\y.log)` and `see C:\x\y.log.`: the closing punctuation is prose.
    while end > body {
        let last = s[..end].chars().next_back().unwrap_or(' ');
        let open = s[start..end].matches('(').count();
        let close = s[start..end].matches(')').count();
        if last == '.' || (last == ')' && close > open) || last == ']' {
            end -= 1;
        } else {
            break;
        }
    }
    end
}
