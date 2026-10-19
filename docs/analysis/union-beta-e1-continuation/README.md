# Union Beta E1 continuation ledger

> **Type:** Reference (campaign ledger)
> **Audience:** The Union Beta coordinator and the packet coders, reviewers and specialists named in the packet table
> **Last updated:** 2026-10-19 (U-D07 Approved 2026-10-19, XP and naq; UB-50a and UB-50b1 Ready, UB-50b2 after them. Earlier the same day: H-LBL, U-D08/U-D11 recorded. Before that: UB-99 close-out)
> **Index:** [beta-work-packets.md](../union-zone-planning/beta-work-packets.md) (packet text, findings F1-F21, HELD list and waves; this ledger does not copy them)
> **Companions:** [RESUME.md](RESUME.md), [reward-proposal.md](reward-proposal.md) (U-D07, Approved 2026-10-19), [mission-seal.md](mission-seal.md) (UB-00a1/a2), [placement-seal.md](placement-seal.md) (UB-00b), [receiver-contract.md](receiver-contract.md), [id-amendment-request.md](id-amendment-request.md), [worknotes/](worknotes/README.md), [execution contract](../union-zone-planning/execution-contract.md), [SGC ledger](../union-sgc-continuation/README.md) (the model for this one)

## Purpose

Track the Beta_Site_Evo_1 (world 23, gate 6) first-visit campaign packet by packet: which packets are ready, which are blocked and why, which decisions are open, and what wave B0 verified before any seed or code packet starts.

## Status of this ledger

**Campaign state: code complete, lab pending.** Every dispatched packet from B0 to B5 is merged into `uzp-e1` (tip `f8e9f2f2e` when UB-99 started). The lab rows (UB-90) are UATPending and no lab was used. The close-out docs (UB-99) are done. The packet table below is current; the earlier B0/B1a text is history and lives in git.

Merged, each with at least one review-fix commit: UB-DP, UB-01a, UB-01b, UB-02a, UB-02b, UB-02c, UB-02d, UB-10 to UB-16, UB-20, UB-21, UB-30 to UB-36, UB-40, UB-45a and UB-45b. B0 (UB-00a1, UB-00a2, UB-00b, UB-00c) is integrated; UB-00a1's review pass 2 landed in `54e76cbbd`.

Not dispatched, by ruling: UB-22 and UB-51 are held (D-UB5 A), UB-52 is closed (D-UB7). UB-50 is split (2026-10-19). The owner approved the [U-D07 reward scale](reward-proposal.md) for XP and naquadah, so UB-50a (XP seed) and UB-50b1 (the `grant_cash` action) are Ready, and UB-50b2 (naquadah seed) follows both. The UB-50 relays stay on Hold.

## Close-out (UB-99)

### Tested and never tested

| Tested (automated, in `uzp-e1`) | Never tested |
|---|---|
| Live-DB `beta_e1` tests: chain replays per mission (`chain_replay_tests/beta_e1/`), spawner, nav and template tests (`spawner_tests/beta_e1/`), the gate and respawner guards, the dialog-override patch agreement test | Anything in the lab or the game client: UB-U1 to UB-U10 are all UATPending |
| The interact-tag linter slice `beta_e1.rs` and its allowlist | Whether the client draws each object template (UB-U2c) and shows disabled SGU steps in the journal (UB-U1b) |
| The cross-race guard (UB-45a one dialog per click, UB-45b login binds equal in-session binds) | The Asgard path for a real player: Omega's UO-16 does not deliver one yet (HELD-B16) |
| The sequence floor test `seeded_sequences_allocate_past_every_seeded_row` | The D-UB5 tails (1503, Jackson and Woolsey receivers): no content exists |

### Integration fix

UB-14's 1425 offer shows on Carter after 478 completes. UB-13's whole-seed Carter dialog count did not allow it, so the m478 test failed once both packets were in `uzp-e1`. Commit `ac2d5e1a5` corrected the count. The behavior was intended (D-UB15 linear offers).

