//! Ability, combat and event-wait tools: `client_hotbar`,
//! `client_use_ability`, `client_combat_log`, `client_die_and_respawn`
//! and `client_wait_event`. The work is in `supervisor::combat` and
//! `supervisor::events`; this module is the MCP surface (arguments,
//! descriptions, result wrapping).

use std::time::Duration;

use rmcp::{
    handler::server::wrapper::Parameters, model::*, schemars, tool, tool_router,
    ErrorData as McpError,
};
use serde_json::{Map, Value};

use super::LabServer;
use crate::supervisor::combat::combat_log::CombatLogRequest;
use crate::supervisor::combat::defeat::{
    DieRequest, Respawn, DEFAULT_DEFEAT_TIMEOUT, DEFAULT_RESPAWN_TIMEOUT,
};
use crate::supervisor::combat::use_ability::resolve::Query;
use crate::supervisor::combat::use_ability::{
    Fallback, Press, UseAbilityRequest, DEFAULT_OBSERVE_MS, MAX_OBSERVE_MS,
};
use crate::supervisor::events::predicate::EventPredicate;
use crate::supervisor::events::wait::{
    WaitRequest, DEFAULT_CURSOR, DEFAULT_POLL_MS, DEFAULT_TIMEOUT_MS, MAX_TIMEOUT_MS,
};
use crate::supervisor::flows::FlowError;

/// Args for `client_hotbar`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct HotbarArgs {
    /// Also list registered buttons with no action (default false).
    #[serde(default)]
    pub include_empty: bool,
}

/// Args for `client_use_ability`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct UseAbilityArgs {
    /// Ability id. Give this or `name`.
    #[serde(default)]
    pub ability_id: Option<i64>,
    /// Ability name (case-insensitive; exact match first, else a unique
    /// substring) from the hotbar and the known abilities.
    #[serde(default)]
    pub name: Option<String>,
    /// Hotbar press: `auto` (default: the bound key when the lab can post
    /// it, else a click on the button), `key`, or `click`.
    #[serde(default)]
    pub press: Option<String>,
    /// When the ability is not on the hotbar, put it on the first visible
    /// empty button (the drop handler's own calls, reported as N3) and
    /// press that. Default false.
    #[serde(default)]
    pub place: bool,
    /// Not on the hotbar and `place` false: `window` (default: click it in
    /// the Ability window, N1), `lua` (`useAbility(id, Unit.Target)`, N3),
    /// or `none` (report and stop).
    #[serde(default)]
    pub fallback: Option<String>,
    /// How long to watch for the result after the press, ms (default 2500,
    /// max 30000). Ends early once an effect or refusal arrives.
    #[serde(default)]
    pub observe_ms: Option<u64>,
}

/// Args for `client_combat_log`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct CombatLogArgs {
    /// Read events after this store seq (overrides the cursor's position).
    #[serde(default)]
    pub since_seq: Option<u64>,
    /// Named cursor (default `combat_log`); independent readers use
    /// different names.
    #[serde(default)]
    pub cursor: Option<String>,
    /// Do not advance the cursor.
    #[serde(default)]
    pub peek: bool,
    /// Max records (default 200).
    #[serde(default)]
    pub max: Option<usize>,
}

/// Args for `client_die_and_respawn`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct DieArgs {
    /// Setup (GM, reported as setup): type `/gmsethealth <n> 0` first so
    /// the next hit is lethal. It does not kill by itself.
    #[serde(default)]
    pub setup_health: Option<i32>,
    /// `release` (default: click Release), `auto` (let the countdown
    /// release), or `none` (stop at the defeat window).
    #[serde(default)]
    pub respawn: Option<String>,
    /// Respawner to pick by name (case-insensitive substring); default the
    /// preselected first row.
    #[serde(default)]
    pub respawner: Option<String>,
    /// Wait for the defeat window this long, ms (default 60000).
    #[serde(default)]
    pub defeat_timeout_ms: Option<u64>,
    /// Wait for the respawn this long, ms (default 60000).
    #[serde(default)]
    pub respawn_timeout_ms: Option<u64>,
}

