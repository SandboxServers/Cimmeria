//! Client event clauses (ability-mechanics AB-L3): the client's own
//! telemetry events (`client.ability.press`, `.sent`, `.recv`,
//! `.applied`, `.shown`, and any other `client.*` target the DLL pushes
//! to the lab ring), read from the lab's event store.
//!
//! The store is the supervisor's, and it is read only through
//! `client_wait_event` with an explicit `since_seq`, never through
//! `client_events_read` (that tool's own cursor belongs to whoever drives
//! the lab, and draining it here would steal their events). Each client
//! the row's client_event clauses read gets a mark at the row start (after
//! the anchor) and before every step a clause names in `since`.
//!
//! A clause is graded like a packet clause: `min_rows` / `max_rows` on
//! the matching events (default at least one) and `field` `op` `value` on
//! every one. Before grading, the runner waits up to `timeout_ms` for
//! `min_rows` events, or for one more than `max_rows`, so a late event is
//! neither missed nor wrongly absent.

use serde_json::{json, Value};

use super::actions::subst;
use super::players::Who;
use super::{RowCtx, Runner};
use crate::uat::clause::grade_signoz;
use crate::uat::evidence::{clip, ClauseResult, Verdict};
use crate::uat::invoke::ToolInvoker;
use crate::uat::spec::{ExpectSpec, RowSpec, Source};

/// The store reader every client_event clause goes through.
pub(crate) const WAIT_EVENT_TOOL: &str = "client_wait_event";
/// The runner's own named cursor on the store (arm only moves this one).
const EVENT_CURSOR: &str = "uat";
/// How long a clause waits for its events by default.
const DEFAULT_TIMEOUT_MS: u64 = 5000;
/// Matches collected for grading (the store's scan window).
const COLLECT_MAX: u64 = 8192;

/// The lab ring's kind for a telemetry target: the DLL pushes
/// `client.ability.sent` as `ability.sent`.
pub(crate) fn ring_kind(event: &str) -> &str {
    event.strip_prefix("client.").unwrap_or(event)
}

/// One stored event flattened for grading: the event's own fields at the
/// top level (they win: `client.ability.applied` has its own `kind`),
/// with the store's columns beside them as `store_seq`, `store_kind` and
/// `store_ts_ms`.
pub fn event_row(e: &Value) -> Value {
    let mut out = match e.get("fields") {
        Some(Value::Object(f)) => f.clone(),
        _ => serde_json::Map::new(),
    };
    for (col, key) in [
        ("seq", "store_seq"),
        ("kind", "store_kind"),
        ("ts_ms", "store_ts_ms"),
    ] {
        if let Some(v) = e.get(col) {
            out.insert(key.into(), v.clone());
        }
    }
    Value::Object(out)
}

/// What may have been lost between the mark and the read, gathered
/// independently of the clause's own match: the client throttle reports a
/// suppressed press only as a `suppressed` count on the *next* row of its
/// family (a `client.ability.press`), and the press's `sent` / `sent_seq`
/// answers are dropped with it without a count of their own (D-AU5,
/// `ability_trace::throttle`). So a clause on `client.ability.sent` must
/// look at every `client.ability.*` row in the window, not its matches.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Loss {
    /// The store evicted events after the mark.
    pub gap: bool,
    /// Sum of `suppressed` over every row of the clause's family.
    pub suppressed: u64,
    /// Events the bridge ring dropped before the store saw them.
    pub upstream_dropped: u64,
}

impl Loss {
    fn why(self) -> Option<String> {
        let mut parts = Vec::new();
        if self.gap {
            parts.push("the event store evicted events after the mark".to_string());
        }
        if self.suppressed > 0 {
            parts.push(format!(
                "the client throttle suppressed {} event(s) of this family",
                self.suppressed
            ));
        }
        if self.upstream_dropped > 0 {
            parts.push(format!(
                "the bridge ring dropped {} event(s)",
                self.upstream_dropped
            ));
        }
        (!parts.is_empty()).then(|| parts.join("; "))
    }
}

