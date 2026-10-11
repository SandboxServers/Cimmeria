//! Entity mirror: what the client has been told about the world, kept as the
//! wire delivers it.
//!
//! The mirror is fed one [`S2CMessage`] at a time. Introductions
//! (`CREATE_BASE_PLAYER`, `CREATE_ENTITY`) add an entry, `leaveAoI` removes it,
//! and the semantic events of entity methods fill its fields in. A test then
//! asks "which entity is the body the client was told about?" with an
//! [`EntityQuery`] instead of hard-coding an entity id.

mod query;

pub use query::EntityQuery;

use std::collections::BTreeMap;

use cimmeria_mercury::channel_bundle::{IDBASE_NPC_DEFAULT, IDBASE_SGW_PLAYER};

use crate::bundle::S2CMessage;
use crate::semantic::{decode_event, InvItem, S2CEvent, SemanticError, TargetKind};

/// One entity as the client was told about it.
#[derive(Debug, Clone, PartialEq)]
pub struct MirroredEntity {
    /// The entity id the introduction carried.
    pub entity_id: u32,
    /// The class id the introduction carried.
    pub class_id: u8,
    /// From the avatar update (`0x10`) that follows the introduction. Stays
    /// `None` until one arrives.
    pub position: Option<[f32; 3]>,
    /// From `onStaticMeshNameUpdate`.
    pub static_mesh: Option<String>,
    /// From `onStaticMeshNameUpdate`, next to `static_mesh`.
    pub body_set: Option<String>,
    /// From `onBeingNameIDUpdate`.
    pub name_id: Option<i32>,
    /// From `InteractionType`. Zero means nothing to interact with.
    pub interaction_type: Option<u64>,
    /// From `onEntityProperty`, keyed by property id.
    pub properties: BTreeMap<i32, i32>,
    /// From `onVisible`.
    pub visible: Option<i8>,
    /// Set by `entityInvisible`, cleared by a re-introduction.
    pub hidden: bool,
    /// Order of introduction, 0-based, across the session.
    pub introduced: u64,
}

/// Why a mirror lookup or an entity method failed.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MirrorError {
    /// A known entity method whose bytes do not fit its decoder.
    #[error("entity {entity_id} method {method_index}: {source}")]
    Semantic {
        /// The target the method was sent to.
        entity_id: u32,
        /// The method index the decoder was asked for.
        method_index: u16,
        /// What the decoder rejected.
        source: SemanticError,
    },
    /// No mirrored entity matches the query.
    #[error("no entity matches {query}")]
    NotFound {
        /// The query, rendered for the message.
        query: String,
    },
    /// More than one mirrored entity matches the query.
    #[error("{query} matches {ids:?}")]
    Ambiguous {
        /// The query, rendered for the message.
        query: String,
        /// Every matching entity id, ascending.
        ids: Vec<u32>,
    },
}

/// The client's view of the world, built by applying server messages in order.
#[derive(Debug, Default)]
pub struct EntityMirror {
    own_id: Option<u32>,
    entities: BTreeMap<u32, MirroredEntity>,
    inventory: BTreeMap<i32, InvItem>,
    orphans: Vec<S2CMessage>,
    next_intro: u64,
}

impl EntityMirror {
    /// An empty mirror: no own player, no entities.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record which entity is the player's own character.
    pub fn set_own_id(&mut self, id: u32) {
        self.own_id = Some(id);
    }

    /// The player's own entity id, once `CREATE_BASE_PLAYER` or
    /// [`EntityMirror::set_own_id`] has set it.
    pub fn own_id(&self) -> Option<u32> {
        self.own_id
    }

    /// The mirrored entity with this id, if the client holds it.
    pub fn get(&self, entity_id: u32) -> Option<&MirroredEntity> {
        self.entities.get(&entity_id)
    }

    /// How many entities the mirror holds.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// True when the mirror holds no entity.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// The player's inventory, by item instance id.
    pub fn inventory(&self) -> &BTreeMap<i32, InvItem> {
        &self.inventory
    }

    /// Entity methods that arrived for an entity the mirror does not hold.
    pub fn orphans(&self) -> &[S2CMessage] {
        &self.orphans
    }

    /// Apply one message. Returns its semantic event, if any, for the caller's log.
    pub fn apply(&mut self, msg: &S2CMessage) -> Result<Option<S2CEvent>, MirrorError> {
        match msg.msg_id {
            0x05 | 0x09 => {
                if let (Some(id), Some(class_id)) = (msg.entity_id, msg.class_id) {
                    if msg.msg_id == 0x05 {
                        self.set_own_id(id);
                    }
                    self.introduce(id, class_id);
                }
                Ok(None)
            }
            0x0B => {
                if let Some(entity) = msg.entity_id.and_then(|id| self.entities.get_mut(&id)) {
                    entity.hidden = true;
                }
                Ok(None)
            }
            0x0C => {
                if let Some(id) = msg.entity_id {
                    self.entities.remove(&id);
                }
                Ok(None)
            }
            0x10 => {
                if let (Some(id), Some(position)) = (msg.entity_id, position_of(&msg.payload)) {
                    if let Some(entity) = self.entities.get_mut(&id) {
                        entity.position = Some(position);
                    }
                }
                Ok(None)
            }
            // TODO(WC-11): other avatar-update variants; the Praxis start needs only 0x10
            0x11..=0x30 => Ok(None),
            0x80..=0xFE => self.apply_method(msg),
            _ => Ok(None),
        }
    }

