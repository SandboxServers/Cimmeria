//! Server→client entity method indices for SGWPlayer.
//!
//! These constants define the flattened ClientMethod indices used in entity
//! method call packets sent FROM the server TO the client. They are a DIFFERENT
//! index space from the cell method indices in `dispatch.rs` (client→server).
//!
//! BigWorld flattening order (from `entity_description.cpp:parseInterface()`):
//! For each entity in the inheritance chain (root→leaf):
//!   1. Parse `<Implements>` interfaces FIRST (recursively, in document order)
//!   2. Then parse the entity's OWN `<ClientMethods>`
//!
//! See `docs/protocol/client-method-dispatch-table.md` for the complete
//! 157-method table with args and wire encoding details.

pub mod spawnable_entity;
pub mod being;
pub mod combatant;
pub mod communicator;
pub mod organization;
pub mod minigame;
pub mod gate_travel;
pub mod inventory;
pub mod mail;
pub mod missionary;
pub mod contact_list;
pub mod black_market;
pub mod client_cache;
pub mod player;
