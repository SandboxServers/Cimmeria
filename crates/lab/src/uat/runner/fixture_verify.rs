//! The verify step of a fixture row (lab-fixtures FX-07, D-FX6): before the
//! row's steps run, the staged state is checked with read-only SQL, and the
//! fixture character's session must be in the fixture's world and its
//! entity near the fixture's position. A state that does not hold yet is
//! retried for `VERIFY_TIMEOUT_MS`; then the row is BLOCKED with every check
//! still failing.
//!
//! The calls go through `server_call`, so they are on the row's record and
//! `establish_fixture` re-marks them `kind = "fixture"`.

use std::time::Duration;

use serde_json::{json, Value};

use super::packet::{as_entity, ENTITY_VAR};
use super::{RowCtx, Runner};
use crate::uat::fixture::{verify_checks, FixtureCheck, FIXTURE_CHARACTER_VAR};
use crate::uat::invoke::{ServerInvoker, ToolInvoker, ToolOutcome};
use crate::uat::spec::FixtureSpec;
use crate::uat::tier::Role;

/// How long the checks may take to hold before the row is BLOCKED.
pub(crate) const VERIFY_TIMEOUT_MS: u64 = 5_000;
/// The pause between two attempts at the whole check set.
pub(crate) const VERIFY_POLL_MS: u64 = 500;
/// How far the session's entity may sit from the fixture's position, metres.
pub(crate) const POSITION_TOLERANCE_M: f64 = 2.0;
/// The longest error the verify step returns; the BLOCKED reason adds a
/// prefix on top.
pub(crate) const REASON_CAP: usize = 300;

impl<I: ToolInvoker> Runner<'_, I> {
    /// Check the fixture is in place: every `verify_checks` check holds
    /// through `server_db_query`, the fixture character's session is in the
    /// fixture's world, and its entity is within [`POSITION_TOLERANCE_M`] of
    /// the fixture's position. Retried until they hold or
    /// [`VERIFY_TIMEOUT_MS`] passes; the Err names each check still failing.
    pub(crate) async fn verify_fixture(
        &mut self,
        f: &FixtureSpec,
        ctx: &mut RowCtx,
    ) -> Result<(), String> {
        let server = self
            .server
            .ok_or("no server lab MCP to verify the fixture")?;
        let name = ctx
            .vars
            .get(FIXTURE_CHARACTER_VAR)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or("no fixture character to verify")?;
        let checks = verify_checks(f, &name)?;
        let entity = ctx.vars.get(ENTITY_VAR).and_then(as_entity);
        if f.position.is_some() && entity.is_none() {
            return Err("no player entity to check the position of".into());
        }
        let mut failing = Vec::new();
        for i in 0..VERIFY_TIMEOUT_MS / VERIFY_POLL_MS {
            // Tests skip the pause but keep the attempt count, so a state
            // that never holds still times out.
            if i > 0 && !self.req.no_settle {
                tokio::time::sleep(Duration::from_millis(VERIFY_POLL_MS)).await;
            }
            failing = self
                .verify_attempt(server, f, &name, &checks, entity, ctx)
                .await;
            if failing.is_empty() {
                return Ok(());
            }
        }
        Err(capped(&failing.join("; ")))
    }

    /// One pass over the checks: each failing one as `what: got v, want w`.
    async fn verify_attempt(
        &self,
        server: &dyn ServerInvoker,
        f: &FixtureSpec,
        name: &str,
        checks: &[FixtureCheck],
        entity: Option<u32>,
        ctx: &mut RowCtx,
    ) -> Vec<String> {
        let mut failing = Vec::new();
        for c in checks {
            let args = json!({ "sql": c.sql });
            let out = self
                .server_call(server, Role::Setup, "server_db_query", args, ctx)
                .await;
            let got = reply_value(&out, |j| {
                j.pointer(&c.pointer).cloned().unwrap_or(Value::Null)
            });
            if !same(&got, &c.want) {
                failing.push(miss(&c.what, &got, &c.want));
            }
        }
        if let Some(world) = &f.world {
            let out = self
                .server_call(server, Role::Setup, "server_sessions", json!({}), ctx)
                .await;
            let got = reply_value(&out, |j| session_world(j, name));
            let want = json!(world);
            if !same(&got, &want) {
                failing.push(miss("session world", &got, &want));
            }
        }
        if let (Some(want), Some(entity)) = (f.position, entity) {
            let args = json!({ "entity_id": entity });
            let out = self
                .server_call(server, Role::Setup, "server_entity_get", args, ctx)
                .await;
            let got = reply_value(&out, |j| j["entity"]["position"].clone());
            if !near(&got, want) {
                let what = format!("position within {POSITION_TOLERANCE_M} m");
                failing.push(miss(&what, &got, &json!(want)));
            }
        }
        failing
    }
}

/// The value a reply holds at the checked place, or the tool's error text
/// when the call failed (so the BLOCKED reason keeps why it could not check).
fn reply_value(out: &ToolOutcome, at: impl FnOnce(&Value) -> Value) -> Value {
    if out.ok {
        at(&out.json)
    } else {
        json!(format!(
            "error: {}",
            out.error.as_deref().unwrap_or_default()
        ))
    }
}

/// The world of the session whose character is `name`, or null when no
/// session lists it. A fresh character is listed by its full name, so any
/// whole word of the listing that is `name` matches.
fn session_world(reply: &Value, name: &str) -> Value {
    reply["sessions"]
        .as_array()
        .and_then(|list| {
            list.iter().find(|s| {
                s["name"]
                    .as_str()
                    .is_some_and(|n| n.split_whitespace().any(|w| w.eq_ignore_ascii_case(name)))
            })
        })
        .map_or(Value::Null, |s| s["world"].clone())
}

/// Whether the reported position is within [`POSITION_TOLERANCE_M`] of
/// `want`. A position that is not three numbers is not near.
fn near(got: &Value, want: [f32; 3]) -> bool {
    let Some(g) = got.as_array().filter(|a| a.len() == 3) else {
        return false;
    };
    let d2: f64 = g
        .iter()
        .zip(want)
        .map(|(a, b)| (a.as_f64().unwrap_or(f64::NAN) - f64::from(b)).powi(2))
        .sum();
    d2.sqrt() <= POSITION_TOLERANCE_M
}

/// Whether a reply equals the wanted value: numbers by value (1 and 1.0
/// agree), anything else by JSON equality (a string `"1"` is not 1).
fn same(got: &Value, want: &Value) -> bool {
    match (got.as_f64(), want.as_f64()) {
        (Some(a), Some(b)) => a == b,
        _ => got == want,
    }
}

fn miss(what: &str, got: &Value, want: &Value) -> String {
    format!("{what}: got {got}, want {want}")
}

/// The joined errors, cut to [`REASON_CAP`] characters ending in `...`.
fn capped(s: &str) -> String {
    if s.chars().count() <= REASON_CAP {
        return s.to_string();
    }
    let head: String = s.chars().take(REASON_CAP - 3).collect();
    format!("{head}...")
}
