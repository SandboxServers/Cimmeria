//! Driving one row once its static checks passed: reach the row's state
//! (its fixture session or the chain), anchor, packet tap and marks,
//! setup, steps with their labelled clauses and evidence, end clauses,
//! teardown, and the fixture session's end. `run_row` (in `mod.rs`) races
//! this against the lease and grades what it left on the context.

use std::collections::HashSet;

use serde_json::json;

use super::players::Who;
use super::{fingerprint, fixture, RowCtx, Runner};
use crate::uat::evidence::ClauseResult;
use crate::uat::invoke::ToolInvoker;
use crate::uat::spec::{RowSpec, SectionSpec, Source};
use crate::uat::tier::Role;

impl<I: ToolInvoker> Runner<'_, I> {
    /// State, anchor, packet tap, setup, steps, clauses, evidence, tap
    /// read, teardown.
    pub(super) async fn drive_row(
        &mut self,
        spec: &SectionSpec,
        row: &RowSpec,
        ctx: &mut RowCtx,
        results: &mut [Option<ClauseResult>],
    ) {
        // Before `ensure_state` puts the client in world: whether this row
        // continues a passing spec neighbour decides if a parity mismatch
        // can fail it (FX-10), in every fixture mode.
        let continues = self.continues_passing_neighbour(spec, row);
        if let Some(f) = &row.fixture {
            if self.applies_fixture(spec, row) {
                fixture::set_mode(ctx, fixture::MODE_FIXTURE);
                if let Err(e) = self.establish_fixture(spec, f, ctx).await {
                    ctx.blocked.push(format!("fixture not established: {e}"));
                    if let Err(e) = self.end_session(ctx).await {
                        ctx.session_end.push(e);
                    }
                    return;
                }
            } else {
                // Continue the chain: the fixture names the state the chain
                // should have produced (parity, FX-10).
                fixture::set_mode(ctx, fixture::MODE_CHAIN);
            }
        }
        // The fixture path left the client in world as the fixture
        // character, and `ctx.character` already names it.
        if ctx.fixture_mode != Some(fixture::MODE_FIXTURE) {
            if let Err(e) = self.ensure_state(spec, &row.state, ctx).await {
                ctx.blocked
                    .push(format!("could not reach state {}: {e}", row.state));
                return;
            }
            ctx.character = self.character_value(spec);
            if ctx.fixture_mode == Some(fixture::MODE_CHAIN) {
                // After `ensure_state`, so `never` mode names the character
                // the row really plays even on a run's first row.
                if let Some(n) = self.in_world_as.clone() {
                    ctx.vars
                        .insert(crate::uat::fixture::FIXTURE_CHARACTER_VAR.into(), json!(n));
                }
                self.check_parity(row, continues, ctx).await; // Before setup (FX-10).
            }
        }
        if row.players == 2 {
            if let Err(e) = self.ensure_p2(ctx).await {
                ctx.blocked
                    .push(format!("could not bring p2 in world: {e}"));
                return;
            }
        }
        if row.state == "in_world" && row.anchor.unwrap_or(true) {
            self.anchor(ctx).await;
        }
        // A row fixture (FX-06) is established before this point: the tap
        // starts after it, then the steps.
        let packet = row.expect.iter().any(|c| c.source == Source::Packet);
        let fp = self.fp_wanted(row);
        let tapped = packet || fp;
        ctx.fp = fp.then(|| fingerprint::FpCapture::new(self.server.is_none(), !packet));
        if packet {
            self.tap_start(ctx).await;
        }
        self.fp_start(ctx).await;
        self.event_marks_at(row, None, ctx).await;
        let setup_ok = self.drive_steps(row, ctx, results).await;
        // Read and stop the tap before teardown, and on the failed-setup
        // path too: a tap left running would keep buffering this session.
        if tapped {
            self.fp_before_tap_read(ctx).await;
            self.tap_finish(ctx).await;
            self.packet_clauses(row, ctx, results);
        }
        if setup_ok {
            for a in &row.teardown {
                let rec = self.exec(a, Role::Teardown, ctx).await;
                ctx.actions.push(rec);
            }
        }
        self.fp_write(ctx);
        // The session ends with the row (D-FX5), on the failed-setup path
        // too. A lease takeover drops this future before here; the next
        // fixture row's `establish_fixture` ends that session instead.
        if ctx.fixture_mode == Some(fixture::MODE_FIXTURE) {
            if let Err(e) = self.end_session(ctx).await {
                ctx.session_end.push(e);
            }
        }
    }

    /// Setup, steps, clauses and evidence. False when setup failed (the
    /// row is BLOCKED and its teardown does not run).
    async fn drive_steps(
        &mut self,
        row: &RowSpec,
        ctx: &mut RowCtx,
        results: &mut [Option<ClauseResult>],
    ) -> bool {
        for (i, a) in row.setup.iter().enumerate() {
            let rec = self.exec(a, Role::Setup, ctx).await;
            let failed = !rec.ok && !a.optional;
            let what = rec.requested.clone();
            let err = rec.error.clone().unwrap_or_default();
            ctx.actions.push(rec);
            self.fp_after_action(a, "setup", i, ctx).await;
            if failed {
                ctx.blocked.push(format!("setup {what} failed: {err}"));
                return false;
            }
        }
        // Chat marks per client: each chat clause reads its own client's box.
        let chat: Vec<(Who, Option<&str>)> = row
            .expect
            .iter()
            .filter(|c| c.source == Source::Chat)
            .map(|c| (Who::of(c.client.as_deref()), c.since.as_deref()))
            .collect();
        let mut readers: Vec<Who> = Vec::new();
        for (who, _) in &chat {
            // Not `dedup`: it drops only neighbours, and a second read of
            // the same client would move its mark past lines to count.
            if !readers.contains(who) {
                readers.push(*who);
            }
        }
        for who in &readers {
            let tail = self.read_chat_of(*who).await.unwrap_or_default();
            ctx.chat_marks.insert(who.mark(""), tail);
        }
        let since: HashSet<(Who, &str)> = chat
            .iter()
            .filter_map(|(w, s)| s.map(|s| (*w, s)))
            .collect();
        for (i, a) in row.steps.iter().enumerate() {
            // `since = <label>` means "lines that arrived after this action
            // started": mark before it runs, so a fast reply is not missed.
            if let Some(label) = &a.label {
                for who in [Who::P1, Who::P2] {
                    if since.contains(&(who, label.as_str())) {
                        let tail = self.read_chat_of(who).await.unwrap_or_default();
                        ctx.chat_marks.insert(who.mark(label), tail);
                    }
                }
                self.event_marks_at(row, Some(label), ctx).await;
            }
            let rec = self.exec(a, Role::Step, ctx).await;
            let stop = !rec.ok && !a.optional;
            ctx.actions.push(rec);
            self.fp_after_action(a, "step", i, ctx).await;
            if let Some(label) = &a.label {
                self.clauses_at(row, Some(label), ctx, results).await;
                self.evidence_at(row, Some(label), ctx).await;
            }
            if stop {
                break;
            }
        }
        self.clauses_at(row, None, ctx, results).await;
        self.evidence_at(row, None, ctx).await;
        self.capture_final(ctx).await;
        if row.players == 2 {
            self.capture_final_p2(ctx).await;
        }
        true
    }

    /// Clauses whose `at` is `label` (or unset, for `None`), in spec order.
    /// Packet clauses wait for the tap read ([`Self::packet_clauses`]).
    async fn clauses_at(
        &mut self,
        row: &RowSpec,
        label: Option<&String>,
        ctx: &mut RowCtx,
        results: &mut [Option<ClauseResult>],
    ) {
        for (i, c) in row.expect.iter().enumerate() {
            if c.at.as_ref() == label && results[i].is_none() && c.source != Source::Packet {
                results[i] = Some(self.eval_clause(c, ctx).await);
            }
        }
    }
}
