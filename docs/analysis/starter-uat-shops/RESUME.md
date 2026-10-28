# starter-uat-shops: resume card

> **Type:** reference. **Audience:** the orchestrator resuming this campaign. **Updated:** 2026-10-27
> **Companions:** [ledger](README.md), [work packets](work-packets.md), [shared UAT contract](../starter-uat-praxis/work-packets.md#shared-uat-contract). Siblings: [praxis](../starter-uat-praxis/RESUME.md), [union](../starter-uat-union/RESUME.md), [debug-armory](../debug-armory/RESUME.md).

**State (2026-10-27):** planned, nothing dispatched. The campaign files live on branch `lab-uat-plans` until ops commits them into `ss-integration`.

**Goal:** every vendor and trainer a new player meets before Tollana or Beta Site is proven in the live client (open, buy, sell, buyback, repair, recharge, train, refusals), guarded by a live-DB test, and specced as `vendors.toml`. Zone hubs already owned by a zone ledger (Castle S1-S5, Omega) stay there (D-SS4).

**Start here:**

1. [Campaign-packet § 0](../../../.claude/skills/campaign-packet/SKILL.md#0-make-sure-nobody-else-is-running-it): check no peer runs it; write `.claude/state/active-campaign`.
2. `ops`: create `ss-integration` (branch `starter-uat-shops/integration`), commit the three campaign files.
3. Wave 1: SS-01, SS-02, SS-04 at once; after SS-01 merges, SS-03a and SS-05; after SS-03a, SS-03b. Merge order SS-01, 02, 04, 03a, 03b, 05. Wave loop, both lane checks (dispatch rule 8), one PR.
4. Wave 2 (ASK-FIRST): SS-D1 on `default` (plan, then execute; [contract § C8](../starter-uat-praxis/work-packets.md#c8-spec-runs-lab_uat_run)); SS-D2 then SS-D3 on `p2`. Server per D-SP2. Results PR.
5. D-SS1 answered (b) or (c): wave 3a SS-20..23 one at a time, one seed PR; then SS-D4 (ASK-FIRST). Answer (a): skip to wave 4.
6. Wave 4: SS-98, SS-99, one PR.

**Hazards:**

- The stasis-room Quartermaster store failed to open on 2026-10-12 (BANK-R10). SS-01 must explain it before SS-D2 runs, or SS-D2 just repeats the failure.
- Sell, buyback, repair and recharge have no dedicated lab tool; rows may grade N3 or BLOCKED. Nothing wears items down today (D-SS3).
- The stasis room is shared: one lane there at a time; never stage SS-D2 beside a Praxis drive in world 12.
- SS-02 may find real seed gaps. Its guard fails closed (no allow-list): the packet reports `blocked` with the miss list, you file an issue and ask the architect for a ruling (expected: a `seed-change` packet before the wave-1 PR).
- The lab-server MCP refused connections on 2026-10-27.

**Open decisions:** D-SS1 (story vendors for Tollana, SGC_W1, Dakara_E1, Beta Site; owner), D-SP2 (server target, shared with Praxis).

**Next ledger update:** when the wave 1 PR opens.
