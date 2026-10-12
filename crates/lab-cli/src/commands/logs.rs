//! `lab logs`: show the daemon log, filtered by instance or level, with
//! tokens masked. `--follow` polls the file and stops on Ctrl+C (exit 0).

use std::fs::{File, Metadata};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use regex::Regex;

use crate::commands::Ctx;
use crate::daemon::paths;
use crate::log_filter::{self, LogFilter};
use crate::ui::{Tone, Ui};

const LONG: &str = concat!(
    "Shows the last lines of the lab daemon's log, filtered by instance or level, with tokens masked. Reads labd.log under the lab home.",
    "\n\n",
    "--instance keeps lines that name the instance (instance=\"x\" or instance=Some(\"x\"), ignoring case). --level keeps lines whose level token, the second whitespace field (INFO, WARN, ...), is at or above the level. A line that does not start with a timestamp (a panic body, a backtrace) is kept or dropped with the entry above it.",
    "\n\n",
    "A bearer token or a 64-hex token prints as <redacted>.",
    "\n\n",
    "--follow streams the file through the same filter, so it can show fewer than --lines lines at first.",
);

const EXAMPLES: &str =
    "Examples:\n  lab logs --lines 5\n  lab logs --instance p2 --level warn --follow";

// Arguments of `lab logs`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Show the daemon log, filtered by instance or level",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {
    /// How many log lines to show
    #[arg(long, default_value_t = 50)]
    pub lines: u32,
    /// Keep streaming new lines (through the same filter)
    #[arg(long)]
    pub follow: bool,
    /// Keep only lines that name this instance
    #[arg(long)]
    pub instance: Option<String>,
    /// Keep lines at or above this level
    #[arg(long, ignore_case = true, value_parser = ["trace", "debug", "info", "warn", "error"])]
    pub level: Option<String>,
}

/// How often `--follow` looks at the file.
pub const POLL: Duration = Duration::from_millis(500);

/// What tells the file being followed from a different file at the same path:
/// the creation time on Windows, the inode elsewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileId(pub u128);

/// The identity of the file `meta` describes; `None` when the platform cannot
/// say. (NTFS "tunnelling" can hand a file renamed into a just-freed name its
/// predecessor's creation time, so a rotation is still mostly caught by the
/// length check in [`was_replaced`].)
#[cfg(windows)]
pub fn file_id(meta: &Metadata) -> Option<FileId> {
    let created = meta.created().ok()?;
    let since = created.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(FileId(since.as_nanos()))
}

/// The identity of the file `meta` describes (its inode).
#[cfg(unix)]
pub fn file_id(meta: &Metadata) -> Option<FileId> {
    use std::os::unix::fs::MetadataExt;
    Some(FileId(u128::from(meta.ino())))
}

/// No identity on other platforms; only the length check runs.
#[cfg(not(any(windows, unix)))]
pub fn file_id(_meta: &Metadata) -> Option<FileId> {
    None
}

/// Whether the file at the path is no longer the one being read: it is shorter
/// than the read position (truncated or rotated), or its identity changed (a
/// new file, possibly longer already).
pub fn was_replaced(pos: u64, len: u64, seen: Option<FileId>, now: Option<FileId>) -> bool {
    len < pos || matches!((seen, now), (Some(a), Some(b)) if a != b)
}

/// Set by the Ctrl+C handler; `--follow` stops when it is set.
static STOP: AtomicBool = AtomicBool::new(false);

/// Ctrl+C and Ctrl+Break set [`STOP`] instead of ending the process, so
/// `--follow` can return exit code 0. Declared by hand (no extra crate).
#[cfg(windows)]
fn install_ctrl_c() {
    #[link(name = "kernel32")]
    extern "system" {
        fn SetConsoleCtrlHandler(
            handler: Option<unsafe extern "system" fn(u32) -> i32>,
            add: i32,
        ) -> i32;
    }
    unsafe extern "system" fn on_ctrl(kind: u32) -> i32 {
        // CTRL_C_EVENT is 0 and CTRL_BREAK_EVENT is 1.
        if kind == 0 || kind == 1 {
            STOP.store(true, Ordering::SeqCst);
            1
        } else {
            0
        }
    }
    // SAFETY: `on_ctrl` is a valid handler for the life of the process and
    // only stores to an atomic.
    unsafe {
        SetConsoleCtrlHandler(Some(on_ctrl), 1);
    }
}

/// Off Windows Ctrl+C ends the process the default way.
#[cfg(not(windows))]
fn install_ctrl_c() {}

/// Plain mode: the line as it is. Fancy mode: the timestamp dim and the level
/// word coloured (ERROR red, WARN yellow, INFO as is, DEBUG and TRACE dim). A
/// continuation line is never coloured.
pub fn paint_line(ui: &Ui, line: &str) -> String {
    if !ui.color || !log_filter::is_entry_start(line) {
        return line.to_string();
    }
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^(\S+)(\s+)(\S+)(.*)$").expect("valid regex"));
    let Some(caps) = re.captures(line) else {
        return line.to_string();
    };
    let tone = match caps[3].to_ascii_lowercase().as_str() {
        "error" => Some(Tone::Fail),
        "warn" => Some(Tone::Warn),
        "debug" | "trace" => Some(Tone::Dim),
        _ => None,
    };
    let level = match tone {
        Some(tone) => ui.paint(tone, &caps[3]),
        None => caps[3].to_string(),
    };
    format!(
        "{}{}{}{}",
        ui.paint(Tone::Dim, &caps[1]),
        &caps[2],
        level,
        &caps[4]
    )
}

