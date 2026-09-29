//! Argument types for the UI reader and item tools.

use rmcp::schemars;

/// Args for `client_window_read`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct WindowReadArgs {
    /// A CEGUI window name (a Lua global such as `LootWin`, or
    /// `Parent/Child`). Give this or `kind`.
    #[serde(default)]
    pub window: Option<String>,
    /// A typed reader: vault (bank), org_vault, trainer, discipline_trainer,
    /// crafting, pet, organization, mail, loot, dhd, dialog (blurb), greet,
    /// vendor, trade, character. Adds the module's state (loot items,
    /// trainer abilities with cost, mail headers, vendor stock, ...).
    #[serde(default)]
    pub kind: Option<String>,
    /// Walk depth below the root (default 8).
    #[serde(default)]
    pub max_depth: Option<u32>,
    /// Node cap (default 400).
    #[serde(default)]
    pub max_nodes: Option<u32>,
    /// Include hidden children (default false).
    #[serde(default)]
    pub include_hidden: Option<bool>,
    /// Rows read per list (default 100).
    #[serde(default)]
    pub max_rows: Option<u32>,
    /// Return every walked node (name, type, text, rect) besides the summary.
    #[serde(default)]
    pub include_nodes: Option<bool>,
}

/// Args for `client_window_click`. Give exactly one target: `target`,
/// `root` + `text`, or `list` + `row_index`/`row_text`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct WindowClickArgs {
    /// A named window to click at its centre (a button, a named row).
    #[serde(default)]
    pub target: Option<String>,
    /// Window to search for `text`.
    #[serde(default)]
    pub root: Option<String>,
    /// Click the visible widget under `root` whose text equals (else
    /// contains) this, case-insensitive.
    #[serde(default)]
    pub text: Option<String>,
    /// A Listbox or MultiColumnList whose row to click.
    #[serde(default)]
    pub list: Option<String>,
    /// Row index, 0-based.
    #[serde(default)]
    pub row_index: Option<u32>,
    /// Row by text (substring, case-insensitive; first cell of a
    /// multi-column list).
    #[serde(default)]
    pub row_text: Option<String>,
    /// `left` (default) or `right`.
    #[serde(default)]
    pub button: Option<String>,
    /// Double-click (default false).
    #[serde(default)]
    pub double: Option<bool>,
    /// When a row click does not select the row, select it through the
    /// list's Lua and report that level (default true).
    #[serde(default)]
    pub allow_lua_fallback: Option<bool>,
}

/// Args for `client_chat_log`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct ChatLogArgs {
    /// Named read cursor (default `default`). Each cursor returns only the
    /// lines after its last read.
    #[serde(default)]
    pub cursor: Option<String>,
    /// Read after this sequence number instead of the cursor (0 = the
    /// whole ring).
    #[serde(default)]
    pub since_seq: Option<u64>,
    /// Channel name (`Say`, `Tell`, `Feedback`, `Server`, `Combat`,
    /// `Splash`, a custom channel) or number.
    #[serde(default)]
    pub channel: Option<String>,
    /// Text substring, case-insensitive.
    #[serde(default)]
    pub contains: Option<String>,
    /// Speaker substring, case-insensitive.
    #[serde(default)]
    pub speaker: Option<String>,
    /// Max lines returned (default 200).
    #[serde(default)]
    pub max: Option<usize>,
    /// Read without moving the cursor.
    #[serde(default)]
    pub peek: Option<bool>,
}

/// Args for `client_inventory`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct InventoryArgs {
    /// Only these containers (`Main`, `Mission`, `Crafting`, `Bandolier`,
    /// `Vault`, `Chest`, ...). Default: all.
    #[serde(default)]
    pub containers: Vec<String>,
    /// List empty slots too (default false).
    #[serde(default)]
    pub include_empty: Option<bool>,
    /// Store this read under a label for a later `diff_against`.
    #[serde(default)]
    pub snapshot: Option<String>,
    /// Diff this read against the snapshot with this label.
    #[serde(default)]
    pub diff_against: Option<String>,
}

/// Args for `client_player_state`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct PlayerStateArgs {
    /// Read every stat the client reports (default true).
    #[serde(default)]
    pub stats: Option<bool>,
    /// Read the player's and the target's effects (default true).
    #[serde(default)]
    pub effects: Option<bool>,
}

/// Args for `client_item_action`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct ItemActionArgs {
    /// use, equip, unequip, rightclick, doubleclick, loot_all, loot_slot.
    pub action: String,
    /// Container of the item (`Main`, `Bandolier`, `Chest`, ...), with `slot`.
    #[serde(default)]
    pub container: Option<String>,
    /// Slot (1-based, as the client numbers them), with `container`.
    #[serde(default)]
    pub slot: Option<u32>,
    /// The item's instance id (`getItemIDForSlot`).
    #[serde(default)]
    pub item_id: Option<i64>,
    /// Item name substring, case-insensitive (first match).
    #[serde(default)]
    pub name: Option<String>,
    /// For loot_slot: the loot row, 1-based.
    #[serde(default)]
    pub loot_index: Option<u32>,
    /// Fall back to the client's slash command when the item cannot be put
    /// on screen (default true).
    #[serde(default)]
    pub allow_fallback: Option<bool>,
    /// How long to wait for the inventory to change, ms (default 3000).
    #[serde(default)]
    pub wait_ms: Option<u64>,
}

/// One end of a drag: a slot (`container` + `slot`) or a named window.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct DragEndArg {
    #[serde(default)]
    pub container: Option<String>,
    #[serde(default)]
    pub slot: Option<u32>,
    #[serde(default)]
    pub window: Option<String>,
}

/// Args for `client_drag_drop`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct DragDropArgs {
    pub from: DragEndArg,
    pub to: DragEndArg,
    /// Ctrl-drag: pull one item off the stack (the stock UI's split).
    #[serde(default)]
    pub split: Option<bool>,
    /// Cursor steps between press and release (default 8).
    #[serde(default)]
    pub steps: Option<u32>,
    /// Replay the motion through CEGUI's input injection when the posted
    /// motion does not start a drag (default true).
    #[serde(default)]
    pub allow_fallback: Option<bool>,
    /// How long to wait for the inventory to change, ms (default 3000).
    #[serde(default)]
    pub wait_ms: Option<u64>,
}
