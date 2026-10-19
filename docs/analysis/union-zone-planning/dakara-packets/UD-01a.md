# UD-01a: Free Jaffa bridge design seal

> **Type:** How-to (work packet) · **Wave:** U1 · **Status:** BlockedDecision D-UD1
> **Index:** [dakara-work-packets.md](../dakara-work-packets.md)

| Field | Value |
|---|---|
| Role | `mission-systems-advisor` (design; no seeds, no code) |
| Size | docs only, about 35k working tokens |
| Branch / worktree | `union-dakara/ud-01a-bridge-seal` / `ud-01a` |
| Commit subject | `docs(union-dakara): Free Jaffa bridge design seal (UD-01a)` |
| Depends on | D-UD1 answered "yes" by the owner; UD-00 merged (the ID block) |

## Goal

Seal the design of the one authored bridge mission (63550) that U-D09 asks for. It gives a level-1 Free Jaffa a short, truthful intro on Dakara_E1 that ends where DK-11 offers 1645, and it claims none of 1570's SGC steps. The seal is precise enough that UD-01b and UD-01c become fill-in packets.

## Owner directives this must satisfy (contract excerpt)

- **U-D09 (owner, 2026-10-14):** author a short Dakara intro for the level-1 Free Jaffa that earns entry to the level-3 to 5 story without claiming unplayed SGC missions.
- **U-D10:** divided loyalties. Foreshadow the withdrawal orders without revealing Moh'katan's betrayal.
- **free-jaffa-route.md rule 5:** keep the early Omega address (chain 8001), but gate 1655 and 1500 on real story state.
- **D-UD1 (recommended):** Union's bridge replaces DK's Jaffa entry chain 8040. DK-11 offers 1645 when the bridge mission is completed. DK keeps the Tau'ri rows.
- **D-UD5:** no ambient population and no vendors.

## Rules (binding)

1. PowerShell only, absolute paths, `Set-Location <worktree>;` on every command. No bash. No cargo.
2. Write only `docs/analysis/union-dakara-e1-continuation/bridge-seal.md` (new). Edit nothing else, and above all nothing under `docs/analysis/dakara-e1-rebuild/`.
3. CRLF: convert after writing (`[IO.File]::ReadAllText` / `WriteAllText` with `-replace "`r?`n","`r`n"`, absolute path).
4. 500-line soft cap.
5. No stash, no prune, no push. One local commit.
6. Every beat carries a provenance label and cites its evidence: file and line, or the seed row.
7. Copy is HELD (H-LBL). Write `[COPY HELD: <one-line intent>]` placeholders, never final dialog text.
8. Rewards are HELD (U-D07). Write `[REWARD HELD]`.
9. Reuse only regions, tags and actors that exist on `origin/main` or are named in a DK packet. Never invent a map actor. Tag each one with its source.

## Steps

1. Read the inputs: [dakara.md](../dakara.md), [free-jaffa-route.md](../free-jaffa-route.md), the DK README decisions D-DK1/D-DK2/D-DK5, DK `packets/wave-3.md` (DK-10a, DK-10b, DK-11), DK `placements/C-landmark-census.md`, and `db/resources/Content/Seed/dakara_e1_space_chains.sql`.
2. Write `bridge-seal.md` with these sections:
   - **Shape.** One mission, 63550, with at most three non-combat steps. Proposed:
     1. Rak'nor briefs the player in the gate plaza on the split loyalties among the Jaffa. This foreshadows the withdrawal orders. Moh'katan is named only as a respected leader.
     2. A scouting walk through existing DK regions or interact tags. Name each with its DK packet or seed source.
     3. Report to Bra'tac in the command tent.

     Give step and objective IDs from 63560 and 63590 upward.
   - **Binds.** For each step: trigger, conditions and action, written as a table in the content-chain vocabulary (`player_loaded`, `interact_tag`, `dialog_choice`, `enter_region`; conditions `archetype eq 7`, `mission_status 63550 ...`, `step_status ...`). Every bind is gated `archetype eq 7`, so it never fires for DK's Tau'ri rows (`lte 6`). Chain IDs come from 63510-63549.
   - **Entry.** What offers 63550. Proposed: a `player_loaded` on Dakara_E1, gated `archetype eq 7` and `mission_status 63550 eq not_active`, runs `accept_mission 63550`. Note that chain 8001's Omega grant stays as it is.
   - **Exit and handoff to DK-11.** Completing 63550 is the gate DK-11 uses to offer 1645. State the exact condition DK-11 needs: `mission_status 63550 eq completed`. Write it as a request in DK's terms. Union does not edit DK files.
   - **Dialog plan.** Dialogs, sets, screens and buttons with their IDs from the sub-blocks, and the speaker for each. Copy stays HELD. Note that the override mechanism follows the Cellblock precedent (`dialog_overrides/override_seed_agreement_cellblock_62050.rs` on `content/cellblock-campaign`), which is **not on main** (F9).
   - **Client proof needed.** The authored mission must appear in the client journal. Cite F9, and list the lab UAT row it adds to UD-05.
   - **Evidence table.** One row per beat: the evidence, its provenance label (the mission itself is CIMMERIA_AUTHORING; the reused regions, actors and Bra'tac's tent are RESTORED_SOURCE or DK reconstruction), and what happens if the beat is dropped.
   - **Rejected beats.** Anything without evidence: combat, new NPCs, vendor stops, any line that reveals the betrayal.
3. Commit `bridge-seal.md` only.

## Acceptance

- Each step, bind, dialog and ID falls inside 63500-63799 and has a provenance label and a source.
- No bind can fire for archetype 1 to 6 or 8.
- The DK-11 handoff condition is stated in one line.
- No final copy and no reward numbers appear.
- Telemetry the seal requires UD-01c to assert: `Content: accepting mission` for the offer chain, `Content: advancing step` for each step chain, and `Content: final required objective completed the mission — firing mission_completed` at the end. All three are in `crates/cell-content/src/cell/content/executor/mission.rs`; re-grep to confirm they are still there.

## Docs rows

- "Campaign ledger / per-packet progress": `docs/analysis/union-dakara-e1-continuation/bridge-seal.md`.

## Return

One line: `UD-01a <done|blocked|failed> <sha|-> <report path>`.
