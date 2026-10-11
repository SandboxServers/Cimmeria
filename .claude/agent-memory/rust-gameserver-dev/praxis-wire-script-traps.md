---
name: praxis-wire-script-traps
description: Wire-scripting the Praxis start (mission 622): the pistol is a chain-1005 grant, ability arrays are hash-ordered, raw trails drown in 0x10, movie vs arrival dialog race
metadata:
  type: project
---

Facts found writing WC-14 (`crates/wireclient/tests/it/praxis_start_live_db.rs`, 2026-10-11):

- A Praxis Commando (char def 3) starts with **no pistol** since OD-CS01; design 55 comes from chain 1005 (`dialog_open` 3996, `add_item` 55 to container 1). Its `onUpdateItem` arrives before the step-6 mission updates (base vs cell order).
- `onKnownAbilitiesUpdate` sends the whole known set from a hash set: order changes per run. Compare as a set; never pin a golden on its order.
- The base does answer `moveItem`: a full `onUpdateItem` resync, slot 1-based on the wire (`send_update_item` adds one). The lab tap misses it because it does not see base traffic.
- `onPlayMovie` (base) and the arrival dialog 2982 (cell) race; scan the log from 0 for both, don't move the `ScriptSession` cursor past one.
- In the Cellblock, NPC `0x10` avatar updates fill `ScriptSession::trail(30)` within a second; print a decoded-events trail too.

**Why:** each of these made a first draft fail or would make a spec-literal test flaky.
**How to apply:** any wireclient script in the Cellblock or any golden over ability arrays. Related: [[aoi-fixture-introducible-and-wire-ledger]].
