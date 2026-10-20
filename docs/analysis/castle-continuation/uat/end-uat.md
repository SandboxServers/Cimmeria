# Castle end UAT

> **Type:** Test script (end-of-campaign UAT ledger). **Audience:** the user, who runs these at the end (D-CA11), and the Castle coordinator.
> **Status:** Every row is `UATPending (end UAT, D-CA11)`. No R2 packet starts a client.
> **Companions:** [overlay ledger](../README.md), [mission catalog evidence](../evidence/mission-catalog.md), [CS-01 walk](CS-01-walk.md), [CS-01 probe](CS-01.md).

On 2026-10-17 the user directed that lab work stops for now, content restoration continues, and UAT is left to the user at the end (D-CA11). A claim only the client can prove is written here instead of being tried.

## How to run

Start the Castle stack as described in [The Castle server instance](../resume-packets.md#the-castle-server-instance) (the CS-01b recipe; the section lives in `resume-packets.md`, not `work-packets.md`). In short:

- Worktree `castle-cs-01b`, DB `sgw_castle_cs_01b`, with `LOGON_PORT=18081`, `BASE_PORT=32932`, `ADMIN_PORT=18443`, `MINIGAME_PORT=30100`, `CIMMERIA_LAB_MCP_BIND` on loopback 8452 and `CIMMERIA_DEPLOY_ENV=dev-castle`.
- Build with `pwsh tools/build-lane/lane.ps1 cargo build -p cimmeria-server --release`, run `--check-db`, then start.
- Never use the Cellblock server, `zone-cellblock`, or `sgw_zone_cellblock`.
- Ask before using the lab, wait until it is free, then follow `docs/guides/unified-uat.md`.

## CS-00B client proof

First authored quest (the `MissionAddition` entry from the CS-00B code half). All rows `UATPending (end UAT, D-CA11)`.

| Row | Expectation | Server evidence | Status |
|---|---|---|---|
| A1 | The mission appears in the journal with its history text and step text. | log `cooked_data.mission_catalog_inserted` at startup; client journal shows the entry | UATPending (end UAT, D-CA11) |
| A2 | The offer dialog opens, shows the authored screens, and Accept starts the mission. | chain fires; `mission_status` becomes `active` | UATPending (end UAT, D-CA11) |
| A3 | Completing the objective advances the step and the journal line updates, with no skip to a later step. | `advance_step` log; step order matches XML declaration order | UATPending (end UAT, D-CA11) |
| A4 | After a relog, the mission, its step and any hidden state are restored. | `sgw_mission` row persisted; journal matches before the relog | UATPending (end UAT, D-CA11) |
| A5 | A client holding an old cached mission category resyncs and shows the entry (metadata bump). | info event "Cimmeria mission overrides applied; metadata bumped" (`apply_overrides.rs:324-330`, fields `bump` and `bumped_metadata`) | UATPending (end UAT, D-CA11) |

## CS-01 walk

The step file is [CS-01-walk.md](CS-01-walk.md). The R1 run reached setup, the GM 688 grant and the Castle landing; walking was blocked by lab input, so the natural route 701 to 708 to the Harset crossing is unproven. All rows stay `UATPending (end UAT, D-CA11)`; the walk's own PASS rules (a server-log line beside every driver claim) apply unchanged.

## Mission 1520: Med Dispenser

Source: [CS-02S](../worknotes/CS-02S.md), "End UAT rows". Mission 1520 is sealed, seeded (CS-02c1) and replay-tested (CS-02c2), never live-tested; every row is `UATPending (end UAT, D-CA11)`. Server-side evidence (chain firing, step and objective state, relog restores) is what the replay tests CS-02c1 and CS-02c2 cover. These rows are the part only the client can prove. See the [chain table](../../../content/mission-chains.md#mission-1520-ambernol-medical-dispenser-quest-working-title-reconstruction-sealed).

| Row | Expectation | Status |
|---|---|---|
| D1 | Ogilvie shows the `!` (bit 24) after 1520 is accepted at the Infirmary, and nobody else's view changes. | UATPending (end UAT, D-CA11) |
| D2 | Dialog 4983 opens with Ogilvie's portrait and name; the close advances the journal to "Hack the Med Dispenser...". | UATPending (end UAT, D-CA11) |
| D3 | The dispenser is visible (Goa'uld puzzle-station mesh) and shows the bit-30 glow only to a player on step 4453; a player on another step sees none and cannot click. | UATPending (end UAT, D-CA11) |
| D4 | A right-click starts the Livewire UI; defeat allows a retry; victory advances to 4454 and shows 4984. | UATPending (end UAT, D-CA11) |
| D5 | 4985 opens after the 4984 close. Its name label is expected to read the dispenser's (cosmetic, client-side). Closing it completes the mission and shows the completed journal entry. | UATPending (end UAT, D-CA11) |
| D6 | A relog at each of 4452, 4453 and 4454 restores (bind, bind, auto-complete). | UATPending (end UAT, D-CA11) |
| D7 | Optional: swap template 412's mesh to `Em-Props.EM-MedicalBox00`; accept only if it renders. | UATPending (end UAT, D-CA11) |

## Mission 1521: Symbiote Tank

Source: [CS-03S](../worknotes/CS-03S.md), "End UAT rows". Option B skeleton (no item, no debuff). Sealed, seeded (CS-03c1) and replay-tested (CS-03c2), never live-tested; every row is `UATPending (end UAT, D-CA11)`. See the [chain table](../../../content/mission-chains.md#mission-1521-symbiote-tank-quest-option-b-skeleton-working-title-reconstruction-sealed).

| Row | Expectation | Status |
|---|---|---|
| S1 | The tank is visible and clickable in the Infirmary; the mission-object highlight (bit 30) shows at steps 4456 and 4457. The static hack cursor shows to every player at every step; confirm that is acceptable. | UATPending (end UAT, D-CA11) |
| S2 | The Livewire window opens and a victory returns to the chain; defeat allows a re-click. | UATPending (end UAT, D-CA11) |
| S3 | Dialog 4990 appears as the player's own narration. | UATPending (end UAT, D-CA11) |
| S4 | Dialog 4991 opens on the second click; the speaker label (tank vs Moh'katan) is acceptable. | UATPending (end UAT, D-CA11) |
| S5 | Journal text for steps 4456-4458 reads sensibly with no implant mechanic. | UATPending (end UAT, D-CA11) |
| S6 | Non-Jaffa archetypes never receive the offer. | UATPending (end UAT, D-CA11) |
| S7 | Added by DOC2, not a seal row: the tank is reachable on foot from the Infirmary (the seal proved navmesh component equality only; the `find_path` replay is a CS-03c2 row). | UATPending (end UAT, D-CA11) |
| S8 | Caution, known dead end (UNRESOLVED (owner decision; no ledger decision ID exists yet)): do not relog at step 4458, between the second tank click and closing dialog 4991. The tank does nothing and 4991 is not re-offered, so 1521 cannot complete without a seed restore chain (an owner decision). Relog at 4456 and 4457 is fine. Added by DOC2 from the CS-03c2 review. | UATPending (end UAT, D-CA11) |

## medical_q1 (Triage Rounds)

Source: [CS-04aD](../worknotes/CS-04aD.md), "End-UAT rows". Mission 10300, chains 10000-10006. Sealed, seeded (CS-04a1c1) and replay-, services- and cooked-sync-tested, never live-tested; every row is `UATPending (end UAT, D-CA11)`. An observed blank dialog, crash, lost progress or missing marker blocks the packet. See the [chain table](../../../content/mission-chains.md#mission-10300-triage-rounds-medical_q1-reconstruction-sealed).

| Row | Check | State |
|-----|-------|-------|
| Q1-U1 | Journal: after accept it shows *Triage Rounds*, the history text, and step B text "Check the Castle front courtyard for wounded." with objective "Reach the Castle front courtyard." | UATPending (end UAT, D-CA11) |
| Q1-U2 | Dialog 10700 renders its text on the first screen with no buttons, closes without a crash, and no blank panel appears. | UATPending (end UAT, D-CA11) |
| Q1-U3 | Marker: the medic shows the first-press marker after 704 completes, and clicking it accepts on the first press (visible feedback). | UATPending (end UAT, D-CA11) |
| Q1-U4 | Advance: step A completes and B becomes active at accept; the journal updates without a reload. | UATPending (end UAT, D-CA11) |
| Q1-U5 | Complete: entering the courtyard completes the mission; the journal marks it done; the marker does not return. | UATPending (end UAT, D-CA11) |
| Q1-U6 | Relog at not-active (marker returns), at B (state restored, then walk in) and after completion (no marker). | UATPending (end UAT, D-CA11) |
| Q1-U7 | Abandon at B returns the marker; re-accept works; a second character is unaffected. | UATPending (end UAT, D-CA11) |
| Q1-U8 | Route: the courtyard is reachable on foot from the Infirmary with no hostile aggro on the route. | UATPending (end UAT, D-CA11) |
| Q1-U9 | Races: click twice quickly; click while out of range (a chat line appears). | UATPending (end UAT, D-CA11) |

## Hub quartermaster

Source: [CS-05S2a](../worknotes/CS-05S2a.md), "End-UAT rows". Template 415, spawn 13804. The medic is not shipped, so the worknote's void S6 is omitted. These rows reuse the names S1 to S5 that the Mission 1521 table above also uses; here they always mean the hub, and other docs should cite them as "hub S1" to "hub S5".

| Row | Expectation | State |
|-----|-------------|-------|
| S1 | A character arriving at the Armory (ring/respawner side) walks to the quartermaster and right-clicks it: the vendor window opens with the 19 rows. | UATPending (end UAT, D-CA11) |
| S2 | Buy a 100-round stack for 1 naq: cash drops by 1, the stack lands in the main bag. With a character that holds BDU pants (or, on char_def 9, 10 or 19, the starter pistol), sell the item to the quartermaster and buy it back: cash unchanged. | UATPending (end UAT, D-CA11) |
| S3 | The quartermaster cannot be damaged and no hostile pulls toward the spot while the character stands there. | UATPending (end UAT, D-CA11) |
| S4 | A character landing at the DHD (component 116) cannot reach the hub on foot; expected until the Alpha-to-Armory route is proven. | UATPending (end UAT, D-CA11) |
| S5 | With the vendor window opened once, walk 30 u away and attempt a purchase: it is refused with the visible line "You are too far from the vendor." (CS-05R1 is in the branch). | UATPending (end UAT, D-CA11) |

## Hub barks

Source: [CS-05S2b](../worknotes/CS-05S2b.md), "End-UAT rows". Chains 10200 and 10201. Nothing was run on a client, so no row is claimed as passed.

| Row | Steps | Expected | State |
|-----|-------|----------|-------|
| HB1 | Complete 1520 on a non-Jaffa character, leave and re-enter the Infirmary. | One Ogilvie chat line (the B1 text) in the say channel, no dialog window; no second line on the next entry in the same session. | UATPending (end UAT, D-CA11) |
| HB2 | Complete 1521 on a Jaffa character, re-enter the Infirmary. | One Castle Medic chat line (the B2 text); no Ambernol offer fires. | UATPending (end UAT, D-CA11) |

## Round 4 quests

Rows for the R4 Ship packets. Every row is `UATPending (end UAT, D-CA11)`; no client has run any of them. Server-side behavior that a replay test covers is listed in the [chain tables](../../../content/mission-chains.md); these rows are the client-only part. An observed blank dialog, crash, lost progress or missing marker blocks the packet. The c2 lifecycle stage (relog, abandon, refusals, sibling disjointness, persistence round trip, repeat cap) is in the reviewed Ship tips and replay-tested server-side, so the relog, abandon and cap rows below check the client side of paths the server tests already pin: the markers, dialogs and journal a client shows after them.

## medical_q2 (Overlooked in the Cells)

Source: [CS-04aD2](../worknotes/CS-04aD2.md), "End-UAT rows". Mission 10301, chains 10010-10017. See the [chain table](../../../content/mission-chains.md#mission-10301-overlooked-in-the-cells-medical_q2-reconstruction-built).

| Row | Check | State |
|-----|-------|-------|
| MQ2-U1 | Journal: after accept it shows *Overlooked in the Cells* with its history text and the step B text for reporting to the medic. | UATPending (end UAT, D-CA11) |
| MQ2-U2 | Dialogs 10701 and 10702 render their text, close without a crash, and no blank panel appears. | UATPending (end UAT, D-CA11) |
| MQ2-U3 | Prisoner marker: the prisoner (template 417) shows the first-press marker once 10300 is completed, and one click accepts. | UATPending (end UAT, D-CA11) |
| MQ2-U4 | Medic marker: after accept the medic shows the turn-in marker (slot 177 swaps from the 10300 map to the 10301 map) and the prisoner's marker is gone. | UATPending (end UAT, D-CA11) |
| MQ2-U5 | Complete: clicking the medic completes the mission, the journal marks it done and no marker returns. | UATPending (end UAT, D-CA11) |
| MQ2-U6 | Relog at not-active, at step 10342 (heals to 10343) and at 10343 (medic marker returns). | UATPending (end UAT, D-CA11) |
| MQ2-U7 | Abandon returns the prisoner marker and removes the medic's turn-in marker; re-accept works; a second character is unaffected. | UATPending (end UAT, D-CA11) |
| MQ2-U8 | Route: the prisoner (component 250) is reachable on foot from the Infirmary with no hostile aggro. | UATPending (end UAT, D-CA11) |
| MQ2-U9 | Races: click twice quickly; click out of range (a chat line appears). | UATPending (end UAT, D-CA11) |

## defense_q1 (Change of Watch)

Source: [CS-04bD2a](../worknotes/CS-04bD2a.md), "End-UAT rows". Mission 10310, chains 10100-10106. See the [chain table](../../../content/mission-chains.md#mission-10310-change-of-watch-defense_q1-reconstruction-built).

| Row | Check | State |
|-----|-------|-------|
| DQ1-U1 | Journal: after accept it shows *Change of Watch*, its history text and the step B objective "Reach the Castle Infirmary." | UATPending (end UAT, D-CA11) |
| DQ1-U2 | Dialog 10709 renders its text, closes without a crash, and no blank panel appears. | UATPending (end UAT, D-CA11) |
| DQ1-U3 | Marker: the sentry (template 419) shows the first-press marker after 10300 completes, and one click accepts. | UATPending (end UAT, D-CA11) |
| DQ1-U4 | Advance: step A completes and B becomes active at accept; the journal updates without a reload. | UATPending (end UAT, D-CA11) |
| DQ1-U5 | Complete: entering the Infirmary completes the mission and the marker does not return. | UATPending (end UAT, D-CA11) |
| DQ1-U6 | Relog at not-active, at step 10354 (heals to 10355) and after completion. | UATPending (end UAT, D-CA11) |
| DQ1-U7 | Abandon returns the marker; re-accept works; a second character is unaffected. | UATPending (end UAT, D-CA11) |
| DQ1-U8 | Route (`nav_path`, CD-9): the sentry spot (component 250) is reachable on foot from the Infirmary. The arrival at component 116 is G-ROUTE, still open. | UATPending (end UAT, D-CA11) |
| DQ1-U9 | Races: click twice quickly; click out of range. | UATPending (end UAT, D-CA11) |
| DQ1-U10 | Sentry look: the NPC renders with a sensible model and name label. | UATPending (end UAT, D-CA11) |

## defense_q3 (Hold the Bravo Route)

Source: [CS-04bD2b](../worknotes/CS-04bD2b.md), "End-UAT rows". Mission 10312, chains 10120-10129. See the [chain table](../../../content/mission-chains.md#mission-10312-hold-the-bravo-route-defense_q3-reconstruction-built).

| Row | Check | State |
|-----|-------|-------|
| DQ3-U1 | Journal: after accept it shows *Hold the Bravo Route* with four objectives (three officers and Muelbach). | UATPending (end UAT, D-CA11) |
| DQ3-U2 | Offer dialog 10717 renders, closes without a crash, and no blank panel appears. | UATPending (end UAT, D-CA11) |
| DQ3-U3 | Marker: the giver (template 425) shows the first-press marker after 10310 completes, and one click accepts. | UATPending (end UAT, D-CA11) |
| DQ3-U4 | Per-kill tick: each kill ticks off exactly its own objective in the journal; a re-kill changes nothing. | UATPending (end UAT, D-CA11) |
| DQ3-U5 | Completion: the fourth kill completes the mission and dialog 10718 opens. | UATPending (end UAT, D-CA11) |
| DQ3-U6 | Relog at not-active, mid-kills (objectives restore) and after completion. | UATPending (end UAT, D-CA11) |
| DQ3-U7 | Abandon returns the marker; re-accept works. | UATPending (end UAT, D-CA11) |
| DQ3-U8 | A second character is unaffected by the first one's progress. | UATPending (end UAT, D-CA11) |
| DQ3-U9 | Route (`nav_path`, CD-9): the giver and each officer are reachable on foot. | UATPending (end UAT, D-CA11) |
| DQ3-U10 | Shared actors: after a kill by another player, the other player still sees the officer die and respawn normally; nothing is despawned or retagged; the arrival giver shows the "Sgt. Stanton" name and the courtyard Stanton is unaffected. | UATPending (end UAT, D-CA11) |
| DQ3-U11 | Races: click twice quickly; click out of range; kill outside the mission window changes nothing. | UATPending (end UAT, D-CA11) |
| DQ3-U12 | Officer 2 island (component 214): Bravo officer 2 can be reached and killed. If it cannot be killed the packet is blocked. | UATPending (end UAT, D-CA11) |

## return_q1 and return_q2 (Report from Harset and Confirm the Gate Address)

Source: [CS-04cS2](../worknotes/CS-04cS2.md), "End-UAT rows". Missions 10320 (chains 10130-10133) and 10321 (chains 10140-10146). See the chain tables for [10320](../../../content/mission-chains.md#mission-10320-report-from-harset-return_q1-reconstruction-built) and [10321](../../../content/mission-chains.md#mission-10321-confirm-the-gate-address-return_q2-reconstruction-built).

| Row | Check | State |
|-----|-------|-------|
| RQ1-U1 | Journal: after the Harset to Castle crossing, *Report from Harset* shows with its history text and "Report to Colonel Marsh." | UATPending (end UAT, D-CA11) |
| RQ1-U2 | Persistence: the accepted mission survives a relog. | UATPending (end UAT, D-CA11) |
| RQ1-U3 | Marker: Marsh shows the marker for a player on step 10372, and for nobody else. | UATPending (end UAT, D-CA11) |
| RQ1-U4 | Click: clicking Marsh opens dialog 10721, completes the mission and removes the marker. | UATPending (end UAT, D-CA11) |
| RQ1-U5 | Relog at step 10372 restores Marsh's marker. | UATPending (end UAT, D-CA11) |
| RQ1-U6 | Abandon removes Marsh's marker; the next crossing offers the mission again. | UATPending (end UAT, D-CA11) |
| RQ1-U7 | No leak: a character without the mission never sees Marsh's marker. | UATPending (end UAT, D-CA11) |
| RQ1-U8 | Route (`nav_path`, CD-9): see the per-actor rows RQ-R1-Marsh and RQ-R1-Mohkatan below. | UATPending (end UAT, D-CA11) |
| RQ1-U9 | 708 regression: the 708 to Harset crossing still works and does not break the offer. | UATPending (end UAT, D-CA11) |
| RQ2-U1 | Offer: after 10320 completes, Moh'katan shows the marker. | UATPending (end UAT, D-CA11) |
| RQ2-U2 | Accept: one click accepts and dialog 10725 opens. | UATPending (end UAT, D-CA11) |
| RQ2-U3 | Dial: dialing Harset at the Castle gate completes the mission; dialing another address does not. | UATPending (end UAT, D-CA11) |
| RQ2-U4 | Relog at not-active, at step 10378 (heals to 10379) and at 10379. | UATPending (end UAT, D-CA11) |
| RQ2-U5 | Abandon returns Moh'katan's marker. | UATPending (end UAT, D-CA11) |
| RQ2-U6 | Races: click twice quickly; dial twice. | UATPending (end UAT, D-CA11) |
| RQ2-U7 | Archetype: the offer behaves for every archetype. | UATPending (end UAT, D-CA11) |
| RQ2-U8 | No leak: slot 54 shows no marker for a character that has not completed 10320. | UATPending (end UAT, D-CA11) |
| RQ-R2 | After the Harset to Castle crossing the player stands at the stargate with no snap-back and no `arrival_unrecoverable_off_mesh` warn, and Marsh's marker is visible. | UATPending (end UAT, D-CA11) |

## return actors: nav_path legs (CD-9)

Source: CS-04cS2 row RQ-R1, split into one row per return actor as CD-9 requires. Run `nav_path` on a running Castle server. A leg passes only if a route is returned and the walk on a client follows it.

| Row | Leg | State |
|-----|-----|-------|
| RQ-R1-Marsh | Arrival at the Castle stargate to `Castle_ColMarsh`. | UATPending (end UAT, D-CA11) |
| RQ-R1-Mohkatan | Marsh to `Castle_Mohkatan`, then Moh'katan to the DHD. | UATPending (end UAT, D-CA11) |

## repeatable_q1 (Supply Run)

Source: [CS-05S3](../worknotes/CS-05S3.md), "End-UAT rows". Mission 10330, chains 10230-10237. See the [chain table](../../../content/mission-chains.md#mission-10330-supply-run-repeatable_q1-reconstruction-built). The cap rows (RP1-U5, RP1-U6) check on a client what the server tests already pin (`three_runs_then_refused`, `refused_click_still_shows_dialog_and_unbinds`, `repeats_survive_relog_and_abandon`): the third run completes and the fourth click removes the marker and shows a neutral dialog.

| Row | Check | State |
|-----|-------|-------|
| RP1-U1 | Journal: after accept it shows *Supply Run*, its history text and the step A objective. | UATPending (end UAT, D-CA11) |
| RP1-U2 | Giver: the supply officer (template 424) shows the first-press marker after 704, and a click opens dialog 10729 with no store break. | UATPending (end UAT, D-CA11) |
| RP1-U3 | Advance: entering the Infirmary advances to step B and the turn-in marker appears. | UATPending (end UAT, D-CA11) |
| RP1-U4 | Turn-in: clicking the officer opens dialog 10730 and completes the run; the offer marker returns. | UATPending (end UAT, D-CA11) |
| RP1-U5 | Repeat x3: the full run succeeds three times in a row. | UATPending (end UAT, D-CA11) |
| RP1-U6 | Cap: the fourth click removes the marker and shows a neutral dialog. | UATPending (end UAT, D-CA11) |
| RP1-U7 | Relog at not-active, at step A and at step B (marker restored). | UATPending (end UAT, D-CA11) |
| RP1-U8 | Abandon returns the offer marker; a second character is unaffected. | UATPending (end UAT, D-CA11) |
| RP1-U9 | Route (`nav_path`, CD-9): the officer and the Infirmary are reachable on foot. | UATPending (end UAT, D-CA11) |
| RP1-U10 | Races: click twice quickly; click out of range. | UATPending (end UAT, D-CA11) |
| RP1-U11 | The officer's name label reads `Op-CORE Soldier`; confirm that is acceptable. | UATPending (end UAT, D-CA11) |

## Mission 567: Romney's Files (Castle leg)

Source: [CS-08bS](../worknotes/CS-08bS.md), "End-UAT rows". Chains 1394-1397, 1400 and 10240-10243. The Castle leg reaches step 4039 only; completion depends on Harset 6503/6504, disabled (CD-11). **Eight of the nine Castle chains also ship `enabled = false` until Harset HA-D04 (CD-11); chain 1400 ships enabled, so the cabinet click gives the 10734 feedback dialog when 2000 is not active. The rows that need the other eight cannot run on a build without HA-D04**: flip those eight flags together with 6503/6504 first (CS-07bF1, CS-07bF4). The Harset turn-in is the Harset agents' row, not one of these. See the [chain table](../../../content/mission-chains.md#mission-567-romneys-files-castle-leg-reconstruction-built-ships-disabled-until-harset-ha-d04-cd-11).

| Row | Check | State |
|-----|-------|-------|
| 567-U1 | Accept: after 701 and 703 the offer dialog 2043 opens from Copplemann and Accept starts 567. | UATPending (end UAT, D-CA11) |
| 567-U2 | X-close: closing dialog 2043 with X (OPEN-1) either accepts or leaves the offer bound; note which. | UATPending (end UAT, D-CA11) |
| 567-U3 | Relog restores the state; Harset shows no offer for this character. | UATPending (end UAT, D-CA11) |
| 567-U4 | Cabinet reach: the cabinet is reachable on foot, and Romney's pull (OPEN-5, the cabinet is 6.0 u inside his 18 u aggro ring) does not make the click impractical. | UATPending (end UAT, D-CA11) |
| 567-U5 | Click: item 2698 lands in the bag with a sensible icon and monologue 10733 opens; a second click shows 10734 and grants nothing. | UATPending (end UAT, D-CA11) |
| 567-U6 | Cross at 2012: the Harset crossing advances the mission to step 4039. | UATPending (end UAT, D-CA11) |
| 567-U7 | Abandon: the item is removed and the offer reopens in Castle. | UATPending (end UAT, D-CA11) |
| 567-U8 | Re-accept loop: accept, click, abandon, accept again works with no duplicate item. | UATPending (end UAT, D-CA11) |

## Mission 1521 implant (chain 1386)

Source: [CS-03dS](../worknotes/CS-03dS.md), "End-UAT rows". Item 5169 "Immature Goa'uld Symbiote", granted on 1521 completion. See the [section](../../../content/mission-chains.md#mission-1521-implant-grant-chain-1386-reconstruction-built).

| Row | Check | State |
|-----|-------|-------|
| CS-03d-U1 | The item appears in the mission bag after 1521 completes. | UATPending (end UAT, D-CA11) |
| CS-03d-U2 | Double-clicking the item is silent: no consume, no error. | UATPending (end UAT, D-CA11) |
| CS-03d-U3 | With a full bag the item arrives by mail with "Your bag was full. Your item is in your mailbox." | UATPending (end UAT, D-CA11) |
| CS-03d-U4 | The item survives a relog. | UATPending (end UAT, D-CA11) |
| CS-03d-U5 | No debuff icon appears after using it. | UATPending (end UAT, D-CA11) |
