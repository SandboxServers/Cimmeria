//! Tests for the `sgw-start32` contract: the portable argument and
//! stdout round trips on every host, and (with `CIMMERIA_TEST_START32`
//! naming the built i686 helper) the helper run against real 32-bit
//! processes.

use super::*;

fn spawn_request() -> Request {
    Request {
        target: Target::Spawn {
            exe: PathBuf::from(r"C:\Game Dir\SGW.exe"),
            cwd: Some(PathBuf::from(r"C:\Game Dir")),
            args: vec!["-log".into(), "a b".into()],
        },
        dlls: vec![
            PathBuf::from(r"C:\p\patches.dll"),
            PathBuf::from(r"C:\p\tel.dll"),
        ],
    }
}

#[test]
fn spawn_request_round_trips_through_the_command_line() {
    let req = spawn_request();
    assert_eq!(parse_args(req.to_args()).unwrap(), req);
}

#[test]
fn pid_request_round_trips() {
    let req = Request {
        target: Target::Pid(4242),
        dlls: vec![PathBuf::from("x.dll")],
    };
    assert_eq!(parse_args(req.to_args()).unwrap(), req);
}

/// The DLL order is the injection order (patches before telemetry).
#[test]
fn dll_order_is_preserved() {
    let parsed = parse_args(spawn_request().to_args()).unwrap();
    assert_eq!(
        parsed.dlls,
        vec![
            PathBuf::from(r"C:\p\patches.dll"),
            PathBuf::from(r"C:\p\tel.dll")
        ]
    );
}

#[test]
fn usage_errors_are_reported() {
    let parse = |a: &[&str]| parse_args(a.iter().map(OsString::from));
    assert!(parse(&[]).is_err());
    assert!(parse(&["spawn", "x.exe"]).is_err(), "no --dll");
    assert!(parse(&["pid", "abc", "--dll", "d"]).is_err());
    assert!(parse(&["pid", "1", "--cwd", "c", "--dll", "d"]).is_err());
    assert!(parse(&["bogus", "x", "--dll", "d"]).is_err());
    assert!(parse(&["spawn", "x.exe", "--dll"]).is_err());
}

#[test]
fn outcomes_round_trip_through_stdout() {
    for o in [
        Outcome::Started { pid: 1234 },
        Outcome::Failed {
            kind: ErrorKind::RemoteLoadFailed,
            detail: "LoadLibraryW returned NULL".into(),
        },
        Outcome::Failed {
            kind: ErrorKind::NotFound,
            detail: String::new(),
        },
    ] {
        assert_eq!(
            parse_outcome(&o.format()),
            Some(o.clone()),
            "{}",
            o.format()
        );
    }
}

/// A multi-line detail must not break the one-line contract.
#[test]
fn failure_detail_is_one_line() {
    let o = Outcome::Failed {
        kind: ErrorKind::Spawn,
        detail: "line one\r\nline two".into(),
    };
    assert_eq!(o.format().lines().count(), 1);
    assert!(matches!(
        parse_outcome(&o.format()),
        Some(Outcome::Failed {
            kind: ErrorKind::Spawn,
            ..
        })
    ));
}

#[test]
fn exit_codes() {
    assert_eq!(Outcome::Started { pid: 1 }.exit_code(), 0);
    assert_eq!(ErrorKind::Usage.exit_code(), 2);
    assert_eq!(ErrorKind::Inject.exit_code(), 1);
}

#[test]
fn unknown_output_is_none() {
    assert_eq!(parse_outcome(""), None);
    assert_eq!(parse_outcome("hello"), None);
    assert_eq!(parse_outcome("error kind=nonsense detail=x"), None);
    assert_eq!(parse_outcome("ok pid=notanumber"), None);
}

/// A reader whose data ends in a read that never returns: a pipe whose
/// write end some other process still holds. Past the data it fails
/// instead of blocking, so a reader that went on to EOF fails the test.
struct PipeHeldOpen(std::io::Cursor<Vec<u8>>);

impl std::io::Read for PipeHeldOpen {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.0.read(buf)? {
            0 => Err(std::io::Error::other("read past the answer line")),
            n => Ok(n),
        }
    }
}

