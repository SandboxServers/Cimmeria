# starter-uat-union: resume card

> **Type:** reference. **Audience:** the orchestrator resuming this campaign. **Updated:** 2026-10-27
> **Companions:** [ledger](README.md), [work packets](work-packets.md), [shared UAT contract](../starter-uat-praxis/work-packets.md#shared-uat-contract). Siblings: [praxis](../starter-uat-praxis/RESUME.md), [shops](../starter-uat-shops/RESUME.md), [debug-armory](../debug-armory/RESUME.md).

**State (2026-10-27):** planned, nothing dispatched. The campaign files live on branch `lab-uat-plans` until ops commits them into `su-integration`.

**Goal:** a graded lab playthrough of every owed Union client row for the Human and the Free Jaffa, SGC_W1 to Beta E1; an issue per failure; a TOML spec per pass. Asgard rows are recorded BLOCKED (D-SU3).

**Start here:**

1. Campaign-packet § 0: check no peer runs it; write `.claude/state/active-campaign`.
2. `ops`: create `su-integration` (branch `starter-uat-union/integration`), commit the three campaign files.
3. SU-00 (documentation-writer) in `su-integration`; packet review loop.
4. Wave 1a (SU-01, 02a, 02b, 02c, 02d, 03) then 1b (SU-04, SU-05), worktrees branched at the `su-integration` head. Wave loop, one docs PR.
5. Before wave 2: D-SU1 and D-SP2 answered (relay to the owner). Ask the user for the lab per wave (contract C1).
6. Per drive packet: stager (fixture), lab-driver, verifier; three attempts plus a Sonnet takeover. Results PR per wave.
7. Wave 7: SU-91, SU-92 (lane check on `cimmeria-lab`), SU-98, SU-99.

**Hazards:**

- The SGC, Omega and Dakara step files were written for the owner ("agents never use the lab"); do not run them before D-SU1.
- M3 to M5 depend on the 1647 fixture (D-SU2); if 1648 is not offered, those rows are BLOCKED.
- Omega services rows share one hub: one lane at a time there.
- Death rows need a self-damage command that may not exist (D-SU4).
- The lab-server MCP refused connections on 2026-10-27.

**Open decisions:** D-SU1 (agents may run owner-reserved Union rows), D-SP2 (server target), D-SP3 (priority; scheduling only).

**Next ledger update:** when SU-00 is integrated.
