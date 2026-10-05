//! Bounded prerequisite helper transport. Durable admission belongs to the caller.
//! Observed output still requires prefix quiescence and a durable result commit.
use crate::helper_supervisor::{Deadlines, Fault, HelperCommand};
use cimmeria_runtime_probe::prerequisite::{
    decode_request, decode_result, PrepareRequest, PrepareResult, MAX_RESULT,
};
use std::process::Stdio;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    time::timeout,
};
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub enum Outcome {
    NotStarted(Fault),
    /// Both strict protocol and successful host exit were observed. This does
    /// not mean the reported installer/SDK succeeded or guest descendants exited.
    Observed(PrepareResult),
    ReconciliationRequired(Fault),
}

/// The caller must own the prefix and record launch intent before invoking this
/// retained native task. `record_host` must persist identity before request bytes
/// are sent. No caller may infer Wine guest death from this function stopping its
/// host child. Cancel/timeout after dispatch always retain a reconciliation gate.
pub async fn run(
    spec: HelperCommand,
    request: PrepareRequest,
    cancel: CancellationToken,
    limits: Deadlines,
    record_host: impl FnOnce(u32) -> Result<(), ()>,
) -> Outcome {
    if !spec.executable.is_absolute() || !spec.directory.is_absolute() {
        return Outcome::NotStarted(Fault::InvalidRequest);
    }
    let Ok(frame) = serde_json::to_vec(&request) else {
        return Outcome::NotStarted(Fault::InvalidRequest);
    };
    if decode_request(&frame).is_err() {
        return Outcome::NotStarted(Fault::InvalidRequest);
    }
    if cancel.is_cancelled() {
        return Outcome::NotStarted(Fault::Cancelled);
    }
    let mut command = Command::new(spec.executable);
    command
        .args(spec.arguments)
        .current_dir(spec.directory)
        .env_clear()
        .envs(spec.environment)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let Ok(mut child) = command.spawn() else {
        return Outcome::NotStarted(Fault::Spawn);
    };
    if child.id().is_none_or(|id| record_host(id).is_err()) {
        drop(child.stdin.take());
        let _ = timeout(limits.exit, child.kill()).await;
        return Outcome::NotStarted(Fault::Ownership);
    }
    let mut input = child.stdin.take().expect("piped stdin");
    let output = child.stdout.take().expect("piped stdout");
    let attempt = async {
        timeout(limits.request, input.write_all(&frame))
            .await
            .map_err(|_| Fault::Deadline)?
            .map_err(|_| Fault::Transport)?;
        drop(input); // Complete one request; worker cannot execute before EOF.
        let mut bytes = Vec::new();
        output
            .take(MAX_RESULT as u64 + 1)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| Fault::Transport)?;
        if bytes.len() > MAX_RESULT {
            return Err(Fault::Protocol);
        }
        let result = decode_result(&bytes, request.operation_id, request.prefix_generation)
            .map_err(|_| Fault::Protocol)?;
        let status = timeout(limits.exit, child.wait())
            .await
            .map_err(|_| Fault::Deadline)?
            .map_err(|_| Fault::Exit)?;
        if !status.success() {
            return Err(Fault::Exit);
        }
        Ok(result)
    };
    let result = tokio::select! {
        _ = cancel.cancelled() => Err(Fault::Cancelled),
        result = timeout(limits.operation, attempt) => result.unwrap_or(Err(Fault::Deadline)),
    };
    match result {
        Ok(result) => Outcome::Observed(result),
        Err(fault) => {
            // Stops only the owned host handle. Prefix-specific cleanup and its
            // durable evidence are mandatory in the platform coordinator.
            let _ = timeout(limits.exit, child.kill()).await;
            Outcome::ReconciliationRequired(fault)
        }
    }
}
