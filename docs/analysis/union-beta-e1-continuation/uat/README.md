# Beta E1 lab UAT step files (UB-90)

> **Type:** How-to · **Ledger:** [../README.md](../README.md) · **Rows:** [unified-uat.md § Beta_Site_Evo_1](../../../guides/unified-uat.md#beta_site_evo_1) · **Results:** [../uat-results.md](../uat-results.md)

**Status:** steps written, awaiting owner lab approval. No row has run and no lab was used to write them. Every flow below was derived from the seeds (`db/resources/`: the Beta E1 chain files, spawn lists, items and ring regions), not from a live client. Lines marked ASSUMPTION are the points a first run must confirm.

## Who approves, and how a run goes

1. **Lab approval.** The owner approves lab use; the orchestrator asks the user before any run and waits until the lab is free. Nothing in this folder grants it.
2. The orchestrator takes the lab lease, then has a **stager** (a `hands-on` or `general-purpose` agent with the `lab-server` MCP) stage the row's fixture ([fixtures.md](fixtures.md)).
3. The orchestrator dispatches a fresh `lab-driver` with [_driver-common.md](_driver-common.md) pasted first, then the one step file, the lease id, the instance and the character.
4. The orchestrator (or a Sonnet verifier) grades the run from the server log (SigNoz service `cimmeria-cell`, or the local server log), the packet tap and the driver's screenshots, using the step file's verify list. The driver's PASS claim is never the grade. A missing screenshot is never a pass.
5. The grade goes in [../uat-results.md](../uat-results.md): pass, fail or not-run, the evidence and the date.

## Prerequisites

- A local lab server built from `main` at or after Beta's merge (#1916) and UB-50a, so it has the Beta E1 seeds (B0-B5) and the XP reward chains (UB-50a). The naquadah half of UR1 also needs UB-50b2 merged; until then grade only its XP half.
- The QA client in the lab with the slash-command XML (for `/gmdhd`, `/gmsetgodmode`), and a GM account for every fixture character.
- Optional but useful: `RUST_LOG` with `cimmeria_content_engine=debug`, because `resolve_event: chain matched` (chain id and name) is a DEBUG line. Without it, grade from the INFO `Content: ...` action lines.
- The packet tap on (UB-U6, UB-U8, UB-U6b use `onRingTransporterList`, `setRingTransporterDestination` and the dialog-choice packets).
- The Free Jaffa character `Fxbetafjdefa` made by hand by the user at character select, during the approved session ([fixtures.md § Characters](fixtures.md#characters)).
- The stand rule: 4.0 to 4.8 m from every target (the packet's "4-6 m", inside the server's 5.0 m interact limit), dialogs cleared before a chat line or click.

## Run order

Each run is one driver dispatch from one fixture. Rows marked "graded from" have no run of their own.

| Order | Run (step file) | Fixture | Also grades | Budget (calls) |
|---|---|---|---|---|
| 1 | [UB-U1](UB-U1.md) | F-H1 | UB-U1b (Human half) | 30 |
| 2 | [UB-U3](UB-U3.md) runs A, B, C (one character, no restage or relog between them) | F-H2 | UB-U3c, UB-U3d, UR1, UB-U6b (247) | 30 + 32 + 42 |
| 3 | [UB-U3b](UB-U3b.md) | F-H3 | - | 40 |
| 4 | [UB-U7b](UB-U7b.md) | F-H3 (restaged) | - | 30 |
| 5 | [UB-U4](UB-U4.md) runs A, B (one character, no restage) | F-H4 | UB-U4b, UB-U2b (4663), UB-U6b (456, 457, 458) | 34 + 38 |
| 6 | [UB-U6](UB-U6.md) | F-FJ1 | UB-U2b (5042, 5043), UB-U6b (575, 576) | 48 |
| 7 | [UB-U8](UB-U8.md) | F-FJ2 | - | 25 |
| 8 | [UB-U7](UB-U7.md) | F-H5 | - | 25 |
| 9 | [UB-U2](UB-U2.md) | F-H5 (restaged) | - | 45 |
| 10 | [UB-U2c](UB-U2c.md) runs A, B (no restage between them) | F-H5 (restaged) | UB-U2 (Mimir) | 45 + 22 |
| 11 | [UB-U9](UB-U9.md) (two clients) | F-FJ1 + F-H2 on `p2` | - | 20 + 26 + 20 (three dispatches) |
| 12 | [UB-U6b](UB-U6b.md) fallbacks only | F-B247 ... F-B576 | - | 15 each |
| - | [UB-U1b](UB-U1b.md), [UB-U2b](UB-U2b.md), [UB-U3c](UB-U3c.md), [UB-U3d](UB-U3d.md), [UB-U4b](UB-U4b.md), [UR1](UR1.md) | - | graded from the runs above | - |

Between two runs of one character: logout (`lab_logout`), restage, then dispatch; the stager is refused with `online` while the character is in the world. When a FIND of a far NPC or cage returns nothing, the step's retry rule applies (TP to the nearest stand, FIND again, else `not found` and `not-run`).

Orders 1 and 2 first: they prove the arrival, the Human arc and the reward, and the rest reuse their stands. Order 12 runs only for a button its arc run did not press.

## Not runnable until UO-16 (HELD-B16)

The Asgard arc has no real-player entry until Omega's UO-16 is built (BlockedDependency, not an owner question). The brief rules these rows not-runnable, so their files are written in full but marked NOT RUNNABLE, and `uat-results.md` keeps them `not-run`:

- [UB-U5](UB-U5.md), [UB-U5b](UB-U5b.md), [UB-U5c](UB-U5c.md), [UB-U10](UB-U10.md).
- The Asgard half of [UB-U1b](UB-U1b.md) (443 S1486) and the Asgard client of [UB-U9](UB-U9.md).

When UO-16 merges, make `Fxbetaasgdefa` by hand and run them from the F-A fixtures in [fixtures.md](fixtures.md).

## UB-U9: parallel clients

UB-U9 needs two lab clients at once (the lab runs up to five per daemon, #1312; set up `p2` with `lab instances init`, on a GM account). A lease is valid only on its own instance, so take two: one on `default`, which drives the Free Jaffa `Fxbetafjdefa` (F-FJ1), and one on `p2`, which drives the Human `Uat Fxbetahumpc` (F-H2). Stage both fixtures before either client logs in. Dispatch **one driver at a time**, each with its own instance's lease, never two at once: the step file says which client clicks when. The third, Asgard, client is HELD until UO-16. Every screenshot must pass that client's `instance`.

## Notes for the close-out

- The row text for UB-U2b names dialogs 5040 and 5047 on an "Asgard fixture". Both are Free Jaffa dialogs: 5040 is bind-only (never displayed) and 5047 belongs to 1503, which is not built (D-UB5 A). [UB-U2b](UB-U2b.md) grades the speaker-0 dialogs the server does show instead (4663, 5042, 5043). The row text is fixed in unified-uat.md.
- unified-uat.md's Beta known-gaps line still says "Mission rewards are xp 0 and naq 0 (D-UB8)". U-D07 (approved 2026-10-19) replaced that, so the line was stale; unified-uat.md now says so.
- UB-U5c's row text says each subject click "shows that subject's dialog"; the UB-35 design says the matched chain suppresses the bound blurb, so the objective update is the feedback. [UB-U5c](UB-U5c.md) grades visible feedback on the first press and records which one shows; the row text now says the same.
- UB-U7b has no fixture of its own: the login resync (chain 23053) removes staged cages, so the run collects them after login and splits the stack in the bag.
