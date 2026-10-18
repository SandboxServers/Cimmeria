# Castle continuation: architect report, wave R2 (no lab)

> **Type:** Reference. **Audience:** Castle orchestrator. **Date:** 2026-10-17.
> **Writes:** [r2-packets.md](r2-packets.md) (new, 497 lines), [README.md](README.md) (F10-F15, D-CA11..D-CA14, D-CA-SYM row, R2 status and wave rows), [RESUME.md](RESUME.md) (rewritten, 37 lines), and a pointer section in [work-packets.md](work-packets.md). Every file is CRLF. Nothing was committed, and no code, seeds or branch files were touched.

## Goal

The user's direction (2026-10-17) was to abandon lab work and return to Castle content restoration; the user UATs everything at the end. Wave R2 restores what evidence allows with no lab use, ordered by value, and respects these gates:

- G-567, G-706 and 1376 (CS-00R2);
- CS-08a2 and HA-1360a/b;
- G-1664 and Harset;
- D-CA-SYM;
- the unresolved G-MED placement, game and radio.

## Findings (new in R2)

- **F10.** The lab is paused. CS-01c reached setup, the GM 688 grant and the Castle landing; walking was blocked by lab input. Its rows stay UATPending.
- **F11.** The Castle server, PID 36540 (`castle-cs-01b\cimmeria-server.exe`, started 2026-10-17 18:52), still holds 18081/32932/18443/8452/30100 on `sgw_castle_cs_01b`.
  - Cellblock's CB-U08 lab attempts logged into it, which made them invalid (Cellblock overlay RESUME).
  - Cellblock's test character `Cellutb` lives on that DB.
  - CS-01b's commit `009ab0536` is not cherry-picked.
- **F12.** The R1a worktrees `castle-cs-00ra/-00rb/-01a/-inf1` are integrated but still on disk, because ops refuses `--force`.
- **F13.** The runtime never reads `step_enabled` or `is_enabled` (only `console_authoring` tests do), and mission 708's step 2417 is `false` yet works. Enabling 1520/1521 needs no flag flips; the missions come from the pak.
- **F14.**
  - No seeded minigame victory callback (1017, 1025, 1042, 1061, 1234, 1293, 13005, 1357, 3024, 7005) has conditions.
  - `fire_guarded_minigame_chain_by_id` already exists (`crates/cell-content/src/cell/content/event_dispatch/direct_invoke.rs`).
  - Today the Cell fires a victory for a missing entity with player 0 (`crates/cell/src/cell/service/base_messages/minigame.rs:38-54`).
  - Base already holds `receipt.player_id` (`cell_dispatch/minigame.rs:245`) but does not forward it (`:291`).
- **F15.** Only chain 1366 is seeded; 1367-1377 are claimed only. `data/spaces/castle.nav` is tracked, so `nav_inspect` and `crates/entity/tests/castle_navmesh.rs` can prove placement and routes offline.

## Decisions

| ID | Decision | Status |
|---|---|---|
| D-CA11 | No lab until the end. Client-only claims go to branch `uat/end-uat.md` as UATPending. | Decided (user) |
| D-CA12 | b2-lite replaces the full CS-02b2 for the Castle callbacks 1372 and 1375. It is internal `player_id` binding plus a guarded fire for conditioned chains. | Conditional on the CS-02L seal |
| D-CA13 | Stop PID 36540 now through CS-01d. Keep the DB and the worktree for the end UAT. | Decided (orchestrator confirms) |
| D-CA14 | The owner retires integrated worktrees with `--force`, `castle-cs-01b` last. | Decided |
| D-CA-SYM | The 1521 implant: Option A or B. | **BlockedDecision (owner)** |
| D-CA1 | Adopted 2026-10-17. D-CA9 is moot while the lab is paused. | Updated |

**Why b2-lite.**
- The branch ledger's reviewed gate (`work-packets.md:197-271`) says no conditioned victory chain ships before the full CS-02b2 seam rework.
- Most of that rework's trust value already landed in b0/b1: Base re-proves the entity, player and session instance before and after the close.
- What is left is small:
  - the Cell does not know which player the receipt was for;
  - unguarded `fire_chain_by_id` ignores chain conditions.
