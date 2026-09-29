//! Row grading. Pure: the runner and `lab_uat_attest` both call
//! [`grade`] on the recorded actions and clauses, so a re-grade after an
//! attestation follows the same rules as the live run.
//!
//! The rules, in precedence order (every applicable reason is listed):
//!
//! 1. **BLOCKED** — the row could not reach its step: a standing
//!    `blocked` reason, a missing tool (named), a second player, a colo
//!    rule-6 command without the owner's say-so, a setup action that
//!    failed, or a required clause whose reader tool is missing.
//! 2. **FAIL** — a required clause's observation contradicts it, or a
//!    step action errored.
//! 3. **UNVERIFIED** — a required clause is still pending (SigNoz not yet
//!    attested) or could not be checked (endpoint unreachable).
//! 4. **NATIVE_SHORTFALL** — everything held, but a step action ran below
//!    `required_native` (an N3 fallback, a server shortcut). Setup and
//!    teardown may use GM (`G`) and server (`X`) shortcuts; those are
//!    flagged, never graded.
//! 5. **NEEDS_HUMAN** — every automatic clause passed; a required human
//!    question is unanswered.
//! 6. **PASS** — none of the above.

use super::evidence::{ActionRecord, ClauseResult, RowResult, Verdict};
use super::tier::{Role, Tier};

/// The grade and every reason it is not PASS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grade {
    pub result: RowResult,
    pub reasons: Vec<String>,
    /// Informational: setup that used a server shortcut, optional actions
    /// that failed. Never affects the result.
    pub flags: Vec<String>,
    pub native_used: Option<Tier>,
}

