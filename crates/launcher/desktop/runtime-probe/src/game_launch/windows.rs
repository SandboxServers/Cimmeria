//! Same suspended/inject/resume primitives as start32, retaining the original
//! process handle so an immediate exit cannot race a later OpenProcess.
use super::*;
use cimmeria_client_launch::{inject::inject_dll, process::create_process_suspended};
use std::io::{Read, Write};

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
    let suspended = match create_process_suspended(&request.exe, Some(&request.directory)) {
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
