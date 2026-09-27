# Pet System Wire Formats

> **Date**: 2026-03-01
> **Phase**: 4 — Secondary Systems RE
> **Confidence**: HIGH (derived from `.def` files + `alias.xml` + universal RPC dispatcher architecture)
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
| `petInvokeAbility` | `INT32 abilityId`, `INT32 targetId` | Use pet ability |
| `petAbilityToggle` | `INT32 abilityId`, `UINT8 toggle` | Toggle auto-cast |
| `petChangeStance` | `INT32 stanceId` | Change pet stance |

---

## Implementation Notes

- **Pet commands**: Sent as player methods, not pet entity methods — the server routes to the pet internally.
