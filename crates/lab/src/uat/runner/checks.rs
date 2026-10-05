//! The up-front checks that BLOCK a row before anything is driven: a
//! standing reason, the players it needs, colo rule 6, and every tool its
//! actions and clauses name, on the client that will run them.

use super::actions::{CHAT_MACRO_TOOLS, CHAT_READ_TOOL, CHAT_SEND_TOOL};
use super::players::Who;
use super::Runner;
use crate::uat::invoke::ToolInvoker;
use crate::uat::lab_commands;
use crate::uat::spec::{ActionKind, ActionSpec, RowSpec, SectionSpec, Source};
use crate::uat::tier;

/// Colo rule 6: these need the owner's say-so in the run's
/// `owner_approvals` (the approval word is the second item).
pub const OWNER_ONLY: [(&str, &str); 5] = [
    (".announce", "announce"),
    (".bm_seed", "bm_seed"),
    (".mute", "mute"),
    ("/gmshout", "gmshout"),
    ("server_content_reload", "content_reload"),
];

/// Characters `client_type_text` can type today.
pub fn typable(line: &str) -> bool {
    line.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '/' | '.'))
}

impl<I: ToolInvoker> Runner<'_, I> {
    /// Reasons this row cannot run at all, found before anything moves.
    pub(crate) fn static_blocks(&self, spec: &SectionSpec, row: &RowSpec) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(b) = &row.blocked {
            out.push(b.clone());
        }
        out.extend(self.player_blocks(row, self.character_name(spec).as_deref()));
        if spec.section.account == "non-gm" {
            out.push("needs a non-GM account (matrix X2): only the GM lab account exists".into());
        }
        let all = row.setup.iter().chain(&row.steps).chain(&row.teardown);
        for a in all {
            self.check_action(a, &mut out);
        }
        for c in row.expect.iter().filter(|c| c.required) {
            let reader = match c.source {
                Source::Chat => Some(CHAT_READ_TOOL),
                Source::Tool => c.tool.as_deref(),
                Source::Lua => Some("client_lua_eval"),
                Source::Wait => Some("client_wait_for"),
                Source::ClientEvent => Some(super::client_events::WAIT_EVENT_TOOL),
                _ => None,
            };
            let who = Who::of(c.client.as_deref());
            if let Some(t) = reader {
                if !self.on(who).has_tool(t) {
                    out.push(format!(
                        "clause {} needs tool {t}, which is not routed{}",
                        c.id,
                        on_p2(who)
                    ));
                }
            }
        }
        out.dedup();
        out
    }

    fn check_action(&self, a: &ActionSpec, out: &mut Vec<String>) {
        let text = a
            .chat
            .clone()
            .or_else(|| a.tool.clone())
            .unwrap_or_default();
        for (pat, word) in OWNER_ONLY {
            let hit = text == pat || text.starts_with(&format!("{pat} "));
            if hit && !self.req.owner_approvals.iter().any(|w| w == word) {
                out.push(format!(
                    "colo rule 6: {pat} needs the owner's say-so (owner_approvals: [\"{word}\"])"
                ));
            }
        }
        let who = Who::of(a.client.as_deref());
        let inv = self.on(who);
        match a.kind() {
            Ok(ActionKind::Tool) => {
                let t = a.tool.as_deref().unwrap_or_default();
                let alt = a.fallback.iter().any(|f| self.action_available(who, f));
                if !self.routed(who, t) && !alt {
                    out.push(format!("needs tool {t}, which is not routed{}", on_p2(who)));
                }
                if let Err(e) = tier::resolve_tool(t, a.tier) {
                    out.push(format!("spec: {e}"));
                }
                // `@clear_effects { name }` targets by clicking first.
                let targets = t == lab_commands::CLEAR_EFFECTS_TOOL
                    && a.args.as_ref().is_some_and(|x| x.get("name").is_some());
                if targets && !inv.has_tool("client_target") {
                    out.push(format!(
                        "{t} with a name needs tool client_target, which is not routed{}",
                        on_p2(who)
                    ));
                }
            }
            Ok(ActionKind::Chat) => {
                let line = a.chat.as_deref().unwrap_or_default();
                if !inv.has_tool(CHAT_SEND_TOOL) {
                    for t in CHAT_MACRO_TOOLS {
                        if !inv.has_tool(t) {
                            out.push(format!(
                                "typing chat needs tool {t}, which is not routed{}",
                                on_p2(who)
                            ));
                        }
                    }
                    // `${var}` values are checked when the line is sent.
                    let bare = crate::uat::spec::without_vars(line);
                    if !typable(&bare) {
                        out.push(format!(
                            "chat line {line:?} has characters the lab cannot type yet (needs {CHAT_SEND_TOOL} / matrix L11)"
                        ));
                    }
                }
            }
            Ok(ActionKind::Capture) => {
                if !inv.has_tool(CHAT_READ_TOOL) {
                    out.push(format!("capture needs tool {CHAT_READ_TOOL}"));
                }
            }
            Ok(ActionKind::Wait) => {}
            Err(e) => out.push(format!("spec: {e}")),
        }
    }

    /// A fallback runs on its parent action's client.
    pub(crate) fn action_available(&self, who: Who, a: &ActionSpec) -> bool {
        let inv = self.on(who);
        match a.kind() {
            Ok(ActionKind::Tool) => a.tool.as_deref().is_some_and(|t| self.routed(who, t)),
            Ok(ActionKind::Chat) => {
                inv.has_tool(CHAT_SEND_TOOL) || CHAT_MACRO_TOOLS.iter().all(|t| inv.has_tool(t))
            }
            _ => true,
        }
    }
}

/// " on p2" for a reason about the second client.
fn on_p2(who: Who) -> &'static str {
    if who == Who::P2 {
        " on p2"
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typable_matches_the_lab_charset() {
        assert!(typable(".bug uat M1-1"));
        assert!(typable("/gmgotolocation Harset 0 0 0"));
        assert!(!typable("/afk afk, back soon"));
        assert!(!typable(".bug what?"));
    }
}
