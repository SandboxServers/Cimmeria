//! Abilities, combat and defeat: the supervisor side of `client_hotbar`,
//! `client_use_ability`, `client_combat_log` and `client_die_and_respawn`
//! (`client_wait_event` lives with the event store in
//! [`super::events`]).
//!
//! - [`binding`] — `getBindingKey` tables and the lab key for a VK code.
//! - [`hotbar`] — read the action bar (`ActionButtonMod`).
//! - [`use_ability`] — find an ability and fire it like a player.
//! - [`combat_log`] — read the floating-combat-text ring.
//! - [`defeat`] — die (setup), handle the defeat window, respawn.
//! - [`player`] — the player-state read the flows compare before/after.
//!
//! Every result says which *native level* drove the step under test (the
//! owner's rule: real input > slash command > the client UI's own Lua >
//! server shortcut), and GM setup is reported as setup, never as the step.

pub mod binding;
pub mod combat_log;
pub mod defeat;
#[cfg(test)]
mod flow_tests;
pub mod hotbar;
pub mod player;
pub mod use_ability;

use serde_json::{json, Value};

/// How a step drove the client, best first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeLevel {
    /// A real key press or mouse click through the lab's hooked input.
    RealInput,
    /// A slash command typed into chat (the client builds the call).
    SlashCommand,
    /// A call into the stock UI's own Lua (what a button handler runs,
    /// minus the input event).
    UiLuaCall,
    /// A read of the stock UI's Lua state (no action).
    UiLuaRead,
    /// GM setup typed into chat: setup only, never the behaviour tested.
    GmSetup,
    /// A server-side shortcut: evidence or emergency setup only.
    ServerShortcut,
}

impl NativeLevel {
    /// The UAT matrix tier (N1/N2/N3/G/X; reads are `read`).
    pub fn tier(self) -> &'static str {
        match self {
            Self::RealInput => "N1",
            Self::SlashCommand => "N2",
            Self::UiLuaCall => "N3",
            Self::UiLuaRead => "read",
            Self::GmSetup => "G",
            Self::ServerShortcut => "X",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::RealInput => "real input",
            Self::SlashCommand => "slash command",
            Self::UiLuaCall => "client UI Lua call",
            Self::UiLuaRead => "client UI Lua read",
            Self::GmSetup => "GM setup (chat command)",
            Self::ServerShortcut => "server shortcut",
        }
    }

    /// Position in the owner's preference order for driving the step under
    /// test (1 = best); reads and GM setup are not ranked.
    pub fn rank(self) -> Option<usize> {
        PREFERENCE.iter().position(|l| *l == self).map(|i| i + 1)
    }

    pub fn to_json(self) -> Value {
        json!({ "tier": self.tier(), "label": self.label(), "rank": self.rank() })
    }
}

/// Real input > slash command > the client UI's own Lua > server shortcut.
pub const PREFERENCE: [NativeLevel; 4] = [
    NativeLevel::RealInput,
    NativeLevel::SlashCommand,
    NativeLevel::UiLuaCall,
    NativeLevel::ServerShortcut,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_follow_the_uat_matrix() {
        assert_eq!(NativeLevel::RealInput.tier(), "N1");
        assert_eq!(NativeLevel::SlashCommand.tier(), "N2");
        assert_eq!(NativeLevel::UiLuaCall.tier(), "N3");
        assert_eq!(NativeLevel::GmSetup.tier(), "G");
        assert_eq!(NativeLevel::ServerShortcut.tier(), "X");
        assert_eq!(NativeLevel::RealInput.to_json()["label"], "real input");
        assert_eq!(NativeLevel::RealInput.rank(), Some(1));
        assert_eq!(NativeLevel::ServerShortcut.rank(), Some(4));
        assert_eq!(NativeLevel::GmSetup.rank(), None);
    }
}
