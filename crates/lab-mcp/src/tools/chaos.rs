//! `server_chaos_set`, `server_chaos_clear` and `server_chaos_status`
//! (lab-chaos CH-05). Built only with the `lab-chaos` feature, so an ordinary
//! server binary does not list them.
//!
//! The server's registry (`base::lab_chaos`) is the authority: these tools
//! translate arguments ([`super::chaos_args`]), resolve a player entity to
//! its session address and report the registry's answer. Bounds are the
//! registry's to check; nothing here clamps. Every refusal, including "not
//! installed" and a malformed call, is a successful call whose reply is
//! `{refused: reason}`, so a lab run can read why, and every call writes one
//! audit row. Status names entity ids only, never an address.

use std::future::Future;
use std::net::SocketAddr;
use std::time::Instant;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router, ErrorData};
use serde_json::{json, Value};

use cimmeria_services::base::lab_chaos::ChaosRefused;
use cimmeria_services::base::lab_chaos::FlushChaos;
use cimmeria_services::base::lab_chaos::LabChaos;
use cimmeria_services::base::lab_chaos::NetChaos;
use cimmeria_services::base::lab_chaos::SessionChaos;
use cimmeria_services::base::lab_chaos::DEFAULT_TTL_S;
use cimmeria_services::base::OnlinePlayer;

use super::chaos_args::{
    audit_args, built_label, check_label, check_seed_used, flush_from, net_from, parse, ClearArgs,
    ClearCall, SetArgs, SetCall,
};
use super::{ok, LabTools};
use crate::audit::{self, OUTCOME_ERROR, OUTCOME_OK};

/// The most sessions `server_chaos_status` lists.
const STATUS_CAP: usize = 10;

/// The refusal every chaos tool gives when the server did not install the
/// registry (`DEVELOPER_MODE` off).
pub(super) const OFF: &str = "lab chaos is off (DEVELOPER_MODE)";

#[tool_router(router = chaos_router, vis = "pub(super)")]
impl LabTools {
    /// Arm chaos on one player's session.
    #[tool(
        description = "Lab-chaos builds only: arm per-session fault injection on a player's session (entity_id). Give a net profile {latency, jitter (ms), loss, dup (percent, at most one decimal), reorder (packets)} and/or a flush profile {mode: delay|reorder|pack|shift, delay_ms, records, shift (bytes or \"sweep\")}, a seed (default 0), ttl_s (default 300, at most 1800) and an optional label. Unknown fields, an empty net profile, a knob the flush mode ignores and an unused seed are refused. Replaces the session's chaos. Returns {set: true, expires_in_s} or {refused: reason}."
    )]
    pub(super) async fn server_chaos_set(
        &self,
        Parameters(SetCall(raw)): Parameters<SetCall>,
    ) -> Result<CallToolResult, ErrorData> {
        let started = Instant::now();
        let audit_args = audit_args(&raw);
        // The parse comes first, so a malformed call says what is wrong with
        // it even on a server where chaos is off.
        let result = match parse::<SetArgs>(raw).and_then(|args| {
            Ok((
                args,
                installed(cimmeria_services::base::lab_chaos::current())?,
            ))
        }) {
            Ok((args, registry)) => set_on(registry, args, || self.state.online_players()).await,
            Err(e) => Err(e),
        };
        refusal_result("server_chaos_set", &audit_args, started, result)
    }

    /// Clear chaos on one player's session, or on every session.
    #[tool(
        description = "Lab-chaos builds only: clear the chaos set for one player entity (entity_id), or on every session when entity_id is omitted. Returns {cleared: n} or {refused: reason}."
    )]
    pub(super) async fn server_chaos_clear(
        &self,
        Parameters(ClearCall(raw)): Parameters<ClearCall>,
    ) -> Result<CallToolResult, ErrorData> {
        let started = Instant::now();
        let audit_args = audit_args(&raw);
        let result = parse::<ClearArgs>(raw).and_then(|args| {
            let registry = installed(cimmeria_services::base::lab_chaos::current())?;
            clear_on(registry, args.entity_id)
        });
        refusal_result("server_chaos_clear", &audit_args, started, result)
    }

    /// List the sessions under live chaos.
    #[tool(
        description = "Lab-chaos builds only: list the sessions under live chaos by player entity id, with their net and flush knobs and expires_in_s, at most 10 (with more: n when there are others). Returns {sessions: [...]} or {refused: reason}."
    )]
    pub(super) async fn server_chaos_status(&self) -> Result<CallToolResult, ErrorData> {
        let started = Instant::now();
        let result = installed(cimmeria_services::base::lab_chaos::current())
            .map(|registry| status_on(registry, Instant::now()));
        refusal_result("server_chaos_status", &json!({}), started, result)
    }
}

/// Emits the one audit event and wraps the result. A refusal is still a
/// successful call: the reply is `{refused: reason}` (the CH-05 contract),
/// and the audit row carries the reason as its error.
fn refusal_result(
    tool: &str,
    audit_args: &Value,
    started: Instant,
    result: Result<Value, String>,
) -> Result<CallToolResult, ErrorData> {
    match result {
        Ok(v) => {
            audit::emit(tool, audit_args, OUTCOME_OK, None, started);
            ok(v)
        }
        Err(reason) => {
            audit::emit(
                tool,
                audit_args,
                OUTCOME_ERROR,
                Some(reason.as_str()),
                started,
            );
            ok(json!({ "refused": reason }))
        }
    }
}

