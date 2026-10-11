//! Chaos around every row (lab-chaos CH-08). `cpu` loads the machine for
//! the whole run through a [`CpuGuard`] held by `run_all`. `net` and
//! `flush` are one `server_chaos_set`, scoped to the row's player entity,
//! sent once the row is in world and before its setup, and one
//! `server_chaos_clear` after its teardown.
//!
//! Before the first row the run probes `server_chaos_status`: a server
//! without lab chaos blocks every net/flush row, so a set that fails later
//! is a transport failure on a server that has the tool, not a missing tool.
//!
//! Cleanup on every path: the guard is armed before the set is sent, so a
//! set cut off mid-call is cleared too. The clear runs after teardown
//! whether the steps passed, failed or setup failed, and before a fixture
//! session logs out; `run_row` clears when a lease takeover cuts the row
//! off; and a row dropped with chaos still armed (a panic) logs a WARN and
//! sends a best-effort clear from its [`ServerChaosGuard`]'s `Drop`. Last of
//! all, the server's own TTL (the row's budget plus a minute) ends it. A
//! clear that fails twice stops the run: every later row is BLOCKED, since
//! chaos left live would run under it.

use serde_json::{json, Value};

use super::packet::ENTITY_VAR;
use super::{RowCtx, Runner};
use crate::chaos::cpu::CpuGuard;
use crate::chaos::run::{seed_of, RunChaos};
use crate::uat::evidence::RunManifest;
use crate::uat::invoke::{ServerInvoker, ServerTools, ToolInvoker, ToolOutcome};
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
/// The row flag when the clear found nothing to clear.
pub(super) const ENDED_EARLY: &str = "chaos had already ended before row end (cleared by a \
     return to character select, the session's teardown or its ttl)";
/// Why a login row cannot take net/flush chaos yet.
pub(super) const LOGIN_ROW: &str = "chaos net/flush on a login row needs arming by account (CH-15)";

/// The chaos one row set on the server, until it is cleared.
#[derive(Debug, Default)]
pub(crate) struct RowChaos {
    guard: Option<ServerChaosGuard>,
    /// Row flags: the chaos ended before the row did, or its clear failed.
    pub(crate) errors: Vec<String>,
}

/// The run's chaos state across rows.
#[derive(Debug, Default)]
pub(crate) struct RunChaosState {
    /// Why net/flush cannot be set on this server (the status probe).
    unavailable: Option<String>,
    /// Set when a clear failed twice: every later row is BLOCKED with it.
    stuck: Option<String>,
}

/// What the runner knows about a row's `server_chaos_set`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SetState {
    /// Sent, no answer yet: a cancel or panic now may leave chaos set.
    Sending,
    /// The call failed in transport, so it may or may not have landed.
    Unconfirmed(String),
    /// The server answered with no refusal.
    Set,
}

/// Builds the lab-MCP client a `Drop` sends its clear through.
type DropServer = fn() -> Option<Box<dyn ServerInvoker>>;

fn env_server() -> Option<Box<dyn ServerInvoker>> {
    ServerTools::from_env().map(|s| Box::new(s) as Box<dyn ServerInvoker>)
}

/// Armed from just before `server_chaos_set` is sent until the clear ran.
#[derive(Debug)]
struct ServerChaosGuard {
    entity: u32,
    ttl_s: u32,
    state: SetState,
    armed: bool,
    drop_server: DropServer,
}

impl ServerChaosGuard {
    fn sending(entity: u32, ttl_s: u32) -> Self {
        Self {
            entity,
            ttl_s,
            state: SetState::Sending,
            armed: true,
            drop_server: env_server,
        }
    }
}

