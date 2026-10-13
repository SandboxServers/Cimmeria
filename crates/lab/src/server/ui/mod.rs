//! The UI reader and item tools for automated UAT: window reads and
//! clicks, the chat log, inventory, player state, item actions and drag
//! and drop (see `supervisor::ui`). Every result reports the native level
//! it used; a failure is an MCP error naming the tool, the widget or step,
//! and the elapsed time.

mod args;

use std::time::{Duration, Instant};

use rmcp::{
    handler::server::wrapper::Parameters, model::*, tool, tool_router, ErrorData as McpError,
};
use serde_json::{json, Value};

use super::LabServer;
use crate::supervisor::ui::chat_log::ChatFilter;
use crate::supervisor::ui::drag_drop::DragEnd;
use crate::supervisor::ui::item_action::{ItemSelector, ItemVerb};
use crate::supervisor::ui::slots::ContainerRef;
use crate::supervisor::ui::window_click::ClickTarget;
use crate::supervisor::ui::window_read::WalkOptions;
use args::*;

/// Default wait for an action's inventory change.
const DEFAULT_WAIT_MS: u64 = 3000;

/// A tool's JSON as text, or its failure as an MCP error whose `data`
/// names the tool and the time spent.
fn ui_result(
    tool: &str,
    t0: Instant,
    r: Result<Value, String>,
) -> Result<CallToolResult, McpError> {
    match r {
        Ok(v) => {
            let text = serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string());
            Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
        }
        Err(e) => Err(McpError::internal_error(
            format!("{tool}: {e}"),
            Some(
                json!({ "tool": tool, "error": e, "elapsed_ms": t0.elapsed().as_millis() as u64 }),
            ),
        )),
    }
}

fn click_target(a: &WindowClickArgs) -> Result<ClickTarget, String> {
    match (&a.target, &a.root, &a.text, &a.list) {
        (Some(t), None, None, None) => Ok(ClickTarget::Named(t.clone())),
        (None, Some(root), Some(text), None) => Ok(ClickTarget::Text {
            root: root.clone(),
            text: text.clone(),
        }),
        (None, None, None, Some(list)) if a.row_index.is_some() || a.row_text.is_some() => {
            Ok(ClickTarget::Row {
                list: list.clone(),
                index: a.row_index,
                text: a.row_text.clone(),
            })
        }
        _ => Err(
            "give one target: `target`, or `root` + `text`, or `list` + `row_index`/`row_text`"
                .into(),
        ),
    }
}

fn mouse_button(s: Option<&str>) -> Result<usize, String> {
    match s.unwrap_or("left").to_ascii_lowercase().as_str() {
        "left" | "0" => Ok(0),
        "right" | "1" => Ok(1),
        other => Err(format!("button must be left or right, not {other:?}")),
    }
}

fn drag_end(which: &str, a: &DragEndArg) -> Result<DragEnd, String> {
    match (&a.container, a.slot, &a.window) {
        (Some(c), Some(s), None) => Ok(DragEnd::Slot {
            container: ContainerRef::parse(c),
            slot: s,
        }),
        (None, None, Some(w)) => Ok(DragEnd::Window(w.clone())),
        _ => Err(format!("`{which}` needs container + slot, or window")),
    }
}

