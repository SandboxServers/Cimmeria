# DR-L2-canary: canary verification (C1, C2; C3 not graded)

> **Type:** results. **Audience:** the orchestrator and architect. **Verified:** 2026-11-02, read-only (run bundles, SigNoz `cimmeria.lab_run = 'dr-lab'`, `lab_uat_report`). **Build:** `service.version` 6b50d9131417b8cf93feb61a7c640577398db567 (read from SigNoz; not attested, see Not run). Method: [lab-run-plan.md § Canary, § Verifier steps](../lab-run-plan.md), [verify/DR-L1.md](../verify/DR-L1.md).

**Recommendation: fix-needed.** One systemic spec/tool defect blocks the sweep: a granted weapon is equipped into a free bandolier slot but is not made the active weapon, so weapon rows hit a wrong-weapon refusal or fire the wrong weapon's binding. Details under "Recommendation".

## Runs

| Call | Spec or split file | plan_only | run_dir | Outcome |
|---|---|---|---|---|
| C1 | armory-abilities-soldier-0 (Soldier-00, 598, 717, 715, 716, 714) | no | 2026-11-02-unknown-cgbqfvxipvwymg | PASS 6, FAIL 1. Fresh character created, world 1300 reached. |
| C2 (attempt 3) | armory-gear-weapons-1 (GW-3128, GW-17, GW-5680) | no | 2026-11-02-unknown-cgbqgkqofkhbys | PASS 1 (_session_end), FAIL 1, BLOCKED 1, UNVERIFIED 1. Fresh character created. |
| C2 (attempt 2) | armory-gear-weapons-1 | no | 2026-11-02-unknown-cgbqgcoconzfan | Not graded (hover failures fixed since). |
| C3 | armory-gear-armor-1 | no | other agent | Not graded here. |

The run ids in the briefing carried the character-name tail; the real directories are `cgbqfvxipvwymg` and `cgbqgkqofkhbys`.

## Per-row grades

| Row | Runner | Server evidence (SigNoz, scope abilities, fresh character) | Grade | Class |
|---|---|---|---|---|
| Soldier-00 | PASS (world 1300) | `server_sessions`: world DebugArea, in_world | PASS | |
| Soldier-598 Quick Burst | PASS | `ability_launched` 17:39:15.300 and 17:39:25.829; `fire_target_skipped no_target` (press carries target 0) | PASS | no target resolved |
| Soldier-717 Snare Shot | FAIL (clause applied: 0 events) | `use_ability_recv` 17:39:49.535 and 17:39:58.280; `wrong_weapon_refused` and `ability_refused reason=wrong_weapon_type`; `active_item_name` SGHC 6 SMG (item 21), `required_monikers` ITEM_Assault_Rifle | FAIL | spec/tool defect |
| Soldier-715 Hollow Point | PASS (its applied clause also had 0 events, not required) | `ability_refused reason=no_mechanics` (`passive_ability`) 17:40:22.175 and 17:40:30.954 | PARTIAL (refusal row) | server gap, not a blocker |
| Soldier-716 Wounding Shot | PASS | `ability_launched`, `warmup_complete`, `fire_target_skipped no_target` at 17:40:58 and 17:41:09 | PASS | no target resolved |
| Soldier-714 Stance: Mobility | PASS | `ability_launched`, `beneficial_cast_applied`, `toggle_pressed on` 17:41:29.996 and `off` 17:41:39.690 | PASS | |
| GW-3128 (bound) | BLOCKED (lab_login focus: bridge connect failed) | none | BLOCKED | lab defect, cold-start blip; row never ran |
| GW-17 (unbound) | UNVERIFIED (unbound clause pending) | 2 `weapon_unbound reason=no_ranged_attack item_type_id=17` rows: 17:55:36.664 (target 100792, GW-17 window) and 17:56:10.590 (target 100793, GW-5680 window) | PASS on evidence; ledger still UNVERIFIED because the attest was refused (no lease) | |
| GW-5680 (bound) | FAIL (client.ability.sent 1216: 0 events; click posted_only) | no `use_ability_recv` for 1216. The right-click reached the server as an interact on target 100793 and logged `weapon_unbound` for item 17, the P-90 left active by GW-17 | FAIL | spec/tool defect |

Report counts: C1 PASS 6, FAIL 1. C2 PASS 1, FAIL 1, BLOCKED 1, UNVERIFIED 1.

