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
        description = "Read an open UI window: title, visible buttons with enabled state, visible text, list rows (Listbox / MultiColumnList with selection), node rectangles on request. `window` reads any CEGUI window by name; `kind` is a typed reader that also returns the module's state through the stock bindings: vault/bank (vault items), org_vault, trainer (abilities with id, cost, trainable), discipline_trainer, crafting (allowed per craft type, known blueprint counts), pet, organization, mail (headers), loot (items), dhd (active), dialog/blurb, greet (topics), vendor (stock), trade, character. Native level: client_ui_lua (read)."
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
        description = "Chat lines the client showed (network chat, feedback and /tell errors, Server Message lines, combat chatter), with channel name and number, speaker and speaker-flag names, the colour and tabs the chat window uses, and the event store seq. Reads the lab's one chat capture (the chat.line ring that wraps ChatMod.onMessageReceived, pumped into the supervisor's event store) through its own named `cursor`, so it never takes lines from client_wait_event or client_events_read; `since_seq` overrides the cursor, `peek` does not advance it. Filters: channel, contains, speaker. Lines shown before the ring was first installed, and centre-screen splash text, are not in the log. Native tier: read."
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
        description = "Act on an item like a player. use / equip / unequip / rightclick: a real right-click on the item's slot (the stock UI's contextSensitiveUseItem), opening the inventory or character window with its bound key and clicking the right tab first; doubleclick: a double left-click. loot_all: click Loot All; loot_slot: page the loot window and double-click row `loot_index` (1-based). Pick the item by container + slot, item_id or name. When the slot cannot be put on screen, falls back to the client's slash command (/useitem, /equip, /lootitem) and reports native_level slash_command. Returns the inventory diff observed within wait_ms."
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
                    Duration::from_millis(a.wait_ms.unwrap_or(DEFAULT_WAIT_MS).min(60_000)),
                )
                .await
        }
        .await;
        ui_result("client_item_action", t0, r)
    }

    #[tool(
        description = "Drag an item from one UI slot to another with real input: press on the source slot, walk the cursor to the target in steps, release; `split` holds Ctrl (the stock UI pulls one item off the stack; its Shift-drag split is unimplemented). Ends are container + slot (the slot's window is put on screen first) or a named window (a trade, crafting or vault slot). Verified by an inventory diff: returns drag_started, moved, snap_back and the diff. If posted motion does not start a CEGUI drag, the motion is replayed through CEGUI's injectMousePosition (reported as client_ui_lua)."
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
}
