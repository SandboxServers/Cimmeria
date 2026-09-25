//! Read access to [`CellEntity::ai_state`] and its one raw writer.
//!
//! The field is private so that a new `npc.ai_state = X` anywhere in the
//! workspace is a compile error. Production code changes the state through
//! `cimmeria_services::cell::service::npc_ai::transition::set_ai_state`,
//! which logs the transition (`npc_ai.transition`, `event="state_change"`)
//! and counts it. This crate cannot host that helper: the row carries the
//! world name and the metric goes through `cimmeria-observability`, neither
//! of which the entity crate knows about.

use super::{AiState, CellEntity};

impl CellEntity {
    /// The NPC's current AI state.
    pub fn ai_state(&self) -> AiState {
        self.ai_state
    }

    /// Overwrite the AI state **without logging**, returning the previous
    /// state.
    ///
    /// Do not call this. It exists only so the services transition helper
    /// (and that module's test-only `force_ai_state`) can write the private
    /// field. The guard test
    /// `raw_ai_state_writer_is_called_only_from_the_transition_helper` in
    /// `crates/services/src/cell/service/npc_ai/transition.rs` fails if any
    /// other file in the workspace names this method.
    #[doc(hidden)]
    pub fn replace_ai_state_unlogged(&mut self, to: AiState) -> AiState {
        std::mem::replace(&mut self.ai_state, to)
    }
}

impl AiState {
    /// Stable snake_case label for logs and metric labels. Treat these
    /// strings as API: SigNoz queries and dashboards group on them.
    pub fn label(self) -> &'static str {
        match self {
            AiState::Spawning => "spawning",
            AiState::Idle => "idle",
            AiState::Investigating => "investigating",
            AiState::Fighting => "fighting",
            AiState::Leashing => "leashing",
            AiState::Dead => "dead",
            AiState::Despawning => "despawning",
            AiState::Follow => "follow",
            AiState::Patrol => "patrol",
            AiState::Wander => "wander",
            AiState::Submit => "submit",
            AiState::Error => "error",
        }
    }
}
