# UB-U9: each race sees only its own Carter (parallel clients)

| Field | Value |
|---|---|
| Row | UB-U9: a Human, a Free Jaffa and an Asgard fixture on Beta at once each click Carter in turn. Each sees only their own race's dialog. The Human and Free Jaffa never see the beam objective |
| Runnable | Yes for the Human and Free Jaffa clients, once `Fxbetafjdefa` exists. The Asgard client is **HELD until UO-16** |
| Instances | `default` (Free Jaffa) and `p2` (Human), one lease each: a lease is valid only on its own instance ([live-research-lab.md § Drive your own client](../../../guides/live-research-lab.md#drive-your-own-client)) |
| Characters | `Fxbetafjdefa` on `default`; `Uat Fxbetahumpc` on `p2` |
| Fixtures | F-FJ1 on `Fxbetafjdefa`; F-H2 on `Uat Fxbetahumpc`. Stage both before either client logs in |
| Budget | three dispatches: H1 20, F 26, H2 20 calls |
| Piggyback | none |

**From the seeds.** Each race's Carter chains are gated on archetype: the Human (1-4) at 1423 S4223 gets S4224 and Carter's 5433 on login (23020), and Carter shows 4643 (23022), then the 381 offer 1882 (23040, 23041). The Free Jaffa (7) at 1500 S4369 gets S4370 and 5871 on login (23220), and Carter shows 5026 (23222). The Asgard (5) at 481 S1761 would get 1915 (23312). The ground pad (spawn 149, `BetaSite_00000007`, (746.555, -19.21, 68.75)) beams anyone to the deck (23314, no gate); only an Asgard at 481 S1762 with O2080 open gets the beam objective (23316). So a Human or Free Jaffa click on the pad must move the player and complete no objective. Binds are per player (D-UB10), so one race's bind never shows to the other.

Stands: Human (1009.4, 13.57, 507.85), south of Carter; Free Jaffa (1009.4, 13.57, 515.85), north of Carter; both about 4.5 m from Carter (1011.405, 13.57, 511.851). Pad stand (746.5, -18.5, 78.0).

## Orchestrator before the run

1. During the approved session, the user makes `Fxbetafjdefa` by hand if it does not exist ([fixtures.md § Characters](fixtures.md#characters)). The `p2` instance must be set up (`lab instances init`, the lab runs up to five clients per daemon, #1312) and its account must be a GM account.
2. Take two leases: L1 on `default` and L2 with `lab_lease_acquire {..., "instance":"p2"}`. The lease routes each call to its client, so a driver passes only its own lease.
3. Stage F-FJ1 on `Fxbetafjdefa` and F-H2 on `Uat Fxbetahumpc`. Both must be offline.
4. Dispatch H1 (lease L2, instance `p2`), then F (lease L1, instance `default`), then H2 (L2, `p2`), one at a time. Never two drivers at once. Every screenshot passes that dispatch's `instance`. Both characters stay in the world between dispatches (no logout).

## Driver steps

Dispatch H1 (lease L2, instance `p2`, character `Uat Fxbetahumpc`; L below is L2):

1. `lab_ensure_in_world {"lease_id":L,"character":"Uat Fxbetahumpc"}`. (On first use only: create it as in [fixtures.md § Characters](fixtures.md#characters), then the orchestrator stages F-H2 and you log in again.) WAIT17. DIALOG.
2. FIND(2300). APPROACH. CLICK(Carter). DLGREAD (expect 4643). SHOT(`u9-human-carter-1`). FINISH.
3. Stop here; leave the character in the world.

Dispatch F (lease L1, instance `default`, character `Fxbetafjdefa`; L below is L1):

1. `lab_ensure_in_world {"lease_id":L,"character":"Fxbetafjdefa"}`. WAIT17. DIALOG.
2. FIND(2300). APPROACH. CLICK(Carter). DLGREAD (expect 5026). SHOT(`u9-fj-carter`). FINISH.
3. `client_entity_find {"lease_id":L,"name":"Fxbetahumpc","fields":["matches"]}`: record the match (the Human stands about 8 m away).
4. TP(746.5, -18.5, 78.0). FIND(3): take the match nearest (746.6, -19.2, 68.8). APPROACH. TAP(it). Then `client_batch {"lease_id":L,"steps":[{"op":"wait","ms":5000},{"op":"player_state","fields":["world_id","position"]}, UTC]}`. Record the position (expect the deck).
5. JOURNAL(`u9-fj-journal`): record any 481 objective.
6. FIND(3): take the match nearest (700.5, 158.0, 70.4). TAP(it). Then the step-4 batch (expect the ground).

Dispatch H2 (lease L2, instance `p2`, character `Uat Fxbetahumpc`; L below is L2):

1. `lab_ensure_in_world {"lease_id":L,"character":"Uat Fxbetahumpc"}` (no relog). DIALOG.
2. FIND(2300). APPROACH. CLICK(Carter). DLGREAD (expect the 381 offer 1882, not 5026). SHOT(`u9-human-carter-2`). FINISH (do not accept).
3. The pad trip: the same as dispatch F steps 4 to 6, with labels `u9-human-*`.

## Stop conditions

- A client cannot log in: record it, stop that dispatch, and tell the orchestrator. The other client's dispatch still runs.
- A call on L2 is refused with `(instance p2)` or runs on the wrong client: record the error verbatim and stop (BLOCKED: the parallel-client setup is wrong).

## Expected and pass/fail

| Check | Pass when | Evidence |
|---|---|---|
| A. Own dialog | The Human sees 4643, then 1882; the Free Jaffa sees 5026; neither ever sees the other's | DLGREADs, screenshots, server log |
| B. Both present | Both characters are in the world at once during dispatch F | F step 3, server log |
| C. No beam objective | The pad moves each to the deck and back, and no 481 objective appears or completes for either | F and H2 pad steps, journals, server log |

The row passes on A, B and C for the two runnable clients. The Asgard client is `not-run (HELD-B16)`: when UO-16 lands, add a third dispatch on `p3` with F-A1 (Carter shows 1915, the pad completes O2080), and re-grade A and C with all three.

## Orchestrator only: verify list

- `lab fixture applied` for both characters.
- One Carter chain per click, each a different id: 23022 (Human), 23222 (Free Jaffa), then 23041 (Human again). No 23312, 23222 or 23022 for the wrong character.
- On each pad tap: `fire_interact_tag: matched` `BetaSite_00000007` with only 23314 (`Content: teleporting entity`), never 23316, and no `Content: complete objective` 481 / 2080.
- Grep by character name: every `Content:` line for a click names the clicking character.
