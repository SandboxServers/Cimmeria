//! `server_chaos_set`, `server_chaos_clear` and `server_chaos_status`
//! (lab-chaos CH-05). Built only with the `lab-chaos` feature, so an ordinary
//! server binary does not list them.
//!
//! The server's registry (`base::lab_chaos`) is the authority: these tools
//! translate arguments ([`super::chaos_args`]), resolve a player entity to
//! its session address, or an account name to its id, and report the
//! registry's answer. Bounds are the registry's to check; nothing here
//! clamps. Every refusal, including "not installed" and a malformed call, is
//! a successful call whose reply is `{refused: reason}`, so a lab run can
//! read why, and every call writes one audit row. Status names entity and
//! account ids only, never an address.
//!
//! A set names an in-world `entity_id`, or an `account` (CH-14, D-CH10):
//! then the chaos is armed for that account's next world entry, one-shot,
//! and the base moves it onto the session at `playCharacter`, before that
//! login's first flush. Like `entity_id`, any account may be named, and an
//! arm does reach further than a set: it waits for an account that is not
//! online, until its TTL. Only deployment keeps it off real players (no
//! colo image is built with `lab-chaos`, and the registry needs
//! `DEVELOPER_MODE`). An armed net profile starts shaping at the
//! RESET_ENTITIES send; authentication, the character list and
//! `playCharacter` itself are never shaped by it.

use std::future::Future;
use std::net::SocketAddr;
use std::time::Instant;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router, ErrorData};
use serde_json::{json, Value};

use cimmeria_services::base::lab_chaos::ArmedChaos;
use cimmeria_services::base::lab_chaos::ChaosRefused;
use cimmeria_services::base::lab_chaos::FlushChaos;
use cimmeria_services::base::lab_chaos::LabChaos;
use cimmeria_services::base::lab_chaos::NetChaos;
use cimmeria_services::base::lab_chaos::SessionChaos;
use cimmeria_services::base::lab_chaos::DEFAULT_TTL_S;
use cimmeria_services::base::OnlinePlayer;

use super::chaos_args::{
    audit_args, built_label, check_label, check_seed_used, clear_target, flush_from, net_from,
    parse, set_target, ClearArgs, ClearCall, SetArgs, SetCall, Target,
};
use super::{ok, LabTools};
use crate::audit::{self, OUTCOME_ERROR, OUTCOME_OK};
use crate::state::LabState;

/// The most sessions, and separately the most arms, `server_chaos_status`
/// lists.
const STATUS_CAP: usize = 10;

/// The refusal every chaos tool gives when the server did not install the
/// registry (`DEVELOPER_MODE` off).
pub(super) const OFF: &str = "lab chaos is off (DEVELOPER_MODE)";

