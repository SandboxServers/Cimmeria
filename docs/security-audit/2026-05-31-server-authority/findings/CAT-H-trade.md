# CAT-H — Trade (P2P) — Findings

## Overall trust posture

**The Rust server has no trade implementation at all.** All four client-callable
trade RPCs (`tradeRequest`, `tradeRequestCancel`, `tradeUpdateProposal`,
`tradeLockState`) dispatch to a single `match` arm in
`crates/services/src/cell/cell_methods/player/social.rs` that decodes the leading
4–9 header bytes, logs `UNIMPLEMENTED:`, and returns `true` (handled). The handler
never decodes the `LocalTradeProposal` payload (item list + cash + lockState), never
allocates a `TradeTransaction`, never validates the partner exists / is in range /
is alive / is a player, never locks items, never emits the corresponding
server-to-client `onTradeState` (method 144) or `onTradeResults` (method 145), and
never persists or reconciles anything to inventory.

This is a "wire exposed but unimplemented" posture — the same shape that, in MMO
history, has produced silent dupe vectors when the handler is filled in piecewise
without the dedicated state machine, because the wire shape encourages the author
to trust the client's view of "what I put in the window" rather than building a
server-authoritative escrow. The Python reference at
`deprecated/python/cell/Trade.py` shows how the original implementers attempted it
(version-numbered proposals, lock-state re-arming, in-place inventory mutation on
confirm), and that reference itself contains several invariant gaps (no item lock,
no disconnect handling, no `canTrade` check, no atomicity) that would carry over
verbatim into a naive Rust port.

The wire definition in `entities/defs/SGWPlayer.def:1033-1072` plus
`entities/defs/alias.xml:355-379` (the `LocalTradeProposal`/`RemoteTradeProposal`
FIXED_DICTs) confirms that every client-supplied trade field is fully
attacker-controlled: `instanceId` (the item DB row id), `slotId`, `cash` (naquadah
delta), `version`, `lockState`, and the trade partner `EntityId` — all
client-asserted, none of them currently validated against any server-side state.

The bulk of the findings below are therefore "what the server WILL need to validate
when the handler is implemented." Because no handler exists today, the
**demonstrable** attack is degenerate: the bytes are consumed and ignored, no item
movement occurs, no dupe results. The **likely-exploitable theoretical** findings
all become demonstrable the moment a developer fills in the match arms without the
named server-side invariant. Treat this category as "the handler authors will need
these explicit guardrails before the trade UI is unblocked client-side." I list
them so the implementer can pre-load the regression-guard list rather than
discovering each vector by review round-trip.

A cross-disciplinary consult with `social-systems-engineer` is strongly recommended
before any of these are filled in — trade state machines, item-lock invariants, and
deadlock-on-disconnect are their domain.

---

### CAT-H-01 — Entire trade RPC surface is silently consumed, no server-authoritative state machine exists

**Severity**: High (foundational — every subsequent finding rides on this)
**Class**: Missing handler / silent ack
**Wire surface**: `Event_NetOut_TradeRequest`, `Event_NetOut_TradeProposal`,
`Event_NetOut_TradeLockState`, `Event_NetOut_TradeRequestCancel`
**Demonstrable / Likely-theoretical**: Demonstrable (the unimplemented stub is the
present behavior; the danger is the next PR that fills it in piecewise)

**Trust violation**
The four `Exposed` trade methods on `SGWPlayer.def` (lines 1034, 1047, 1053, 1067)
are dispatched by `crates/services/src/cell/cell_methods/player/social.rs:103-149`
to a `match` arm that logs `UNIMPLEMENTED:` and returns `true` (handled). The
handler decodes only the leading INT32 (target EntityId, or LocalVersionId for
tradeLockState) and never touches the rest of the wire payload. There is no
`TradeTransaction` struct anywhere in `crates/` (grep for
`trade_transaction|TradeTransaction|trade_session` returns zero matches outside the
deprecated Python reference). No server-side trade response is ever emitted:
`onTradeState` (method 144) and `onTradeResults` (method 145) are defined as
constants in `crates/services/src/cell/client_methods/player.rs:96,98` and decoded
by `wire_log/decoders/generated.rs` but no production code path emits them.

**Evidence**
- Ghidra: `019d898c`, `019d89c0`, `019d89f0` — strings
  `Event_NetOut_TradeRequestCancel`, `Event_NetOut_TradeLockState`,
  `Event_NetOut_TradeProposal` (the `TradeRequest` variant is name-mangled into the
  vftable type strings at `01e2c224`, `01e37fb8`).
  `00d68330` `SGWNetworkManager_VEvent_NetOut_TradeRequest___EventHandler__vfunc_0`
  + sibling vfunc destructors at `00d68350`/`00d68370`/`00d68390` confirm the
  client wires all four into the standard `SGWNetworkManager::EventHandler<T>`
  Mercury emit pipeline. The client UI verbs (`requestTrade`, `cancelTrade`,
  `setTradeItem`, `setTradeCash`, `setTradeLockState`, `removeTradeItem`) at
  `01952a70`, `01952a58`, `01952a3c`, `019529b4`, `01952944`, `019529d0` are wired
  into a button handler at `00ad5b7c` (CEGUI button-base 3), confirming UI is
  fully present.
