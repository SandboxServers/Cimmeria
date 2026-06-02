# CAT-E — Vendor (PurchaseItems / SellItems / BuybackItems / RepairItems / RechargeItems)

**Overall trust posture: GOOD with two real exploit-shaped holes and several
softer issues.** The vendor stack consistently treats the client's wire payload
as a request, not a result — for every flow the server (a) looks up the vendor
template via the player's server-side `vendor_entity` (not a client field), (b)
recomputes price authoritatively from `resources.item_list_items.naquadah` (and
for repair, scales by current `durability`), (c) wraps the mutation +
cash-balance update in a Postgres transaction with `FOR UPDATE` locks, and (d)
balances are read with `FOR UPDATE` before the debit. The known holes:

- **CAT-E-01** (High) — The cell dispatcher routes `REPAIR_ITEMS` and
  `RECHARGE_ITEMS` to a "free" code path whenever the trailing
  `vendor_template_id` is absent from the client payload. The free path
  performs durability=100 / charges=full UPDATEs with **no payment, no vendor
  template check**, and no equivalence to the paid handler. The PURCHASE / SELL
  / BUYBACK arms in the same dispatcher reject a missing template id; the
  REPAIR / RECHARGE arms do not. A client that simply omits the trailing 4
  bytes on its `repairItems` packet gets free durability restore on every
  carried item. This is a wire-shape inversion: the *absence* of a client
  field selects the free flow.
- **CAT-E-02** (Medium) — `player.vendor_entity` is set by the Interact path
  and **never cleared** — not on cell→base disconnect, not on
  `OpenVendorStore`-close, not on death, not on instance/zone transfer. There
  is also no distance / proximity / line-of-sight check between the player
  entity and the vendor entity at op time. A player who once interacted with
  vendor A can submit purchase/sell/buyback/repair packets while standing
  arbitrarily far away (other side of the map, in another instance if the
  entity persists, after combat / death). The current `vendor_context`
  validation only rejects when the vendor entity has despawned.

