//! Sets up the private prefix before the first Play (C-3, D-PP2).
//!
//! `ensure` runs `proton run wineboot.exe` only when the prefix stamp is missing
//! or names another Proton build. The setup counts as good only when the script
//! exits 0 and the prefix has its `system.reg`; the stamp then records the build
//! and keeps the PhysX state the prefix already had.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::File;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use super::discovery::ProtonInstall;
use super::invocation;
use super::prefix::{Prefix, ProtonError, Stamp};

/// The stamp schema this build writes.
const SCHEMA_VERSION: u32 = 1;

/// How long the setup may run before it is killed.
const SETUP_TIMEOUT: Duration = Duration::from_secs(180);

/// Runs the prefix setup when the stamp needs it, then writes the stamp.
/// `env` is the complete child environment (`environment::base`); stderr is
/// appended to `log` when given, otherwise discarded.
pub async fn ensure(
    install: &ProtonInstall,
    prefix: &Prefix,
    env: &BTreeMap<OsString, OsString>,
    log: Option<&Path>,
) -> Result<(), ProtonError> {
    ensure_within(install, prefix, env, log, SETUP_TIMEOUT).await
}

/// `ensure` with the time limit as a parameter, so the tests can use a short one.
async fn ensure_within(
    install: &ProtonInstall,
    prefix: &Prefix,
    env: &BTreeMap<OsString, OsString>,
    log: Option<&Path>,
    limit: Duration,
) -> Result<(), ProtonError> {
    if !prefix.needs_setup(install) {
        return Ok(());
    }
    let inv = invocation::setup(install);
    let mut command = tokio::process::Command::new(&inv.program);
    command
        .args(&inv.args)
        .env_clear()
        .envs(env)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(stderr_for(log)?)
        .kill_on_drop(true);
    let mut child = command.spawn().map_err(|_| ProtonError::Setup("prefix"))?;
    // A timeout returns early and drops the child, which `kill_on_drop` kills.
    let status = tokio::time::timeout(limit, child.wait()).await;
    let exited_ok = matches!(status, Ok(Ok(code)) if code.success());
    if !exited_ok || !prefix.pfx.join("system.reg").is_file() {
        return Err(ProtonError::Setup("prefix"));
    }
    let physx = prefix.stamp().map(|stamp| stamp.physx).unwrap_or_default();
    prefix.write_stamp(&Stamp {
        schema_version: SCHEMA_VERSION,
        proton_version: install.version.clone(),
        physx,
    })
}

/// Where the setup's stderr goes: appended to `log`, or discarded.
fn stderr_for(log: Option<&Path>) -> Result<Stdio, ProtonError> {
    match log {
        None => Ok(Stdio::null()),
        Some(path) => {
            let file = File::options()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|_| ProtonError::Setup("prefix"))?;
            Ok(Stdio::from(file))
        }
    }
}

#[cfg(test)]
#[path = "setup_tests.rs"]
mod tests;