/// The registry, or the refusal every chaos tool gives when it is not installed.
pub(super) fn installed(registry: Option<&LabChaos>) -> Result<&LabChaos, String> {
    registry.ok_or_else(|| OFF.to_string())
}

/// Arms the request on the entity's session, then checks the entity still
/// owns that address. `roster` is read twice: once to resolve the address,
/// once after `set`, because the session can end and another can take the
/// address in between (the CH-04 review's race).
pub(super) async fn set_on<F, Fut>(
    registry: &LabChaos,
    args: SetArgs,
    roster: F,
) -> Result<Value, String>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Vec<OnlinePlayer>>,
{
    // The roster lists a session with no player entity as id 0, so 0 never
    // names a player.
    if args.entity_id == 0 {
        return Err(no_session(0));
    }
    let net = args
        .net
        .as_ref()
        .map(|n| net_from(n, args.seed))
        .transpose()?;
    let flush = args
        .flush
        .as_ref()
        .map(|f| flush_from(f, args.seed))
        .transpose()?;
    check_seed_used(args.seed, net.as_ref(), flush.as_ref())?;
    let label = match args.label {
        Some(label) => {
            check_label(&label)?;
            label
        }
        None => built_label(net.as_ref(), flush.as_ref()),
    };
    // Only an absent TTL gets the default: a named one, 0 included, goes to
    // the registry unchanged and is refused there if out of range.
    let ttl_s = args.ttl_s.unwrap_or(DEFAULT_TTL_S);

    let addr =
        session_addr(&roster().await, args.entity_id).ok_or_else(|| no_session(args.entity_id))?;
    let now = Instant::now();
    let expires_at = registry
        .set(addr, args.entity_id, label, net, flush, ttl_s, now)
        .map_err(|e| e.to_string())?;

    let after = roster().await;
    if session_addr(&after, args.entity_id) != Some(addr) {
        registry.clear_if_not(addr, entity_at(&after, addr));
        return Err(no_session(args.entity_id));
    }
    Ok(json!({ "set": true, "expires_in_s": secs_until(expires_at, now) }))
}

/// Clears the chaos set for one entity, or every session with `None`. By the
/// entry's entity tag, not the live roster: after a return to character
/// select the entity has no session, but status still lists it.
pub(super) fn clear_on(registry: &LabChaos, entity_id: Option<u32>) -> Result<Value, String> {
    let cleared = match entity_id {
        None => registry.clear(None),
        Some(0) => return Err(no_session(0)),
        Some(id) => registry.clear_entity(id),
    };
    Ok(json!({ "cleared": cleared }))
}

/// The live sessions by entity id, at most [`STATUS_CAP`], and how many more.
pub(super) fn status_on(registry: &LabChaos, now: Instant) -> Value {
    let live = registry.status(now);
    let rows: Vec<Value> = live
        .iter()
        .take(STATUS_CAP)
        .map(|s| status_row(s, now))
        .collect();
    let mut reply = json!({ "sessions": rows });
    if live.len() > STATUS_CAP {
        reply["more"] = json!(live.len() - STATUS_CAP);
    }
    reply
}

fn status_row(s: &SessionChaos, now: Instant) -> Value {
    let mut row = json!({
        "entity_id": s.entity_id,
        "expires_in_s": secs_until(s.expires_at, now),
    });
    if let Some(net) = &s.net {
        row["net"] = net_json(net);
    }
    if let Some(flush) = &s.flush {
        row["flush"] = flush_json(flush);
    }
    row
}

/// The net knobs as the set request spells them: percent, not per thousand.
fn net_json(net: &NetChaos) -> Value {
    json!({
        "latency": net.latency_ms,
        "jitter": net.jitter_ms,
        "loss": f64::from(net.loss_per_thousand) / 10.0,
        "dup": f64::from(net.dup_per_thousand) / 10.0,
        "reorder": net.reorder,
        "seed": net.seed,
    })
}

fn flush_json(flush: &FlushChaos) -> Value {
    match flush {
        FlushChaos::Delay { ms } => json!({ "mode": "delay", "delay_ms": ms }),
        FlushChaos::Reorder { seed } => json!({ "mode": "reorder", "seed": seed }),
        FlushChaos::Pack { records } => json!({ "mode": "pack", "records": records }),
        FlushChaos::Shift { len } => json!({ "mode": "shift", "shift": len }),
        FlushChaos::ShiftSweep { seed } => {
            json!({ "mode": "shift", "shift": "sweep", "seed": seed })
        }
    }
}

/// Whole seconds left, rounded down.
fn secs_until(expires_at: Instant, now: Instant) -> u64 {
    expires_at.saturating_duration_since(now).as_secs()
}

/// The session address of a connected player, from the roster
/// `server_sessions` and the packet tap read. A row with entity id 0 is a
/// session with no player entity (account or character select) and never
/// matches.
fn session_addr(roster: &[OnlinePlayer], entity_id: u32) -> Option<SocketAddr> {
    roster
        .iter()
        .find(|p| p.id != 0 && p.id == entity_id)
        .and_then(|p| p.session.parse().ok())
}

/// The player entity at `addr` now, or 0 when no player holds it.
fn entity_at(roster: &[OnlinePlayer], addr: SocketAddr) -> u32 {
    roster
        .iter()
        .find(|p| p.session.parse::<SocketAddr>().ok() == Some(addr))
        .map_or(0, |p| p.id)
}

fn no_session(entity_id: u32) -> String {
    ChaosRefused::NoSession(entity_id).to_string()
}
