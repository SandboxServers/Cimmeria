//! The flow tools: log in, manage characters, play, finish a dialog, log
//! out. Each drives the client with its own input (see
//! `supervisor::flows`) and returns per-step timings; a failure names the
//! step, the widget or state that failed, and the screens visible then.

use std::time::Duration;

use rmcp::{
    handler::server::wrapper::Parameters, model::*, schemars, tool, tool_router,
    ErrorData as McpError,
};
use serde_json::Value;

use super::LabServer;
use crate::supervisor::flows::characters::CreateRequest;
use crate::supervisor::flows::login::LoginRequest;
use crate::supervisor::flows::world::{DEFAULT_MAX_PAGES, DEFAULT_PLAY_TIMEOUT};
use crate::supervisor::flows::FlowError;

/// Args for `lab_login`. Each falls back to `lab-account.json`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct LoginArgs {
    /// Account name (default: `username` in lab-account.json).
    #[serde(default)]
    pub account: Option<String>,
    /// Password (default: lab-account.json). Letters, digits and -_/. only:
    /// the lab types it key by key.
    #[serde(default)]
    pub password: Option<String>,
    /// Server row to select by name (default: lab-account.json's `server`;
    /// when that name is not listed, the preselected row is used).
    #[serde(default)]
    pub server: Option<String>,
}

/// Args for `lab_create_character`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateCharacterArgs {
    /// First name (letters only).
    pub first: String,
    /// Last name (letters only). This is the character's name in the list.
    pub last: String,
    /// `sgu` or `praxis`.
    pub alignment: String,
    /// SGU: Soldier, Commando, Scientist, Archeologist, Asgard, Sholva.
    /// Praxis: Soldier, Commando, Scientist, Archeologist, Goauld, Jaffa.
    pub archetype: String,
    /// `male` or `female` (ignored for Asgard).
    pub gender: String,
}

/// Args naming one character.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CharacterNameArgs {
    /// Exact character name as the list shows it.
    pub name: String,
}

/// Args for `lab_play_character`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PlayArgs {
    /// Exact character name as the list shows it.
    pub name: String,
    /// Press Escape through the loading cutscene and a new character's
    /// arrival cutscene (default true). Stops once the world HUD is up.
    #[serde(default)]
    pub skip_cutscene: Option<bool>,
    /// Budget from Play to the world HUD in ms (default 120000).
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// Args for `lab_finish_dialog`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct FinishDialogArgs {
    /// Press Accept when the dialog offers it and no Done (a mission
    /// offer). Default false: stop and report instead.
    #[serde(default)]
    pub accept: bool,
    /// Pages to turn with Next before giving up (default 12).
    #[serde(default)]
    pub max_pages: Option<u32>,
}

/// Args for `lab_ensure_character_slot`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct EnsureSlotArgs {
    /// Slots to leave free of the client's 8 (default 1, minimum 1).
    #[serde(default)]
    pub free_slots: Option<usize>,
    /// Names never deleted. The lab account's own character is always
    /// protected.
    #[serde(default)]
    pub protect: Vec<String>,
}

