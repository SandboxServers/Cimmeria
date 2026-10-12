//! The live view of `lab watch`: alternate screen, hidden cursor, a redraw
//! every interval, quit on `q`, `Esc` or Ctrl+C, and the screen always put back.

use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use chrono::Local;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::Command;

use crate::daemon::mcp::hide_lease_ids;
use crate::ui::{Tone, Ui};

use super::frame::{fit_line, render_frame};
use super::{take_snapshot, Args};

/// How long one wait for a key lasts; also the sleep step without keys.
const TICK: Duration = Duration::from_millis(100);

/// The ANSI bytes of a crossterm command, the same on every OS.
fn ansi(command: impl Command) -> String {
    let mut text = String::new();
    command
        .write_ansi(&mut text)
        .expect("writing to a String cannot fail");
    text
}

/// Enters the alternate screen with the cursor hidden; puts both back on drop.
pub struct ScreenGuard<W: Write> {
    out: W,
    /// Whether this guard's owner switched raw mode on, so drop switches it off.
    raw: bool,
}

impl<W: Write> ScreenGuard<W> {
    /// Writes the enter sequence. `raw` says raw mode is on and is to be
    /// switched off on drop. If the write fails the guard is dropped on the way
    /// out, so the screen and raw mode are restored.
    pub fn enter(out: W, raw: bool) -> io::Result<Self> {
        let mut guard = Self { out, raw };
        let seq = format!("{}{}", ansi(EnterAlternateScreen), ansi(Hide));
        guard.out.write_all(seq.as_bytes())?;
        guard.out.flush()?;
        Ok(guard)
    }

    /// Draws one frame over the previous one.
    pub fn draw(&mut self, lines: &[String]) -> io::Result<()> {
        self.out.write_all(encode_frame(lines).as_bytes())?;
        self.out.flush()
    }
}

impl<W: Write> Drop for ScreenGuard<W> {
    fn drop(&mut self) {
        // Raw mode first: a write to a closed console must not leave the parent
        // shell without its line editing.
        if self.raw {
            let _ = terminal::disable_raw_mode();
        }
        let seq = format!("{}{}", ansi(Show), ansi(LeaveAlternateScreen));
        let _ = self.out.write_all(seq.as_bytes());
        let _ = self.out.flush();
    }
}

/// One frame as terminal bytes: home, each line then erase-to-end-of-line and
/// CRLF (raw mode does not translate `\n`), then erase below.
pub fn encode_frame(lines: &[String]) -> String {
    let mut out = ansi(MoveTo(0, 0));
    let erase_line = ansi(Clear(ClearType::UntilNewLine));
    for line in lines {
        out.push_str(line);
        out.push_str(&erase_line);
        out.push_str("\r\n");
    }
    out.push_str(&ansi(Clear(ClearType::FromCursorDown)));
    out
}

/// Whether this event asks to quit: `q`, `Esc`, or Ctrl+C (key presses only).
pub fn is_quit(event: &Event) -> bool {
    let Event::Key(key) = event else {
        return false;
    };
    if key.kind != KeyEventKind::Press {
        return false;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Esc => true,
        KeyCode::Char(c) => {
            (!ctrl && c.eq_ignore_ascii_case(&'q')) || (ctrl && c.eq_ignore_ascii_case(&'c'))
        }
        _ => false,
    }
}

/// The frame width for a terminal `columns` wide: one less (a full-width line
/// wraps on some hosts), or 120 when the terminal is narrower than 41 columns
/// or its size is unknown (0).
pub fn live_width(columns: u16) -> usize {
    if columns >= 41 {
        usize::from(columns) - 1
    } else {
        120
    }
}

/// The line above the frame.
pub fn title_line(clock: &str, interval_seconds: u32, ui: &Ui) -> String {
    ui.paint(
        Tone::Dim,
        &format!("lab watch  {clock}  every {interval_seconds}s  q to quit"),
    )
}

/// One tick's lines: the title, then the frame (cut to the rows left under the
/// title) or, when the snapshot cannot be read, one red error line.
fn compose(
    args: &Args,
    batch_dir: &Option<PathBuf>,
    ui: &Ui,
    columns: u16,
    rows: u16,
) -> Vec<String> {
    let width = live_width(columns);
    let clock = Local::now().format("%H:%M:%S").to_string();
    let mut lines = vec![fit_line(
        &title_line(&clock, args.interval_seconds, ui),
        width,
    )];
    match take_snapshot(args, batch_dir.clone()) {
        Ok(snapshot) => {
            let height = usize::from(rows).saturating_sub(1);
            lines.extend(render_frame(&snapshot, width, height, ui));
        }
        Err(why) => {
            // One bad tick (a file mid-write, a daemon restart) is shown, not fatal.
            let first = why.lines().next().unwrap_or("");
            let text = format!("error: {}", hide_lease_ids(first));
            lines.push(fit_line(&ui.paint(Tone::Fail, &text), width));
        }
    }
    lines
}

/// Waits one interval; true when a quit key was pressed. Without keys (raw mode
/// unavailable) it only sleeps.
fn wait_tick(interval: Duration, keys: bool) -> bool {
    let until = Instant::now() + interval;
    while Instant::now() < until {
        if !keys {
            std::thread::sleep(TICK);
            continue;
        }
        match event::poll(TICK) {
            Ok(true) => {
                if event::read().is_ok_and(|event| is_quit(&event)) {
                    return true;
                }
            }
            Ok(false) => {}
            Err(_) => std::thread::sleep(TICK),
        }
    }
    false
}

