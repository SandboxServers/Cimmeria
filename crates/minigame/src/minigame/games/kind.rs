//! The registry of minigame kinds: the client type-table names, and who
//! validates a victory for each.

use super::{alignment, converse_humanoid, crystal_game, goauld_crystals};

/// Every minigame type the client can name in a `StartMinigame`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameKind {
    Livewire,
    Alignment,
    CrystalGame,
    GoauldCrystals,
    ConverseBasicHumanoid,
    Hack,
    Activate,
    Analyze,
    Bypass,
    Converse,
}

/// Who checks a victory: the server itself, or the SWF's own `victory` command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Validation {
    Server,
    ClientDeclared,
}

impl Validation {
    /// The name this validation reports under in the content reference.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Server => "server",
            Self::ClientDeclared => "client_declared",
        }
    }
}

impl GameKind {
    /// Every kind, in the order of the client type table.
    pub const ALL: [GameKind; 10] = [
        GameKind::Livewire,
        GameKind::Alignment,
        GameKind::CrystalGame,
        GameKind::GoauldCrystals,
        GameKind::ConverseBasicHumanoid,
        GameKind::Hack,
        GameKind::Activate,
        GameKind::Analyze,
        GameKind::Bypass,
        GameKind::Converse,
    ];

    /// The kind for a client type-table name. The match is exact and
    /// case-sensitive, so `"livewire"` is not a kind.
    pub fn from_name(name: &str) -> Option<GameKind> {
        Self::ALL.into_iter().find(|kind| kind.name() == name)
    }

    /// The client type-table name, which is also the `target_key`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Livewire => "Livewire",
            Self::Alignment => "Alignment",
            Self::CrystalGame => "CrystalGame",
            Self::GoauldCrystals => "GoauldCrystals",
            Self::ConverseBasicHumanoid => "ConverseBasicHumanoid",
            Self::Hack => "Hack",
            Self::Activate => "Activate",
            Self::Analyze => "Analyze",
            Self::Bypass => "Bypass",
            Self::Converse => "Converse",
        }
    }

    /// Livewire is server-validated. A port is server-validated only once its
    /// module's `PORTED` flag is set; every other kind is client-declared.
    pub fn validation(self) -> Validation {
        let server = match self {
            Self::Livewire => true,
            Self::Alignment => alignment::PORTED,
            Self::CrystalGame => crystal_game::PORTED,
            Self::GoauldCrystals => goauld_crystals::PORTED,
            Self::ConverseBasicHumanoid => converse_humanoid::PORTED,
            Self::Hack | Self::Activate | Self::Analyze | Self::Bypass | Self::Converse => false,
        };
        if server {
            Validation::Server
        } else {
            Validation::ClientDeclared
        }
    }
}

#[cfg(test)]
#[path = "kind_tests.rs"]
mod tests;