#[tool_router(router = chaos_router, vis = "pub(super)")]
impl LabTools {
    /// Arm chaos on one player's session, or for an account's next login.
    #[tool(
        description = "Lab-chaos builds only: arm per-session fault injection on a player's session (entity_id), or for an account's next world entry (account: the account name as at login; one-shot, attached at that login's playCharacter so the flush profile reaches its first-login flush). Give exactly one of entity_id and account. Give a net profile {latency, jitter (ms), loss, dup (percent, at most one decimal), reorder (packets)} and/or a flush profile {mode: delay|reorder|pack|shift, delay_ms, records, shift (bytes or \"sweep\")}, a seed (default 0), ttl_s (default 300, at most 1800; an arm keeps counting it down until and after its login) and an optional label. Unknown fields, an empty net profile, a knob the flush mode ignores and an unused seed are refused. Replaces the session's chaos or the account's arm. Returns {set: true, expires_in_s}, {armed: true, account_id, expires_in_s} or {refused: reason}."
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
            set_target(&args)?;
            Ok((
                args,
                installed(cimmeria_services::base::lab_chaos::current())?,
            ))
        }) {
            Ok((args, registry)) => {
                set_on(
                    registry,
                    args,
                    || self.state.online_players(),
                    |name| account_id(&self.state, name),
                )
                .await
            }
            Err(e) => Err(e),
        };
        refusal_result("server_chaos_set", &audit_args, started, result)
    }

    /// Clear chaos on one player's session, one account's arm, or all.
    #[tool(
        description = "Lab-chaos builds only: clear the chaos set for one player entity (entity_id), or armed for one account's next world entry (account), or with neither every session and every arm. Returns {cleared: n} or {refused: reason}."
    )]
    pub(super) async fn server_chaos_clear(
        &self,
        Parameters(ClearCall(raw)): Parameters<ClearCall>,
    ) -> Result<CallToolResult, ErrorData> {
        let started = Instant::now();
        let audit_args = audit_args(&raw);
        let result = async {
            let target = parse::<ClearArgs>(raw).and_then(|args| clear_target(&args))?;
            let registry = installed(cimmeria_services::base::lab_chaos::current())?;
            match target {
                Some(Target::Account(name)) => {
                    clear_account_on(registry, name, |name| account_id(&self.state, name)).await
                }
                Some(Target::Entity(id)) => clear_on(registry, Some(id)),
                None => clear_on(registry, None),
            }
        }
        .await;
        refusal_result("server_chaos_clear", &audit_args, started, result)
    }

    /// List the sessions under live chaos and the pending arms.
    #[tool(
        description = "Lab-chaos builds only: list the sessions under live chaos by player entity id, and the chaos armed for an account's next world entry by account id, each with its net and flush knobs and expires_in_s, at most 10 of each (with more / armed_more: n when there are others). Returns {sessions: [...], armed: [...]} or {refused: reason}."
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

/// The account `name` resolves to, through the server's database.
async fn account_id(state: &LabState, name: String) -> Result<Option<u32>, String> {
    let pool = state
        .db_pool()
        .await
        .ok_or_else(|| "no database to look the account up in".to_string())?;
    super::account_lookup::account_id_named(&pool, &name).await
}

/// A set request's profiles, label and TTL, checked as far as the tool can.
struct Request {
    net: Option<NetChaos>,
    flush: Option<FlushChaos>,
    label: String,
    ttl_s: u32,
}

fn request_from(args: SetArgs) -> Result<Request, String> {
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
    Ok(Request {
        net,
        flush,
        label,
        ttl_s,
    })
}

/// Sets the request on the entity's session, or arms it for the account's
/// next world entry. `roster` resolves an entity, `accounts` an account name
/// (`Ok(None)` when no account has it).
pub(super) async fn set_on<F, Fut, A, AFut>(
    registry: &LabChaos,
    args: SetArgs,
    roster: F,
    accounts: A,
) -> Result<Value, String>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Vec<OnlinePlayer>>,
    A: Fn(String) -> AFut,
    AFut: Future<Output = Result<Option<u32>, String>>,
{
    let target = set_target(&args)?;
    // The roster lists a session with no player entity as id 0, so 0 never
    // names a player.
    if target == Target::Entity(0) {
        return Err(no_session(0));
    }
    let request = request_from(args)?;
    match target {
        Target::Entity(entity_id) => set_entity(registry, entity_id, request, roster).await,
        Target::Account(name) => {
            let account_id = resolve(&accounts, name).await?;
            arm_account(registry, account_id, request)
        }
    }
}

/// Sets on the entity's session, then checks the entity still owns that
/// address. `roster` is read twice: once to resolve the address, once after
/// `set`, because the session can end and another can take the address in
/// between (the CH-04 review's race).
async fn set_entity<F, Fut>(
    registry: &LabChaos,
    entity_id: u32,
    request: Request,
    roster: F,
) -> Result<Value, String>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Vec<OnlinePlayer>>,
{
    let addr = session_addr(&roster().await, entity_id).ok_or_else(|| no_session(entity_id))?;
    let now = Instant::now();
    let expires_at = registry
        .set(
            addr,
            entity_id,
            request.label,
            request.net,
            request.flush,
            request.ttl_s,
            now,
        )
        .map_err(|e| e.to_string())?;

    let after = roster().await;
    if session_addr(&after, entity_id) != Some(addr) {
        registry.clear_if_not(addr, entity_at(&after, addr));
        return Err(no_session(entity_id));
    }
    Ok(json!({ "set": true, "expires_in_s": secs_until(expires_at, now) }))
}