/// Issue #1064, the caller's half: `run` takes the first non-blank line
/// and stops, never reading on to EOF.
#[test]
fn answer_is_the_first_non_blank_line_and_reading_stops_there() {
    let mut reader = std::io::BufReader::with_capacity(
        4,
        PipeHeldOpen(std::io::Cursor::new(b"\r\n\nok pid=77\r\n".to_vec())),
    );
    let line = read_answer_line(&mut reader).unwrap();
    assert_eq!(parse_outcome(&line), Some(Outcome::Started { pid: 77 }));
}

/// A helper that exits without a word gives an empty answer (reported as
/// `Garbled`), and non-UTF-8 in the detail does not lose the line.
#[test]
fn answer_at_eof_is_empty_and_bad_utf8_is_kept() {
    let mut empty = std::io::Cursor::new(Vec::new());
    assert_eq!(read_answer_line(&mut empty).unwrap(), "");
    let mut bad = std::io::Cursor::new(b"error kind=spawn detail=C:\\\xff\n".to_vec());
    assert!(matches!(
        parse_outcome(&read_answer_line(&mut bad).unwrap()),
        Some(Outcome::Failed {
            kind: ErrorKind::Spawn,
            ..
        })
    ));
}

/// The built i686 helper, from `CIMMERIA_TEST_START32`. Unset locally
/// skips these tests; unset in CI fails them, so the helper tests can
/// never pass vacuously there (the launcher workflow builds the helper
/// and sets it).
#[cfg(windows)]
fn helper() -> Option<PathBuf> {
    match std::env::var_os("CIMMERIA_TEST_START32") {
        Some(p) => Some(PathBuf::from(p)),
        None if std::env::var_os("CI").is_some() => {
            panic!("CIMMERIA_TEST_START32 must name the built i686 sgw-start32.exe in CI")
        }
        None => {
            eprintln!("CIMMERIA_TEST_START32 unset; skipping the helper test");
            None
        }
    }
}

/// A copy of a 32-bit system program renamed SGW.exe, and a 32-bit
/// system DLL, or `None` without WOW64.
#[cfg(windows)]
fn wow64(exe: &str) -> Option<(tempfile::TempDir, PathBuf, PathBuf)> {
    let wow = PathBuf::from(std::env::var_os("SystemRoot")?).join("SysWOW64");
    let dll = wow.join("version.dll");
    if !wow.join(exe).is_file() || !dll.is_file() {
        return None;
    }
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("SGW.exe");
    std::fs::copy(wow.join(exe), &target).unwrap();
    Some((dir, target, dll))
}

/// The whole point: from THIS (x64) test build, the i686 helper
/// starts a real 32-bit process suspended, loads a real 32-bit DLL
/// into it, and resumes it. The caller then follows the pid and sees a
/// clean exit. A direct x64 injection of the same pair gets
/// `BitnessMismatch` (see `launch.rs`); through the helper it loads.
/// The `-n 3` arguments also pin argument passing (ping would exit
/// with a usage error without them).
#[cfg(windows)]
#[test]
fn helper_injects_a_dll_into_a_real_32_bit_process() {
    let Some(helper) = helper() else { return };
    let Some((_dir, exe, dll)) = wow64("PING.EXE") else {
        eprintln!("no SysWOW64 PING.EXE/version.dll; skipping");
        return;
    };
    let req = Request {
        target: Target::Spawn {
            exe,
            cwd: None,
            args: vec!["-n".into(), "3".into(), "127.0.0.1".into()],
        },
        dlls: vec![dll],
    };
    let pid = run(&helper, &req).expect("the helper must inject and resume");
    let game = crate::process::RunningProcess::open(pid).expect("follow the pid");
    assert_eq!(
        game.wait().unwrap(),
        0,
        "the injected program should exit cleanly"
    );
}

