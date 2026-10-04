//! Two-player rows (AB-L6). A `players = 2` row drives a second lab
//! client, `p2`, through its own invoker: the runner brings it in world as
//! its own account's character, runs the actions, clauses and evidence
//! tagged `client = "p2"` against it, and expands `@target_player` into a
//! real-input `client_target` on the other player's character.
//!
//! Without a configured second instance a two-player row is BLOCKED and
//! says why; nothing is driven.

use serde_json::{json, Value};

use super::{RowCtx, Runner};
use crate::uat::invoke::ToolInvoker;
use crate::uat::spec::{ActionSpec, RowSpec};
use crate::uat::tier::Role;
use crate::uat::tools::TARGET_PLAYER_TOOL;

/// The `client` value that selects the second player.
pub const P2: &str = "p2";
/// The row var holding the second player's character name.
pub const P2_CHARACTER_VAR: &str = "p2_character";

/// The second lab client a two-player row drives.
pub struct SecondPlayer<'a, I: ToolInvoker> {
    pub inv: &'a I,
    /// The lab instance name (`p2`), for the bundle.
    pub instance: String,
    /// `lab-account.<instance>.json`'s account and character.
    pub account: Option<String>,
    pub character: String,
}

/// Which lab client an action, clause or evidence item runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Who {
    P1,
    P2,
}

impl Who {
    pub(crate) fn of(client: Option<&str>) -> Self {
        if client == Some(P2) {
            Who::P2
        } else {
            Who::P1
        }
    }

    /// The bundle's `client` field: unset for p1, as in one-player runs.
    pub(crate) fn tag(self) -> Option<String> {
        (self == Who::P2).then(|| P2.to_string())
    }

    /// A chat-mark key: p1 keeps the bare label; p2's are prefixed.
    pub(crate) fn mark(self, label: &str) -> String {
        match self {
            Who::P1 => label.to_string(),
            Who::P2 => format!("{P2}:{label}"),
        }
    }
}

/// The default reason a two-player row cannot run.
pub(crate) const NO_P2: &str = "players = 2 needs a second lab instance: write sessions\\lab-account.p2.json (account lab2 and its character) and run lab_uat_run from the default instance (CIMMERIA_LAB_UAT_P2 names another instance)";

impl<'a, I: ToolInvoker> Runner<'a, I> {
    /// Give the runner its second player, or the reason there is none.
    pub fn with_p2(mut self, p2: Result<SecondPlayer<'a, I>, String>) -> Self {
        if let Ok(p) = &p2 {
            self.manifest.characters.push(json!({
                "name": p.character, "client": P2, "instance": p.instance, "account": p.account,
            }));
            let _ = self.save_manifest();
        }
        self.p2 = p2;
        self
    }

