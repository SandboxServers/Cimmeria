//! Same suspended/inject/resume primitives as start32, retaining the original
//! process handle so an immediate exit cannot race a later OpenProcess.
use super::*;
use cimmeria_client_launch::{
    inject::inject_dll, launch::strip_verbatim, process::create_process_suspended,
};
use std::io::{Read, Write};

/// The game's exe and working directory, without the verbatim prefix.
///
/// A native launcher sends canonical `\\?\C:\...` paths. Windows does not
/// resolve `..` under a verbatim working directory, and SGW.exe finds its
/// config as `..\SGWGame\Config\`, so it would quit with "Failed to find
/// default engine .ini file". A Wine guest path is already plain.
fn game_paths(request: &Request) -> (PathBuf, PathBuf) {
    (
        strip_verbatim(request.exe.clone()),
        strip_verbatim(request.directory.clone()),
    )
}

pub fn run() -> Result<(), &'static str> {
    if !cfg!(target_pointer_width = "32") {
        return Err("requires_x86");
    }
    let mut bytes = Vec::new();
    std::io::stdin()
        .take((MAX_MESSAGE + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "input")?;
    let request = Request::decode(&bytes)?;
    let emit = |observation| -> Result<(), &'static str> {
        let message = Message {
            schema_version: 1,
            operation_id: request.operation_id,
            observation,
        };
        let mut out = std::io::stdout().lock();
        serde_json::to_writer(&mut out, &message).map_err(|_| "output")?;
        out.write_all(b"\n")
            .and_then(|_| out.flush())
            .map_err(|_| "output")
    };
    if !request.exe.is_absolute()
        || !request.directory.is_absolute()
        || request.exe.parent() != Some(request.directory.as_path())
        || request
            .dlls
            .iter()
            .any(|p| !p.is_absolute() || !p.is_file())
    {
        return emit(Event::NotStarted);
    }
    let (exe, directory) = game_paths(&request);
    let suspended = match create_process_suspended(&exe, Some(&directory)) {
        Ok(process) => process,
        Err(_) => return emit(Event::NotStarted),
    };
    for dll in &request.dlls {
        if inject_dll(suspended.process_handle(), dll).is_err() {
            // The shared termination primitive is best effort, not proof of exit.
            suspended.terminate();
            return emit(Event::Unknown);
        }
    }
    let running = match suspended.resume_running() {
        Ok(process) => process,
        Err(_) => return emit(Event::Unknown),
    };
    let guest_pid = running.pid();
    // Loss of the launcher's pipe must not drop our process observation early.
    let started = emit(Event::ProcessStarted { guest_pid });
    let exited = running.wait();
    started?;
    match exited {
        Ok(code) => emit(Event::ProcessExited { guest_pid, code }),
        Err(_) => emit(Event::Unknown),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(exe: &str, directory: &str) -> Request {
        Request {
            schema_version: 1,
            operation_id: Uuid::from_u128(1),
            exe: exe.into(),
            directory: directory.into(),
            dlls: Vec::new(),
        }
    }

    // Bug shape: Play on Windows started SGW.exe under `\\?\C:\...` and the
    // game quit at once with "Failed to find default engine .ini file".
    #[test]
    fn a_verbatim_request_starts_the_game_under_plain_paths() {
        let (exe, directory) = game_paths(&request(
            r"\\?\C:\Games\SGW\Working\Binaries\SGW.exe",
            r"\\?\C:\Games\SGW\Working\Binaries",
        ));
        assert_eq!(exe, PathBuf::from(r"C:\Games\SGW\Working\Binaries\SGW.exe"));
        assert_eq!(directory, PathBuf::from(r"C:\Games\SGW\Working\Binaries"));
    }

    #[test]
    fn a_plain_guest_request_is_unchanged() {
        let plain = (
            r"C:\Games\SGW\Working\Binaries\SGW.exe",
            r"C:\Games\SGW\Working\Binaries",
        );
        let (exe, directory) = game_paths(&request(plain.0, plain.1));
        assert_eq!((exe, directory), (plain.0.into(), plain.1.into()));
    }
}
