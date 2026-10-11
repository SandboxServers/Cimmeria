//! Chaos around every row (lab-chaos CH-08). `cpu` loads the machine for
//! the whole run through a [`CpuGuard`] held by `run_all`. `net` and
//! `flush` are one `server_chaos_set`, scoped to the row's player entity,
//! sent once the row is in world and before its setup, and one
//! `server_chaos_clear` after its teardown.
//!
//! Cleanup on every path: the clear runs after teardown whether the steps
//! passed, failed or setup failed, and before a fixture session logs out;
//! `run_row` clears when a lease takeover cuts the row off; and a row
//! dropped with chaos still set (a panic) logs a WARN and sends a
//! best-effort clear from its [`ServerChaosGuard`]'s `Drop`. Last of all,
//! the server's own TTL (the row's budget plus a minute) ends it.

use serde_json::json;

use super::packet::ENTITY_VAR;
use super::{RowCtx, Runner};
use crate::chaos::cpu::CpuGuard;
use crate::chaos::run::{seed_of, RunChaos};
use crate::uat::evidence::RunManifest;
use crate::uat::invoke::{ServerTools, ToolInvoker, ToolOutcome};
use crate::uat::spec::RowSpec;
use crate::uat::tier::Role;

/// A row's budget before its own waits: reaching its state, the anchor and
/// the actions themselves.
const ROW_BASE_S: u64 = 300;
/// Added to the row's budget, so the server's TTL outlives the row's own
/// clear (D-CH3: the TTL is the backstop, not the switch).
const TTL_MARGIN_S: u64 = 60;
/// The server refuses a TTL over this (lab-chaos `MAX_TTL_S`).
const MAX_TTL_S: u64 = 1800;

/// The chaos one row set on the server, until it is cleared.
#[derive(Debug, Default)]
pub(crate) struct RowChaos {
    guard: Option<ServerChaosGuard>,
    /// A clear that failed twice; a flag on the row, since the next row may
    /// meet the chaos until the server's TTL ends it.
    pub(crate) errors: Vec<String>,
}

/// Armed while a `server_chaos_set` may be live for `entity`.
#[derive(Debug)]
struct ServerChaosGuard {
    entity: u32,
    ttl_s: u32,
    armed: bool,
}

impl Drop for ServerChaosGuard {
    /// Only reached armed when the row unwound without its clear (a panic):
    /// `Drop` cannot await, so the clear is spawned on the runtime through a
    /// fresh lab-MCP client, and the TTL covers a clear that never lands.
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let entity = self.entity;
        tracing::warn!(target: "lab.chaos", entity_id = entity, ttl_s = self.ttl_s,
            "UAT row ended with server chaos still set; sending a best-effort clear (the server's ttl ends it otherwise)");
        if let (Ok(rt), Some(server)) = (
            tokio::runtime::Handle::try_current(),
            ServerTools::from_env(),
        ) {
            rt.spawn(async move {
                let _ = server
                    .call("server_chaos_clear", json!({ "entity_id": entity }))
                    .await;
            });
        }
    }
}

/// Why a chaos call did not take: a failed call, or the tool's `{refused}`.
fn refusal(out: &ToolOutcome) -> Option<String> {
    if !out.ok {
        return Some(out.error.clone().unwrap_or_default());
    }
    out.json
        .get("refused")
        .map(|r| r.as_str().map_or_else(|| r.to_string(), str::to_string))
}

/// The run's chaos, resolved with the run's seed, and its header written
/// into `run.json` (an appended run keeps an earlier header unless this
/// call has chaos of its own).
pub(super) fn resolve(texts: &[String], manifest: &mut RunManifest) -> Result<RunChaos, String> {
    let chaos = RunChaos::parse(texts, seed_of(&manifest.run_id))?;
    if let Some(line) = chaos.header_line() {
        manifest.chaos = Some(line);
    }
    Ok(chaos)
}

/// The server TTL for a row: its budget (a base plus every wait and clause
/// timeout it names) plus a minute. A row whose budget is already over the
/// server's cap is refused rather than clamped, so chaos never ends mid-row.
pub(super) fn row_ttl_s(row: &RowSpec) -> Result<u32, String> {
    let action_ms: u64 = row
        .setup
        .iter()
        .chain(&row.steps)
        .chain(&row.teardown)
        .map(|a| a.wait_ms.unwrap_or(0) + a.wait_server.as_ref().map_or(0, |w| w.timeout_ms))
        .sum();
    let clause_ms: u64 = row.expect.iter().filter_map(|c| c.timeout_ms).sum();
    let budget = ROW_BASE_S + (action_ms + clause_ms).div_ceil(1000);
    let ttl = budget + TTL_MARGIN_S;
    if ttl > MAX_TTL_S {
        return Err(format!(
            "chaos: the row's budget ({budget} s) plus {TTL_MARGIN_S} s is over the server's \
             {MAX_TTL_S} s chaos ttl; shorten its waits or run it without net/flush chaos"
        ));
    }
    u32::try_from(ttl).map_err(|_| "chaos: ttl out of range".to_string())
}

