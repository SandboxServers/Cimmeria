# Pet System Wire Formats

> **Date**: 2026-03-01
> **Phase**: 4 — Secondary Systems RE
> **Confidence**: HIGH (derived from `.def` files + `alias.xml` + universal RPC dispatcher architecture). Corrected 2026-09-27 (stance types, PT-E1) and 2026-09-28 (the player-method argument lists, #804): the first edition's INT32 stance fields and short argument lists were wrong.
> **Sources**: `SGWPet.def`, `SGWPlayer.def`, `alias.xml`

---

Defined on `SGWPet.def` (entity type, parent: `SGWMob`).

### Server → Client

#### `onPetAbilityList` — Pet's Available Abilities

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `abilitiesList` | `ARRAY<INT32>` | 4B count + N×4B |

#### `onPetStanceList` — Available Stances

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `stanceList` | `ARRAY<INT8>` | 4B count + N×1B |

> **Correction (2026-09-27, PT-E1)**: previously documented as `ARRAY<INT32>`. `SGWPet.def:88`
> declares `ARRAY<INT8>` — confirmed against the binary in
> [`pet-client-contract.md`](pet-client-contract.md) §1.

#### `onPetStanceUpdate` — Current Stance Changed

| Field | Type | Size |
|-------|------|------|
| `stance` | `INT8` | 1B |

**Total wire size**: 1B header + 1B = **2 bytes**

> **Correction (2026-09-27, PT-E1)**: previously documented as `INT32` (5 bytes total).
> `SGWPet.def:92` declares `INT8` — confirmed against the binary in
> [`pet-client-contract.md`](pet-client-contract.md) §1.

### Client → Server (via SGWPlayer.def — separate from pet entity)

Pet commands are sent as player cell methods, not pet entity methods:

| Method | Args | Notes |
|--------|------|-------|
| `petInvokeAbility` | `INT32 aEntityId`, `INT32 aAbilityId`, `INT32 aTargetId` | Use pet ability |
| `petAbilityToggle` | `INT32 aEntityId`, `INT32 aAbilityId`, `INT8 aToggle` | Toggle auto-cast |
| `petChangeStance` | `INT32 aEntityId`, `INT8 aStance` | Change pet stance |

> **Correction (2026-09-28, #804)**: previously listed without the leading `aEntityId` (the pet's
> entity id) and with `UINT8 toggle` / `INT32 stanceId`. `SGWPlayer.def:827-845` declares the
> lists above; the Rust parser (`cell_methods/player/social.rs`) and
> [cell-method-dispatch-table.md](../../protocol/cell-method-dispatch-table.md) already follow it.

---

## Implementation Notes

- **Pet commands**: Sent as player methods, not pet entity methods — the server routes to the pet internally.
