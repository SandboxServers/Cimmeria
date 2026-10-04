use super::*;
use std::{fs, path::PathBuf};

/// An inert stand-in for the stock loader: the system shell under the loader's name,
/// and `script` under the name of the program the keeper asks Wine to run, so the
/// shell reads it. No Wine runs here.
fn fixture(script: &str) -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    std::os::unix::fs::symlink("/bin/sh", root.join("wine")).unwrap();
    fs::write(root.join(SHELL), format!("echo $$ > pid\n{script}\n")).unwrap();
    (temp, root)
}

fn environment() -> BTreeMap<OsString, OsString> {
    BTreeMap::from([
        ("PATH".into(), "/usr/bin:/bin".into()),
        ("FIXTURE_GAME_ENVIRONMENT".into(), "1".into()),
    ])
}

fn quick(ready: u64, exit: u64) -> Limits {
    Limits {
        ready: Duration::from_millis(ready),
        exit: Duration::from_millis(exit),
    }
}

fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn pid(root: &Path) -> i32 {
    wait_for("the keeper to record its pid", || {
        fs::read_to_string(root.join("pid")).is_ok_and(|text| text.ends_with('\n'))
    });
    fs::read_to_string(root.join("pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

fn running(pid: i32) -> bool {
    // SAFETY: signal 0 only checks that the process exists; nothing is delivered.
    unsafe { libc::kill(pid, 0) == 0 }
}

/// What the pinned `cmd` prints, then a wait for end of input like its `pause`.
const KEEPER: &str = "printf '%s\\n' \"$@\" > arguments\nenv > environment\n\
    printf 'CIMMERIA-DESKTOP-READY \\r\\nPress any key to continue... '\n\
    cat > /dev/null\necho ended > ended";

#[test]
fn keeper_gets_the_game_environment_and_lives_until_released() {
    let (_temp, root) = fixture(KEEPER);
    let host = DesktopHost::start(&root.join("wine"), &environment(), &|| false, LIMITS).unwrap();
    let keeper = pid(&root);
    assert_eq!(
        fs::read_to_string(root.join("arguments")).unwrap(),
        format!("/d\n/c\n{SCRIPT}\n")
    );
    let passed = fs::read_to_string(root.join("environment")).unwrap();
    assert!(passed
        .lines()
        .any(|line| line == "FIXTURE_GAME_ENVIRONMENT=1"));
    // The launcher's own environment must not reach Wine.
    assert!(!passed.lines().any(|line| line.starts_with("CARGO")));

    // Retained: output after the ready line did not end it, and nothing else does.
    std::thread::sleep(Duration::from_millis(300));
    assert!(running(keeper));
    assert!(!root.join("ended").exists());

    // Released: end of input alone ends it, and it is reaped.
    drop(host);
    wait_for("the keeper to end on end of input", || {
        root.join("ended").exists() && !running(keeper)
    });
}

#[test]
fn keeper_that_never_reports_is_stopped_before_the_fallback_launch() {
    // A near miss, then a keeper that ignores end of input.
    let (_temp, root) = fixture("echo \"$3\"\necho CIMMERIA-DESKTOP-READY-NOT\nexec sleep 600");
    let started = Instant::now();
    let result = DesktopHost::start(
        &root.join("wine"),
        &environment(),
        &|| false,
        quick(400, 200),
    );
    assert!(result.is_err());
    assert!(started.elapsed() >= Duration::from_millis(400));
    // Synchronously: the stock launch that follows must not overlap this keeper.
    assert!(!running(pid(&root)));
}

#[test]
fn keeper_that_ends_without_reporting_is_refused_at_once() {
    let (_temp, root) = fixture("exit 3");
    let started = Instant::now();
    assert!(DesktopHost::start(&root.join("wine"), &environment(), &|| false, LIMITS).is_err());
    assert!(started.elapsed() < Duration::from_secs(10));
    assert!(DesktopHost::start(&root.join("absent"), &environment(), &|| false, LIMITS).is_err());
}

#[test]
fn cancellation_ends_the_wait_and_the_keeper() {
    let (_temp, root) = fixture("exec sleep 600");
    let started = Instant::now();
    let limits = Limits {
        exit: Duration::from_secs(1),
        ..LIMITS
    };
    assert!(DesktopHost::start(&root.join("wine"), &environment(), &|| true, limits).is_err());
    assert!(started.elapsed() < Duration::from_secs(10));
    let keeper = pid(&root);
    wait_for("the cancelled keeper to be stopped", || !running(keeper));
}