impl<I: ToolInvoker> Runner<'_, I> {
    /// Static reasons a row cannot run under this chaos: the server profiles
    /// need the lab MCP, a player entity to scope to, and a TTL under cap.
    pub(super) fn chaos_blocks(&self, row: &RowSpec) -> Vec<String> {
        if !self.chaos.has_server_profile() {
            return vec![];
        }
        let mut blocks = Vec::new();
        if self.server.is_none() {
            blocks.push(
                "chaos net/flush needs the server lab MCP (CIMMERIA_LAB_MCP_URL/_TOKEN)".into(),
            );
        }
        if row.state != "in_world" {
            blocks.push(format!(
                "chaos net/flush is scoped to the row's player entity; a {} row has none",
                row.state
            ));
        }
        if let Err(e) = row_ttl_s(row) {
            blocks.push(e);
        }
        blocks
    }

    /// The run's cpu load, started once and stopped when the guard drops
    /// (`run_all` holds it, so every return path stops it). None for a plan.
    pub(super) fn chaos_cpu_start(&self) -> Option<CpuGuard> {
        if self.req.plan_only {
            return None;
        }
        match self.chaos.cpu_stress() {
            Ok(Some(stress)) => {
                let guard = stress.start();
                tracing::info!(target: "lab.chaos", threads = guard.running(),
                    duty_percent = stress.duty_percent, "UAT run: cpu chaos started");
                Some(guard)
            }
            Ok(None) => None,
            // `RunChaos::parse` already refused bad cpu knobs.
            Err(e) => {
                tracing::warn!(target: "lab.chaos", error = %e, "UAT run: cpu chaos not started");
                None
            }
        }
    }

    /// Set the row's `net`/`flush` chaos on its player entity. An error
    /// blocks the row: a row must not grade as chaos-tested without it.
    pub(super) async fn chaos_set(
        &mut self,
        row: &RowSpec,
        ctx: &mut RowCtx,
    ) -> Result<(), String> {
        let Some(server) = self.server else {
            // No server profile, or the static check already blocked the row.
            return Ok(());
        };
        if !self.chaos.has_server_profile() {
            return Ok(());
        }
        let ttl_s = row_ttl_s(row)?;
        let entity = self.tap_entity(server, ctx).await?;
        ctx.vars.insert(ENTITY_VAR.into(), json!(entity));
        let Some(args) = self.chaos.server_set_args(entity, ttl_s)? else {
            return Ok(());
        };
        let out = self
            .server_call(server, Role::Setup, "server_chaos_set", args, ctx)
            .await;
        let refused = out.ok && out.json.get("refused").is_some();
        // A failed call may still have reached the server, so only a
        // `{refused}` answer leaves nothing to clear.
        if !refused {
            ctx.chaos.guard = Some(ServerChaosGuard {
                entity,
                ttl_s,
                armed: true,
            });
        }
        match refusal(&out) {
            Some(why) => Err(format!("server_chaos_set: {why}")),
            None => Ok(()),
        }
    }

    /// Clear the row's chaos, once with one retry. A no-op when the row set
    /// none or already cleared it.
    pub(crate) async fn chaos_clear(&mut self, ctx: &mut RowCtx) {
        let Some(mut guard) = ctx.chaos.guard.take() else {
            return;
        };
        let Some(server) = self.server else {
            guard.armed = false;
            return;
        };
        let args = json!({ "entity_id": guard.entity });
        let mut why = None;
        for _ in 0..2 {
            let out = self
                .server_call(
                    server,
                    Role::Teardown,
                    "server_chaos_clear",
                    args.clone(),
                    ctx,
                )
                .await;
            why = refusal(&out);
            if why.is_none() {
                break;
            }
        }
        guard.armed = false;
        if let Some(why) = why {
            ctx.chaos.errors.push(format!(
                "chaos clear failed twice ({why}); the server ends it within {} s",
                guard.ttl_s
            ));
        }
    }
}
