//! The output kit (D-LX5): colours, glyphs, widths and error output.
//!
//! Plain mode prints exactly what the PowerShell command printed. Fancy mode
//! adds the style table. Commands build their text with [`Ui::paint`] and
//! [`Ui::mark`]; no command writes escape codes itself.

pub mod table;

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
        let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
        let dumb = std::env::var_os("TERM").is_some_and(|v| v.to_str() == Some("dumb"));
        let color = tty && !no_color && !dumb;
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

/// The width of `text` on screen: its characters, without `\x1b[...m` codes.
pub fn visible_width(text: &str) -> usize {
    strip_ansi(text).chars().count()
}

/// `text` without its `\x1b[...m` colour codes.
pub fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip the sequence through its final `m`.
            for next in chars.by_ref() {
                if next == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
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
    fn visible_width_ignores_colour_codes() {
        assert_eq!(visible_width("\x1b[1;33mUsage\x1b[0m"), 5);
        assert_eq!(strip_ansi("a\x1b[2mb\x1b[0m"), "ab");
    }
}
