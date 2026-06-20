---
name: reference-duel-design-notes
description: Duel state machine design: method indices, state layout, file skeleton, failure modes, integration points
metadata:
  type: reference
---

## Wire indices (confirmed from codebase)

**Inbound (client → server)** — SGWPlayer cell methods:
- `SEND_DUEL_RESPONSE` = 102 (currently UNIMPLEMENTED stub in `social.rs:92`)
- `DUEL_FORFEIT` = 103 (currently UNIMPLEMENTED stub in `social.rs:100`)
- No `DUEL_CHALLENGE` inbound index yet confirmed — RE needed. Likely in
  the 94–103 gap or between 94 (ORG_CREATION) and 101 (CLIENT_CHALLENGE_RESPONSE).
  Ghidra/binary RE must pin this before implementing the challenge handler.

**Outbound (server → client)** — SGWPlayer client methods:
- `ON_DUEL_CHALLENGE` = 143 (confirmed, `client_methods/player.rs:94`)
- `ON_DUEL_ENTITIES_SET` = 151
- `ON_DUEL_ENTITIES_REMOVE` = 152
- `ON_DUEL_ENTITIES_CLEAR` = 153

## State machine (designed, not yet implemented)

```
Idle ──challengeSent──> ChallengePending (both sides)
ChallengePending ──accept──> Active (after countdown)
ChallengePending ──decline/timeout/disconnect──> Idle (both sides)
Active ──HP=0 / forfeit / timeout / out-of-bounds / disconnect──> Ended
Ended ──cleanup (un-flag, clear state)──> Idle
```

Per-player state lives on `CellEntity` (no DB persistence). Two entities
reference each other by entity_id (same pattern as `trade_partner_entity_id`).

## State fields to add to CellEntity (no DB)

```rust
pub duel_partner_entity_id: Option<u32>,
pub duel_state: DuelState,       // Idle | Challenging | Active
pub pvp_flagged: bool,           // set on duel start, cleared on end
pub duel_arena_bounds: Option<DuelArenaBounds>, // [center: [f32;3], radius: f32]
```

## File skeleton

Under `crates/services/src/cell/cell_methods/player/duel/`:
- `mod.rs` — module wiring, re-exports `dispatch` + `cancel_duel_on_disconnect`
- `handlers.rs` — inbound dispatch (SEND_DUEL_RESPONSE=102, DUEL_FORFEIT=103, + TBD challenge method)
- `state.rs` — state-machine helpers: `begin_challenge`, `accept_duel`, `end_duel`, `clear_duel_state`
- `wire.rs` — outbound: `send_on_duel_challenge`, `send_on_duel_entities_set/remove/clear`, `send_pvp_flag_update`
- `tests/mod.rs`, `tests/handlers.rs`, `tests/state.rs`

Registration in `dispatch.rs`: duel methods fit inside the existing
`ORG_CREATION..=CANCEL_MOVIE` (94..=108) outer arm. The challenge inbound
method index must be RE'd before the static-assert guard can be updated.

## Integration points

**Combat:** duel-end hooks into `apply_death_transition` (death.rs) via a
duel_check after the dead-state flip. On player death, if attacker and
target are duel partners: do NOT deal lethal damage (stop at 1 HP or
treat 0 HP as duel-end trigger) and call `end_duel(winner, loser)` instead
of the normal PvP death path. Non-lethal end needs RE verification.

**AoI:** `ON_DUEL_ENTITIES_SET` / `REMOVE` / `CLEAR` are the broadcast
mechanism. These go to the two participants. Witnesses may see PvP flag
change via `onStateFieldUpdate` (BSF_PvP bit) — same fan-out as BSF_IN_COMBAT.

**Disconnect hook:** mirrors `cancel_trade_on_disconnect` in state.rs.
`cancel_duel_on_disconnect(entity_id, tx, space_mgr)` — checks
`duel_partner_entity_id`, calls `end_duel` with Disconnect reason, returns
`Option<u32>` partner for log correlation.

## Failure modes + guard requirements

1. **PvP flag stuck on** (invariant #6): every `end_duel` path MUST call `un_flag_pvp(entity_id)` AND `un_flag_pvp(partner_id)`. Disconnect, out-of-bounds, surrender, and timeout all terminate through `end_duel`. Regression guard must remove the un-flag call and confirm the test fails.
2. **Challenge timeout — no orphan**: challenge timer expiry calls `clear_duel_state` on both sides. If challenger disconnects mid-challenge, `cancel_duel_on_disconnect` runs before the timer fires; it must also clear the challengee's pending state. Guard: disconnect during ChallengePending leaves neither side with `duel_state != Idle`.
3. **Double-challenge**: if A already has `duel_state != Idle`, reject a new challenge from B. Same for self-challenge (`partner == self`). Guards mirror `begin_trading` checks in `state.rs:37-40`.
4. **Zone leave**: when a player's entity is destroyed (zone transfer / gate travel), the destroy hook calls `cancel_duel_on_disconnect`. Same path as disconnect.
5. **Arena-bounds violation**: an out-of-bounds tick check (run on the cell tick or movement handler) calls `end_duel(OutOfBounds)` for the violator. Both players get un-flagged.

## Pending RE work

- Confirm the inbound `duelChallenge` method index (send challenge to target). Likely in the 94–101 range; check Ghidra SGWPlayer CellMethods table.
- Confirm the wire payload of `ON_DUEL_CHALLENGE` (143): likely INT32 challenger_entity_id + optional challenge text.
- Confirm `ON_DUEL_ENTITIES_SET` (151) payload shape: two INT32 entity ids, or a FIXED_DICT?
- Confirm whether 0-HP duel end is "stop at 1 HP" (non-lethal) or "kill normally then respawn at arena" — check Ghidra SGWPlayer.onDead() path for duel flag check.
- Confirm countdown timer length from binary.

## Related memory

[[reference_contact_list_system]] — pattern for login/logout fan-out notification
