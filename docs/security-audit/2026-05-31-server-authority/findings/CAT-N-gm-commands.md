# CAT-N — GM / Debug / Cheat commands — Findings

> **Status re-verification (2026-07-25)** — deltas against `origin/main`:
>
> - **CAT-N-03 (no `access_level` in cell dispatch): RESOLVED (#475).**
>   `crates/services/src/cell/dispatch/gm_gate.rs` is the single choke
>   point. `enforce_gm_gate` reads `CellEntity::access_level` (set once at
>   `InitPlayerState` from `account.accesslevel`, never from a
>   client-supplied byte), rejects callers below `AccessLevel::GameMaster`,
>   emits a `warn!` audit line, and sends an `onErrorCode` response. A
>   missing entity **fails closed**. This unblocks the rest of CAT-N.
> - **CAT-N-01 (`WORLD_INSTANCE_RESET`, CM 92): RESOLVED (#475).** Index 92
>   is named explicitly in `requires_gm`
>   (`crates/services/src/cell/dispatch/gm_gate.rs:81`). The same fix
>   covers **CAT-O-04**.
> - **CAT-N-02 (`RESET_MY_ABILITIES`, CM 72): STILL OPEN.** The gate covers
>   `matches!(index, 2 | 3 | 6 | 92)` plus the whole SGWGmPlayer tail
>   (index ≥ 109). CM 72 is in neither set
>   (`crates/services/src/cell/cell_methods/player/dispatch.rs:85` pins
>   `RESET_MY_ABILITIES == 72`), so it is **not** gated. The handler is
>   still a stub, so this is latent rather than live — but the finding's
>   original shape ("free respec when implemented") is unchanged, and the
>   implementer will not be protected by #475. Adding `72` to `requires_gm`
>   is a one-line fix.
> - **The rest of CAT-N: STILL OPEN but now structurally protected.** The
>   `gm*` surface at index ≥ 109 is GM-gated by construction, including
>   indices with no handler yet, so the "future implementer ships it
>   unauthenticated" systemic risk is closed for that range. Per-command
>   *bounds* checks (the second half of most CAT-N findings) are not.

**Overall trust posture.** CAT-N covers the largest single attack surface in the
SGW protocol — ~125 distinct `Event_NetOut_*` messages produced by the
`Event_SlashCmd_*` → `Event_NetOut_*` debug-command pipeline in
`SGWTextCommandMgr`, mapped onto the `gm*` exposed CellMethods + BaseMethods
of `entities/defs/SGWGmPlayer.def` (and a handful that also live on the
regular `SGWPlayer.def`). The current Rust server is in a paradoxical state:

- **It is NOT *currently* exploitable through CAT-N** because (a) zero `gm*`
  cell methods are implemented (every one falls through to the
  `warn!("Unhandled cell method call")` arm at
  `crates/services/src/cell/dispatch/router.rs:101`) and (b) the server
  hard-codes the entity class to SGWPlayer (`class_id` 0x02) regardless of
  `access_level`, with the explanatory comment "Until we build a separate
  SGWGmPlayer index table, always use SGWPlayer (0x02) regardless of
  access_level" at `crates/services/src/base/world_entry/play_character.rs:89-93`.
- **It is structurally unsecurable as written** because the cell-dispatch
  layer has no access to `access_level` — the auth-level bit lives only on
  `ConnectedClientState.access_level` (`crates/services/src/base/mod.rs:117`),
  which is base-only state. The cell `dispatch_cell_method` signature
  (`entity_id, method_index, args, tx, space_mgr, engine`) carries
  no caller identity beyond the entity id; nothing in the cell layer knows
  the caller is or isn't a GM. The moment ANY `gm*` handler is added without
  also plumbing `access_level` through the cell-base boundary, that handler
  is unauthenticated-by-default.
- **Three GM-shaped methods are reachable today and are stubs**:
  `WORLD_INSTANCE_RESET` (CM 92), `RESET_MY_ABILITIES` (CM 72), and the
  three combat/heal debug toggles (`TOGGLE_COMBAT_DEBUG` CM 2,
  `TOGGLE_COMBAT_VERBOSE_DEBUG` CM 3, `TOGGLE_HEAL_DEBUG` CM 6). All have
  `Exposed/` in `SGWPlayer.def` (i.e., they're in the regular-player flat
  index table, not just SGWGmPlayer's). All stub-log and return; no GM
  check. If implemented to do anything real without adding gating, they
  become privilege-escalation vectors.

The findings below are filed in two tiers:

1. **Implementation gaps reachable today** (the three stub methods plus the
   missing-plumbing systemic finding). These are the only items a black-box
   attacker can poke at today.
2. **Wire-surface findings** for the ~120 `gm*` methods that aren't yet
   dispatched but where the wire shape is already accepted on the
   `dispatch_cell_method` router and would immediately escalate the moment
   a handler is added. These are filed because (a) the SGW.exe binary
   actively emits these messages from the local slash-command UI and
   (b) repeated additions of gm* handlers without auth plumbing are the
   most plausible regression path — the user said this is the largest
   single risk surface for a reason.

Severity calibration: Critical is reserved for "anyone can become GM"
(SetHideGM into authoritative access_level), High for "anyone can mutate
authoritative inventory/state of self or others" (SetGodMode, SetHealth,
GiveItem, Kill, Spawn, WorldInstanceReset), Medium for info-disclosure or
self-only state hacks, Low for log-noise / DoS-shape findings.

---

### CAT-N-01 — `WORLD_INSTANCE_RESET` (CM 92) exposed on regular `SGWPlayer.def`, stub handler has no GM check

**Severity**: High
**Class**: Server-authority bypass — privileged action exposed at non-privileged
flat index
**Wire surface**: `Event_NetOut_WorldInstanceReset` → cell method index 92
**Demonstrable / Likely-theoretical**: Likely-theoretical (handler is a stub
today; the finding is the un-gated wire surface + the missing gating
infrastructure that a future implementer will likely miss)

**Trust violation**
`SGWPlayer.def:868-870` declares `<onWorldInstanceReset><Exposed/></onWorldInstanceReset>`
with no args — i.e., the method is in the flat exposed-CellMethod table that
*every* SGWPlayer instance, including access_level=0 accounts, can call
directly. The 2009 design intent (per the `gm*` family of equivalent
commands) was that this would tear down and recreate the current world
instance, kicking every other player in the same space. Today the Rust
handler at `crates/services/src/cell/cell_methods/player/world/mod.rs:230-233`
is `tracing::info!(entity_id, "UNIMPLEMENTED: onWorldInstanceReset")` and
returns, but the method-index slot is fully reachable on the wire (CM 92).
The moment someone implements the real reset path without first checking
`access_level`, every player in the shard can grief any space by sending a
single packet — the index is `0xBD` + sub-byte `92 - 61 = 31` per the
extended cell-method encoding in
`docs/protocol/client-method-dispatch-table.md`.

**Evidence**
- Ghidra: `019b4340` `Event_NetOut_WorldInstanceReset` — RTTI string for
  the client-side event class; `01df5b90` is the typed RTTI descriptor.
  Confirmed as an outbound (client→server) typed event with no args.
- entities/defs/SGWPlayer.def:868-870 — `<onWorldInstanceReset><Exposed/></onWorldInstanceReset>`
  with no args, i.e. this CellMethod is exposed on the regular SGWPlayer
  flat-index table, not gated by SGWGmPlayer parentage.
- Cross-ref to Rust handler (for the fix author, NOT as truth):
  `crates/services/src/cell/cell_methods/player/world/mod.rs:230-233` —
  stub `UNIMPLEMENTED` arm; index pinned to 92 at
  `crates/services/src/cell/cell_methods/player/constants.rs:30`.

**Attack scenario**
1. Modified client (or replay tool) sends a single cell-method call to
   index 92 with empty args bytes.
2. Server dispatch routes to the world-method handler which currently
   logs and returns — but the dispatch *succeeds* (no rejection).
3. When this is wired up: every player in the current `space_id` (typically
   a 30+ population zone in the live design) is torn down and respawned,
   wiping in-progress combat / loot / mission state across all of them.

**Suggested remediation (one line)**
Move the dispatch entry behind an access-level check; the cell-dispatch
needs a caller `access_level` plumbed in (see CAT-N-15) OR the index
needs to be removed from the regular SGWPlayer table and only honoured
on the SGWGmPlayer table once the latter is implemented.

**Would benefit from x64dbg trace?**
No — the wire surface and the def file are sufficient.

---

### CAT-N-02 — `RESET_MY_ABILITIES` (CM 72) exposed on regular `SGWPlayer.def`, stub handler will be a free respec when implemented

**Severity**: High
**Class**: Server-authority bypass — economy-impacting privileged action
exposed at non-privileged flat index
**Wire surface**: `Event_NetOut_ResetAbilities` → cell method index 72
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
`SGWPlayer.def:602-604` declares `<resetMyAbilities><Exposed/></resetMyAbilities>`
with no args. The implied design is that any player can free-reset their
ability tree, which in the live MMO would mean refunding all training
points spent on `trainAbility` — a feature that on most MMOs is gated by
either a per-account cooldown, a cash cost, or an NPC interaction (the
`gmRespec` cell method on SGWGmPlayer is the GM variant; `respecCrafting`
on the regular SGWPlayer is the *crafting-only* variant). The Rust handler
is a stub `UNIMPLEMENTED: resetMyAbilities` at
`crates/services/src/cell/cell_methods/player/combat/mod.rs:120`. When
this is implemented, if it does what its name says without cost / cooldown
/ NPC gating, a player can spam it to unlock infinite respecs every
millisecond.

**Evidence**
- entities/defs/SGWPlayer.def:602-604 — `<resetMyAbilities><Exposed/></resetMyAbilities>`,
  zero args.
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/player/combat/mod.rs:119-122`
  (`UNIMPLEMENTED: resetMyAbilities`); constant at
  `crates/services/src/cell/cell_methods/player/constants.rs:10`.
- The Ghidra-side `Event_NetOut_ResetAbilities` client event has no
  arguments (the typed event class carries no payload beyond its RTTI),
  matching the .def's zero-arg signature.

**Attack scenario**
1. Player levels and spends all `trainingPoints` on ability A.
2. Player sends `resetMyAbilities()` — once implemented, all points refund.
3. Player spends all on ability B for a different rotation.
4. Player repeats infinitely with zero cooldown / cost.

**Suggested remediation (one line)**
Implementing handler must (a) charge cash from `sgw_player.cash` and (b)
enforce a per-character cooldown row before refunding — wire-shape match
to `gmRespec` (which is GM-only and uncosted) is not the right model.

**Would benefit from x64dbg trace?**
No — wire surface is fully specified by the .def.

---

### CAT-N-03 — Cell-method dispatch has no access to `access_level`; every future `gm*` handler is unauthenticated-by-default

**Status**: ✅ RESOLVED (#475) — `CellEntity::access_level` is plumbed from
`ConnectedClientState.access_level` via `InitPlayerState`, and a
dispatch-layer gate (`crates/services/src/cell/dispatch/gm_gate.rs`)
rejects GM/debug indices from non-privileged callers before routing, with
a `warn!` audit log + `onErrorCode` wire response. See
[gm-cell-method-gating.md](../../../architecture/gm-cell-method-gating.md).
The reachable GM-shaped methods (CM 2/3/6 debug toggles, CM 92
`onWorldInstanceReset`) are gated; the rest of the CAT-N surface lands
behind the same gate as each handler is implemented (add the index to
`requires_gm`).

**Severity**: Critical
**Class**: GM auth bypass — missing infrastructure / systemic
**Wire surface**: Every `gm*` cell method (~85 entries on SGWGmPlayer.def +
inheritance), reachable via `dispatch_cell_method` at
`crates/services/src/cell/dispatch/router.rs:33`
**Demonstrable / Likely-theoretical**: Likely-theoretical (the gap is
real today, but no `gm*` handler is wired up yet)

**Trust violation**
The cell-method dispatch entry point takes
`(entity_id, method_index, args, tx, space_mgr, engine)` — none of these
carry the caller's `access_level`. That field lives on
`ConnectedClientState.access_level` (`crates/services/src/base/mod.rs:117`),
which is set from the `account.accesslevel` DB column at login
(`crates/services/src/auth/handlers.rs:486-488`) and currently consumed
only by the chat dispatch (`SEND_PLAYER_COMMUNICATION`'s `SPEAKER_GM` bit
computation at `crates/services/src/base/dispatch.rs:131-133`). When a
cell-method dispatch arm needs to check "is the caller a GM?", it has no
way to do so without breaking the cell/base layer boundary. The natural
implementation path for any future contributor adding `gmGiveItem`,
`gmSpawnByCmd`, etc. is to drop the handler into `cell_methods/...` and
ship — and the resulting handler has zero GM check because there's
nothing to check against.

**Evidence**
- `crates/services/src/cell/dispatch/router.rs:33-93` — the dispatch
  signature has no caller-identity parameter beyond `entity_id`. None of
  the per-interface dispatchers (`cell_methods::being::dispatch`,
  `cell_methods::ability_manager::dispatch`, ...) accept an access_level
  either.
- `crates/services/src/base/mod.rs:108-117` — `access_level: u32` is on
  `ConnectedClientState`. Grep `crates/services/src/cell` for
  `access_level`: zero hits (the field never crosses into the cell layer).
- Account DB read: `crates/services/src/auth/handlers.rs:439-488` —
  the field is sourced authoritatively from the `account.accesslevel`
  column; this part is correct.
- Comment at `crates/services/src/base/world_entry/play_character.rs:89-94`:
  "C++ Account.py:293-296 uses SGWGmPlayer (0x03) for access_level > 0,
  but ... always use SGWPlayer (0x02) regardless of access_level. TODO:
  Build SGWGmPlayer method index table to enable GM entity type."

**Attack scenario**
1. Future contributor implements `gmGiveItem` (CM index unknown until
   SGWGmPlayer table is built — likely as a new `match` arm in the
   player dispatch).
2. Implementation grabs the player's inventory and inserts the item.
3. There is no `if !access_level.can_execute(GM) { return; }` because
   no `access_level` is in scope.
4. Any non-GM player sending the same wire shape (modified client or
   replayed packet) triggers the same code path. Free items for everyone.

**Suggested remediation (one line)**
Plumb `access_level: u32` through `BaseToCellMsg::CellMethodCall` (the
cell receives the auth level alongside the entity id) and through
`dispatch_cell_method` as an additional parameter; gate `gm*`-named
methods on `access_level >= AccessLevel::GameMaster as u32`.

**Would benefit from x64dbg trace?**
No — the gap is purely a server-side architectural omission.

---

### CAT-N-04 — Server forces `class_id = 0x02 (SGWPlayer)` for all logins, ignoring `access_level`; legitimate GM accounts cannot use GM commands AND there is no entity-type isolation between GM and non-GM index spaces

**Severity**: Medium
**Class**: GM auth design omission — wrong entity class flattens index spaces
**Wire surface**: All `gm*` cell methods (currently inaccessible due to
class hardcode, but a future fix that just flips the class will expose
them all without gating)
**Demonstrable / Likely-theoretical**: Demonstrable (the hardcode is
documented in code) / Likely-theoretical (the exploit needs a future flip)

**Trust violation**
The C++ reference (per the inline comment) selects entity class
`SGWGmPlayer (0x03)` for `access_level > 0` so the GM client gets the
extended 80+ CellMethod index table that includes the `gm*` family. The
Rust server hardcodes `SGWPlayer (0x02)` regardless of `access_level`
because the SGWGmPlayer flat-index table isn't built. This has two
consequences:

1. Today: legitimate GMs cannot issue GM commands (the indices for
   `gmGiveItem` etc. don't exist in the SGWPlayer table, so the client's
   serializer would compute the wrong index — the index 0–108 space is
   fully occupied by non-GM methods).
2. Tomorrow, when someone fixes (1) by switching the class on
   `access_level > 0`: the *same* `dispatch_cell_method` router will
   receive the GM indices, and per CAT-N-03 has no access_level
   parameter. The natural and incorrect fix is to add the `gm*` arms
   inside the same dispatch fns the regular `gm*`-less methods land in
   — at which point a non-GM client can send the GM index too (it's
   just a method_index number on the wire; the server has no way to
   tell whether the *sender* is on the GM class).

**Evidence**
- `crates/services/src/base/world_entry/play_character.rs:89-94` — the
  hardcode + the TODO comment.
- `crates/services/src/cell/cell_methods/player/constants.rs` — only 42
  SGWPlayer-own indices (67-108). SGWGmPlayer adds 80+ more; none are
  present.
- entities/defs/SGWGmPlayer.def — declares the parent as `SGWPlayer` so
  the GM class would extend, not replace, the index table. Future
  implementers must NOT collapse the two index spaces into one
  dispatch fn unless `access_level` is also threaded through.

**Attack scenario**
1. Future contributor fixes the entity-class hardcode and adds GM method
   indices (109+) to the `dispatch_cell_method` chain.
2. They implement `gm*` handlers inside the same dispatch arms,
   because that's where dispatch lives.
3. A non-GM client modifies its serializer to use the GM indices and
   sends `gmGiveItem("ItemTemplateXYZ", 99999)`.
4. Server has no `access_level` in scope; the arm runs; the inventory
   row is inserted. Item-dupe / cash-dupe / level-dupe at will.

**Suggested remediation (one line)**
Couple the entity-class fix with the CAT-N-03 access_level plumbing in
the same PR; do NOT ship the class switch without the auth threading.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-05 — `Event_NetOut_SetHideGM` wire surface accepted on regular player; nothing prevents a client from claiming GM-visible state, which is the GM-visibility property `bHideGM` on SGWGmPlayer

**Severity**: Critical (worst case — if `bHideGM` is ever read as a
GM-effective bit)
**Class**: GM auth bypass — privileged property mutation exposed via
client-trusted setter
**Wire surface**: `Event_NetOut_SetHideGM(UINT8 bTurnOn)` → `gmSetHideGM`
cell method (and base method) on SGWGmPlayer
**Demonstrable / Likely-theoretical**: Likely-theoretical (today
unimplemented; the property itself is also unimplemented in Rust)

**Trust violation**
`Event_NetOut_SetHideGM` carries `UINT8 bTurnOn` — a client-supplied
boolean for whether the GM should be invisible to non-GM players. The
SGWGmPlayer.def declares both a CellMethod and a BaseMethod with this
name (lines 367-370 and 722-724), AND the `bHideGM` property on
SGWGmPlayer (lines 19-23). The danger is shape-specific: if any future
code path reads `bHideGM` as a proxy for "is this player a GM?" (e.g.,
for /who filtering, AoI broadcast suppression, or moderator-channel
membership), and the property mutation accepts the client-supplied byte
without `access_level` verification, then a non-GM client can flip
their own `bHideGM = 1` and gain whatever effects flow from it.

The wire surface is already in the binary (the Slash command, the
NetOut emit, the registration). Today no Rust handler exists for the
property OR the method — but the *systemic gap* (CAT-N-03) means
when a handler is added in the obvious place (a player-methods
arm), it'll trust the byte.

**Evidence**
- Ghidra: `019b2ec8` / `019be278` `Event_NetOut_SetHideGM` —
  client-side RTTI strings. `01df329c` is the typed RTTI descriptor.
- entities/defs/SGWGmPlayer.def:19-23 — `bHideGM` is a `CELL_PRIVATE`
  property with default 0. Lines 367-370 expose `gmSetHideGM` as a
  CellMethod (`<Exposed/>` + `UINT8 bTurnOn`). Lines 722-724 also
  expose it as a BaseMethod with the same signature.
- No Rust handler. Wire surface is fully unhandled today.

**Attack scenario**
1. Non-GM client constructs the `gmSetHideGM(1)` cell-method bytes.
2. Server today: dispatch falls through to the `warn!` arm (CAT-N-03
   prevents the index from mapping into a real arm). Today this is
   inert.
3. Future state: handler is added without GM gating; `bHideGM=1` is
   written to the entity / DB.
4. Any subsequent code that uses `bHideGM` as a privilege bit
   (extremely plausible — Python `python/cell/SGWGmPlayer.py`
   references it as the visibility gate) treats the regular player
   as a GM for visibility purposes.

**Suggested remediation (one line)**
Treat `bHideGM` strictly as a presentation hint downstream of the
access_level bit and never as a privilege gate; the setter, when
implemented, must verify `access_level >= GameMaster` before
writing.

**Would benefit from x64dbg trace?**
Yes — if the live Python reference reads `bHideGM` from a non-Python
caller (e.g., a base-side authority check), x64dbg breakpoints on
the C++ `bHideGM` accessors would confirm whether the byte ever
gates a real authority check vs. only the visual hide.

---

### CAT-N-06 — Wire surface for `Event_NetOut_SetGodMode` accepts client-supplied byte; the underlying `bGodMode` is `CELL_PUBLIC` on SGWAbilityManager (every player+NPC)

**Severity**: High
**Class**: Damage-immunity flag exposed via client setter
**Wire surface**: `Event_NetOut_SetGodMode(UINT8 bTurnOn)` → `gmSetGodMode`
cell method
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The `bGodMode` property is declared on the `SGWAbilityManager` interface
(`entities/defs/interfaces/SGWAbilityManager.def:67-72`) with `CELL_PUBLIC`
visibility. The SGWGmPlayer.def line 236-239 exposes
`gmSetGodMode(UINT8 bTurnOn)` as the setter. Because `bGodMode` is in the
ability-manager interface that *every* combat-capable entity implements
(SGWPlayer, SGWMob, SGWPet), the same byte determines damage immunity for
mobs and pets, not just players. A future handler that trusts the
client-supplied byte either (a) makes the calling player immune (no-aggro
+ no-damage) or (b) — if `gmSetGodMode` is dispatched on a target other
than the caller — lets the caller flip mobs / pets to god mode and grind
them safely. Wire shape carries no target id (the SGWGmPlayer.def setter
is single-arg: `UINT8 bTurnOn`), implying the caller is the target —
which is the worst case (no target_id check needed).

**Evidence**
- Ghidra: `019b3848` / `019bef48` `Event_NetOut_SetGodMode`. Strings
  `019c3498` (`gmSetGodMode`) confirm the cell method name.
- entities/defs/interfaces/SGWAbilityManager.def:67-72 — `bGodMode`
  property, `CELL_PUBLIC` (visible to AoI witnesses; this is wire-public).
- entities/defs/SGWGmPlayer.def:236-239 — `<gmSetGodMode><Exposed/><Arg>UINT8 bTurnOn</Arg></gmSetGodMode>`.
- No Rust handler; no `bGodMode` property on the Rust `CellEntity`.

**Attack scenario**
1. Player sends `gmSetGodMode(1)` cell-method call (post-CAT-N-04 fix).
2. Server (lacking CAT-N-03 access_level gating) writes
   `entity.b_god_mode = true`.
3. Combat code reads the flag and skips damage application.
4. Caller is now invulnerable for the session.

**Suggested remediation (one line)**
The damage-resolution path must read `access_level` (server-side
session record) as the authoritative god-mode gate, not the
client-controlled `bGodMode` property.

**Would benefit from x64dbg trace?**
No — wire is sufficient.

---

### CAT-N-07 — `Event_NetOut_SetHealth` / `SetHealthMax` / `SetFocus` / `SetFocusMax` wire surfaces carry a client-supplied `INT64 TargetId`; future handler can mutate any entity's vital stats

**Severity**: High
**Class**: Authority bypass — direct stat-set with client-chosen target
**Wire surface**: `Event_NetOut_SetHealth(INT32 Amount, INT64 TargetId)` and
3 siblings; `gmSetHealth` / `gmSetHealthMax` / `gmSetFocus` / `gmSetFocusMax`
cell methods
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:259-281 declares all four methods with a payload that
includes `INT64 TargetId` — i.e., the GM can set health/focus on any
entity, including PvP opponents, raid bosses, or fellow players. The
wire signature is `<INT32 Amount, INT64 TargetId>`. A future handler
that (a) misses CAT-N-03 access_level gating AND (b) trusts the
client-supplied `TargetId` lets any player one-shot any other entity
(`gmSetHealth(0, BossEntityId)`) or top up their own health to the
cap mid-fight. The target is not validated against the caller's
perception list, ownership, or even space — any 64-bit entity id is
accepted.

**Evidence**
- Ghidra string scan: `Event_NetOut_SetHealth`, `Event_NetOut_SetHealthMax`,
  `Event_NetOut_SetFocus`, `Event_NetOut_SetFocusMax` all present as
  outbound NetOut classes.
- entities/defs/SGWGmPlayer.def:259-281 — full setters, all carry
  `INT32 Amount` + `INT64 TargetId`.
- No Rust handler.

**Attack scenario**
1. Player observes a high-priority NPC's `entity_id` via the standard
   AoI sync (the field is wire-public).
2. Player sends `gmSetHealth(0, <npc_id>)` cell-method call.
3. Future handler — lacking access_level gate and target authority
   check — applies the 0-HP write; the NPC dies, drops loot,
   awards XP/credit to whoever currently has aggro.

**Suggested remediation (one line)**
When the handlers are implemented: enforce both access_level AND
re-validate `TargetId` against `space_mgr.get_entity(target_id)` +
caller's witness list / aggro authority — never accept a bare client
entity id.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-08 — `Event_NetOut_SetSpeed` accepts a `FLOAT Multiplier` with no range bound

**Severity**: High
**Class**: Speed-hack flag exposed via client setter
**Wire surface**: `Event_NetOut_SetSpeed(FLOAT Multiplier)` → `gmSetSpeed`
cell method
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:254-257 declares `gmSetSpeed(FLOAT Multiplier)` with no
range. Comment says "Speed Multiplier (1.0 default)" — implying 1.0 is
normal speed. A future handler that writes the multiplier into the
player's movement state without clamping makes the player teleport-fast
(values >> 1.0) or freeze (values near 0). This is the classic speed-hack
shape. The wire byte sequence is unconstrained float — clients can send
+inf, NaN, denorm, negative — all of which would land in whatever
movement math the future handler uses.

**Evidence**
- Ghidra: `019b3818` `Event_NetOut_SetSpeed` (strings list); typed
  RTTI is `Event_NetOut_SetSpeed@@`.
- entities/defs/SGWGmPlayer.def:254-257 — single-arg `FLOAT Multiplier`,
  Exposed.
- No Rust handler.

**Attack scenario**
1. Player sends `gmSetSpeed(99999.0)` — or `gmSetSpeed(f32::NAN)`.
2. Future handler writes into the player's `top_speed` or movement
   scalar without clamping.
3. Movement physics applies the multiplier — player teleports across
   the map per tick; navmesh / collision is bypassed at high
   per-tick deltas. (NaN path is worse: produces NaN positions that
   break ANYTHING that does position math, potentially crashing
   downstream code that doesn't check for NaN inputs.)

**Suggested remediation (one line)**
Clamp the multiplier to `[0.1, 5.0]` (or similar designer-decided
band) and reject non-finite floats before applying — even after
access_level gating, the un-bounded float arg is hostile to
movement physics.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-09 — `Event_NetOut_SetLevel` accepts client-supplied `INT32 aLevel` with no XP-table consistency

**Severity**: High
**Class**: Power-progression bypass via client setter
**Wire surface**: `Event_NetOut_SetLevel(INT32 aLevel)` → `gmSetLevel`
cell method
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:288-291 — single-arg `INT32 aLevel`. The Rust server
maintains a `level` column on `sgw_player` that gates ability unlocks,
gear binding, and damage / health scaling. A future handler that writes
`aLevel` directly into the level column bypasses all of:

- The XP curve (`sgw_player.exp`)
- Per-archetype ability progression (only some abilities are awarded
  per level via `trainAbility`)
- Faction/Mission level gates

Wire surface has no `TargetId` so this is self-only; but combined with
GiveAbility, the player gains a level-cap-equivalent character in
seconds.

**Evidence**
- Ghidra: `Event_NetOut_SetLevel` RTTI confirmed.
- entities/defs/SGWGmPlayer.def:288-291.
- No Rust handler.

**Attack scenario**
1. Send `gmSetLevel(60)` (or whatever the cap is).
2. Future handler writes `sgw_player.level = 60`.
3. Without also updating XP, ability list, and gear flags, the player
   ends up with a level-60 character with level-1 abilities — but
   damage/health scaling uses `level`, so they outscale all
   matched-level content.

**Suggested remediation (one line)**
The level-set path must recompute derived stats (HP/QR/cap, ability
list, XP rebase) atomically, AND require access_level gating; clamp
`aLevel` to `[1, level_cap]`.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-10 — `Event_NetOut_GiveItem` carries `WSTRING DesignId, INT32 Quantity` with no template whitelist, item-binding, or quantity bound

**Severity**: High
**Class**: Authority bypass — item creation with client-chosen template
**Wire surface**: `Event_NetOut_GiveItem(WSTRING DesignId, INT32 Quantity)`
→ `gmGiveItem` cell method
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:185-189 — `<gmGiveItem><WSTRING DesignId><INT32 Quantity>`.
This is the same shape as the `give` command stub in
`crates/game/src/commands/gm_cmds.rs:83-104`, which IS access-level
gated (`AccessLevel::GameMaster`). But that stub is for the in-process
slash-command path (chat → registry), NOT for the wire `gmGiveItem`
cell-method that the SGW client emits. The two paths diverge. A future
implementer who plumbs `gmGiveItem` through the cell-dispatch lane
without access_level gating (CAT-N-03) lets any player materialise any
item template by name, with any quantity (positive or negative; the
arg is `INT32` not `UINT32`).

The wire surface also carries no `recipient TargetId` field — implying
self-give — but the SGWGmPlayer base method variant might differ. Either
way: free items, any template, any (signed) quantity.

**Evidence**
- Ghidra: `019b373c` `Event_NetOut_GiveItem`; `00cb7880`
  `register_NetOut_GiveItem`.
- entities/defs/SGWGmPlayer.def:185-189.
- The slash-command path at `crates/game/src/commands/gm_cmds.rs:83-104`
  is gated `AccessLevel::GameMaster`; the wire-method path is not yet
  gated because it doesn't exist yet — the gating gap is the
  *architecture*, not the code.

**Attack scenario**
1. Player sends `gmGiveItem("Item_Naqahdah_Brick", 999999999)`.
2. Future handler resolves the template + inserts the row into
   `sgw_inventory` without checking access_level OR the quantity sign.
3. If quantity is negative + handler does naïve arithmetic, this also
   becomes an item-DELETION primitive (`gmGiveItem("Item_X", -50)`
   subtracts from someone else's stack if the template-matching is
   misimplemented to find an existing stack to "add to").

**Suggested remediation (one line)**
Cell-method `gmGiveItem`, when implemented, must (a) check
access_level via the CAT-N-03 plumbing, (b) clamp Quantity to a
positive band ≤ template's max stack, and (c) validate `DesignId`
exists in the item-template loader (not blindly trust the WSTRING).

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-11 — `Event_NetOut_GiveNaqahdah` (currency) and `Event_NetOut_GiveXp` accept unbounded INT32 with no overflow check

**Severity**: High
**Class**: Currency / XP inflation via client setter
**Wire surface**: `Event_NetOut_GiveNaqahdah` / `Event_NetOut_GiveXp` →
`gmGiveCash` / `gmGiveXp` cell methods
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:180-194 — `<gmGiveXp><INT32 XpAmount>` and
`<gmGiveCash><INT32 Amount>`. Both are signed 32-bit integers, both with
no clamp. Two failure modes:

1. **Inflation**: positive `XpAmount` / `Amount` of `INT32::MAX`
   instantly maxes the player's stats / currency.
2. **Negative-take**: negative values, if the handler does
   `cash += Amount` without a `>=0` check, become a take-from-account
   primitive (or, with overflow, a wrap-around).

The Ghidra binary already has the error string "Amount to be given (or
taken away) cannot be 0." at `019ad958` — i.e., the C++ Python-side
implementation explicitly allowed negative amounts (the GM CAN take
cash away), so the wire-shape contract is "any non-zero INT32". A
Rust handler that mirrors the Python intent will be a take-from-anyone
primitive once a TargetId arg is added (the wire shape is self-only
today but the BaseMethod variant may differ).

**Evidence**
- Ghidra: `019b362c` `Event_NetOut_GiveXp`, `019b370c`
  `Event_NetOut_GiveNaqahdah`; `00cb6e60` `register_NetOut_GiveXp`.
- Behavioral string in binary: `019ad890` "Amount of XP to give must
  not be 0." and `019ad958` "Amount to be given (or taken away)
  cannot be 0." — confirms the historical Python allowed negatives.
- entities/defs/SGWGmPlayer.def:180-194.

**Attack scenario**
1. Player sends `gmGiveNaqahdah(INT32::MAX)`.
2. Future Rust handler `sgw_player.cash += amount as i64` — i64
   add doesn't overflow but DB column is i32; the SQL UPDATE
   fails or wraps. Either way the player's currency now reads
   maxed or unpredictable.
3. Or `gmGiveNaqahdah(-100000)` against any TargetId — siphon cash
   from victim.

**Suggested remediation (one line)**
Handlers must check `Amount > 0`, that the post-add total doesn't
exceed the column's max, and (for the BaseMethod variant) that the
caller has access_level + that the target is reachable.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-12 — `Event_NetOut_Kill` carries `INT64 TargetId` with no target authority check

**Severity**: High
**Class**: Authority bypass — instakill any entity
**Wire surface**: `Event_NetOut_Kill(INT64 TargetId)` → `gmKillTarget`
cell method
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:482-485 — `<gmKillTarget><INT64 TargetId>`. Direct
"set this entity's health to 0" with a client-chosen target. Future
handler without (a) access_level gate and (b) target authority check
lets any player instakill any entity they can name an id for. Since
entity ids are wire-public via AoI broadcast, all visible NPCs are
candidates; non-visible NPCs are candidates too if their id is
guessable (entity ids on this server are sequential per shard).

**Evidence**
- Ghidra: `019b3820` `Event_NetOut_Kill`.
- entities/defs/SGWGmPlayer.def:482-485.
- No Rust handler. The `kill_handler` in
  `crates/game/src/commands/gm_cmds.rs:74-81` is a slash-command stub
  gated on `AccessLevel::GameMaster` — but again, that's the
  in-process chat-command path, NOT the wire cell-method that the
  SGW client actually emits.

**Attack scenario**
1. Player picks any visible NPC's entity id (or guesses a player id
   in range; entity ids are sequential).
2. Sends `gmKillTarget(<target_eid>)`.
3. Future handler sets the target's HP to 0; the target dies. Kill
   credit / XP / loot accrues to whichever player currently has
   aggro on that target — or to the caller, depending on impl.

**Suggested remediation (one line)**
Handler must enforce access_level + validate the target via
`space_mgr.get_entity(target)` + verify it's in the same space as
the caller; even GM kills should fail closed against ghost ids.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-13 — `Event_NetOut_Spawn` and `Event_NetOut_Despawn` accept client-supplied template ids / target ids

**Severity**: High
**Class**: Entity creation/destruction with client-chosen parameters
**Wire surface**: `Event_NetOut_Spawn` → `gmSpawnByCmd(WSTRING DesignId,
FLOAT XOffset, FLOAT ZOffset)`; `Event_NetOut_Despawn` →
`gmDespawnByCmd(INT32 TargetID)`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:454-464 — `gmSpawnByCmd(WSTRING DesignId, FLOAT XOffset,
FLOAT ZOffset)` and `gmDespawnByCmd(INT32 TargetID)`. Spawn template is a
WSTRING with no whitelist; offsets are unclamped floats; despawn target
is a 32-bit entity id with no authority check.

A future spawn handler with no access_level check lets a regular player
spawn arbitrary NPCs at arbitrary world positions. Worst case:
- Spawn a max-level raid boss in a low-level newbie zone → mass-grief.
- Spawn 10,000 NPCs (loop the call) → DoS the cell.
- Spawn template = a player-faction NPC and use it for PvP-by-proxy.

A future despawn handler with no target authority lets the player remove
any NPC visible to them — clearing camps, removing quest objectives,
etc.

**Evidence**
- Ghidra: `019b3bd8` `Event_NetOut_Spawn`, `019b3c00`
  `Event_NetOut_Despawn`.
- entities/defs/SGWGmPlayer.def:454-464.
- No Rust handler. `spawn_handler` in
  `crates/game/src/commands/gm_cmds.rs:50-61` is a slash-command stub
  `AccessLevel::GameMaster`-gated but doesn't share code with the
  wire path.

**Attack scenario**
1. Player crafts a `gmSpawnByCmd("HighLevelRaidBoss_Apophis", 0.0, 0.0)`
   cell call.
2. Future handler resolves the template + spawns at caller's position.
3. Boss kills every other player in AoI; OR caller solos a boss
   designed for raid-group XP/loot.

**Suggested remediation (one line)**
Access-level gate + DesignId whitelist (e.g., only spawn templates
flagged `gm_spawnable` in the content DB) + offset clamping (e.g.,
within ±50m of caller); despawn must enforce same-space + access_level.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-14 — `Event_NetOut_GiveAbility` / `GiveAllAbilities` / `GiveBlueprint` / `GiveGearset` accept arbitrary ability/blueprint ids

**Severity**: High
**Class**: Authority bypass — content unlock via client setter
**Wire surface**: `gmGiveAbility(INT32 aAbilityID)`,
`gmGiveAllAbilities()`, `gmGiveTrainingPoints(INT32)`,
`gmGiveExpertise(INT32, INT32)`, `gmGiveAppliedSciencePoints(INT32)`,
`gmGiveRacialParadigmLevels(INT32, INT32)`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:202-232 — a family of GiveXxx methods, each accepting
INT32 ids / quantities with no validation. `gmGiveAllAbilities()` has
ZERO args — fire-and-forget all-ability-unlock. The other variants
take an INT32 ability/expertise/discipline id with no whitelist.

A future implementer that mirrors the Python's "trust the GM" semantic
without the access_level gate exposes every player to:
- Self-unlocking any ability id (including dev-only test abilities).
- Maxing all crafting / R&D progression in one call.
- Maxing racial paradigm levels (`gmGiveRacialParadigmLevels`).

**Evidence**
- Ghidra: `019b3794` `Event_NetOut_GiveAbility`, `019b36b0`
  `Event_NetOut_GiveAllAbilities`, etc. — all RTTI strings present.
- entities/defs/SGWGmPlayer.def:202-232.
- No Rust handler.

**Attack scenario**
1. Player sends `gmGiveAllAbilities()` — single cell-method with no
   args.
2. Future handler iterates all known abilities and inserts them onto
   the player's `known_abilities`.
3. Player can now invoke any ability via `useAbility`.

**Suggested remediation (one line)**
Access_level gating per CAT-N-03; the AbilityID arg must be
whitelisted against the loaded ability set; `gmGiveAllAbilities` has
no business existing on the non-GM wire surface and should reject
unconditionally below the access_level check.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-15 — `Event_NetOut_SetMobAttribute` / `SetMobAbilitySet` / `SetMobStance` / `SetMobVariable` carry arbitrary mob id + arbitrary mutation payload

**Severity**: Medium
**Class**: Authority bypass — mob mutation via client setter
**Wire surface**: `gmSetMobAttribute(INT32 TargetID, WSTRING Attribute,
WSTRING AttributeType, INT32 Value)`,
`gmSetMobAbilitySet(INT32 aAbilitySetId)`,
`gmSetMobStance(INT32 aNewStance)`, `setMobVariable(INT32, INT32)`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:307-321, 470-476, 561-565. These methods mutate NPC
attributes / ability sets / stances via client-supplied identifiers.
The `gmSetMobAttribute` shape `(TargetID, Attribute name, type name,
value)` is the dangerous one: it's a reflection-style "set any property
to any value" on any target. A future handler that doesn't validate
the attribute name against a property whitelist lets a non-GM player
write any property of any visible mob (cell-public properties carry
no GM-only flag; an `attribute name` of `bGodMode`, `level`, `faction`
etc. would all map onto real fields).

Lower severity than instakill because the worst-case is still a
combat-balance imbalance, not an instant-kill or item dupe.

**Evidence**
- Ghidra: `Event_NetOut_SetMobAttribute`, `SetMobAbilitySet`,
  `SetMobStance`, `SetMobVariable` strings present.
- entities/defs/SGWGmPlayer.def:307-321, 470-476, 561-565.

**Attack scenario**
1. Player sends `gmSetMobAttribute(<boss_id>, "level", "INT32", 1)`.
2. Future handler reflects the WSTRING attribute name to a setter on
   the mob; boss drops from level 60 to level 1.
3. Solo the boss for raid-loot.

**Suggested remediation (one line)**
Access_level + an explicit per-attribute-name whitelist; treat
the WSTRING attribute arg as untrusted input that must match an
allow-list (no reflection without a closed set).

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-16 — `Event_NetOut_EmitBehaviorEventOnMob` / `AddBehaviorEventSet` / `RemoveBehaviorEventSet` fire arbitrary scripted behaviors on any mob

**Severity**: Medium
**Class**: Authority bypass — fire scripted AI events on client-chosen target
**Wire surface**: `gmEmitBehaviorEventOnMob(INT32 aBehaviorEventId)` +
the two `BehaviorEventSet` mutators
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:437-450. The `gmEmitBehaviorEventOnMob` takes a single
INT32 — the behavior event id — and applies it to the caller's current
target (no target arg in this wire shape; the Python reference resolves
to the cell's target). A future handler that fires the event without
access_level gating lets a regular player nudge any NPC into any AI
state — including states normally reached only on specific
trigger conditions (e.g., "boss enrages", "NPC drops loot", "NPC opens
dialog").

The two `*BehaviorEventSet` methods add/remove entire AI event sets
on a target mob, a more powerful primitive (can reshape the target's
behavior tree at runtime).

**Evidence**
- Ghidra: `Event_NetOut_EmitBehaviorEventOnMob`, `AddBehaviorEventSet`,
  `RemoveBehaviorEventSet` strings present.
- entities/defs/SGWGmPlayer.def:437-450.

**Attack scenario**
1. Player engages a tough boss.
2. Sends `gmEmitBehaviorEventOnMob(<flee_event_id>)`.
3. Future handler fires flee on the boss — combat ends without
   boss attacking back; boss dies on respawn timer (or wanders into
   range of a friendly NPC who finishes it for free XP/loot).

**Suggested remediation (one line)**
Access_level gating + a whitelist of "safe" behavior events
(emit_event must verify the event is in a curated set if the caller
isn't a GM, which should always be the case here).

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-17 — `Event_NetOut_LoadConstants` / `LoadAbility` / `LoadAbilitySet` / `LoadBehavior` / `LoadMOB` / `LoadInteractionSet` / `LoadItem` / `LoadMission` / `LoadNACSI` — wire surface for hot-reloading server-side content data

**Severity**: High
**Class**: Authority bypass — server-data reload via client trigger
**Wire surface**: 9 distinct `Load*` cell methods on SGWGmPlayer
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:517-559 — a family of `load*` methods on SGWGmPlayer
that, per the Python reference, hot-reload server-side content data
(ability defs, mob defs, mission defs, etc.) from disk. The wire
signature is single-arg (usually `INT32 aXxxId`) with no validation.

If/when implemented in Rust, a non-GM player triggering these:
- Causes a server-wide file read on every call → DoS shape (especially
  `loadConstants` which has no id arg → reload everything).
- Could trigger reload races during live combat — if `loadAbility`
  reloads an ability def while it's being resolved on an in-flight
  combat tick, the cell can crash or produce inconsistent damage.

**Evidence**
- Ghidra: All 9 `Event_NetOut_Load*` strings present in the binary
  near `019b370c+` (the Give/Set NetOut block).
- entities/defs/SGWGmPlayer.def:517-559.

**Attack scenario**
1. Player loops `loadConstants()` at maximum tick rate.
2. Future handler does a synchronous file read on each call → cell
   thread stalls, all other players in the cell experience
   tick-lag / disconnects.

**Suggested remediation (one line)**
Access_level gating + an explicit dev-only build flag for the entire
reload family — these should not be reachable in a release build at
all without an environment variable.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-18 — `Event_NetOut_Goto` / `GotoXYZ` / `GotoLocation` / `Summon` — debug teleport with no navmesh / authority check

**Severity**: High
**Class**: Position spoof via client setter
**Wire surface**: `gmGoto(WSTRING aNameOrID)`, `gmGotoXYZ(FLOAT, FLOAT,
FLOAT)`, `gmGotoLocation(WSTRING WorldName, FLOAT X, FLOAT Y, FLOAT Z)`,
`gmSummon(WSTRING aNameOrID)`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:323-353 — teleport commands. The most dangerous shape
is `gmGotoXYZ(x, y, z)` which sets the player's position to arbitrary
floats — no clamping, no navmesh reachability check, no space-boundary
check. `gmSummon` and `gmGoto` resolve a player name and teleport the
*caller* to the target — meaning a non-GM player gains a free "find
player and teleport to them" primitive even if the target is in a
restricted area (raid instance, GM-only zone).

This is the canonical position-spoof class described in the agent
brief's CAT-B handling. CAT-N includes it because of the dev-command
origin; CAT-B agent owns the broader movement-physics validation.

**Evidence**
- Ghidra: `Event_NetOut_Goto`, `GotoXYZ`, `GotoLocation`, `Summon`
  strings present.
- entities/defs/SGWGmPlayer.def:323-353.
- Today: no Rust handler (the `teleport_handler` in
  `crates/game/src/commands/gm_cmds.rs:63-72` is a slash-command stub
  with `AccessLevel::GameMaster` gating, NOT the wire path).

**Attack scenario**
1. Player sends `gmGotoXYZ(<raid_boss_room_coords>)`.
2. Future handler writes the player's position directly.
3. Player is now inside the raid boss room without going through
   the dungeon — skipping all the trash, all the gates, all the
   instance-entry checks.

**Suggested remediation (one line)**
Access_level + navmesh reachability check from caller's last
server-confirmed position + space-boundary check; never write a
client-supplied position unless the caller is GM AND the position
is in-bounds.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-19 — `Event_NetOut_ShowIP` / `ShowInventory` / `ShowPlayer` — info-disclosure setters with client-supplied target

**Severity**: Medium
**Class**: Info disclosure via client setter
**Wire surface**: `gmShowIP(INT32 TargetID)`,
`gmShowInventory(INT32 TargetID)`, `gmShowPlayer(INT32 TargetID)`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:164-177. These methods send back the target's IP /
inventory contents / general player info. The wire shape carries
client-chosen `TargetID`. A future handler without access_level gating
becomes a "look up any player's IP address" primitive (PII leak),
"look up any player's inventory" (loot scouting), "look up any player's
location / stats" (PvP target scouting).

`gmShowIP` is the worst — explicit PII (player IP) disclosure.

**Evidence**
- Ghidra: `Event_NetOut_ShowIP`, `Event_NetOut_ShowInventory`,
  `Event_NetOut_ShowPlayer` strings present.
- entities/defs/SGWGmPlayer.def:164-177.

**Attack scenario**
1. Player sends `gmShowIP(<another_player_eid>)`.
2. Future handler resolves the target → reads
   `connected[target_addr].peer_ip` (or similar) → sends back via
   `onShowPlayer` client callback.
3. Caller now has the target's IP address.

**Suggested remediation (one line)**
Access_level gating; `gmShowIP` should be GM-only at minimum and
ideally audit-logged on every invocation.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-20 — `Event_NetOut_Invisible` / `XRayEyes` — visibility / vision toggles bypass legitimate stealth/visibility systems

**Severity**: Medium
**Class**: Authority bypass — visibility toggles via client setter
**Wire surface**: `onInvisible(UINT8 bTurnOn)`, `onXRayEyes(UINT8 bTurnOn)`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:635-643. `onXRayEyes` — see through walls / through
stealth. `onInvisible` — render self invisible. Both single-byte
toggles. Without access_level gating, any player can fly through
walls visually OR vanish from other players' AoI, breaking PvP /
positional combat fundamentals.

**Evidence**
- Ghidra: `Event_NetOut_Invisible`, `Event_NetOut_XRayEyes` strings
  present (in the SGWGmPlayer-keyed block near `019b3848` and
  neighbours).
- entities/defs/SGWGmPlayer.def:635-643.

**Attack scenario**
1. Player flips `onInvisible(1)`.
2. Future handler sets the visibility bit; AoI broadcasts to other
   players suppress this entity.
3. Player can attack from invisibility in PvP — opponents have no
   sight of the attacker.

**Suggested remediation (one line)**
Access_level gating; if invisibility is meant for some non-GM
flow (e.g., stealth ability), it must come from the ability path
(`useAbility` → effect-grant), not a direct toggle.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-21 — `Event_NetOut_SetFlag` / `SetInstanceFlag` / `ShowFlag` / `ShowInstanceFlag` — arbitrary global/instance flag mutation

**Severity**: Medium
**Class**: Authority bypass — game state mutation via client setter
**Wire surface**: `gmSetFlag(INT32 aFlagId, UINT8 aForceVal)`,
`gmSetInstanceFlag(INT32 aFlagNumber, INT8 aFlagValue)`,
`gmShowFlag(INT32 aFlagId)`, `gmShowInstanceFlag(INT32 aFlagNumber)`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:144-147, 283-287, 580-589. Flags are the game's
shared state (mission progress, quest unlocks, world events, etc.).
A future handler without gating lets any player set or read any
flag — bypassing mission gates ("set the 'completed-prologue' flag
to 1 to skip the intro chain"), or scouting the value of flags they
shouldn't see (instance flags often hold seed values, raid lockout
counters, etc.).

`gmShowFlag` is read-only and less severe; the setters are higher
severity but the wire shapes are siblings.

**Evidence**
- Ghidra: `Event_NetOut_SetFlag`, `SetInstanceFlag`, `ShowFlag`,
  `ShowInstanceFlag` strings present.
- entities/defs/SGWGmPlayer.def lines as cited.

**Attack scenario**
1. Player sends `gmSetFlag(<key_quest_completion_flag>, 1)`.
2. Future handler writes the flag without GM check.
3. Mission/raid/instance state advances without the player having
   played the content.

**Suggested remediation (one line)**
Access_level gating; flag setters should be GM-only at minimum.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-22 — `Event_NetOut_SetTarget` (GM variant) accepts WSTRING name-or-id with no AoI / perception check

**Severity**: Medium
**Class**: AoI bypass — target acquire via client setter
**Wire surface**: `gmSetTarget(WSTRING aNameOrID)`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:302-305 — `gmSetTarget(WSTRING aNameOrID)`. Differs
from the regular `setTargetID(INT32 aTargetID)` (CM 0; CAT-C surface)
in that it resolves names → ids server-side. Once it resolves, the
target is set without verifying caller's perception. The regular
`setTargetID` already has this property (no AoI check; see
`crates/services/src/cell/cell_methods/being.rs:20-63`) — but
that's the agent's CAT-C concern. The CAT-N variant is worse because
the WSTRING resolution can return targets not currently in the
caller's AoI (e.g., name-lookup across the entire shard's online
players).

**Evidence**
- Ghidra: `Event_NetOut_SetTarget` (in addition to the regular
  `setTargetID` wire).
- entities/defs/SGWGmPlayer.def:302-305.
- `crates/services/src/cell/cell_methods/being.rs:20-63` shows the
  regular `setTargetID` already accepts any int with no perception
  check.

**Attack scenario**
1. Player sends `gmSetTarget("BossNameInAnotherZone")`.
2. Future handler resolves name → entity_id → writes to
   `current_target_id`.
3. Caller's combat / autocycle path can now target an entity
   outside their AoI.

**Suggested remediation (one line)**
Access_level gating; after name resolution, the target_id must pass
the same perception/AoI check as the regular setTargetID path —
which itself needs a perception check (CAT-C).

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-23 — `Event_NetOut_GMRemoveItem` allows client-supplied `ItemID + INT16 quantity` deletion

**Severity**: Medium
**Class**: Authority bypass — inventory deletion via client setter
**Wire surface**: `gmRemoveItem(ItemID itemID, INT16 quantity)`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:196-200 — `gmRemoveItem(ItemID itemID, INT16 quantity)`.
The `ItemID` type is the inventory row id (server-trusted in
`crates/services/src/base/world_entry/methods/inventory/` paths).
But because the wire surface accepts it as a client-supplied INT,
a future handler without GM gating + ownership check lets the caller
delete arbitrary inventory rows — including other players' items if
the row id is guessable (it's a sequential bigint per server).

The signed INT16 quantity is also a footgun: negative quantity =
add (depending on impl), so this can become a give-item primitive
in disguise.

**Evidence**
- Ghidra: `Event_NetOut_GMRemoveItem` (RTTI string present).
- entities/defs/SGWGmPlayer.def:196-200.

**Attack scenario**
1. Attacker guesses or scrapes target's `sgw_inventory.item_id`.
2. Sends `gmRemoveItem(<victim_item_id>, 1)`.
3. Future handler executes a DB DELETE without owner check.
4. Victim's item vanishes. Reverse: negative quantity could add.

**Suggested remediation (one line)**
Access_level gating + the row must belong to the caller's
character_id (the standard `where character_id = $player_id`
guard from CAT-D handlers).

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-24 — `Event_NetOut_TestLOS` and `ToggleCombatLOS` — info-leak / runtime-toggle of combat LOS testing

**Severity**: Low
**Class**: Info disclosure / runtime-toggle without GM gate
**Wire surface**: `testLOS(INT32 aSourceEntityID, INT32 aTargetEntityID)`,
`toggleCombatLOS()`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:619-628 — `testLOS` returns whether two arbitrary
entity ids have line-of-sight; `toggleCombatLOS` flips the cell's
combat-LOS-enforcement flag globally. The former is a
target-scouting primitive (which mobs / players can see each other);
the latter is a server-side rule toggle that, if accessible to
non-GMs, lets a regular player disable LOS for all combat in the
cell — making cover useless.

**Evidence**
- Ghidra: `Event_NetOut_TestLOS`, `Event_NetOut_ToggleCombatLOS`
  strings present.
- entities/defs/SGWGmPlayer.def:619-628.

**Attack scenario**
1. Player sends `toggleCombatLOS()` — single zero-arg call.
2. Future handler flips the cell-global flag; cover is now ignored
   in combat resolution.
3. Caller — and every player in the cell — can hit through cover.

**Suggested remediation (one line)**
Access_level gating; `toggleCombatLOS` is a designer-only test
toggle and must never be reachable on a release build.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-25 — `Event_NetOut_PerfStats` / `PerfStatsByChannel` accept client-supplied performance counters

**Severity**: Low
**Class**: Trust-of-client-stats / log noise
**Wire surface**: `perfStats(FLOAT fpsAvg, FLOAT fpsMin, FLOAT fpsMax,
FLOAT bpsIn, FLOAT bpsOut, FLOAT packetsIn, FLOAT packetsOut,
FLOAT lagAvgMS, FLOAT lagMinMS, FLOAT lagMaxMS, FLOAT resends,
FLOAT AppearanceJobs)` (regular SGWPlayer BaseMethod),
`gmPerfStatsByChannel(INT8 aOnOff)` (SGWGmPlayer CellMethod)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
`perfStats` is exposed on the *regular* SGWPlayer.def
(`SGWPlayer.def:529-543`) and carries 12 floats of client-side
performance telemetry. Today the Rust server doesn't handle it
(falls through to the unhandled-base-method warn arm). The wire
surface is wide open — any future handler that ingests these
floats into a telemetry pipeline or anti-cheat heuristic must
remember the values are 100% client-controlled and can be
arbitrary (NaN, +inf, manipulated to mislead anti-cheat).

`gmPerfStatsByChannel` is the SGWGmPlayer toggle for
per-channel perf logging — same handler-doesn't-exist-yet status
but the wire is reachable on the GM index space once SGWGmPlayer
is enabled.

**Evidence**
- Ghidra: `Event_NetOut_PerfStats`, `Event_NetOut_PerfStatsByChannel`.
- entities/defs/SGWPlayer.def:529-543 (perfStats), SGWGmPlayer.def
  (perfStatsByChannel).

**Attack scenario**
1. Player sends `perfStats(fps=999, bpsIn=0, ...)` continuously to
   inflate / poison any FPS-based anti-cheat heuristic.
2. Future handler logs the values into observability.
3. Anti-cheat ML/heuristic training data is poisoned.

**Suggested remediation (one line)**
Any consumer of these floats must clamp / NaN-reject them and
weight them as "advisory only" — never as authoritative for
anti-cheat decisions.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-26 — `Event_NetOut_SetMovementType` on SGWPlayer (CM 1) accepts the full `EMobMovementType` enum (0-6), exposing NPC-AI-only states to the player

**Severity**: Low
**Class**: Authority bypass — NPC-AI-state byte exposed at player-CellMethod index
**Wire surface**: `setMovementType(UINT8 aMovementType)` (CM 1)
**Demonstrable / Likely-theoretical**: Demonstrable (handler IS
implemented and accepts the bytes; effect-on-game is unclear)

**Trust violation**
`crates/services/src/cell/cell_methods/being.rs:65-105` accepts the
client's movement-type byte and broadcasts it to AoI witnesses (also
caches it on the entity). The byte values are the
`EMobMovementType` enum: `Cover=0, CombatAdvance=1, Patrol=2,
Follow=3, Wander=4, Leash=5, Avoid=6`. These are NPC AI states —
they're not meaningful for the player. A regular player sending
`setMovementType(5)` (`Leash`) gets the byte broadcast to all
witnesses; what they make of it depends on the client's
animation/AI rendering pipeline. Worst case: the byte triggers
a client-side AI animation on the player (e.g., wander/patrol)
that's not part of the player's normal pose set — visually
disruptive but not exploit-class on its own.

CAT-N severity is Low because: (a) the handler is implemented
defensively (unknown bytes 7+ are dropped with a warn — already
correctly hardened), and (b) the wire surface is intended for
NPCs/peers per the BigWorld call-on-ghost model anyway, so this is
mostly the client passing through AI broadcasts. But: nothing
limits the byte to player-valid values (probably 0-1 for
Walk/Run). A player can persistently broadcast a stale AI state
to their AoI peers.

**Evidence**
- entities/defs/interfaces/SGWBeing.def:225-228 — `setMovementType`
  is `Exposed`, taking `UINT8 aMovementType`.
- `crates/services/src/cell/cell_methods/being.rs:65-105` — handler
  accepts 0-6, rejects 7+, broadcasts to witnesses.

**Attack scenario**
1. Player sends `setMovementType(5)` (`Leash`).
2. Server caches + broadcasts to AoI witnesses.
3. Witnesses see the player marked as in an `EMobMovementType::Leash`
   state, which the client may render differently.

**Suggested remediation (one line)**
Filter the accepted byte set per entity-kind: players should only be
allowed to send walk/run movement-types; the full enum is reserved
for NPC ghost-call paths.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-27 — `Event_NetOut_Users` and `Event_NetOut_ShowMobCount` — info-disclosure surfaces with no GM check

**Severity**: Low
**Class**: Info disclosure via client setter
**Wire surface**: `gmUsers()` (zero-arg), `gmShowMobCount(INT32 SpaceID)`
(cell) / `gmShowMobCount(WSTRING aAreaKey)` (base)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:363-365 (`gmUsers`), 159-162 (cell `gmShowMobCount`),
715-717 (base `gmShowMobCount`). These leak the shard's user count and
the current mob count by space. Modest info disclosure — useful for an
attacker doing population mapping to find empty zones for griefing or
overloaded zones for DoS targeting. Low severity because the data
isn't PII and doesn't directly enable an exploit on its own.

**Evidence**
- Ghidra: `Event_NetOut_Users`, `Event_NetOut_ShowMobCount` strings
  present.
- entities/defs/SGWGmPlayer.def lines cited.

**Attack scenario**
1. Loop `gmShowMobCount(<every_space_id>)` → map of mob population
   per space.
2. Choose under-populated space to spawn-camp; over-populated to
   DoS-target.

**Suggested remediation (one line)**
Access_level gating; info-disclosure surfaces must still respect GM
boundary even if "merely" telemetric.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-28 — `Event_NetOut_GiveStargateAddress` / `RemoveStargateAddress` — gate-network unlocks via client setter

**Severity**: Medium
**Class**: Authority bypass — fast-travel network unlock
**Wire surface**: `gmGiveStargateAddress(WSTRING AddressId, INT64
TargetId, UINT8 Hidden)`, `gmRemoveStargateAddress(WSTRING AddressId,
INT64 TargetId)`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:504-515. Stargate addresses are the SGW fast-travel
network — gating which destinations a player can dial via the DHD.
Adding addresses unlocks fast-travel; removing them locks the target
out. Without access_level gating, any player can self-unlock the
entire gate network (skipping the in-game progression that gates
addresses), or grief another player by removing their addresses.

The TargetId field also allows other-player mutation.

**Evidence**
- Ghidra: `Event_NetOut_GiveStargateAddress`, `RemoveStargateAddress`.
- entities/defs/SGWGmPlayer.def:504-515.

**Attack scenario**
1. Player sends `gmGiveStargateAddress("Atlantis", <self_eid>, 0)`.
2. Future handler inserts the address into the player's gate book.
3. Player can now dial Atlantis from any gate, skipping the
   in-game mission chain that unlocks it.

**Suggested remediation (one line)**
Access_level gating + ownership check on TargetId.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-29 — `Event_NetOut_RequestReload` (CM 86) handler does not validate that the reload is initiated by the entity-owner

**Severity**: Low
**Class**: Authority bypass — but for a player-facing method, low risk
**Wire surface**: `requestReload(UINT8 reloadType)` (CM 86, normal player method)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
The `requestReload` handler at
`crates/services/src/cell/cell_methods/player/world/mod.rs:193-200`
takes the dispatched `entity_id` from the cell-method framing and
calls `handle_reload(entity_id, ...)`. The framing layer is supposed
to substitute `entity_id = caller's player_eid` (per the connect-loop's
cell-method routing); a regression here means a player could trigger
another player's reload. Today the framing path does substitute
correctly (`crates/services/src/base/...connect_loop...`), so this is
benign — but the handler itself trusts the supplied entity_id without
cross-checking that it matches the caller's player_eid. Low severity
because the worst-case effect is "another player's mag refills" which
is friendly, not exploit-class.

This is included for completeness of the CAT-N review even though the
exploit shape is mild — RequestReload was explicitly listed in the
CAT-N scope.

**Evidence**
- `crates/services/src/cell/cell_methods/player/world/mod.rs:193-200`
  — calls `handle_reload(entity_id, ...)` with no caller-vs-entity
  cross-check.
- Framing layer substitution: `crates/services/src/base/connect_loop/`
  (caller_eid → entity_id is the framing layer's responsibility, not
  the handler's).

**Attack scenario**
1. Player A forges a cell-method call setting `entity_id = <player_B_eid>`.
2. Framing layer should overwrite the entity_id with player A's
   player_eid — but if a future refactor removes that substitution,
   the handler trusts B's id and reloads B's bandolier.

**Suggested remediation (one line)**
Defense-in-depth: the cell-method handler should assert
`entity_id == caller_player_eid` (which requires plumbing caller
identity into dispatch — same plumbing as CAT-N-03).

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-30 — `Event_NetOut_ToggleCombatDebug` / `ToggleCombatVerboseDebug` / `ToggleHealDebug` accepted at SGWAbilityManager (CM 2/3) and SGWCombatant (CM 6) without GM check

**Severity**: Low
**Class**: Info disclosure (potential — handler is currently a stub)
**Wire surface**: `toggleCombatDebug()` (CM 2), `toggleCombatVerboseDebug()`
(CM 3), `toggleHealDebug()` (CM 6)
**Demonstrable / Likely-theoretical**: Demonstrable (handlers exist and
are stubs)

**Trust violation**
These three are declared on the SGWAbilityManager / SGWCombatant
interfaces (which every combat entity implements, including the
non-GM SGWPlayer), with `<Exposed/>`. Today the Rust handlers
log-and-no-op
(`crates/services/src/cell/cell_methods/ability_manager.rs:22-25`,
`crates/services/src/cell/cell_methods/combatant.rs:59-62`). When
implemented to actually surface combat/damage/heal debug info — the
intended design per the Python reference — the data they leak (raw
QR rolls, damage formulas, target HP, modifier breakdown) gives the
caller information that's normally hidden from the client (e.g., a
boss's exact remaining HP, the QR threshold that just decided the
crit roll).

Today: benign. Future: info disclosure if implemented without GM
gating.

**Evidence**
- entities/defs/interfaces/SGWAbilityManager.def:249-256 — both
  toggle methods Exposed.
- entities/defs/interfaces/SGWCombatant.def — `toggleHealDebug` is
  the third (CM 6).
- `crates/services/src/cell/cell_methods/ability_manager.rs:22-25`
  + `combatant.rs:59-62` — stub log-only handlers.

**Attack scenario**
1. Player flips `toggleCombatDebug()` once implemented.
2. Server-side combat resolution starts dumping per-roll details
   to the player's combat log.
3. Player sees boss-HP / QR-threshold information they shouldn't
   have, enabling optimization of damage rotations or
   boss-aware-attack timing.

**Suggested remediation (one line)**
Access_level gating; these toggles must require GM.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-31 — `Event_NetOut_DebugEvents` / `DebugInteract` carry client-supplied `TargetId` and arbitrary debug levels

**Severity**: Low
**Class**: Info disclosure / debug-trigger via client setter
**Wire surface**: `gmDebugEvents(INT32 TargetId, INT32 InformLevel)`,
`gmDebugInteract()`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:421-435. The `gmDebugEvents` shape sets event-throw /
event-receive debugging on an arbitrary `TargetId`, with a
client-supplied `InformLevel`. Once implemented this becomes a
"dump all behavior events on any target mob" primitive — useful for
boss-fight scouting (understanding a boss's full event tree).

**Evidence**
- Ghidra: `Event_NetOut_DebugEvents` string present.
- entities/defs/SGWGmPlayer.def:421-435.

**Attack scenario**
1. Player sends `gmDebugEvents(<boss_id>, 3)`.
2. Future handler enables full event tracing on the boss; client
   sees every behavior event the boss fires.
3. Player reverse-engineers the boss AI from the trace.

**Suggested remediation (one line)**
Access_level gating.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-32 — `Event_NetOut_RegenerateCoverLinks` / `ChangeCoverWeight` / `ChangeCoverStanceWeight` — runtime AI tuning via client setter

**Severity**: Low
**Class**: Server tuning bypass via client setter
**Wire surface**: `regenerateCoverLinks(FLOAT NormalLimit, UINT32
MaxLinks, FLOAT MaxDistance)`, `changeCoverWeight(...)`,
`changeCoverStanceWeight(...)`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:656-682. These methods retune the cell's NPC cover
pathfinding at runtime, with client-supplied float weights. A future
handler without gating lets any player nudge enemy AI to make
suboptimal cover choices (sit in the open, never break cover, etc.).
Worst case: a player tunes the NPC cover weights such that enemies
attack from random low-effective positions and die easily.

`regenerateCoverLinks` also takes a `MaxLinks: UINT32` with no
bound — looping with a huge value could DoS the cell on cover-link
regeneration.

**Evidence**
- Ghidra: `Event_NetOut_RegenerateCoverLinks`, `ChangeCoverWeight`,
  `ChangeCoverStanceWeight` strings present.
- entities/defs/SGWGmPlayer.def:656-682.

**Attack scenario**
1. Player sends `regenerateCoverLinks(0.001, INT32::MAX, 9999.0)`.
2. Future handler runs an O(N²) cover-link regen with billions of
   links — cell thread stalls for seconds.

**Suggested remediation (one line)**
Access_level gating + bound the `MaxLinks` to sane limits even when
caller is GM.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-33 — `Event_NetOut_PrintStats` exposes per-cell statistics for client capture

**Severity**: Low
**Class**: Info disclosure via client setter
**Wire surface**: `gmPrintStats(WSTRING aStat)`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:372-375 — `gmPrintStats(WSTRING aStat)`. Per Python
reference, dumps per-cell statistics matching the named stat
category. Without GM gating, lets any player observe cell-internal
state (NPC count, tick rate, memory usage, event-rate per channel).
Operational info disclosure shape only — not direct exploit.

**Evidence**
- Ghidra: `Event_NetOut_PrintStats`.
- entities/defs/SGWGmPlayer.def:372-375.

**Attack scenario**
1. Player loops `gmPrintStats("tickrate")` → measures the cell's
   tick lag remotely.
2. Player uses the tick-lag signal to time-coordinate griefing
   actions during stalls.

**Suggested remediation (one line)**
Access_level gating.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-34 — `Event_NetOut_GiveRespawner` / `GiveGearset` / `GiveInventory` / `GiveBlueprint` accept arbitrary template / set ids

**Severity**: Medium
**Class**: Authority bypass — content unlock via client setter
**Wire surface**: `gmGiveRespawner(INT32 aRespawnerMobID)`,
`gmGiveGearset` (no .def-confirmed args), `gmGiveInventory` (no
.def-confirmed args), `gmGiveBlueprint` (not in SGWGmPlayer.def
proper but in the NetOut RTTI strings)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:212-215 — `gmGiveRespawner(INT32 aRespawnerMobID)`.
The other Give* in this group (`GiveGearset`, `GiveInventory`,
`GiveBlueprint`) appear in the Ghidra NetOut string table but are not
all declared in the visible SGWGmPlayer.def excerpt — likely they're
implemented via the slash-command → server-side resolution path in
the Python reference. Same trust violation as CAT-N-14: arbitrary
template ids accepted from client; future handler that doesn't gate
unlocks content for free.

**Evidence**
- Ghidra: `Event_NetOut_GiveRespawner`, `GiveGearset`,
  `GiveInventory`, `GiveBlueprint` strings present.
- entities/defs/SGWGmPlayer.def:212-215.

**Attack scenario**
1. Player sends `gmGiveGearset(<endgame_gearset_id>)`.
2. Future handler grants the gear without level/quest gates.
3. Player skips entire gearing progression.

**Suggested remediation (one line)**
Access_level gating + per-gearset prerequisite check (level, faction,
mission completion).

**Would benefit from x64dbg trace?**
Yes — the gearset / inventory grant flow in the binary may use
templates that aren't fully captured in the .def excerpt; x64dbg
breakpoints would confirm the wire shape of these specific Gives.

---

### CAT-N-35 — `Event_NetOut_SetFaction` allows arbitrary faction reassignment

**Severity**: Medium
**Class**: Authority bypass — faction-membership setter
**Wire surface**: `Event_NetOut_SetFaction` (declared in NetOut
RTTI strings; per Python reference this is `gmSetFaction` which is
not directly in the SGWGmPlayer.def excerpt but is on the SGWBeing
or related interface based on the wire shape)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The `setFaction` family lets the caller change their (or a target's)
faction. Faction affects PvP-flag, NPC hostility, mission
availability, and access to faction-locked content. Without GM
gating: free faction-hop, mid-fight defection, accessing the
opposing faction's missions.

**Evidence**
- Ghidra: `Event_NetOut_SetFaction` string (in the SetGodMode /
  SetNoXP block — same family as the other Set* gm-commands).
- Python reference shows `gmSetFaction` as the corresponding
  cell method.

**Attack scenario**
1. Player in PvP combat with a hostile faction member sends
   `gmSetFaction(<their_own_faction>)`.
2. PvP-flag drops; opponent can no longer attack.
3. Caller can disengage at will.

**Suggested remediation (one line)**
Access_level gating; if any non-GM faction-change feature is added,
it must route through a dedicated, designer-controlled path with
cost/cooldown, not via the GM setter.

**Would benefit from x64dbg trace?**
Yes — confirm the wire shape of `Event_NetOut_SetFaction` to know
if it's self-only or also takes a target id.

---

### CAT-N-36 — `Event_NetOut_SetNoXP` / `SetNoAggro` — combat-discipline toggles via client setter

**Severity**: Medium
**Class**: Authority bypass — combat-rule toggle via client setter
**Wire surface**: `gmSetNoXP()` (zero-arg toggle),
`gmSetNoAggro(UINT8 bTurnOn)`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:241-252. `gmSetNoXP` toggles whether the player
gains XP from kills (intended for GMs who want to gain levels via
GiveXp without polluting the normal curve). `gmSetNoAggro` toggles
whether NPCs aggro on the player. Both are server-side flags that,
without GM gating, let a regular player either:

- Set `noXP` to bypass anti-cheat heuristics that look for
  unusual XP gain patterns.
- Set `noAggro` to walk through hostile zones unmolested
  (extreme power-leveling shortcut).

**Evidence**
- Ghidra: `Event_NetOut_SetNoXP`, `Event_NetOut_SetNoAggro`.
- entities/defs/SGWGmPlayer.def:241-252.
- `bNoAggro` property is declared on
  `interfaces/SGWAbilityManager.def:96-100` as `CELL_PUBLIC` —
  meaning the setter, if implemented, affects a wire-visible
  property the server broadcasts.

**Attack scenario**
1. Player sends `gmSetNoAggro(1)`.
2. Future handler sets `bNoAggro = 1`; AoI broadcasts this to
   the cell.
3. NPC aggro tables skip this player; player walks through hostile
   territory un-targeted.

**Suggested remediation (one line)**
Access_level gating; even after gating, broadcasting `bNoAggro`
publicly is questionable (advertising "I'm a GM" to everyone in
the cell defeats `bHideGM`'s purpose).

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-37 — `Event_NetOut_AddBehaviorEventSet` / `RemoveBehaviorEventSet` — AI behavior-set runtime modification per target

**Severity**: Medium
**Class**: Authority bypass — AI tree modification via client setter
**Wire surface**: `gmAddBehaviorEventSet(INT32 aBehaviorEventSetId)`,
`gmRemoveBehaviorEventSet(INT32 aBehaviorEventSetId)`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:442-450. These add or remove entire event sets from
a mob's behavior tree at runtime. More powerful than the single-event
`gmEmitBehaviorEventOnMob` (CAT-N-16): the caller can permanently
disable or enable whole categories of behavior (combat AI, dialog
AI, idle AI). Without gating: a player can lobotomize any boss
("remove all combat behaviors") and kill it trivially.

**Evidence**
- Ghidra: `Event_NetOut_AddBehaviorEventSet`,
  `Event_NetOut_RemoveBehaviorEventSet`.
- entities/defs/SGWGmPlayer.def:442-450.

**Attack scenario**
1. Player targets a boss.
2. Sends `gmRemoveBehaviorEventSet(<boss_combat_eventset>)`.
3. Future handler removes the event set from the boss's behavior
   tree; boss stops attacking; player kills it solo.

**Suggested remediation (one line)**
Access_level gating; the behavior-set id should also be whitelisted
to a curated "safe to manipulate" set.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-38 — `Event_NetOut_Respec` / `RespecAbility` / `ResetAbilities` — per-character ability tree resets via client setter

**Severity**: Medium
**Class**: Authority bypass — economy primitive via client setter
**Wire surface**: `gmRespec()` (zero-arg full respec),
`Event_NetOut_RespecAbility` (per-ability variant in NetOut RTTI),
`Event_NetOut_ResetAbilities` → already covered as
`resetMyAbilities` (CAT-N-02)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
SGWGmPlayer.def:299-301 — `gmRespec` (zero-arg). The `RespecAbility`
variant is per-ability. Without GM gating, lets any player wipe
their training-points investment AND get them back free — the
classic economy-bypass shape covered separately for the
SGWPlayer-exposed `resetMyAbilities` in CAT-N-02. This one is the
SGWGmPlayer-only variant; the exploit shape is identical but the
wire surface is on a different index space.

**Evidence**
- Ghidra: `Event_NetOut_Respec`, `Event_NetOut_RespecAbility`.
- entities/defs/SGWGmPlayer.def:299-301.

**Attack scenario**
Same as CAT-N-02 with the GM variant index instead of the regular
player one.

**Suggested remediation (one line)**
Access_level gating; respec without cost is a GM-only operation.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-39 — `Event_NetOut_SetTechSkill` and `SetAutoCycle` (regular player CM 83) — handler-implementation correctness check for SET_AUTO_CYCLE

**Severity**: Low
**Class**: Self-only state mutation, but no defense-in-depth on the
caller-vs-entity_id assertion
**Wire surface**: `setAutoCycle(INT8 enabled)` (CM 83 on regular
SGWPlayer), `Event_NetOut_SetTechSkill` (likely SGWGmPlayer-only)
**Demonstrable / Likely-theoretical**: Demonstrable for SET_AUTO_CYCLE
(handler is implemented)

**Trust violation**
`SET_AUTO_CYCLE` (CM 83) is a regular-player command (not GM) and
its handler at
`crates/services/src/cell/cell_methods/player/world/mod.rs:18-118`
reads the caller's `last_fired_ability_id` and `current_target_id`
from the server-side entity. That part is correct (server-authority).
The handler then routes through `handle_use_ability_with_kill_credit`
without re-validating that the `current_target_id` is in the caller's
perception list — but `current_target_id` is itself set via
`SET_TARGET_ID` (CM 0) which also doesn't validate perception. So
auto-cycle inherits the missing AoI check from `setTargetID`.

The `Event_NetOut_SetTechSkill` is the GM variant of a tech-skill
setter — same shape concerns as CAT-N-14 (arbitrary content unlock).

**Evidence**
- `crates/services/src/cell/cell_methods/player/world/mod.rs:18-118`
  — uses server-side `current_target_id`, no perception re-check.
- `crates/services/src/cell/cell_methods/being.rs:20-63` —
  `setTargetID` also has no perception check.
- Ghidra: `Event_NetOut_SetTechSkill` string present.

**Attack scenario**
1. Player sets target to an out-of-AoI entity via a crafted
   `setTargetID` packet (no perception check).
2. Enables auto-cycle.
3. Auto-cycle fires ability at the out-of-AoI target every
   cooldown — bypassing line-of-sight, range, and perception
   discipline.

**Suggested remediation (one line)**
The auto-cycle fire path must re-validate target perception /
range on each fire; this overlaps with CAT-C's missing
perception check on `setTargetID`.

**Would benefit from x64dbg trace?**
No.

---

### CAT-N-40 — Wire surface for `Event_NetOut_GotoXYZ` self-emit allows malformed-but-valid float triples to teleport the caller

**Severity**: High
**Class**: Position-spoof via client setter (specific shape)
**Wire surface**: `gmGotoXYZ(FLOAT aX, FLOAT aY, FLOAT aZ)` — also
covered under CAT-N-18 generally, but called out specifically because
of the float-NaN handling concern
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
Reiterating CAT-N-18 with a specific float-validation focus:
`gmGotoXYZ` accepts three IEEE-754 floats with no constraint that
they be finite, within world bounds, or even non-NaN. A future
handler that copies them into the entity's position field will
break (or exploit) any downstream code that doesn't NaN-check.
Specifically:

- NaN position → AoI broadcast → witnesses receive NaN → client
  may crash, may render at random coordinates.
- ±inf position → out-of-bounds, may bypass per-space culling.
- Negative-zero distinct from positive-zero in some hash code paths.

**Evidence**
- entities/defs/SGWGmPlayer.def:348-353.
- No Rust handler today.

**Attack scenario**
1. Player sends `gmGotoXYZ(NaN, NaN, NaN)`.
2. Future handler writes NaN into entity position.
3. AoI tick broadcasts NaN to witnesses; clients render the player
   at "wherever NaN ends up" (typically origin).
4. Distance / hit-detection code that compares against NaN may
   short-circuit (any comparison with NaN is false), making the
   player effectively un-targetable.

**Suggested remediation (one line)**
Float-finite check (and per-space bounds check) before writing
client-supplied floats into authoritative position state — even
after GM gating.

**Would benefit from x64dbg trace?**
No.

---

## Not Filed

The following items I considered but did not file as findings. Some
overlap with other CATs that own them; some are not exploit-class.

- **`Event_NetOut_LogOff` / `LogOff`** — already CAT-A's concern;
  the existing handler is gated on the connection identity
  (the addr resolved by the framing layer), not client-controlled.
  Server validates correctly.

- **`Event_NetOut_Disconnect`** — same as LogOff; CAT-A surface.

- **`Event_NetOut_Unstuck`** — CAT-B (movement) owns this; the
  current Rust dispatch has no handler beyond the warn-fall-through,
  so there's no current trust violation. Future implementation
  needs cooldown + position check.

- **`Event_NetOut_CombatDebug` / `CombatDebugVerbose` /
  `AbilityDebug` / `HealDebug`** — CAT-C territory; the cell
  toggles ARE CAT-N-30 already filed; the slash-command variants
  go through SGWTextCommandMgr and route to the same server-side
  effect. No additional finding beyond CAT-N-30.

- **`Event_NetOut_Petition`** — CAT-L (chat / contact) owns this.
  The petition is a *request* to a GM, not a GM command.

- **`Event_NetOut_Who`** — exposed to all players regularly
  (`SGWPlayer.def`); not a GM command despite the name. The
  `gmShowPlayer` / `gmShowIP` variants are GM and ARE filed as
  CAT-N-19.

- **`Event_NetOut_CancelMovie`** — exposed in SGWPlayer regular
  methods, not GM. Not a CAT-N concern.

- **`Event_NetOut_BroadcastMinimapPing`** — CAT-L (chat) owns this.
  Not a GM command per the .def declaration (it's on
  OrganizationMember interface).

- **`Event_NetOut_OnSpaceQueueStatus`** / `OnSpaceQueueReadyResponse`
  / `OnSpaceQueuedResponse` / `OnStrikeTeamResponse` — these are
  RESPONSES to server prompts (the client confirming a space queue
  result, etc.), exposed on the regular SGWPlayer because they're
  normal-play UI confirmations, not GM. CAT-O owns the world-queue
  surface.

- **`Event_NetOut_SetMovementType` defense beyond CAT-N-26** —
  the broader concern (does the SET_MOVEMENT_TYPE bytes affect
  server-side movement calc?) is CAT-B's movement-physics
  validation. CAT-N-26 captures the GM-adjacent piece (enum-byte
  exposed beyond walk/run).

- **`Event_NetOut_SetCrouched` / `ChangeWeaponState`** — CAT-B
  movement; not GM-flagged in the .def. The current
  SET_CROUCHED handler trusts the byte but it's a local
  pose-state, not a privilege bit. Not a CAT-N exploit.

- **`Event_NetOut_TestLOS` and `ToggleCombatLOS`** — filed as
  CAT-N-24.

- **`Event_NetOut_SendGMShout`** — IS a GM command, but no Rust
  handler exists today and no chat-broadcast infrastructure
  surfaces shouts. When implemented it needs GM gating — but the
  systemic finding (CAT-N-03) already covers it. Filing a
  CAT-N-41 just for this would be redundant.

- **`Event_NetOut_ReloadOrganizations` / `ReloadInventory`** —
  partial overlap with CAT-N-17 (LoadX family). Same severity,
  same fix. Not double-filed.

- **`Event_NetOut_PetInvokeAbility` / `PetAbilityToggle` /
  `PetChangeStance`** — pet ability surfaces. Not GM commands.
  CAT-C owns these.

- **`Event_NetOut_MissionAbandon` / `MissionAssign` /
  `MissionAdvance` / `MissionReset` / `MissionComplete` /
  `MissionSetAvailable` / `MissionList` / `MissionListFull` /
  `MissionDetails` / `MissionClear` / `MissionClearActive` /
  `MissionClearHistory`** — these are GM variants of mission
  control (`gmMissionAssign` etc.), per SGWGmPlayer.def:65-123.
  The shape is identical to other GM authority bypasses
  (arbitrary mission state mutation) but the exploit class is
  the same as CAT-N-21 (flag mutation). Could be filed as
  individual findings but the systemic CAT-N-03 + CAT-N-21
  pattern already covers them. CAT-J (mission) ALSO owns the
  legitimate non-GM `MissionAbandon` and friends.

- **`Event_NetOut_OnPhysics`** — `onPhysics(UINT8 bTurnOn)` on
  SGWGmPlayer.def:645-648. Toggles physics on/off for the player.
  Same shape as CAT-N-20 (visibility toggles); same severity;
  same fix. Captured under "all SGWGmPlayer toggles need
  access_level gating" — not double-filed.

- **`Event_NetOut_SetCallback`** — `gmSetCallback` on SGWGmPlayer.def
  takes a `PYTHON` arg. The PYTHON wire type means **pickled
  Python data deserialized at the server** — a classic pickle-RCE
  primitive in the original Python server. Today the Rust server
  has no PYTHON deserializer for cell-method args (the cell
  decoder doesn't handle the `PYTHON` type at all). When/if
  implemented, this needs (a) GM gating, (b) NO unpickling of
  client data — switch to a typed schema. Not filed as a numbered
  CAT-N because (1) it's a generic CAT-A-style code-execution
  concern about the PYTHON wire type as a whole that affects
  several entries beyond CAT-N, and (2) the Rust server doesn't
  even ingest PYTHON args today.

- **`Event_NetOut_DialGate` / `DHD`** — gate-dialing is the normal
  player path, but `gmDHD` is the GM variant
  (SGWGmPlayer.def:325-328). Same shape as CAT-N-18 (free
  fast-travel). Severity-equivalent; the systemic finding covers
  it. CAT-O (world / gate) owns the legitimate non-GM path.

- **`Event_NetOut_SetRingTransporterDestination`** — already
  has a Rust handler at
  `crates/services/src/cell/cell_methods/player/world/mod.rs:207-228`;
  it's a CAT-O concern. Not GM-marked in .def.

- **`Event_NetOut_DebugAbilityOnMob` / `DebugBehaviorsOnMob` /
  `DebugPathsOnMob` / `DebugMobData`** — GM debug surfaces for
  observation only. CAT-N-31 covers `DebugEvents`; the others
  are sibling shapes with the same severity. Not double-filed.

- **`Event_NetOut_EnterErrorAIState` / `ExitErrorAIState`** —
  AI-state toggles. Same shape as CAT-N-16 (behavior event
  emission). Not double-filed.

- **`Event_NetOut_DespawnMob` / `SpawnEntityLoot` /
  `ActivateSpawnSet` / `DeactivateSpawnSet`** — alternate spawn
  primitives. Same shape as CAT-N-13. Not double-filed.

- **`Event_NetOut_TrackMob`** — debug command for tracking a
  mob's AI state. Info-disclosure shape like CAT-N-19 / N-24.
  Not double-filed.

- **`Event_NetOut_DebugMinigameComplete`** — CAT-K (minigame)
  surface, not pure-GM. Filed elsewhere.

- **`Event_NetOut_OnReady` / `OnPlayerReady` / `OnPlayerFailed`** —
  internal lifecycle methods, not GM commands.

- **`Event_NetOut_setClientVersion`** — sent at handshake, not GM.

- **`Event_NetOut_SetAutoCycle` (CM 83)** — auto-cycle toggle is
  a *regular* player command, not GM. The handler IS implemented
  and uses server-side state correctly (`last_fired_ability_id`,
  `current_target_id`). The remaining concern is target perception
  on the auto-cycle fire path which inherits from `setTargetID`
  — that's CAT-N-39 already filed (perception check).

- **`Event_NetOut_ShowPointSet`** — info-disclosure shape on a
  point-set type; same severity as CAT-N-19. Not double-filed.

- **`Event_NetOut_ShowRotation`** / **`ShowVariable`** — read-only
  observation of an entity's rotation/variable. Low-severity
  info-disclosure same as CAT-N-19. Not double-filed.

- **`Event_NetOut_ListInteractions`** — GM listing of available
  interactions on the caller's target. Same shape as
  `gmShowPlayer`. Not double-filed.

- **`Event_NetOut_GetMobAttribute`** — GM read of a mob attribute.
  Pure info-disclosure variant of CAT-N-15 (set side). Same fix.
  Not double-filed.

- **The remaining `Show*` family in CAT-N** — every Show* method
  is an info-disclosure variant. All share the CAT-N-19 fix
  shape. Not individually filed.

- **`Event_NetOut_NoDamage`** — `gmSetNoDamage` was in SGWGmPlayer
  but `Event_NetOut_NoDamage` doesn't appear in the Ghidra RTTI
  string scan as a separate event — likely it's grouped with
  `SetGodMode` (both toggle damage immunity). Not double-filed
  beyond CAT-N-06.