### Findings

1. **Stock screens sit inside the Dakara and Omega authored blocks.** Beyond the screens already named in [§ Screens re-carve](#screens-re-carve-arch-b1), the Dakara 63500-63799 block holds stock screens 63509-63513, 63517, 63518 and 63644-63647, and the Omega 63800-63999 block holds 63847-63852, 63854, 63874 and 63881-63887. Those campaigns must re-carve (or skip those ids) before they author screens. Beta's own screens range 64124-64193 is clear.
2. **The UB-02a revert proof (c) was wrong.** A revert-proof that changes footer text in `UB-02bcd.md` is theatre at database level: every Beta footer sets its sequence to 23199 and the Debug Area re-sets sequences. Done: the `seeded_sequences_allocate_past_every_seeded_row` floor in `live_db_seed_sequences.rs` is already 23199, and the spawner sequence test `live_db_spawnlist_sequence.rs` now uses 23199 too (UB-F1).
3. **The UB-DP shared-infrastructure registration gap also affected the Castle and Cellblock tables.** The shared registration step that UB-DP had to extend was missing for the Castle and Cellblock tables too, so their patch tables were not covered by the same agreement guard. Closed by UB-F1: both tables are now listed in `DIALOG_PATCH_TABLES`, and the guard tests `the_castle_table_is_registered_for_serving` and `the_cellblock_table_is_registered_for_serving` cover them (see `RESUME.md`).
4. **`find_path` caps at 256 polys.** Navigation tests therefore chain legs instead of asserting one long path. RuinNote 23021 was re-sealed for this reason.
5. **The Beta decisions D-UB4 to D-UB13 were all ruled at high confidence** (see [§ Decisions](#decisions)): the ring pair, stop-on-Beta tails, the Beta-local infirmary, twins out, xp/naq 0, the ID block, per-player binds, no phasing, the beam via `interact_tag` and `move_entity`, and archetype-5 replays.
6. **`missions.reward_xp` / `reward_naq` are never read** (2026-10-19, U-D07 research). Mission reward dispatch is unimplemented (#310: `chosenRewards` is a stub, `ON_MISSION_REWARDS_DISPLAY` has no sender), so editing those columns changes nothing. D-UB16 (Decided 2026-10-19) delivers rewards by reward chains (`grant_xp`, then `grant_cash`) instead and keeps the columns at 0, so #310 cannot pay twice.

### Documentation debt

- `docs/content/mission-chains.md` is over the 500-line soft cap and was already over the 700-line hard cap before UB-99; a split by zone is owed.
- The doc-update map has no row for per-zone content pages; this close-out used the "Game systems, content chains" row.

## Decisions

Wording of D-UB1 to D-UB15 stays in the index § Decisions. This table tracks status, and for the ARCH-B1 rulings the reason and the confidence. The user was asleep; the orchestrator told the architect to rule at high confidence and ledger each ruling. Owner-only questions stay BlockedDecision.

| ID | Subject | Status | Ruling, reason, confidence | Blocks |
|---|---|---|---|---|
| **W-UB1** | **Owner waiver of the U-D03 / FZ-99 start gate for Beta.** The user waived the gate for the whole Beta campaign (wave B0 docs and onward). The Cellblock, Castle, Harset and Tollana ledgers were **not independently verified** by UB-00c or by this waiver; the packets rely on the waiver, not on a check of those ledgers. | **Waived by the user** (2026-10-18) | - | none; replaces "BlockedDependency (U-D03)" on UB-00a1/a2/b/c |
| D-UB1 | Race gates: Human `archetype gte 1` and `lte 4`; Free Jaffa `eq 7`; Asgard `eq 5` | Decided | - | all chain packets |
| D-UB2 | Beta fires the arrival steps 1423 S4223, 1500 S4369, 481 S1760 | Decided (Omega D-UO2 agrees); Asgard leg HELD behind UO-16 | - | UB-10, UB-20, UB-30 |
| D-UB3 | Beta never offers 1423, 1500 or 481 | Decided | - | all chain packets |
| D-UB4 | West-valley access for the Nine Peaks camp (ring pair, walk route, or hold) | **Decided A: ring pair 2300/2301** (ARCH-B1) | UB-00b found no walk route: the camp (nav 70) and the hub (nav 251) are not connected. The cooked map ships a real ring prefab (`GLB-RingTransporter00`) with its matinee at SW_RING, so a server-only `ring_transport_regions` pair linking hub ring 148 to SW_RING needs no client patch. B (walk) is impossible; C (hold) would drop 1502 and the camp for no gain. **High** | UB-02d, UB-21, UB-40 camp rows (all unblocked) |
| H-RING | Owner question "ring the camp or hold it" | **Answered by D-UB4 A** (ARCH-B1) | Same evidence as D-UB4. If the owner later prefers hold, drop UB-02d's ring rows and UB-21; nothing else depends on them. **High** | - |
| D-UB5 | Offworld tails (A stop, B Omega/SGC receivers, C authored relays) | **Decided A: tails stop on Beta** (ARCH-B1) | A is the planned default, needs no sibling receiver and no authored content, and keeps Beta shippable alone. B needs UB-51 notes that Omega/SGC have not planned; C needs UB-50, which is owner-held. UB-22 is not dispatched, UB-51 is held. Either can be reopened by a later campaign without undoing A. **High** | UB-22 (Hold), UB-51 (Hold) |
| D-UB6 | 421 infirmary is Beta-local | **Confirmed** (ARCH-B1) | UB-00b probed INFIRMARY_CENTRE (961.300, 15.97, 284.900), radius 10, on the Beta navmesh. The brief's hold was only "confirm against the seal", and the seal agrees. **High** | UB-32 (unblocked) |
| D-UB7 | FV twins 383, 384, 1146 | **Decided: out** (ARCH-B1) | They are not in the P4 first-visit rows 121-124, no first-visit row needs them, and they would need a fresh step matrix. UB-52 closed; templates 2303, 2327 and 2336 stay reserved. **High** | UB-52 (Closed) |
| D-UB8 | Reward amounts | **Decided: xp 0, naq 0, no reward groups** (ARCH-B1) | UB-00a2 Q11: all 17 Beta missions carry xp/naq 0 and no reward groups in the seed. The packets keep the seed as is; real amounts are U-D07, an owner call held with UB-50. **High**. **Amended 2026-10-19** by U-D07 Approved: the missions pay the [approved scale](reward-proposal.md) through the reward chains (D-UB16); the `missions.reward_xp` / `reward_naq` columns stay 0 and there are still no reward groups | UB-50a, UB-50b1, UB-50b2 deliver the amounts |
| D-UB9 | ID block: world-prefix ranges plus 64000-64499 authored | **Approved by the coordinator (the orchestrator), applied by ARCH-B1** | The orchestrator's brief approves the amendment. UB-00c's re-check was clear for every planned id; ARCH-B1 found that the authored screens sub-range 64310-64379 holds four stock screens and re-carved it to 64124-64193 (below). The `id-allocation.md` row is added in this commit. **High** | UB-01a (unblocked) |
| D-UB10 | Per-player binds only | Decided | - | UB-02a and chain packets |
| D-UB11 | No phasing | Decided | - | all |
| D-UB12 | Asgard beam via `interact_tag` and `move_entity` | Decided; destinations from UB-00b (BEAM_DECK_DEST "698.98,157.93,70.36", BEAM_GROUND_DEST "745.05,-19.60,68.75") | - | UB-30 |
| D-UB13 | Asgard packets replay-tested with archetype 5 | Decided. UB-30 to UB-36 are dispatchable without the Asgard producer | - | UB-30 to UB-36 |
| D-UB14 | Authored content only in UB-50 | Decided | - | UB-50 |
| D-UB15 | Strict linear offer order per race | Decided (HELD-B1 kept the order) | - | UB-45a, UB-45b |
| D-UB16 | Reward delivery: one `grant_xp` reward chain per mission on `mission_completed` (chains 23480-23496, `beta_e1_rewards_chains.sql`); `missions.reward_xp` / `reward_naq` stay 0; naquadah needs a new `grant_cash` content action (UB-50b1), then one `grant_cash` row per reward chain (UB-50b2) | **Decided** 2026-10-19 (architect proposal, accepted with U-D07) | Finding 6: the columns are never read (#310), and `grant_xp` already works. Hanging off `mission_completed` pays on every completion path and keeps rewards in one droppable file (D-UB14). Columns at 0 prevent double pay when #310 lands. **High**; the amounts are the owner's (U-D07 Approved) | UB-50a, UB-50b1, UB-50b2 |

### Owner decisions, 2026-10-19

The orchestrator relayed these owner decisions. The architect records them; it did not ask the user.

| ID | Decision | Status | Effect on Beta |
|---|---|---|---|
| H-LBL | Player-facing CIMMERIA_AUTHORING copy carries **no visible label**. Provenance stays in seed comments and evidence labels only. Shared policy for every Union campaign (Beta, Omega UO-15, Dakara UD-01a/c): [union-zone-planning § Shared policy](../union-zone-planning/README.md#shared-policy-all-union-campaigns) | **Decided** (owner) | UB-50's relay copy, if ever revived, has no label. UB-50a/b1/b2 have no player-facing copy |
| U-D07 | Determine level-appropriate xp and naq for the 17 Beta missions and for authored content generally; evidence first, otherwise a labelled reconstruction for approval | **Decided: U-D07 Approved 2026-10-19** (owner), both XP and naq. No formula evidence exists; the owner approved the [reward-proposal.md](reward-proposal.md) scale unchanged (XP = 50 x level x difficulty factor, naq = 10 x level x factor, CIMMERIA_RECONSTRUCTION). It applies to authored content generally, with non-mission side activity at 50%: the shared-policy row in [union-zone-planning § Shared policy](../union-zone-planning/README.md#shared-policy-all-union-campaigns) is the authority for other campaigns. D-UB8 amended, D-UB16 Decided | UB-50a and UB-50b1 Ready; UB-50b2 BlockedDependency on both |
| U-D08 / U-D11 | The Asgard extension uses a **remote Council hearing after Pertho's evacuation** and a **databank-recovery prologue that issues item 3399** (the README direction in [asgard-post-pertho.md](../union-zone-planning/asgard-post-pertho.md)) | **Decided** (owner confirmed) | HELD-B16 is no longer an owner question. It is **BlockedDependency** on Omega expanding and building UO-16. No Beta code changes: UB-30 to UB-36 already expect 481 S1760 with item 3399 (D-UB13) |

### Omega lines still open on U-D08/U-D11 (for Omega's coordinator)

Beta does not edit another campaign's ledger. These lines still list U-D08/U-D11 (and UO-15's label question) as open and must be updated by Omega's coordinator (line numbers as of `origin/main` on 2026-10-19):

| File (under `docs/analysis/union-zone-planning/`) | Lines | Says now |
|---|---|---|
| `omega-work-packets.md` | 17 | "The Asgard route (1522 and 481) stays a stub until the owner rules on U-D08/U-D11." |
| `omega-work-packets.md` | 86, 220 | D-UO18 BlockedDecision, folds into U-D08 / UO-16 |
| `omega-work-packets.md` | 152, 164, 191 | Asgard 481 S1760 item 3399 HELD; UO-16 stub; UO-16 row `BlockedDecision (U-D08, U-D11)` |
| `omega-work-packets.md` | 222, 321 | U-D08 / U-D11 owner row; "holds still open" |
| `omega-work-packets.md` | 190, 214, 215, 300 | H-LBL / U-D07 holds. Both are now answered: no label (H-LBL), and U-D07 Approved 2026-10-19 (the shared formula, side activity at 50%) |
| `omega-packets/UO-16.md` | 3, 11, 36-37 | Status BlockedDecision U-D08, U-D11; "depends on U-D08 and U-D11 answered"; the two questions |
| `omega-packets/UO-15.md` | 3, 11, 15, 26, 27 | Label hold; line 27 proposes a "[Cimmeria]" suffix, now answered "no label" |
| `omega-packets/RESUME.md` | 7-8 | Open owner decisions |
| `omega-packets/UO-99.md` | 60-61 | "Asgard route: blocked on U-D08/U-D11" |
| `omega-packets/handoff-sgc-beta.md` | 22, 36 | UO-16 HELD on U-D08/U-D11; label hold |

Dakara lines on H-LBL and U-D07 (for Dakara's coordinator): `dakara-packets/UD-01a.md` 34-35 ("Copy is HELD (H-LBL)", "Rewards are HELD (U-D07)"; both are answered: no label, and the U-D07 shared formula applies), `dakara-packets/UD-01bc.md` 10, `dakara-packets/RESUME.md` 30.

U-D07 for Omega and Dakara: their reward rows (Omega UO-15, Dakara UD-01a/c) list a table computed from the shared formula at each mission's own level and difficulty, side activity at 50%. They need no further owner approval for the scale; a reward needs a `grant_cash` action, which UB-50b1 adds to the engine for everyone.

### Owner questions (BlockedDecision)

None open for Beta's packets. U-D07 (approval and the general rule) was answered on 2026-10-19. HELD-B16 is BlockedDependency on Omega UO-16, not an owner question. The only owner gate left is lab approval for UB-90.

## ID re-check (UB-00c step 1)

Checked against `origin/main` at `102d15444d15ef8f8113f3edcaa8affd4df69cd8` (fetched 2026-10-18; the packet worktree was cut from `b4a027d67`, an ancestor), every other remote branch, every local branch and every sibling worktree.

**Method.** The three literal commands in the packet spec match only single-line `VALUES (N,` rows, and the seeds also use `VALUES (N,` on its own line and bare `(N,` continuation rows. So the scan used a wider pattern, a superset of the spec's: a tuple head `(N,` with no word character, digit or dot before the paren, where N is `23[0-4]dd` or `64[0-4]dd` for the 5-digit ranges and `23[0-4]d` for the template, respawner and ring ranges.

| Scope | Count | How it was scanned | Hits |
|---|---:|---|---:|
| `origin/main` working tree of seeds | 1 | `git grep` over `db/resources` | 0 |
| Remote branches, all `origin/*` refs (including the 30-day set of 780) | 1047 refs, 150 distinct `db/resources` trees | one `git grep` per distinct tree; refs sharing a tree were not repeated | 0 |
| Local branches (`refs/heads`) | 185 distinct trees | same | 0 |
| Sibling worktrees (`git worktree list`) | 188 | `git grep --untracked` of the working directory, so uncommitted seed edits count | 0 |

Scoping notes.

- The 5-digit scan excluded `effect_nvps.sql` and `texts.sql`. Their 23xxx values are effect and text ids, not chain, spawn or point ids. Texts were checked separately for the one range that applies to them, 64460-64499: 0 hits.
- The 4-digit scan covers `*entity_templates*.sql`, `*respawners*.sql` and `*ring_transport*.sql`. Other 23xx ids (abilities, items, dialogs, missions, sequences, effects) belong to other tables and do not collide.
- Sanity check that the pattern sees rows: it finds `(8001,` in the Dakara chains, `(440,` in `entity_templates.sql`, and the highest ids in use are template 1595 and chain 60100.
- The cooked-client dialog `23080` named in `docs/analysis/dialog-ui-redesign/worknotes/du02a.md` is a client file id, not a seed row; the other 23xxx/64xxx hits in `docs/` are hash fragments, coordinates (`na40-interp-actor-decisions.tsv`) and handoff-pack prose. None is a planned seed id.

### Per range

| Table | Range | Owner | Result |
|---|---|---|---|
| content_chains | 23000-23499 | Union Beta E1 | **clear** |
| spawnlist | 23000-23199 | Union Beta E1 | **clear** |
| point_sets | 23000-23099 | Union Beta E1 | **clear** |
| point_set_points | 23000-23299 | Union Beta E1 | **clear** |
| entity_templates | 2300-2349 | Union Beta E1 | **clear** |
| respawners | 2300-2309 (new rows; 6 and 7 are re-authored in place) | Union Beta E1 | **clear** |
| ring_transport_regions | 2300-2301 used (D-UB4 = A); 2302-2309 reserved | Union Beta E1 | **clear** |
| authored: missions, steps, objectives, tasks, dialogs, dialog_sets, buttons, dialog_set_maps, texts | 64000-64499 | Union Beta E1 (UB-50 only, HELD) | **clear** |
| authored: dialog_screens | 64124-64193 (re-carved by ARCH-B1; was 64310-64379) | Union Beta E1 (UB-50 only, HELD) | **clear** after the re-carve |

### Screens re-carve (ARCH-B1)

The UB-00c scan looked for `(N,` at the tuple head. `dialog_screens.sql` is the one authored table whose id sits in the **second** column (`dialog_id, screen_id`), so the scan read the dialog id and missed four stock screens inside the old screens sub-range 64310-64379:

| Screen | Stock dialog |
|---|---|
| 64341 | 2002 |
| 64373, 64374, 64375 | 3122 |

ARCH-B1 re-carved Beta's screens to **64124-64193**, rescanned on the second column and found it free (the only textual hits are pg_dump OID comments, not rows). `id-amendment-request.md`, the index ID block and UB-50 carry the new range. No packet before UB-50 authors a screen, so nothing dispatched changes.

**Sibling risk (reported, not fixed here).** The same column-2 blind spot affects the sibling authored blocks. `dialog_screens.sql` already holds 26 stock screens inside them:

- Dakara 63500-63799 (11): 63509, 63510, 63511, 63512, 63513, 63517, 63518, 63644, 63645, 63646, 63647.
- Omega 63800-63999 (15): 63847-63852, 63854, 63874, 63881-63887.

Beta's docs do not own those ranges; the Dakara and Omega architects should re-carve or skip those ids before they author screens.

### Sibling plans

Read: `omega-work-packets.md`, `sgc-work-packets.md`, `dakara-work-packets.md` and the Omega, SGC and Dakara UO-00 / UD-00 / US-00 packet files, plus the SGC continuation ledger.

| Sibling | Its block | Relation to Beta |
|---|---|---|
| Dakara (D-UD6) | 63500-63799 every table; chains 8001-8499; templates 440-479; spawns 8000-8099; point sets 2123-2199; respawners 611-619 | disjoint. Dakara point sets 2123-2199 are a different table from Beta templates 2300-2349 |
| Omega (D-UO1) | 63800-63999 every table; item_list_items 638000-639999; respawner 180 | disjoint. Omega cites Beta's `23020`, `23220`, `23310` as receiver chains and `23001` as the way-back chain; these are Beta's, not Omega's |
| SGC (D-US1) | chains 3051-3199; spawns 15800-15849; point sets 15800-15819; points 15800-15899; templates 5800-5809; dialogs 58000-58049 | disjoint. `sgc-work-packets.md` § ID allocation already lists Beta's block |
| SGC's old list | Omega 64000-64199; point sets 2300-2329 | superseded by the SGC and Omega planning review. The old point-set range 2300-2329 overlapped Beta's template and respawner ranges only in documentation (different tables) |
| Tollana, Castle, Cellblock, Harset | `id-allocation.md` § reserved blocks: 1366-1400, 6001-6800, 9000-9999, 10000-11499, 11500-12999 | disjoint. The 23000+ and 64000+ ranges are above every reserved block. These ledgers were **not independently verified** (W-UB1); only their branch seeds were scanned |

Beta's authored 64000-64499 sits next to Dakara 63500-63799 and Omega 63800-63999 with no gap and no overlap.

The M3 and M5 labels were never defined in the planning folder. ARCH-B1 retired them: the index F17 now says so, and any new mismatch takes M10 or higher. See [receiver-contract.md](receiver-contract.md).

## Packet table

Status vocabulary (as of UB-99 every B-wave row is Done, UATPending, Hold or Closed; the reward waves R1/R2 were added 2026-10-19): Ready / BlockedDependency / BlockedDecision / Writing / Review / Integrated / UATPending / Done, plus Hold and Closed for packets the rulings removed. Mirrors the index § Packets as of UB-99. Worktree = branch = the name shown, based on `uzp-e1` unless the note says stacked.

| Wave | Packet | Worktree / branch | Status | Note |
|---|---|---|---|---|
| B0 | UB-00a1 | (concurrent editor) | Done (integrated; UB-00a1 pass 2 landed in `54e76cbbd`) | mission seal Q1-Q6; R3 finding C6, already folded into UB-DP, UB-31, UB-33 |
| B0 | UB-00a2 | - | Done (integrated) | mission seal Q7-Q14; a2 changes 1-6 folded |
| B0 | UB-00b | - | Done (integrated) | placement seal; anchors inlined in UB-02b/c/d, UB-30, UB-40 |
| B0 | UB-00c | - | Done (integrated) | ledger, ID re-check, receiver contract, amendment request |
| B1a | UB-01a | `uzp-ub01a` | Done (merged into `uzp-e1`, reviewed) | gate 6 pin, respawners 6/7, exemption removed |
| B1a | UB-02a | `uzp-ub02a` | Done (merged into `uzp-e1`, reviewed) | templates 2300-2335 (2303, 2327, 2336 reserved) |
| B1a | UB-DP | `uzp-ubdp` | Done (merged into `uzp-e1`, reviewed) | dialog patches 1997/3228 (C5), 1916/3905 stripped (C6) |
| B1b | UB-01b | `uzp-ub01b` | Done (merged into `uzp-e1`, reviewed) | UB-01a |
| B1b | UB-02b | `uzp-ub02b` | Done (merged into `uzp-e1`, reviewed) | UB-02a |
| B1b | UB-02c | `uzp-ub02c` (stacked on `uzp-ub02b`) | Done (merged into `uzp-e1`, reviewed) | UB-02b |
| B1b | UB-02d | `uzp-ub02d` (stacked on `uzp-ub02b`) | Done (merged into `uzp-e1`, reviewed) | UB-02b; ring pair 2300/2301 (D-UB4 A) |
| B2 | UB-10 | `uzp-ub10` | Done (merged into `uzp-e1`, reviewed) | B1b merged |
| B2 | UB-11 | `uzp-ub11` (stacked on `uzp-ub10`) | Done (merged into `uzp-e1`, reviewed) | UB-10 |
| B2 | UB-12 | `uzp-ub12` (stacked on `uzp-ub11`) | Done (merged into `uzp-e1`, reviewed) | UB-11 |
| B2 | UB-13 | `uzp-ub13` (stacked on `uzp-ub12`) | Done (merged into `uzp-e1`, reviewed) | UB-12 |
| B2 | UB-20 | `uzp-ub20` | Done (merged into `uzp-e1`, reviewed) | B1b merged |
| B2 | UB-30 | `uzp-ub30` | Done (merged into `uzp-e1`, reviewed) | end-to-end reachability waits on HELD-B16 (owner), not on the packet |
| B3 | UB-14 | `uzp-ub14` | Done (merged into `uzp-e1`, reviewed) | UB-13 (HELD-B12 answered) |
| B3 | UB-15 | `uzp-ub15` (stacked on `uzp-ub14`) | Done (merged into `uzp-e1`, reviewed) | UB-14 (camera item 4832) |
| B3 | UB-16 | `uzp-ub16` (stacked on `uzp-ub15`) | Done (merged into `uzp-e1`, reviewed) | UB-15 |
| B3 | UB-21 | `uzp-ub21` | Done (merged into `uzp-e1`, reviewed) | UB-20, UB-02d |
| B3 | UB-31 | `uzp-ub31` | Done (merged into `uzp-e1`, reviewed) | UB-30, UB-DP |
| B3 | UB-40 | `uzp-ub40` | Done (merged into `uzp-e1`, reviewed) | B1b merged |
| B4 | UB-32 | `uzp-ub32` | Done (merged into `uzp-e1`, reviewed) | UB-31 (D-UB6 confirmed) |
| B4 | UB-33 | `uzp-ub33` (stacked on `uzp-ub32`) | Done (merged into `uzp-e1`, reviewed) | UB-32, UB-DP |
| B4 | UB-34 | `uzp-ub34` (stacked on `uzp-ub33`) | Done (merged into `uzp-e1`, reviewed) | UB-33 |
| B4 | UB-35 | `uzp-ub35` (stacked on `uzp-ub34`) | Done (merged into `uzp-e1`, reviewed) | UB-34, UB-DP |
| B4 | UB-36 | `uzp-ub36` (stacked on `uzp-ub35`) | Done (merged into `uzp-e1`, reviewed) | UB-35, UB-DP |
| B5 | UB-45a | `uzp-ub45a` | Done (merged into `uzp-e1`, reviewed) | UB-16, UB-21, UB-36, UB-40 |
| B5 | UB-45b | `uzp-ub45b` (stacked on `uzp-ub45a`) | Done (merged into `uzp-e1`, reviewed) | UB-45a |
| B6 | UB-90 | - (lab) | UATPending: the owner approves lab use; no row has run | rows UB-U1 to UB-U10; B5 merged; ask the user before any lab use |
| B6 | UB-99 | `uzp-ub99` | Done (this close-out) | close-out docs, this commit |
| - | UB-22 | - | Hold: not dispatched (D-UB5 A) | not dispatched (D-UB5 A) |
| R1 | UB-50a | `uzp-ub50a` (from `origin/main`) | Ready | XP reward chains 23480-23496, [UB-50.md](../union-zone-planning/beta-packets/UB-50.md); CB-N05a does not block it (no new mission) |
| R1 | UB-50b1 | `uzp-ub50b1` (from `origin/main`) | Ready | `grant_cash` content action (code); no file shared with UB-50a |
| R2 | UB-50b2 | `uzp-ub50b2` (from `origin/main` after R1 merges) | BlockedDependency (UB-50a and UB-50b1 merged) | one `grant_cash` row per reward chain, tests extended |
| - | UB-50 relays | - | Hold (D-UB5 A) | H-LBL no longer blocks; chain range shrinks to 23497-23499 plus a grant |
| - | UB-51 | - | Hold (D-UB5 A) | D-UB5 A |
| - | UB-52 | - | Closed (D-UB7: out) | D-UB7 out |
| - | UB-L | - | Hold | Visit 2 / E2 research note |

Waves run in order; inside a wave the listed lanes run in parallel. B1a has three independent lanes. UB-DP may merge after the rest of B1a, but it must be merged before B3 starts. B2 has three lanes (Human stack, Free Jaffa, Asgard), B3 four (Human stack, UB-21, UB-31, UB-40), B4 one stack. Each wave gets one packet-review-fixer. The reward waves are cut from `origin/main` (Beta is merged, #1916) and each packet ships as its own PR: R1 runs UB-50a and UB-50b1 in parallel, then one packet-review-fixer for both; R2 runs UB-50b2 alone once both R1 PRs have merged, then its own packet-review-fixer.

UB-45b now has its own row in the index § Packets (debt from UB-00c, resolved by ARCH-B1).

## Worknotes

Per-packet worknotes go in [worknotes/](worknotes/README.md), one file per packet, named `ub-<id>.md` (for example `ub-00b.md`). A worknote records what the packet measured or decided that a later packet needs and that is not in the packet's own commit.
