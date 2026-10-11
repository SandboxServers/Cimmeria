//! The runner's half of fixture parity (lab-fixtures FX-10, D-FX4, D-FX10):
//! a row that continues the chain instead of staging its fixture is
//! snapshotted before its setup, and the snapshot is compared with the
//! fixture. The pure half (SQL, parsing, the comparison, the grade change)
//! is `uat::snapshot`.
//!
//! The calls go through `server_call`, so they are on the row's record, and
//! are re-marked `kind = "fixture"` like the rest of the fixture plumbing
//! (lab-golden strips them; a fixture-mode run of the same row makes none).

use std::time::Duration;

use serde_json::{json, Value};

use super::fixture::mark_fixture;
use super::fixture_verify::session_world;
use super::{now_ms, RowCtx, Runner};
use crate::uat::evidence::Attachment;
use crate::uat::fixture::FIXTURE_CHARACTER_VAR;
use crate::uat::invoke::ToolInvoker;
use crate::uat::snapshot::{from_rows, parity, snapshot_sql, Parity, RowSnapshot};
use crate::uat::spec::{FixtureSpec, RowSpec};
use crate::uat::tier::Role;

/// What each [`snapshot_sql`] statement reads, for an error.
const PARTS: [&str; 3] = ["missions", "items", "tutorials"];
/// Re-reads after a mismatching snapshot, and the wait before each: the
/// previous row's last writes (a mission step, a granted item) can still
/// be on their way to the database when the row ends.
pub(crate) const SETTLE_READS: usize = 3;
const SETTLE_MS: u64 = 500;

impl<I: ToolInvoker> Runner<'_, I> {
    /// On the chain path, before setup: snapshot the chain character and
    /// store its parity with the row's fixture on `ctx`. A mismatch is
    /// re-read up to [`SETTLE_READS`] times; the last read is kept. A
    /// snapshot that fails is kept as `snapshot_error`, which never changes
    /// the result. `binding` is [`Self::continues_passing_neighbour`]:
    /// without it a mismatch is advisory (a flag).
    pub(crate) async fn check_parity(&mut self, row: &RowSpec, binding: bool, ctx: &mut RowCtx) {
        let Some(f) = &row.fixture else {
            return;
        };
        let from = ctx.actions.len();
        let mut reads = 1;
        let (mut p, mut snap) = match self.snapshot_row(f, ctx).await {
            Ok(snap) => (parity(f, &snap), Some(snap)),
            Err(e) => (
                Parity {
                    snapshot_error: Some(e),
                    ..Default::default()
                },
                None,
            ),
        };
        while snap.is_some() && !p.mismatches.is_empty() && reads <= SETTLE_READS {
            // Tests skip the wait but keep the read count.
            if !self.req.no_settle {
                tokio::time::sleep(Duration::from_millis(SETTLE_MS)).await;
            }
            reads += 1;
            match self.snapshot_row(f, ctx).await {
                Ok(s) => {
                    p = parity(f, &s);
                    snap = Some(s);
                }
                Err(e) => {
                    // Keep the last snapshot that was read: its mismatch is
                    // still what the chain showed.
                    tracing::warn!(target: "lab.uat", event = "fixture_parity_reread_failed",
                        row = %ctx.row_id, reads, reason = %e, "chain re-snapshot failed");
                    break;
                }
            }
        }
        p.advisory = !binding;
        if let Some(s) = &snap {
            self.write_snapshot(s, ctx);
        }
        mark_fixture(ctx, from);
        if let Some(e) = &p.snapshot_error {
            tracing::warn!(target: "lab.uat", event = "fixture_parity_unchecked",
                row = %ctx.row_id, reason = %e, "chain snapshot failed");
        } else if !p.mismatches.is_empty() {
            tracing::warn!(target: "lab.uat", event = "fixture_parity_mismatch",
                row = %ctx.row_id, reads, advisory = p.advisory,
                mismatches = %p.mismatches.join("; "),
                "chain state differs from the row's fixture");
        }
        ctx.parity = Some(p);
    }

    /// The chain character's missions, carried items, seen tutorials and
    /// world. The session list is only required when the fixture names a
    /// world; otherwise a failed read leaves `world` unset.
    pub(crate) async fn snapshot_row(
        &mut self,
        f: &FixtureSpec,
        ctx: &mut RowCtx,
    ) -> Result<RowSnapshot, String> {
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
        if out.ok {
            snap.world = session_world(&out.json, &name).as_str().map(str::to_string);
        } else if f.world.is_some() {
            return Err(format!("sessions: {}", out.error.unwrap_or_default()));
        }
        snap.host_ms = now_ms();
        Ok(snap)
    }

    /// Write `rows/<section>/<row>.snapshot.json` and attach it. A failed
    /// write loses only the file: the parity already computed stands.
    fn write_snapshot(&self, snap: &RowSnapshot, ctx: &mut RowCtx) {
        let path = self.run.snapshot_json(&ctx.section, &ctx.row_id);
        if let Err(e) = self.run.write_json(&path, snap) {
            tracing::warn!(target: "lab.uat", event = "fixture_snapshot_unwritten",
                row = %ctx.row_id, reason = %e, "chain snapshot file not written");
            return;
        }
        ctx.attachments.push(Attachment {
            name: "snapshot".into(),
            path: self.rel(&path),
            tool: "server_db_query".into(),
            host_ms: snap.host_ms,
        });
    }
}