/// Runs the live view until `q`, `Esc` or Ctrl+C. Returns 0, or 3 when the
/// terminal cannot be written to. `batch_dir` is `--batch`, already checked.
pub fn run_live(args: &Args, ui: &Ui, batch_dir: Option<PathBuf>) -> u8 {
    let interval = Duration::from_secs(u64::from(args.interval_seconds));
    // On Windows, execute! switches virtual-terminal output on as a side effect
    // (the guard writes raw ANSI); on other systems it does nothing of note.
    let _ = crossterm::execute!(io::stdout(), crossterm::style::ResetColor);
    let raw = terminal::enable_raw_mode().is_ok();
    let mut guard = match ScreenGuard::enter(io::stdout(), raw) {
        Ok(guard) => guard,
        Err(_) => return 3,
    };
    loop {
        let (columns, rows) = terminal::size().unwrap_or((0, 0));
        let lines = compose(args, &batch_dir, ui, columns, rows);
        if guard.draw(&lines).is_err() {
            return 3; // the guard restores on drop
        }
        if wait_tick(interval, raw) {
            return 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventState};

    const ENTER: &str = "\x1b[?1049h\x1b[?25l";
    const LEAVE: &str = "\x1b[?25h\x1b[?1049l";

    fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    // Entering writes the alternate screen and hide-cursor sequences, in that order.
    // Fails if reverted: nothing is written, or the cursor is hidden before the switch.
    #[test]
    fn the_guard_enters_the_alternate_screen_and_hides_the_cursor() {
        let mut buf: Vec<u8> = Vec::new();
        let guard = ScreenGuard::enter(&mut buf, false).unwrap();
        std::mem::forget(guard); // look at the bytes before the drop writes more
        assert_eq!(String::from_utf8(buf).unwrap(), ENTER);
    }

    // Dropping writes show-cursor, then leave-screen.
    // Fails if reverted: with the Drop impl removed the buffer ends after ENTER and
    // a user's terminal is left on the alternate screen with no cursor.
    #[test]
    fn dropping_the_guard_restores_the_cursor_and_the_screen() {
        let mut buf: Vec<u8> = Vec::new();
        drop(ScreenGuard::enter(&mut buf, false).unwrap());
        assert_eq!(String::from_utf8(buf).unwrap(), format!("{ENTER}{LEAVE}"));
    }

    // A panic while the guard is alive still restores the terminal.
    // Fails if reverted: restoring in a normal-exit path only (not Drop) misses this.
    #[test]
    fn a_panic_restores_the_screen_too() {
        let mut buf: Vec<u8> = Vec::new();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = ScreenGuard::enter(&mut buf, false).unwrap();
            panic!("a bad tick");
        }));
        assert!(result.is_err());
        assert!(String::from_utf8(buf).unwrap().ends_with(LEAVE));
    }

    // A frame: home, each line + erase-to-eol + CRLF, then erase below.
    // Fails if reverted: a missing CR (raw mode staircases the lines), a missing
    // erase-to-eol (a shorter line keeps the old tail) or erase-below (a shorter
    // frame keeps the old bottom).
    #[test]
    fn a_frame_homes_erases_each_line_and_clears_below() {
        let lines = vec!["a".to_string(), "bc".to_string()];
        assert_eq!(
            encode_frame(&lines),
            "\x1b[1;1Ha\x1b[K\r\nbc\x1b[K\r\n\x1b[J"
        );
        let mut buf: Vec<u8> = Vec::new();
        let mut guard = ScreenGuard::enter(&mut buf, false).unwrap();
        guard.draw(&lines).unwrap();
        drop(guard);
        let text = String::from_utf8(buf).unwrap();
        assert!(text.contains("\x1b[1;1Ha\x1b[K\r\n"), "{text:?}");
    }

    // The quit keys: q, Q, Esc, Ctrl+C, and nothing else.
    // Fails if reverted: a lost key (q, Esc or Ctrl+C), or a stray key quitting
    // (plain c, Ctrl+Q, a key release, a resize).
    #[test]
    fn q_esc_and_ctrl_c_quit() {
        let none = KeyModifiers::NONE;
        let ctrl = KeyModifiers::CONTROL;
        assert!(is_quit(&key(KeyCode::Char('q'), none)));
        assert!(is_quit(&key(KeyCode::Char('Q'), KeyModifiers::SHIFT)));
        assert!(is_quit(&key(KeyCode::Esc, none)));
        assert!(is_quit(&key(KeyCode::Char('c'), ctrl)));
        assert!(!is_quit(&key(KeyCode::Char('c'), none)));
        assert!(!is_quit(&key(KeyCode::Char('x'), none)));
        assert!(!is_quit(&key(KeyCode::Char('q'), ctrl)));
        assert!(!is_quit(&key(KeyCode::Enter, none)));
        let release = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: none,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        };
        assert!(!is_quit(&Event::Key(release)));
        assert!(!is_quit(&Event::Resize(80, 24)));
    }

    // Width: columns less one, or 120 under 41 columns or when unknown.
    // Fails if reverted: a full-width frame (wraps), or 40 columns kept narrow.
    #[test]
    fn the_frame_is_one_column_narrower_than_the_terminal() {
        assert_eq!(live_width(120), 119);
        assert_eq!(live_width(41), 40);
        assert_eq!(live_width(40), 120);
        assert_eq!(live_width(0), 120);
    }

    // The title.
    // Fails if reverted: the interval or the quit hint is missing.
    #[test]
    fn the_title_shows_the_clock_the_interval_and_the_quit_key() {
        assert_eq!(
            title_line("01:41:23", 2, &Ui::plain()),
            "lab watch  01:41:23  every 2s  q to quit"
        );
    }
}
