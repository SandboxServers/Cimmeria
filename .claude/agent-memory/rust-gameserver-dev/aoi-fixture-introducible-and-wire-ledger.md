---
name: aoi-fixture-introducible-and-wire-ledger
description: A test player with account_id set never enters anyone's AoI without archetype_id; ability client sends go through wire_ledger (AB-T4), not the bare messaging helpers.
metadata:
  type: project
---

**AoI fixture trap (2026-10-04, AB-T4).** `CellEntity::is_introducible` is
`account_id.is_none() || (is_player && archetype_id.is_some())`. A test
fixture that sets `account_id` (to assert Rule-5 ids on a row) but not
`archetype_id` silently drops that player out of every other player's AoI:
`get_witnesses_of(player)` comes back empty, and witness-count asserts fail
with `0`. Set `archetype_id = Some(1)` alongside `account_id`, and assert the
witness relationship in the fixture so the failure names the cause.

**Ability sends since AB-T4.** Every client-bound send in the ability
subsystem goes through `cell::abilities::wire_ledger::send(entity, method,
args, WireRoute, WireCtx, tx, mgr)` (or `prepare(..)` +
`sent_to_owner[_as]` for a site that sends its own `EntityMethodCall` and
keeps its own failure WARN). That writes the `abilities.wire` `wire_sent`
row. The plain `send_entity_method*` helpers still exist for other systems
and share the router (`messaging::deliver`), so a failed send is a
`wire_send_failed` WARN everywhere. A new ability send that uses the plain
helper ships with no ledger row: AB-C7's coverage gate expects one.

`WireCtx` defaults `cast_id` to `SpaceManager::current_cast_id()`; a send
outside the cast scope (a pulse end, an expiry) passes `.cast(..)` itself.
See [[cell-systems-index]].