- b2-lite closes both with one internal field, one accessor and one guarded branch, with no client wire change. Legacy parity holds because no legacy callback has conditions.
- It reverses a reviewed gate, so the architect does not seal it alone. CS-02L sends it to server-authority-enforcer and minigame-systems-advisor. A reject brings back the full b2 split.

## Packets (by value)

| Wave | Packet | Agent | Value / purpose |
|---|---|---|---|
| R2-0 | CS-01d | packet-coder | Stops the stray server that hijacked Cellblock's lab login. |
| R2a | CS-02S | mission-systems-advisor | The 1520 frozen rows: dispenser 412/13801 placement (nav-proven), Ogilvie binding, Livewire, the 4984 and 4985 closes, relog, and the interaction-scope question. |
| R2a | CS-03S | mission-systems-advisor | The 1521 Option B skeleton rows, and the G-ROUTE placement choice (reachable spot, or BlockedEvidence). |
| R2a | CS-02L | server-authority-enforcer, then minigame-systems-advisor | The b2-lite trust verdict. |
| R2a | CS-00R2 | game-archaeology-specialist | G-567 (may unlock CS-08b, IDs 1394-1397), G-706, 1376 lineage. |
| R2a | CS-00Be | mission-systems-advisor | The CS-00B evidence half: catalog API, 10000-10999 reservation, collision audit. Creates `uat/end-uat.md`. |
| gate | CS-INT2a | packet-coder | Integrates R2a plus `009ab0536`; copies `r2-packets.md` to the branch. |
| R2b | CS-MGF1 | packet-coder | `player_id` on `BaseToCellMsg::MinigameResult`; `ChainEngine::chain_has_conditions`. Behavior-neutral. |
| R2b | CS-02c1 | packet-coder | 1520 seeds plus entry replay tests. |
| R2b | CS-03c1 | packet-coder | 1521 skeleton seeds plus entry replay tests. |
| R2c | CS-MGF2 | packet-coder | The player-bound victory guard, with four Cell tests. |
| R2c | CS-02c2 / CS-03c2 | packet-coder | Graph replay tests plus navmesh route tests. |
| R2c | CS-DOC2 | documentation-writer | `mission-chains.md` rows and end-UAT rows. |
| gate | CS-INT2 | packet-coder | Integrates R2b/R2c in order. The seeds never land without CS-MGF2. |

**Held:**
- CS-03d: BlockedDecision, D-CA-SYM.
- CS-08b: BlockedEvidence, until CS-00R2 seals G-567.
- CS-08a2: BlockedDependency on HA-1360a/b.
- CS-06: BlockedDependency on G-1664, HA-D05 and Kaerlia.
- CS-04/05/07: unchanged.

**Chain allocation:**
- 1378-1385: CS-02S;
- 1386-1393: CS-03S;
- 1394-1397: CS-00R2/CS-08b;
- 1398-1400: reserve (CS-06 and fixes).

## Risks the seals must close

- **The `set_interaction_type` scope.** If it is entity-global, per-player toggling of the dispenser or tank would break a second player at another step. CS-02S/03S point 3 forces a design that is correct for two players.
- **The population guard.** Its comment says "nothing here is chain-spawned or tagged". CS-02S point 11 checks whether a tagged mission actor needs a guard edit.
- **G-ROUTE.** Chamber component 468 is isolated, so a chamber tank would strand 1521. CS-03S must either move the tank to a nav-proven reachable spot (CIMMERIA_AUTHORING) or return BlockedEvidence, and 1521 then does not ship.
- **Cherry-pick adjacency.** `db/database.sql` include lines, adjacent template, spawn and `mod` rows: CS-INT2 keeps both sides of a pure-addition conflict and treats any other conflict as blocked.

## Tested vs untested

Nothing was built or run for this plan. All the code facts come from reading `zone-castle` at `da2b82c70`. Every R2 claim about the client (highlight bit 30, Livewire UI, journal text, the dialog UIScreenTypes in the live client) is untested until the user's end UAT.
