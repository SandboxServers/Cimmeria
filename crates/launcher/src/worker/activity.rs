//! The worker's own record of what is running, so conflicting commands
//! are refused here as well as greyed out in the UI.
//!
//! The UI disables Install while the game runs and Play while an install
//! runs, but a disabled button is only a hint: a click that races a state
//! change, a stale frame, or a future caller can still dispatch. The
//! worker therefore checks every file-mutating or launching command
//! against [`Activity`] and answers a conflict with
//! [`super::Event::Refused`] instead of starting it.
//!
//! Besides what the worker started itself, an install or launch is
//! refused while any `SGW.exe` runs from the install
//! ([`crate::game_process`]), which covers a game started by an Atera
//! bat, by another launcher window, or one whose process could not be
//! followed.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Which command was refused, so the UI can undo its first-press state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Busy {
    Install,
    Launch,
    /// A maintenance job on the game's or the client's files (client
    /// setup, the client-state resets, Fix ASLR): refused while the game
    /// runs.
    Files,
}

/// Why a command was refused; its `Display` is the player-facing reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Conflict {
    InstallRunning,
    MaintenanceRunning,
    GameStarting,
    GameRunning { pid: Option<u32> },
}

impl std::fmt::Display for Conflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Conflict::InstallRunning => {
                write!(f, "an installation is in progress; wait for it to finish")
            }
            Conflict::MaintenanceRunning => write!(
                f,
                "a maintenance task is changing game files; wait for it to finish"
            ),
            Conflict::GameStarting => write!(f, "the game is already starting"),
            Conflict::GameRunning { pid: Some(pid) } => {
                write!(
                    f,
                    "Stargate Worlds is running (pid {pid}); close the game first"
                )
            }
            Conflict::GameRunning { pid: None } => {
                write!(f, "Stargate Worlds is running; close the game first")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Game {
    #[default]
    Idle,
    Launching,
    Running(u32),
}

/// A job that writes the game's or the client's files. At most one runs,
/// and never beside a launch or a running game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileJob {
    Install,
    Maintenance,
}

#[derive(Debug, Default)]
struct State {
    files: Option<FileJob>,
    game: Game,
}

/// Finds `SGW.exe` processes running from an install directory.
pub type GameProbe = Arc<dyn Fn(&Path) -> Vec<u32> + Send + Sync>;

/// Shared between the worker and the tasks it spawns; cheap to clone.
#[derive(Clone)]
pub struct Activity {
    state: Arc<Mutex<State>>,
    probe: GameProbe,
}

impl Activity {
    pub fn new(probe: GameProbe) -> Self {
        Self {
            state: Arc::default(),
            probe,
        }
    }