/// Grade one row. `blocked` holds the runner's own blocking reasons
/// (missing tools, players, colo rule, setup errors).
pub fn grade(
    blocked: &[String],
    actions: &[ActionRecord],
    clauses: &[ClauseResult],
    required_native: Tier,
) -> Grade {
    let mut reasons: Vec<String> = blocked.to_vec();
    let mut flags = Vec::new();

    let steps: Vec<&ActionRecord> = actions.iter().filter(|a| a.role == Role::Step).collect();
    let native_used = steps.iter().filter_map(|a| a.tier).max();

    let mut step_failed = false;
    for a in &steps {
        if !a.ok {
            step_failed = true;
            reasons.push(format!(
                "step action {} ({}) failed: {}",
                a.index,
                a.requested,
                a.error.as_deref().unwrap_or("error")
            ));
        }
    }
    for a in actions.iter().filter(|a| a.role != Role::Step) {
        if a.tier == Some(Tier::X) {
            flags.push(format!(
                "{:?} used a server shortcut: {}",
                a.role, a.requested
            ));
        }
        if !a.ok && a.role == Role::Teardown {
            flags.push(format!(
                "teardown {} failed: {}",
                a.requested,
                a.error.as_deref().unwrap_or("error")
            ));
        }
    }

    let mut shortfall = false;
    for a in &steps {
        if let Some(t) = a.tier {
            if !t.satisfies(required_native) {
                shortfall = true;
                reasons.push(format!(
                    "step {} ran at {} (row needs {}){}",
                    a.requested,
                    t.as_str(),
                    required_native.as_str(),
                    if a.fallback_used {
                        ", via a fallback"
                    } else {
                        ""
                    }
                ));
            }
        }
    }

    let any = |v: Verdict| {
        let hits: Vec<&ClauseResult> = clauses
            .iter()
            .filter(|c| c.required && c.verdict == v)
            .collect();
        hits
    };
    let failed = any(Verdict::Fail);
    let clause_blocked = any(Verdict::Blocked);
    let pending: Vec<&ClauseResult> = any(Verdict::Pending)
        .into_iter()
        .chain(any(Verdict::Unverified))
        .collect();
    let human = any(Verdict::NeedsHuman);
    for c in &clause_blocked {
        reasons.push(format!(
            "clause {} blocked: {}",
            c.id,
            c.detail.as_deref().unwrap_or("reader missing")
        ));
    }
    for c in &failed {
        reasons.push(format!("clause {} failed: {}", c.id, c.expected));
    }
    for c in &pending {
        reasons.push(format!(
            "clause {} {}: {}",
            c.id,
            if c.verdict == Verdict::Pending {
                "pending attestation"
            } else {
                "unverified"
            },
            c.detail.as_deref().unwrap_or(&c.expected)
        ));
    }
    for c in &human {
        reasons.push(format!("needs a human: {}", c.expected));
    }

    let result = if !blocked.is_empty() || !clause_blocked.is_empty() {
        RowResult::Blocked
    } else if !failed.is_empty() || step_failed {
        RowResult::Fail
    } else if !pending.is_empty() {
        RowResult::Unverified
    } else if shortfall {
        RowResult::NativeShortfall
    } else if !human.is_empty() {
        RowResult::NeedsHuman
    } else {
        RowResult::Pass
    };
    Grade {
        result,
        reasons,
        flags,
        native_used,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uat::spec::Source;
    use serde_json::Value;

    fn action(role: Role, tier: Tier, ok: bool, fallback: bool) -> ActionRecord {
        ActionRecord {
            index: 0,
            role,
            label: None,
            kind: "tool".into(),
            requested: "client_world_click".into(),
            tool: Some("client_lua_eval".into()),
            args: Value::Null,
            tier: Some(tier),
            tier_source: Some("tool".into()),
            fallback_used: fallback,
            host_started_ms: 0,
            elapsed_ms: 1,
            ok,
            error: (!ok).then(|| "boom".to_string()),
            result: Value::Null,
            calls: vec![],
        }
    }

    fn clause(v: Verdict, required: bool) -> ClauseResult {
        ClauseResult {
            id: "c".into(),
            text: "t".into(),
            source: Source::Chat,
            required,
            expected: "e".into(),
            verdict: v,
            observed: Value::Null,
            detail: None,
            evaluated_ms: 0,
            query: None,
            evidence_refs: vec![],
        }
    }

    #[test]
    fn a_clean_native_row_passes() {
        let g = grade(
            &[],
            &[action(Role::Step, Tier::N1, true, false)],
            &[clause(Verdict::Pass, true)],
            Tier::N1,
        );
        assert_eq!(g.result, RowResult::Pass);
        assert!(g.reasons.is_empty());
    }

    /// The core guard: an N3 fallback on a step never grades PASS, even
    /// when every clause held.
    #[test]
    fn a_step_fallback_below_required_native_is_not_a_pass() {
        let g = grade(
            &[],
            &[action(Role::Step, Tier::N3, true, true)],
            &[clause(Verdict::Pass, true)],
            Tier::N1,
        );
        assert_eq!(g.result, RowResult::NativeShortfall);
        assert!(g.reasons[0].contains("via a fallback"), "{:?}", g.reasons);
        let g = grade(
            &[],
            &[action(Role::Step, Tier::X, true, false)],
            &[clause(Verdict::Pass, true)],
            Tier::N2,
        );
        assert_eq!(g.result, RowResult::NativeShortfall);
    }

    #[test]
    fn setup_shortcuts_are_flagged_not_graded() {
        let acts = [
            action(Role::Setup, Tier::X, true, false),
            action(Role::Step, Tier::N1, true, false),
        ];
        let g = grade(&[], &acts, &[clause(Verdict::Pass, true)], Tier::N1);
        assert_eq!(g.result, RowResult::Pass);
        assert_eq!(g.flags.len(), 1);
    }

    #[test]
    fn an_unattested_required_clause_is_never_a_pass() {
        let g = grade(
            &[],
            &[action(Role::Step, Tier::N1, true, false)],
            &[clause(Verdict::Pass, true), clause(Verdict::Pending, true)],
            Tier::N1,
        );
        assert_eq!(g.result, RowResult::Unverified);
        // An optional pending clause does not hold the row back.
        let g = grade(
            &[],
            &[action(Role::Step, Tier::N1, true, false)],
            &[clause(Verdict::Pass, true), clause(Verdict::Pending, false)],
            Tier::N1,
        );
        assert_eq!(g.result, RowResult::Pass);
    }

    #[test]
    fn precedence_blocked_then_fail_then_human() {
        let pass_step = [action(Role::Step, Tier::N1, true, false)];
        let g = grade(
            &["needs client_cache_files".into()],
            &pass_step,
            &[clause(Verdict::Fail, true)],
            Tier::N1,
        );
        assert_eq!(g.result, RowResult::Blocked);
        let g = grade(
            &[],
            &pass_step,
            &[
                clause(Verdict::Fail, true),
                clause(Verdict::NeedsHuman, true),
            ],
            Tier::N1,
        );
        assert_eq!(g.result, RowResult::Fail);
        let g = grade(
            &[],
            &pass_step,
            &[
                clause(Verdict::Pass, true),
                clause(Verdict::NeedsHuman, true),
            ],
            Tier::N1,
        );
        assert_eq!(g.result, RowResult::NeedsHuman);
        let g = grade(
            &[],
            &[action(Role::Step, Tier::N1, false, false)],
            &[clause(Verdict::Pass, true)],
            Tier::N1,
        );
        assert_eq!(g.result, RowResult::Fail);
        let g = grade(&[], &pass_step, &[clause(Verdict::Blocked, true)], Tier::N1);
        assert_eq!(g.result, RowResult::Blocked);
    }
}
