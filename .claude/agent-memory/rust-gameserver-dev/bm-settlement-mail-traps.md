---
name: bm-settlement-mail-traps
description: BM-02b (2026-09-27) - Black Market settlement through send_system_mail_tx; exactly-once status gate first; quarantine status 4; test and mutation traps (last_auction_of, unused bind params, live-db-test output on stderr)
metadata:
  type: project
---

Learned doing Black Market packet BM-02b (branch `bm/02b-mail-settlement`).

- **Status gate first.** The mail writer mints cash on every call, so `settle_locked` and `cancel_auction` do the conditional `UPDATE sgw_auction ... WHERE status = ACTIVE RETURNING` as their FIRST write; a later error rolls it back. A player listing's second settlement also fails on `ItemNotFound`, so only a boot-seed (`item_id = 0`, `Minted`) double-settle proves the gate (`a_second_settlement_pays_nothing`).
- **Quarantine = status 4.** `SettleError::is_permanent` (escrow missing, any `SystemMailError` but `Db`) -> rolled back, then `QUARANTINED` in its own tx; `Db` errors retry. Search/cap/seed all filter `status = 0`, so status 4 is inert. No GM tool resolves it yet.
- **A refused mail leaves the tx usable only for pre-insert refusals.** `RecipientNotFound` is raised before any SQL write, so `refund_standing_bid` can log `bm.refund_skipped` and carry on inside the same transaction.
- **`tracing` macro + `.await` in a field = non-Send future** (`format_args!` held across the await) -> `tokio::spawn` of the sweep fails to compile. Hoist the await into a `let`.
- **Test traps.** `last_auction_of` is `MAX(sequence_id)`: a directly inserted auction with a sentinel `sequence_id` (0x7000_Axxx) outranks every sequence-allocated id; look up by `item_id`. `tools/build-lane/live-db-test.sh` prints nextest's result to stderr: a mutation script that captures only stdout sees just the `reload-db` line.
- **Revert-proof SQL mutations:** dropping `AND status = $3` leaves `$3` bound but unused, and Postgres rejects the statement (wrong failure). Neutralise with `... AND status = $3 OR sequence_id = $2` instead.

Related: [[black-market-escrow-and-authority]], [[mail-expiry-and-notify-seams]], [[revert-proof-mutation-must-be-confirmed]].
