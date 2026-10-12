//! CH-14 (D-CH10): `server_chaos_set` and `server_chaos_clear` take an
//! `account` in place of `entity_id`, to arm chaos for that account's next
//! world entry. Exactly one target per set, an unknown account refused, and
//! status lists arms by account id, never an address.

use std::sync::Arc;
use std::time::Instant;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use serde_json::{json, Value};

use cimmeria_admin_api::ws::broadcast_layer::LogBuffer;
use cimmeria_services::base::lab_chaos::FlushChaos;
use cimmeria_services::base::lab_chaos::LabChaos;
use cimmeria_services::orchestrator::Orchestrator;

use super::chaos::{clear_account_on, clear_on, set_on, status_on};
use super::chaos_args::{audit_args, ClearCall, FlushArgs, SetArgs, SetCall};
use super::LabTools;
use crate::state::LabState;

const ACCOUNT: u32 = 4242;
const NAME: &str = "lab-ch14";

fn tools() -> LabTools {
    let orchestrator = Arc::new(Orchestrator::new(Default::default()));
    LabTools::new(LabState::new(orchestrator, LogBuffer::new()))
}

fn reply(result: Result<CallToolResult, rmcp::ErrorData>) -> Value {
    let result = result.expect("a refusal is a successful call");
    assert_ne!(result.is_error, Some(true), "a refusal is not a tool error");
    let text = &result.content[0].as_text().expect("JSON text").text;
    serde_json::from_str(text).expect("the reply parses")
}

/// A flush pack request for `account`.
fn arm_args(account: &str) -> SetArgs {
    SetArgs {
        entity_id: None,
        account: Some(account.to_string()),
        net: None,
        flush: Some(FlushArgs {
            mode: "pack".to_string(),
            delay_ms: None,
            records: Some(24),
            shift: None,
        }),
        seed: None,
        ttl_s: None,
        label: None,
    }
}

/// Knows one account, [`NAME`] as [`ACCOUNT`].
async fn one_account(name: String) -> Result<Option<u32>, String> {
    Ok((name == NAME).then_some(ACCOUNT))
}

async fn arm(registry: &LabChaos, args: SetArgs) -> Result<Value, String> {
    set_on(registry, args, || async { Vec::new() }, one_account).await
}

/// Both targets, neither, and an empty account name are refused by the
/// parse, before the registry is looked at: the test binary installs none,
/// so a call that got past the parse would read "lab chaos is off". Fails
/// if the exactly-one check is dropped or moved behind the registry.
#[tokio::test]
async fn set_refuses_both_targets_neither_and_an_empty_account() {
    let tools = tools();
    let flush = json!({ "mode": "pack", "records": 4 });
    for (raw, why) in [
        (
            json!({ "entity_id": 7, "account": NAME, "flush": flush }),
            "give entity_id or account, not both",
        ),
        (json!({ "flush": flush }), "set needs entity_id or account"),
        (json!({ "account": "", "flush": flush }), "account is empty"),
    ] {
        let call: SetCall = serde_json::from_value(raw.clone()).expect("a call");
        let v = reply(tools.server_chaos_set(Parameters(call)).await);
        assert_eq!(
            v,
            json!({ "refused": format!("bad chaos knob: {why}") }),
            "{raw}"
        );
    }
    let call: ClearCall =
        serde_json::from_value(json!({ "entity_id": 7, "account": NAME })).expect("a call");
    let v = reply(tools.server_chaos_clear(Parameters(call)).await);
    assert_eq!(
        v,
        json!({ "refused": "bad chaos knob: give entity_id or account, not both" })
    );
}

/// The arm reply names the account id and the expiry, and status lists the
/// arm (not as a session) with no address. Fails if the account path sets
/// nothing, or status drops the arms.
#[tokio::test]
async fn set_with_an_account_arms_and_status_lists_it() {
    let registry = LabChaos::new();
    assert_eq!(
        arm(&registry, arm_args(NAME)).await,
        Ok(json!({ "armed": true, "account_id": ACCOUNT, "expires_in_s": 300 }))
    );
    let status = status_on(&registry, Instant::now());
    assert_eq!(status["sessions"], json!([]), "an arm is not a session yet");
    let row = &status["armed"][0];
    assert_eq!(row["account_id"], ACCOUNT);
    assert!(row["expires_in_s"].as_u64().is_some_and(|s| s <= 300));
    assert_eq!(row["flush"], json!({ "mode": "pack", "records": 24 }));
    assert!(row.get("entity_id").is_none(), "{row}");
    assert!(
        !status.to_string().contains("127.0.0.1"),
        "status never names an address: {status}"
    );
    // The registry holds what the tool armed, ready for the account's
    // next playCharacter.
    assert!(registry
        .promote_armed(
            ACCOUNT,
            "127.0.0.1:50100".parse().unwrap(),
            9,
            Instant::now()
        )
        .is_some());
    assert_eq!(
        registry.flush_for("127.0.0.1:50100".parse().unwrap(), Instant::now()),
        Some(FlushChaos::Pack { records: 24 })
    );
}

