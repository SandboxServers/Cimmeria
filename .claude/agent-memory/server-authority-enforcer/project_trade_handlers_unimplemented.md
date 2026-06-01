---
name: project-trade-handlers-unimplemented
description: All four player-to-player trade RPCs are unimplemented stubs; future implementers inherit the full dupe-prone wire surface unvalidated
metadata:
  type: project
---

All four `<Exposed/>` trade RPCs on `SGWPlayer.def` — `tradeRequest`,
`tradeRequestCancel`, `tradeUpdateProposal`, `tradeLockState` — dispatch to a
single `match` block in `crates/services/src/cell/cell_methods/player/social.rs`
(approximately lines 103–149) that decodes only the leading INT32 header bytes,
logs `UNIMPLEMENTED: …`, and returns `true` (Handled). No `TradeTransaction`
struct, no item-lock table, no escrow object, and no `onTradeState` /
`onTradeResults` emitter exist anywhere in `crates/`.

**Why:** The wire is exposed and the client UI fires the RPCs, but the server
discards the proposal payload. The danger is the *next* PR that fills the match
arms in piecewise without the dedicated state machine — this is exactly the shape
that produced the named [[stack-duplication-via-disconnect-timing]] pattern in
classic MMO history.

**How to apply:** Any review touching `social.rs` trade handlers, or any new file
under a `crates/services/src/.../trade/` directory, must verify the implementer
has added (in this order):

1. Same-space + range + alive + is-player + not-already-trading + not-self
   validation on `tradeRequest`.
2. Per-`instance_id` item-lock table at `tradeUpdateProposal` time (not at
   confirm), keyed by item_id not type_id (per the
   [[bandolier-ammo-key-by-item-id]] invariant).
3. Strict version-counter enforcement on `tradeLockState` — reject on mismatch,
   do not silently downgrade lockState to None as the deprecated Python
   reference does at `cell/Trade.py:203-204`.
4. Single-transaction confirm with rollback (all four item moves + both cash
   deltas in one `BEGIN/COMMIT`).
5. Disconnect cleanup hook from `cell/base` player-disconnect path that
   force-cancels the trade and emits a single `onTradeResults(Cancelled)` to the
   surviving partner; outbox must never re-fire trade messages on reconnect.
6. `can_trade()` predicate that respects `ITEM_FLAG_BindOnAcquire` and
   `ITEM_FLAG_BindOnEquip` (from `Atrea/enums.py:794-795`) — NOT `canSell()`,
   which the deprecated Python conflates.

The deprecated reference at `deprecated/python/cell/Trade.py` and
`deprecated/python/cell/SGWPlayer.py:1669-1814` shows the *attempted* shape but
itself contains gaps (no item lock, no atomicity, no disconnect handling, the
`canSell` ≠ `canTrade` conflation) — it is **not safe to port verbatim**.

Wire shape: `entities/defs/SGWPlayer.def:1033-1072` plus `LocalTradeProposal` /
`RemoteTradeProposal` FIXED_DICTs in `entities/defs/alias.xml:355-379`. Lock-state
enum at `entities/defs/enumerations.xml:1784-1791`.
