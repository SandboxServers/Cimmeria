---
name: mail-placement-rule-and-fixture-types
description: mail take/send/pay-COD share take::carried_bag; resources.items' lowest id (10) is a {2} mission-only type, so "first item" fixtures break mail tests
metadata:
  type: project
---

Since SS-M4 (#933), a mail take places an item by `container_sets` (bag 1 or 15), and a type with no carried bag is refused `no_carried_bag`. Since ss-fix1 (2026-09-27), the send (`item_no_carried_bag`) and pay COD (`no_carried_bag`, before the debit) refuse such a type too. All three use `mail/take.rs::carried_bag`. System mail (`mail/system/write.rs`) still escrows such types (open follow-up).

`SELECT item_id FROM resources.items ORDER BY item_id LIMIT 1` returns type 10 ("Gopher Head", `{2}`). It is one of 801 mission-only types. A test that mails "the first item" gets a refusal, and the wireclient `two_client_mail_cod` test broke this way. Use `mail::tests::any_type_id`'s predicate (`container_sets` empty or `container_sets[1] = 1`).

**Why:** the type-11 wireclient tests are not in CI, so a base-methods fixture fix silently missed the wireclient copy for a day.

**How to apply:** any new mail/inventory test fixture picks its type by `container_sets`, never by lowest id. When the take/placement rule changes, grep `crates/wireclient/tests/it/` for fixtures too. Related: [[grant-paths-pick-different-containers]].