/// The kinds whose throttle governs `kind`: the whole `ability.*` family
/// for an ability row (a press and its answers share one decision), else
/// the kind itself.
pub(crate) fn throttle_family(kind: &str) -> String {
    match kind.split_once('.') {
        Some(("ability", _)) => "ability.*".into(),
        _ => kind.to_string(),
    }
}

/// `suppressed` summed over a family read's matched rows.
pub fn suppressed_in(rows: &[Value]) -> u64 {
    rows.iter()
        .filter_map(|e| e.get("fields").and_then(|f| f.get("suppressed")))
        .filter_map(Value::as_u64)
        .sum()
}

/// Grade a client_event clause from the matched events. A PASS that rests
/// on an upper bound or on every row is UNVERIFIED when events may be
/// missing ([`Loss`]): the missing ones were never checked. The throttle
/// reports a suppression only on a later row, so a burst at the very end
/// of the window with nothing after it cannot be seen; a clause that
/// bounds a burst of more than 8 presses a second should not be written.
pub fn grade_events(
    c: &ExpectSpec,
    matched: &[Value],
    loss: Loss,
) -> (Verdict, Value, Option<String>) {
    let rows: Vec<Value> = matched.iter().map(event_row).collect();
    let observed = json!({
        "matching_events": rows.len(),
        "gap": loss.gap,
        "family_suppressed": loss.suppressed,
        "upstream_dropped": loss.upstream_dropped,
        "events": clip(&json!(rows.iter().take(5).collect::<Vec<_>>()), 1500),
    });
    let verdict = match grade_signoz(c, rows.len() as u64, &rows) {
        Ok(v) => v,
        Err(e) => return (Verdict::Unverified, observed, Some(e)),
    };
    let bounded = c.max_rows.is_some() || c.field.is_some();
    if let (Verdict::Pass, true, Some(why)) = (verdict, bounded, loss.why()) {
        return (
            Verdict::Unverified,
            observed,
            Some(format!(
                "{why}, so an upper bound or an every-event check cannot be proven"
            )),
        );
    }
    (verdict, observed, None)
}

