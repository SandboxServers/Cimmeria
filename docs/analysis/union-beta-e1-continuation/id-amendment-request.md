# Union Beta E1: ID amendment request (D-UB9)

> **Type:** Reference (request)
> **Audience:** The Union coordinator, who owns `docs/analysis/four-zone-planning/id-allocation.md`
> **Last updated:** 2026-10-18 (ARCH-B1: approved and applied; screens re-carved)
> **Companions:** [README.md § ID re-check](README.md#id-re-check-ub-00c-step-1), [beta-work-packets.md § ID block](../union-zone-planning/beta-work-packets.md#id-block-d-ub9)

**Approved** by the coordinator (the orchestrator) on 2026-10-18 and **applied by ARCH-B1**: the chain row below is now in id-allocation.md § reserved blocks, with a link back here for the non-chain tables. ARCH-B1 also re-carved the authored dialog_screens sub-range (see the note under the table).

## Where the rows go

`id-allocation.md` § reserved blocks is a content-chain table (Block, Owner, Use) sorted ascending. The chain row sorts by number: **after** `11500–12999 | Cross-zone / infrastructure` and **before** any Dakara `63500–63799` and Omega `63800–63999` rows the coordinator has merged. The other tables have no home in that page today. The coordinator may add a short "Other tables" section for them, or keep them in this file by link.

## Row for the reserved-blocks table

| Block | Owner | Use |
|---|---|---|
| 23000–23499 | Union Beta E1 | Beta_Site_Evo_1 (world 23) first-visit chains ([ledger](../union-beta-e1-continuation/README.md)): travel 23000–23019; 1423 23020–23039; 381 23040–23069; 473 23070–23099; 478 23100–23129; 1425 23130–23159; 1426 23160–23189; 1427 23190–23219; 1500 23220–23239; 1502 23240–23279; 1503 23280–23309; 481 and beam 23310–23339; 420 23340–23359; 421 23360–23389; 1147 23390–23404; 443 23405–23434; 477 23435–23459; 479 23460–23469; reserved 23470–23479; authored 23480–23499 (UB-50, HELD). |

## Rows for the non-chain tables

| Table | Block | Owner | Use |
|---|---|---|---|
| `spawnlist` | 23000–23199 | Union Beta E1 | hub 23000–23059; Asgard 23060–23109; Nine Peaks 23110–23159; life 23160–23199 |
| `point_sets` | 23000–23099 | Union Beta E1 | hub 23000–23029; Asgard 23030–23059 (23031 = FireSite); Nine Peaks 23060–23079; travel 23080–23099 |
| `point_set_points` | 23000–23299 | Union Beta E1 | hub 23000–23099; Asgard 23100–23179; Nine Peaks 23180–23259; travel 23260–23299 |
| `entity_templates` | 2300–2349 | Union Beta E1 | NPCs 2300–2319 (cast 2300–2307, 2303 reserved for UB-52; life 2308–2309); objects 2320–2349 (2320–2335 used, 2327 and 2336 reserved) |
| `respawners` | 2300–2309 | Union Beta E1 | any new rows; respawners 6 and 7 are re-authored in place |
| `ring_transport_regions` | 2300–2309 | Union Beta E1 | 2300/2301 = the hub ring 148 to SW_RING pair (D-UB4 = A, decided by ARCH-B1); 2302–2309 reserved |
| authored `missions` | 64000–64019 | Union Beta E1 (UB-50, HELD) | CIMMERIA_AUTHORING only |
| authored `mission_steps` | 64020–64099 | Union Beta E1 (UB-50, HELD) | |
| authored `mission_objectives` | 64100–64179 | Union Beta E1 (UB-50, HELD) | |
| authored `mission_tasks` | 64180–64259 | Union Beta E1 (UB-50, HELD) | |
| authored `dialogs` | 64260–64299 | Union Beta E1 (UB-50, HELD) | |
| authored `dialog_sets` | 64300–64309 | Union Beta E1 (UB-50, HELD) | |
| authored `dialog_screens` | 64124–64193 | Union Beta E1 (UB-50, HELD) | re-carved by ARCH-B1 from 64310–64379 (see below) |
| authored `dialog_screen_buttons` | 64380–64419 | Union Beta E1 (UB-50, HELD) | |
| authored `dialog_set_maps` | 64420–64459 | Union Beta E1 (UB-50, HELD) | |
| authored `texts` | 64460–64499 | Union Beta E1 (UB-50, HELD) | |

**Screens re-carve (ARCH-B1).** dialog_screens.sql keeps the screen id in its second column (dialog_id, screen_id), so the tuple-head scan below read the dialog id and missed four stock screens in the old 64310–64379: 64341 (dialog 2002) and 64373–64375 (dialog 3122). The new 64124–64193 was scanned on the second column and is free. Each table numbers its own ids, so sharing numbers with the objectives and tasks sub-ranges is harmless.

Authored rows stay at most 65535 and outside the stock pak range 38–1826. They sit directly above Dakara 63500–63799 and Omega 63800–63999.

## Checks run

Each check was made on 2026-10-18 and returned **no match**. `git grep` exits 1 on no match; that is read as `clear`. Full method, counts and scoping notes are in the [ledger](README.md#id-re-check-ub-00c-step-1).

Scopes: `origin/main` @ `102d15444d15ef8f8113f3edcaa8affd4df69cd8`; all 1047 remote refs (150 distinct `db/resources` trees); 185 distinct local-branch trees; 188 sibling worktrees including untracked files; the Omega, SGC and Dakara plan and packet docs.

| Table | Range | Pattern (tuple head) | Scope | Result |
|---|---|---|---|---|
| content_chains | 23000–23499 | `(23[0-4]dd,` | all `db/resources` except `effect_nvps.sql` and `texts.sql` | clear |
| spawnlist | 23000–23199 | `(23[0-4]dd,` | same | clear |
| point_sets | 23000–23099 | `(23[0-4]dd,` | same | clear |
| point_set_points | 23000–23299 | `(23[0-4]dd,` | same | clear |
| entity_templates | 2300–2349 | `(23[0-4]d,` | `*entity_templates*.sql` | clear (highest template in use: 1595) |
| respawners | 2300–2309 | `(23[0-4]d,` | `*respawners*.sql` | clear |
| ring_transport_regions | 2300–2309 | `(23[0-4]d,` | `*ring_transport*.sql` | clear |
| authored mission, dialog and text tables | 64000–64499 | `(64[0-4]dd,` | all `db/resources` except `effect_nvps.sql` and `texts.sql`; `texts.sql` separately for 64460–64499 | clear |
| plan ledgers | all of the above | numeric search | Omega, SGC and Dakara plans and packets | clear: they cite Beta's own ids (23001, 23020, 23220, 23310, 64000–64499) and no sibling plans an id inside a Beta range |

Not independently verified: the Cellblock, Castle, Harset and Tollana ledgers (owner waiver W-UB1). Their branch seeds were scanned, but their planning documents were not read for planned ids. All their reserved blocks in `id-allocation.md` are below 12999 and so do not overlap.

ARCH-B1 applied this request on 2026-10-18, the day of the check. A branch pushed after that is not covered.
