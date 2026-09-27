# SS-FIX1 Worknote: the two-client COD mail round trip

> Type: reference. Audience: the social-systems coordinator and the PR reviewers.
> Companions: [README.md](../README.md), [ss-m3.md](ss-m3.md), [ss-m4.md](ss-m4.md).

## Contract

- **Task:** the type-11 wire test `two_client_mail_cod::cod_item_round_trip_between_two_clients` failed on clean main with "never received B's header without the item". Find the change that broke it, fix the server or the test, and add a CI-run guard for the same bug shape.
- **Base:** `origin/main` @ `3ad5e571b`. Branch `fix/mail-cod-roundtrip-header`.
- **Edited:**
  - `crates/base-methods/src/base/world_entry/methods/mail/take.rs`: `carried_bag`, the one placement rule, used by the take.
  - `crates/base-methods/src/base/world_entry/methods/mail/send/{deliver.rs,attachment.rs}`: the send refuses `item_no_carried_bag`.
  - `crates/base-methods/src/base/world_entry/methods/mail/cod.rs`: pay COD refuses `no_carried_bag` before any debit.
  - `crates/base-methods/src/base/world_entry/methods/mail/tests/{placement_gate.rs,mod.rs}`: the guards.
  - `crates/wireclient/tests/it/two_client_mail_cod.rs`: the fixture picks a backpack type.
  - Docs: `docs/gameplay/mail-system.md` (send and Pay COD), `docs/architecture/observability.md` (`mail` row reasons).
- **Read set:** `mail/{take,cod,headers,notify,claim}.rs`, `mail/send/{escrow,deliver,attachment}.rs`, `mail/system/write.rs`, `mail/tests/{mod,take_placement,cod_live,attach_vault}.rs`, `cell-catalog/src/item_placement.rs`, worknotes `ss-m3.md` and `ss-m4.md`.

## Root cause

It was neither the header refresh nor the channel or login-burst changes. The header path is unchanged: `headers.rs:234` `refresh_one` still sends the remove, then the re-add.

1. SS-M4 (#933, `d97e35ec9`, the "allow the crafting bag (15)" commit in the squash) changed `take_item_tx` from an unconditional backpack insert to placement by the type's `container_sets` (`item_placement::first_player_container`). A type that names no carried bag is refused `no_carried_bag` and stays in escrow. `git log -S first_player_container -- mail/take.rs` names only this commit.
2. The wire test's fixture took `SELECT item_id FROM resources.items ORDER BY item_id LIMIT 1`, which is type 10, "Gopher Head", `container_sets = {2}`: a mission-only item, one of 801. SS-M4 moved the base-methods fixtures to `mail::tests::any_type_id`, a backpack type, but the wireclient fixture was missed.
3. So in the wire test, B paid the COD and the take was then refused. A database read after the failing run showed B's mail `cash 0, flags 0, cod_paid true`, the item still in `sgw_gate_mail_item`, and no inventory row. B had paid 300 and never received the item. No header refresh was sent because the take was refused, which produced the "never received" failure.

The stale fixture broke the test, but it also exposed a real server gap. The send and the COD payment accepted an item that the take can never place. A paid COD cannot be returned, so a buyer who pays for such an item loses the price and never gets the item. A GM grant (`gmGiveItem` grants to bag 1) can put a `{2}` item in a backpack, so a player could hit this.

## Fix

- `take::carried_bag(conn, type_id)` is the one rule: the first bag 1 or 15 in `container_sets`, bag 1 for an empty list, `None` for no carried bag or an unknown type. The take uses it unchanged.
- **Send:** after `check_source`, before any debit, an item with no carried bag is refused `MAILRESULT_ItemNotAvailable`, `reason=item_no_carried_bag`, with a feedback line. Nothing moves.
- **Pay COD:** after the escrow and player locks, before the debit, an item with no carried bag is refused `reason=no_carried_bag` with a feedback line. The mail stays an unpaid COD, so the payer can return it.
- **Wire test:** the fixture uses `any_type_id`'s predicate. With the old `{2}` fixture the test would now stop at the send, because `sendMailResult` is `ItemNotAvailable`.

## Tests

| Command | Result |
|---|---|
| `bash tools/build-lane/reload-db.sh`, then `lane.sh cargo test -p cimmeria-wireclient --test it two_client_mail_cod -- --test-threads=1` on `3ad5e571b` | exit 101, FAILED: "never received B's header without the item (method 76)" (reproduced) |
| same, with the fix | ok, 1 passed |
| `bash tools/build-lane/live-db-test.sh "placement_gate"` | exit 0, 2 passed, 0 skipped |
| `bash tools/build-lane/live-db-test.sh "methods::mail::"` | exit 0, 136 passed |
| wireclient `it`: `two_client_mail_cod`, `two_client_tell`, `duel_two_duelists`, `sparbot_duel` (`--test-threads=1`, freshly reloaded DB) | all ok (sparbot keep-alive test ignored by design) |
| `lane.sh cargo fmt --all -- --check` | exit 0 |
| `lane.sh cargo clippy -p cimmeria-base-methods -p cimmeria-wireclient --all-targets -- -D warnings` | exit 0 |

The CI-run guards are in `mail::tests::placement_gate`:

- `send_refuses_an_item_no_take_could_place`: a `{2}` item in the backpack, sent by COD, is refused: `sendMailResult` is `ItemNotAvailable`, the feedback line is sent, the refusal log carries `reason=item_no_carried_bag`, and nothing is debited, escrowed or mailed.
- `pay_cod_refuses_an_item_no_take_could_place`: this is the wire test's bug shape. A COD mail escrows a `{2}` item, then B pays and takes. The payer is not charged, and the mail is still `cash 300`, `MAIL_COD`, `cod_paid false`. No payment mail is written, the escrow is kept, and the log has `mail.op_refused op=pay_cod reason=no_carried_bag`.

## Regression proof

The fix was committed first (`933120eed`). Both gates were then disabled in place (`if false && carried_bag(...)` in `cod.rs` and `send/deliver.rs`), and `live-db-test.sh "placement_gate"` exited 100 with 2 failed:

- pay COD: `nothing charged`, left 700, right 1000;
- send: `Reply { code: Some(0), cash: [475], removed: [..] }`, where `ItemNotAvailable` (2) was expected. The item was mailed.

The files were restored with `git checkout HEAD --` and touched, and the rerun passed.

## Telemetry

No new events. There are two new `reason` values on existing WARNs: `mail.send_refused reason=item_no_carried_bag`, and `mail.op_refused op=pay_cod reason=no_carried_bag`. Both carry `account_id`, `player_id`, `entity_id` and `mail_id`, the latter where one exists. SigNoz query: `service.name = cimmeria AND attributes.reason IN ('item_no_carried_bag','no_carried_bag')`.

## Known gaps

- **System mail** (`mail/system/write.rs`, the GM `.mail` and the content `send_system_mail`) still mints or moves a type with no carried bag into escrow. There is no COD on server mail, so no money is lost, but the item cannot be taken and is later quarantined by expiry. That path is out of scope here, so it is left as a follow-up: refuse such a type in `prepare_item` with `carried_bag`.
- CI does not run type-11 tests, which is why this went unnoticed. The base-methods guards now cover the bug shape in CI.

## Integration edits for the coordinator

None needed. `work-packets.md` and `README.md` are untouched.