    /// The invoker for `who`. Callers check [`Self::player_blocks`] first,
    /// so p2 without a second player only happens on a blocked row.
    pub(crate) fn on(&self, who: Who) -> &'a I {
        match (who, &self.p2) {
            (Who::P2, Ok(p)) => p.inv,
            _ => self.inv,
        }
    }

    /// Whether `who` can run `tool`. `@target_player` needs `client_target`.
    pub(crate) fn routed(&self, who: Who, tool: &str) -> bool {
        let tool = if tool == TARGET_PLAYER_TOOL {
            "client_target"
        } else {
            tool
        };
        self.on(who).has_tool(tool)
    }

    /// Why this row cannot have the players it needs, found before
    /// anything is driven.
    pub(crate) fn player_blocks(&self, row: &RowSpec, p1_character: Option<&str>) -> Vec<String> {
        match row.players {
            0 | 1 => vec![],
            2 => match &self.p2 {
                Err(why) => vec![why.clone()],
                // One account cannot be logged in twice: p2's login would
                // evict p1 (`duplicate_login`) mid-row. Refuse up front.
                Ok(p) if p
                    .account
                    .as_deref()
                    .zip(self.req.account_name.as_deref())
                    .is_some_and(|(a, b)| a.eq_ignore_ascii_case(b)) =>
                {
                    vec![format!(
                        "p2 logs in as {:?}, p1's own account (the second login evicts the first): give lab-account.{}.json its own account",
                        p.account.as_deref().unwrap_or_default(),
                        p.instance
                    )]
                }
                Ok(p) if p1_character.is_some_and(|c| c.eq_ignore_ascii_case(&p.character)) => {
                    vec![format!(
                        "p2 plays {:?}, the same character as p1: give lab-account.{}.json its own account and character",
                        p.character, p.instance
                    )]
                }
                Ok(_) => vec![],
            },
            n => vec![format!(
                "needs {n} players: the runner drives at most two lab clients (p1 and p2; matrix X1)"
            )],
        }
    }

    /// `@target_player` on `who`: `client_target` naming the other
    /// client's character, with the spec's own args (`allow_fallback`,
    /// `settle_ms`) kept. The click is real input; a fallback the tool
    /// reports lowers the action's tier as usual.
    pub(crate) fn target_player_args(
        &self,
        who: Who,
        args: Value,
        ctx: &RowCtx,
    ) -> Result<Value, String> {
        let var = match who {
            Who::P1 => P2_CHARACTER_VAR,
            Who::P2 => "character",
        };
        let other = ctx
            .vars
            .get(var)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("@target_player: no other player (${{{var}}} is unset)"))?;
        let mut args = match args {
            Value::Object(o) => o,
            _ => serde_json::Map::new(),
        };
        args.insert("name".into(), json!(other));
        Ok(Value::Object(args))
    }

    /// Bring p2 in world as its character, recording each call as a p2
    /// setup action. Reuses a p2 already in world as that character.
    pub(crate) async fn ensure_p2(&mut self, ctx: &mut RowCtx) -> Result<(), String> {
        let name = match &self.p2 {
            Ok(p) => p.character.clone(),
            Err(e) => return Err(e.clone()),
        };
        let status = self
            .setup_call_on(Who::P2, "lab_client_status", json!({}), ctx)
            .await?;
        let running = status
            .get("running")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let mut login = status
            .get("login_state")
            .and_then(Value::as_str)
            .unwrap_or("not_started")
            .to_string();
        if !running {
            self.setup_call_on(Who::P2, "lab_client_start", json!({}), ctx)
                .await?;
            login = "not_started".into();
            self.p2_in_world_as = None;
        }
        if !matches!(login.as_str(), "character_select" | "in_world") {
            self.setup_call_on(Who::P2, "lab_login", json!({}), ctx)
                .await?;
            login = "character_select".into();
        }
        if login == "in_world" {
            // The p2 supervisor outlives a run, so a fresh runner does not
            // know who p2 is playing: ask the client before reusing it.
            let current = match self.p2_in_world_as.clone() {
                Some(c) => Some(c),
                None => self.p2_playing(ctx).await,
            };
            if current.is_some_and(|c| c.eq_ignore_ascii_case(&name)) {
                self.p2_in_world_as = Some(name);
                return Ok(());
            }
            self.setup_call_on(Who::P2, "lab_logout", json!({}), ctx)
                .await?;
        }
        self.setup_call_on(Who::P2, "lab_play_character", json!({ "name": name }), ctx)
            .await?;
        self.p2_in_world_as = Some(name);
        Ok(())
    }

    /// The character p2's client says it is playing, or `None` when it
    /// cannot say (no reader, an error, no name): the caller then logs out
    /// and re-selects rather than guess.
    async fn p2_playing(&mut self, ctx: &mut RowCtx) -> Option<String> {
        if !self.on(Who::P2).has_tool("client_player_state") {
            return None;
        }
        let state = self
            .setup_call_on(Who::P2, "client_player_state", json!({}), ctx)
            .await
            .ok()?;
        state
            .get("name")
            .and_then(Value::as_str)
            .filter(|n| !n.is_empty())
            .map(str::to_string)
    }

    /// One setup call on `who`, recorded on the row.
    pub(crate) async fn setup_call_on(
        &mut self,
        who: Who,
        tool: &str,
        args: Value,
        ctx: &mut RowCtx,
    ) -> Result<Value, String> {
        let a = ActionSpec {
            tool: Some(tool.to_string()),
            args: Some(args),
            client: who.tag(),
            ..Default::default()
        };
        let rec = self.exec(&a, Role::Setup, ctx).await;
        let out = if rec.ok {
            Ok(rec.result.clone())
        } else {
            let tag = if who == Who::P2 { "p2 " } else { "" };
            Err(format!(
                "{tag}{tool}: {}",
                rec.error.clone().unwrap_or_default()
            ))
        };
        ctx.actions.push(rec);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn who_follows_the_client_field() {
        assert_eq!(Who::of(None), Who::P1);
        assert_eq!(Who::of(Some("p1")), Who::P1);
        assert_eq!(Who::of(Some("p2")), Who::P2);
        assert_eq!(Who::P1.tag(), None);
        assert_eq!(Who::P2.mark("cast"), "p2:cast");
        assert_eq!(Who::P1.mark(""), "");
    }
}
