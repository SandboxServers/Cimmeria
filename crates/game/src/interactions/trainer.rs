use serde::{Deserialize, Serialize};

/// A trainable ability offered by a trainer NPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainableAbility {
    pub ability_id: i32,
    pub name: String,
    pub required_level: u32,
    pub cost: i64,
    pub already_learned: bool,
}

/// Trainer interaction state for a player browsing an NPC's ability list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainerSession {
    pub trainer_entity_id: i32,
    pub player_entity_id: i32,
    pub trainer_list_id: i32,
    pub abilities: Vec<TrainableAbility>,
}

impl TrainerSession {
    /// Open a trainer session, loading available abilities.
    pub fn open(_trainer_entity_id: i32, _player_entity_id: i32, _trainer_list_id: i32) -> Self {
        todo!("TrainerSession::open - load trainer_abilities + trainer_ability_lists from DB")
    }

    /// Player learns an ability from the trainer.
    pub fn learn_ability(&mut self, _ability_id: i32) -> TrainerResult {
        todo!("TrainerSession::learn_ability - check level, money, add ability to player")
    }
}

/// Result of a trainer transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrainerResult {
    Success,
    LevelTooLow,
    NotEnoughMoney,
    AlreadyLearned,
    AbilityNotFound,
}
