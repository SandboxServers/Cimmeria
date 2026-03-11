//! Mailbox abstractions for inter-component RPC.
//!
//! In the BigWorld architecture, entities are split across processes (BaseApp,
//! CellApp, client). A "mailbox" is a lightweight handle that knows how to
//! route a message to the correct remote half. Calling a method on a mailbox
//! serializes the arguments, wraps them in a Mercury message, and sends them
//! to the appropriate destination.
//!
//! There are three mailbox types:
//!
//! - **BaseMailbox** -- routes messages to the base entity on BaseApp.
//! - **CellMailbox** -- routes messages to the cell entity on CellApp.
//! - **ClientMailbox** -- routes messages to the player's game client.

use bytes::Bytes;

use cimmeria_common::{EntityId, SessionId, SpaceId};

/// Handle for sending messages to a base entity on the BaseApp.
#[derive(Clone, Debug)]
pub struct BaseMailbox {
    /// The target entity's runtime ID.
    pub entity_id: EntityId,
}

impl BaseMailbox {
    /// Create a new mailbox targeting the given base entity.
    pub fn new(entity_id: EntityId) -> Self {
        Self { entity_id }
    }

    /// Serialize and send a message to the base entity.
    ///
    /// `msg_id` is the Mercury message identifier; `payload` is the
    /// already-serialized argument data.
    ///
    /// # Panics
    ///
    /// Not yet implemented -- will look up the Mercury channel to the
    /// owning BaseApp and enqueue the message for delivery.
    pub fn send_message(&self, msg_id: u8, payload: Bytes) {
        let _ = (msg_id, payload);
        todo!("BaseMailbox::send_message - route via Mercury to BaseApp")
    }
}

/// Handle for sending messages to a cell entity on the CellApp.
#[derive(Clone, Debug)]
pub struct CellMailbox {
    /// The target entity's runtime ID.
    pub entity_id: EntityId,

    /// The space the cell entity currently resides in. Used to determine
    /// which CellApp instance owns the entity.
    pub space_id: SpaceId,
}

impl CellMailbox {
    /// Create a new mailbox targeting the given cell entity in the given space.
    pub fn new(entity_id: EntityId, space_id: SpaceId) -> Self {
        Self {
            entity_id,
            space_id,
        }
    }

    /// Serialize and send a message to the cell entity.
    ///
    /// `msg_id` is the Mercury message identifier; `payload` is the
    /// already-serialized argument data.
    ///
    /// # Panics
    ///
    /// Not yet implemented -- will look up the Mercury channel to the
    /// CellApp owning `space_id` and enqueue the message for delivery.
    pub fn send_message(&self, msg_id: u8, payload: Bytes) {
        let _ = (msg_id, payload);
        todo!("CellMailbox::send_message - route via Mercury to CellApp")
    }
}

/// Handle for sending messages to a player's game client.
#[derive(Clone, Debug)]
pub struct ClientMailbox {
    /// The entity ID of the player entity this client is attached to.
    pub entity_id: EntityId,

    /// The session identifier assigned at login, used to look up the
    /// client's Mercury channel on the BaseApp.
    pub session_id: SessionId,
}

impl ClientMailbox {
    /// Create a new mailbox targeting the given client session.
    pub fn new(entity_id: EntityId, session_id: SessionId) -> Self {
        Self {
            entity_id,
            session_id,
        }
    }

    /// Serialize and send a message to the client.
    ///
    /// `msg_id` is the Mercury message identifier; `payload` is the
    /// already-serialized argument data.
    ///
    /// # Panics
    ///
    /// Not yet implemented -- will look up the client's UDP channel on
    /// the BaseApp and enqueue the message for delivery.
    pub fn send_message(&self, msg_id: u8, payload: Bytes) {
        let _ = (msg_id, payload);
        todo!("ClientMailbox::send_message - route via Mercury to client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_mailbox_new() {
        let mb = BaseMailbox::new(EntityId(42));
        assert_eq!(mb.entity_id, EntityId(42));
    }

    #[test]
    fn cell_mailbox_new() {
        let mb = CellMailbox::new(EntityId(7), SpaceId(100));
        assert_eq!(mb.entity_id, EntityId(7));
        assert_eq!(mb.space_id, SpaceId(100));
    }

    #[test]
    fn client_mailbox_new() {
        let mb = ClientMailbox::new(EntityId(1), SessionId(9001));
        assert_eq!(mb.entity_id, EntityId(1));
        assert_eq!(mb.session_id, SessionId(9001));
    }
}
