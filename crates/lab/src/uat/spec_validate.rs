//! Spec rules the types cannot express: labels that exist, the fields
//! each clause source needs, regexes that compile, and which rows may
//! use the second player. Every problem names its row so an author can
//! find it; [`super::spec::parse`] runs these after resolving `@aliases`.

use std::collections::HashSet;

use serde_json::Value;

use super::spec::{ActionKind, ActionSpec, ExpectSpec, Op, RowSpec, SectionSpec, Source, SCHEMA};
use super::tools::TARGET_PLAYER_TOOL;

/// Structural checks the type system cannot express. Every problem is
/// reported with its row id so an author can find it.
pub fn validate(spec: &SectionSpec) -> Result<(), String> {
    let mut errs = Vec::new();
    if spec.schema != SCHEMA {
        errs.push(format!(
            "schema {} (this runner reads {SCHEMA})",
            spec.schema
        ));
    }
    if spec.section.character == "fresh" && spec.section.fresh.is_none() {
        errs.push("character = \"fresh\" needs a [section.fresh] table".into());
    }
    if !matches!(spec.section.character.as_str(), "lab" | "fresh") {
        errs.push(format!(
            "character {:?}: lab or fresh",
            spec.section.character
        ));
    }
    if !matches!(spec.section.account.as_str(), "gm" | "non-gm") {
        errs.push(format!("account {:?}: gm or non-gm", spec.section.account));
    }
    let mut ids = HashSet::new();
    for row in &spec.rows {
        let r = &row.id;
        if !ids.insert(r.clone()) {
            errs.push(format!("{r}: duplicate row id"));
        }
        if !matches!(
            row.state.as_str(),
            "in_world" | "char_select" | "any" | "client_stopped"
        ) {
            errs.push(format!("{r}: state {:?}", row.state));
        }
        let mut labels = HashSet::new();
        for a in row.setup.iter().chain(&row.steps).chain(&row.teardown) {
            check_action(r, a, &mut errs);
            if let Some(l) = &a.label {
                if !labels.insert(l.clone()) {
                    errs.push(format!("{r}: duplicate action label {l:?}"));
                }
            }
        }
        if row.steps.is_empty() && row.blocked.is_none() {
            errs.push(format!("{r}: no steps (add `blocked` if it cannot run)"));
        }
        if row.expect.is_empty() && row.blocked.is_none() {
            errs.push(format!("{r}: no expected clauses"));
        }
        let mut cids = HashSet::new();
        for c in &row.expect {
            if !cids.insert(c.id.clone()) {
                errs.push(format!("{r}/{}: duplicate clause id", c.id));
            }
            for l in [&c.at, &c.since, &c.action].into_iter().flatten() {
                if !labels.contains(l) {
                    errs.push(format!("{r}/{}: no action labelled {l:?}", c.id));
                }
            }
            if let Err(e) = check_clause(c) {
                errs.push(format!("{r}/{}: {e}", c.id));
            }
        }
        for e in &row.evidence {
            if let Some(l) = &e.at {
                if !labels.contains(l) {
                    errs.push(format!("{r}/evidence {}: no action labelled {l:?}", e.name));
                }
            }
        }
        check_players(row, &mut errs);
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs.join("; "))
    }
}

/// `client` names `p1` or `p2`; `p2` and `@target_player` need a
/// two-player row; only clauses that read a client take a `client`.
fn check_players(row: &RowSpec, errs: &mut Vec<String>) {
    let r = &row.id;
    let mut check = |what: &str, client: Option<&str>, targets_player: bool| {
        match client {
            None | Some("p1" | "p2") => {}
            Some(other) => errs.push(format!("{r}/{what}: client {other:?}: p1 or p2")),
        }
        if row.players < 2 && (client == Some("p2") || targets_player) {
            errs.push(format!("{r}/{what}: the second player needs players = 2"));
        }
    };
    // Fallbacks run on their action's client (the runner never switches
    // client mid-action), so a fallback may repeat that client but not
    // name another one. Checked at every depth.
    fn walk(
        a: &ActionSpec,
        parent: Option<&str>,
        check: &mut dyn FnMut(&str, Option<&str>, bool),
        errs_out: &mut Vec<String>,
    ) {
        let what = a
            .label
            .as_deref()
            .or(a.tool.as_deref())
            .or(a.chat.as_deref())
            .unwrap_or("action");
        let client = a.client.as_deref().or(parent);
        if parent.is_some() && a.client.is_some() && a.client.as_deref() != parent {
            errs_out.push(format!(
                "{what}: a fallback runs on its action's client ({}), not {:?}",
                parent.unwrap_or("p1"),
                a.client.as_deref().unwrap_or_default()
            ));
        }
        check(what, client, a.tool.as_deref() == Some(TARGET_PLAYER_TOOL));
        for f in &a.fallback {
            walk(f, Some(client.unwrap_or("p1")), check, errs_out);
        }
    }
    let mut nested = Vec::new();
    for a in row.setup.iter().chain(&row.steps).chain(&row.teardown) {
        walk(a, None, &mut check, &mut nested);
    }
    for c in &row.expect {
        check(&c.id, c.client.as_deref(), false);
    }
    for e in &row.evidence {
        check(&format!("evidence {}", e.name), e.client.as_deref(), false);
    }
    errs.extend(nested.into_iter().map(|e| format!("{r}/{e}")));
    for c in &row.expect {
        let reads_client = matches!(
            c.source,
            Source::Chat | Source::Tool | Source::Lua | Source::Wait | Source::ClientEvent
        );
        if c.client.is_some() && !reads_client {
            errs.push(format!(
                "{r}/{}: client applies to chat, tool, lua, wait and client_event clauses",
                c.id
            ));
        }
    }
}

