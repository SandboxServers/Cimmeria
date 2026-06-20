---
name: reference-witness-fanout-helper
description: Fanout helpers in messaging.rs — signatures, semantics, entity_is_player idbase threading
metadata:
  type: reference
---

File: `crates/services/src/cell/abilities/messaging.rs`

## Three fanout helpers

```rust
// Player → self only; NPC → witnesses only. Entity-aware default.
pub(crate) async fn send_entity_method(
    entity_id, method_index, args, tx, space_mgr
)

// Strict witness-only. Never sends to entity's own client. Returns witness count.
pub(crate) async fn send_entity_method_to_witnesses(
    entity_id, method_index, args, tx, space_mgr
) -> usize

// Owner + all witnesses. For NPCs collapses to witnesses-only. Returns witness count.
pub(crate) async fn send_entity_method_to_self_and_witnesses(
    entity_id, method_index, args, tx, space_mgr
) -> usize
```

No `#[allow(dead_code)]` — all three have production callsites as of commit `163be645`.

## entity_is_player threading

`CellToBaseMsg::WitnessEntityMethod` carries `entity_is_player: bool` (added in commit `163be645`).

`send_entity_method_to_witnesses` (and the `send_entity_method` NPC path) compute:
```rust
let entity_is_player = space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player);
```
once before the witness loop and stamp it on every `WitnessEntityMethod`.

`aoi.rs::witness_entity_method` selects:
```rust
let idbase = if entity_is_player { IDBASE_SGW_PLAYER } else { IDBASE_NPC_DEFAULT };
```

This is required because SGWPlayer has 157 exposed methods (idbase=61) vs NPC ≤62 (idbase=62). Method indices ≥61 encode as two-byte extended under idbase=61 but as single-byte direct under idbase=62. Wrong selection = corrupt wire byte.