#[tool_router(router = flows_router, vis = "pub(super)")]
impl LabServer {
    #[tool(
        description = "Log in with the client's own input: Escape through the intro movies, type the account and password into the login boxes, press Login, pick the server row (by name when given), press Select, and stop at character select. Credentials default to lab-account.json. Returns the character list and per-step timings; a failure names the step and shows any prompt text (e.g. a bad password)."
    )]
    async fn lab_login(
        &self,
        Parameters(a): Parameters<LoginArgs>,
    ) -> Result<CallToolResult, McpError> {
        let req = LoginRequest {
            account: a.account,
            password: a.password,
            server: a.server,
        };
        flow_result(self.supervisor.login_flow(req).await)
    }

    #[tool(
        description = "List the characters at character select: slot index (1-based), name, level, alignment, archetype, playable. Errors when the client is not at character select."
    )]
    async fn lab_characters(&self) -> Result<CallToolResult, McpError> {
        flow_result(self.supervisor.characters_flow().await)
    }

    #[tool(
        description = "Create a character from character select: Create, alignment, archetype, gender, first and last name, Create; waits for the return to character select. A creation error prompt (name taken, invalid) is returned as the error. Needs a free slot (see lab_ensure_character_slot)."
    )]
    async fn lab_create_character(
        &self,
        Parameters(a): Parameters<CreateCharacterArgs>,
    ) -> Result<CallToolResult, McpError> {
        let req = CreateRequest {
            first: a.first,
            last: a.last,
            alignment: a.alignment,
            archetype: a.archetype,
            gender: a.gender,
        };
        flow_result(self.supervisor.create_character_flow(req).await)
    }

    #[tool(
        description = "Delete a character by name: select its slot (verified against the client's selection), Delete, and confirm only when the prompt names that character; waits until it leaves the list."
    )]
    async fn lab_delete_character(
        &self,
        Parameters(a): Parameters<CharacterNameArgs>,
    ) -> Result<CallToolResult, McpError> {
        flow_result(self.supervisor.delete_character_flow(&a.name).await)
    }

    #[tool(
        description = "Keep room under the client's 8-character cap: delete the oldest (lowest slot) characters not in `protect` until `free_slots` slots are open. The lab account's character is always protected. Returns what was deleted."
    )]
    async fn lab_ensure_character_slot(
        &self,
        Parameters(a): Parameters<EnsureSlotArgs>,
    ) -> Result<CallToolResult, McpError> {
        flow_result(
            self.supervisor
                .ensure_slot_flow(a.free_slots.unwrap_or(1), a.protect)
                .await,
        )
    }

    #[tool(
        description = "Enter the world on a character: select it, Play, Escape through the loading and arrival cutscenes (skip_cutscene, default true), and stop when the world HUD (SelfStatusWin) is visible; a new character's intro dialog over the arrival cutscene is not enough, Escape continues until the HUD shows or the timeout. Reports whether a dialog is open and its title/text/buttons."
    )]
    async fn lab_play_character(
        &self,
        Parameters(a): Parameters<PlayArgs>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = a
            .timeout_ms
            .map(Duration::from_millis)
            .unwrap_or(DEFAULT_PLAY_TIMEOUT);
        flow_result(
            self.supervisor
                .play_flow(&a.name, a.skip_cutscene.unwrap_or(true), timeout)
                .await,
        )
    }

    #[tool(
        description = "Finish the open dialog like a player: page with Next until the green checkmark (Done) shows, then press it. Never closes with the X. With accept=true, presses Accept on an offer that has no Done. Reports the title and pages clicked; no dialog open is not an error."
    )]
    async fn lab_finish_dialog(
        &self,
        Parameters(a): Parameters<FinishDialogArgs>,
    ) -> Result<CallToolResult, McpError> {
        flow_result(
            self.supervisor
                .finish_dialog_flow(a.accept, a.max_pages.unwrap_or(DEFAULT_MAX_PAGES))
                .await,
        )
    }

    #[tool(
        description = "Log out to character select: Enter, type /logout, Enter, wait for character select. Returns the character list. Already at character select is not an error."
    )]
    async fn lab_logout(&self) -> Result<CallToolResult, McpError> {
        flow_result(self.supervisor.logout_flow().await)
    }
}

/// A flow's JSON as text content, or its failure as an MCP error whose
/// `data` carries the step log.
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

    fn server() -> LabServer {
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
        LabServer::new(Arc::new(Supervisor::new(bridge, config)))
    }

    /// The three routers are combined: every flow and client-state tool is
    /// reachable, and `lab_login` is registered once (the native flow, not
    /// the retired Lua autologin).
    #[test]
    fn combined_router_exposes_the_flow_and_state_tools() {
        let s = server();
        for name in [
            "lab_login",
            "lab_characters",
            "lab_create_character",
            "lab_delete_character",
            "lab_ensure_character_slot",
            "lab_play_character",
            "lab_finish_dialog",
            "lab_logout",
            "client_ui_state",
            "client_wait_for",
            "client_entity_table",
            "lab_screenshot_region",
            "lab_pixel_probe",
            "client_ui_click",
            "lab_client_start",
            "client_window_read",
            "client_window_click",
            "client_chat_log",
            "client_inventory",
            "client_player_state",
            "client_item_action",
            "client_drag_drop",
        ] {
            assert!(s.tool_router.has_route(name), "{name} is not routed");
        }
        let logins = s
            .tool_router
            .list_all()
            .into_iter()
            .filter(|t| t.name == "lab_login")
            .count();
        assert_eq!(logins, 1);
        let login = s.tool_router.get("lab_login").unwrap();
        assert!(login
            .description
            .as_deref()
            .unwrap_or_default()
            .contains("Escape through the intro movies"));
    }
}
