//! Fixture rows (lab-fixtures FX-06): a row with a fixture runs in its own
//! session (D-FX5). The runner ends any running session, logs in, stages
//! the fixture for the offline fixture character at character select,
//! presses Play once, runs the row, and ends the session the same way. A
//! fixture character that does not exist yet is created and gets one
//! bootstrap session first. Fixture mode `auto` lets a row continue a
//! passing chain instead (D-FX10).
//!
//! Every call goes through `setup_call` or `server_call`, so it is on the
//! row's record, and is re-marked `kind = "fixture"` so lab-golden can
//! strip it.

use std::time::Duration;

use serde_json::{json, Value};

use super::packet::ENTITY_VAR;
use super::{now_ms, FixtureMode, RowCtx, Runner};
use crate::uat::evidence::ActionRecord;
use crate::uat::fixture::{fixture_character_name, stage_args, FIXTURE_CHARACTER_VAR};
use crate::uat::invoke::{ServerInvoker, ToolInvoker};
use crate::uat::spec::{FixtureSpec, RowSpec, SectionSpec};
use crate::uat::tier::Role;

/// `RowCtx::fixture_mode` when the row ran in its own staged session.
pub(crate) const MODE_FIXTURE: &str = "fixture";
/// `RowCtx::fixture_mode` when the row continued the previous row's session.
pub(crate) const MODE_CHAIN: &str = "chain";
/// The `ActionRecord::kind` of every call that set up or ended a fixture
/// session.
pub(crate) const FIXTURE_KIND: &str = "fixture";
/// The label of the recorded first-login hold.
pub(crate) const FIRST_LOGIN_HOLD_LABEL: &str = "first_login_hold";

/// How long `end_session` waits for the server to stop listing the
/// character. `lab_client_stop` kills SGW.exe and the server only learns
/// of it from the dropped connection (F24), so a login straight after can
/// find the account still online.
pub(crate) const SESSION_DRAIN_MS: u64 = 30_000;
pub(crate) const SESSION_POLL_MS: u64 = 1_000;
/// The server holds a new character in its first-login cinematic for 16 s;
/// a logout inside the hold would leave the character's first login half
/// done. lab-spec-vocab replaces this with a wait for the server's event.
const FIRST_LOGIN_HOLD_MS: u64 = 17_000;

const NO_ENDPOINT: &str = "fixture needs the server lab MCP (CIMMERIA_LAB_MCP_URL/_TOKEN)";

/// Record how the row's fixture ran, on the context and in the row's vars.
pub(crate) fn set_mode(ctx: &mut RowCtx, mode: &'static str) {
    ctx.fixture_mode = Some(mode);
    ctx.vars.insert("fixture_mode".into(), json!(mode));
}

