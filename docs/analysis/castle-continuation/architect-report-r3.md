# Castle continuation: architect report, wave R3 (rest of the campaign)

> **Type:** Reference. **Audience:** Castle orchestrator. **Date:** 2026-10-17.
> **Writes:**
> - [r3-packets.md](r3-packets.md): rules, contract, R3a restoration seals, R3b/R3c restoration builds.
> - [r3-packets-2.md](r3-packets-2.md): authored and hub seals, the authored-quest template and instance table, CS-05, CS-INT3, R3d, the final phase, the status table, the dispatch briefs, and the R2+R3 mermaid graph.
> - [README.md](README.md): F16-F19, D-CA15..D-CA18, the R3 status rows and the R3 wave row.
> - [RESUME.md](RESUME.md): rewritten.
>
> Every file is CRLF and under 500 lines. Nothing was committed, and no code, seeds or branch files were touched.

## Goal

Pre-write every Castle packet that R2 held, in full prescriptive form, so the next stage is dispatch and review only: CS-03d, CS-04a/b/c, CS-05, CS-06, CS-07, CS-08a2 and CS-08b. Where evidence is missing, the packet is a seal-first pair, and the build's gate names the seal's verdict. No ID or mechanic is guessed:
- restoration chains come from 1394-1400;
- authored chains come from 10000-10999;
- other authored IDs come from the CS-00Be sub-blocks, with the no-collision check inlined.

## Findings (new in R3)

- **F16. Cap pressure.** Three test files are already past the soft cap: `mission_round_trip_tests.rs` (638), `mission_1360.rs` (555) and `resources/tests/mission_additions.rs` (610). Harset's HA-1360b draft extends the first in place. CS-08a2M splits it, which is behavior-neutral and Ready at R3b.
- **F17. The 1664 mechanism exists.** `mission_completed` is a working trigger, and `CompleteMission` fires it synchronously inside the executor arm. The world at fire time and the failed-travel state stay unproven. CS-06S seals both.
- **F18. One shared catalog array.** `MISSION_ADDITIONS` is a single array, and an empty-assert sits at `tests/mission_additions.rs:125`. Cellblock's CB-N05 fills the same array. D-CA17 keeps Castle's XML in its own file, so the conflict is reduced to array entries that CS-07R unions.
- **F19. The proof ordering forces two UAT rounds.** The contract holds authored quests 2..n until a live catalog proof. Under D-CA11, that proof comes only at the end UAT, so the default plan needs two user UAT rounds. Cellblock's CB-N05 proof is itself waiting on the user's CB-U09.

## Decisions

| ID | Decision | Status |
|---|---|---|
| D-CA15 | CS-08a2M splits the letter tests out. The campaign that reaches `main` second owns the recipient round trip. The orchestrator relays the new file name to Harset (HA-1360b). | Decided (architect) |
| D-CA16 | Authored quests after `medical_q1`: hold until the proof (A) or build now (B). Does a CB-N05 proof count for Castle? | **BlockedDecision (owner)**; recommended A + yes |
| D-CA17 | The Castle authored layout (`mission_additions_castle.rs`, `tests/mission_additions_castle.rs`, `castle_side_stories_chains.sql`, `castle_repeatable_chains.sql`). | Decided (architect) |
| D-CA18 | CS-06a waits for HA-01b on the Harset branch. The PR merges only with HA-01b on `main` or in the same window; otherwise CS-06a/b are reverted out of the PR. | Decided (architect) |
| Chains | 1398 is the 1664 grant; 1399 is a retry or catch-up, only if CS-06S proves it is needed; 1400 is reserve. CS-03dS may use only an unused ID from 1386-1393. | Decided (O14-22 delegation) |

## Packets

| Wave | Packets | Size |
|---|---|---|
| R3a seals (docs) | CS-06S, CS-08bM, CS-03dS, CS-05S, CS-04aD, CS-04bD, CS-04cS | 7 specialist seals, one reviewer |
| gate | CS-INT3a | packet-coder |
| R3b | CS-08a2M, CS-08b1, CS-06a, CS-03d, CS-05a, CS-04a1s | 6 packet-coder packets |
| R3c | CS-08b2, CS-06b, CS-04a1c, CS-05c, CS-08a2S, CS-DOC3 | 6 |
| gate | CS-INT3 | packet-coder |
| R3d | the remaining authored "s"/"c" pairs and CS-05b, from the instance table, at most 6 per wave; CS-INT4 | after D-CA16 |
| final | CS-07R (merge `main`), CS-08a2, CS-07a (docs), CS-07b1/b2/bF (G-REVIEW, at most 3 rounds), CS-07c (user UAT), PR | |

Each authored quest is two packets of about 40k tokens:
- the **"s"** packet adds the catalog XML, `MISSION_ADDITIONS`, the resources tests and the mission and dialog seeds;
- the **"c"** packet adds the chains, the replay tests and any spawns.

The seals freeze the exact bytes, and the builds copy them verbatim.

## Owner decisions needed (short list)

1. **D-CA-SYM.** The 1521 implant: Option A (`add_item 5169`, optional debuff 1926) or Option B (none). It gates CS-03dS and CS-03d.
2. **D-CA16.** Build the authored quests after the first before a live catalog proof? Recommended: no. And does Cellblock's CB-N05 live proof count for Castle? Recommended: yes.
3. **D-CA14 (action, not a decision).** Retire the integrated `castle-*` worktrees with `--force`, as each wave integrates.
4. **At CS-07c.** Run the end UAT. Re-ask D-CA9 only if an agent drives it.

Not owner decisions, but cross-campaign dependencies the orchestrator tracks through ops: Harset HA-01b (D-CA18), HA-1360a3c/b (CS-08a2S), HA-D05 (recorded by CS-06S), HA-D04 (Harset enables 6503/6504). The CS-00R2 G-567 verdict gates CS-08bM. If CS-05S needs a time-cooldown primitive or new vendor UI, it returns Blocked, and the architect slices a CS-05E pair.

## Risks

- **Stranded 1664.** It is covered by D-CA18 and by CS-06S point 4 (travel failure, and characters who finished 708 before CS-06 shipped).
- **Catalog conflicts with Cellblock.** CS-07R unions the `MISSION_ADDITIONS` entries and the `:125` assertion. Any other conflict means `blocked`.
- **Dropped quests.** If the seals land fewer than 5 quests, the count rule triggers an architect pass.

## Tested vs untested

Nothing was built or run. The code facts come from reading `zone-castle` at `da2b82c70`: line counts, `mission_completed` call sites, the `MISSION_ADDITIONS` layout, `lib.rs:34`, `services/lib.rs:91-92` and `missions.num_repeats`. The Cellblock and Harset facts come from `git show` and `git grep` on their branches. Every client-facing claim (journal, dialogs, icons, barks, the 1664 hand-off) is untested until CS-07c.
