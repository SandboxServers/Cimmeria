//! `server_chaos_set`, `server_chaos_clear` and `server_chaos_status`
//! (lab-chaos CH-05). Built only with the `lab-chaos` feature, so an ordinary
//! server binary does not list them.
//!
//! The server's registry (`base::lab_chaos`) is the authority: these tools
//! translate arguments, resolve a player entity to its session address and
//! report the registry's answer. Bounds are the registry's to check; nothing
//! here clamps. Every refusal, including "not installed", is a successful
//! call whose reply is `{refused: reason}`, so a lab run can read why.
//! Status names entity ids only, never an address.

use std::future::Future;
use std::net::SocketAddr;
use std::time::Instant;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::schemars::{self, JsonSchema};
use rmcp::{tool, tool_router, ErrorData};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use cimmeria_services::base::lab_chaos::ChaosRefused;
use cimmeria_services::base::lab_chaos::FlushChaos;
use cimmeria_services::base::lab_chaos::LabChaos;
use cimmeria_services::base::lab_chaos::NetChaos;
use cimmeria_services::base::lab_chaos::SessionChaos;
use cimmeria_services::base::lab_chaos::DEFAULT_TTL_S;
use cimmeria_services::base::OnlinePlayer;

use super::{ok, LabTools};
use crate::audit::{self, OUTCOME_ERROR, OUTCOME_OK};

/// The most sessions `server_chaos_status` lists.
const STATUS_CAP: usize = 10;

/// The longest `label` a set takes. The label is logged on every set WARN;
/// a CH-01 header is a few dozen characters.
const LABEL_MAX: usize = 120;