#[tool_router(router = ui_router, vis = "pub(super)")]
impl LabServer {
    #[tool(
        description = "Read an open UI window: title, buttons (enabled), text, list rows; node rectangles with include_nodes. `window` reads any CEGUI window by name; `kind` is a typed reader that adds the module state: vault/bank, org_vault, trainer, discipline_trainer, crafting, pet, organization, mail, loot, dhd, dialog/blurb, greet, vendor, trade, character, prompt. The `prompt` reader returns visible numbered prompts and their visible button names; click a returned button name. Read-only (client_ui_lua)."
    )]
    async fn client_window_read(
        &self,
        Parameters(a): Parameters<WindowReadArgs>,
    ) -> Result<CallToolResult, McpError> {
        let t0 = Instant::now();
        let d = WalkOptions::default();
        let opts = WalkOptions {
            max_depth: a.max_depth.unwrap_or(d.max_depth).min(32),
            max_nodes: a.max_nodes.unwrap_or(d.max_nodes).min(4000),
            include_hidden: a.include_hidden.unwrap_or(false),
            max_rows: a.max_rows.unwrap_or(d.max_rows).min(1000),
        };
        let nodes = a.include_nodes.unwrap_or(false);
        let r = match (&a.window, &a.kind) {
            (Some(w), None) => self.supervisor.window_read(w, opts, nodes).await,
            (None, Some(k)) => self.supervisor.window_read_typed(k, opts, nodes).await,
            _ => Err("give `window` or `kind`, not both".to_string()),
        };
        ui_result("client_window_read", t0, r)
    }

    #[tool(
        description = "Click in a UI window like a player: a named window (`target`, e.g. a button or a named row), the visible widget under `root` whose text matches `text`, or a row of a Listbox/MultiColumnList (`list` + `row_index` or `row_text`). Real button messages at the widget's screen point; refuses hidden or disabled widgets. A row click is verified by reading the selection back; if the click missed, the row is selected through the list's Lua and the result says so (native_level client_ui_lua)."
    )]
    async fn client_window_click(
        &self,
        Parameters(a): Parameters<WindowClickArgs>,
    ) -> Result<CallToolResult, McpError> {
        let t0 = Instant::now();
        let r = async {
            let target = click_target(&a)?;
            let button = mouse_button(a.button.as_deref())?;
            self.supervisor
                .window_click(
                    target,
                    button,
                    a.double.unwrap_or(false),
                    a.allow_lua_fallback.unwrap_or(true),
                )
                .await
        }
        .await;
        ui_result("client_window_click", t0, r)
    }

    #[tool(
        description = "Click a ring destination at UI pixel (x,y) on the open world map with a real left click. If the map stays open, allow_lua_fallback (default true) invokes the stock WorldMapMod.onClick at the same pixel and reports client_ui_lua (N3); false requires a native pass. map_closed/effect_ok report whether the click selected a destination; verify method 91 with a packet tap."
    )]
    async fn client_world_map_click(
        &self,
        Parameters(a): Parameters<WorldMapClickArgs>,
    ) -> Result<CallToolResult, McpError> {
        let t0 = Instant::now();
        ui_result(
            "client_world_map_click",
            t0,
            self.supervisor
                .world_map_click(a.x, a.y, a.allow_lua_fallback.unwrap_or(true))
                .await,
        )
    }

    #[tool(
        description = "Chat lines the client showed (chat, feedback and /tell errors, Server Message, combat chatter): channel, speaker, colour, tabs, store seq. Reads through its own named `cursor`, so it never takes lines from client_wait_event or client_events_read; `since_seq` overrides the cursor, `peek` does not advance it. Filters: channel, contains, speaker. Lines from before the capture was first installed, and splash text, are not in the log. Native tier: read."
    )]
    async fn client_chat_log(
        &self,
        Parameters(a): Parameters<ChatLogArgs>,
    ) -> Result<CallToolResult, McpError> {
        let t0 = Instant::now();
        let filter = ChatFilter {
            channel: a.channel,
            contains: a.contains,
            speaker: a.speaker,
        };
        let r = self
            .supervisor
            .chat_log(
                a.cursor.as_deref().unwrap_or("default"),
                a.since_seq,
                &filter,
                a.max.unwrap_or(200).clamp(1, 1000),
                a.peek.unwrap_or(false),
            )
            .await;
        ui_result("client_chat_log", t0, r)
    }

    #[tool(
        description = "Every container the client has loaded (Main, Mission, Crafting, equipment, Bandolier with per-slot ammo type and count and the active slot, Vault / team / command vaults while open) with per-slot item id, name, quantity, quality, icon; plus cash. Container ids are read from the client's Container table. `snapshot` stores the read under a label; `diff_against` returns slot changes, per-item quantity deltas and the cash delta against an earlier snapshot. Native level: client_ui_lua (read)."
    )]
    async fn client_inventory(
        &self,
        Parameters(a): Parameters<InventoryArgs>,
    ) -> Result<CallToolResult, McpError> {
        let t0 = Instant::now();
        let r = self
            .supervisor
            .inventory(
                &a.containers,
                a.include_empty.unwrap_or(false),
                a.snapshot.as_deref(),
                a.diff_against.as_deref(),
            )
            .await;
        ui_result("client_inventory", t0, r)
    }

    #[tool(
        description = "The player as the client sees it: name, level, position {x,y,z}, facing (orientation_turns 0..1 and heading_deg), world id, alignment and archetype, experience, cash, every stat the client reports (health and focus lifted to the top), effects (getEffectInfo), the active weapon's ammo type and count, and the current target (name, level, hostility, mob id, health, position, effects). Native level: client_ui_lua (read)."
    )]
    async fn client_player_state(
        &self,
        Parameters(a): Parameters<PlayerStateArgs>,
    ) -> Result<CallToolResult, McpError> {
        let t0 = Instant::now();
        let r = self
            .supervisor
            .ui_player_state(a.stats.unwrap_or(true), a.effects.unwrap_or(true))
            .await;
        ui_result("client_player_state", t0, r)
    }

    #[tool(
        description = "Act on an item like a player. use / equip / unequip / rightclick: a right-click on the item's slot, which the stock UI turns into contextSensitiveUseItem; it is sent through CEGUI's own button injectors first, then as posted button messages (both N1), each followed by an inventory check, and with allow_fallback the stock handler is called through Lua (client_ui_lua, N3). The inventory or character window is opened with its bound key and the right tab clicked first. doubleclick on a slot is performed as the same right-click (the stock inventory has no double-click handler; result fields requested/performed). loot_all: click Loot All; loot_slot: page the loot window and double-click row `loot_index` (1-based). Pick the item by container + slot, item_id or name. When the slot cannot be put on screen, falls back to the client's slash command (/useitem, /equip, /lootitem) and reports native_level slash_command. Returns the inventory diff observed within wait_ms. effect_ok and native_pass are false when the inventory did not change (expect_change false disables the check)."
    )]
    async fn client_item_action(
        &self,
        Parameters(a): Parameters<ItemActionArgs>,
    ) -> Result<CallToolResult, McpError> {
        let t0 = Instant::now();
        let r = async {
            let verb = ItemVerb::parse(&a.action)?;
            let sel = ItemSelector {
                container: a.container.clone(),
                slot: a.slot,
                item_id: a.item_id,
                name: a.name.clone(),
            };
            self.supervisor
                .item_action(
                    verb,
                    &sel,
                    a.loot_index,
                    a.allow_fallback.unwrap_or(true),
                    a.expect_change.unwrap_or(true),
                    Duration::from_millis(a.wait_ms.unwrap_or(DEFAULT_WAIT_MS).min(60_000)),
                )
                .await
        }
        .await;
        ui_result("client_item_action", t0, r)
    }

    #[tool(
        description = "Drag an item between UI slots through the client's own CEGUI injectors: press on the source, walk the cursor to the target (at least 3 steps), release; `split` holds Ctrl (one item off the stack). Ends: container + slot (put on screen first) or a named window; the source must be a CEGUI DragContainer, and a press CEGUI does not take is refused. When CEGUI resolves no drop target and the source is dragging, fires the target's DragDropItemDropped before release (native_call, N3) unless allow_fallback is false; otherwise native_cegui (N1). Verified by an inventory diff: moved (source item reached the target slot, or any change for a window end), effect_ok and native_pass false when nothing moved; also drag_started, drop_target_resolved, drop_notified, snap_back, diff."
    )]
    async fn client_drag_drop(
        &self,
        Parameters(a): Parameters<DragDropArgs>,
    ) -> Result<CallToolResult, McpError> {
        let t0 = Instant::now();
        let r = async {
            let from = drag_end("from", &a.from)?;
            let to = drag_end("to", &a.to)?;
            self.supervisor
                .drag_drop(
                    &from,
                    &to,
                    a.split.unwrap_or(false),
                    a.steps.unwrap_or(8).clamp(1, 60),
                    a.allow_fallback.unwrap_or(true),
                    Duration::from_millis(a.wait_ms.unwrap_or(DEFAULT_WAIT_MS).min(60_000)),
                )
                .await
        }
        .await;
        ui_result("client_drag_drop", t0, r)
    }

    #[tool(
        description = "Buy from the open vendor window like a player. Sets row `row`'s quantity spinner to `quantity` by clicking its increment or decrement button (real or CEGUI-injected clicks, N1), then clicks Vendor_BuyButton. Refuses when the vendor is closed, the Buy tab is not active or the row is empty. When the spinner buttons cannot reach the quantity and allow_fallback is on, sets it through the spinner's Lua / setVendorSelectedQuantity (client_ui_lua, N3). Verified by an inventory and cash diff: bought, effect_ok and native_pass false when nothing changed (expect_change false disables the check)."
    )]
    async fn client_vendor_buy(
        &self,
        Parameters(a): Parameters<VendorBuyArgs>,
    ) -> Result<CallToolResult, McpError> {
        let t0 = Instant::now();
        let r = self
            .supervisor
            .vendor_buy(
                a.row,
                a.quantity,
                a.allow_fallback.unwrap_or(true),
                a.expect_change.unwrap_or(true),
                Duration::from_millis(a.wait_ms.unwrap_or(DEFAULT_WAIT_MS).min(60_000)),
            )
            .await;
        ui_result("client_vendor_buy", t0, r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_targets_need_exactly_one_shape() {
        let named = WindowClickArgs {
            target: Some("Loot_LootAllButton".into()),
            ..Default::default()
        };
        assert_eq!(
            click_target(&named).unwrap(),
            ClickTarget::Named("Loot_LootAllButton".into())
        );
        let text = WindowClickArgs {
            root: Some("GreetWin".into()),
            text: Some("Train".into()),
            ..Default::default()
        };
        assert!(matches!(
            click_target(&text).unwrap(),
            ClickTarget::Text { .. }
        ));
        let row = WindowClickArgs {
            list: Some("Trainer_Choices".into()),
            row_index: Some(0),
            ..Default::default()
        };
        assert!(matches!(
            click_target(&row).unwrap(),
            ClickTarget::Row { index: Some(0), .. }
        ));
        let list_without_row = WindowClickArgs {
            list: Some("L".into()),
            ..Default::default()
        };
        assert!(click_target(&list_without_row).is_err());
        let both = WindowClickArgs {
            target: Some("A".into()),
            list: Some("L".into()),
            row_index: Some(1),
            ..Default::default()
        };
        assert!(click_target(&both).is_err());
    }

    #[test]
    fn drag_ends_are_a_slot_or_a_window() {
        let slot = DragEndArg {
            container: Some("Main".into()),
            slot: Some(3),
            window: None,
        };
        assert_eq!(
            drag_end("from", &slot).unwrap(),
            DragEnd::Slot {
                container: ContainerRef::Name("Main".into()),
                slot: 3
            }
        );
        let win = DragEndArg {
            window: Some("Trade_LocalSlot1".into()),
            ..Default::default()
        };
        assert_eq!(
            drag_end("to", &win).unwrap(),
            DragEnd::Window("Trade_LocalSlot1".into())
        );
        assert!(drag_end("to", &DragEndArg::default())
            .unwrap_err()
            .contains("`to`"));
    }

    #[test]
    fn buttons_parse() {
        assert_eq!(mouse_button(None).unwrap(), 0);
        assert_eq!(mouse_button(Some("Right")).unwrap(), 1);
        assert!(mouse_button(Some("middle")).is_err());
    }
    #[test]
    fn client_window_read_description_lists_prompt() {
        use std::sync::Arc;

        use crate::client::BridgeClient;
        use crate::supervisor::{Supervisor, SupervisorConfig};

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
        let server = LabServer::new(Arc::new(Supervisor::new(bridge, config)));
        let tool = server
            .tool_router
            .get("client_window_read")
            .expect("client_window_read is routed");
        assert!(tool
            .description
            .as_deref()
            .unwrap_or_default()
            .contains("`prompt`"));
    }

    #[test]
    fn hub_pass_prompt_briefs_use_returned_button_names() {
        use std::{fs, path::Path};

        let briefs =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/analysis/hub-pass-uat/briefs");
        let mut checked = 0;
        let hardcoded_button = ["PromptN_Button", "1"].concat();
        let placeholder_button = ["Prompt<N>_Button", "1"].concat();

        for entry in fs::read_dir(briefs).expect("hub-pass briefs directory exists") {
            let path = entry.expect("brief directory entry").path();
            if path.extension().is_none_or(|extension| extension != "md")
                || !path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("HP-"))
            {
                continue;
            }

            let brief = fs::read_to_string(&path).expect("hub-pass brief is UTF-8");
            for line in brief.lines().filter(|line| line.contains("**PROMPT")) {
                assert!(
                    line.contains("client_window_read")
                        && line.contains("returned button")
                        && line.contains("`window`"),
                    "{} prompt macro does not use a returned button name",
                    path.display()
                );
            }
            assert!(
                !brief.contains(hardcoded_button.as_str()),
                "{} still assumes the first numbered prompt button",
                path.display()
            );
            assert!(
                !brief.contains(placeholder_button.as_str()),
                "{} still assumes the first numbered prompt button",
                path.display()
            );
            checked += 1;
        }

        assert!(checked > 0, "at least one hub-pass brief was checked");
    }
}
