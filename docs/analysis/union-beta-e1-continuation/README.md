# Union Beta E1 continuation ledger

> **Type:** Reference (campaign ledger)
> **Audience:** The Union Beta coordinator and the packet coders, reviewers and specialists named in the packet table
> **Last updated:** 2026-10-18 (UB-00c)
> **Index:** [beta-work-packets.md](../union-zone-planning/beta-work-packets.md) (packet text, findings F1-F21, HELD list and waves; this ledger does not copy them)
> **Companions:** [RESUME.md](RESUME.md), [receiver-contract.md](receiver-contract.md), [id-amendment-request.md](id-amendment-request.md), [worknotes/](worknotes/README.md), [execution contract](../union-zone-planning/execution-contract.md), [SGC ledger](../union-sgc-continuation/README.md) (the model for this one)

## Purpose

Track the Beta_Site_Evo_1 (world 23, gate 6) first-visit campaign packet by packet: which packets are ready, which are blocked and why, which decisions are open, and what UB-00c verified before any seed or code packet starts. UB-00c changes no seed, no code and not `id-allocation.md`. It records the ID re-check, the frozen Omega receiver contract and the amendment request.

## Status of this ledger

- UB-00c (this commit): **Done** when the ID re-check reports `clear` for every range. It does (see [§ ID re-check](#id-re-check-ub-00c-step-1)), so UB-01a is not stopped by a conflict.
- UB-01a still waits for the **coordinator** to merge the D-UB9 amendment request ([id-amendment-request.md](id-amendment-request.md)) into `docs/analysis/four-zone-planning/id-allocation.md`.
- Nothing else is dispatched by this commit.

## Decisions

Wording of D-UB1 to D-UB15 stays in the index § Decisions. This table tracks status only. Statuses are copied from the index as of this commit; the SGC ledger style is followed.

| ID | Subject | Status | Blocks |
|---|---|---|---|
| **W-UB1** | **Owner waiver of the U-D03 / FZ-99 start gate for Beta.** The user waived the gate for the whole Beta campaign (wave B0 docs and onward). The Cellblock, Castle, Harset and Tollana ledgers were **not independently verified** by UB-00c or by this waiver; the packets rely on the waiver, not on a check of those ledgers. | **Waived by the user** (2026-10-18) | none; replaces "BlockedDependency (U-D03)" on UB-00a1/a2/b/c |
| D-UB1 | Race gates: Human `archetype gte 1` and `lte 4`; Free Jaffa `eq 7`; Asgard `eq 5` | Decided | all chain packets |
| D-UB2 | Beta fires the arrival steps 1423 S4223, 1500 S4369, 481 S1760 | Decided (Omega D-UO2 agrees); Asgard leg HELD behind UO-16 | UB-10, UB-20, UB-30 |
| D-UB3 | Beta never offers 1423, 1500 or 481 | Decided | all chain packets |
| D-UB4 | West-valley access for the Nine Peaks camp (ring pair, walk route, or hold) | **BlockedDecision** (owner) | UB-02d, UB-21, UB-22, UB-40 camp rows |
| D-UB5 | Offworld tails (A stop, B Omega/SGC receivers, C authored relays) | **BlockedDecision** (owner); A ships unless B or C is picked | UB-22, UB-50, UB-51 |
| D-UB6 | 421 infirmary is Beta-local | Decided (orchestrator confirms against the brief's hold) | UB-32 |
| D-UB7 | FV twins 383, 384, 1146 (recommend out) | **BlockedDecision** (HELD-B6) | UB-52 |
| D-UB8 | No reward amounts until U-D07 | **BlockedDecision** | UB-50 |
| D-UB9 | ID block: world-prefix ranges plus 64000-64499 authored | **Pending**: re-check passed (below); the coordinator merges the amendment | UB-01a |
| D-UB10 | Per-player binds only | Decided | UB-02a and chain packets |
| D-UB11 | No phasing | Decided | all |
| D-UB12 | Asgard beam via `interact_tag` and `move_entity` | Decided (structure); destinations from UB-00b | UB-30 |
| D-UB13 | Asgard packets replay-tested with archetype 5 | Decided | UB-30 |
| D-UB14 | Authored content only in UB-50 | Decided | UB-50 |
| D-UB15 | Strict linear offer order per race | Decided (HELD-B1 can reorder) | UB-45a, UB-45b |

Open owner questions that are not decisions yet: H-LBL (label policy), H-RING (= D-UB4), HELD-B16 (which Omega packet delivers an Asgard player at S1760 with item 3399; UO-16 is HELD on U-D08/U-D11).

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
| ring_transport_regions | 2300-2309 (only if D-UB4 = A) | Union Beta E1 | **clear** |
| authored: missions, steps, objectives, tasks, dialogs, dialog_sets, screens, buttons, dialog_set_maps, texts | 64000-64499 | Union Beta E1 (UB-50 only, HELD) | **clear** |

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

Debt noted: the index uses the labels M1 to M9 but names only M1, M2, M4, M6, M7, M8 and M9. M3 and M5 are not defined anywhere in the planning folder. See [receiver-contract.md](receiver-contract.md).

## Packet table

Status vocabulary: Ready / BlockedDependency / BlockedDecision / Writing / Review / Integrated / UATPending / Done. Copied from the index § Packets as of this commit, with the W-UB1 waiver applied to the former "BlockedDependency (U-D03)" rows. "Ready (waiver)" means the start gate is lifted; this ledger does not verify whether the packet is already running or merged.

| Wave | Packet | Status | Note |
|---|---|---|---|
| B0 | UB-00a1 | Ready (waiver) | mission seal Q1-Q6 |
| B0 | UB-00a2 | Ready (waiver) | mission seal Q7-Q14 |
| B0 | UB-00b | Ready (waiver) | placement seal, nav probes |
| B0 | UB-00c | Writing (this commit) | ledger, ID re-check, receiver contract, amendment request |
| B0 | UB-01a | BlockedDependency | UB-00c merged and the coordinator's ID row |
| B1 | UB-01b | BlockedDependency | UB-01a |
| B1 | UB-02a | BlockedDependency | UB-00a2 |
| B1 | UB-02b | BlockedDependency | UB-02a, UB-00b |
| B1 | UB-02c | BlockedDependency | UB-02a, UB-00b |
| B1 | UB-02d | BlockedDecision | D-UB4 |
| B2 | UB-10 | BlockedDependency | UB-02b, D-UB2 (Decided) |
| B2 | UB-11 | BlockedDependency | UB-10 |
| B2 | UB-12 | BlockedDependency | UB-11 |
| B2 | UB-13 | BlockedDependency | UB-12 |
| B2 | UB-20 | BlockedDependency | UB-02b, D-UB2 (Decided) |
| B2 | UB-30 | BlockedDependency | UB-02b/c; HELD on the Asgard producer UO-16 (U-D08/U-D11) |
| B3 | UB-14 | BlockedDependency | UB-13, HELD-B12 |
| B3 | UB-15 | BlockedDependency | UB-14, HELD-B9 |
| B3 | UB-16 | BlockedDependency | UB-15 |
| B3 | UB-21 | BlockedDependency | UB-02d, UB-20 |
| B3 | UB-22 | BlockedDecision | D-UB5 (not dispatched under A) |
| B3 | UB-31 | BlockedDependency | UB-30 |
| B4 | UB-32 | BlockedDependency | UB-31, D-UB6, HELD-B3 |
| B4 | UB-33 | BlockedDependency | UB-32 (stack) |
| B4 | UB-34 | BlockedDependency | UB-33 (stack), FireSite 23031 |
| B4 | UB-35 | BlockedDependency | UB-34 (stack), HELD-B15 |
| B4 | UB-36 | BlockedDependency | UB-35 (stack) |
| B4 | UB-40 | BlockedDependency | UB-02a/b/c, UB-00b Life anchors |
| B5 | UB-45a | BlockedDependency | UB-16, UB-21, UB-36 (UB-22 if dispatched) |
| B5 | UB-45b | BlockedDependency | UB-45a |
| B5 | UB-50 | BlockedDecision | D-UB5 C, D-UB8, H-LBL, CB-N05a |
| B5 | UB-51 | BlockedDecision | D-UB5 |
| B5 | UB-52 | BlockedDecision | D-UB7 |
| B6 | UB-90 | UATPending | lab; asks the user first |
| B6 | UB-99 | BlockedDependency | close-out |
| - | UB-L | Hold | Visit 2 / E2 research note |

UB-45b has no row in the index § Packets table (it appears in the wave table and in UB-40-52.md). Documentation debt: the architect should add it to the index.

## Worknotes

Per-packet worknotes go in [worknotes/](worknotes/README.md), one file per packet, named `ub-<id>.md` (for example `ub-00b.md`). A worknote records what the packet measured or decided that a later packet needs and that is not in the packet's own commit.