fn contains_ci(hay: &str, needle: &str) -> bool {
    hay.to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

/// Whether any `name` in the array at `key` contains `name`. The lists
/// show a character by its full name (`Uat Fxpraxisdefa`), the runner
/// knows only the last name.
fn lists_name(v: &Value, key: &str, name: &str) -> bool {
    v.get(key).and_then(Value::as_array).is_some_and(|list| {
        list.iter().any(|r| {
            r.get("name")
                .and_then(Value::as_str)
                .is_some_and(|n| contains_ci(n, name))
        })
    })
}

/// The tool's own refusal text. `server_stage_fixture` refuses with a
/// JSON-RPC `invalid_params` error, which the HTTP session surfaces as
/// `lab-mcp error -32602: stage refused: ...`; the BLOCKED reason keeps
/// the part from `stage refused: ` on. Any other failure is kept whole.
fn stage_refusal(err: &str) -> &str {
    err.find(STAGE_REFUSED).map_or(err, |i| &err[i..])
}

const STAGE_REFUSED: &str = "stage refused: ";

/// Every record from `from` on was made by fixture plumbing.
pub(super) fn mark_fixture(ctx: &mut RowCtx, from: usize) {
    if let Some(recs) = ctx.actions.get_mut(from..) {
        for r in recs {
            r.kind = FIXTURE_KIND.into();
        }
    }
}

impl<I: ToolInvoker> Runner<'_, I> {
    /// Whether `row`'s fixture is applied (D-FX10): `auto` continues the
    /// chain only when the row continues a passing spec neighbour.
    pub(crate) fn applies_fixture(&self, spec: &SectionSpec, row: &RowSpec) -> bool {
        match self.req.fixtures {
            FixtureMode::Never => false,
            FixtureMode::Always => true,
            FixtureMode::Auto => !self.continues_passing_neighbour(spec, row),
        }
    }

    /// The previous row of this section in spec order ran last in this
    /// run, passed, and left the client in world. Fixture mode `auto`
    /// chains on it, and only then can a parity mismatch fail a chained
    /// row (FX-10): in `never` mode without it the chain state is whatever
    /// the client happened to hold, so a mismatch is a flag.
    pub(crate) fn continues_passing_neighbour(&self, spec: &SectionSpec, row: &RowSpec) -> bool {
        let prev = spec
            .rows
            .iter()
            .position(|r| r.id == row.id)
            .and_then(|i| i.checked_sub(1))
            .and_then(|i| spec.rows.get(i));
        let continues = match (&self.last_row, prev) {
            (Some(last), Some(prev)) => {
                last.section == spec.section.id && last.row_id == prev.id && last.passed
            }
            _ => false,
        };
        continues && self.in_world_as.is_some()
    }

    /// End the running session: log out when in world, stop the client,
    /// then wait until the server no longer lists the character (F24). A
    /// name an earlier `end_session` failed to drain is drained first.
    pub(crate) async fn end_session(&mut self, ctx: &mut RowCtx) -> Result<(), String> {
        let from = ctx.actions.len();
        // Taken before any call that can fail, so a failed status call does
        // not leave the runner believing it is still in world (and `auto`
        // continuing a chain from a client it stopped).
        let mut name = self.in_world_as.take();
        let mut pending = self.pending_drain.take();
        let r = self.end_session_inner(&mut name, &mut pending, ctx).await;
        mark_fixture(ctx, from);
        if let Err(e) = &r {
            // Keep the oldest undrained name: the next end_session drains it
            // before its own, so the next login cannot meet it still online.
            let keep = pending.or(name);
            tracing::warn!(target: "lab.uat", event = "session_end_failed",
                row = %ctx.row_id, character = keep.as_deref().unwrap_or("-"),
                reason = %e, "fixture session did not end cleanly");
            self.pending_drain = keep;
        }
        r
    }

    async fn end_session_inner(
        &mut self,
        name: &mut Option<String>,
        pending: &mut Option<String>,
        ctx: &mut RowCtx,
    ) -> Result<(), String> {
        let status = self.setup_call("lab_client_status", json!({}), ctx).await?;
        let running = status
            .get("running")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let in_world = status.get("login_state").and_then(Value::as_str) == Some("in_world");
        // A client the run found in world plays the lab character: that is
        // the run's starting point until the runner enters the world itself.
        if name.is_none() && running && in_world {
            name.clone_from(&self.req.lab_character);
        }
        if running {
            let logout = if in_world {
                self.setup_call("lab_logout", json!({}), ctx)
                    .await
                    .map(|_| ())
            } else {
                Ok(())
            };
            // Stop even after a failed logout: a client left running keeps
            // the account online for the next session.
            self.setup_call("lab_client_stop", json!({}), ctx).await?;
            logout?;
        }
        let Some(server) = self.server else {
            return Ok(());
        };
        // After the stop, so the leftover is drained with no client holding
        // any session; before this session's own name.
        if let Some(prev) = pending.as_deref() {
            self.drain_session(server, prev, ctx).await?;
            *pending = None;
        }
        match name.as_deref() {
            Some(n) => self.drain_session(server, n, ctx).await,
            None => Ok(()),
        }
    }

    /// Poll `server_sessions` until no session's name contains `name`.
    async fn drain_session(
        &self,
        server: &dyn ServerInvoker,
        name: &str,
        ctx: &mut RowCtx,
    ) -> Result<(), String> {
        for i in 0..SESSION_DRAIN_MS / SESSION_POLL_MS {
            // Tests skip the sleep but keep the poll count, so a session
            // that never drains still times out.
            if i > 0 && !self.req.no_settle {
                tokio::time::sleep(Duration::from_millis(SESSION_POLL_MS)).await;
            }
            let out = self
                .server_call(server, Role::Setup, "server_sessions", json!({}), ctx)
                .await;
            if !out.ok {
                return Err(format!(
                    "server_sessions: {}",
                    out.error.unwrap_or_default()
                ));
            }
            if !lists_name(&out.json, "sessions", name) {
                return Ok(());
            }
        }
        Err(format!(
            "server still lists {name} after {} s",
            SESSION_DRAIN_MS / 1000
        ))
    }

    /// Put the client in world as the fixture character with `f` applied:
    /// end any earlier session, reach character select, create and
    /// bootstrap the character if it is new, stage, Play, and resolve the
    /// player entity.
    pub(crate) async fn establish_fixture(
        &mut self,
        spec: &SectionSpec,
        f: &FixtureSpec,
        ctx: &mut RowCtx,
    ) -> Result<(), String> {
        let from = ctx.actions.len();
        let r = self.establish_inner(spec, f, ctx).await;
        mark_fixture(ctx, from);
        r
    }

    async fn establish_inner(
        &mut self,
        spec: &SectionSpec,
        f: &FixtureSpec,
        ctx: &mut RowCtx,
    ) -> Result<(), String> {
        let server = self.server.ok_or(NO_ENDPOINT)?;
        let instance = self.req.instance.as_deref().unwrap_or("default");
        let name = fixture_character_name(&f.character, instance);
        ctx.vars.insert(FIXTURE_CHARACTER_VAR.into(), json!(name));
        // `${character}` targets the character in world: the fixture one.
        ctx.vars.insert("character".into(), json!(name));
        ctx.character = json!({ "name": name });

        self.end_session(ctx).await?;
        self.ensure_state(spec, "char_select", ctx).await?;
        let list = self.setup_call("lab_characters", json!({}), ctx).await?;
        if !lists_name(&list, "characters", &name) {
            self.bootstrap(spec, f, &name, ctx).await?;
        }
        ctx.character = self.listed_character_value(&name);

        // Staged at character select, before the one Play: the base applies
        // it as the character enters the world (D-FX1, F7).
        let out = self
            .server_call(
                server,
                Role::Setup,
                "server_stage_fixture",
                stage_args(f, &name),
                ctx,
            )
            .await;
        if !out.ok {
            return Err(format!(
                "stage: {}",
                stage_refusal(&out.error.unwrap_or_default())
            ));
        }

        // Before Play: a Play that errors after the client reached the world
        // must still drain this character, not the lab-character fallback.
        self.in_world_as = Some(name.clone());
        self.setup_call("lab_play_character", json!({ "name": name }), ctx)
            .await?;
        // First thing after Play: content may rewrite the staged rows once
        // the character is in world (FX-07, see `verify_staged`).
        self.verify_staged(f, ctx).await?;
        // A caller's `player_entity_id` names some other session.
        ctx.vars.remove(ENTITY_VAR);
        let entity = self
            .tap_entity(server, ctx)
            .await
            .map_err(|e| format!("player entity: {e}"))?;
        ctx.vars.insert(ENTITY_VAR.into(), json!(entity));
        self.verify_position(f, entity, ctx).await
    }

    /// Create the fixture character and give it its one first-login
    /// session (D-FX5), so its staged sessions start past the intro.
    async fn bootstrap(
        &mut self,
        spec: &SectionSpec,
        f: &FixtureSpec,
        name: &str,
        ctx: &mut RowCtx,
    ) -> Result<(), String> {
        let p = spec
            .section
            .profiles
            .get(&f.character)
            .ok_or_else(|| format!("no [section.profiles.{}]", f.character))?
            .clone();
        // Every fixture character this instance can own in this section, so
        // one profile's bootstrap never deletes another profile's character.
        let instance = self.req.instance.as_deref().unwrap_or("default");
        let protect: Vec<String> = self
            .req
            .lab_character
            .iter()
            .cloned()
            .chain(
                spec.section
                    .profiles
                    .keys()
                    .map(|k| fixture_character_name(k, instance)),
            )
            .collect();
        self.setup_call(
            "lab_ensure_character_slot",
            json!({ "free_slots": 1, "protect": protect }),
            ctx,
        )
        .await?;
        self.setup_call(
            "lab_create_character",
            json!({ "first": p.first, "last": name, "alignment": p.alignment,
                    "archetype": p.archetype, "gender": p.gender }),
            ctx,
        )
        .await?;
        // Before Play, for the same reason as in `establish_inner`.
        self.in_world_as = Some(name.to_string());
        self.setup_call("lab_play_character", json!({ "name": name }), ctx)
            .await?;
        self.first_login_hold(ctx).await;
        let _ = self.setup_call("lab_finish_dialog", json!({}), ctx).await;
        self.end_session(ctx).await?;
        self.ensure_state(spec, "char_select", ctx).await
    }

    /// Wait out the first-login hold, recorded as a `wait` either way so
    /// the row shows it was owed.
    async fn first_login_hold(&mut self, ctx: &mut RowCtx) {
        let skipped = self.req.no_settle;
        let started = now_ms();
        let t0 = std::time::Instant::now();
        if !skipped {
            tokio::time::sleep(Duration::from_millis(FIRST_LOGIN_HOLD_MS)).await;
        }
        ctx.actions.push(ActionRecord {
            index: ctx.actions.len(),
            role: Role::Setup,
            label: Some(FIRST_LOGIN_HOLD_LABEL.into()),
            kind: "wait".into(),
            requested: format!("wait {FIRST_LOGIN_HOLD_MS} ms"),
            tool: None,
            args: Value::Null,
            tier: None,
            tier_source: None,
            fallback_used: false,
            host_started_ms: started,
            elapsed_ms: t0.elapsed().as_millis() as u64,
            ok: true,
            error: None,
            result: json!({ "skipped": skipped }),
            calls: vec![],
            client: None,
        });
    }
}
