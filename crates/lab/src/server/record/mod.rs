//! The record tools (lab-record LR-07, LR-08): `lab_record_start`,
//! `lab_record_status`, `lab_record_stop` and `lab_record_transform`.
//!
//! A person plays one lab instance by hand while the recorder samples it
//! (D-LR1, D-LR2); stop turns the raw log into a draft spec. Each tool runs
//! on the instance the call is routed to (`instance`, else the lease's,
//! else the first, F12). Answers are one compact object under 400
//! characters with no nulls; a refusal or failure is
//! `{"ok":false,"exit":N,"error":"..."}` marked as a tool error (D-LR16).
//! The recorder is [`run`] (start and the sampler task) and [`stop`]
//! (status, stop and the draft).

mod checks;
mod run;
mod stop;
mod transform;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use rmcp::{
    handler::server::wrapper::Parameters, model::*, schemars, tool, tool_router,
    ErrorData as McpError,
};
use serde_json::Value;

use super::uat::lab_account;
use super::LabServer;
use crate::supervisor::now_ms;
use crate::uat::evidence::default_root;
use crate::uat::invoke::{ServerInvoker, ServerTools};
use crate::uat::record::log::TICK_MS;
use crate::uat::record::sampler::SamplerConfig;
use crate::uat::specs_dir;
use run::{refused, Failure, Recorder, StartReq};
use stop::{StopReq, STOP_WAIT};

/// Args for `lab_record_start`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct RecordStartArgs {
    /// The section being played (`first-session`, `^[a-z0-9][a-z0-9-]{0,47}$`);
    /// it names the draft and, when the spec directory holds it, its
    /// `[section]` is copied.
    pub section: String,
    /// The character being played (default: the instance's lab-account
    /// character).
    #[serde(default)]
    pub character: Option<String>,
    /// Spec directory the stop writes `drafts/` under: an existing local
    /// directory, not a UNC or `\\?\` path (default: CIMMERIA_LAB_UAT_SPECS,
    /// else the repo's docs/guides/uat-specs).
    #[serde(default)]
    pub specs_dir: Option<String>,
    /// Stop on its own after this many minutes (default 30, at most 30).
    #[serde(default)]
    pub max_minutes: Option<u32>,
}

/// Args for `lab_record_stop`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct RecordStopArgs {
    /// The id `lab_record_start` returned; any other id is refused and the
    /// recording keeps running.
    pub recording_id: String,
    /// Replaces the recorded character's name in the draft's SQL (default
    /// `${character}`; `Px${run_id}` for a fresh character per run;
    /// `^[A-Za-z0-9 _${}]{1,64}$`).
    #[serde(default)]
    pub character_expr: Option<String>,
    /// The fixture profile (LR-09).
    #[serde(default)]
    pub profile: Option<String>,
    /// Stop and keep the raw log, but write no draft.
    #[serde(default)]
    pub no_draft: bool,
}

/// Args for `lab_record_transform`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct RecordTransformArgs {
    /// The recording: its id (`20261012-143000-p2`) or its folder, which
    /// must be directly under `<uat root>/records/`.
    pub dir: String,
    /// Draft it as this section instead of the recorded one
    /// (`^[a-z0-9][a-z0-9-]{0,47}$`).
    #[serde(default)]
    pub section: Option<String>,
    /// As in `lab_record_stop`.
    #[serde(default)]
    pub character_expr: Option<String>,
    /// The fixture profile (LR-09).
    #[serde(default)]
    pub profile: Option<String>,
    /// Spec directory the draft goes under (`drafts/`): an existing local
    /// directory (default: CIMMERIA_LAB_UAT_SPECS, else the repo's
    /// docs/guides/uat-specs).
    #[serde(default)]
    pub specs_dir: Option<String>,
}

/// The answer: compact JSON, a tool error for a failure.
fn answer(out: Result<Value, Failure>) -> CallToolResult {
    match out {
        Ok(v) => CallToolResult::success(vec![ContentBlock::text(v.to_string())]),
        Err(f) => CallToolResult::error(vec![ContentBlock::text(f.body())]),
    }
}

/// The failure shape for a refusal made before the recorder is reached.
fn refusal(error: impl Into<String>) -> CallToolResult {
    answer(Err(refused(error)))
}

