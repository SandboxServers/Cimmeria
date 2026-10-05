---
name: cell-gm-class-id-and-raw-bundle-blobs
description: Cell player entities are always class_id 0x02 (GMs too), and ChannelBundle raw appends can hold several Mercury messages; both mislead per-entity-type naming and per-message bookkeeping
metadata:
  type: project
---

Two server-side facts found while reviewing PR #1201 (NT-30 names), 2026-10-04:

- `SpaceManager::connect_entity` (`crates/cell-world/src/cell/space_manager/entities.rs`) sets `class_id = 0x02` for every player. No cell crate ever sets 0x03 (SGWGmPlayer). So a lookup keyed by the cell entity's `class_id` misses the GM tail: ClientMethods 157-162 and CellMethods 109-225. The base keeps the real class in `ConnectedClientState::player_class_id`.
- `ChannelBundle::append_raw_message` takes pre-composed blobs that can hold several messages, and counts each blob as one message. Examples: `compose_create_entity_base_body` is createEntity 0x09 followed by avatarUpdate 0x10, and `compose_cascade_body` is the whole createOnClient cascade. Any per-message bookkeeping (num_messages, message heads) must frame the body with `packet::server_message_framing`, not trust the append calls.

**Why:** both broke NT-30's naming: the GM tail went unnamed on the cell, and tx_hole named the wrong message.
**How to apply:** when naming or counting messages on the cell or in bundles, use the GM superset for players and frame the body. Related: [[sgwplayer-method-index-table]], [[aoi-entity-introduction]].