impl<I: ToolInvoker> Runner<'_, I> {
    /// Mark the store head for each client a client_event clause reads
    /// from `label` (`None`: the row start). Done before the labelled step
    /// runs, so nothing it causes is missed.
    pub(crate) async fn event_marks_at(
        &mut self,
        row: &RowSpec,
        label: Option<&String>,
        ctx: &mut RowCtx,
    ) {
        // One arm per client: a second arm would move the mark past events
        // the first one's clauses must see (`dedup` only drops neighbours).
        let mut readers: Vec<Who> = Vec::new();
        for c in &row.expect {
            let who = Who::of(c.client.as_deref());
            if c.source == Source::ClientEvent
                && c.since.as_ref() == label
                && !readers.contains(&who)
            {
                readers.push(who);
            }
        }
        for who in readers {
            let key = who.mark(label.map_or("", String::as_str));
            let inv = self.on(who);
            let mark = if inv.has_tool(WAIT_EVENT_TOOL) {
                let out = inv
                    .call(
                        WAIT_EVENT_TOOL,
                        json!({ "arm": true, "cursor": EVENT_CURSOR }),
                    )
                    .await;
                match out.json.pointer("/cursor/seq").and_then(Value::as_u64) {
                    Some(seq) if out.ok => Ok(seq),
                    _ => Err(format!(
                        "{WAIT_EVENT_TOOL} arm: {}",
                        out.error.unwrap_or_else(|| "no cursor seq".into())
                    )),
                }
            } else {
                Err(format!("tool {WAIT_EVENT_TOOL} is not routed"))
            };
            ctx.event_marks.insert(key, mark);
        }
    }

    /// Evaluate one client_event clause into `r`.
    pub(crate) async fn eval_client_event(
        &mut self,
        who: Who,
        c: &ExpectSpec,
        ctx: &mut RowCtx,
        r: &mut ClauseResult,
    ) {
        let key = who.mark(c.since.as_deref().unwrap_or_default());
        let start = match ctx.event_marks.get(&key) {
            Some(Ok(seq)) => *seq,
            Some(Err(e)) => {
                r.detail = Some(format!("no event mark: {e}"));
                return;
            }
            None => {
                r.detail = Some("no event mark was taken for this clause".into());
                return;
            }
        };
        let kind = ring_kind(c.event.as_deref().unwrap_or_default()).to_string();
        let fields = c
            .match_fields
            .as_ref()
            .map(|m| subst(&Value::Object(m.clone()), &ctx.vars))
            .unwrap_or_else(|| json!({}));
        let query = |count: u64, timeout_ms: u64| {
            json!({
                "kind": kind, "fields": fields, "since_seq": start,
                "count": count, "timeout_ms": timeout_ms, "cursor": EVENT_CURSOR,
            })
        };
        let inv = self.on(who);
        // Wait for enough events to decide (one past max_rows proves FAIL).
        let min = c
            .min_rows
            .unwrap_or(if c.max_rows.is_some() { 0 } else { 1 });
        let want = c.max_rows.map_or(min, |m| m + 1);
        if want > 0 {
            let timeout = c.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
            let out = inv.call(WAIT_EVENT_TOOL, query(want, timeout)).await;
            if !out.ok {
                r.detail = Some(format!(
                    "{WAIT_EVENT_TOOL}: {}",
                    out.error.unwrap_or_default()
                ));
                return;
            }
        }
        // Then take everything that matched since the mark, in one scan.
        let out = inv.call(WAIT_EVENT_TOOL, query(COLLECT_MAX, 0)).await;
        if !out.ok {
            r.detail = Some(format!(
                "{WAIT_EVENT_TOOL}: {}",
                out.error.unwrap_or_default()
            ));
            return;
        }
        let Some(matched) = out.json.get("matched").and_then(Value::as_array) else {
            r.detail = Some(format!("{WAIT_EVENT_TOOL} returned no matched list"));
            r.observed = clip(&out.json, 300);
            return;
        };
        let matched = matched.clone();
        // The loss signals, read apart from the clause's match: every row
        // of the throttle family since the mark.
        let family = throttle_family(&kind);
        let fam = inv
            .call(
                WAIT_EVENT_TOOL,
                json!({
                    "kind": family, "since_seq": start, "count": COLLECT_MAX,
                    "timeout_ms": 0, "cursor": EVENT_CURSOR,
                }),
            )
            .await;
        let Some(family_rows) = fam.json.get("matched").and_then(Value::as_array) else {
            r.detail = Some(format!(
                "{WAIT_EVENT_TOOL} ({family}): could not read the throttle family: {}",
                fam.error.unwrap_or_default()
            ));
            return;
        };
        let flag = |v: &Value| v.get("gap").and_then(Value::as_bool).unwrap_or(false);
        let dropped = |v: &Value| {
            v.pointer("/last_pump/bridge_dropped")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        };
        let loss = Loss {
            gap: flag(&out.json) || flag(&fam.json),
            suppressed: suppressed_in(family_rows),
            upstream_dropped: dropped(&out.json) + dropped(&fam.json),
        };
        let (verdict, mut observed, detail) = grade_events(c, &matched, loss);
        observed["since_seq"] = json!(start);
        observed["kind"] = json!(kind);
        r.verdict = verdict;
        r.observed = observed;
        r.detail = detail;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clause(extra: &str) -> ExpectSpec {
        toml::from_str(&format!(
            "id = \"c\"\ntext = \"t\"\nsource = \"client_event\"\nevent = \"client.ability.sent\"\n{extra}"
        ))
        .unwrap()
    }

    fn ev(seq: u64, fields: Value) -> Value {
        json!({ "seq": seq, "kind": "ability.sent", "ts_ms": 100 + seq, "fields": fields })
    }

    #[test]
    fn the_ring_kind_drops_the_client_prefix() {
        assert_eq!(ring_kind("client.ability.recv"), "ability.recv");
        assert_eq!(ring_kind("client.ability.*"), "ability.*");
        assert_eq!(ring_kind("chat.line"), "chat.line");
    }

    #[test]
    fn event_fields_win_over_store_columns() {
        let row = event_row(&json!({
            "seq": 9, "kind": "ability.applied", "ts_ms": 1,
            "fields": { "kind": "effect_bar_add", "effect_id": 637 }
        }));
        assert_eq!(row["kind"], "effect_bar_add");
        assert_eq!(row["store_kind"], "ability.applied");
        assert_eq!(row["store_seq"], 9);
    }

    #[test]
    fn counts_and_fields_grade_every_event() {
        let sent = [
            ev(
                1,
                json!({ "ability_id": 597, "target_id": 0, "client_target_id": 0 }),
            ),
            ev(
                2,
                json!({ "ability_id": 597, "target_id": 7, "client_target_id": 7 }),
            ),
        ];
        assert_eq!(
            grade_events(&clause(""), &sent, Loss::default()).0,
            Verdict::Pass
        );
        assert_eq!(
            grade_events(&clause(""), &[], Loss::default()).0,
            Verdict::Fail
        );
        let two = clause("min_rows = 2\nmax_rows = 2");
        assert_eq!(grade_events(&two, &sent, Loss::default()).0, Verdict::Pass);
        assert_eq!(
            grade_events(&two, &sent[..1], Loss::default()).0,
            Verdict::Fail
        );
        let none = clause("max_rows = 0");
        assert_eq!(grade_events(&none, &[], Loss::default()).0, Verdict::Pass);
        assert_eq!(grade_events(&none, &sent, Loss::default()).0, Verdict::Fail);
        // B-15: what was sent is what the UI had targeted, on every send.
        let same = clause("field = \"target_id\"\nop = \"gte\"\nvalue = 0");
        assert_eq!(grade_events(&same, &sent, Loss::default()).0, Verdict::Pass);
        let seven = clause("field = \"target_id\"\nop = \"eq\"\nvalue = 7");
        assert_eq!(
            grade_events(&seven, &sent, Loss::default()).0,
            Verdict::Fail
        );
    }

    #[test]
    fn a_gap_or_a_throttle_cannot_prove_an_upper_bound() {
        let none = clause("max_rows = 0");
        let gap = Loss {
            gap: true,
            ..Loss::default()
        };
        let (v, _, why) = grade_events(&none, &[], gap);
        assert_eq!(v, Verdict::Unverified);
        assert!(why.unwrap().contains("evicted"));
        // The suppression sits on a later *press* row the clause does not
        // match, and the suppressed send left no row at all: the family
        // read still sees it.
        let family = [
            json!({ "seq": 4, "kind": "ability.press", "fields": { "ability_id": 1 } }),
            json!({ "seq": 9, "kind": "ability.press", "fields": { "ability_id": 1, "suppressed": 4 } }),
        ];
        let throttled = Loss {
            suppressed: suppressed_in(&family),
            ..Loss::default()
        };
        assert_eq!(throttled.suppressed, 4);
        let sent = [ev(5, json!({ "ability_id": 1 }))];
        let (v, _, why) = grade_events(&clause("max_rows = 1"), &sent, throttled);
        assert_eq!(v, Verdict::Unverified);
        assert!(why.unwrap().contains("suppressed 4"));
        let every = clause("field = \"ability_id\"\nop = \"eq\"\nvalue = 1");
        assert_eq!(
            grade_events(&every, &sent, throttled).0,
            Verdict::Unverified
        );
        let dropped = Loss {
            upstream_dropped: 2,
            ..Loss::default()
        };
        assert_eq!(grade_events(&none, &[], dropped).0, Verdict::Unverified);
        // A plain "at least one" still passes: the one it saw is real.
        assert_eq!(grade_events(&clause(""), &sent, throttled).0, Verdict::Pass);
        // Without loss the same bound is proven.
        assert_eq!(
            grade_events(&clause("max_rows = 1"), &sent, Loss::default()).0,
            Verdict::Pass
        );
    }

    #[test]
    fn the_ability_family_shares_one_throttle() {
        assert_eq!(throttle_family("ability.sent"), "ability.*");
        assert_eq!(throttle_family("ability.*"), "ability.*");
        assert_eq!(throttle_family("lua.print"), "lua.print");
    }
}
