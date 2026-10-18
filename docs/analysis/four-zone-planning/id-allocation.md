# Four-zone ID allocation

> **Type:** Reference
> **Audience:** Zone campaign coordinators and the ID-lock packets (CB-N00, CS-00, HA-00b, T-01)
> **Last updated:** 2026-10-18
> **Companions:** [four-zone index](README.md), [execution contract](execution-contract.md), [Harset chain ranges](harset-work-packets.md#contract-and-dispatch-boundary), [Tollana packets](tollana-work-packets.md)

Owner decision, 2026-10-13: the four zone campaigns share one content-chain ID table so that no two campaigns, branches or workers pick the same `chain_id`. This page is that table. A zone's ID-lock packet reads it, claims individual IDs inside its own block, and records the claim in its zone ledger before any coder receives an ID.

## Chain IDs in use today

Measured on 2026-10-13 at `origin/main` 885bbae05 from the first value of each `INSERT INTO content_chains` statement in `db/resources/Content/Seed/*.sql`. Every file uses single-row inserts; read only the `chain_id` column, because description strings such as `change_stat(8, 0, 100)` contain numbers that are not chain IDs. No `chain_id` appears in two files. Chain 4001 exists only in the one-off script `db/scripts/add_health_slappack_use_chain.sql`, not in the seed; it collides with nothing here.

| Seed file | Chains | IDs in use |
|---|---:|---|
| `effects_chains.sql` | 7 | 2001, 2011, 2021–2025 |
| `castle_cellblock_chains.sql` | 120 | 1001–1195 (sparse: 1001–1025, 1031–1034, 1041–1046, 1051–1066, 1071–1074, 1081–1094, 1097–1112, 1121, 1131–1133, 1141–1142, 1151–1152, 1154, 1161–1162, 1171–1178, 1181–1195) and the single outlier 5030 |
| `castle_701_chains.sql` | 18 | 1201–1205, 1231–1243 |
| `castle_702_704_chains.sql` | 23 | 1261–1265, 1271–1276, 1291–1302 |
| `castle_706_708_chains.sql` | 28 | 1321–1323, 1341–1365 |
| `sgc_w1_chains.sql` | 41 | 3001–3037, 3041–3044 |
| `harset_space_chains.sql` | 7 | 6001–6007 |
| `harset_goauld_chains.sql` | 27 | 6101–6127 |
| `harset_jaffa_chains.sql` | 18 | 6301, 6303–6308, 6331, 6333–6342 |
| `harset_opcore_chains.sql` | 21 | 6501–6505, 6511–6516, 6518–6523, 6525–6528 |
| `debug_hub_chains.sql` | 8 | 7001–7005, 7010–7011, 7020 |
| `tutorial_chains.sql` | 1 | 7101 |
| `dakara_e1_space_chains.sql` | 6 | 8001–8006 |
| `debug_area_plaza_chains.sql` | 9 | 13000–13008 |
| `debug_area_ring_chains.sql` | 9 | 13810–13818 |
| `debug_area_lineup_chains.sql` | 6 | 14090–14095 |

The planning review's working figure of "castle_cellblock 1001–5030" was a min/max span, not a block. The real Cellblock chains stop at 1195 plus one outlier, 5030, so they do not overlap the Castle (1201–1365), effects (2001–2025) or SGC W1 (3001–3044) chains. Treat every listed ID as taken even where it sits inside another zone's span.

**Free and cited:** 1381–1400, the range the old Castle CA15 draft proposed, has no chain today. **Proposal, pending owner sign-off** (not part of the owner's 2026-10-13 ID decision): Castle restoration chains (CS-02, CS-03, CS-06, CS-08) take the free 1366–1400 block next to the existing Castle chains. Until the owner signs off, CS-00 treats it as a candidate and records any claim here.

## Reserved blocks

| Block | Owner | Use |
|---|---|---|
| 1366–1400 | Castle (proposed) | Castle restoration chains. Proposal pending owner sign-off; see above. |
| 6001–6800 | Harset | Reserved by the old Harset ledger: space 6001–6099, Goa'uld 6101–6300, Jaffa 6301–6500, OP-CORE 6501–6800. Per-mission sub-ranges are in [harset-work-packets.md](harset-work-packets.md). |
| 8001–8499 | Dakara_E1 | Dakara_E1 rebuild ([ledger](../dakara-e1-rebuild/README.md)): space and travel 8001–8039 (8001–8006 in use), mission 1570 8040–8074 and 8160–8179, mission 1645 8080–8119, mission 1646 8120–8159, later missions 8180–8499 (8075–8079 unassigned). Per-file ranges: [Dakara work packets, C3](../dakara-e1-rebuild/work-packets.md#c3-seed-files-and-chain-ranges-d-dk14). |
| 9000–9799 | Tollana | Tollana restoration, population, authored and repeatable chains. |
| 9800–9999 | Tollana (Harset recipients) | Owner decision, 2026-10-13: Tollana authors the Harset-side recipient chains for 603, 611, 618, 621, 1626, 1632 and 1643 here. Harset HA-00b signs off the NPC tags they use. |
| 10000–10999 | Castle | Castle authored stories and repeatables (CS-04a/b/c, CS-05). |
| 11000–11499 | Cellblock | Cellblock authored stories (CB-N05, CB-N05b, CB-N06) and any new restoration chain CB-N00 cannot fit beside the existing Cellblock chains. |
| 11500–12999 | Cross-zone / infrastructure | Reserve. A packet claims IDs here only with the coordinator's amendment to this page. |

13000–14999 holds the debug-area chains listed above; leave it alone.

## Other IDs

Mission, step, objective, dialog, spawn, template, point-set and item IDs are not block-reserved here. Each zone's ID-lock packet (CB-N00, CS-00, HA-00b, T-01) allocates them against the current seed and every other zone's ledger, runs a no-collision check across all seed files and all active campaign branches, and records the result in its zone ledger. A new authored mission ID must also be absent from the stock `CookedDataMissions.pak`, since CB-N05a inserts it as a new catalog entry.

## Keeping this table true

- Recount before each zone's ID-lock packet and again at FZ-99, and update the "in use" table with the date and commit.
- A claim is a reviewed amendment to this page in the claiming campaign's branch. Two campaigns never claim from the same block.
- An ID that a merged PR has used moves from "reserved" to "in use"; an abandoned claim is released by amendment, never silently reused.
