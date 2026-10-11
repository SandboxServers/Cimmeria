//! The verify step of a fixture row (lab-fixtures FX-07, D-FX6): the staged
//! state is checked with read-only SQL, the fixture character's session must
//! be in the fixture's world, and its entity near the fixture's position. A
//! state that does not hold yet is retried for `VERIFY_TIMEOUT_MS`; then the
//! row is BLOCKED with every check still failing.
//!
//! It runs in two parts. The database checks and the session world run right
//! after Play returns, before anything else, because game content may change
//! those rows once the character is in world (`show_tutorial` inserts the
//! tutorial row, a mission chain can advance a step). The position check
//! needs the player entity, so it runs once that is resolved.
//!
//! The calls go through `server_call`, so they are on the row's record and
//! `establish_fixture` re-marks them `kind = "fixture"`.

use std::time::Duration;

use serde_json::{json, Value};
use tokio::time::Instant;

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
///
/// A fixture position must be on the ground: the client, not the server,
/// moves the entity after world entry, so a point in the air has fallen to
/// the ground before the check reads it, and the row BLOCKs.
pub(crate) const POSITION_TOLERANCE_M: f64 = 2.0;
/// The longest error the verify step returns; the BLOCKED reason adds a
/// prefix on top.
pub(crate) const REASON_CAP: usize = 300;

/// Which half of the verify step an attempt runs.
#[derive(Debug, Clone, Copy)]
enum Part {
    /// The database checks and the session's world.
    Staged,
    /// The player entity's position.
    Position(u32),
}

impl<I: ToolInvoker> Runner<'_, I> {
    /// Check the staged state right after Play: every `verify_checks` check
    /// holds through `server_db_query` and the fixture character's session
    /// is in the fixture's world.
    ///
    /// Game content can still change these rows between world entry and the
    /// first query (a tutorial shown on entry inserts its row). Checking this
    /// early narrows that window but does not close it; the race is accepted,
    /// and FX-12's live run decides whether it bites.
    pub(crate) async fn verify_staged(
        &mut self,
        f: &FixtureSpec,
        ctx: &mut RowCtx,
    ) -> Result<(), String> {
        self.verify_until(f, Part::Staged, ctx).await
    }

    /// Check the player entity `entity` is within [`POSITION_TOLERANCE_M`]
    /// of the fixture's position. Nothing to check without a `position`.
    pub(crate) async fn verify_position(
        &mut self,
        f: &FixtureSpec,
        entity: u32,
        ctx: &mut RowCtx,
    ) -> Result<(), String> {
        if f.position.is_none() {
            return Ok(());
        }
        self.verify_until(f, Part::Position(entity), ctx).await
    }

    /// Run `part` until every check holds or [`VERIFY_TIMEOUT_MS`] has
    /// passed, always finishing at least one full attempt; the Err names each
    /// check still failing.
    async fn verify_until(
        &mut self,
        f: &FixtureSpec,
        part: Part,
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
        let checks = match part {
            Part::Staged => verify_checks(f, &name)?,
            Part::Position(_) => Vec::new(),
        };
        let timeout = Duration::from_millis(VERIFY_TIMEOUT_MS);
        // Tests skip the pause, so they bound the attempts by count instead:
        // as many as the pauses would allow, and a state that never holds
        // still times out.
        let max_attempts = VERIFY_TIMEOUT_MS / VERIFY_POLL_MS;
        let started = Instant::now();
        let mut attempts = 0;
        loop {
            let failing = self
                .verify_attempt(server, f, &name, &checks, part, ctx)
                .await;
            attempts += 1;
            if failing.is_empty() {
                return Ok(());
            }
            let out_of_time = if self.req.no_settle {
                attempts >= max_attempts
            } else {
                started.elapsed() >= timeout
            };
            if out_of_time {
                let reason = capped(&failing.join("; "));
                tracing::warn!(target: "lab.uat", event = "fixture_verify_blocked",
                    row = %ctx.row_id, character = %name, attempts, reason = %reason,
                    "fixture state did not hold");
                return Err(reason);
            }
            if !self.req.no_settle {
                tokio::time::sleep(Duration::from_millis(VERIFY_POLL_MS)).await;
            }
        }
    }

    /// One pass over `part`'s checks: each failing one as
    /// `what: got v, want w`.
    async fn verify_attempt(
        &self,
        server: &dyn ServerInvoker,
        f: &FixtureSpec,
        name: &str,
        checks: &[FixtureCheck],
        part: Part,
        ctx: &mut RowCtx,
    ) -> Vec<String> {
        let mut failing = Vec::new();
        match part {
            Part::Staged => {
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
            }
            Part::Position(entity) => {
                let Some(want) = f.position else {
                    return failing;
                };
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
pub(super) fn session_world(reply: &Value, name: &str) -> Value {
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
/// `want` (see there: `want` must be on the ground). A position that is not
/// three numbers is not near.
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
    crate::uat::snapshot::cap_chars(s, REASON_CAP)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capped_counts_characters_not_bytes() {
        // Two bytes per char: a byte-indexed cut would split one and panic.
        let s = capped(&"é".repeat(400));
        assert_eq!(s.chars().count(), REASON_CAP);
        assert!(s.ends_with("..."), "{s}");
        assert!(s.starts_with('é'));
    }

    #[test]
    fn capped_leaves_a_short_reason_alone() {
        assert_eq!(capped("é"), "é");
    }
}
