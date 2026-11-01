//! One row, start to finish: the per-row driver that builds `RowEvidence`.

use std::collections::HashMap;

use serde_json::{json, Value};

use super::{now_ms, revocation, utc_of, RowCtx, Runner};
use crate::uat::evidence::{ClauseResult, RowEvidence, BUNDLE_SCHEMA};
use crate::uat::grade::{grade, mark_unverified};
use crate::uat::invoke::ToolInvoker;
use crate::uat::spec::{RowSpec, SectionSpec};

impl<I: ToolInvoker> Runner<'_, I> {
    /// One row, start to finish. Never returns an error: every problem
    /// becomes a reason on the row.
    pub(super) async fn run_row(
        &mut self,
        spec: &SectionSpec,
        row: &RowSpec,
        prior_blocks: Vec<String>,
    ) -> RowEvidence {
        let mut ctx = RowCtx {
            section: spec.section.id.clone(),
            row_id: row.id.clone(),
            vars: self.base_vars(spec, row),
            actions: vec![],
            blocked: vec![],
            unverified: vec![],
            inconclusive: vec![],
            chat_marks: HashMap::new(),
            attachments: vec![],
            character: json!({ "name": self.character_name(spec) }),
            anchor: None,
            started_ms: now_ms(),
            tap: None,
            event_marks: HashMap::new(),
            fp: None,
            fixture_mode: None,
            session_end: vec![],
            parity: None,
            poses: row.pose.clone(),
            log_mark: None,
            chaos: Default::default(),
        };
        ctx.blocked = self.static_blocks(spec, row);
        ctx.blocked.extend(prior_blocks);
        ctx.blocked.extend(self.chaos_blocks(row));
        if let Some(r) = self.revocation_reason() {
            // The lease went before this row started: drive nothing.
            ctx.blocked.push(format!("{}: {r}", revocation::REVOKED));
        }
        let mut results: Vec<Option<ClauseResult>> = vec![None; row.expect.len()];

        let planned = self.req.plan_only && ctx.blocked.is_empty();
        if ctx.blocked.is_empty() && !self.req.plan_only {
            // Race the row against the lease: a takeover cuts it off at its
            // next await (tool call, wait_ms sleep, server read).
            let rx = self.revoked.clone();
            let lost = tokio::select! {
                _ = self.drive_row(spec, row, &mut ctx, &mut results) => None,
                r = revocation::revoked(rx) => Some(r),
            };
            if let Some(r) = lost {
                ctx.blocked.push(format!("{}: {r}", revocation::REVOKED));
                // The cut-off row's tap and capture are still in `ctx`.
                self.tap_abandon(&mut ctx).await;
                self.chaos_clear(&mut ctx).await; // Not lease-gated, like the tap.
            }
        }

        let clauses: Vec<ClauseResult> = results.into_iter().flatten().collect();
        let mut g = grade(&ctx.blocked, &ctx.actions, &clauses, row.required_native);
        mark_unverified(&mut g, &ctx.unverified);
        super::verdict::settle(&mut g, &ctx.inconclusive, &clauses);
        // A failed session end does not change what the row proved, but the
        // next login may meet the character still online: show it.
        g.flags
            .extend(ctx.session_end.iter().map(|e| format!("session end: {e}")));
        g.flags.extend(ctx.chaos.errors.iter().cloned());
        crate::uat::snapshot::apply_parity(&mut g, ctx.parity.as_ref());
        // Plan only: a row that could run is SKIPPED, not graded.
        let result = if planned {
            g.reasons = vec!["plan only: ready to run, nothing was driven".into()];
            crate::uat::evidence::RowResult::Skipped
        } else {
            g.result
        };
        let ended = now_ms();
        RowEvidence {
            schema: BUNDLE_SCHEMA,
            run_id: self.manifest.run_id.clone(),
            section: spec.section.id.clone(),
            system: spec.section.system.clone(),
            row_id: row.id.clone(),
            title: row.title.clone(),
            guide_ref: spec.section.guide.clone(),
            ledger_ref: spec.section.ledger.clone(),
            expected: row.expected.trim().to_string(),
            result,
            reasons: g.reasons,
            flags: g.flags,
            required_native: row.required_native,
            native_used: g.native_used,
            character: ctx.character,
            account_kind: spec.section.account.clone(),
            started_utc: utc_of(ctx.started_ms),
            ended_utc: utc_of(ended),
            host_started_ms: ctx.started_ms,
            host_ended_ms: ended,
            anchor: ctx.anchor,
            actions: ctx.actions,
            clauses,
            attachments: ctx.attachments,
            known_issues: row.known_issues.clone(),
            relog: row.relog.clone(),
            notes: row.notes.clone(),
            vars: Value::Object(ctx.vars),
            attestations: vec![],
            blocked: ctx.blocked,
            unverified: ctx.unverified,
            fixture_parity: ctx.parity,
        }
    }
}