/// Args for `client_wait_event`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct WaitEventArgs {
    /// Event kind glob: `cme.event`, `net.out`, `entity.*`, `cegui.log`,
    /// `lua.error`, `lua.print`, `hook.hit`, `combat.hit`, `chat.line`.
    #[serde(default)]
    pub kind: Option<String>,
    /// Name glob over `event` / `method` / `name` / `ability_name` /
    /// `channel_name` (e.g. `*onEffectResults`, `useAbility`).
    #[serde(default)]
    pub name: Option<String>,
    /// Entity id in `entity_id` / `source_id` / `target_id` / `id`.
    #[serde(default)]
    pub entity_id: Option<i64>,
    /// Text: case-insensitive substring (glob when it has `*`/`?`) of a
    /// chat line, CEGUI message, or the event's fields.
    #[serde(default)]
    pub text: Option<String>,
    /// Field equality: strings are globs, numbers compare numerically,
    /// e.g. `{"hit_name": "Critical", "target_is_player": true}`.
    #[serde(default)]
    pub fields: Option<Map<String, Value>>,
    /// Also met while this CEGUI window is visible (level-triggered).
    #[serde(default)]
    pub window: Option<String>,
    /// Named cursor (default `wait`). A met wait moves it to the match; a
    /// timeout leaves it.
    #[serde(default)]
    pub cursor: Option<String>,
    /// Start after this store seq instead of the cursor.
    #[serde(default)]
    pub since_seq: Option<u64>,
    /// Matches needed (default 1).
    #[serde(default)]
    pub count: Option<usize>,
    /// Only put the cursor at the newest event and return (arm, act, wait).
    #[serde(default)]
    pub arm: bool,
    /// Give up after this many ms (default 10000, max 600000): `met: false`.
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    /// Poll interval ms (default 300, 100..=5000).
    #[serde(default)]
    pub poll_ms: Option<u64>,
}