/// `lab logs --follow`: prints the last `tail` lines of the file that pass
/// `filter` (the raw tail is taken first, then filtered, so fewer can show),
/// then polls every `poll` for appended bytes and prints each new complete
/// line that passes. A file that shrinks, or is replaced by another file at the
/// same path (a different creation time or inode), is read again from the top.
/// A last line without its newline is held back, in the first snapshot too,
/// until the newline arrives.
/// Returns `Ok` when `stop` is set; an `Err` is a failed read of the first
/// snapshot or a failed write to `out`.
pub fn follow_file(
    path: &Path,
    ui: &Ui,
    filter: &mut LogFilter,
    tail: usize,
    stop: &AtomicBool,
    poll: Duration,
    out: &mut dyn Write,
) -> io::Result<()> {
    let mut first = File::open(path)?;
    let mut bytes = Vec::new();
    first.read_to_end(&mut bytes)?;
    let mut id = first.metadata().ok().as_ref().and_then(file_id);
    drop(first);
    let pos_at_eof = bytes.len() as u64;
    // A last line without its newline may still be growing: it is not part of
    // the snapshot but waits in `pending` for the poll loop to complete it.
    let complete = bytes
        .iter()
        .rposition(|&byte| byte == b'\n')
        .map_or(0, |last| last + 1);
    let mut pending: Vec<u8> = bytes.split_off(complete);
    let mut pos = pos_at_eof;
    let text = String::from_utf8_lossy(&bytes);
    let raw: Vec<&str> = text.lines().collect();
    let first = raw.len().saturating_sub(tail);
    for line in &raw[first..] {
        if let Some(kept) = filter.accept(line) {
            writeln!(out, "{}", paint_line(ui, &kept))?;
        }
    }
    out.flush()?;

    while !stop.load(Ordering::SeqCst) {
        std::thread::sleep(poll);
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let Ok(meta) = std::fs::metadata(path) else {
            continue;
        };
        let len = meta.len();
        let now = file_id(&meta);
        if was_replaced(pos, len, id, now) {
            pos = 0;
            pending.clear();
        }
        id = now.or(id);
        if len == pos {
            continue;
        }
        let Ok(mut file) = File::open(path) else {
            continue;
        };
        if file.seek(SeekFrom::Start(pos)).is_err() {
            continue;
        }
        let mut chunk = Vec::new();
        if file.read_to_end(&mut chunk).is_err() {
            continue;
        }
        pos += chunk.len() as u64;
        pending.extend_from_slice(&chunk);
        while let Some(end) = pending.iter().position(|&byte| byte == b'\n') {
            let raw_line: Vec<u8> = pending.drain(..=end).collect();
            let line = String::from_utf8_lossy(&raw_line);
            let line = line.trim_end_matches(['\r', '\n']);
            if let Some(kept) = filter.accept(line) {
                writeln!(out, "{}", paint_line(ui, &kept))?;
            }
        }
        out.flush()?;
    }
    Ok(())
}

/// Runs `lab logs`. Exit 0; 1 when there is no log (plain stderr text
/// `no daemon log yet: <path>`) or it cannot be read.
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let path = paths::labd_log_path();
    if !path.exists() {
        let message = format!("no daemon log yet: {}", path.display());
        if ctx.ui.fancy {
            ctx.ui.error(&message, Some("start the daemon: lab start"));
        } else {
            anstream::eprintln!("{message}");
        }
        return 1;
    }
    let instance = args.instance.as_deref().filter(|name| !name.is_empty());
    let level = args.level.as_deref().filter(|word| !word.is_empty());
    let count = args.lines as usize;
    let mut out = anstream::stdout().lock();

    if args.follow {
        install_ctrl_c();
        let mut filter = LogFilter::new(instance, level);
        return match follow_file(&path, &ctx.ui, &mut filter, count, &STOP, POLL, &mut out) {
            Ok(()) => 0,
            Err(err) if err.kind() == io::ErrorKind::BrokenPipe => 0,
            Err(err) => {
                ctx.ui
                    .error(&format!("cannot read {}: {err}", path.display()), None);
                1
            }
        };
    }

    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) => {
            ctx.ui
                .error(&format!("cannot read {}: {err}", path.display()), None);
            return 1;
        }
    };
    let text = String::from_utf8_lossy(&bytes);
    for line in log_filter::tail_lines(&text, instance, level, count) {
        if writeln!(out, "{}", paint_line(&ctx.ui, &line)).is_err() {
            break;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, try_parse, Command};

    /// Both documented example lines parse, with the right values.
    #[test]
    fn examples_parse() {
        let Command::Logs(a) = parse_ok("logs", &["-Lines", "5"]) else {
            panic!("not logs")
        };
        assert_eq!((a.lines, a.follow), (5, false));
        let Command::Logs(a) = parse_ok("logs", &["-Instance", "p2", "-Level", "warn", "-Follow"])
        else {
            panic!("not logs")
        };
        assert_eq!(a.instance.as_deref(), Some("p2"));
        assert_eq!(a.level.as_deref(), Some("warn"));
        assert!(a.follow);
    }

    #[test]
    fn defaults_and_bad_level() {
        let Command::Logs(a) = parse_ok("logs", &[]) else {
            panic!("not logs")
        };
        assert_eq!(a.lines, 50);
        assert!(try_parse("logs", &["-Level", "loud"]).is_err());
    }
}