impl Drop for ServerChaosGuard {
    /// Only reached armed when the row unwound without its clear (a panic):
    /// `Drop` cannot await, so the clear is spawned on the runtime through a
    /// fresh lab-MCP client, and the TTL covers a clear that never lands.
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let (entity, ttl_s) = (self.entity, self.ttl_s);
        let Ok(rt) = tokio::runtime::Handle::try_current() else {
            tracing::warn!(target: "lab.chaos", entity_id = entity, ttl_s,
                "UAT row ended with server chaos still set and no async runtime to clear it from; the server's ttl ends it");
            return;
        };
        let Some(server) = (self.drop_server)() else {
            tracing::warn!(target: "lab.chaos", entity_id = entity, ttl_s,
                "UAT row ended with server chaos still set and no lab MCP (CIMMERIA_LAB_MCP_URL/_TOKEN) to clear it; the server's ttl ends it");
            return;
        };
        tracing::warn!(target: "lab.chaos", entity_id = entity, ttl_s,
            "UAT row ended with server chaos still set; sending a best-effort clear (the server's ttl ends it otherwise)");
        rt.spawn(async move {
            let out = server
                .call("server_chaos_clear", json!({ "entity_id": entity }))
                .await;
            let cleared = out.json.get("cleared").and_then(Value::as_u64);
            match refusal(&out) {
                None => tracing::info!(target: "lab.chaos", entity_id = entity, ?cleared,
                    "best-effort chaos clear sent"),
                Some(why) => tracing::warn!(target: "lab.chaos", entity_id = entity, ttl_s,
                    error = %why, "best-effort chaos clear failed; the server's ttl ends it"),
            }
        });
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
/// call has chaos of its own). A plan's header leaves an unnamed seed to
/// the real run.
pub(super) fn resolve(
    texts: &[String],
    plan_only: bool,
    manifest: &mut RunManifest,
) -> Result<RunChaos, String> {
    let chaos = RunChaos::parse(texts, seed_of(&manifest.run_id))?;
    let line = if plan_only {
        chaos.plan_header_line()
    } else {
        chaos.header_line()
    };
    if let Some(line) = line {
        manifest.chaos = Some(line);
    }
    Ok(chaos)
}

/// The server TTL for a row: its budget (a base plus every wait and clause
/// timeout it names) plus a minute. A row whose budget is already over the
/// server's cap is refused rather than clamped, so chaos never ends mid-row.
pub(super) fn row_ttl_s(row: &RowSpec) -> Result<u32, String> {
    let action_ms = row
        .setup
        .iter()
        .chain(&row.steps)
        .chain(&row.teardown)
        .map(|a| {
            a.wait_ms
                .unwrap_or(0)
                .saturating_add(a.wait_server.as_ref().map_or(0, |w| w.timeout_ms))
        })
        .fold(0u64, u64::saturating_add);
    let clause_ms = row
        .expect
        .iter()
        .filter_map(|c| c.timeout_ms)
        .fold(0u64, u64::saturating_add);
    let budget = ROW_BASE_S.saturating_add(action_ms.saturating_add(clause_ms).div_ceil(1000));
    let ttl = budget.saturating_add(TTL_MARGIN_S);
    if ttl > MAX_TTL_S {
        return Err(format!(
            "chaos: the row's budget ({budget} s) plus {TTL_MARGIN_S} s is over the server's \
             {MAX_TTL_S} s chaos ttl; shorten its waits or run it without net/flush chaos"
        ));
    }
    u32::try_from(ttl).map_err(|_| "chaos: ttl out of range".to_string())
}

impl<I: ToolInvoker> Runner<'_, I> {
    /// Reasons a row cannot run under this chaos, known before it starts:
    /// an earlier clear that failed, no lab MCP or no lab chaos on the
    /// server, a login row (no player entity), a TTL over cap. An `any` row
    /// is checked when its chaos is set ([`Self::chaos_set`]).
    pub(super) fn chaos_blocks(&self, row: &RowSpec) -> Vec<String> {
        if !self.chaos.has_server_profile() {
            return vec![];
        }
        let mut blocks = Vec::new();
        blocks.extend(self.chaos_run.stuck.clone());
        if self.server.is_none() {
            blocks.push(
                "chaos net/flush needs the server lab MCP (CIMMERIA_LAB_MCP_URL/_TOKEN)".into(),
            );
        }
        blocks.extend(self.chaos_run.unavailable.clone());
        if matches!(row.state.as_str(), "client_stopped" | "char_select") {
            blocks.push(format!(
                "{LOGIN_ROW}: a {} row starts with no player entity to scope it to",
                row.state
            ));
        }
        if let Err(e) = row_ttl_s(row) {
            blocks.push(e);
        }
        blocks
    }

    /// Before the first row: whether the server has lab chaos at all. A
    /// failed or refused `server_chaos_status` blocks every net/flush row,
    /// plan included, so the plan exits 3 instead of the run finding out.
    pub(super) async fn chaos_probe(&mut self) {
        let (Some(server), true) = (self.server, self.chaos.has_server_profile()) else {
            return;
        };
        let out = server.call("server_chaos_status", json!({})).await;
        if let Some(why) = refusal(&out) {
            self.chaos_run.unavailable = Some(format!(
                "chaos net/flush needs a server built with lab-chaos and DEVELOPER_MODE on; \
                 server_chaos_status: {why}"
            ));
        }
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
        let entity = self.tap_entity(server, ctx).await.map_err(|e| {
            if row.state == "in_world" {
                e
            } else {
                format!(
                    "chaos net/flush is scoped to the player entity, and this {} row had no \
                     player in world when its chaos was to be set ({e}); start it in_world",
                    row.state
                )
            }
        })?;
        if entity == 0 {
            return Err("entity id 0 names no player; chaos not set".into());
        }
        ctx.vars.insert(ENTITY_VAR.into(), json!(entity));
        let Some(args) = self.chaos.server_set_args(entity, ttl_s)? else {
            return Ok(());
        };
        // Armed before the call: a takeover or panic mid-call leaves a
        // guard behind that clears, since the set may already have landed.
        ctx.chaos.guard = Some(ServerChaosGuard::sending(entity, ttl_s));
        let out = self
            .server_call(server, Role::Setup, "server_chaos_set", args, ctx)
            .await;
        let why = refusal(&out);
        match (&why, out.ok) {
            // A failed call may still have reached the server: clear it.
            (Some(e), false) => {
                if let Some(guard) = ctx.chaos.guard.as_mut() {
                    guard.state = SetState::Unconfirmed(e.clone());
                }
            }
            // `{refused}`: nothing was set, so nothing to clear.
            (Some(_), true) => {
                if let Some(mut guard) = ctx.chaos.guard.take() {
                    guard.armed = false;
                }
            }
            (None, _) => {
                if let Some(guard) = ctx.chaos.guard.as_mut() {
                    guard.state = SetState::Set;
                }
            }
        }
        match why {
            Some(why) => Err(format!("server_chaos_set: {why}")),
            None => Ok(()),
        }
    }