    /// Every mirrored entity the query matches, in id order.
    pub fn find(&self, q: &EntityQuery) -> Vec<&MirroredEntity> {
        self.entities.values().filter(|e| q.matches(e)).collect()
    }

    /// Exactly one match, or NotFound / Ambiguous.
    pub fn find_one(&self, q: &EntityQuery) -> Result<&MirroredEntity, MirrorError> {
        let matches = self.find(q);
        match matches.as_slice() {
            [only] => Ok(only),
            [] => Err(MirrorError::NotFound {
                query: q.to_string(),
            }),
            many => Err(MirrorError::Ambiguous {
                query: q.to_string(),
                ids: many.iter().map(|e| e.entity_id).collect(),
            }),
        }
    }

    /// Insert an entry for a new introduction. A re-introduction of an id the
    /// mirror already holds replaces the whole entry, so no field of the old
    /// one survives; the client rebuilds the entity from the new introduction.
    fn introduce(&mut self, entity_id: u32, class_id: u8) {
        let introduced = self.next_intro;
        self.next_intro += 1;
        self.entities.insert(
            entity_id,
            MirroredEntity {
                entity_id,
                class_id,
                position: None,
                static_mesh: None,
                body_set: None,
                name_id: None,
                interaction_type: None,
                properties: BTreeMap::new(),
                visible: None,
                hidden: false,
                introduced,
            },
        );
    }

    /// Decode an entity method against the table its target uses, then apply
    /// the event. A target the mirror does not hold is kept as an orphan.
    fn apply_method(&mut self, msg: &S2CMessage) -> Result<Option<S2CEvent>, MirrorError> {
        let class_id = msg
            .entity_id
            .and_then(|id| self.entities.get(&id))
            .map(|entity| entity.class_id);
        let (Some(target), Some(class_id)) = (msg.entity_id, class_id) else {
            self.orphans.push(msg.clone());
            return Ok(None);
        };
        let kind = if self.own_id == Some(target) || matches!(class_id, 2 | 3) {
            TargetKind::Player
        } else {
            TargetKind::Npc
        };
        let Some(index) = method_index(msg, kind) else {
            self.orphans.push(msg.clone());
            return Ok(None);
        };
        let event =
            decode_event(kind, index, msg.args().unwrap_or_default()).map_err(|source| {
                MirrorError::Semantic {
                    entity_id: target,
                    method_index: index,
                    source,
                }
            })?;
        if let Some(event) = &event {
            self.apply_event(target, event);
        }
        Ok(event)
    }

    /// Fold one semantic event into the entry it is about. Inventory events
    /// only change the own player's inventory.
    fn apply_event(&mut self, target: u32, event: &S2CEvent) {
        let own = self.own_id == Some(target);
        match event {
            S2CEvent::UpdateItem { items } if own => {
                for item in items {
                    self.inventory.insert(item.id, item.clone());
                }
            }
            S2CEvent::RemoveItem { item_ids } if own => {
                for id in item_ids {
                    self.inventory.remove(id);
                }
            }
            _ => {
                let Some(entity) = self.entities.get_mut(&target) else {
                    return;
                };
                match event {
                    S2CEvent::StaticMeshNameUpdate {
                        static_mesh,
                        body_set,
                    } => {
                        entity.static_mesh = Some(static_mesh.clone());
                        entity.body_set = Some(body_set.clone());
                    }
                    S2CEvent::BeingNameIdUpdate { name_id } => entity.name_id = Some(*name_id),
                    S2CEvent::InteractionType { type_id } => {
                        entity.interaction_type = Some(*type_id);
                    }
                    S2CEvent::EntityProperty { property, value } => {
                        entity.properties.insert(*property, *value);
                    }
                    S2CEvent::Visible { visible } => entity.visible = Some(*visible),
                    _ => {}
                }
            }
        }
    }
}

/// The method index of an entity-method message under the idbase its target
/// uses. The idbase matters only for the `0xBD` sub-slot, so passing the
/// message's own [`S2CMessage::method_index`] (always the player's 61) would
/// misread every NPC method that sits past 61.
fn method_index(msg: &S2CMessage, kind: TargetKind) -> Option<u16> {
    let idbase = match kind {
        TargetKind::Player => IDBASE_SGW_PLAYER,
        TargetKind::Npc => IDBASE_NPC_DEFAULT,
    };
    msg.method_index_for(idbase)
}

/// The avatar update's position: three little-endian `f32` after the 4-byte
/// entity id. `None` when the payload is too short to hold them.
fn position_of(payload: &[u8]) -> Option<[f32; 3]> {
    let (chunks, _) = payload.get(4..16)?.as_chunks::<4>();
    let [x, y, z] = chunks else {
        return None;
    };
    Some([
        f32::from_le_bytes(*x),
        f32::from_le_bytes(*y),
        f32::from_le_bytes(*z),
    ])
}

#[cfg(test)]
mod tests;
