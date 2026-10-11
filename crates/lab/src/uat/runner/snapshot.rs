//! The runner's half of fixture parity (lab-fixtures FX-10, D-FX4, D-FX10):
//! a row that continues the chain instead of staging its fixture is
//! snapshotted before its setup, and the snapshot is compared with the
//! fixture. The pure half (SQL, parsing, the comparison, the grade change)
//! is `uat::snapshot`.
//!
//! The calls go through `server_call`, so they are on the row's record, and
//! are re-marked `kind = "fixture"` like the rest of the fixture plumbing
//! (lab-golden strips them; a fixture-mode run of the same row makes none).

use serde_json::{json, Value};

use super::fixture::mark_fixture;
use super::fixture_verify::session_world;
use super::{now_ms, RowCtx, Runner};
use crate::uat::evidence::Attachment;
use crate::uat::fixture::FIXTURE_CHARACTER_VAR;
use crate::uat::invoke::ToolInvoker;
use crate::uat::snapshot::{from_rows, parity, snapshot_sql, Parity, RowSnapshot};
use crate::uat::spec::RowSpec;
use crate::uat::tier::Role;

/// What each [`snapshot_sql`] statement reads, for an error.
const PARTS: [&str; 3] = ["missions", "items", "tutorials"];

impl<I: ToolInvoker> Runner<'_, I> {
    /// On the chain path, before setup: snapshot the chain character and
    /// store its parity with the row's fixture on `ctx`. A snapshot that
    /// fails is kept as `snapshot_error`, which never changes the result.
    pub(crate) async fn check_parity(&mut self, row: &RowSpec, ctx: &mut RowCtx) {
        let Some(f) = &row.fixture else {
            return;
        };
        let from = ctx.actions.len();
        let p = match self.snapshot_row(ctx).await {
            Ok(snap) => parity(f, &snap),
            Err(e) => Parity {
                snapshot_error: Some(e),
                ..Default::default()
            },
        };
        mark_fixture(ctx, from);
        if let Some(e) = &p.snapshot_error {
            tracing::warn!(target: "lab.uat", event = "fixture_parity_unchecked",
                row = %ctx.row_id, reason = %e, "chain snapshot failed");
        } else if !p.mismatches.is_empty() {
            tracing::warn!(target: "lab.uat", event = "fixture_parity_mismatch",
                row = %ctx.row_id, mismatches = %p.mismatches.join("; "),
                "chain state differs from the row's fixture");
        }
        ctx.parity = Some(p);
    }

    /// The chain character's missions, carried items, seen tutorials and
    /// world, written to `rows/<section>/<row>.snapshot.json`.
    pub(crate) async fn snapshot_row(&mut self, ctx: &mut RowCtx) -> Result<RowSnapshot, String> {
        let server = self
            .server
            .ok_or("no server lab MCP (CIMMERIA_LAB_MCP_URL/_TOKEN) to snapshot the chain")?;
        let name = ctx
            .vars
            .get(FIXTURE_CHARACTER_VAR)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or("no chain character to snapshot")?;
        let mut replies = Vec::with_capacity(PARTS.len());
        for (what, sql) in PARTS.iter().zip(snapshot_sql(&name)?) {
            let out = self
                .server_call(
                    server,
                    Role::Setup,
                    "server_db_query",
                    json!({ "sql": sql }),
                    ctx,
                )
                .await;
            if !out.ok {
                return Err(format!("{what}: {}", out.error.unwrap_or_default()));
            }
            replies.push(out.json);
        }
        let mut snap = from_rows(&name, &replies[0], &replies[1], &replies[2])?;
        let out = self
            .server_call(server, Role::Setup, "server_sessions", json!({}), ctx)
            .await;
        if !out.ok {
            return Err(format!("sessions: {}", out.error.unwrap_or_default()));
        }
        snap.world = session_world(&out.json, &name).as_str().map(str::to_string);
        snap.host_ms = now_ms();
        let path = self.run.snapshot_json(&ctx.section, &ctx.row_id);
        self.run.write_json(&path, &snap)?;
        ctx.attachments.push(Attachment {
            name: "snapshot".into(),
            path: self.rel(&path),
            tool: "server_db_query".into(),
            host_ms: snap.host_ms,
        });
        Ok(snap)
    }
}
