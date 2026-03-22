//! SGWSpawnableEntity ClientMethods (indices 0–11).

/// Update static mesh and body set names.
pub const ON_STATIC_MESH_NAME_UPDATE: u16 = 0;
/// Play a kismet sequence/animation.
pub const ON_SEQUENCE: u16 = 1;
/// Force entity movement (teleport/slide).
pub const ON_ENTITY_MOVE: u16 = 2;
/// Set the entity's interaction type bitfield.
pub const INTERACTION_TYPE: u16 = 3;
/// Set the entity's flags bitfield.
pub const ON_ENTITY_FLAGS: u16 = 4;
/// Request available interactions from entity.
pub const GET_INTERACTIONS: u16 = 5;
/// Toggle interaction debug overlay.
pub const TOGGLE_INTERACTION_DEBUGGING: u16 = 6;
/// Set a generic entity property (INT32 type, INT32 value).
pub const ON_ENTITY_PROPERTY: u16 = 7;
/// Show/hide the entity.
pub const ON_VISIBLE: u16 = 8;
/// Update the entity's kismet event set.
pub const ON_KISMET_EVENT_SET_UPDATE: u16 = 9;
/// Set entity tint colors.
pub const ON_ENTITY_TINT: u16 = 10;
/// Update the entity's localized name ID.
pub const ON_BEING_NAME_ID_UPDATE: u16 = 11;