    /// The production probe: [`crate::game_process::running_game_pids`].
    pub fn production() -> Self {
        Self::new(Arc::new(crate::game_process::running_game_pids))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        // A panic while holding this lock leaves plain flags behind;
        // carrying on with them is safer than refusing every command.
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn conflict(&self, s: &State, dir: &Path) -> Option<Conflict> {
        match s.files {
            Some(FileJob::Install) => return Some(Conflict::InstallRunning),
            Some(FileJob::Maintenance) => return Some(Conflict::MaintenanceRunning),
            None => {}
        }
        match s.game {
            Game::Launching => return Some(Conflict::GameStarting),
            Game::Running(pid) => return Some(Conflict::GameRunning { pid: Some(pid) }),
            Game::Idle => {}
        }
        (self.probe)(dir)
            .first()
            .map(|&pid| Conflict::GameRunning { pid: Some(pid) })
    }

    /// Check and claim the file slot under one lock, so a launch cannot
    /// slip in between the check and the file change.
    fn begin_files(&self, dir: &Path, job: FileJob) -> Result<(), Conflict> {
        let mut s = self.lock();
        if let Some(c) = self.conflict(&s, dir) {
            return Err(c);
        }
        s.files = Some(job);
        Ok(())
    }

    /// Claim the file slot for an install or adoption: refused while any
    /// file job, launch or game from `install_dir` runs. Pair with
    /// [`Self::end_install`].
    pub fn begin_install(&self, install_dir: &Path) -> Result<(), Conflict> {
        self.begin_files(install_dir, FileJob::Install)
    }

    pub fn end_install(&self) {
        self.lock().files = None;
    }

    /// Claim the file slot for a maintenance job (client setup, Fix ASLR,
    /// the client-state resets) until [`Self::end_maintenance`]. An empty
    /// `dir` means any running game counts: the per-user client folders
    /// are shared by every install.
    pub fn begin_maintenance(&self, dir: &Path) -> Result<(), Conflict> {
        self.begin_files(dir, FileJob::Maintenance)
    }

    pub fn end_maintenance(&self) {
        self.lock().files = None;
    }

    /// Claim the game slot for a launch: refused while a file job runs,
    /// a launch is under way, or the game already runs.
    pub fn begin_launch(&self, install_dir: &Path) -> Result<(), Conflict> {
        let mut s = self.lock();
        if let Some(c) = self.conflict(&s, install_dir) {
            return Err(c);
        }
        s.game = Game::Launching;
        Ok(())
    }

    pub fn game_started(&self, pid: u32) {
        self.lock().game = Game::Running(pid);
    }

    /// The launch failed, the game exited, or it can no longer be
    /// followed (the probe still guards it then).
    pub fn game_ended(&self) {
        self.lock().game = Game::Idle;
    }

    /// Wait until the probe sees a game under `dir`, up to `timeout`.
    /// For a launch the worker cannot follow (an Atera bat starts
    /// `SGW.exe` itself) the launch slot stays claimed until then, so a
    /// second click cannot start another game.
    pub async fn wait_for_game(&self, dir: &Path, timeout: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if !(self.probe)(dir).is_empty() {
                return true;
            }
            if tokio::time::Instant::now() >= deadline {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiet() -> Activity {
        Activity::new(Arc::new(|_| Vec::new()))
    }

    #[test]
    fn a_second_install_is_refused_until_the_first_ends() {
        let a = quiet();
        let dir = Path::new("C:/g");
        a.begin_install(dir).unwrap();
        assert_eq!(a.begin_install(dir), Err(Conflict::InstallRunning));
        a.end_install();
        a.begin_install(dir).unwrap();
    }

    #[test]
    fn launch_and_install_exclude_each_other() {
        let a = quiet();
        let dir = Path::new("C:/g");
        a.begin_install(dir).unwrap();
        assert_eq!(a.begin_launch(dir), Err(Conflict::InstallRunning));
        a.end_install();

        a.begin_launch(dir).unwrap();
        assert_eq!(a.begin_install(dir), Err(Conflict::GameStarting));
        assert_eq!(a.begin_launch(dir), Err(Conflict::GameStarting));
        a.game_started(42);
        assert_eq!(
            a.begin_install(dir),
            Err(Conflict::GameRunning { pid: Some(42) })
        );
        a.game_ended();
        a.begin_install(dir).unwrap();
    }

    // Bug shape: a maintenance check that claimed nothing let a launch
    // start between the check and the file change.
    #[test]
    fn maintenance_and_launch_exclude_each_other() {
        let a = quiet();
        let dir = Path::new("C:/g");
        a.begin_maintenance(dir).unwrap();
        assert_eq!(a.begin_launch(dir), Err(Conflict::MaintenanceRunning));
        assert_eq!(a.begin_install(dir), Err(Conflict::MaintenanceRunning));
        assert_eq!(a.begin_maintenance(dir), Err(Conflict::MaintenanceRunning));
        a.end_maintenance();
        a.begin_launch(dir).unwrap();
        assert_eq!(a.begin_maintenance(dir), Err(Conflict::GameStarting));
    }

    #[tokio::test]
    async fn wait_for_game_sees_the_probe_or_times_out() {
        let dir = Path::new("C:/g");
        let seen = Activity::new(Arc::new(|_| vec![3]));
        assert!(seen.wait_for_game(dir, Duration::from_secs(1)).await);
        assert!(!quiet().wait_for_game(dir, Duration::from_millis(300)).await);
    }

    // A game this worker never started (reopened launcher, Atera bat)
    // still blocks file operations and a second launch.
    #[test]
    fn a_game_found_by_the_probe_blocks_install_and_launch() {
        let a = Activity::new(Arc::new(|_| vec![7]));
        let dir = Path::new("C:/g");
        let running = Err(Conflict::GameRunning { pid: Some(7) });
        assert_eq!(a.begin_install(dir), running);
        assert_eq!(a.begin_launch(dir), running);
    }
}
