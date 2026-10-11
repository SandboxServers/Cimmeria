//! Startup of a launcher an update restarted. The updater starts the new
//! launcher with `--launcher-update-restart <old pid>` and then exits. The two
//! overlap briefly, so the new one waits (bounded) for the old process to
//! exit and only then registers the single-instance guard, as a normal start
//! does. Registering first would find the old window still holding the guard,
//! focus it and exit, and the update would never come back up. Skipping the
//! guard instead (LX-09's stopgap) left the restarted launcher unguarded for
//! its whole session.
use cimmeria_launcher_engine::updater::RESTART_ARG;
use std::{ffi::OsString, time::Duration};

/// How long the restarted launcher waits for the old one to exit. The state
/// lock retry in `NativeHost::store` uses the same bound.
pub const OLD_LAUNCHER_WAIT: Duration = Duration::from_secs(30);
const POLL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OldLauncher {
    /// An ordinary start.
    NotRestarted,
    /// A restart without a process id: nothing to wait for.
    Unknown,
    Exited {
        pid: u32,
    },
    /// Still running at the deadline. The guard is registered anyway; the
    /// state lock still keeps the two from sharing launcher state.
    StillRunning {
        pid: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Startup {
    pub old_launcher: OldLauncher,
    pub single_instance_guard: bool,
}

/// Wait for the old launcher, if this is an updater restart, then say how to
/// start. `alive` and `sleep` are injected so tests never wait for real.
pub fn prepare(
    args: impl IntoIterator<Item = OsString>,
    mut alive: impl FnMut(u32) -> bool,
    mut sleep: impl FnMut(Duration),
    wait: Duration,
) -> Startup {
    let old_launcher = match restart_pid(args) {
        None => OldLauncher::NotRestarted,
        Some(None) => OldLauncher::Unknown,
        Some(Some(pid)) => {
            let mut waited = Duration::ZERO;
            loop {
                if !alive(pid) {
                    break OldLauncher::Exited { pid };
                }
                if waited >= wait {
                    break OldLauncher::StillRunning { pid };
                }
                sleep(POLL);
                waited += POLL;
            }
        }
    };
    Startup {
        old_launcher,
        // Every start registers the guard; the wait above is what makes that
        // safe for a restart.
        single_instance_guard: true,
    }
}

/// `None`: not a restart. `Some(None)`: the flag without a usable pid.
fn restart_pid(args: impl IntoIterator<Item = OsString>) -> Option<Option<u32>> {
    let mut args = args.into_iter();
    args.find(|arg| arg == RESTART_ARG)?;
    Some(args.next().and_then(|pid| pid.to_str()?.parse().ok()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn a_restart_waits_for_the_old_pid_then_registers_the_guard() {
        let mut probes = Vec::new();
        let mut slept = Duration::ZERO;
        let startup = prepare(
            args(&["launcher.exe", RESTART_ARG, "4242"]),
            |pid| {
                probes.push(pid);
                probes.len() < 4 // alive for three probes, then gone
            },
            |d| slept += d,
            OLD_LAUNCHER_WAIT,
        );
        assert_eq!(startup.old_launcher, OldLauncher::Exited { pid: 4242 });
        assert_eq!(probes, [4242; 4]);
        assert_eq!(slept, POLL * 3, "slept between probes, not after the exit");
        assert!(startup.single_instance_guard);
    }

    #[test]
    fn the_wait_is_bounded_and_still_registers_the_guard() {
        let mut slept = Duration::ZERO;
        let startup = prepare(
            args(&["launcher.exe", RESTART_ARG, "7"]),
            |_| true,
            |d| slept += d,
            Duration::from_millis(500),
        );
        assert_eq!(startup.old_launcher, OldLauncher::StillRunning { pid: 7 });
        assert_eq!(slept, Duration::from_millis(500));
        assert!(startup.single_instance_guard);
    }

    #[test]
    fn an_ordinary_start_never_probes_and_registers_the_guard() {
        for list in [&["launcher.exe"][..], &["launcher.exe", "4242"]] {
            let startup = prepare(
                args(list),
                |_| panic!("probed an ordinary start"),
                |_| panic!("slept on an ordinary start"),
                OLD_LAUNCHER_WAIT,
            );
            assert_eq!(startup.old_launcher, OldLauncher::NotRestarted);
            assert!(startup.single_instance_guard);
        }
        let bare = prepare(
            args(&["launcher.exe", RESTART_ARG]),
            |_| panic!("no pid to probe"),
            |_| panic!("nothing to wait for"),
            OLD_LAUNCHER_WAIT,
        );
        assert_eq!(bare.old_launcher, OldLauncher::Unknown);
        assert!(bare.single_instance_guard);
    }
}
