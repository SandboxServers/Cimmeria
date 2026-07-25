---
title: "GM gating for cell methods — access_level plumbing"
type: explanation
audience: engineers
last_updated: 2026-07-25
---

# GM gating for cell methods — `access_level` plumbing

This ADR documents the server-authoritative gate that protects GM / debug
cell methods, added for [CAT-N-03](../security-audit/2026-05-31-server-authority/findings/CAT-N-gm-commands.md)
(issue #475). It is the foundation every other CAT-N (#473) GM-command
fix builds on.

## The problem it solves

`access_level` (0=Player … 4=Developer) is sourced from the
`account.accesslevel` DB column at login and lived only on the base
layer's `ConnectedClientState`. The cell-method dispatcher
(`crates/services/src/cell/dispatch/router.rs`) had no access to it, so
**every future `gm*` handler added to the cell layer was
unauthenticated-by-default** — a handler that did `if !is_gm { return }`
had nothing to check against. The moment any GM handler shipped without
also plumbing `access_level`, a modified client could send the wire shape
and the handler would run.

## The fix

### 1. `access_level` rides into the cell on the entity

`CellEntity::access_level: u32` is set once at `InitPlayerState`, sourced
from `ConnectedClientState.access_level`. It is **never** derived from a
client-supplied byte — the wire carries only a method index and args, not
the caller's privilege. Storing it on the entity (alongside `player_id`,
`archetype_id`, `system_options`) keeps the value reachable anywhere a
handler has the entity, with no per-call message widening.

Plumbing path:

```text
account.accesslevel (DB)
  → auth handler → ConnectedClientState.access_level (base)
  → world_entry_appearance/ builds InitPlayerState { access_level, … }
  → handle_init_player_state → CellEntity::access_level
```

### 2. A single dispatch-layer gate

`crates/services/src/cell/dispatch/gm_gate.rs` is the choke point.
`dispatch_cell_method` calls `enforce_gm_gate` **before** routing to any
interface handler:

- `requires_gm(method_index)` is the allow-list of restricted indices. It
  covers two classes: a few GM/debug methods that live INSIDE the inherited
  SGWPlayer range (named explicitly: 2, 3, 6, 92) and the **entire SGWGmPlayer
  tail** (`index >= SGWGMPLAYER_CELL_METHOD_BASE`, i.e. 109+ — every method
  there is a gm*/debug command by construction). Everything else passes
  untouched.
- For a restricted index, the gate reads `CellEntity::access_level` and
  checks `>= AccessLevel::GameMaster`.
- On rejection it emits a `warn!` audit log (with `entity_id`,
  `method_index`, `access_level` for ops pivoting) **and** sends an
  `onErrorCode` (method 121) wire response to the caller, then returns
  `false` so the router never reaches the handler. A missing entity fails
  closed.

## Adding the next GM method

- **A new SGWGmPlayer method (flattened index >= 109):** nothing to do for
  gating. The `index >= SGWGMPLAYER_CELL_METHOD_BASE` range rule already
  covers the entire tail, so any new gm*/debug method is GM-gated the moment
  it exists. Just implement the handler in `cell_methods/gm/` (or leave it
  to fall through the auth-gated router warn arm until you do).
- **A GM/debug method inside the inherited 0-108 range:** add its flattened
  index to the `matches!` in `requires_gm`. These share an interface with
  ordinary player methods, so they have to be named explicitly.

Either way, enforcement, audit logging, and the wire-visible error response
are shared. **Do not** put the `access_level` check inside the handler; the
gate runs first and centralizes the policy so a new handler can't forget it.

### Gated today

GM/debug methods that live inside the inherited SGWPlayer range (0-108) and
must be named explicitly:

| Index | Method | Finding |
|---|---|---|
| 2 | `toggleCombatDebug` | CAT-N intro |
| 3 | `toggleCombatVerboseDebug` | CAT-N intro |
| 6 | `toggleHealDebug` | CAT-N intro |
| 92 | `onWorldInstanceReset` | CAT-N-01 (High) |

**Plus the entire SGWGmPlayer tail (index >= 109)** — added under CAT-N-04
when GMs began entering the world as SGWGmPlayer (`class_id 0x03`). SGWGmPlayer's
117 own `<Exposed/>` gm*/debug CellMethods append at the end of the flattened
table at 109-225, so the whole native GM surface (SetGodMode, SetHealth, GiveItem, Kill,
Spawn, Goto*, …) is GM-only by construction and gated by the single
`index >= SGWGMPLAYER_CELL_METHOD_BASE` rule. This holds even for the gm*
indices that don't yet have a handler — those are still rejected for non-GMs at
the gate, then (for a GM) hit the router's "unhandled cell method" warn arm. A
verified subset (gmGiveItem 133, gmGotoXYZ 163, gmKillTarget 190) is implemented;
the full per-index table lives in
[cell-method-dispatch-table.md](../protocol/cell-method-dispatch-table.md#sgwgmplayer-extension-indices-109--473--cat-n-04).

## Why not per-call plumbing?

The audit's literal suggestion was to widen `BaseToCellMsg::CellMethodCall`
and `dispatch_cell_method` with an `access_level` parameter. We store it
on the entity instead because access level is **session-stable** (it can't
change mid-connection), so a per-call parameter would thread the same
constant through every dispatch signature for no added correctness. The
entity-stored value is equally authoritative — it comes from the same
`ConnectedClientState.access_level`, set once at world entry.

## Moderation surface still missing

Folded in from the superseded server-systems survey (see
[server-systems.md](server-systems.md)), whose "admin and GM tools" section is
the one place its design thinking outlived its current-state claims. The gate
above answers *who may run a GM command*. These are the things a GM still
cannot do at all.

**No GM action audit log.** Every accepted `.`-console command is logged at
`info` for the audit trail
([`crates/services/src/cell/console/dispatch.rs`](../../crates/services/src/cell/console/dispatch.rs)),
and login events land in the `login_audit` table, but there is no durable,
queryable record of GM *actions* — who granted what item to whom, and when.
The design is small: wrap the dispatch seam and write command name, actor,
target, and arguments to a table for any caller above access level 0. Log lines
are not an audit trail; they rotate.

**No ban or mute.** There is no account-level ban and no chat mute — no schema
columns, no commands, no enforcement point. Adding them means an `is_banned`
check on the login path and an `is_muted` check in chat routing, plus the two
commands. This is the gap that matters first if the server ever opens beyond a
trusted group.

**No server-wide announcement.** Nothing broadcasts a system message to all
connected clients — useful for maintenance warnings long before it is useful for
moderation.

**No rollback.** There is no way to reverse a GM action or an exploited
transaction. This one is genuinely hard and depends on the currency-flow
instrumentation in
[server-infrastructure-proposals.md §5](server-infrastructure-proposals.md#5-economy-instrumentation-before-economy-balance)
existing first — you cannot reverse what you never recorded.

Current status for each of these is tracked in
[gap-analysis.md](../gap-analysis.md) §"Server Infrastructure (Cross-Cutting)".

## Related

- [CAT-N findings](../security-audit/2026-05-31-server-authority/findings/CAT-N-gm-commands.md) — the full GM-command surface.
- `cimmeria_commands::permissions::AccessLevel` — the typed level + `can_execute` ordering.
- [state-field-bits.md](state-field-bits.md) — neighbouring server-authority concern (which `state_field` bits persist).