- `entities/defs/SGWPlayer.def:1034-1072` — the four `<Exposed/>` trade methods'
  signatures (the wire shape, authoritative).
- `entities/defs/alias.xml:356-379` — `LocalTradeProposal`/`RemoteTradeProposal`
  FIXED_DICT payload shapes.
- Client behavioral log: n/a (trade UI requires a partner online; not exercised in
  the most recent `SGWDebugLog.log`).
- Cross-ref to Rust handler (for the fix author, NOT as truth):
  `crates/services/src/cell/cell_methods/player/social.rs:103-149`.

**Attack scenario**
1. Player A clicks Trade on Player B in the client UI.
2. Client emits `tradeRequest(B.entityId, LocalTradeProposal{version:1,
   items:[], cash:0, lockState:0})`.
3. Server logs `UNIMPLEMENTED: tradeRequest` and returns. No `onTradeState`
   reply is sent.
4. Observable effect on the server: nothing. Observable effect on Player A:
   client likely hangs in the "waiting for partner" UI state, because the
   client-side state machine in `Trade@@` (Ghidra) is expecting an
   `onTradeState` callback from the server. Player B is never told a trade was
   requested.

This is benign today **only because the handler stub does nothing else**. The
moment any subsequent PR fills in even part of this surface (e.g. "emit
`onTradeState` so the UI doesn't hang") without the full escrow state machine, the
class of exploits in CAT-H-02 through CAT-H-09 become live.

**Suggested remediation (one line)**
Do not begin filling in any of the four `match` arms in `social.rs` until the
server-side `TradeTransaction` (escrow with item-lock table, version-counter
arbiter, disconnect rollback) exists and is unit-tested; route the redesign back
to `social-systems-engineer`.

**Would benefit from x64dbg trace?**
Yes — trace the client's `Trade@@` state machine through `requestTrade` →
`onTradeState` round-trip to pin the exact serializer for `LocalTradeProposal`
(item array element count, endian, FIXED_DICT framing) so the Rust handler decodes
it byte-exact when implemented.

---

### CAT-H-02 — `tradeRequest` will trust client-supplied target `EntityId` without same-space / range / alive / is-player check

**Severity**: High (when handler is implemented)
**Class**: Missing target validation
**Wire surface**: `Event_NetOut_TradeRequest` (cell method 104)
**Demonstrable / Likely-theoretical**: Likely-theoretical (handler is a stub today;
the trust violation becomes demonstrable on first fill-in)

**Trust violation**
`tradeRequest` takes an arbitrary client-supplied INT32 EntityId
(`SGWPlayer.def:1036`). The Rust stub at `social.rs:104-108` reads the id and logs
it; no validation against `space_mgr.find_entity(id)`, no class check (is it an
`SGWPlayer`?), no spatial proximity check, no `isAlive`/`isInCombat` filter, and no
"target is not already trading" check. The deprecated Python at
`deprecated/python/cell/SGWPlayer.py:1685-1700` (the reference for what the design
once enforced) imposed: same-space, `distanceTo(self.position) >
MAX_INTERACT_DISTANCE`, `partner.isTrading()`, `partner is SGWPlayer`, and
`entityId != self.entityId`. None of those are wired in Rust yet.

**Evidence**
- Ghidra: `019d89f0` `Event_NetOut_TradeProposal` (and the `Trade@@` C++ class
  whose vftable mentions `setTradeItem`/`setTradeCash`) — the client supplies the
  EntityId from the local target selection. There is no client-side enforcement
  that the target is in range; the UI button just emits whatever target id is
  currently selected.
- `entities/defs/SGWPlayer.def:1036` — `INT32 EntityId` is the only target field;
  client-controlled.
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/player/social.rs:103-109`.

**Attack scenario**
1. Adversary selects a target via Tab and notes the entity id.
2. Adversary walks across the map, well past `MAX_INTERACT_DISTANCE`.
3. Adversary emits `tradeRequest(remote_target_id, LocalTradeProposal{...})`.
4. If the future handler accepts without range check: the trade window opens
   regardless of distance, enabling long-range dupe-coordination (one mule
   character stays in town, the other roams; items move freely).
5. Worse variants: send `tradeRequest` with the EntityId of a vendor NPC, a mob,
   a corpse, or the player's own id (self-trade — see CAT-H-08).

**Suggested remediation (one line)**
On handler implementation, gate with `space_mgr.find_entity(target).is_player() &&
in_same_space && distance <= MAX_INTERACT_DISTANCE && target.is_alive() &&
!target.is_trading() && target != self.entity_id` before allocating the
`TradeTransaction`.

**Would benefit from x64dbg trace?**
No — wire shape is fully known from the .def.

---

### CAT-H-03 — `tradeUpdateProposal` payload (item list, cash, lockState) is fully decoded client-side and currently never validated server-side; will be the primary dupe surface once handler exists

**Severity**: Critical (when handler is implemented)
**Class**: TOCTOU / missing item-ownership re-check / missing item-lock invariant
**Wire surface**: `Event_NetOut_TradeProposal` (cell method 106 — note: the wire
name is `tradeUpdateProposal` despite the `Event_NetOut_TradeProposal` class
naming)
**Demonstrable / Likely-theoretical**: Likely-theoretical (stub today)

**Trust violation**
The `LocalTradeProposal` FIXED_DICT (`entities/defs/alias.xml:363-370`) carries
`{INT32 version, ARRAY<LocalTradeItem{INT32 instanceId, INT32 slotId}> items,
INT32 cash, INT8 lockState}` — all four fields client-supplied. The Rust stub at
`social.rs:123-133` reads only the leading INT32 (the partner EntityId, before the
payload starts) and discards the rest. When implemented, the handler MUST:

1. Verify each `instanceId` resolves to an item row currently owned by the caller
   (server-side inventory query, not client claim).
2. Verify the item is not bind-on-acquire / bind-on-equip-bound to the caller
   (`ITEM_FLAG_BindOnAcquire = 4`, `ITEM_FLAG_BindOnEquip = 8` per
   `deprecated/python/Atrea/enums.py:794-795`). The Python reference at
   `cell/Trade.py:49` punts on this with a literal `TODO: Do we need a separate
   canTrade() ?`. Answer: yes — and the canonical SGW client-side flag must be
   re-checked server-side.
3. Verify each item is **not currently locked in another transaction** (vendor
   sale in flight, mail attach in flight, another trade in flight, currently
   equipped — see CAT-H-07).
4. Verify the same `instanceId` does not appear twice in the items array (the
   Python reference at `cell/Trade.py:53-55` warns and skips dupes; a Rust port
   should *reject the whole proposal*, not silently filter, because the silent
   filter means the client and server disagree on what's in the window).
5. Verify `cash` is non-negative and `cash <= self.inventory.naquadah` at this
   instant.
6. Verify `version == server_known_version + 1` — single-step monotonic.
7. Re-check (1)/(5) at lock-confirm time, because the player can drop / use / sell
   the item *between* `tradeUpdateProposal` and `tradeLockState` if the item is
   not locked at proposal time.

The crucial dupe-prevention invariant (3) — **lock items into escrow at
proposal-update time** — is absent from the Python reference and would be absent
from a naive Rust port that copies it. The deprecated reference at
`cell/Trade.py:67-74` only removes items from inventory at the *confirm* step
(after both sides lock and confirm). Between proposal and confirm, the items are
duplicatable via concurrent vendor-sell, mail-attach, drop, or use.

**Evidence**
- `entities/defs/alias.xml:355-370` — `LocalTradeProposal` FIXED_DICT contents,
  every field client-supplied.
- `entities/defs/SGWPlayer.def:1053-1057` — Exposed RPC signature.
- Ghidra: `019d89f0` `Event_NetOut_TradeProposal`; the `Trade@@` C++ class on the
  client (mangled type `.?AVTradeState@@` at `01de9594` and the UI verbs
  `setTradeItem`/`removeTradeItem` at `01952a3c`/`019529d0`) constructs the
  `items` array from the local inventory model — fully attacker-modifiable.
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/player/social.rs:123-133`.

**Attack scenario** (assuming a naive future implementation that copies the Python
reference's lock-at-confirm semantics)
1. A and B initiate trade. A puts InstanceId I (a rare item) into the proposal
   and locks. Server records: A's window contains I.
2. A simultaneously opens vendor UI and emits `SellItems(I, …)`. Vendor handler
   (if it doesn't query the trade-lock table — there isn't one yet) sells I and
   credits A with naquadah.
3. A confirms the trade. Server attempts `inventory.removeItem(I, quantity, True)`
   — fails because I no longer exists.
4. If the server bails out gracefully: B does not receive I, trade voids — OK.
5. If the server proceeds with the partial transfer of remaining items + cash but
   does not re-validate (this is the dupe-shape pattern history shows in nearly
   every "missing item lock" trade exploit): B's inventory may be credited with a
   phantom InstanceId, or A's removed-items list silently shrinks while B's
   credited-items list does not — duplicating I.

The shape is similar to the **stack duplication via disconnect-timing** trap from
the SGW exploit notes (in `social-systems-engineer`'s domain). Without an explicit
escrow / item-lock table, this is unavoidable.

**Suggested remediation (one line)**
On handler implementation, lock each `instanceId` into a server-side escrow table
at `tradeUpdateProposal` time (not at confirm time), and reject any concurrent
vendor / mail / use / drop / move on a locked instanceId.

**Would benefit from x64dbg trace?**
Yes — confirm byte-exact `LocalTradeProposal` framing (ARRAY-of-FIXED_DICT
length-prefix shape) before authoring the decoder, so the Rust handler doesn't
mis-frame the array bound (a length-confusion bug could itself become a dupe
vector).

---

### CAT-H-04 — `tradeLockState` `localVersionId` / `remoteVersionId` enforcement absent in stub; partial-lock acceptance is the classic dupe path

**Severity**: High (when handler is implemented)
**Class**: Version-counter desync / replay
**Wire surface**: `Event_NetOut_TradeLockState` (cell method 107)
**Demonstrable / Likely-theoretical**: Likely-theoretical (stub today)

**Trust violation**
`tradeLockState(INT32 LocalVersionId, INT32 RemoteVersionId, INT8 LockState)` —
the stub at `social.rs:135-149` reads all three but does nothing. When
implemented, the invariant (the only invariant that keeps "lock state" honest
relative to "proposal state") is:

- `LocalVersionId == server.knownLocalVersion(self)`. If they don't match, the
  client is trying to lock a stale proposal — reject and reset the partner's
  lock-state to None (the Python reference at `cell/Trade.py:196-199` warns and
  returns False; a Rust port must do the same plus reset partner lock).
- `RemoteVersionId == server.knownRemoteVersion(partner)`. The client must
  acknowledge the partner's latest proposal version. The Python reference at
  `cell/Trade.py:203-204` *silently downgrades* lockState to None in this case —
  that's a UX leniency but it's also a partial-lock attack: a client can emit
  `tradeLockState(localVersion=N, remoteVersion=N-1, lockState=Locked)` to assert
  Locked on a stale partner proposal; the silent-downgrade behavior means the
  server transitions to a not-quite-locked state that's still observable on the
  partner side. Reject, don't downgrade.
- `LockState` must be a valid `ETradeLockState` enum value (0=None, 1=Locked,
  2=LockedAndConfirmed; `entities/defs/enumerations.xml:1784-1791`). The Python
  reference rejects out-of-range values; the Rust stub does no enum-bound check
  and would happily forward any signed 8-bit value into the state machine.

**Evidence**
- `entities/defs/SGWPlayer.def:1066-1072` — Exposed RPC signature, all three INT32
  / INT8 fields client-supplied.
- `entities/defs/enumerations.xml:1784-1791` — three legal `ETradeLockState`
  values.
- Ghidra: `01944d80` `#ferror in function 'setTradeLockState'`, `01952944`
  `setTradeLockState` — the client UI verb that emits this RPC. The state machine
  in the client (lock button → `setTradeLockState`) trusts whatever the server
  echoes back via `onTradeState`, so a divergence here is "client-visible weird
  UI" not "client-side rejection".
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/player/social.rs:135-149`.

**Attack scenario** (assuming naive future implementation)
1. A puts items+cash into the proposal, server-known LocalVersion = 5.
2. A emits `tradeLockState(LocalVersion=5, RemoteVersion=3, LockState=2/Confirmed)`
   while B's actual current proposal is at version 7.
3. Server, if it silently downgrades the lock as the Python reference does,
   records "A is Confirmed" but with stale remote view.
4. B (the partner), seeing A as locked-and-confirmed, may also confirm. If the
   server's `confirm()` step (`cell/Trade.py:231-273`) does not re-check that
   `proposal.lockState == LockedAndConfirmed && partner.lockState ==
   LockedAndConfirmed && both saw each other's latest version`, the transaction
   fires on B's stale view.

**Suggested remediation (one line)**
On handler implementation, reject (not downgrade) on any version mismatch, and at
`confirm()` re-verify both proposals' `lockState == LockedAndConfirmed && both
mutual versions are current` before any inventory mutation.

**Would benefit from x64dbg trace?**
No — wire shape and enum values are fully known.

---

### CAT-H-05 — No transactional rollback exists for trade confirm; partial-transfer dupe is unavoidable without a single-commit invariant

**Severity**: Critical (when handler is implemented)
**Class**: Atomicity violation / partial-commit dupe
**Wire surface**: `Event_NetOut_TradeLockState` (the confirm transition fires
inventory mutation)
**Demonstrable / Likely-theoretical**: Likely-theoretical (no confirm handler
exists today)

**Trust violation**
The Python reference at `deprecated/python/cell/Trade.py:265-273` performs four
non-atomic steps:

```python
p1.removeItems()           # 1. A's items leave A's inventory
p2.removeItems()           # 2. B's items leave B's inventory
p1.player.inventory.addItems(p2Items)   # 3. A gets B's items
p2.player.inventory.addItems(p1Items)   # 4. B gets A's items
```

If any step after (1) raises, A's items are gone and B never received them — but
also B's items may have been removed (between 2 and 3) and never delivered. A
modified server, or any panic / DB disconnect / process kill between (1) and (4),
produces a dupe (items credited to neither party but possibly already removed)
or a loss (items removed from A but never added to B). The naquadah (cash) move
is not even shown — the Python reference does it implicitly via `removeItems` of
the cash quantity (since cash is fungible) but with the same atomicity gap.

When the Rust handler is written, this must be a single DB transaction with
rollback, executed against the inventory tables in
`db/sgw/` (the player-inventory-item table layouts; see `db/database.sql`). The
deprecated reference is **not safe to port directly** — it was acknowledged
unsafe in the original codebase (see also `deprecated/python/cell/Trade.py`
absence of any `begin`/`commit`/`rollback`).

**Evidence**
- `deprecated/python/cell/Trade.py:265-273` — the four-step non-atomic confirm
  sequence (reference only; the *truth* is the absence of any Rust impl).
- No transactional commit boundary in `crates/services/src/cell/cell_methods/player/social.rs`
  because no handler exists.
- Ghidra: the client side does not arbitrate the confirm; it simply emits
  `tradeLockState(…, LockState=2 LockedAndConfirmed)` and waits for `onTradeResults`
  (method 145). The server is sole arbiter of the commit boundary.

**Attack scenario**
1. A and B fully fill the trade and both lock-and-confirm.
2. Server begins the confirm sequence: A's items removed.
3. Server is killed (operator pulls a kill switch, OOM, network partition
   between cell and base, anything that interrupts the four-step sequence).
4. On recovery, A's items are gone from A's inventory and B never received them.
5. Variant: with appropriate timing or a chunked DB write that partially commits,
   the items may be present in both A's and B's inventory — a clean dupe.

**Suggested remediation (one line)**
Implement confirm as a single `BEGIN/COMMIT` with all four mutations (A's items
out, B's items in, B's items out, A's items in, both naquadah deltas) in one
transaction, with `ROLLBACK` on any error and *no* partial-success path; pair
with an outbox so `onTradeResults` is sent only after commit.

**Would benefit from x64dbg trace?**
No — this is purely a server-side invariant.

---

### CAT-H-06 — Disconnect during trade has no rollback; items in escrow are lost or duplicated depending on which side dropped

**Severity**: Critical (when handler is implemented)
**Class**: Disconnect-timing dupe (canonical MMO dupe shape)
**Wire surface**: implicit — Mercury connection drop after a `tradeLockState`
LockedAndConfirmed but before the server commits both sides
**Demonstrable / Likely-theoretical**: Likely-theoretical (no handler; the named
exploit pattern is from the SGW agent memory)

**Trust violation**
This is the named **stack duplication via disconnect-timing** pattern from the
agent memory: "During trade, a client that disconnects between the
item-transfer commit and the counterparty-credit commit can cause one side to
keep the item and the other to gain it. Trade flows must be transactional
end-to-end with a rollback on disconnect."

The Rust server has no disconnect→trade-cancel hook. There is no
`TradeTransaction` to be canceled, but more importantly, when the handler is
written, the disconnect path in `crates/services/src/base/` and
`crates/services/src/cell/` must call into the trade module to fail-fast any
in-progress trade, return locked items to their original owner, and emit a
synthetic `tradeRequestCancel` to the surviving partner so their UI clears.

There is currently no test surface (no live-DB chain-replay test, no Mercury
session test) covering "A locks, B locks, A disconnects" — because there's no
handler to test.

**Evidence**
- `crates/services/src/cell/cell_methods/player/social.rs:103-149` — stub, no
  disconnect interaction.
- No grep matches for `trade_transaction|TradeTransaction|trade_session` outside
  the deprecated Python reference, confirming no escrow object exists on which a
  disconnect cleanup could fire.
- Agent memory: the SGW exploit-pattern catalog explicitly names this shape.

**Attack scenario**
1. A and B fully fill the trade. Both lock.
2. A confirms (`tradeLockState(..., LockedAndConfirmed)`).
3. Server transitions to commit. B has not yet sent the second confirm.
4. A force-kills the network (pulls cable, kill SGW.exe, exploits a `LogOff`
   timing — see CAT-A for `LogOff`/Disconnect interaction surface).
5. If the server commits A's removeItems but the disconnect aborts before B's
   addItems: A loses items, B never gets them. *Loss variant.*
6. If the server emits `onTradeResults(Completed)` to B optimistically and a
   queued / outbox-style replay re-fires the commit on reconnect (see also the
   `Mercury session replay` pattern in `docs/architecture/`): A's items are
   added to B's inventory while A's session, on reconnect, re-claims them via
   the outbox. *Dupe variant.*

**Suggested remediation (one line)**
On handler implementation, wire `cell.on_player_disconnect(entity_id)` into a
trade-cancel callback that locks the trade transaction with a server-side
mutex, returns escrow items, and emits a one-shot `tradeRequestCancel`-equivalent
to the partner — ensure the outbox **never** re-fires a `tradeLockState` or
`onTradeResults` after the disconnect-cancel.

**Would benefit from x64dbg trace?**
Yes — observe the client's behavior on receiving `onTradeResults(Cancelled)`
after a partial-confirm, to confirm the client cleans up its local view (no
ghost items shown / no UI hang).

---

### CAT-H-07 — Items in the trade window are not locked against concurrent inventory operations (use / drop / sell / move / mail-attach)

**Severity**: Critical (when handler is implemented)
**Class**: TOCTOU dupe via concurrent inventory mutation
**Wire surface**: any concurrent `MoveItem`, `UseItem`, `RemoveItem`, `LootItem`,
`SellItems`, `MailAttachItem`, `RequestAmmoChange`, etc., issued while the same
`instanceId` is present in the trade window
**Demonstrable / Likely-theoretical**: Likely-theoretical (stub today)

**Trust violation**
A trade implementation that does not lock the item rows it references in the
proposal window opens every other inventory mutation handler as a dupe surface.
The deprecated Python at `cell/Trade.py:67-74` calls `inventory.removeItem` only
at `confirm()` time — leaving an arbitrarily long window during which:

- `SellItems` (CAT-E) could sell the proposed item.
- `UseItem` (CAT-D) could consume a stackable proposed item.
- `MoveItem` (CAT-D) could move it (and depending on bandolier semantics, also
  manipulate the ammo row — see the `ammo_dup_via_same_type_swap_toctou` pattern
  in the agent memory).
- `MailSendMessage` with item attachment (CAT-G) could attach and ship the same
  instanceId.

A robust Rust implementation must add an "item-lock" guard that lives on the
`InventoryItem` row (or a side table keyed on `item_id` — never `type_id` per the
`bandolier_ammo_key_by_item_id_not_type_id` invariant) and is checked by **every**
mutation path. The current Rust impl has no such table.

**Evidence**
- No matches for `item_lock|inventory_lock|escrow|trade_lock` in `crates/services/src/`.
- Cross-ref to siblings' handlers (for fix author):
  - `crates/services/src/cell/cell_methods/player/social.rs:123-133` (the
    tradeUpdateProposal stub that should be locking).
  - `crates/services/src/base/world_entry/methods/vendor/` (sells).
  - `crates/services/src/cell/cell_methods/player/inventory.rs` (move / use /
    remove — exact location varies; the audit cross-referenced via the file
    inventory map at top of CAT-D-inventory).
- Ghidra: client-side has no awareness of locking — the inventory UI happily
  allows MoveItem on an item that is also in the trade window. The race is
  unmistakable.

**Attack scenario**
1. A puts InstanceId I (stack of 99 ammo) into the trade window.
2. A simultaneously emits `UseItem(I)` (it's consumed; `RequestAmmoChange`-style
   load).
3. A emits `tradeLockState(..., Confirmed)`. B confirms.
4. Server confirm: `inventory.removeItem(I, 99, True)` — fails because the stack
   is now 98 or 0.
5. Naive future handler proceeds with partial transfer: B gains 99 ammo, A only
   lost 98. Dupe.
6. Variant with two items: I1 (proposed) and I2 (used). A successfully tricks
   the partial-failure path into transferring I1 while keeping it. Dupe.

**Suggested remediation (one line)**
Implement an item-lock table keyed by `instance_id` (not `type_id`), checked by
every inventory-mutation handler; on lock conflict, the *competing* operation
fails fast with a typed error (not the trade).

**Would benefit from x64dbg trace?**
No.

---

### CAT-H-08 — Self-trade (player trading with own EntityId) is not explicitly rejected; dupe surface if any handler short-circuits the partner lookup

**Severity**: High (when handler is implemented)
**Class**: Self-target dupe
**Wire surface**: `Event_NetOut_TradeRequest`, `Event_NetOut_TradeProposal`
**Demonstrable / Likely-theoretical**: Likely-theoretical (stub today)

**Trust violation**
`tradeRequest(EntityId, ...)` takes the partner's entity id. The Rust stub at
`social.rs:103-109` does no `target_entity_id != self_entity_id` check. The
deprecated Python at `cell/SGWPlayer.py:1685-1687` does reject self-trade, but
the Rust port may forget. The dupe shape: if the server's "find partner" lookup
returns `self`, and the confirm step does
`p1.removeItems(); p1.addItems(p1Items)`, then any non-atomic ordering between
remove and add can dupe (or worse, the lookup-returns-self case may make
`p1Items == p2Items` so adding the same set back after removing produces an
identity — but if there's any rounding / quantity-cap / per-type-stack overflow
in `addItems` the second add can silently truncate while the cash credit went
through).

**Evidence**
- `entities/defs/SGWPlayer.def:1036` — INT32 EntityId, no constraint.
- `crates/services/src/cell/cell_methods/player/social.rs:104-109` — no self-id
  check.
- `deprecated/python/cell/SGWPlayer.py:1685-1687` — the prior-art rejection
  (informational).

**Attack scenario**
1. Adversary patches the client (or scripts a raw Mercury emitter) to send
   `tradeRequest(self.entity_id, LocalTradeProposal{items:[I1, I2], cash:N})`.
2. Future naive handler creates a `TradeTransaction(self, self)`, allocates
   proposals for both sides keyed by `entity_id` — and in the
   `TradeTransaction.proposals` dict (Python reference at
   `cell/Trade.py:122-126`) the dict literal collapses two identical keys to
   one entry. The confirm step then operates on a degenerate transaction
   (likely panics or silently completes with no transfer).
3. Variants depending on Rust implementation: any path that increments cash
   twice, any path that does
   `inventory.add(other.items); inventory.remove(self.items)` in either order
   when `self == other` could dupe or lose items.

**Suggested remediation (one line)**
First-line check in `tradeRequest` and `tradeUpdateProposal` handlers:
`if target_entity_id == self_entity_id { reject; return }`.

**Would benefit from x64dbg trace?**
No.

---

### CAT-H-09 — Bind-on-acquire / bind-on-equip items can be placed in the trade window; `canTrade` invariant is missing from both reference and Rust

**Severity**: Medium (when handler is implemented — bypasses an item-progression
constraint, not a dupe per se)
**Class**: Missing item-flag enforcement
**Wire surface**: `Event_NetOut_TradeProposal`
**Demonstrable / Likely-theoretical**: Likely-theoretical (stub today)

**Trust violation**
SGW items carry `ITEM_FLAG_BindOnAcquire = 4` and `ITEM_FLAG_BindOnEquip = 8`
(`deprecated/python/Atrea/enums.py:794-795`). These flags are the SGW analog of
WoW's "soulbound" — the item should not be transferable once it's bound to the
acquirer. The deprecated Python `cell/Trade.py:48-51` punts on this with:

```python
# TODO: Do we need a separate canTrade() ?
if not item.canSell():
    warn(..., "Sent unsellable item in proposal")
    continue
```

`canSell()` is not the same as `canTrade()`. An item may be sellable to a vendor
(it has a sale price set) but bound to the player (not tradable). Conversely,
some items may be tradable but not sellable. The conflation in the reference is
a bug that would carry into a naive Rust port. The Rust stub does no flag check
at all (no decode of `InvItem` flags from the proposal payload, since the
payload is not decoded).

**Evidence**
- `deprecated/python/Atrea/enums.py:794-795` — bind-flag definitions.
- `deprecated/python/cell/Trade.py:48-51` — the explicit `TODO` in the reference.
- No matches for `bind|canTrade|notrade|soulbound` in `crates/services/`.

**Attack scenario**
1. A has a high-end raid item with `ITEM_FLAG_BindOnAcquire`.
2. A puts it in the trade window with player B.
3. Naive handler (copying `canSell()` semantics from the Python reference): the
   item passes the sellable check, gets included, transfers to B.
4. Observable: a bind-on-acquire item now belongs to B. Progression bypass.
5. Compounding factor: if B equips and rebinds it, the audit trail of who
   originally acquired it is lost.

**Suggested remediation (one line)**
On handler implementation, add a real `can_trade(item)` predicate that returns
false for any `ITEM_FLAG_BindOnAcquire`-set item and any
`ITEM_FLAG_BindOnEquip`-set item that has actually been equipped; reject the
*entire* proposal on first violation (do not silently filter).

**Would benefit from x64dbg trace?**
No — flag enforcement is server-only.

---

### CAT-H-10 — `tradeUpdateProposal` "no active trade session" workaround in the deprecated reference opens an out-of-band escrow vector if copied

**Severity**: Medium (only if a Rust port copies the Python workaround verbatim)
**Class**: State-machine bypass
**Wire surface**: `Event_NetOut_TradeProposal`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The deprecated Python at `cell/SGWPlayer.py:1785-1790` contains:

```python
# WORKAROUND: We need to start a trade session here as the 0.8384 QA client
# doesn't send a tradeRequest RPC at all
if not self.isTrading():
    if not self.beginTrading(entityId):
        self.client.onTradeResults(entityId, Atrea.enums.Cancelled)
        return
```

This means `tradeUpdateProposal` can implicitly *start* a trade with an arbitrary
EntityId — bypassing the more-restrictive `tradeRequest` flow (which in some
implementations should require partner accept, distance check at request time,
etc.). The QA client behavior is a workaround for a missing client-side packet,
but as a server invariant it's a bypass: an adversary that skips `tradeRequest`
entirely and goes straight to `tradeUpdateProposal` is implicitly granted a
trade session.

If a Rust port copies this workaround (which is reasonable on the assumption that
the 0.8384 QA client we're targeting still doesn't send `tradeRequest`), then
every range / alive / partner-not-already-trading check that lives in
`beginTrading` must be invoked here too, and the partner must be notified of the
implicit trade-start (otherwise the partner sees an `onTradeState` for a trade
they never agreed to — a UI grief vector at minimum, a phishing surface at
worst, e.g. "click here to confirm" social-engineering against a partner who
didn't initiate).

**Evidence**
- `deprecated/python/cell/SGWPlayer.py:1785-1790` — the explicit workaround
  comment, with reference to the 0.8384 QA client.
- The client behavioral log dir
  `C:\Users\Steve\source\projects\sgw\Stargate Worlds-QA\Working\binaries\SGWDebugLog.log`
  contains no trade-flow evidence (UI not exercised in the captured run).
- Ghidra: confirm the QA build does/doesn't emit `tradeRequest` is needed via
  x64dbg trace; the static decompile is consistent with the client *being able to*
  emit it (the EventHandler is wired) but not whether the actual UI path does.

**Attack scenario**
1. Adversary scripts: `tradeUpdateProposal(victim_id, {items:[I1], cash:0, version:1, lockState:0})`.
2. Naive Rust port copies the Python workaround: `beginTrading(victim_id)` is
   invoked, victim receives an `onTradeState` for a trade they did not request.
3. Victim, if they accidentally click "lock" while the window has their attention
   (which a clever adversary could time with another UI event), confirms.
4. Adversary's prior `tradeLockState(..., Confirmed)` was already queued. Commit
   fires.

**Suggested remediation (one line)**
On handler implementation, if the QA client really doesn't send `tradeRequest`,
make `tradeUpdateProposal`'s implicit-begin go through the **exact same**
validation gates as `tradeRequest` (distance, alive, is-player, not-already-trading,
not-self), and require partner-side `onTradeRequestFromEntity` ack before the
window opens on the victim's screen.

**Would benefit from x64dbg trace?**
Yes — first, confirm the 0.8384 QA client's actual emit sequence on "Trade"
button-press, so we know whether to gate or to drop the workaround.

---

## Not Filed

- **"Trade-from-banker / vendor / mailbox window dupe"** — the
  `EInteractionType.Trade = 10` value in `entities/defs/enumerations.xml:855`
  is only the dialog-type token, not a code path that bypasses player-to-player
  trade. Not filed because the wire surface is the same four RPCs already
  covered above.
- **"`Event_NetIn_TradeState` / `Event_NetIn_TradeResults` server→client
  spoofability"** — these are inbound on the client, not the server. Out of
  scope for a server-authority audit (the trust direction is wrong — client
  trusts the server here, and a malicious server isn't the threat model).
- **"`tradeRequestFromEntity` / `updateTradeState` / `updateTradeLockState` /
  `tradeCancel` as base-entity RPCs"** — these (`SGWPlayer.def:1041-1064,
  1080-1089`) lack the `<Exposed/>` flag, so they are server-internal
  base-to-cell or cell-to-cell RPCs, not client-callable. They are not part of
  the inbound wire surface and so cannot carry a client trust violation.
- **"Currency overflow on naquadah field in `LocalTradeProposal.cash`"** — the
  INT32 cash field could be MAX_INT and a naive add to partner's naquadah could
  overflow; not filed as a distinct finding because it folds into CAT-H-03's
  enumerated server-side re-checks (the "cash <= self.inventory.naquadah at
  this instant" check covers the upper bound; overflow on the partner side is
  a CAT-D inventory concern, not a CAT-H trade concern). Cross-ref to
  CAT-D-inventory for currency-overflow handling.
- **"Trade window opens cross-space (different cells / different worlds)"** —
  same-space enforcement is part of CAT-H-02; not split out separately.
- **"`tradeRequestCancel(target_id)` where target is not the actual partner"** —
  the stub at `social.rs:111-121` reads target_id but the Python reference at
  `cell/SGWPlayer.py:1768-1774` doesn't even use it (`self.cancelTrading(entityId)`
  cancels whatever trade `self` is in). Not filed as distinct — folds into the
  general "use server-tracked partner id, not client-supplied" idiom in
  CAT-H-02.
- **"Replay attack on `tradeLockState`"** — the per-tick authenticate-token and
  the 512-entry dedup hash from spec §1.7 are framing-layer concerns handled
  upstream of the cell dispatcher. Not filed as a CAT-H finding because the
  dedup is not trade-specific; the trade handler relies on the framing layer
  for replay rejection. (The implementer should still verify the framing-layer
  check is applied to cell-method packets, not just base-method packets.)
- **"Stack-size manipulation in `LocalTradeItem`"** — the `LocalTradeItem`
  FIXED_DICT (`alias.xml:356-361`) carries only `instanceId` and `slotId`, *not*
  quantity. Quantity is looked up server-side from the actual item row. So this
  is not a trust-violation surface — the dupe shape would have to come from a
  TOCTOU on the item row (covered by CAT-H-07), not from a client-asserted
  quantity. Filed as "not a distinct finding" precisely *because* the wire is
  correctly shaped here.
