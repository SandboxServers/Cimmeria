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