/// A name no account has is refused and arms nothing; so is a failed
/// lookup, with its error.
#[tokio::test]
async fn set_refuses_an_unknown_account() {
    let registry = LabChaos::new();
    assert_eq!(
        arm(&registry, arm_args("nobody")).await,
        Err("no account named \"nobody\"".to_string())
    );
    let failed = set_on(
        &registry,
        arm_args(NAME),
        || async { Vec::new() },
        |_name| async { Err("account lookup failed: down".to_string()) },
    )
    .await;
    assert_eq!(failed, Err("account lookup failed: down".to_string()));
    assert!(registry.armed_status(Instant::now()).is_empty());
}

/// The arm goes through the registry's bounds like a set: over the TTL cap
/// is refused, not clamped.
#[tokio::test]
async fn an_arm_gets_the_registry_refusal() {
    let registry = LabChaos::new();
    let args = SetArgs {
        ttl_s: Some(1801),
        ..arm_args(NAME)
    };
    assert_eq!(
        arm(&registry, args).await,
        Err("ttl 1801s is over the 1800 s cap".to_string())
    );
    assert!(registry.armed_status(Instant::now()).is_empty());
}

/// Clear by account removes that arm; an unknown name is refused; clear
/// with no target counts the arms with the sessions.
#[tokio::test]
async fn clear_by_account_and_clear_all() {
    let registry = LabChaos::new();
    arm(&registry, arm_args(NAME)).await.expect("armed");
    assert_eq!(
        clear_account_on(&registry, NAME.to_string(), one_account).await,
        Ok(json!({ "cleared": 1 }))
    );
    assert_eq!(
        clear_account_on(&registry, NAME.to_string(), one_account).await,
        Ok(json!({ "cleared": 0 }))
    );
    assert_eq!(
        clear_account_on(&registry, "nobody".to_string(), one_account).await,
        Err("no account named \"nobody\"".to_string())
    );
    arm(&registry, arm_args(NAME)).await.expect("armed");
    assert_eq!(clear_on(&registry, None), Ok(json!({ "cleared": 1 })));
    assert_eq!(
        status_on(&registry, Instant::now()),
        json!({ "sessions": [], "armed": [] })
    );
}

/// An account name longer than `account.account_name` (32 characters) or
/// with a control character is refused by the parse, on a set and on a
/// clear, without echoing it; 32 characters reach the lookup. Fails if the
/// cap is dropped (the long name would reach the lookup and read "lab
/// chaos is off" here, where no registry is installed).
#[tokio::test]
async fn an_account_name_no_account_can_have_is_refused() {
    let tools = tools();
    let long = "a".repeat(33);
    let flush = json!({ "mode": "pack", "records": 4 });
    for (account, why) in [
        (long.as_str(), "account is over 32 characters"),
        ("lab\nERROR forged", "account has a control character"),
    ] {
        let call: SetCall =
            serde_json::from_value(json!({ "account": account, "flush": flush })).expect("a call");
        let v = reply(tools.server_chaos_set(Parameters(call)).await);
        assert_eq!(v, json!({ "refused": format!("bad chaos knob: {why}") }));
        let call: ClearCall =
            serde_json::from_value(json!({ "account": account })).expect("a call");
        let v = reply(tools.server_chaos_clear(Parameters(call)).await);
        assert_eq!(v, json!({ "refused": format!("bad chaos knob: {why}") }));
    }
    let registry = LabChaos::new();
    let at_cap = "b".repeat(32);
    assert_eq!(
        arm(&registry, arm_args(&at_cap)).await,
        Err(format!("no account named {at_cap:?}")),
        "32 characters is a name the lookup is asked about"
    );
}

/// The audit row never carries a refused account name, only its length; an
/// acceptable one is kept as sent. Fails if `audit_args` passes the account
/// through unchecked.
#[test]
fn audit_args_redacts_a_refused_account() {
    let long = "c".repeat(40);
    assert_eq!(
        audit_args(&json!({ "account": long }))["account"],
        "<account: 40 chars, refused>"
    );
    assert_eq!(
        audit_args(&json!({ "account": "lab\nERROR forged" }))["account"],
        "<account: 16 chars, refused>"
    );
    let fine = json!({ "account": NAME, "flush": { "mode": "reorder" } });
    assert_eq!(audit_args(&fine), fine);
}