/// The refusal every chaos tool gives when the server did not install the
/// registry (`DEVELOPER_MODE` off).
pub(super) const OFF: &str = "lab chaos is off (DEVELOPER_MODE)";

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub(super) struct SetArgs {
    /// The player entity whose session gets the chaos.
    pub(super) entity_id: u32,
    /// The net profile's knobs. Give `net`, `flush` or both.
    #[serde(default)]
    pub(super) net: Option<NetArgs>,
    /// The flush profile's knobs. Give `net`, `flush` or both.
    #[serde(default)]
    pub(super) flush: Option<FlushArgs>,
    /// Seeds the random parts of the profiles, so a run replays. Default 0.
    #[serde(default)]
    pub(super) seed: Option<u64>,
    /// Seconds until the chaos expires. Default 300; 0 or over 1800 is refused.
    #[serde(default)]
    pub(super) ttl_s: Option<u32>,
    /// The chaos spec text the server logs as `level`, such as
    /// `flush:pack seed=7`. Built from the profiles when omitted.
    #[serde(default)]
    pub(super) label: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
pub(super) struct NetArgs {
    /// Added delay on each server send, in ms (at most 5000).
    #[serde(default)]
    pub(super) latency: Option<u32>,
    /// Random spread on top of the latency, in ms (at most 5000).
    #[serde(default)]
    pub(super) jitter: Option<u32>,
    /// Percent of packets dropped, one decimal (0 to 100).
    #[serde(default)]
    pub(super) loss: Option<f64>,
    /// Percent of packets sent twice, one decimal (0 to 100).
    #[serde(default)]
    pub(super) dup: Option<f64>,
    /// Reorder buffer size in packets (at most 1000); 0 keeps order.
    #[serde(default)]
    pub(super) reorder: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub(super) struct FlushArgs {
    /// One of `delay`, `reorder`, `pack`, `shift`.
    pub(super) mode: String,
    /// For `delay`: the hold in ms (at most 60000).
    #[serde(default)]
    pub(super) delay_ms: Option<u32>,
    /// For `pack`: the records in the single bundle (1 to 255).
    #[serde(default)]
    pub(super) records: Option<u16>,
    /// For `shift`: the filler's length in bytes (0 or 2 to 36), or `"sweep"`.
    #[serde(default)]
    pub(super) shift: Option<ShiftArg>,
}

/// A `shift` knob: a byte count, or the word `sweep` for a seeded length.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(untagged)]
pub(super) enum ShiftArg {
    Bytes(u16),
    Word(String),
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub(super) struct ClearArgs {
    /// Clear only this player entity's session. Omit to clear every session.
    #[serde(default)]
    pub(super) entity_id: Option<u32>,
}

#[tool_router(router = chaos_router, vis = "pub(super)")]
impl LabTools {
    /// Arm chaos on one player's session.
    #[tool(
        description = "Lab-chaos builds only: arm per-session fault injection on a player's session (entity_id). Give a net profile {latency, jitter (ms), loss, dup (percent, one decimal), reorder (packets)} and/or a flush profile {mode: delay|reorder|pack|shift, delay_ms, records, shift (bytes or \"sweep\")}, a seed (default 0), ttl_s (default 300, at most 1800) and an optional label. Replaces the session's chaos. Returns {set: true, expires_in_s} or {refused: reason}."
    )]
    pub(super) async fn server_chaos_set(
        &self,
        Parameters(args): Parameters<SetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let started = Instant::now();
        let audit_args = json!(args);
        let result = match installed(cimmeria_services::base::lab_chaos::current()) {
            Ok(registry) => set_on(registry, args, || self.state.online_players()).await,
            Err(e) => Err(e),
        };
        refusal_result("server_chaos_set", &audit_args, started, result)
    }

    /// Clear chaos on one player's session, or on every session.
    #[tool(
        description = "Lab-chaos builds only: clear the chaos on one player's session (entity_id), or on every session when entity_id is omitted. Returns {cleared: n} or {refused: reason}."
    )]
    pub(super) async fn server_chaos_clear(
        &self,
        Parameters(args): Parameters<ClearArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let started = Instant::now();
        let audit_args = json!(args);
        let result = match installed(cimmeria_services::base::lab_chaos::current()) {
            Ok(registry) => {
                let roster = self.state.online_players().await;
                Ok(clear_on(registry, &roster, args.entity_id))
            }
            Err(e) => Err(e),
        };
        refusal_result("server_chaos_clear", &audit_args, started, result)
    }

    /// List the sessions under live chaos.
    #[tool(
        description = "Lab-chaos builds only: list the sessions under live chaos by player entity id, with their net and flush knobs and expires_in_s, at most 10. Returns {sessions: [...]} or {refused: reason}."
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
    let label = match args.label {
        Some(label) if label.chars().count() > LABEL_MAX => {
            return Err(knob_refusal(format!(
                "label is over {LABEL_MAX} characters"
            )));
        }
        Some(label) => label,
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

/// Clears one entity's session, or every session with `None`. An entity
/// with no session clears nothing.
pub(super) fn clear_on(
    registry: &LabChaos,
    roster: &[OnlinePlayer],
    entity_id: Option<u32>,
) -> Value {
    let cleared = match entity_id {
        None => registry.clear(None),
        Some(id) => session_addr(roster, id).map_or(0, |addr| registry.clear(Some(addr))),
    };
    json!({ "cleared": cleared })
}

/// The live sessions by entity id, at most [`STATUS_CAP`].
pub(super) fn status_on(registry: &LabChaos, now: Instant) -> Value {
    let rows: Vec<Value> = registry
        .status(now)
        .iter()
        .take(STATUS_CAP)
        .map(|s| status_row(s, now))
        .collect();
    json!({ "sessions": rows })
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

pub(super) fn net_from(args: &NetArgs, seed: Option<u64>) -> Result<NetChaos, String> {
    Ok(NetChaos {
        latency_ms: args.latency.unwrap_or(0),
        jitter_ms: args.jitter.unwrap_or(0),
        loss_per_thousand: per_thousand(args.loss, "loss")?,
        dup_per_thousand: per_thousand(args.dup, "dup")?,
        reorder: args.reorder.unwrap_or(0),
        seed: seed.unwrap_or(0),
    })
}

/// A percent with one decimal, as tenths of a percent: 1.2 is 12 per
/// thousand. A negative value is refused here, because the cast to the
/// registry's unsigned knob would turn it into 0. A value too large for
/// `u32` saturates and the registry refuses it as over 1000.
fn per_thousand(percent: Option<f64>, knob: &str) -> Result<u32, String> {
    let percent = percent.unwrap_or(0.0);
    if percent.is_nan() || percent < 0.0 {
        return Err(knob_refusal(format!("net {knob} is negative")));
    }
    Ok((percent * 10.0).round() as u32)
}

pub(super) fn flush_from(args: &FlushArgs, seed: Option<u64>) -> Result<FlushChaos, String> {
    let seed = seed.unwrap_or(0);
    match args.mode.as_str() {
        "delay" => args
            .delay_ms
            .map(|ms| FlushChaos::Delay { ms })
            .ok_or_else(|| knob_refusal("flush delay needs delay_ms")),
        "reorder" => Ok(FlushChaos::Reorder { seed }),
        "pack" => args
            .records
            .map(|records| FlushChaos::Pack { records })
            .ok_or_else(|| knob_refusal("flush pack needs records")),
        "shift" => match &args.shift {
            Some(ShiftArg::Bytes(len)) => Ok(FlushChaos::Shift { len: *len }),
            Some(ShiftArg::Word(word)) if word == "sweep" => Ok(FlushChaos::ShiftSweep { seed }),
            _ => Err(knob_refusal("flush shift needs a byte count or \"sweep\"")),
        },
        other => Err(knob_refusal(format!("unknown flush mode {other:?}"))),
    }
}

/// The label logged as `level` when the request gives none, in the CH-01
/// header's shape: `net seed=7`, `flush:pack`, `flush:reorder seed=7`.
pub(super) fn built_label(net: Option<&NetChaos>, flush: Option<&FlushChaos>) -> String {
    let mut parts = Vec::new();
    if let Some(net) = net {
        parts.push(format!("net seed={}", net.seed));
    }
    if let Some(flush) = flush {
        parts.push(match flush {
            FlushChaos::Delay { .. } => "flush:delay".to_string(),
            FlushChaos::Reorder { seed } => format!("flush:reorder seed={seed}"),
            FlushChaos::Pack { .. } => "flush:pack".to_string(),
            FlushChaos::Shift { .. } => "flush:shift".to_string(),
            FlushChaos::ShiftSweep { seed } => format!("flush:shift seed={seed}"),
        });
    }
    parts.join(" ")
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
/// `server_sessions` and the packet tap read.
fn session_addr(roster: &[OnlinePlayer], entity_id: u32) -> Option<SocketAddr> {
    roster
        .iter()
        .find(|p| p.id == entity_id)
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

/// Shaped like the registry's `BadKnob` text, so a reader sees one form.
fn knob_refusal(reason: impl std::fmt::Display) -> String {
    format!("bad chaos knob: {reason}")
}