/// Issue #1064. A console-subsystem child used to receive a copy of the
/// helper's stdout pipe (Windows duplicates a console child's std
/// handles even with `bInheritHandles = FALSE`), and `run` read that pipe
/// to EOF, so it blocked for the target's whole life. The fix has two
/// halves, each with its own guard below, and this end-to-end guard
/// fails only when both are reverted (measured 2127 ms and 2465 ms
/// unfixed, 73-240 ms fixed).
///
/// The target is a 32-bit `PING -n 3`, a console program that lives
/// about 2 s: three echoes one second apart. Timing is the check, not "is
/// the target still running": the pipe closes while PING tears down,
/// before its process object is signalled, so a liveness probe right
/// after a blocked `run` still says "running".
#[cfg(windows)]
fn console_target_request() -> Option<(tempfile::TempDir, Request)> {
    let Some((dir, exe, dll)) = wow64("PING.EXE") else {
        eprintln!("no SysWOW64 PING.EXE/version.dll; skipping");
        return None;
    };
    let req = Request {
        target: Target::Spawn {
            exe,
            cwd: None,
            args: vec!["-n".into(), "3".into(), "127.0.0.1".into()],
        },
        dlls: vec![dll],
    };
    Some((dir, req))
}

/// Well under the ~2 s the console target lives.
#[cfg(windows)]
const PROMPT: std::time::Duration = std::time::Duration::from_millis(1000);

/// End to end, as the lab or a test host would call it: `run` returns
/// while the console target is still running.
#[cfg(windows)]
#[test]
fn helper_returns_before_a_console_target_exits() {
    let Some(helper) = helper() else { return };
    let Some((_dir, req)) = console_target_request() else {
        return;
    };
    let started = std::time::Instant::now();
    let pid = run(&helper, &req).expect("the helper must inject and resume");
    let elapsed = started.elapsed();
    // Opened before the assertion, so the pid is still this target's.
    let game = crate::process::RunningProcess::open(pid);
    eprintln!(
        "start32::run returned in {} ms against a ~2 s console target",
        elapsed.as_millis()
    );
    assert!(
        elapsed < PROMPT,
        "run took {} ms against a ~2 s console target: it waited for EOF \
         on a stdout pipe the target still held",
        elapsed.as_millis()
    );
    let game = game.expect("the target is still running, so its pid opens");
    assert_eq!(game.wait().unwrap(), 0, "PING should exit cleanly");
}

/// The helper's half (`DETACHED_PROCESS` in
/// `create_process_suspended_with_args`): the helper's stdout reaches EOF
/// when the helper exits, not when its console target does, and carries
/// only the helper's one line. Read to EOF on purpose, as the old `run`
/// did, so it measures the pipe and not `run`'s read.
#[cfg(windows)]
#[test]
fn helper_stdout_closes_with_the_helper_not_the_console_target() {
    let Some(helper) = helper() else { return };
    let Some((_dir, req)) = console_target_request() else {
        return;
    };
    let started = std::time::Instant::now();
    let out = std::process::Command::new(&helper)
        .args(req.to_args())
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .expect("run the helper");
    let elapsed = started.elapsed();
    let stdout = String::from_utf8_lossy(&out.stdout);
    eprintln!(
        "helper stdout reached EOF in {} ms against a ~2 s console target",
        elapsed.as_millis()
    );
    let pid = match parse_outcome(&stdout) {
        Some(Outcome::Started { pid }) => pid,
        other => panic!("expected `ok pid=`, got {other:?} from {stdout:?}"),
    };
    let game = crate::process::RunningProcess::open(pid);
    assert!(
        elapsed < PROMPT,
        "the helper's stdout stayed open {} ms: the console target held a copy",
        elapsed.as_millis()
    );
    assert_eq!(
        stdout.lines().filter(|l| !l.trim().is_empty()).count(),
        1,
        "only the helper may write to its stdout; the target's output got in: {stdout:?}"
    );
    if let Ok(game) = game {
        assert_eq!(game.wait().unwrap(), 0, "PING should exit cleanly");
    }
}

