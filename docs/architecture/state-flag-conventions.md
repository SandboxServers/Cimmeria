# State-Flag Write Conventions

> **Last updated**: 2026-05-03
> **Audience**: Engineers touching `CellEntity.state_field` (BSF_*) or any
> future `combatantState`-style flag map
> **Type**: Reference + decision guide
> **Owner**: Combat / entity systems

## TL;DR

`state_field` writes pick **one** of two patterns per flag, and never mix
them within a single flag's writers:

- **Ref-counted** via `entity.set_state_flag(mask)` /
  `entity.unset_state_flag(mask)` — for flags that have or could have
  multiple uncoordinated sources (death + stun + cast + fear all want
  to apply `BSF_MovementLock`).
- **Raw bitmask** via `entity.state_field |= mask` /
  `entity.state_field &= !mask` — for flags driven by a single source,
  idempotent player input, or externally deduplicated state.

Mixing the two on the same flag is a bug — see the BSF_IN_COMBAT case
below.

## When to use each pattern

### Ref-counted (the helpers)

Use `set_state_flag` / `unset_state_flag` when **all** of these hold:

- The flag can be set by two or more independent sources that don't
  know about each other.
- A single source clearing must NOT drop the others' refs.
- The wire-side bit semantics are "any one source held → bit set";
  not "exactly one source → bit set".

Today this applies to:

| Flag | Sources |
|------|---------|
| `BSF_DEAD` | Damage-kill / respawn pair (single source today, but the helper provides the right semantics for any future "revive resurrects mid-stun" interactions) |
| `BSF_MOVEMENT_LOCK` | Death applies it. Future stun, cast, fear, knockback effects will too |

Python parity: `SGWBeing.py:697-734` (generic `combatantStates` map),
`SGWBeing.py:770-787` (`addMovementLock` / `removeMovementLock`).

### Raw bitmask (`|=` / `&= !`)

Use raw ops when **either** holds:

- The flag is driven by a single, idempotent player input. Clicking
  crouch twice should set the bit, not bump a counter the second
  click would have to drain.
- The flag is externally deduplicated by a separate mechanism that
  already coordinates the writers.

Today this applies to:

| Flag | Reason |
|------|--------|
| `BSF_CROUCHING` | Idempotent player input via `requestCrouched` |
| `BSF_HOLSTER` | Mostly idempotent player input; weapon-fire also unholsters |
| `BSF_IN_COMBAT` | Externally managed via `threatened_mobs` set in `combat::threat` — that set IS the dedup mechanism |
| `BSF_WALKING` | Idempotent input |

## The mixing bug

If you migrate **some** writers of a flag to the helpers but leave
**others** writing raw bitmask ops, the helpers' counter will be out
of sync with the bit. The first `unset_state_flag` then sees
`count == 0`, takes the silent no-op branch, and **does not clear
the bit** — it stays stuck.

This nearly shipped in PR #128's first revision. `BSF_IN_COMBAT` is
written as raw `|=` in `cell::abilities::use_ability` (weapon-fire) and
`cell::combat::threat` (threat-table managed). PR #128 originally also
added `target.unset_state_flag(BSF_IN_COMBAT)` in damage_apply on death.
Result: NPCs would have rendered as still-in-combat after death because
the helper's counter was 0 (the raw `|=` writers never bumped it), the
unset branched to no-op, the bit stayed set.

The fix is to pick one pattern per flag and apply it to **every**
writer of that flag. Reverted that one death-site to raw `&= !mask`,
matching the rest of the BSF_IN_COMBAT writers.

## Adding a new BSF_* flag

1. **Inventory the writers.** List every code path that will set or
   clear the new flag. If it's just one writer (or one set + one matching
   clear, single-source), raw bitmask is fine. If two or more independent
   set-sources exist or are likely, use the helpers.
2. **Pick ONE pattern.** Document it in the const definition's docstring
   in `cell/combat/state.rs` so the next reader knows.
3. **Apply consistently.** Every writer goes through the chosen pattern.
   No mixing.
4. **Hard resets use `clear_all_state_flags()`.** Respawn, world-entry,
   and any other "reset to clean" path drops both the bit pattern AND
   the counter map. A raw `state_field = 0` would clear the bits but
   leave stale counters that the next `unset_state_flag` would see as
   still-positive.

## Reference: the helpers

In `crates/entity/src/cell_entity/mod.rs`:

```rust
// Ref-counted set: bumps the per-flag counter, sets the bit on 0->1.
// Returns true on transition (caller should send onStateFieldUpdate).
pub fn set_state_flag(&mut self, mask: u32) -> bool;

// Ref-counted unset: decrements the counter, clears the bit on 1->0.
// Best-effort clears (no prior set, or counter already 0) silently
// return false — they don't leak map entries and they don't warn,
// so hot defensive paths can call them safely.
pub fn unset_state_flag(&mut self, mask: u32) -> bool;

// Hard reset: drops bit pattern + counter map. Use on respawn,
// world entry, or any "back to clean state" path.
pub fn clear_all_state_flags(&mut self);

// Read: cheap bit check, safe regardless of which pattern is in use.
pub fn has_state_flag(&self, mask: u32) -> bool;
```

The mask must be a single bit. The helpers `debug_assert!` the
single-bit invariant — multi-bit masks would conflate counts across
independent flags.

## Tests

Regression tests live in `crates/services/src/cell/combat/state.rs::tests`:

- `refcount_keeps_flag_set_after_partial_unset` — the named multi-source
  semantic
- `unset_at_zero_is_noop` — best-effort clears don't underflow
- `unset_on_unowned_flag_does_not_grow_counter_map` — pin the
  no-allocation invariant for hot paths
- `drained_counter_drops_map_entry` — balanced set/unset leaves the map
  empty
- `clear_all_resets_counters_too` — hard-reset semantics
- `independent_flags_dont_share_counters` — single-bit isolation

When adding a new flag, copy the matching test pattern.