/// Arms the request for the account's next world entry. Whether the account
/// is online does not matter: the arm waits for its next `playCharacter`,
/// which a character in the world reaches through character select.
fn arm_account(registry: &LabChaos, account_id: u32, request: Request) -> Result<Value, String> {
    let now = Instant::now();
    let expires_at = registry
        .arm(
            account_id,
            request.label,
            request.net,
            request.flush,
            request.ttl_s,
            now,
        )
        .map_err(|e| e.to_string())?;
    Ok(json!({
        "armed": true,
        "account_id": account_id,
        "expires_in_s": secs_until(expires_at, now),
    }))
}

/// An account name to its id, or the refusal for a name no account has.
async fn resolve<A, AFut>(accounts: &A, name: String) -> Result<u32, String>
where
    A: Fn(String) -> AFut,
    AFut: Future<Output = Result<Option<u32>, String>>,
{
    accounts(name.clone())
        .await?
        .ok_or_else(|| format!("no account named {name:?}"))
}

/// Clears the chaos set for one entity, or every session and arm with
/// `None`. By the entry's entity tag, not the live roster: after a return
/// to character select the entity has no session, but status still lists it.
pub(super) fn clear_on(registry: &LabChaos, entity_id: Option<u32>) -> Result<Value, String> {
    let cleared = match entity_id {
        None => registry.clear(None),
        Some(0) => return Err(no_session(0)),
        Some(id) => registry.clear_entity(id),
    };
    Ok(json!({ "cleared": cleared }))
}

/// Clears the arm for the account `name` resolves to.
pub(super) async fn clear_account_on<A, AFut>(
    registry: &LabChaos,
    name: String,
    accounts: A,
) -> Result<Value, String>
where
    A: Fn(String) -> AFut,
    AFut: Future<Output = Result<Option<u32>, String>>,
{
    let account_id = resolve(&accounts, name).await?;
    Ok(json!({ "cleared": registry.clear_account(account_id) }))
}

/// The live sessions by entity id and the arms by account id, at most
/// [`STATUS_CAP`] of each, and how many more.
pub(super) fn status_on(registry: &LabChaos, now: Instant) -> Value {
    let live = registry.status(now);
    let armed = registry.armed_status(now);
    let mut reply = json!({
        "sessions": live.iter().take(STATUS_CAP).map(|s| status_row(s, now)).collect::<Vec<_>>(),
        "armed": armed.iter().take(STATUS_CAP).map(|a| armed_row(a, now)).collect::<Vec<_>>(),
    });
    if live.len() > STATUS_CAP {
        reply["more"] = json!(live.len() - STATUS_CAP);
    }
    if armed.len() > STATUS_CAP {
        reply["armed_more"] = json!(armed.len() - STATUS_CAP);
    }
    reply
}

fn status_row(s: &SessionChaos, now: Instant) -> Value {
    let row = json!({
        "entity_id": s.entity_id,
        "expires_in_s": secs_until(s.expires_at, now),
    });
    with_profiles(row, s.net.as_ref(), s.flush.as_ref())
}

fn armed_row(a: &ArmedChaos, now: Instant) -> Value {
    let row = json!({
        "account_id": a.account_id,
        "expires_in_s": secs_until(a.expires_at, now),
    });
    with_profiles(row, a.net.as_ref(), a.flush.as_ref())
}

fn with_profiles(mut row: Value, net: Option<&NetChaos>, flush: Option<&FlushChaos>) -> Value {
    if let Some(net) = net {
        row["net"] = net_json(net);
    }
    if let Some(flush) = flush {
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