    /// Clear the row's chaos, once with one retry. A no-op when the row set
    /// none or already cleared it. A clear that fails twice flags the row
    /// and stops the run ([`RunChaosState::stuck`]).
    pub(crate) async fn chaos_clear(&mut self, ctx: &mut RowCtx) {
        let Some(mut guard) = ctx.chaos.guard.take() else {
            return;
        };
        let Some(server) = self.server else {
            guard.armed = false;
            return;
        };
        let args = json!({ "entity_id": guard.entity });
        let mut result = Err(String::new());
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
            result = match refusal(&out) {
                Some(why) => Err(why),
                None => Ok(out.json),
            };
            if result.is_ok() {
                break;
            }
        }
        guard.armed = false;
        match result {
            Ok(reply) => {
                let cleared = reply.get("cleared").and_then(Value::as_u64);
                if guard.state == SetState::Set && cleared == Some(0) {
                    ctx.chaos.errors.push(ENDED_EARLY.into());
                }
            }
            Err(why) => {
                let what = match &guard.state {
                    SetState::Set => format!("chaos clear failed twice ({why})"),
                    SetState::Unconfirmed(e) => format!(
                        "chaos set was never confirmed ({e}) and its clear failed twice ({why})"
                    ),
                    SetState::Sending => {
                        format!("chaos set was cut off mid-call and its clear failed twice ({why})")
                    }
                };
                ctx.chaos.errors.push(format!(
                    "{what}; chaos may still be live on entity {} until the server's ttl ({} s)",
                    guard.entity, guard.ttl_s
                ));
                self.chaos_run.stuck = Some(format!(
                    "chaos may still be live on entity {}; run stopped",
                    guard.entity
                ));
            }
        }
    }
}

#[cfg(test)]
mod drop_tests {
    //! The `Drop` path, with the lab-MCP client swapped for a recorder.

    use std::sync::Mutex;

    use super::*;
    use crate::uat::invoke::BoxedOutcome;

    /// Every clear a dropped guard sent, by entity id (shared: tests run
    /// in parallel under `cargo test`, so each uses its own entity).
    static CLEARS: Mutex<Vec<Value>> = Mutex::new(Vec::new());

    struct Recorder;

    impl ServerInvoker for Recorder {
        fn url(&self) -> &str {
            "recorder"
        }

        fn call<'a>(&'a self, name: &'a str, args: Value) -> BoxedOutcome<'a> {
            if name == "server_chaos_clear" {
                CLEARS.lock().unwrap().push(args["entity_id"].clone());
            }
            Box::pin(async {
                ToolOutcome {
                    ok: true,
                    json: json!({ "cleared": 1 }),
                    ..Default::default()
                }
            })
        }
    }

    fn recorder() -> Option<Box<dyn ServerInvoker>> {
        Some(Box::new(Recorder))
    }

    fn guard(entity: u32) -> ServerChaosGuard {
        let mut g = ServerChaosGuard::sending(entity, 360);
        g.drop_server = recorder;
        g
    }

    async fn cleared(entity: u32) -> bool {
        for _ in 0..50 {
            if CLEARS.lock().unwrap().contains(&json!(entity)) {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        false
    }

    /// Regression guard: a row unwinding with its guard armed (a panic)
    /// sends a clear for its entity; a disarmed guard sends none.
    #[tokio::test]
    async fn a_panic_with_chaos_armed_sends_a_clear() {
        let g = guard(90_001);
        let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _held = g;
            panic!("row panicked mid-drive");
        }));
        assert!(unwound.is_err());
        assert!(cleared(90_001).await, "no clear after the panic");

        let mut quiet = guard(90_002);
        quiet.armed = false;
        drop(quiet);
        assert!(!cleared(90_002).await, "a disarmed guard sent a clear");
    }
}