The other findings are: a freely-readable `repair_ratio: f32` arrives on the
single-item `RepairItem` path (currently UNIMPLEMENTED in the cell handler —
flagged so it doesn't ship), buyback rows are not bound to the vendor that
sold them and have no expiration window, and the buyback unit price is stored
in the inventory row's `flags` column where it would survive any future bug
that lets a client edit `flags` via a different mutation path.

---

### CAT-E-01 — Missing trailing `vendor_template_id` selects free repair / free recharge

**Severity**: High
**Class**: Wire-shape inversion / missing-field bypass — server-authority bypass that grants a free service
**Wire surface**: `Event_NetOut_RepairItems`, `Event_NetOut_RechargeItems`
**Demonstrable / Likely-theoretical**: Likely-exploitable theoretical (needs live debugger to confirm at triage time)

**Trust violation**
The wire payload for cell methods 78–82 (PURCHASE_ITEMS … RECHARGE_ITEMS) is:
`u32 count, (i32 item_id, i32 quantity)[count], [i32 trailing_template_id]`.
The trailing `template_id` is parsed via `read_trailing_template_id`, which
returns `Some(_)` if 4 trailing bytes remain and `None` otherwise. In the
cell dispatcher, PURCHASE / SELL / BUYBACK reject `None` with a "missing
vendor_template_id" warn and `return true` — they require the field. REPAIR
and RECHARGE accept `None` and pass it through to the base handler. The
base handler `handle_repair_inventory_items` / `handle_recharge_inventory_items`
treats `Some(id)` as the paid path (cost computed from
`resources.item_list_items.naquadah` and debited from `sgw_player.naquadah`)
and `None` as a **free path** that simply runs `UPDATE sgw_inventory SET
durability = 100 …` / `charges = ri.charges …` against every owned item in
the request list. The free path has no template lookup, no `naquadah`
balance check, and no `tx.begin()` cost debit. A modified client (or
replayed-with-truncation client) that sends a `repairItems` packet without
the trailing 4 bytes restores every item's durability to 100 for free; the
same for `rechargeItems`.

**Evidence**
- Ghidra: string `0x019c2cf8` = `"repairItems"`, registered as wire method
  name in `register_NetOut_onStrikeTeamResponse` at xref `00dc290d`; the
  EventHandler constructor for `Event_NetOut_RepairItems` lives at
  `0x00d6cfa0` (called from the registration at `00dc295b`). RTTI string
  `.?AVEvent_NetOut_RepairItems@@` at `0x01e2b984`. The client emits a
  message whose typed payload is a `(item_id, quantity)[]` array plus —
  per the matching server parser — an optional trailing template id. Whether
  this trailing field is **always** present on the client emit needs an
  x64dbg trace of the actual emit site (the registration only proves the
  *registration*; the constructor of the typed payload was not traced in
  this audit) — flagging as likely-theoretical.
- Client behavioral log: n/a
- Cross-ref to Rust handler (for the fix author, NOT as truth):
  `crates/services/src/cell/cell_methods/player/vendor.rs:189-256` (the
  dispatcher arm that yields `validated_template_id = None` for REPAIR /
  RECHARGE but rejects `None` for PURCHASE / SELL / BUYBACK), and
  `crates/services/src/base/world_entry/methods/vendor/repair.rs:122-135` +
  `crates/services/src/base/world_entry/methods/vendor/recharge.rs:32-45`
  (the wrapper functions that branch to free vs. paid on `Option<i32>`).
  Items-systems-advisor: the vendor state machine should make the free
  repair path reachable only from a "free-repair NPC" (e.g., training
  area / mission reward), not from any vendor's `repairItems` invocation.

**Attack scenario**
1. Player interacts with a paid repair vendor (`OpenVendorStore` runs;
   `player.vendor_entity` is set; UI loads the vendor's `repair_item_list`).
2. Adversary sends `cellMethod 81` (REPAIR_ITEMS) with payload
   `count=N, (item_id, 0) × N` and **omits the trailing 4-byte template id**.
3. Cell dispatcher `vendor::dispatch` reads `validated_template_id = None`
   (the `Some(client_id)` arm requires a non-empty trailing read; absent
   bytes drop through to the `None => None` arm), builds
   `CellToBaseMsg::RepairInventoryItems { vendor_template_id: None, .. }`.
4. Base handler `handle_repair_inventory_items(None)` falls through past
   the `if let Some(...) = vendor_template_id` arm and executes
   `UPDATE sgw_inventory SET durability = 100 WHERE character_id = $1 AND
   item_id = ANY($2) AND container_id = ANY(VENDOR_FILTER_BAGS) AND
   stack_size = 1 AND durability < 100` — no payment.
5. Observable effect on the server: every requested item's durability
   restored to 100, no `sgw_player.naquadah` row mutated, no
   `onCashChanged` packet emitted to the witness. The same shape works for
   RECHARGE_ITEMS — restoring `charges` for free. Repeatable indefinitely.

**Suggested remediation (one line)**
Reject `None` `validated_template_id` for REPAIR_ITEMS and RECHARGE_ITEMS at
the cell dispatcher (mirror the PURCHASE / SELL / BUYBACK arms), and route the
free-repair / free-recharge code paths through a distinct cell method index
that is only invoked by trusted server-side flows (mission-grant, GM, etc.) —
not by `repairItems` / `rechargeItems` from the wire.

**Would benefit from x64dbg trace?**
Yes — confirm at the actual `Event_NetOut_RepairItems` / `RechargeItems`
emit site (the typed payload's serializer) whether the trailing
`template_id` is encoded in **all** code paths, including the
`free repair from non-vendor context` UI path. If the client always packs
the trailing id, the exploit requires a modified client; if it ever omits
the trailing id (e.g., from a self-repair hotkey), the exploit is reachable
from an unmodified client.

---

### CAT-E-02 — `vendor_entity` is never cleared; no proximity / line-of-sight check

**Severity**: Medium
**Class**: Stale session / missing proximity check — server-authority gap in vendor session lifecycle
**Wire surface**: All five — `Event_NetOut_PurchaseItems`, `SellItems`, `BuybackItems`, `RepairItems`, `RechargeItems`
**Demonstrable / Likely-theoretical**: Likely-exploitable theoretical (needs live debugger to confirm at triage time)

**Trust violation**
The cell-side `vendor_context` looks up the vendor session via
`player.vendor_entity` (a server-side `Option<u32>` set by the
Interact→vendor path). It is **set** at `crates/services/src/cell/interactions/vendor.rs:20`
and is never written elsewhere. No code clears it on disconnect, on
`CancelMovie`, on death, on respawn, on instance / world transfer, or on a
"close vendor store" UI message (there is no such message). The cell-side
op also performs no proximity check between the player's current position
and the vendor entity's position, and no line-of-sight or "is the player
even in the same space" check. The only validation against arbitrary
operation is `validate_template_id`, which catches the case where the
vendor entity despawned (template_id reads as `None`) — but for any
persisting vendor entity, the player can purchase / sell / buyback / repair
at any range as long as they ever interacted with that vendor.

**Evidence**
- Ghidra: n/a (this is a server-side stateful gap; the client's emit is
  uncontrolled but the gating is meant to be server-authoritative)
- Client behavioral log: n/a
- Cross-ref to Rust handler (for the fix author, NOT as truth):
  `crates/services/src/cell/interactions/vendor.rs:18-21` (the only
  assignment site of `vendor_entity`); `crates/services/src/cell/cell_methods/player/vendor.rs:75-88`
  (`vendor_context` reads it without proximity check); the missing
  inverse is anywhere that should clear it. Items-systems-advisor: the
  vendor state machine in SGW's spec almost certainly requires
  player-in-range and a server-side "store open" turn-of-state that
  decays on disconnect / respawn — the current implementation models
  the open state as a never-decaying pointer.

**Attack scenario**
1. Player interacts with vendor A (cell-side `vendor_entity` set, vendor
   open UI loads on client).
2. Player walks 1 km away (or dies and respawns, or zones, or
   disconnects+reconnects if `vendor_entity` survives the reload — flag
   for verification).
3. Adversary sends a well-formed `purchaseItems` packet with the
   correct vendor A `template_id`.
4. Server-side `vendor_context` returns `Some(VendorSession { ..,
   server_template_id: Some(A) })` because vendor A is still alive in
   the cell. `validate_template_id` accepts.
5. Observable effect on the server: purchase / sell / repair runs at
   arbitrary distance from the vendor entity. In PvP / instance content
   this lets a player buy ammo / repair mid-fight without seeking
   shelter at a vendor.

**Suggested remediation (one line)**
Clear `player.vendor_entity` on (a) cell-side disconnect, (b) any other
Interact target, (c) respawn / death, and add a server-side range check
between the player's position and the vendor entity's position at the top
of `vendor_context` (and a same-space check if vendor entities can leak
across spaces).

**Would benefit from x64dbg trace?**
Yes — confirm whether the live client's UI ever emits a vendor packet
after the player has moved out of vendor-UI range, to distinguish
"adversary must use a modified client" from "the legitimate client emits
this naturally on UI lag."

---

### CAT-E-03 — `RepairItem` (singular, client-supplied `repair_ratio: f32`) is wired but UNIMPLEMENTED on the cell side

**Severity**: Low (latent — not currently reachable)
**Class**: Latent client-trust seam — would-be exploit awaiting wiring
**Wire surface**: `Event_NetOut_RepairItem` (singular; per surface inventory CAT-D)
**Demonstrable / Likely-theoretical**: Likely-exploitable theoretical, currently latent

**Trust violation**
`crates/services/src/cell/cell_methods/inventory/item_ops.rs:176-186`
parses a `repairItemRequest` packet as `(i32 item_id, f32 repair_ratio)`
and logs `UNIMPLEMENTED`. The matching base-side handler
`handle_repair_inventory_item` (called via `CellToBaseMsg::RepairInventoryItem`)
*does* exist and accepts the client-supplied `repair_ratio: f32` as the
authoritative repair amount: it clamps to `[0.0, 1.0]`, rounds to integer
points, and adds to `durability` — **with no naquadah debit at all**. If
the cell-side wiring is ever completed (the message routing from
`repairItemRequest` → `CellToBaseMsg::RepairInventoryItem` is currently
missing), the handler would let any client supply
`repair_ratio = 1.0` and fully repair any owned item without paying.

**Evidence**
- Ghidra: not investigated (latent — no `Event_NetOut_RepairItem` xref
  trace performed because the cell handler is a no-op today). Flag for
  re-audit if a future PR wires the cell→base path.
- Client behavioral log: n/a
- Cross-ref to Rust handler (for the fix author, NOT as truth):
  `crates/services/src/cell/cell_methods/inventory/item_ops.rs:184`
  (`UNIMPLEMENTED: repairItemRequest`) and
  `crates/services/src/base/world_entry/methods/vendor/repair.rs:22-104`
  (the function that *would* be called).

**Attack scenario**
1. (Hypothetical, gated by future wiring.) Adversary sends
   `repairItemRequest(item_id=X, repair_ratio=1.0)`.
2. Cell-side handler forwards to base as
   `CellToBaseMsg::RepairInventoryItem { repair_ratio: 1.0, .. }`.
3. Base handler runs `UPDATE sgw_inventory SET durability = LEAST(100,
   durability + round(1.0 * 100))` — no cost, no balance check.

**Suggested remediation (one line)**
Before wiring the cell-side path, make `handle_repair_inventory_item` debit
naquadah computed from `(repair_ratio, item template's repair cost)` inside
a transaction, OR drop `repair_ratio` from the wire and have the server
compute it (the client should not assert how much it pays for).

**Would benefit from x64dbg trace?**
No — the seam is on the server side; the issue is the *handler shape*, not
the client emit.

---

### CAT-E-04 — Buyback price stored in `sgw_inventory.flags`; recoverable independent of vendor identity

**Severity**: Low
**Class**: Cross-cutting state coupling — buyback queue not bound to the originating vendor
**Wire surface**: `Event_NetOut_BuybackItems`
**Demonstrable / Likely-theoretical**: Likely-exploitable theoretical (minor — outcomes are server-favorable in current shape)

**Trust violation**
When a player sells items at vendor A, the sell handler moves the items
to `container_id = 16` (INV_BUYBACK) and stores the sell unit price in
the row's `flags` column. The buyback handler later reads `flags AS
unit_price` and charges the player that amount to retrieve the item. The
buyback handler's query is keyed on `(character_id, item_id IN $items,
container_id = 16, flags > 0)` — it is **not** keyed on the originating
vendor's template id. This means:

- A player can sell at vendor A (a high-sell-price vendor) and buy back at
  vendor B (any other open vendor). The buyback price is the original
  vendor-A sell price, but the operation is performed in vendor-B's
  session. This violates the spec assumption that buyback is per-vendor.
- The buyback row persists in `sgw_inventory` indefinitely — across
  logouts, instance transfers, and any time window. There is no
  expiration / decay. Real SGW likely had a per-session buyback window.
- Because the unit price lives in `flags`, any future bug that lets a
  client mutate the `flags` column on an INV_BUYBACK row (via MoveItem,
  ammo edit, etc.) would let them buy a row back for an arbitrary
  unit price. The buyback handler accepts whatever `flags` says
  without an upper bound. Today no client path writes `flags` for
  INV_BUYBACK rows so this is latent.

The exploit shape today is "buy back at the wrong vendor", which is not
server-favorable (player still pays the original sell price for their own
item). The risk is the indefinite persistence + the side channel.

**Evidence**
- Ghidra: n/a (server-side schema-coupling concern)
- Client behavioral log: n/a
- Cross-ref to Rust handler (for the fix author, NOT as truth):
  `crates/services/src/base/world_entry/methods/vendor/buyback/mod.rs:80-108`
  (the buyback query — no `vendor_template_id` filter, no time filter),
  `crates/services/src/base/world_entry/methods/vendor/sell/mod.rs:222-227`
  (the sell-side `flags = row.unit_price` UPDATE). Items-systems-advisor:
  the spec'd vendor state machine likely has buyback bound to the sell
  session and expiring on session close — verify before re-platforming.

**Attack scenario**
1. (Spec-divergence, not a direct dupe.) Player sells item X at vendor A
   for 100 naquadah. INV_BUYBACK row exists with `flags = 100`.
2. Player logs out, logs back in three days later, walks to vendor B.
3. Submits `buybackItems(item_id=X, qty=1)` with vendor B's template id.
4. Handler accepts (no vendor binding); debits 100 naquadah; restores
   item X to INV_MAIN.
5. Observable effect: spec drift; potentially detectable via a future
   audit that asserts "buyback retrieval at same vendor as sell."

**Suggested remediation (one line)**
Store the originating vendor template id and sell timestamp on the
INV_BUYBACK row (extend schema or repurpose another column), filter the
buyback query by `template_id = current_vendor AND timestamp > now() -
buyback_window`, and audit any future code path that mutates
`sgw_inventory.flags` to ensure it cannot touch INV_BUYBACK rows.

**Would benefit from x64dbg trace?**
No.

---

### CAT-E-05 — `INV_BUYBACK` capacity (12) silently aborts sell; client-side reconciliation needed

**Severity**: Low (UX / availability, not direct exploit)
**Class**: Server-authority-correct but client-trust adjacent — sell silently aborts when buyback full
**Wire surface**: `Event_NetOut_SellItems`
**Demonstrable / Likely-theoretical**: N/A — not exploit-shaped; flagged for completeness

**Trust violation**
The sell handler reserves INV_BUYBACK slots via `reserve_free_inventory_slots`
and aborts the whole transaction with a warn when there are not enough free
buyback slots. INV_BUYBACK has capacity 12. The client likely does not know
this server-side limit; the abort surfaces as a silent no-op on the client
(no error packet is sent to the wire — `tracing::warn!` is server-side only).
A player whose buyback is full will see "sell did nothing" with no
explanation, which is not an exploit but is a server-authority correctness
question if the client UI is showing the sale completed. Not filing as a
finding but documenting for the items-systems-advisor review.

**Evidence**: server-only, no Ghidra needed.

---

### CAT-E-06 — Buyback handler accepts arbitrary `vendor_template_id` in the message payload but never uses it for filtering

**Severity**: Low (informational — see CAT-E-04)
**Class**: Wire field is present but ignored — defense-in-depth gap
**Wire surface**: `Event_NetOut_BuybackItems`
**Demonstrable / Likely-theoretical**: Likely-exploitable theoretical (latent, not a direct exploit today)

**Trust violation**
`handle_buyback_vendor_items` accepts `vendor_template_id: i32` as a
required parameter (rejects `None` at the cell dispatcher), but the
function body uses it only for `tracing::instrument` fields and for the
`handle_open_vendor_store` callback at the end. The buyback query is not
filtered by `vendor_template_id`. So while the cell dispatcher correctly
verifies that the client's claimed template id matches the player's
opened vendor (via `validate_template_id`), the base handler then
discards that information. This is the same shape as CAT-E-04 — the
finding is the same, but emphasising that a *future* validation that
relies on `vendor_template_id` reaching the SQL layer would not work
because the field is dropped.

**Evidence**
- Cross-ref to Rust handler (for the fix author, NOT as truth):
  `crates/services/src/base/world_entry/methods/vendor/buyback/mod.rs:38`
  (`vendor_template_id: i32` parameter), `line 80-108` (query without
  template filter). Items-systems-advisor: see CAT-E-04 remediation.

**Suggested remediation (one line)**
See CAT-E-04 — if buyback should be per-vendor, the SQL filter is the
right place to enforce it.

**Would benefit from x64dbg trace?**
No.

---

## Not Filed

- **Client-supplied `(item_id, quantity)` array in PURCHASE_ITEMS** — the
  `item_id` is actually a `store_index` (server-recomputed via
  `ROW_NUMBER() OVER (ORDER BY item_id) - 1` on the vendor's
  `item_list_items`), and `quantity` is a multiplier capped at 9,999 with
  overflow/negativity guards. `cash_cost` and `grant_quantity` are both
  recomputed from DB via `checked_mul`. The server never reads a price
  from the client. Not filing — this is the correct shape.
- **Client-supplied price field in any vendor message** — searched for it,
  doesn't exist. The wire only carries `(item_id/index, quantity)` and
  optional trailing `template_id`. Not filing.
- **Currency type validation (naqahdah vs. training points etc.)** —
  vendor purchases always debit `sgw_player.naquadah`; there is no
  multi-currency selector on the wire today. Not filing in CAT-E; flag
  for CAT-F if crafting / R&D has training-point or expertise spending.
- **Faction / reputation discount computed client-side** — no such discount
  is present in any vendor handler today; the `naquadah` field in
  `resources.item_list_items` is the price, multiplied only by the
  client's requested quantity (server-side `checked_mul`). Not filing.
- **`PurchaseItems` slot reservation race against concurrent
  `MoveItem`** — purchase reserves free INV_MAIN slots inside the same
  `tx.begin()` that takes the `sgw_player.naquadah` `FOR UPDATE` and the
  per-row inventory locks via `consume_design_quantity`'s `FOR UPDATE`.
  Slot reservation is part of the locked window. The TOCTOU is contained.
  Not filing.
- **Free repair sets durability = 100 instead of an item-cost-weighted
  partial repair** — by design (the free path is meant for content-driven
  repair sources). The exploit is *whether the free path is reachable
  from the wire*, which is CAT-E-01. Not filing separately.
- **Sell handler doesn't notify the cell of the moved item** — the buyback
  handler comment explicitly documents that `InventoryItemGranted` is
  not emitted on buyback (the item id is reused). Sell does enqueue
  `InventoryItemRemoved` to the outbox. Correct shape. Not filing.
- **`repair_ratio: f32` floor-clamp via `.max(1)`** — the server clamps
  silly client values (NaN, negative, > 1.0) before use. The trust
  violation is that the client supplies the ratio at all — flagged as
  CAT-E-03 (latent, not currently reachable). Not refiling the float
  edge cases separately.
- **Vendor handler returns silently on DB error (no error packet to
  client)** — observability gap, not server-authority. Out of scope.
- **`paid_repair` UPDATE row-count check returns `r.rows_affected() !=
  item_ids.len()` as a rollback condition** — this is the correct
  pessimistic shape; not a finding.
- **`reserve_free_inventory_slots` runs an unbounded loop scanning slots**
  — capacity-bounded by `bag_max_slots` and gated by the prior duplicate
  check. Performance concern at scale, not an exploit. Out of scope.