fn check_action(row: &str, a: &ActionSpec, errs: &mut Vec<String>) {
    match a.kind() {
        Err(e) => errs.push(format!("{row}: {e}")),
        Ok(ActionKind::Capture) => {
            if a.regex.is_none() || a.var.is_none() {
                errs.push(format!("{row}: capture needs regex and var"));
            }
            if a.capture.as_deref() != Some("chat") {
                errs.push(format!("{row}: capture source must be \"chat\""));
            }
        }
        Ok(ActionKind::Tool) => {
            // The ability lab commands are chat lines the runner builds:
            // a bad disposition or id is a spec error, not a typed typo.
            if let Some(t) = a.tool.as_deref() {
                if let Err(e) = super::lab_commands::check(t, a.args.as_ref()) {
                    errs.push(format!("{row}: {e}"));
                }
            }
        }
        Ok(_) => {}
    }
    if let Some(re) = &a.regex {
        if let Err(e) = regex::Regex::new(&without_vars(re)) {
            errs.push(format!("{row}: bad regex {re:?}: {e}"));
        }
    }
    for f in &a.fallback {
        if f.tool.is_none() && f.chat.is_none() {
            errs.push(format!("{row}: a fallback must be a tool or chat action"));
        }
        check_action(row, f, errs);
    }
}

/// A pattern with its `${var}` placeholders replaced by a literal, so it
/// can be compiled before the variables are known.
pub fn without_vars(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find("${") {
        out.push_str(&rest[..i]);
        match rest[i..].find('}') {
            Some(j) => {
                out.push('X');
                rest = &rest[i + j + 1..];
            }
            None => {
                out.push_str(&rest[i..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

fn check_clause(c: &ExpectSpec) -> Result<(), String> {
    let need = |ok: bool, what: &str| if ok { Ok(()) } else { Err(what.to_string()) };
    match c.source {
        Source::Chat => need(
            c.contains.is_some() || c.matches.is_some(),
            "a chat clause needs contains or matches",
        )?,
        Source::Tool | Source::Server => need(c.tool.is_some(), "needs tool")?,
        Source::Lua => need(c.chunk.is_some(), "a lua clause needs chunk")?,
        Source::Wait => need(
            c.lua_condition.is_some(),
            "a wait clause needs lua_condition",
        )?,
        Source::Timing => need(
            c.action.is_some() && c.max_ms.is_some(),
            "a timing clause needs action and max_ms",
        )?,
        Source::Signoz => need(c.filter.is_some(), "a signoz clause needs filter")?,
        Source::Packet => {
            need(c.message.is_some(), "a packet clause needs message")?;
            need(
                matches!(c.direction.as_deref(), Some("to_client" | "to_server")),
                "a packet clause needs direction = \"to_client\" or \"to_server\"",
            )?;
            // One tap covers the row and is read once, at teardown.
            need(
                c.at.is_none() && c.since.is_none(),
                "a packet clause is graded over the whole row: no at or since",
            )?;
            // Without a field, op/value would be silently ignored and any
            // matching message would pass.
            need(
                c.field.is_some() || (c.op.is_none() && c.value.is_none() && c.tolerance.is_none()),
                "a packet clause's op, value and tolerance need a field",
            )?;
        }
        Source::ClientEvent => {
            need(
                c.event.as_deref().is_some_and(|e| e.starts_with("client.")),
                "a client_event clause needs event = \"client.<kind>\" (the telemetry target)",
            )?;
            // As for packets: op/value without a field would pass on any event.
            need(
                c.field.is_some() || (c.op.is_none() && c.value.is_none() && c.tolerance.is_none()),
                "a client_event clause's op, value and tolerance need a field",
            )?;
        }
        Source::Human => need(c.question.is_some(), "a human clause needs question")?,
    }
    if c.source != Source::ClientEvent && (c.event.is_some() || c.match_fields.is_some()) {
        return Err("event and match_fields belong to client_event clauses".into());
    }
    if matches!(c.source, Source::Tool | Source::Server | Source::Lua)
        && c.op.is_none()
        && c.value.is_none()
    {
        return Err("needs op and/or value".into());
    }
    if c.field.is_some() && c.op.is_none() {
        return Err("field needs op".into());
    }
    if c.op == Some(Op::Approx)
        && (c.tolerance.is_none_or(|t| !t.is_finite() || t < 0.0)
            || !c.value.as_ref().is_some_and(Value::is_number))
    {
        return Err("op approx needs a numeric value and a finite tolerance >= 0".into());
    }
    if let Some(re) = &c.matches {
        regex::Regex::new(&without_vars(re)).map_err(|e| format!("bad regex {re:?}: {e}"))?;
    }
    Ok(())
}