/// The caller's half (`run` reads one line, then waits for the helper):
/// even if something the helper started kept its stdout open, `run`
/// returns once the helper has answered and exited. A stand-in helper,
/// a batch file, answers `ok pid=4242`, leaves a `PING -n 3` holding its
/// stdout, and exits. Needs no built helper, so it runs on every Windows
/// test run.
#[cfg(windows)]
#[test]
fn run_returns_after_the_answer_even_if_the_pipe_stays_open() {
    let dir = tempfile::tempdir().unwrap();
    let stand_in = dir.path().join("stand-in-helper.cmd");
    // `start /b` gives PING cmd's own std handles, so PING holds the
    // stdout pipe for ~2 s after the batch has exited.
    std::fs::write(
        &stand_in,
        "@echo off\r\n\
         echo ok pid=4242\r\n\
         start \"\" /b \"%SystemRoot%\\System32\\PING.EXE\" -n 3 127.0.0.1\r\n",
    )
    .unwrap();
    let req = Request {
        target: Target::Pid(4242),
        dlls: vec![dir.path().join("unused.dll")],
    };
    let started = std::time::Instant::now();
    let pid = run(&stand_in, &req).expect("the stand-in answers ok");
    let elapsed = started.elapsed();
    eprintln!(
        "start32::run returned in {} ms with the pipe held open ~2 s",
        elapsed.as_millis()
    );
    assert_eq!(pid, 4242);
    assert!(
        elapsed < PROMPT,
        "run took {} ms: it read the helper's stdout to EOF instead of one line",
        elapsed.as_millis()
    );
}

/// The helper starts for a standard user: a spawn that fails with
/// os error 740 (UAC installer detection) never gets to answer. It
/// answers a usage error with exit code 2.
#[cfg(windows)]
#[test]
fn helper_starts_without_elevation_and_reports_usage() {
    let Some(helper) = helper() else { return };
    let out = std::process::Command::new(&helper)
        .output()
        .expect("the helper must start without elevation (os error 740 = UAC)");
    assert_eq!(out.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        matches!(
            parse_outcome(&stdout),
            Some(Outcome::Failed {
                kind: ErrorKind::Usage,
                ..
            })
        ),
        "{stdout}"
    );
}

/// The helper binary carries its asInvoker manifest.
#[cfg(windows)]
#[test]
fn helper_embeds_an_as_invoker_manifest() {
    let Some(helper) = helper() else { return };
    let bytes = std::fs::read(&helper).unwrap();
    assert!(
        bytes.windows(9).any(|w| w == b"asInvoker"),
        "no asInvoker manifest in {}",
        helper.display()
    );
}

#[cfg(windows)]
#[test]
fn helper_reports_a_missing_dll_as_not_found() {
    let Some(helper) = helper() else { return };
    let Some((dir, exe, _)) = wow64("PING.EXE") else {
        return;
    };
    let req = Request {
        target: Target::Spawn {
            exe,
            cwd: None,
            args: vec![],
        },
        dlls: vec![dir.path().join("missing.dll")],
    };
    match run(&helper, &req) {
        Err(HelperError::Failed {
            kind: ErrorKind::NotFound,
            ..
        }) => {}
        other => panic!("expected not_found, got {other:?}"),
    }
}

/// Proves the helper really runs `LoadLibraryW` inside the target and
/// checks its answer: a file that exists but is not a PE image makes
/// the remote load return NULL, reported as `remote_load_failed`. A
/// helper that skipped the injection would wrongly answer `ok`.
#[cfg(windows)]
#[test]
fn helper_reports_a_dll_that_fails_to_load() {
    let Some(helper) = helper() else { return };
    let Some((dir, exe, _)) = wow64("PING.EXE") else {
        return;
    };
    let not_a_dll = dir.path().join("not-a-dll.dll");
    std::fs::write(&not_a_dll, b"this is not a PE image").unwrap();
    let req = Request {
        target: Target::Spawn {
            exe,
            cwd: None,
            args: vec![],
        },
        dlls: vec![not_a_dll],
    };
    match run(&helper, &req) {
        Err(HelperError::Failed {
            kind: ErrorKind::RemoteLoadFailed,
            ..
        }) => {}
        other => panic!("expected remote_load_failed, got {other:?}"),
    }
}

#[test]
fn every_kind_name_parses_back() {
    for k in [
        ErrorKind::Usage,
        ErrorKind::NotFound,
        ErrorKind::Spawn,
        ErrorKind::OpenProcess,
        ErrorKind::BitnessMismatch,
        ErrorKind::RemoteLoadFailed,
        ErrorKind::Inject,
        ErrorKind::Resume,
        ErrorKind::Unsupported,
    ] {
        assert_eq!(ErrorKind::from_name(k.name()), Some(k));
    }
}
