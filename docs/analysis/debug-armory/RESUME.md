# debug-armory: resume card

> **Type:** reference. **Audience:** the orchestrator resuming this campaign. **Updated:** 2026-10-27
> **Companions:** [ledger](README.md), [work packets](work-packets.md), [shared UAT contract](../starter-uat-praxis/work-packets.md#shared-uat-contract). Siblings: [praxis](../starter-uat-praxis/RESUME.md), [union](../starter-uat-union/RESUME.md), [shops](../starter-uat-shops/RESUME.md).

**State (2026-10-27):** planned, nothing dispatched. The campaign files live on branch `lab-uat-plans` until ops commits them into `dr-integration`.

**Goal:** a Debug Area (world 1300) Armory: plaza vendors selling one representative of every weapon family and equipment slot per tier and quality; generated lab specs that equip and fire each one and cast every ability of the 7 archetype trees; graded lab runs; an issue per failure.

**Start here:**

1. [Campaign-packet § 0](../../../.claude/skills/campaign-packet/SKILL.md#0-make-sure-nobody-else-is-running-it): check no peer runs it; write `.claude/state/active-campaign`.
2. `ops`: create `dr-integration` (branch `debug-armory/integration`), commit the three campaign files.
3. Wave 1: DR-01 and DR-02 at once; then DR-03a. Wave 2: DR-03b, then DR-04 and DR-05 at once. Wave loop, the pre-PR checks ([dispatch rule 7](work-packets.md#dispatch-rules)), PR 1. Merge order DR-01, 02, 03a, 03b, 04, 05.
4. After PR 1 merges: rebase `dr-integration`; wave 3 DR-06 and DR-07 at once, then DR-08; wave loop, PR 2.
5. Waves 4 and 5 (ASK-FIRST, D-SP2 and D-DR7 answered first): DR-L1, then one spec run at a time on `default` ([contract § C8](../starter-uat-praxis/work-packets.md#c8-spec-runs-lab_uat_run)). Results PR per wave.
6. Wave 6: DR-30 (issues), DR-99 close-out.

**Hazards:**

- Id blocks 1300-1609 / 13000-14109 are taken; use only templates 1610-1659, spawns 14110-14299, buy lists 1310-1348 with sell list 1349 (at most 39 vendors), list rows 131000-134999, and re-run `debug_armory.py audit` first.
- Buy prices must keep `live_db_vendor_arbitrage` green (buy quantity 1). A probe that cannot be grounded after 3 re-probes stops DR-03b (exit 4): ask the architect for a new vendor line.
- The store window's row cap is unknown until DR-02 (fallback 19).
- 269 weapons have no ranged binding: their gear rows grade on `weapon_unbound`, not a FAIL.
- Jaffa has no tree (D-AT04); Asgard creation may fail (F42 / HELD-B16).
- Every Armory spec uses the plaza: one run at a time, never beside [starter-uat-shops SS-D1](../starter-uat-shops/work-packets.md#ss-d1-run-vendorstoml).
- The lab-server MCP refused connections on 2026-10-27.

**Open decisions:** D-DR4 (cross-archetype grant, DR-40), D-DR5 (`.removeitem` question), D-DR7 (sweep server), D-SP2 (shared).

**Next ledger update:** when PR 1 opens.