#[tool_router(router = record_router, vis = "pub(super)")]
impl LabServer {
    #[tool(
        description = "Start recording a person playing this lab instance by hand (lab-record): every 500 ms read the player's packet tap, server entity, camera readout, visible windows and dialog, and a DB snapshot after server activity, into <uat root>/records/<recording_id>/record.jsonl. Only reads: it never drives input, camera or focus. Takes a lease of its own (owner lab_record, purpose `recording a human: <section>`) and holds it until lab_record_stop, so no agent drives the client meanwhile; refused while anyone holds the lab or another recording runs on the instance. Stops by itself after max_minutes (default and at most 30) or when its lease is taken over (lease_lost). Do not call lab_timeline on this instance while recording: the recorder owns the player's tap. Returns {ok, recording_id, instance, section}; the folder is <uat root>/records/<recording_id>/."
    )]
    async fn lab_record_start(
        &self,
        Parameters(a): Parameters<RecordStartArgs>,
    ) -> Result<CallToolResult, McpError> {
        let Some(server) = ServerTools::from_env() else {
            return Ok(refusal(
                "no server lab MCP: set CIMMERIA_LAB_MCP_URL and CIMMERIA_LAB_MCP_TOKEN",
            ));
        };
        let character =
            match a
                .character
                .or_else(|| lab_account(self.supervisor.instance()).1)
            {
                Some(c) => c,
                None => return Ok(refusal(
                    "no character: pass character, or name one in the instance's lab-account file",
                )),
            };
        let rec = Recorder {
            slot: self.supervisor.record_slot().clone(),
            book: self.supervisor.leases().clone(),
            server: Arc::new(server) as Arc<dyn ServerInvoker>,
            client: self.supervisor.clone(),
            instance: self.supervisor.label().to_string(),
            records: default_root().join("records"),
            clock: Arc::new(now_ms),
            tick: Duration::from_millis(TICK_MS.unsigned_abs()),
            cfg: SamplerConfig::default(),
        };
        let req = StartReq {
            section: a.section,
            character,
            specs_dir: a.specs_dir.map(PathBuf::from),
            max_minutes: a.max_minutes,
        };
        Ok(answer(run::start(rec, req).await))
    }

    #[tool(
        description = "This instance's recording: {ok, recording, recording_id, instance, section, age_s, ticks, anchors}. recording is false once the recorder has stopped by itself (max_minutes, lease_lost) and the recording waits for lab_record_stop to collect it; anchors counts the interact, useItem, moveItem and useAbility calls seen so far."
    )]
    async fn lab_record_status(&self) -> Result<CallToolResult, McpError> {
        let v = stop::status(
            self.supervisor.record_slot(),
            self.supervisor.label(),
            now_ms(),
        );
        Ok(answer(Ok(v)))
    }

    #[tool(
        description = "Stop this instance's recording and write its draft spec. Gated by recording_id: a wrong id is refused and the recording keeps running. Waits (at most 60 s) for the recorder to write its stop line and release its lease, transforms the raw log (one row per interaction, `# TODO` for anything outside the mapping table), and writes <specs_dir>/drafts/<section>-<yyyyMMdd-HHmm>.toml, never over an existing file. The raw log is always kept. Returns {ok, draft (file name), rows, steps, todo, dropped, raw (recording id), ended?}; ended is why the recorder stopped early with an error. A failure is {ok:false, exit, error} naming the raw folder first (a failed transform's whole error is in transform-error.txt there); the recording then stays, so call lab_record_stop again, with other options if they were the problem. no_draft stops and keeps the log only."
    )]
    async fn lab_record_stop(
        &self,
        Parameters(a): Parameters<RecordStopArgs>,
    ) -> Result<CallToolResult, McpError> {
        let req = StopReq {
            recording_id: a.recording_id,
            character_expr: a.character_expr,
            profile: a.profile,
            no_draft: a.no_draft,
            specs_dir: specs_dir(),
            instance: self.supervisor.label().to_string(),
            wait: STOP_WAIT,
            transform: crate::uat::record::emit::transform,
        };
        Ok(answer(stop::stop(self.supervisor.record_slot(), req).await))
    }

    #[tool(
        description = "Transform a recording already on disk into a draft spec again, with other options: no client, no lease. dir is the recording id or its folder, directly under <uat root>/records/ (anything else is refused). section drafts it as another section. A recording without its stop line is refused: stop it first. Writes <specs_dir>/drafts/<section>-<yyyyMMdd-HHmm>.toml, or <name>-2.toml, -3, ... when that exists, never over an existing file. Returns lab_record_stop's answer {ok, draft (file name), rows, steps, todo, dropped, raw (recording id)}; a failure is {ok:false, exit, error}."
    )]
    async fn lab_record_transform(
        &self,
        Parameters(a): Parameters<RecordTransformArgs>,
    ) -> Result<CallToolResult, McpError> {
        let req = transform::TransformReq {
            dir: a.dir,
            section: a.section,
            character_expr: a.character_expr,
            profile: a.profile,
            specs_dir: a.specs_dir.map(PathBuf::from),
            default_specs: specs_dir(),
            transform: crate::uat::record::emit::transform,
        };
        let records = default_root().join("records");
        // File reads and a transform: off the async workers.
        let out = tokio::task::spawn_blocking(move || transform::transform_dir(&records, req))
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(target: "lab.record", error = %e,
                    "recording transform task panicked or was cancelled");
                Err(run::failed(format!("transform task: {e}")))
            });
        Ok(answer(out))
    }
}

#[cfg(test)]
mod fakes;
#[cfg(test)]
mod stop_tests;
#[cfg(test)]
mod tests;