## Soldier-717: decision

The server saw the press twice (`use_ability_recv`, mercury_seq 69 and 75) and refused both at the gate with `wrong_weapon_type`. The active item was the starter SGHC 6 SMG (item 21), not the AR 21. The row's setup `client_item_action equip "AR 21 Assault Rifle"` moved item 10318 from Main to Bandolier slot 2, but the bandolier is the only container with an active slot (inventory-system.md:144) and slot 1 already held the SMG, so the AR never became active. The client correctly emitted `client.ability.sent` and, being refused, never `client.ability.applied`.

Verdict: not a server defect (the gate did its job) and not a missing client event. It is a spec/tool defect: `equip` does not make the weapon active and the spec has no step that selects the slot.

## GW-5680: decision

In one character: GW-17 equipped the P-90 into Bandolier slot 1 (free, so it became active); GW-5680 then equipped the Rekkr Attack Program into slot 2 and the P-90 stayed active. The right-click hit `weapon_unbound` for item 17 (SigNoz 17:56:10, inside the GW-5680 window, target 100793 = that row's dummy), so the client sent an interact rather than `useAbility 1216` and the `sent` clause found 0 events. The server did receive the click and the target was in range (3.0 m). Same root cause as Soldier-717, plus state leaking between rows that share a character in one file.

## Canary checklist

| Check | Result | Evidence |
|---|---|---|
| C1 and C2 each created a fresh character, named in `fresh_characters` | PASS | run.json: C1 and C2 each name one Soldier |
| Soldier-00 PASS (world 1300) | PASS | row JSON; server_sessions DebugArea |
| Every row JSON has an `anchor` with a bookmark id | MISS (flag, not a stop) | Soldier-00 "no bookmark reply seen"; GW-17 `bookmark_id: null`; GW-3128 no anchor. Other rows have ids, so replies arrive intermittently |
| Every weapon row shows `client.ability.sent` and has a SigNoz abilities row | MISS | Soldier-717: sent yes, SigNoz refusal row yes. GW-5680: sent 0, SigNoz row is the wrong weapon's `weapon_unbound`. GW-3128 never ran. GW-17 is unbound by design |
| GW-17 SigNoz clause: at least 1 `weapon_unbound` / `no_ranged_attack` row | PASS on evidence, attest not recorded | Query returns 2 rows (one per window); `lab_uat_attest` refused: "lease none is not the current lease" |
| GA-33 `moved` clause finds `Inventory move persisted` | Not graded | C3 belongs to another agent |

## Server-side defects

None from the canary. For DR-99: Hollow Point Ammunition (715) is refused as `passive_ability` / `no_mechanics`; Quick Burst (598) and Wounding Shot (716) launch but `fire_target_skipped no_target` because the press carries target 0 and the spec sets no target.

## Not run

- GW-3128: BLOCKED, `lab_login` failed at step `focus` (bridge connect). Re-run in the next canary pass.
- The GW-17 `unbound` attest and the `service.version` attest: `lab_uat_attest` requires a lease and the verifier brief forbids one (R5). Evidence is above; a lease holder can attest with the exact query (below).
- C3 armor: another agent.

## Recipe

Exact query (SigNoz logs): `service.name = 'cimmeria-server' AND cimmeria.lab_run = 'dr-lab' AND scope_name = 'abilities' AND event = 'weapon_unbound' AND reason = 'no_ranged_attack' AND item_type_id = 17 AND player_name CONTAINS '<C2 character>'`, window 2026-11-02T17:54:25.103Z to 17:57:40.139Z: 2 rows. Soldier cross-check: `scope_name = 'abilities' AND player_name CONTAINS '<C1 character>'`, 3 h range. The SigNoz tool takes `filter`, not `query`.

## Recommendation

Blocks the sweep (fix before the full DR-L2 runs):

1. Active weapon selection (every weapon row; Soldier-717, GW-5680). Options: (a) `@item_action equip` sends `requestActiveSlotChange` for the slot it filled; (b) the spec generator emits a setup step selecting the granted weapon's bandolier slot before the press, in abilities and gear files (least invasive); (c) reset the bandolier first (automated-uat.md:533 item reset moves the readied slot to the lowest occupied one). Re-run C1 (Soldier-717 must reach `ability_launched`) and C2 GW-5680 (must show `use_ability_recv` 1216).
2. Row isolation in gear files: with 1 fixed, a bound row after GW-17 must not inherit the previous weapon; reset the bandolier per row, or run unbound rows last.

Does not block:

3. GW-3128 BLOCKED is a cold-start `lab_login` bridge blip; re-run.
4. Missing `.bug` bookmark ids: flag only.
5. `no_target` on 598/716 and the 715 passive refusal: graded PASS/PARTIAL; add a target step only if the sweep needs damage evidence.
6. Verifier lease: R5 should let the verifier use the orchestrator's lease_id (or the attest tool accept none) so SigNoz clauses can be attested.

canary-verify fix-needed

## Round 2

**Verified:** 2026-11-02 18:40 UTC, read-only plus one short own lease (free at the time; acquired, attested, released, no driving). **Build:** `service.version` 6b50d9131417b8cf93feb61a7c640577398db567, attested for run `cgbqhrgysppyec`. Round 1's fix-needed items 1 and 2 (active weapon selection, row isolation) are fixed: every row now shows `armed` PASS with the right weapon name. Round 2 finds one remaining spec defect.

### Runs (round 2)

| Call | Spec or split file | plan_only | run_dir | Outcome |
|---|---|---|---|---|
| C1c | armory-abilities-soldier-0 (Soldier-00, 598, 717, 716) | no | 2026-11-02-unknown-cgbqhoifvzqqyr | PASS 5 (incl. _session_end), no FAIL. Weapons SGHC 6 SMG / AR 21 / SGHC 6 SMG. |
| C2 | armory-gear-weapons-1 (GW-3128, GW-17, GW-5680) | no | 2026-11-02-unknown-cgbqhrgysppyec | After attest: PASS 2 (GW-17, _session_end), FAIL 2 (GW-3128, GW-5680). |
| C3 | armory-gear-armor-1 (GA-33, GA-2933), old run before the close-bag change | no | 2026-11-02-unknown-cgbqgmvmfbwcex | NEEDS_HUMAN 2 after attest; the `moved` clause holds for both. |

### Per-row grades (round 2)

| Row | Runner | Server evidence (SigNoz, scope abilities, fresh character) | Grade | Class |
|---|---|---|---|---|
| Soldier-00 | PASS | `gmGiveAllAbilities` added 72 at 18:28:52.98 | PASS | |
| Soldier-598 Quick Burst | PASS | `use_ability_recv`, `ability_launched` 18:29:07.076 and 18:29:17.560 (target 0, `fire_target_skipped no_target`) | PASS | no target resolved (DR-99) |
| Soldier-717 Snare Shot | PASS | `ability_launched` 18:29:45.199 and 18:29:55.776 (AR 21 active; round 1 refused) | PASS | round-1 defect fixed |
| Soldier-716 Wounding Shot | PASS | `ability_launched` 18:30:21.436 and 18:30:32.039, `warmup_complete` | PASS | no target resolved (DR-99) |
| GW-5680 Rekkr Attack Program | FAIL (`sent` clause: 0 `client.ability.sent`) | `ability_launched` ability 1216 Drone Ranged AA, target 100942 (the row's dummy), 18:33:47.579 | FAIL by runner, server PASS | spec defect |
| GW-3128 Test Weapon | FAIL (`sent` clause: 0 events) | `wrong_weapon_refused` and `ability_refused reason=wrong_weapon_type`, ability 559 Automatic Weapon Auto Attack, `active_item_id` 3128, required moniker ITEM_Automatic_Weapon, active item has none; client told "You need a different weapon to use that ability." 18:32:20.181 | FAIL | spec defect (clause) plus a real data finding |
| GW-17 P-90 (unbound) | UNVERIFIED, now PASS | exactly 1 `weapon_unbound reason=no_ranged_attack item_type_id=17` in the row window (18:33:11.466, target 100941) | PASS (measured gap, F8) | |
| GA-33 (armor) | NEEDS_HUMAN | `moved` holds: 1 `Inventory move persisted` Aramid Fiber Ballistic Helmet MAIN to HEAD (container 4) 17:57:41.166 | PASS (server clause); human clause NEEDS_HUMAN | `rows/armory-gear-armor-1/GA-33/action13.png` shows no sheet (HUD only, portrait without helmet) |
| GA-2933 (armor) | NEEDS_HUMAN | `moved` holds: 1 row Boron-Carbide Ballistic Helmet MAIN to HEAD 17:57:53.298 (swapped out item 10340) | PASS (server clause); human clause NEEDS_HUMAN | `.../GA-2933/action7.png` shows no sheet; the portrait shows a helmet |

Attested this round: GW-17 `unbound` (1 row), GA-33 `moved` (1), GA-2933 `moved` (1), `service.version`. Lease released.

### GW-3128 and GW-5680: decision

**The click reached the client and the server. The failure is in the spec, not the Armory or the lab tools.**

- Client side: `client_target` and the right-click `client_world_click` both hit the Jaffa dummy at pixel (676,391), `hover_verified` true, `mouse_over_after` equal to `target_after` equal to the dummy id, 3.0 m (interact range 5.0 m), `InventoryWin` not open, `armed` PASS. Bundle: `rows/armory-gear-weapons-1/GW-3128.json` actions 16 to 19, `GW-5680.json` actions 10 to 13; final shots `GW-3128/final.png`, `GW-5680/final.png`. GW-5680 bookmark id 1793644403766 (GW-3128 has none).
- Server side: the right-click is **not a `useAbility`**. It arrives as `wire_inbound cellMethod interact` (method 74, args = the target id, 18:32:20.172), then `interact: targeting hostile NPC for combat` (route `hostile_attack`), and the server resolves the weapon's RANGED binding itself. For GW-5680 that launched ability 1216. For GW-3128 it tried ability 559 and the weapon gate refused. There was never a `use_ability_recv` for either, and there cannot be.
- `client.ability.sent` is emitted when `RouteOutgoingEntityRpc` reaches an allowlisted `start*Message` for an ability method (`crates/client-telemetry/src/hooks/ability_trace/mod.rs`). `interact` is not an ability method, so the clause `client.ability.sent ability_id 559/1216 since press` can never match on a right-click. `gap: false`, `suppressed: 0`, `upstream_dropped: 0` confirm a clean read, not a loss.
- Classification: spec defect (the generator's `sent` clause assumes the wrong wire path). Round 1's reading (the right-click hit `weapon_unbound` for GW-5680) is superseded: with the round-1 fix the right weapon is active and 1216 launches.
- Separate real finding for GW-3128 (Armory data, not a sweep blocker): item 3128 Test Weapon has a RANGED event binding to 559, but 559 requires the ITEM_Automatic_Weapon moniker and the item carries none, so the weapon can never fire its own binding. Triage under DR-99: add the moniker to the item, or drop the binding.

### What fixes it (generator only; not applied)

There is no way to produce `client.ability.sent` from a gear weapon: a hotbar key sends `useAbility`, but the weapon's auto-attack ability is not on the hotbar, and the right-click path is `interact` by design. Grade the server instead, as the unbound row already does.

In `tools/ability_mechanics/debug_armory_gear.py` `weapon_row`, replace the `sent` clause (source `client_event`) with a SigNoz clause on bound rows: `scope_name = 'abilities' AND event = 'ability_launched' AND ability_id = {ability} AND player_name CONTAINS '${character}'`, min_rows 1, text "the right-click launches ability {ability} (RANGED binding)", same window as the unbound row's clause. GW-3128 keeps the same clause; until the item gets its moniker it grades FAIL on the true defect (no `ability_launched`, a `wrong_weapon_refused`). Keep `armed` as is (it works). Update `test_debug_armory_gear.py` to assert the SigNoz clause and no `client_event` `sent` clause, regenerate the gear TOMLs (LF), run the spec guards. The verifier then attests the SigNoz clause per row (own lease), as for GW-17.

Minor, not blocking: `.bug` bookmark replies still missing on some rows (GW-3128, GA-33 have no id); the `uat_clear_effects` teardown fails in every gear row ("no rendered entity matches" the own name) and is flagged only; GW-3128's optional `reset-slot1` fails with "Bandolier slot 1 is empty", as expected on a fresh character.

### Canary checklist (round 2)

| Check | Result | Evidence |
|---|---|---|
| C1 and C2 each created a fresh character | PASS | run.json: Cgbqhoifvzqqyrdj, Cgbqhrgysppyecij |
| Soldier-00 PASS | PASS | row JSON, `gmGiveAllAbilities` 72 |
| Every weapon row shows the right weapon armed | PASS | `armed` PASS in all rows |
| Every ability row has a SigNoz `ability_launched` | PASS for 598, 717, 716, 5680; 3128 refused at the gate | above |
| `client.ability.sent` on gear rows | MISS by design | right-click is `interact`; spec defect |
| GW-17 SigNoz clause attested | PASS | 1 row |
| GA-33 / GA-2933 `moved` attested | PASS | 1 row each |
| Build version attested | PASS | 6b50d9131417b8cf93feb61a7c640577398db567 |
| Every row JSON has an anchor bookmark id | partial (flag only) | 5680 and GW-17 have ids; 3128 and GA-33 do not |

### Recommendation (round 2)

**fix-needed, small, generator only:** replace the gear weapon rows' `client_event` `sent` clause with the SigNoz `ability_launched` clause before the sweep. The abilities files (C1c) are ready to run as they are. Armor files need only the already-planned close-bag step (the old C3 run predates it) and a screenshot of the open character sheet for the human clause. The Test Weapon moniker mismatch (item 3128, ability 559) goes on the DR-99 list and does not block the sweep.

canary-verify-2 fix-needed

## Round 3

**Verified:** 2026-11-02 18:50 UTC, read-only plus one short own lease (free at the time; acquired, attested, released, no driving). **Build:** `service.version` 6b50d9131417b8cf93feb61a7c640577398db567, attested for run `cgbqiepaxidmrb`. The generator fix (SigNoz `ability_launched` clause on bound gear rows) is in: GW-5680 now carries a `launched` clause instead of the `client_event` `sent` clause.

### Runs (round 3)

| Call | Spec or split file | plan_only | run_dir | Outcome |
|---|---|---|---|---|
| C2 | armory-gear-weapons-1 (GW-17, GW-5680) | no | 2026-11-02-unknown-cgbqiepaxidmrb | Before attest: UNVERIFIED 2, PASS 1 (_session_end). After attest: PASS 3. Fresh character Cgbqiepaxidmrbij. |

### Per-row grades (round 3)

| Row | Runner | Server evidence (SigNoz, scope abilities) | Grade |
|---|---|---|---|
| GW-17 P-90 of the Whale 2 (unbound) | UNVERIFIED (`unbound` pending), `armed` PASS | Exactly 1 `weapon_unbound reason=no_ranged_attack item_type_id=17` in the window 18:46:00Z to 18:49:17Z: 18:47:13.511Z, target 100979 (the row's dummy), item P-90 of the Whale 2 | PASS (measured gap, F8), attested |
| GW-5680 Rekkr Attack Program (bound) | UNVERIFIED (`launched` pending), `armed` PASS | 1 `ability_launched` ability 1216 Drone Ranged AA at 18:47:49.462Z, target 100980 (the row's dummy), window 18:47:14Z to 18:49:53Z; bookmark id 1793645245742 | PASS, attested |

Round 2's decision stands: the right-click arrives as `interact`, so the server resolves the RANGED binding itself, which the SigNoz clause now measures. Flags only (not blocking): `uat_clear_effects` teardown fails ("no rendered entity matches"), GW-17 has no `.bug` bookmark id, `reset-slot1` on a fresh character is "empty" as expected.

### Final canary checklist

| Check | Status | Evidence |
|---|---|---|
| C1c Soldier rows (Soldier-00, 598, 717, 716) | PASS | Round 2, run cgbqhoifvzqqyr: 5 PASS, 717 reached `ability_launched` with the AR 21 active |
| C2 gear weapons: GW-17 | PASS | above (round 3) |
| C2 gear weapons: GW-5680 | PASS | above (round 3) |
| C2 gear weapons: GW-3128 | Known data finding, not a blocker | `wrong_weapon_type` (ability 559 needs ITEM_Automatic_Weapon; item 3128 lacks the moniker). Recorded for DR-99: add the moniker or drop the binding. Not re-run in round 3 (spec fix gives the true-defect FAIL by design) |
| C3 armor: GA-33, GA-2933 `moved` clauses | PASS (server clause) | Round 2 attests; the human clauses stay NEEDS_HUMAN (D-DR9) |
| Armed weapon correct in every row | PASS | `armed` PASS in all rows |
| Build version attested | PASS | 6b50d9131417b8cf93feb61a7c640577398db567 |

**ALL canary checks now pass** (C1c, C2, C3), with GW-3128's `wrong_weapon_type` recorded as a known data finding and the armor human clauses left NEEDS_HUMAN per D-DR9. Nothing blocks the sweep.

canary-verify-3 proceed