#[tool_router(router = combat_router, vis = "pub(super)")]
impl LabServer {
    #[tool(
        description = "Read the action bar as the client holds it: every bound button (id, window, visible), its action (type, ability or item id, name, quantity), cooldown remaining/total, and both key bindings (virtual key, text, the lab key that presses it). The hotbar is client-local; the server has no copy."
    )]
    async fn client_hotbar(
        &self,
        Parameters(a): Parameters<HotbarArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.wrap(self.supervisor.hotbar(a.include_empty).await)
    }

    #[tool(
        description = "Use an ability like a player, by id or name. On the hotbar: press its bound key (real input), or click the button. Not on the hotbar: click it in the Ability window (real click), or with place=true put it on an empty button first (reported N3); fallback=lua calls useAbility (N3). Reports the native level, what was pressed, and the result read from client events: sent (net.out useAbility), cast started (onSequence), effect applied (onEffectResults / combat text), refused (onErrorCode, feedback text), timings, and the button's cooldown after."
    )]
    async fn client_use_ability(
        &self,
        Parameters(a): Parameters<UseAbilityArgs>,
    ) -> Result<CallToolResult, McpError> {
        let query = match (a.ability_id, a.name) {
            (Some(id), _) => Query::Id(id),
            (None, Some(n)) => Query::Name(n),
            (None, None) => {
                return Err(McpError::invalid_params("give ability_id or name", None));
            }
        };
        let press = Press::parse(a.press.as_deref()).map_err(bad)?;
        let fallback = Fallback::parse(a.fallback.as_deref()).map_err(bad)?;
        let observe = Duration::from_millis(
            a.observe_ms
                .unwrap_or(DEFAULT_OBSERVE_MS)
                .min(MAX_OBSERVE_MS),
        );
        let req = UseAbilityRequest {
            query,
            press,
            place: a.place,
            fallback,
            observe,
        };
        flow_result(self.supervisor.use_ability_flow(req).await)
    }

    #[tool(
        description = "Floating combat text feed with a read cursor: every UnitCombat event the client showed (ability id/name, hit type, source/target names and whether each is the player, mortal, stat changes with values and result codes), captured from a wrapper on SCTMod.onUnitCombat (raw event, not filtered by the SCT verbosity option). Returns the records after the cursor, a summary (hits by type, damage dealt/taken), and next_since_seq. Capture starts at the first call in the world."
    )]
    async fn client_combat_log(
        &self,
        Parameters(a): Parameters<CombatLogArgs>,
    ) -> Result<CallToolResult, McpError> {
        let req = CombatLogRequest {
            since_seq: a.since_seq,
            cursor: a.cursor,
            peek: a.peek,
            max: a.max,
        };
        self.wrap(self.supervisor.combat_log(req).await)
    }

    #[tool(
        description = "Die and respawn: optional GM setup (/gmsethealth n 0, reported as setup; it does not kill by itself), wait for the defeat window, read its respawners and countdown, click Release (real click) or let the countdown release, then verify: window closed, alive, position and world before / at death / after."
    )]
    async fn client_die_and_respawn(
        &self,
        Parameters(a): Parameters<DieArgs>,
    ) -> Result<CallToolResult, McpError> {
        let req = DieRequest {
            setup_health: a.setup_health,
            respawn: Respawn::parse(a.respawn.as_deref()).map_err(bad)?,
            respawner: a.respawner,
            defeat_timeout: a
                .defeat_timeout_ms
                .map_or(DEFAULT_DEFEAT_TIMEOUT, Duration::from_millis),
            respawn_timeout: a
                .respawn_timeout_ms
                .map_or(DEFAULT_RESPAWN_TIMEOUT, Duration::from_millis),
        };
        flow_result(self.supervisor.die_and_respawn_flow(req).await)
    }

    #[tool(
        description = "Wait for a client event matching a predicate (kind / name / entity_id / text / fields globs over the lab's event history: CME events incl. every inbound server method, net.out, entity lifecycle, CEGUI log, Lua errors, combat text, chat lines), or a window becoming visible. Reads through a named persistent cursor, so successive waits never race or miss events: a match moves the cursor past it, a timeout leaves it. `arm: true` marks now (arm, act, then wait). A timeout is met:false, not an error."
    )]
    async fn client_wait_event(
        &self,
        Parameters(a): Parameters<WaitEventArgs>,
    ) -> Result<CallToolResult, McpError> {
        let predicate = EventPredicate {
            kind: a.kind,
            name: a.name,
            entity_id: a.entity_id,
            text: a.text,
            fields: a.fields.unwrap_or_default(),
        };
        if predicate.is_empty() && a.window.is_none() && !a.arm {
            return Err(bad(
                "give at least one of kind, name, entity_id, text, fields or window (or arm)",
            ));
        }
        let req = WaitRequest {
            predicate,
            window: a.window,
            cursor: a.cursor.unwrap_or_else(|| DEFAULT_CURSOR.to_string()),
            since_seq: a.since_seq,
            count: a.count.unwrap_or(1),
            arm: a.arm,
            timeout: Duration::from_millis(
                a.timeout_ms
                    .unwrap_or(DEFAULT_TIMEOUT_MS)
                    .min(MAX_TIMEOUT_MS),
            ),
            poll: Duration::from_millis(a.poll_ms.unwrap_or(DEFAULT_POLL_MS).clamp(100, 5000)),
        };
        self.wrap(self.supervisor.wait_event(req).await)
    }
}

fn bad(e: impl Into<String>) -> McpError {
    McpError::invalid_params(e.into(), None)
}

/// A flow's JSON as text content, or its failure as an MCP error whose
/// `data` carries the step log (same shape as the `lab_*` flows).
fn flow_result(r: Result<Value, FlowError>) -> Result<CallToolResult, McpError> {
    match r {
        Ok(v) => {
            let text = serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string());
            Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
        }
        Err(e) => Err(McpError::internal_error(e.summary(), Some(e.to_json()))),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::client::BridgeClient;
    use crate::server::LabServer;
    use crate::supervisor::{Supervisor, SupervisorConfig};

    #[test]
    fn the_combat_tools_are_routed() {
        let config = SupervisorConfig {
            install_dir: None,
            dll_path: None,
            patches_dll: None,
            helper_path: None,
            bind: "127.0.0.1".into(),
            port: 8770,
            instance: None,
            telemetry: Default::default(),
        };
        let bridge = Arc::new(BridgeClient::new("127.0.0.1:1", ""));
        let s = LabServer::new(Arc::new(Supervisor::new(bridge, config)));
        for name in [
            "client_hotbar",
            "client_use_ability",
            "client_combat_log",
            "client_die_and_respawn",
            "client_wait_event",
            "client_events_read",
        ] {
            assert!(s.tool_router.has_route(name), "{name} is not routed");
        }
    }
}
