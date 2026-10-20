# HA-00c worknote: static audit result

> Type: packet result (docs-only audit). Run 2026-10-18 against campaign tip `3e5f503e861fc39d37526732f2846adfbc2bf053` (branch `content/harset-campaign`, worktree `zone-harset`). Executor: mission-systems advisor (Sonnet). Nothing was allocated, no seed or code was edited, no lab, client or cargo was used. Parent contract: [four-zone HA-00c](../../four-zone-planning/harset-work-packets.md). Precedent: [HA-00b](HA-00b.md). Ledger: [work-packets](../work-packets.md), [id-allocation](../id-allocation.md).

**Verdict for the static half: complete, zero collisions.** HA-00c as a whole stays open: Cellblock's first authored quest client proof is BlockedEvidence and the premise, reward and reset decisions and all UAT stay owner-manual. HA-17a/b/c and HA-18a/b/c stay BlockedDependency.

## 1. Scope split

HA-00c reserves authored and repeatable mission IDs. It cannot finish without the lab, because the first authored quest needs a client proof. The part that needs no client is executed here. No ID is allocated by the static half: no numeric-gap picks, no census, carry-in or 1664 reuse, and no row is added to the claim tables.

| Part | Status after this result |
|---|---|
| Stock catalog inspection (committed pak) | Done, section 3 |
| Current active-branch collision audit | Done, sections 4 to 6 |
| Mission graph signoff list (nine graphs plus shared claims) | Listed, section 7; signatures stay with the signers and are blank |
| First authored quest client proof (journal, dialog, advance, relog) | BlockedEvidence: owner-run lab UAT; not claimed |
| Premise, reward and reset decisions within O14-04, O14-10, O14-13 | Owner-manual; not claimed |
| HA-17 and HA-18 | BlockedDependency on HA-00c |

## 2. Packet contract (static half)

