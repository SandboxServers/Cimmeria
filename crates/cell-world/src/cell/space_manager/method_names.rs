//! [`SpaceManager::client_method_name`]: a client-method index named for
//! the entity it is about, for logs.

use cimmeria_wire::names;

use super::SpaceManager;

impl SpaceManager {
    /// The name of client method `index` on `entity_id`, read from that
    /// entity's own type table (`SGWPlayer`, `SGWMob`, `SGWPet`, ...: indices
    /// 27-31 mean different methods on each). An entity the manager no longer
    /// holds falls back to the name every in-world type agrees on, which is
    /// `None` for 27-31. Log-only: resolve it inside the logging macro.
    pub fn client_method_name(&self, entity_id: u32, index: u16) -> Option<&'static str> {
        match self.get_entity(entity_id) {
            // A player's table is SGWGmPlayer's: the cell keeps every player
            // at class 0x02, and the GM tail (157-162) extends SGWPlayer's.
            Some(entity) if entity.is_player => names::player_client_method(index),
            Some(entity) => names::client_method(entity.class_id, index),
            None => names::any_entity_client_method(index),
        }
    }
}
