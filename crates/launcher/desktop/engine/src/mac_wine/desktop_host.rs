//! Wine's desktop host for a Play that runs the staged loader.
//!
//! Every Wine desktop has one windowless `explorer.exe /desktop` process. Wine starts
//! it from the loader of whichever process first asks for the desktop window. Left to
//! the game, that is the staged bundle, and the host becomes a second running
//! application with the game's bundle identifier. So a keeper from the stock loader
//! asks first and stays for the whole Play.
//!
//! Nothing here stops the host. Wine closes a desktop one second after its last user
//! leaves, and the keeper leaves when its input closes, which also covers the launcher
//! dying. A game that is still running keeps the host; one that never started does not.
use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::{self, BufReader, Read},
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

const READY: &str = "CIMMERIA-DESKTOP-READY";
const SHELL: &str = r"C:\windows\system32\cmd.exe";
/// `rundll32` with no arguments creates its hidden owner window and exits. Creating a
/// window is what makes Wine start the desktop host and wait for it. `cmd /c` waits
/// for that, reports only if it succeeded, then blocks in `pause` until input ends.
const SCRIPT: &str = r"C:\windows\system32\rundll32.exe && echo CIMMERIA-DESKTOP-READY && pause";

#[derive(Clone, Copy)]
pub(crate) struct Limits {
    pub(crate) ready: Duration,
    pub(crate) exit: Duration,
}

/// The keeper may be the first process of the Wine session and wait out its start-up.
/// The launch worker's own first reply has the same bound.
pub(crate) const LIMITS: Limits = Limits {
    ready: Duration::from_secs(60),
    exit: Duration::from_secs(10),
};

/// Keeps the stock desktop host alive until dropped.
pub(crate) struct DesktopHost {
    keeper: Option<(Child, ChildStdin)>,
    exit: Duration,
}

impl DesktopHost {
    /// Returns once the desktop window exists, so nothing started afterwards can start
    /// a second host. `environment` must be the one the game gets: the host loads the
    /// graphics driver for the whole desktop.
    pub(crate) fn start(
        stock: &Path,
        environment: &BTreeMap<OsString, OsString>,
        cancelled: &dyn Fn() -> bool,
        limits: Limits,
    ) -> io::Result<Self> {
        let directory = stock
            .parent()
            .ok_or_else(|| io::Error::other("loader has no directory"))?;
        let mut keeper = Command::new(stock)
            .args([SHELL, "/d", "/c", SCRIPT])
            .env_clear()
            .envs(environment)
            .current_dir(directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let input = keeper.stdin.take().expect("piped above");
        let output = keeper.stdout.take().expect("piped above");
        let mut host = Self {
            keeper: Some((keeper, input)),
            exit: limits.exit,
        };
        let (report, reported) = mpsc::channel();
        std::thread::spawn(move || watch(output, &report));
        let deadline = Instant::now() + limits.ready;
        loop {
            match reported.recv_timeout(Duration::from_millis(50)) {
                Ok(()) => return Ok(host),
                Err(mpsc::RecvTimeoutError::Timeout)
                    if !cancelled() && Instant::now() < deadline => {}
                Err(_) => break,
            }
        }
        if !cancelled() {
            // The caller falls back to a stock launch. Wait, so that launch cannot
            // overlap a host this keeper is still starting.
            host.settle();
        }
        Err(io::Error::other("Wine desktop host did not start"))
    }

    fn settle(&mut self) {
        if let Some((keeper, input)) = self.keeper.take() {
            drop(input);
            reap(keeper, self.exit);
        }
    }
}

impl Drop for DesktopHost {
    fn drop(&mut self) {
        if let Some((keeper, input)) = self.keeper.take() {
            // End of input ends the keeper's `pause` now; only reaping is deferred,
            // because this runs where Play's task finishes.
            drop(input);
            let exit = self.exit;
            std::thread::spawn(move || reap(keeper, exit));
        }
    }
}

/// Reports each ready line and keeps the pipe open until the keeper is gone, so its
/// later output never fails.
fn watch(output: impl Read, report: &mpsc::Sender<()>) {
    let mut line = Vec::new();
    for byte in BufReader::new(output).bytes().map_while(Result::ok) {
        if byte != b'\n' {
            if line.len() < 64 {
                line.push(byte);
            }
            continue;
        }
        if line.trim_ascii() == READY.as_bytes() {
            let _ = report.send(());
        }
        line.clear();
    }
}

/// A keeper that has not reached `pause` is killed. It is a windowless `cmd`, never
/// the game, and a host it leaves behind closes with its last user.
fn reap(mut keeper: Child, limit: Duration) {
    let deadline = Instant::now() + limit;
    while matches!(keeper.try_wait(), Ok(None)) {
        if Instant::now() >= deadline {
            let _ = keeper.kill();
            let _ = keeper.wait();
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(test)]
#[path = "desktop_host_tests.rs"]
mod tests;
