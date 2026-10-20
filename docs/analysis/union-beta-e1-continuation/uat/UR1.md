# UR1: the 1423 reward (Human: +300 XP, +60 naquadah)

| Field | Value |
|---|---|
| Row | UR1: a Human completing 1423 on Beta gains 300 XP and 60 naquadah (U-D07, approved 2026-10-19) |
| Runnable | XP half: yes, graded from [UB-U3](UB-U3.md) run A. Naquadah half: only once UB-50b2 is merged into the lab server's build; until then `not-run` |
| Instance | `default` |
| Character | `Uat Fxbetahumdefa` |
| Fixture | F-H2, through UB-U3 |
| Budget | none of its own |
| Piggyback | UB-U3 run A steps 3, 4 and 7 (`xp-before`, `cash-before`, `xp-after`, `cash-after`) |

**From the seeds.** Chain 23480 (`beta_e1_rewards_chains.sql`) fires on `mission_completed` 1423 for archetypes 1 to 4 and runs `grant_xp` 300 at action sort 0. Sort 1 is kept for UB-50b2's `grant_cash` 60. The mission's own reward columns stay 0, so all of the reward comes from this chain.

## Driver steps

None of its own. If the lab build already has UB-50b2, nothing changes in the run: `cash-before` and `cash-after` are already in run A.

## Expected and pass/fail

| Check | Pass when | Evidence |
|---|---|---|
| A. XP | `xp-after` minus `xp-before` is 300 (if the level changed, the log decides) | run A steps 3 and 7, server log |
| B. Naquadah | `cash-after` minus `cash-before` is 60 | run A steps 4 and 7, server log |
| C. Once | One XP grant and at most one naquadah grant for the completion | server log |

The XP half passes on A and C. The naquadah half passes on B and C; without UB-50b2 in the build it is `not-run` (not a FAIL), and `uat-results.md` gets two lines' worth of evidence in one row.

## Orchestrator only: verify list

- With debug on, `resolve_event: chain matched` chain_id 23480 right after `Content: completing mission` 1423.
- `Content: granting XP` amount 300 for this character, once.
- With UB-50b2: `Content: granting naquadah` amount 60, once, from the same chain.
- No other XP or naquadah grant in run A's window.