| Item | Value |
|---|---|
| Executor | Sonnet or mission advisor. Not Haiku: it reads a binary container and interprets collisions |
| Branch / worktree | `harset/ha-00c-authored-ids` / `harset-ha-00c` (sealed name; this run was executed in `zone-harset` on the campaign branch at the owner's direction) |
| Commit subject | Sealed as `Harset HA-00c: seal authored and repeatable IDs (static half)`; this run committed `Harset HA-00c: record static audit results` |
| Base | Campaign tip at dispatch: `3e5f503e861fc39d37526732f2846adfbc2bf053`. Evidence base of the ledger: `d3efb7d41aad0b6008397f112d224ce7430d7acf` |
| Deliverable | This file (the seal was replaced by this result; the contract table is kept) |

Allowlist (docs only): this file, `id-allocation.md`, `work-packets.md`, `README.md`, `RESUME.md` in `docs/analysis/harset-continuation/`. Scratch scripts and outputs lived in the session scratchpad, not in the repo. No cargo, lab, client, seed, SQL or `db/scripts`.

Stop conditions (none fired): reader check mismatch; a collision found; a branch that cannot be read; a need to edit outside the allowlist.

## 3. Result: reader check and stock census

### 3.1 Reader check (gate; passed first)

Scratch reader over `data/cache/CookedDataMissions.pak` (a ZIP holding `MetaData` and `_<id>` entries, per [cooked-data-pak-format.md](../../../engine/cooked-data-pak-format.md)). Every [CB-N05a](../../four-zone-planning/cb-n05a-seal.md) number reproduced without tuning:

| Witness | Sealed value | Observed |
|---|---|---|
| SHA-256 | `9A96ED8278957CF37E337367AE592C8D8155A0901F1061CA0D3873D3AAA570EA` | identical |
| Size | 749028 bytes | 749028 |
| Numeric entries | 1040 (1041 total with `MetaData`) | 1040 numeric, 1041 total |
| Entry ID range | 38 to 1826 | 38 to 1826 |
| MetaData | 7538 | 7538 |
| Stock ID attributes | 12,906 | 12,906 (1040 + 3475 + 4033 + 4358) |
| Witness `_1624` | 1093 bytes, SHA-256 `9AFFDF20C17B238A8DE76B27001E7EFEA31D0EFE1B5AB5E141D723B859B0DE21` | identical |

### 3.2 Stock census (pak) against the seeds at the tip

Seed counts were recounted from `db/resources/` with the named-column parser (section 4.1), not copied. Gap runs are internal to [min, max] of the pak attribute.

| Kind | Pak count | Pak min..max | Gap runs | Free ints in gaps | Largest runs | Seed count | Seed max | In seed, not pak | In pak, not seed |
|---|---:|---|---:|---:|---|---:|---:|---|---|
| MissionID | 1040 | 38..1826 | 101 | 749 | 519-540, 1582-1602, 422-441, 1021-1040, 721-739 | 1041 | 1826 | 689 | none |
| StepID | 3475 | 89..5384 | 500 | 1821 | 969-1046, 2964-3022, 1846-1879, 165-197, 3196-3222 | 3480 | 80688 | 68900, 80622, 80623, 80641, 80688 | none |
| ObjectiveID | 4033 | 107..6404 | 646 | 2265 | 1154-1226, 2179-2217, 191-226, 3489-3516, 1826-1850 | 4037 | 90688 | 90622, 90623, 90641, 90688 | none |
| TaskID | 4358 | 97..8712 | 1011 | 4258 | 584-772, 1684-1843, 1151-1239, 1510-1582, 7096-7135 | 4358 | 8712 | none | none |

The ten seed-only rows (689, 68900, 80622, 80623, 80641, 80688, 90622, 90623, 90641, 90688) are real seed occupants the pak does not list. A pak-only gap scan would call them free; they are not. Any later pick must be checked against the seed as well as the pak.

Seed maxima for the other families (tip, `db/resources/`): `content_chains` 14095 (350 rows), `dialogs` 100011 (5418 rows), `dialog_set_maps` 1000000 (4674 rows), `items` 9014 (6074 rows), `entity_templates` 1595 (449 rows), `spawnlist` 632 rows with max spawn_id 14095 in the column scan, `point_sets` 101 rows (max 13818), `point_set_points` 202 rows (max point_id 13818).

Companion packs, compared with their seeds (scratch reader, same method):

| Pak | SHA-256 (first 16) | Entries | ID range | Seed rows | Seed max | In seed, not pak | In pak, not seed |
|---|---|---:|---|---:|---:|---|---|
| `CookedDataDialogs.pak` | `7F52B5AF989686E5` | 5405 | 11..6427 | 5418 | 100011 | 13: 3996, 60100-60104, 60110, 100006-100011 | none |
| `CookedDataItems.pak` | `CBEAE74024B2E8B1` | 6059 | 10..8951 | 6074 | 9014 | 15: 9000-9014 | none |

Dialog seed rows above 6427 are seed-only; item IDs 9000-9014 are seed-only. `dialog_set_maps` has no pak and its tip max is 1000000; the top five keys are 100007, 120001, 120002, 120101, 1000000 (120101 is the Harset DSM added since the base).

### 3.3 Tip against the evidence base

`d3efb7d41` to the tip adds exactly `content_chains` 1366 (the Castle-origin `stargate_crossed` reconstruction) and `dialog_set_maps` 120101 (the interaction-only bound added by HA-1360b1). Nothing was removed. Every other table's row count is unchanged from the base (349 chains to 350; 4673 DSM to 4674).

## 4. Result: claim tables

### 4.1 Method

Named-column parser (`INSERT INTO [resources.]<table> (<cols>) VALUES (...),(...);`): the key is read from the column named for the table (`point_set_points` uses `point_id`, the second column), quotes and `''`/backslash escapes are respected, `--` comments are skipped, a non-numeric key raises. Row hash is SHA-1 of the row text, used only to tell "same row" from "different row".

### 4.2 Merged claim table (authored and repeatable mission family)

| Claim | IDs | Owner / source | Status |
|---|---|---|---|
| 73 reviewed Harset missions | per [harset-73-mission-review.csv](../../four-zone-planning/harset-73-mission-review.csv) | Harset census | Existing stock; 73 distinct, all in pak and seed |
| Four carry-ins | 567, 1360, 1377, 1410 | Harset (O14-17/19 for 1360) | Existing stock; disjoint from the 73; 77 distinct total, all 77 in pak and seed |
| 1664 interface | 1664 | HA-01b (Castle accept trigger) | In pak (true) and seed (true); not in the 77; provisional, not allocated here |
| Test reservation | 61000 (missions), 61001 (step), 61002 (objective), 61003 (task) | CB-N05a 1139-byte fixture; `TEST_MISSION_ADDITIONS` in `crates/resources/src/base/mission_additions.rs` | Absent from every pak and seed mission-family key; `MISSION_ADDITIONS` is `&[]` in production |
| Catalog policy | root IDs 1 to 65535; 355840-byte cap | CB-N05a | Policy bound, not a reservation |
| Cellblock authored | missions 62050, 62060; steps 62051, 62054, 62061; objectives 62052, 62055, 62062; tasks 62053, 62056, 62063; dialogs 60150, 60151, 60160 | Cellblock branches (sibling claims) | Seen on branch snapshots only; the first authored quest in the 62xxx range; below the 65535 bound |
| Harset chain candidates | 6008-6032, 6131-6710 per-mission ranges, 6151-6165, 6531-6550 | [id-allocation.md](../id-allocation.md) | Inactive candidates; none of the 61 chain IDs added by any branch falls in a Harset range |
| Four-zone shared rows | see [four-zone id-allocation](../../four-zone-planning/id-allocation.md) | Coordinator | Unchanged by this packet |
| HA-17 / HA-18 | none, unallocated | this packet | Authored mission, step, objective and task IDs stay unallocated |

The only IDs this packet records are existing claims. No row was added to any claim table.

### 4.3 Owner delegations

O14-04 lets the coordinator allocate individual IDs after a stock catalog, shared table, seed and active-branch collision audit (this result is that audit, minus allocation). O14-10 and O14-11 delegate lore-safe premise and rewards; O14-13 delegates the persisted cooldown, reward and anti-farm design for the repeatables. These bound the work; none is decided here.

## 5. Result: frozen SHAs and branch collision scan

### 5.1 Frozen refs

`git worktree list --porcelain` enumerated **109 distinct registered branch refs** (119 worktree entries). Each was resolved with `git rev-parse` before any scan; the SHA map is kept in the scratchpad `frozen-shas.json`. The four campaign heads:

| Ref | Frozen SHA |
|---|---|
| `content/harset-campaign` (tip) | `3e5f503e861fc39d37526732f2846adfbc2bf053` |
| `content/castle-campaign` | `5177290c5ca5f027867e9f0c484930e3f01f95ab` |
| `content/cellblock-campaign` | `836e1209131dae8ec8ff238777681ffe9cc0d284` |
| `content/tollana-campaign` | `e57840443b32db40551f1214756b884eaeb58682` |

The worktree was clean at the start and end of the scan.

### 5.2 Scan result

Every `db/resources/**/*.sql` whose committed content differs from the tip on a ref was read through `git show <frozen-sha>:<path>` and parsed with the section 4.1 parser. Twelve key tables were covered: `content_chains`, `spawnlist`, `entity_templates`, `point_sets`, `point_set_points`, `missions`, `mission_steps`, `mission_objectives`, `mission_tasks`, `items`, `dialogs`, `dialog_set_maps`.

| Measure | Value |
|---|---:|
| Registered refs | 109 |
| Refs whose seed `.sql` differs from the tip | 105 |
| Files parsed | 959 |
| Unreadable parses | 0 |
| Duplicate keys inside any branch snapshot file | 0 |
| Keys added by a branch (not in the tip seed), distinct | `content_chains` 61, `spawnlist` 8, `entity_templates` 4, `dialogs` 3, `mission_steps` 3, `mission_objectives` 3, `mission_tasks` 3, `missions` 2 |
| Added keys with different row content on two or more refs (conflicting claims) | 0 |
| Added keys that hit a stock pak ID or a claim table (section 4.2) | **0** |
| Tip-key rows that a branch modifies (lineage skew, information only) | 792 |

Added mission-family keys, all on Cellblock branches: missions 62050, 62060; steps 62051, 62054, 62061; objectives 62052, 62055, 62062; tasks 62053, 62056, 62063; dialogs 60150, 60151, 60160. No branch adds `dialog_set_maps` or `items` rows. The first thirty of the 61 added chain IDs are 1026-1028, 1036, 1037, 1367-1383, 4001, 9001-9007; none lies in a Harset candidate range.

Reading the numbers: "105 differing" is measured against the tip, so it counts any ref that has not yet merged the newest Harset commits. The existing validator measures against the base and reports 78 (section 6). Both are correct for their reference point. The 792 modified tip rows are the same lineage skew: branches that predate tip merges carry older copies of rows, which is not a collision (a collision needs the same key with different content added by two sources, or an added key inside the stock or claim set).

### 5.3 Not covered

Uncommitted work in sibling worktrees, claims written after the SHA freeze, unregistered branches, and live database state. The coordinator must reconcile these at allocation time, as the [id-allocation](../id-allocation.md) caveat already says.

## 6. Result: existing validator, unchanged

`.claude/reports/four-zone/validate-harset.ps1` was read, then run unchanged. Output:

| Check | Result |
|---|---|
| Seed PK inventories | 12 tables, rows = distinct for all (350, 4674, 5418, 449, 6074, 4037, 3480, 4358, 1041, 202, 101, 632) |
| Seed PKs and candidate collisions | passed, 495 chain candidates |
| Census | passed: 73 rows, 73 unique, dispositions 29/39/2/3, 4 carry-ins disjoint |
| Scratch decision emptied (`mission_id=571`) | `expected_failure`: census disposition HOLD_OLD_REVISION |
| Scratch restored | passed on the same checker |
| Registered committed branch scan | passed, 109 refs, 78 differing refs, zero candidate collisions |

Against [HA-00b](HA-00b.md) (84 refs, 72 differing), the ref count grew by 25 and differing refs by 6. Zero collisions at both points.

## 7. Result: duplicate-key proof

Scratch fixtures only (copies of the tip seed tree in the scratchpad, never in the repo). The scan is `scan.py dir`, exit code 1 on any duplicate key.

| Fixture | Change | Output | Exit |
|---|---|---|---:|
| clean | tip copy, unmodified | `status=PASS no duplicate keys` (missions 1041/1041, point_set_points 186/186 in the fixture subset) | 0 |
| dup | one deliberate duplicate `mission_id=1360` and one duplicate `point_id=2257` | `status=FAIL duplicate keys {"point_set_points": [2257], "missions": [1360]}` (missions 1042 rows / 1041 distinct) | 1 |
| legalrepeat | `point_set_points` with a repeated `set_id` but distinct `point_id` | `status=PASS no duplicate keys` (187/187) | 0 |

The third row proves the second-column rule: a repeated `set_id` is legal and passes, a repeated `point_id` fails. The clean copy passes and the duplicate fails, as the seal required.

## 8. Result: mission graph signoff list

Nine graphs plus the coordinator's shared claims. HA-17a/b/c hold two stories each (six stories); HA-18a/b/c are the three repeatables. None is authored: there is no graph to sign, so every graph-specific cell reads as a precondition, not a design. The mission advisor signature cells are blank on purpose.

| # | Graph | Mission / steps / objectives / tasks | Objective primitives available | Reward path | Repeatable | Claimed IDs | Open question | Mission-advisor signature |
|---|---|---|---|---|---|---|---|---|
| 1 | HA-17a Jaffa story 1 | not authored | KillCount, CollectItem, VisitRegion, TalkToNpc, UseObject (server-side primitives; the client does not see the type taxonomy) | grant XP / cash / item via the existing grant actions; amounts owner-manual (O14-11) | no | none, unallocated | lore-safe premise (O14-10); catalog client proof; HA-01 placements | |
| 2 | HA-17a Jaffa story 2 | not authored | as above | as above | no | none, unallocated | as graph 1 | |
| 3 | HA-17b OP-CORE story 1 | not authored | as above | as above | no | none, unallocated | premise; HA-09/10 services | |
| 4 | HA-17b OP-CORE story 2 | not authored | as above | as above | no | none, unallocated | as graph 3 | |
| 5 | HA-17c Goa'uld story 1 | not authored | as above | as above | no | none, unallocated | premise; HA-13/14 services | |
| 6 | HA-17c Goa'uld story 2 | not authored | as above | as above | no | none, unallocated | as graph 5 | |
| 7 | HA-18a Jaffa repeatable | not authored | as above | persisted reward and anti-farm design (O14-13) | yes; the `repeats` UPSERT gap is a known trap and needs its own architect-approved arm | none, unallocated | cooldown persistence; HA-17a first | |
| 8 | HA-18b OP-CORE repeatable | not authored | as above | as graph 7 | yes | none, unallocated | cooldown persistence; HA-17b first | |
| 9 | HA-18c Goa'uld repeatable | not authored | as above | as graph 7 | yes | none, unallocated | cooldown persistence; HA-17c first | |
| S | Coordinator shared claims | n/a | n/a | n/a | n/a | existing claims only (section 4.2); none added | reconcile uncommitted sibling claims and the seed-only occupants before any pick | |

Persistence note for graphs 7 to 9: the `repeats` field is missing from the world-entry mission UPSERT (a confirmed engine bug), so a repeatable cannot be proven until that arm exists. This is why [work-packets](../work-packets.md) keeps HA-18 behind a separate architect-approved cooldown packet.

## 9. What this does not prove

It does not prove a client can display, accept or advance an authored quest. The first authored quest client proof (journal, dialog, advance, relog) is BlockedEvidence and belongs to the owner's manual UAT; CB-N05a infrastructure on `main` does not supply it, and nothing in this result changes that. It does not decide any premise, reward or reset rule. It allocates no ID. The signoff list is a list: until the named signers sign, HA-00c stays open and HA-17 and HA-18 stay blocked.

## 10. Checks run

Relative links and anchors resolve; CRLF on every touched `docs/analysis` file; scoped `npx --yes markdownlint-cli2 --no-globs`; `git -c core.whitespace=cr-at-eol diff --check`; line count under the 500-line soft cap. Results are in the ignored report `.claude/reports/harset-continuation/HA-00c-advisor.md`.
