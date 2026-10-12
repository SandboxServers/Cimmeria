//! The output kit (D-LX5): colours, glyphs, widths and error output.
//!
//! Plain mode prints exactly what the PowerShell command printed. Fancy mode
//! adds the style table. Commands build their text with [`Ui::paint`] and
//! [`Ui::mark`]; no command writes escape codes itself.

pub mod table;

use std::ffi::OsStr;
use std::io::IsTerminal;

/// What the terminal supports, decided once per run by [`Ui::detect`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ui {
    /// Stdout is a terminal, `NO_COLOR` is unset or empty, and `TERM` is not `dumb`.
    pub color: bool,
    /// Glyphs, box characters and live redraws. The same as `color`.
    pub fancy: bool,
    /// Terminal columns when stdout is a terminal, else `None` (no truncation).
    pub width: Option<usize>,
}

/// The style of a piece of text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Section headings: bold yellow.
    Heading,
    /// Command names and paths a user types: cyan.
    Cmd,
    /// Secondary text, hints and ages: dim.
    Dim,
    /// Success: green.
    Ok,
    /// Warnings: yellow.
    Warn,
    /// Failures and the `error:` label: bold red.
    Fail,
    /// Emphasis: bold.
    Bold,
}

/// A status glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// `✓`, success.
    Pass,
    /// `!`, warning.
    Warn,
    /// `✗`, failure.
    Fail,
    /// `●`, running.
    Running,
    /// `○`, stopped.
    Stopped,
    /// `▶`, in progress.
    Active,
}

impl Ui {
    /// Decides colour and width from the environment and stdout.
    pub fn detect() -> Self {
        let tty = std::io::stdout().is_terminal();
        let color = color_enabled(
            tty,
            std::env::var_os("NO_COLOR").as_deref(),
            std::env::var_os("TERM").as_deref(),
        );
        let width = if tty {
            crossterm::terminal::size()
                .ok()
                .map(|(columns, _)| usize::from(columns))
        } else {
            None
        };
        Self {
            color,
            fancy: color,
            width,
        }
    }

    /// No colour, no glyphs, no truncation: the output tests compare against.
    pub fn plain() -> Self {
        Self {
            color: false,
            fancy: false,
            width: None,
        }
    }

    /// Wraps `text` in the escape codes for `tone`; unchanged when colour is off.
    pub fn paint(&self, tone: Tone, text: &str) -> String {
        if !self.color {
            return text.to_string();
        }
        let code = match tone {
            Tone::Heading => "1;33",
            Tone::Cmd => "36",
            Tone::Dim => "2",
            Tone::Ok => "32",
            Tone::Warn => "33",
            Tone::Fail => "1;31",
            Tone::Bold => "1",
        };
        format!("\x1b[{code}m{text}\x1b[0m")
    }

    /// The glyph for `mark`; empty when glyphs are off.
    pub fn mark(&self, mark: Mark) -> &'static str {
        if !self.fancy {
            return "";
        }
        match mark {
            Mark::Pass => "✓",
            Mark::Warn => "!",
            Mark::Fail => "✗",
            Mark::Running => "●",
            Mark::Stopped => "○",
            Mark::Active => "▶",
        }
    }

    /// Writes `error: <message>` to stderr, then an indented `hint: <hint>`.
    pub fn error(&self, message: &str, hint: Option<&str>) {
        anstream::eprintln!("{} {message}", self.paint(Tone::Fail, "error:"));
        if let Some(hint) = hint {
            anstream::eprintln!("  {}", self.paint(Tone::Dim, &format!("hint: {hint}")));
        }
    }
}

/// Colour and glyphs are on only for a terminal, with `NO_COLOR` unset or
/// empty and `TERM` not `dumb`.
fn color_enabled(tty: bool, no_color: Option<&OsStr>, term: Option<&OsStr>) -> bool {
    let no_color = no_color.is_some_and(|v| !v.is_empty());
    let dumb = term.is_some_and(|v| v == "dumb");
    tty && !no_color && !dumb
}

/// Terminal cells one character takes: 0 for combining marks, zero-width
/// joiners and variation selectors, 2 for East Asian wide characters and
/// emoji, else 1.
pub fn cell_width(c: char) -> usize {
    match u32::from(c) {
        0..=0x1f | 0x7f..=0x9f => 0,
        0x0300..=0x036f | 0x200b..=0x200f | 0x20d0..=0x20ff | 0xfe00..=0xfe0f => 0,
        0x1100..=0x115f
        | 0x231a..=0x231b
        | 0x2e80..=0x303e
        | 0x3041..=0x33ff
        | 0x3400..=0x4dbf
        | 0x4e00..=0xa4cf
        | 0xac00..=0xd7a3
        | 0xf900..=0xfaff
        | 0xfe30..=0xfe6f
        | 0xff00..=0xff60
        | 0xffe0..=0xffe6
        | 0x1f300..=0x1f64f
        | 0x1f680..=0x1f6ff
        | 0x1f900..=0x1f9ff
        | 0x20000..=0x3fffd => 2,
        _ => 1,
    }
}

/// The columns `text` takes on screen: colour codes count 0, wide characters 2.
pub fn text_width(text: &str) -> usize {
    strip_ansi(text).chars().map(cell_width).sum()
}

/// `text` without its `\x1b[...m` colour codes.
pub fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            skip_escape(&mut chars);
        } else {
            out.push(c);
        }
    }
    out
}

/// Consumes the rest of an escape sequence whose `` was just read: a `[`,
/// parameter bytes, then one final byte in `@`..=`~`.
pub(crate) fn skip_escape(chars: &mut std::str::Chars<'_>) -> String {
    let mut seq = String::new();
    let mut rest = chars.clone();
    if rest.next() != Some('[') {
        return seq;
    }
    seq.push('[');
    chars.next();
    for next in chars.by_ref() {
        seq.push(next);
        if ('@'..='~').contains(&next) {
            break;
        }
    }
    seq
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_is_unchanged_without_colour() {
        assert_eq!(Ui::plain().paint(Tone::Heading, "Usage"), "Usage");
        assert_eq!(Ui::plain().mark(Mark::Pass), "");
    }

    #[test]
    fn paint_wraps_text_with_colour() {
        let ui = Ui {
            color: true,
            fancy: true,
            width: None,
        };
        assert_eq!(ui.paint(Tone::Cmd, "uat"), "\x1b[36muat\x1b[0m");
        assert_eq!(ui.mark(Mark::Pass), "✓");
    }

    #[test]
    fn text_width_ignores_colour_codes_and_counts_wide_characters_twice() {
        assert_eq!(text_width("\x1b[1;33mUsage\x1b[0m"), 5);
        assert_eq!(strip_ansi("a\x1b[2mb\x1b[0m"), "ab");
        assert_eq!(text_width("日本語"), 6);
        assert_eq!(text_width("\x1b[36m日\x1b[0m"), 2);
    }

    #[test]
    fn strip_ansi_removes_non_colour_sequences_too() {
        assert_eq!(strip_ansi("a[2Kb"), "ab");
        assert_eq!(strip_ansi("a[1;33mb[0mc"), "abc");
    }

    #[test]
    fn colour_needs_a_terminal_no_no_color_and_no_dumb_term() {
        let os = |s: &'static str| Some(OsStr::new(s));
        assert!(color_enabled(true, None, None));
        assert!(color_enabled(true, os(""), os("xterm-256color")));
        assert!(!color_enabled(false, None, os("xterm")));
        assert!(!color_enabled(true, os("1"), None));
        assert!(!color_enabled(true, None, os("dumb")));
    }
}
